//! The bounds-checked little-endian reader of the model-data format's sections and records (an
//! internal helper of [`ModelDataSet::read`](crate::ModelDataSet::read)).

use crate::context::GraphicsException;

/// "None" for a u32 reference (a null string's offset).
pub(crate) const NONE_U32: u32 = u32::MAX;

/// One section of a blob: its record size, record count and bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Section<'a> {
    pub(crate) tag: &'static str,
    pub(crate) record_size: usize,
    pub(crate) count: usize,
    pub(crate) bytes: &'a [u8],
}

impl<'a> Section<'a> {
    /// The record at `index` (the first `known` bytes the reader understands; a later minor
    /// version may make records longer).
    pub(crate) fn record(&self, index: usize) -> Record<'a> {
        let start = index * self.record_size;
        Record { bytes: &self.bytes[start..start + self.record_size] }
    }

    /// The bytes `offset .. offset + length` of a byte section, or an error naming what asked.
    pub(crate) fn range(&self, offset: u32, length: usize, what: &dyn Fn() -> String) -> Result<&'a [u8], GraphicsException> {
        let start = offset as usize;
        match start.checked_add(length) {
            Some(end) if end <= self.bytes.len() => Ok(&self.bytes[start..end]),
            _ => Err(GraphicsException::new(format!("{} reaches past the end of section {} ({} bytes from byte {offset}, but the section has {} bytes); the model data is damaged or was cut short - write it again with ModelDataWriter.", what(), self.tag, length, self.bytes.len()))),
        }
    }
}

/// One fixed-size record, read field by field at byte offsets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Record<'a> {
    bytes: &'a [u8],
}

impl<'a> Record<'a> {
    pub(crate) fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes([self.bytes[at], self.bytes[at + 1], self.bytes[at + 2], self.bytes[at + 3]])
    }

    pub(crate) fn i32(&self, at: usize) -> i32 {
        self.u32(at) as i32
    }

    pub(crate) fn f32(&self, at: usize) -> f32 {
        f32::from_bits(self.u32(at))
    }

    pub(crate) fn bool(&self, at: usize) -> bool {
        self.u32(at) != 0
    }

    pub(crate) fn vec3(&self, at: usize) -> glam::Vec3 {
        glam::Vec3::new(self.f32(at), self.f32(at + 4), self.f32(at + 8))
    }

    pub(crate) fn vec4(&self, at: usize) -> glam::Vec4 {
        glam::Vec4::new(self.f32(at), self.f32(at + 4), self.f32(at + 8), self.f32(at + 12))
    }

    pub(crate) fn mat4(&self, at: usize) -> glam::Mat4 {
        let mut values = [0.0f32; 16];
        for (index, value) in values.iter_mut().enumerate() {
            *value = self.f32(at + index * 4);
        }
        glam::Mat4::from_cols_array(&values)
    }

    /// A (start, count) pair.
    pub(crate) fn list(&self, at: usize) -> (u32, u32) {
        (self.u32(at), self.u32(at + 4))
    }
}

/// Reads a little-endian u32 at a byte offset of a slice the caller has bounds-checked.
pub(crate) fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// The pools every list and string refers to: the string bytes, the u32 pool and the f32 pool.
pub(crate) struct Pools<'a> {
    pub(crate) strings: Section<'a>,
    pub(crate) u32s: Section<'a>,
    pub(crate) f32s: Section<'a>,
}

impl<'a> Pools<'a> {
    /// A string reference (offset, length) at a record field: None for the null string.
    pub(crate) fn string(&self, record: &Record<'_>, at: usize, what: &dyn Fn() -> String) -> Result<Option<String>, GraphicsException> {
        self.string_at(record.u32(at), record.u32(at + 4), what)
    }

    fn string_at(&self, offset: u32, length: u32, what: &dyn Fn() -> String) -> Result<Option<String>, GraphicsException> {
        if offset == NONE_U32 {
            return Ok(None);
        }
        let bytes = self.strings.range(offset, length as usize, &|| format!("the name of {}", what()))?;
        match std::str::from_utf8(bytes) {
            Ok(text) => Ok(Some(text.to_string())),
            Err(_) => Err(GraphicsException::new(format!("The name of {} is not UTF-8; the model data is damaged - write it again with ModelDataWriter.", what()))),
        }
    }

    /// The u32 list (start, count) at a record field.
    pub(crate) fn u32_list(&self, record: &Record<'_>, at: usize, what: &dyn Fn() -> String) -> Result<Vec<u32>, GraphicsException> {
        let (start, count) = record.list(at);
        let bytes = self.pool_range(&self.u32s, start, count as usize, what)?;
        Ok(bytes.chunks_exact(4).map(|chunk| read_u32(chunk, 0)).collect())
    }

    /// The f32 list (start, count) at a record field.
    pub(crate) fn f32_list(&self, record: &Record<'_>, at: usize, what: &dyn Fn() -> String) -> Result<Vec<f32>, GraphicsException> {
        let (start, count) = record.list(at);
        let bytes = self.pool_range(&self.f32s, start, count as usize, what)?;
        Ok(bytes.chunks_exact(4).map(|chunk| f32::from_bits(read_u32(chunk, 0))).collect())
    }

    /// The string list (start, count) at a record field: two u32 pool elements per string.
    pub(crate) fn string_list(&self, record: &Record<'_>, at: usize, what: &dyn Fn() -> String) -> Result<Vec<String>, GraphicsException> {
        let (start, count) = record.list(at);
        let pairs = match (count as usize).checked_mul(2) {
            Some(pairs) => pairs,
            None => return Err(GraphicsException::new(format!("The string list of {} is too long; the model data is damaged.", what()))),
        };
        let bytes = self.pool_range(&self.u32s, start, pairs, what)?;
        let mut strings = Vec::with_capacity(count as usize);
        for pair in bytes.chunks_exact(8) {
            let text = self.string_at(read_u32(pair, 0), read_u32(pair, 4), what)?;
            strings.push(text.unwrap_or_default());
        }
        Ok(strings)
    }

    fn pool_range(&self, pool: &Section<'a>, start: u32, count: usize, what: &dyn Fn() -> String) -> Result<&'a [u8], GraphicsException> {
        let in_range = (start as usize).checked_add(count).is_some_and(|end| end <= pool.count);
        if !in_range {
            return Err(GraphicsException::new(format!("A list of {} names elements {start} to {} of section {}, which holds {}; the model data is damaged - write it again with ModelDataWriter.", what(), start as u64 + count as u64, pool.tag, pool.count)));
        }
        Ok(&pool.bytes[start as usize * 4..(start as usize + count) * 4])
    }
}
