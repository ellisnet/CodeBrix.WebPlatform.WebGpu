//! Reading a glTF accessor's elements as floats or as indices, with every byte range checked
//! first (C#: the reference converter's glTF library, `Accessor.AsVector3Array` and its
//! siblings). The typed elements come from the `gltf` crate's accessor iterator (byte strides,
//! interleaved views, sparse accessors); the element type asked for is always the one the
//! accessor itself declares, and every range the iterator will touch is checked here before it
//! runs, so a damaged file gives a message and never a panic. The conversion of integer
//! components to floats is written here, to give the reference converter's numbers.

use gltf::accessor::{DataType, Dimensions, Iter};
use gltf::json;

/// The bytes of every buffer of a document, in document order; None for a buffer that was not
/// needed or not supplied.
pub(crate) type BufferData<'a> = [Option<&'a [u8]>];

/// The elements of an accessor as floats: `components` floats per element, `count` elements.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FloatElements {
    /// Floats per element: 1 (SCALAR), 2, 3, 4 (VEC2..VEC4) or 16 (MAT4).
    pub(crate) components: usize,
    /// The floats, element after element.
    pub(crate) values: Vec<f32>,
}

impl FloatElements {
    /// The number of elements.
    pub(crate) fn count(&self) -> usize {
        if self.components == 0 {
            0
        } else {
            self.values.len() / self.components
        }
    }

    /// Element `index` as N floats; components the accessor does not have are `fill` (C#: a
    /// `Vector3` read from a VEC2 accessor has z = 0, a colour read from VEC3 has alpha = the
    /// default). None past the end.
    pub(crate) fn get<const N: usize>(&self, index: usize, fill: [f32; N]) -> Option<[f32; N]> {
        let start = index.checked_mul(self.components)?;
        let element = self.values.get(start..start.checked_add(self.components)?)?;
        let mut out = fill;
        for (target, value) in out.iter_mut().zip(element) {
            *target = *value;
        }
        Some(out)
    }
}

/// The largest number of elements an accessor may have: more cannot be a model, and the limit
/// keeps a damaged count from asking for a huge allocation (a sparse accessor without a buffer
/// view allocates `count` elements with no bytes behind them).
pub(crate) const MAX_ELEMENTS: u64 = 1 << 28;

fn component_size(data_type: DataType) -> usize {
    match data_type {
        DataType::I8 | DataType::U8 => 1,
        DataType::I16 | DataType::U16 => 2,
        DataType::U32 | DataType::F32 => 4,
    }
}

fn component_count(dimensions: Dimensions) -> usize {
    match dimensions {
        Dimensions::Scalar => 1,
        Dimensions::Vec2 => 2,
        Dimensions::Vec3 => 3,
        Dimensions::Vec4 | Dimensions::Mat2 => 4,
        Dimensions::Mat3 => 9,
        Dimensions::Mat4 => 16,
    }
}

fn type_name(data_type: DataType, dimensions: Dimensions) -> String {
    let component = match data_type {
        DataType::I8 => "BYTE",
        DataType::U8 => "UNSIGNED_BYTE",
        DataType::I16 => "SHORT",
        DataType::U16 => "UNSIGNED_SHORT",
        DataType::U32 => "UNSIGNED_INT",
        DataType::F32 => "FLOAT",
    };
    let shape = match dimensions {
        Dimensions::Scalar => "SCALAR",
        Dimensions::Vec2 => "VEC2",
        Dimensions::Vec3 => "VEC3",
        Dimensions::Vec4 => "VEC4",
        Dimensions::Mat2 => "MAT2",
        Dimensions::Mat3 => "MAT3",
        Dimensions::Mat4 => "MAT4",
    };
    format!("{shape} of {component}")
}

/// Checks that `count` elements of `element_size` bytes, `stride` apart, starting `offset` bytes
/// into buffer view `view`, lie inside the view and inside the bytes the buffer really has.
fn check_view_range(root: &json::Root, buffers: &BufferData<'_>, view: usize, offset: u64, count: u64, element_size: usize, what: &str) -> Result<(), String> {
    let Some(view_json) = root.buffer_views.get(view) else {
        return Err(format!("{what} names buffer view {view}, but the document has {}", root.buffer_views.len()));
    };
    let buffer_index = view_json.buffer.value();
    let Some(data) = buffers.get(buffer_index).copied().flatten() else {
        return Err(format!("{what} reads buffer {buffer_index}, whose bytes were not supplied"));
    };
    let view_offset = view_json.byte_offset.map_or(0, |o| o.0);
    let view_length = view_json.byte_length.0;
    if view_offset.checked_add(view_length).is_none_or(|end| end > data.len() as u64) {
        return Err(format!("{what} reads buffer view {view}, bytes {view_offset} to {} of buffer {buffer_index}, which has only {} bytes", view_offset as u128 + view_length as u128, data.len()));
    }
    let stride = match view_json.byte_stride {
        Some(stride) => stride.0 as u64,
        None => element_size as u64,
    };
    if stride < element_size as u64 {
        return Err(format!("{what} has elements of {element_size} bytes in buffer view {view}, whose byte stride is only {stride}"));
    }
    if count == 0 {
        return Ok(());
    }
    let end = (count - 1).checked_mul(stride).and_then(|n| n.checked_add(offset)).and_then(|n| n.checked_add(element_size as u64));
    if end.is_none_or(|end| end > view_length) {
        return Err(format!("{what} needs {count} elements of {element_size} bytes ({stride} apart, from byte {offset}) in buffer view {view}, which is only {view_length} bytes long"));
    }
    Ok(())
}

