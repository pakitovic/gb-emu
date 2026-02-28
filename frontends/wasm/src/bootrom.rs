use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = classifyBootRomFileName)]
pub fn classify_boot_rom_file_name(boot_rom_bytes: &[u8]) -> Option<String> {
    gb_runtime::bootrom::classify_known_boot_rom_file_name(boot_rom_bytes).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::classify_boot_rom_file_name;

    #[test]
    fn classify_boot_rom_file_name_returns_none_for_unknown_payload() {
        let mut boot = vec![0u8; 0x100];
        boot[0] = 0x01;
        assert!(classify_boot_rom_file_name(&boot).is_none());
    }
}
