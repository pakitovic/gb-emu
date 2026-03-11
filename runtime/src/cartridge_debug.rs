use gb_emu::cartridge::CartridgeMetadata;

pub fn format_cartridge_debug_report(metadata: &CartridgeMetadata) -> String {
    let mut lines = Vec::with_capacity(12 + metadata.header_warnings.len());
    let title = if metadata.title.trim().is_empty() {
        "<empty title>".to_string()
    } else {
        metadata.title.clone()
    };
    lines.push("Cartridge Metadata".to_string());
    lines.push(format!("Title: {title}"));
    lines.push(format!("Header CRC32: 0x{:08X}", metadata.header_crc32));
    lines.push(format!(
        "Type: 0x{:02X} ({})",
        metadata.cart_type_code, metadata.mapper
    ));
    lines.push(format!(
        "ROM: code 0x{:02X}, {} bytes, {} banks",
        metadata.rom_size_code, metadata.rom_size_bytes, metadata.rom_bank_count
    ));
    lines.push(format!(
        "RAM: code 0x{:02X}, declared {} bytes, effective {} bytes, {} banks",
        metadata.ram_size_code,
        metadata.declared_ram_size_bytes,
        metadata.effective_ram_size_bytes,
        metadata.ram_bank_count
    ));
    lines.push(format!(
        "Compatibility RAM mode: {}",
        yes_no(metadata.compatibility_ram_mode)
    ));
    lines.push(format!(
        "Capabilities: battery={}, timer={}, rumble={} (active={}), battery-save={}",
        yes_no(metadata.has_battery),
        yes_no(metadata.has_timer),
        yes_no(metadata.has_rumble),
        yes_no(metadata.rumble_active),
        yes_no(metadata.has_battery_save)
    ));
    lines.push(format!(
        "Header warnings ({}):",
        metadata.header_warnings.len()
    ));
    if metadata.header_warnings.is_empty() {
        lines.push("- none".to_string());
    } else {
        for warning in &metadata.header_warnings {
            lines.push(format!("- {warning}"));
        }
    }
    lines.join("\n")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::format_cartridge_debug_report;
    use gb_emu::cartridge::{CartridgeHeaderWarning, CartridgeMapper, CartridgeMetadata};

    #[test]
    fn cartridge_debug_report_formats_core_fields_and_warnings() {
        let metadata = CartridgeMetadata {
            title: "TESTROM".to_string(),
            header_crc32: 0x1234ABCD,
            cart_type_code: 0x03,
            mapper: CartridgeMapper::Mbc1,
            rom_size_code: 0x01,
            ram_size_code: 0x03,
            rom_size_bytes: 64 * 1024,
            rom_bank_count: 4,
            declared_ram_size_bytes: 32 * 1024,
            effective_ram_size_bytes: 32 * 1024,
            ram_bank_count: 4,
            compatibility_ram_mode: false,
            has_battery: true,
            has_timer: false,
            has_rumble: false,
            has_battery_save: true,
            rumble_active: false,
            header_warnings: vec![
                CartridgeHeaderWarning::NintendoLogoMismatch,
                CartridgeHeaderWarning::HeaderChecksumMismatch {
                    header_value: 0xAA,
                    computed_value: 0xBB,
                },
            ],
        };

        let report = format_cartridge_debug_report(&metadata);
        assert!(report.contains("Cartridge Metadata"));
        assert!(report.contains("Title: TESTROM"));
        assert!(report.contains("Header CRC32: 0x1234ABCD"));
        assert!(report.contains("Type: 0x03 (MBC1)"));
        assert!(report.contains("Header warnings (2):"));
        assert!(report.contains("- Nintendo logo mismatch"));
        assert!(report.contains("- Header checksum mismatch (header 0xAA, computed 0xBB)"));
    }

    #[test]
    fn cartridge_debug_report_marks_empty_warning_list() {
        let metadata = CartridgeMetadata {
            title: String::new(),
            header_crc32: 0,
            cart_type_code: 0x00,
            mapper: CartridgeMapper::RomOnly,
            rom_size_code: 0x00,
            ram_size_code: 0x00,
            rom_size_bytes: 32 * 1024,
            rom_bank_count: 2,
            declared_ram_size_bytes: 0,
            effective_ram_size_bytes: 8 * 1024,
            ram_bank_count: 1,
            compatibility_ram_mode: true,
            has_battery: false,
            has_timer: false,
            has_rumble: false,
            has_battery_save: false,
            rumble_active: false,
            header_warnings: Vec::new(),
        };

        let report = format_cartridge_debug_report(&metadata);
        assert!(report.contains("Title: <empty title>"));
        assert!(report.contains("Header warnings (0):"));
        assert!(report.contains("- none"));
    }
}
