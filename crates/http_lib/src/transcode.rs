use std::fmt;

#[derive(Debug)]
pub struct TranscodeError;

impl fmt::Display for TranscodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "transcode error")
    }
}

impl std::error::Error for TranscodeError {}

#[inline]
pub fn ascii_digit_to_u8(byte: u8) -> u8 {
    byte & 0xcf
}

pub fn percent_decode(mut bytes: &[u8]) -> Result<Vec<u8>, TranscodeError> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut valid = 0;

    while valid < bytes.len() {
        if bytes[valid] != b'%' {
            valid += 1;
            continue;
        }

        let decode_start = valid + 1;
        let decode_end = valid + 3;
        if bytes.len() < decode_end {
            return Err(TranscodeError);
        }

        let bytes_to_decode = &bytes[decode_start..decode_end];
        let str_to_decode = std::str::from_utf8(bytes_to_decode).map_err(|_| TranscodeError)?;
        let decoded = u8::from_str_radix(str_to_decode, 16).map_err(|_| TranscodeError)?;
        out.extend_from_slice(&bytes[..valid]);
        out.push(decoded);
        bytes = &bytes[decode_end..];
        valid = 0;
    }

    out.extend_from_slice(bytes);
    Ok(out)
}
