#[macro_export]
macro_rules! byte_map {
    (for $identifier:ident; $condition:expr) => {{
        let mut char_map: [u8; 256] = [0; 256];
        let mut $identifier: u8 = 0;
        loop {
            char_map[$identifier as usize] = $condition;
            $identifier = $identifier.wrapping_add(1);
            if $identifier == 0 {
                break;
            }
        }

        char_map
    }};
}
