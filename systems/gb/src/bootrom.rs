pub const BOOT_ROM_WINDOW_SIZE: usize = 0x0100;
pub type BootRomData = [u8; BOOT_ROM_WINDOW_SIZE];

pub fn parse_boot_rom_prefix(bytes: &[u8]) -> Option<BootRomData> {
    if bytes.len() < BOOT_ROM_WINDOW_SIZE {
        return None;
    }

    let mut boot_rom = [0; BOOT_ROM_WINDOW_SIZE];
    boot_rom.copy_from_slice(&bytes[..BOOT_ROM_WINDOW_SIZE]);
    Some(boot_rom)
}

#[cfg(test)]
mod tests {
    use super::{BOOT_ROM_WINDOW_SIZE, parse_boot_rom_prefix};

    #[test]
    fn parse_boot_rom_prefix_requires_full_window() {
        assert!(parse_boot_rom_prefix(&vec![0x00; BOOT_ROM_WINDOW_SIZE - 1]).is_none());
    }

    #[test]
    fn parse_boot_rom_prefix_copies_first_256_bytes() {
        let bytes = vec![0xAB; BOOT_ROM_WINDOW_SIZE + 8];
        let parsed = parse_boot_rom_prefix(&bytes).expect("256-byte prefix should parse");
        assert!(parsed.iter().all(|byte| *byte == 0xAB));
    }
}
