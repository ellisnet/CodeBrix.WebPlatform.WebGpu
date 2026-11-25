//! The addresses a glTF document names: `data:` URIs decoded, percent escapes undone, and a
//! relative URI joined to the model's own address (C#: the reference converter's
//! `GltfModelConverter.CombineKey`; the `data:` decoding is its glTF library's).

/// Undoes percent escapes (`%20` -> a space), reading the escaped bytes as UTF-8 (C#:
/// `Uri.UnescapeDataString`). An escape that is not two hexadecimal digits, and escaped bytes that
/// are not UTF-8, stay as they are written.
pub(crate) fn unescape(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            // A run of escapes is decoded together, so that a multi-byte character comes out whole.
            let mut run = Vec::new();
            let mut end = index;
            while end + 2 < bytes.len() && bytes[end] == b'%' {
                match (hex(bytes[end + 1]), hex(bytes[end + 2])) {
                    (Some(high), Some(low)) => run.push(high * 16 + low),
                    _ => break,
                }
                end += 3;
            }
            if !run.is_empty() {
                if std::str::from_utf8(&run).is_ok() {
                    out.extend_from_slice(&run);
                } else {
                    out.extend_from_slice(&bytes[index..end]);
                }
                index = end;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| uri.to_string())
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Joins a base address and a relative URI into one address: '/' separators, percent escapes
/// undone, "." and ".." segments resolved, so that two models in sibling folders that name one
/// image as "../Textures/bark.jpg" get the same address - and share one texture (C#:
/// `GltfModelConverter.CombineKey`, the same result for the same arguments). A ".." that would
/// climb above the first segment is kept.
pub(crate) fn combine_address(base: &str, relative_uri: &str) -> String {
    let path = unescape(relative_uri).replace('\\', "/");
    let joined = format!("{}/{}", base.replace('\\', "/").trim_end_matches('/'), path.trim_start_matches('/'));
    let mut segments: Vec<&str> = Vec::new();
    for segment in joined.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." && segments.last().is_some_and(|last| *last != "..") {
            segments.pop();
        } else {
            segments.push(segment);
        }
    }
    segments.join("/")
}

/// True when a URI is a `data:` URI (the scheme in any letter case).
pub(crate) fn is_data_uri(uri: &str) -> bool {
    uri.len() >= 5 && uri.as_bytes()[..5].eq_ignore_ascii_case(b"data:")
}

/// Decodes a `data:` URI: `data:[<media type>][;base64],<data>`. Base64 data may leave out its
/// padding and may hold white space; data without `;base64` is percent-escaped bytes. An error
/// says what is wrong.
pub(crate) fn decode_data_uri(uri: &str) -> Result<Vec<u8>, String> {
    if !is_data_uri(uri) {
        return Err("it is not a data: URI".to_string());
    }
    let rest = &uri[5..];
    let Some(comma) = rest.find(',') else {
        return Err("the data: URI has no ',' before its data".to_string());
    };
    let (header, data) = (&rest[..comma], &rest[comma + 1..]);
    let base64 = header.rsplit(';').next().is_some_and(|last| last.trim().eq_ignore_ascii_case("base64")) && header.contains(';');
    if base64 {
        decode_base64(data)
    } else {
        Ok(percent_bytes(data))
    }
}

/// Percent-escaped text as bytes (an escape that is not two hexadecimal digits stays as written).
fn percent_bytes(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    out
}

/// Decodes standard base64 (also the URL-safe letters '-' and '_'); white space is skipped and
/// the '=' padding may be missing.
fn decode_base64(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3 + 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    let mut padding = 0usize;
    for (position, byte) in text.bytes().enumerate() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => {
                padding += 1;
                continue;
            }
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            other => return Err(format!("its base64 data has the character {:?} at position {position}, which is not base64", other as char)),
        };
        if padding > 0 {
            return Err(format!("its base64 data goes on after the '=' padding (position {position})"));
        }
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if bits >= 6 {
        return Err("its base64 data is cut short (one character is left over)".to_string());
    }
    if padding > 2 {
        return Err("its base64 data ends in more than two '=' characters".to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{combine_address, decode_data_uri, is_data_uri, unescape};

    #[test]
    fn percent_escapes_are_undone_as_utf8() {
        assert_eq!(unescape("Textures/color%20map.png"), "Textures/color map.png");
        assert_eq!(unescape("b%C3%A4rk.png"), "bärk.png");
        assert_eq!(unescape("100%.png"), "100%.png");
        assert_eq!(unescape("a%zz%4"), "a%zz%4");
        assert_eq!(unescape("bad%FFutf8"), "bad%FFutf8");
        assert_eq!(unescape("%41%42"), "AB");
        assert_eq!(unescape(""), "");
    }

    #[test]
    fn an_address_is_joined_with_dots_resolved_as_the_reference_converter_does() {
        assert_eq!(combine_address("survival-kit", "Textures/colormap.png"), "survival-kit/Textures/colormap.png");
        assert_eq!(combine_address("kits/forest/", "/bark.jpg"), "kits/forest/bark.jpg");
        assert_eq!(combine_address("kits/forest", "../Textures/bark.jpg"), "kits/Textures/bark.jpg");
        assert_eq!(combine_address("kits/meadow", "../Textures/bark.jpg"), "kits/Textures/bark.jpg");
        assert_eq!(combine_address("kits\\forest", ".\\a\\..\\b%20c.png"), "kits/forest/b c.png");
        assert_eq!(combine_address("a", "../../x.png"), "../x.png");
        assert_eq!(combine_address("", "x.png"), "x.png");
        assert_eq!(combine_address("https://host/models/a", "../t.png"), "https:/host/models/t.png");
    }

    #[test]
    fn a_data_uri_is_decoded() {
        assert!(is_data_uri("DATA:application/octet-stream;base64,AAAA"));
        assert!(!is_data_uri("model.bin"));
        assert_eq!(decode_data_uri("data:application/octet-stream;base64,AAECAw==").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(decode_data_uri("data:application/gltf-buffer;base64,AAEC").unwrap(), vec![0, 1, 2]);
        assert_eq!(decode_data_uri("data:;base64,AAE").unwrap(), vec![0, 1]);
        assert_eq!(decode_data_uri("data:image/png;base64,/+8=\n").unwrap(), vec![0xFF, 0xEF]);
        assert_eq!(decode_data_uri("data:text/plain,a%20b").unwrap(), b"a b".to_vec());
        assert_eq!(decode_data_uri("data:,").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn a_bad_data_uri_says_what_is_wrong() {
        assert!(decode_data_uri("data:application/octet-stream;base64").unwrap_err().contains("no ','"));
        assert!(decode_data_uri("data:x;base64,AA*A").unwrap_err().contains("'*'"));
        assert!(decode_data_uri("data:x;base64,A").unwrap_err().contains("cut short"));
        assert!(decode_data_uri("data:x;base64,AA==AA").unwrap_err().contains("after the '=' padding"));
        assert!(decode_data_uri("file.bin").unwrap_err().contains("not a data: URI"));
    }
}