/// Checks everything the `gltf` crate's iterator will touch for accessor `index` and returns its
/// (data type, dimensions, element count). `what` describes the accessor's use, for messages.
fn check_accessor(document: &gltf::Document, buffers: &BufferData<'_>, index: usize, what: &str) -> Result<(DataType, Dimensions, usize), String> {
    let root = document.as_json();
    let Some(accessor) = root.accessors.get(index) else {
        return Err(format!("{what} names accessor {index}, but the document has {}", root.accessors.len()));
    };
    let (json::validation::Checked::Valid(component), json::validation::Checked::Valid(dimensions)) = (accessor.component_type, accessor.type_) else {
        return Err(format!("{what} (accessor {index}) has a component type or element type glTF 2.0 does not have"));
    };
    let data_type = component.0;
    let count = accessor.count.0;
    if count > MAX_ELEMENTS {
        return Err(format!("{what} (accessor {index}) says it has {count} elements, more than the {MAX_ELEMENTS} this loader reads"));
    }
    if matches!(dimensions, Dimensions::Mat2 | Dimensions::Mat3) || (dimensions == Dimensions::Mat4 && data_type != DataType::F32) {
        return Err(format!("{what} (accessor {index}) is {}, which this loader does not read", type_name(data_type, dimensions)));
    }
    let element_size = component_size(data_type) * component_count(dimensions);
    let label = format!("{what} (accessor {index})");
    if let Some(view) = accessor.buffer_view {
        check_view_range(root, buffers, view.value(), accessor.byte_offset.map_or(0, |o| o.0), count, element_size, &label)?;
    } else if accessor.sparse.is_none() {
        return Err(format!("{label} has neither a buffer view nor sparse values"));
    }
    if let Some(sparse) = &accessor.sparse {
        let sparse_count = sparse.count.0;
        if sparse_count == 0 || sparse_count > count {
            return Err(format!("{label} has {sparse_count} sparse values for {count} elements; a sparse accessor has 1 to count values"));
        }
        let json::validation::Checked::Valid(index_type) = sparse.indices.component_type else {
            return Err(format!("{label} has sparse indices of a type glTF 2.0 does not have"));
        };
        check_view_range(root, buffers, sparse.indices.buffer_view.value(), sparse.indices.byte_offset.0, sparse_count, component_size(index_type.0), &format!("{label}, its sparse indices,"))?;
        check_view_range(root, buffers, sparse.values.buffer_view.value(), sparse.values.byte_offset.0, sparse_count, element_size, &format!("{label}, its sparse values,"))?;
    }
    Ok((data_type, dimensions, count as usize))
}

/// The conversions of one integer component to a float (C#: the glTF library's accessor
/// decoding). A normalized component is DIVIDED by its type's largest value, and a signed one is
/// then kept at or above -1; a component that is not normalized is the integer as a float.
pub(crate) mod convert {
    /// UNSIGNED_BYTE.
    pub(crate) fn u8(value: u8, normalized: bool) -> f32 {
        if normalized {
            value as f32 / 255.0
        } else {
            value as f32
        }
    }

    /// BYTE.
    pub(crate) fn i8(value: i8, normalized: bool) -> f32 {
        if normalized {
            (value as f32 / 127.0).max(-1.0)
        } else {
            value as f32
        }
    }

    /// UNSIGNED_SHORT.
    pub(crate) fn u16(value: u16, normalized: bool) -> f32 {
        if normalized {
            value as f32 / 65535.0
        } else {
            value as f32
        }
    }

    /// SHORT.
    pub(crate) fn i16(value: i16, normalized: bool) -> f32 {
        if normalized {
            (value as f32 / 32767.0).max(-1.0)
        } else {
            value as f32
        }
    }

