//! Helpers shared by the glTF loader's host tests: a small writer of glTF documents in memory
//! (accessors packed into one buffer, as a `data:` URI, an external file or a `.glb` chunk).

use crate::gltf::{GltfDocument, GltfLoadOptions, GltfModelSet};

/// Standard base64 with padding.
pub(crate) fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(LETTERS[(n >> 18) as usize & 63] as char);
        out.push(LETTERS[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { LETTERS[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { LETTERS[n as usize & 63] as char } else { '=' });
    }
    out
}

/// A `data:` URI of the bytes.
pub(crate) fn data_uri(bytes: &[u8]) -> String {
    format!("data:application/octet-stream;base64,{}", base64(bytes))
}

/// Builds the one buffer of a test document and the JSON of its buffer views and accessors.
#[derive(Default)]
pub(crate) struct Buffer {
    pub(crate) bytes: Vec<u8>,
    views: Vec<String>,
    accessors: Vec<String>,
}

impl Buffer {
    /// Adds raw bytes as a buffer view (aligned to 4) and returns the view's index.
    pub(crate) fn view(&mut self, bytes: &[u8]) -> usize {
        while self.bytes.len() % 4 != 0 {
            self.bytes.push(0);
        }
        self.views.push(format!("{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{}}}", self.bytes.len(), bytes.len()));
        self.bytes.extend_from_slice(bytes);
        self.views.len() - 1
    }

    /// Adds an accessor over a new view and returns the accessor's index. `extra` is further
    /// JSON members (for example `"normalized":true`), or empty.
    pub(crate) fn accessor(&mut self, bytes: &[u8], component_type: u32, count: usize, kind: &str, extra: &str) -> usize {
        let view = self.view(bytes);
        let extra = if extra.is_empty() { String::new() } else { format!(",{extra}") };
        self.accessors.push(format!("{{\"bufferView\":{view},\"componentType\":{component_type},\"count\":{count},\"type\":\"{kind}\"{extra}}}"));
        self.accessors.len() - 1
    }

    /// Adds an accessor from JSON written by the caller (for sparse and damaged accessors).
    pub(crate) fn raw_accessor(&mut self, json: String) -> usize {
        self.accessors.push(json);
        self.accessors.len() - 1
    }

    /// FLOAT accessor of `kind` ("SCALAR", "VEC2", "VEC3", "VEC4", "MAT4").
    pub(crate) fn floats(&mut self, values: &[f32], kind: &str) -> usize {
        let per = match kind {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            _ => 16,
        };
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let extra = if kind == "VEC3" { min_max(values) } else { String::new() };
        self.accessor(&bytes, 5126, values.len() / per, kind, &extra)
    }

    /// UNSIGNED_SHORT SCALAR accessor (indices).
    pub(crate) fn u16s(&mut self, values: &[u16]) -> usize {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.accessor(&bytes, 5123, values.len(), "SCALAR", "")
    }

    /// The `"bufferViews":[..],"accessors":[..]` members.
    pub(crate) fn json(&self) -> String {
        format!("\"bufferViews\":[{}],\"accessors\":[{}]", self.views.join(","), self.accessors.join(","))
    }

    /// A whole document: the asset, this buffer as a `data:` URI, its views and accessors, and
    /// `rest` (further top-level members, without a leading comma).
    pub(crate) fn document(&self, rest: &str) -> String {
        self.document_with_buffer(&format!(",\"uri\":\"{}\"", data_uri(&self.bytes)), rest)
    }

    /// As [`Buffer::document`], with the buffer's other members written by the caller: empty for
    /// the binary chunk of a `.glb`, or `,"uri":"data.bin"` for an external file.
    pub(crate) fn document_with_buffer(&self, buffer_members: &str, rest: &str) -> String {
        format!("{{\"asset\":{{\"version\":\"2.0\"}},\"buffers\":[{{\"byteLength\":{}{buffer_members}}}],{},{rest}}}", self.bytes.len(), self.json())
    }
}

/// `"min":[..],"max":[..]` of VEC3 values (glTF asks for them on POSITION).
pub(crate) fn min_max(values: &[f32]) -> String {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in values.chunks_exact(3) {
        for i in 0..3 {
            min[i] = min[i].min(v[i]);
            max[i] = max[i].max(v[i]);
        }
    }
    format!("\"min\":[{},{},{}],\"max\":[{},{},{}]", min[0], min[1], min[2], max[0], max[1], max[2])
}

/// Parses and builds one document that needs no external file, with the given options.
pub(crate) fn build_with(json: &str, options: &GltfLoadOptions) -> GltfModelSet {
    let document = GltfDocument::parse(json.as_bytes()).expect("the test document parses");
    let mut set = GltfModelSet::new();
    set.add(&document, "test", None, &|_| None, options).expect("the test document builds");
    set
}

/// Parses and builds one document that needs no external file, with the default options.
pub(crate) fn build(json: &str) -> GltfModelSet {
    build_with(json, &GltfLoadOptions::default())
}

/// The smallest document with a drawable triangle: positions (0,0,0) (1,0,0) (0,1,0), indices
/// 0 1 2, one mesh, one node, one scene. `primitive_extra` is added to the primitive (with its
/// own leading comma), `rest` to the document (with its own leading comma).
pub(crate) fn triangle_document(primitive_extra: &str, rest: &str) -> String {
    let mut buffer = Buffer::default();
    let positions = buffer.floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], "VEC3");
    let indices = buffer.u16s(&[0, 1, 2]);
    buffer.document(&format!("\"meshes\":[{{\"primitives\":[{{\"attributes\":{{\"POSITION\":{positions}}},\"indices\":{indices}{primitive_extra}}}]}}],\"nodes\":[{{\"mesh\":0}}],\"scenes\":[{{\"nodes\":[0]}}],\"scene\":0{rest}"))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A PNG file of `width` x `height` RGBA8 pixels (`rgba` rows top to bottom), written with
/// uncompressed deflate blocks - enough for the loader's decoder to read back the same pixels.
pub(crate) fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    let mut raw = Vec::new();
    for row in rgba.chunks_exact((width * 4) as usize) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(block) = blocks.next() {
        zlib.push(if blocks.peek().is_none() { 1 } else { 0 });
        zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut file = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        file.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        file.extend_from_slice(&body);
        file.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(b"IDAT", &zlib);
    chunk(b"IEND", &[]);
    file
}

/// A `.glb` file of a JSON document and an optional binary chunk (both padded to 4 bytes).
pub(crate) fn glb(json: &str, bin: Option<&[u8]>) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut body = Vec::new();
    body.extend_from_slice(&(json.len() as u32).to_le_bytes());
    body.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
    body.extend_from_slice(&json);
    if let Some(bin) = bin {
        let mut bin = bin.to_vec();
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        body.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        body.extend_from_slice(&0x004E_4942u32.to_le_bytes());
        body.extend_from_slice(&bin);
    }
    let mut file = b"glTF".to_vec();
    file.extend_from_slice(&2u32.to_le_bytes());
    file.extend_from_slice(&((body.len() + 12) as u32).to_le_bytes());
    file.extend_from_slice(&body);
    file
}
