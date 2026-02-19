use super::Bus;

impl Bus {
    pub fn read_byte(&self, addr: u16) -> u8 {
        if let Some(value) = self.blocked_read_value(addr) {
            return value;
        }

        self.read_byte_raw(addr)
    }
}
