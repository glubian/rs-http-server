use bytes::Bytes;

pub trait Advance {
    fn advance_byte(&mut self, byte: u8) -> bool;
    fn advance_bytes(&mut self, bytes: &[u8]) -> bool;
    fn advance_while<F>(&mut self, predicate: F) -> usize
    where
        F: FnMut(&u8) -> bool;

    fn split_one_byte(&mut self) -> Option<u8>;
    fn split_while<F>(&mut self, predicate: F) -> Self
    where
        F: FnMut(&u8) -> bool;
}

impl Advance for Bytes {
    fn advance_byte(&mut self, byte: u8) -> bool {
        let advanced = self.first().is_some_and(|&b| b == byte);
        if advanced {
            *self = self.slice(1..);
        }

        advanced
    }

    fn advance_bytes(&mut self, bytes: &[u8]) -> bool {
        if self.starts_with(bytes) {
            *self = self.slice(bytes.len()..);
            true
        } else {
            false
        }
    }

    fn advance_while<F>(&mut self, predicate: F) -> usize
    where
        F: FnMut(&u8) -> bool,
    {
        let amt = self
            .iter()
            .copied()
            .take_while(predicate)
            .count();
        *self = self.slice(amt..);
        amt
    }

    fn split_one_byte(&mut self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            let res = self[0];
            *self = self.slice(1..);
            Some(res)
        }
    }

    fn split_while<F>(&mut self, predicate: F) -> Self
    where
        F: FnMut(&u8) -> bool 
    {
        let amt = self
            .iter()
            .copied()
            .take_while(predicate)
            .count();
        self.split_to(amt)
    }
}
