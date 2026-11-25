//! The binary glTF container (.glb): a 12-byte header, a JSON chunk and an optional binary chunk
//! (Not in C#: the reference converter's glTF library reads the container). Written here, with
//! every length checked, instead of calling the `gltf` crate's `Glb::from_slice`, which subtracts
//! the header size from the file's stated length without a check.

use crate::context::GraphicsException;

/// The four bytes a .glb file starts with.
pub(crate) const GLB_MAGIC: [u8; 4] = *b"glTF";
const CHUNK_JSON: u32 = 0x4E4F_534A;
const CHUNK_BIN: u32 = 0x004E_4942;

/// The two parts of a .glb file, borrowed from it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct GlbParts<'a> {
    /// The JSON chunk (the glTF document).
    pub(crate) json: &'a [u8],
    /// The binary chunk (the buffer without a URI), or None.
    pub(crate) bin: Option<&'a [u8]>,
}

/// True when the bytes start like a .glb file.
pub(crate) fn is_glb(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == GLB_MAGIC
}

fn refuse(what: String) -> GraphicsException {
    GraphicsException::new(format!("The .glb file is damaged: {what}. Fetch the file again, or export the model again."))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Splits a .glb file into its JSON chunk and its optional binary chunk. Chunks of other kinds are
/// skipped, as the glTF specification asks. Bytes after the length the header states are ignored;
/// a header that states more bytes than were handed over is an error (a download cut short).
pub(crate) fn split_glb(bytes: &[u8]) -> Result<GlbParts<'_>, GraphicsException> {
    if bytes.len() < 12 {
        return Err(refuse(format!("it is {} bytes long, shorter than the 12-byte header", bytes.len())));
    }
    if !is_glb(bytes) {
        return Err(refuse("it does not start with \"glTF\"".to_string()));
    }
    let version = u32_at(bytes, 4);
    if version != 2 {
        return Err(GraphicsException::new(format!("The .glb file is binary glTF version {version}; only version 2 (glTF 2.0) is read. Convert the model to glTF 2.0.")));
    }
    let length = u32_at(bytes, 8) as usize;
    if length < 12 {
        return Err(refuse(format!("its header says the file is {length} bytes long, less than the header itself")));
    }
    if length > bytes.len() {
        return Err(refuse(format!("its header says the file is {length} bytes long, but {} bytes were handed over (the download was cut short)", bytes.len())));
    }
    let bytes = &bytes[..length];
    let mut json = None;
    let mut bin = None;
    let mut at = 12usize;
    let mut chunk = 0usize;
    while at < bytes.len() {
        if bytes.len() - at < 8 {
            return Err(refuse(format!("chunk {chunk} starts {} bytes before the end, too few for a chunk header", bytes.len() - at)));
        }
        let chunk_length = u32_at(bytes, at) as usize;
        let kind = u32_at(bytes, at + 4);
        let start = at + 8;
        if chunk_length > bytes.len() - start {
            return Err(refuse(format!("chunk {chunk} says it holds {chunk_length} bytes, but only {} follow", bytes.len() - start)));
        }
        let data = &bytes[start..start + chunk_length];
        if chunk == 0 {
            if kind != CHUNK_JSON {
                return Err(refuse("its first chunk is not the JSON chunk".to_string()));
            }
            json = Some(data);
        } else if kind == CHUNK_BIN && bin.is_none() && chunk == 1 {
            bin = Some(data);
        }
        // Chunks are aligned to 4 bytes; a length that is not a multiple of 4 is tolerated by
        // stepping to the next boundary (and stopping at the end).
        at = (start + chunk_length).saturating_add(3) & !3;
        chunk += 1;
    }
    match json {
        Some(json) => Ok(GlbParts { json, bin }),
        None => Err(refuse("it has no JSON chunk".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::{split_glb, GlbParts};

    /// A .glb with the given chunks (kind, data); lengths are written as they are.
    pub(crate) fn glb(chunks: &[(u32, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (kind, data) in chunks {
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(&kind.to_le_bytes());
            body.extend_from_slice(data);
        }
        let mut file = Vec::new();
        file.extend_from_slice(b"glTF");
        file.extend_from_slice(&2u32.to_le_bytes());
        file.extend_from_slice(&((body.len() + 12) as u32).to_le_bytes());
        file.extend_from_slice(&body);
        file
    }

    #[test]
    fn a_glb_splits_into_json_and_bin() {
        let file = glb(&[(0x4E4F_534A, b"{}  "), (0x004E_4942, &[1, 2, 3, 4])]);
        assert_eq!(split_glb(&file).unwrap(), GlbParts { json: b"{}  ", bin: Some(&[1, 2, 3, 4]) });
        let json_only = glb(&[(0x4E4F_534A, b"{}  ")]);
        assert_eq!(split_glb(&json_only).unwrap(), GlbParts { json: b"{}  ", bin: None });
    }

    #[test]
    fn an_unknown_chunk_is_skipped_and_trailing_bytes_are_ignored() {
        let mut file = glb(&[(0x4E4F_534A, b"{}  "), (0x004E_4942, &[9, 9, 9, 9]), (0x1234_5678, &[7; 8])]);
        file.extend_from_slice(&[0xFF; 5]);
        assert_eq!(split_glb(&file).unwrap().bin, Some(&[9u8, 9, 9, 9][..]));
        // A binary chunk that is not the second chunk is not the glTF buffer.
        let late = glb(&[(0x4E4F_534A, b"{}  "), (0x1234_5678, &[7; 4]), (0x004E_4942, &[9; 4])]);
        assert_eq!(split_glb(&late).unwrap().bin, None);
    }

    #[test]
    fn a_damaged_glb_is_an_error_with_a_message_never_a_panic() {
        let good = glb(&[(0x4E4F_534A, b"{}  "), (0x004E_4942, &[1, 2, 3, 4])]);
        for cut in 0..good.len() {
            let result = split_glb(&good[..cut]);
            assert!(result.is_err(), "a file cut to {cut} bytes must be refused");
        }
        let message = |bytes: &[u8]| split_glb(bytes).unwrap_err().message().to_string();
        let mut short_length = good.clone();
        short_length[8..12].copy_from_slice(&5u32.to_le_bytes());
        assert!(message(&short_length).contains("less than the header itself"));
        let mut version = good.clone();
        version[4] = 1;
        assert!(message(&version).contains("version 1"));
        let mut chunk = good.clone();
        chunk[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(message(&chunk).contains("chunk 0 says it holds 4294967295 bytes"));
        let bin_first = glb(&[(0x004E_4942, &[1, 2, 3, 4])]);
        assert!(message(&bin_first).contains("first chunk is not the JSON chunk"));
        assert!(message(b"glTF").contains("shorter than the 12-byte header"));
        assert!(message(&glb(&[])).contains("no JSON chunk"));
    }
}
