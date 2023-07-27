#[inline]
pub fn ascii_digit_to_u8(byte: u8) -> u8 {
    byte & 0xcf
}

