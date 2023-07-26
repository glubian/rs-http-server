use crate::byte_map;

pub const CRLF: &[u8] = b"\r\n";

pub const URI_MAP: [u8; 256] = byte_map!(
    for c; match c {
        c if c.is_ascii_alphanumeric() => c,
        b':' | b'|' | b'?' | b'#' | b'[' | b']' |
        b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' |
        b')' | b'*' | b'+' | b',' | b';' | b'=' |
        b'%' | b'-' | b'.' | b'_' | b'~' | b'/' => c,
        _ => 0,
    }
);

pub const TCHAR_MAP: [u8; 256] = byte_map!(
    for c; match c {
        c if c.is_ascii_alphanumeric() => c,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' |
        b'*' | b'+' | b'-' | b'.' | b'^' | b'_' |
        b'`' | b'|' | b'~' => c,
        _ => 0,
    }
);

pub const TOKEN_MAP: [u8; 256] = byte_map!(
    for c; match c {
        c if TCHAR_MAP[c as usize] != 0 => c,
        // delimiters, see https://httpwg.org/specs/rfc9110.html#tokens
        // excluding ',' meant to be used as a list separator,
        // '"' and '(' used for quoted strings and comments respectively
        // including ' '
        b')' | b'/' | b':' | b';' | b'<' | b'=' |
        b'>' | b'?' | b'@' | b'[' | b']' | b'{' |
        b'}' | b' ' => c,
        _ => 0,
    }
);

pub const DATE_MAP: [u8; 256] = byte_map!(
    for c; match c {
        c if c.is_ascii_alphanumeric() => c,
        // treat date as a singular value
        b' ' | b',' | b':' => c,
        _ => 0,
    }
);

pub const CTEXT_MAP: [u8; 256] = byte_map!(
    for c; match c {
        b'\t' | b' ' | b'!'..=b'\'' | b'*'..=b'[' | b']'..=b'~' | 0x80.. => c,
        _ => 0,
    }
);

pub const QUOTED_TEXT_MAP: [u8; 256] = byte_map!(
    for c; match c {
        b'\t' | b' ' | 0x21 | 0x23..=0x5b | 0x5d..=0x7e | 0x80.. => c,
        _ => 0,
    }
);