    /// UNSIGNED_INT (normalized or not: glTF has no normalized 32-bit integers).
    pub(crate) fn u32(value: u32, _normalized: bool) -> f32 {
        value as f32
    }

    /// FLOAT.
    pub(crate) fn f32(value: f32, _normalized: bool) -> f32 {
        value
    }
}

/// Reads accessor `index` as floats (see [`FloatElements`]). An accessor with no elements gives
/// none. `what` describes the use, for the error message.
pub(crate) fn read_floats(document: &gltf::Document, buffers: &BufferData<'_>, index: usize, what: &str) -> Result<FloatElements, String> {
    let (data_type, dimensions, count) = check_accessor(document, buffers, index, what)?;
    let components = component_count(dimensions);
    if count == 0 {
        return Ok(FloatElements { components, values: Vec::new() });
    }
    let Some(accessor) = document.accessors().nth(index) else {
        return Err(format!("{what} names accessor {index}, which the document does not have"));
    };
    let normalized = accessor.normalized();
    let get = |buffer: gltf::Buffer<'_>| buffers.get(buffer.index()).copied().flatten();
    let mut values: Vec<f32> = Vec::with_capacity(count * components);
    let unreadable = || format!("{what} (accessor {index}) could not be read from its buffer view");
    macro_rules! read {
        ($component:ty, $convert:path) => {
            match dimensions {
                Dimensions::Scalar => Iter::<$component>::new(accessor, get).ok_or_else(unreadable)?.for_each(|v| values.push($convert(v, normalized))),
                Dimensions::Vec2 => Iter::<[$component; 2]>::new(accessor, get).ok_or_else(unreadable)?.for_each(|v| values.extend(v.map(|c| $convert(c, normalized)))),
                Dimensions::Vec3 => Iter::<[$component; 3]>::new(accessor, get).ok_or_else(unreadable)?.for_each(|v| values.extend(v.map(|c| $convert(c, normalized)))),
                Dimensions::Vec4 => Iter::<[$component; 4]>::new(accessor, get).ok_or_else(unreadable)?.for_each(|v| values.extend(v.map(|c| $convert(c, normalized)))),
                Dimensions::Mat4 => Iter::<[[$component; 4]; 4]>::new(accessor, get).ok_or_else(unreadable)?.for_each(|m| {
                    for column in m {
                        values.extend(column.map(|c| $convert(c, normalized)));
                    }
                }),
                Dimensions::Mat2 | Dimensions::Mat3 => return Err(unreadable()),
            }
        };
    }
    match data_type {
        DataType::I8 => read!(i8, convert::i8),
        DataType::U8 => read!(u8, convert::u8),
        DataType::I16 => read!(i16, convert::i16),
        DataType::U16 => read!(u16, convert::u16),
        DataType::U32 => read!(u32, convert::u32),
        DataType::F32 => read!(f32, convert::f32),
    }
    if values.len() != count * components {
        return Err(format!("{what} (accessor {index}) gave {} values where {count} elements of {components} were expected", values.len()));
    }
    Ok(FloatElements { components, values })
}

/// Reads accessor `index` as unsigned integers (a primitive's indices): a SCALAR accessor of
/// UNSIGNED_BYTE, UNSIGNED_SHORT or UNSIGNED_INT. Any other type is an error.
pub(crate) fn read_indices(document: &gltf::Document, buffers: &BufferData<'_>, index: usize, what: &str) -> Result<Vec<u32>, String> {
    let (data_type, dimensions, count) = check_accessor(document, buffers, index, what)?;
    if dimensions != Dimensions::Scalar || !matches!(data_type, DataType::U8 | DataType::U16 | DataType::U32) {
        return Err(format!("{what} (accessor {index}) is {}; indices are SCALAR of UNSIGNED_BYTE, UNSIGNED_SHORT or UNSIGNED_INT", type_name(data_type, dimensions)));
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    let Some(accessor) = document.accessors().nth(index) else {
        return Err(format!("{what} names accessor {index}, which the document does not have"));
    };
    let get = |buffer: gltf::Buffer<'_>| buffers.get(buffer.index()).copied().flatten();
    let unreadable = || format!("{what} (accessor {index}) could not be read from its buffer view");
    let values: Vec<u32> = match data_type {
        DataType::U8 => Iter::<u8>::new(accessor, get).ok_or_else(unreadable)?.map(u32::from).collect(),
        DataType::U16 => Iter::<u16>::new(accessor, get).ok_or_else(unreadable)?.map(u32::from).collect(),
        _ => Iter::<u32>::new(accessor, get).ok_or_else(unreadable)?.collect(),
    };
    if values.len() != count {
        return Err(format!("{what} (accessor {index}) gave {} indices where {count} were expected", values.len()));
    }
    Ok(values)
}
