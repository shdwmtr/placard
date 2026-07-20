const URL_SAFE_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn encode_with(data: &[u8], alphabet: &[u8; 64], pad: bool) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        match chunk {
            [b0, b1, b2] => {
                out.push(alphabet[(b0 >> 2) as usize] as char);
                out.push(alphabet[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
                out.push(alphabet[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
                out.push(alphabet[(b2 & 0x3F) as usize] as char);
            }
            [b0, b1] => {
                out.push(alphabet[(b0 >> 2) as usize] as char);
                out.push(alphabet[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
                out.push(alphabet[((b1 & 0x0F) << 2) as usize] as char);
                if pad {
                    out.push('=');
                }
            }
            [b0] => {
                out.push(alphabet[(b0 >> 2) as usize] as char);
                out.push(alphabet[((b0 & 0x03) << 4) as usize] as char);
                if pad {
                    out.push_str("==");
                }
            }
            _ => unreachable!(),
        }
    }
    out
}

pub fn encode(data: &[u8]) -> String {
    encode_with(data, URL_SAFE_ALPHABET, false)
}

pub fn encode_standard(data: &[u8]) -> String {
    encode_with(data, STANDARD_ALPHABET, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4648_vectors_unpadded() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg");
        assert_eq!(encode(b"fo"), "Zm8");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg");
        assert_eq!(encode(b"fooba"), "Zm9vYmE");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn rfc4648_vectors_standard_padded() {
        assert_eq!(encode_standard(b""), "");
        assert_eq!(encode_standard(b"f"), "Zg==");
        assert_eq!(encode_standard(b"fo"), "Zm8=");
        assert_eq!(encode_standard(b"foo"), "Zm9v");
        assert_eq!(encode_standard(b"foob"), "Zm9vYg==");
        assert_eq!(encode_standard(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_standard(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn standard_uses_plus_and_slash_not_dash_and_underscore() {
        assert_eq!(encode_standard(&[0xFF, 0xFF, 0xFF]), "////");
        assert_eq!(encode_standard(&[0xF8]), "+A==");
    }

    #[test]
    fn url_safe_substitution() {
        assert_eq!(encode(&[0xFF, 0xFF, 0xFF]), "____");
        assert_eq!(encode(&[0xF8]), "-A");
    }
}
