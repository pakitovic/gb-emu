use super::{
    Cartridge, CartridgeError, FixedRtcClock, HEADER_MIN_LEN, MBC2_RAM_BYTES, MapperType, Mbc3Rtc,
    RAM_BANK_BYTES, ROM_BANK_BYTES, ROM_ONLY_ROM_BANK_COUNT, RtcClock, SystemRtcClock,
    cartridge_spec, diagnose_header, is_mbc5_rumble_type, mapper_uses_ram_gate, parse_title,
    ram_size_bytes_from_code, rom_size_bytes_from_code,
};

impl Cartridge {
    pub fn from_bytes(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        Self::from_bytes_with_clock(rom, Box::new(SystemRtcClock))
    }

    pub fn from_bytes_with_initial_rtc_epoch(
        rom: Vec<u8>,
        rtc_epoch_secs: u64,
    ) -> Result<Self, CartridgeError> {
        Self::from_bytes_with_clock(rom, Box::new(FixedRtcClock::new(rtc_epoch_secs)))
    }

    pub(super) fn from_bytes_with_clock(
        rom: Vec<u8>,
        clock: Box<dyn RtcClock>,
    ) -> Result<Self, CartridgeError> {
        if rom.len() < HEADER_MIN_LEN {
            return Err(CartridgeError::RomTooSmall { actual: rom.len() });
        }

        let cart_type = rom[0x0147];
        let Some(spec) = cartridge_spec(cart_type) else {
            return Err(CartridgeError::UnsupportedCartridgeType(cart_type));
        };
        let has_rumble = is_mbc5_rumble_type(cart_type);

        let rom_size_code = rom[0x0148];
        let Some(expected_len) = rom_size_bytes_from_code(rom_size_code) else {
            return Err(CartridgeError::UnsupportedRomSizeCode(rom_size_code));
        };

        if rom.len() != expected_len {
            return Err(CartridgeError::UnsupportedRomLength {
                expected: expected_len,
                actual: rom.len(),
            });
        }

        let rom_bank_count = (rom.len() / ROM_BANK_BYTES).max(1);
        if spec.mapper == MapperType::RomOnly && rom_bank_count != ROM_ONLY_ROM_BANK_COUNT {
            return Err(CartridgeError::UnsupportedRomSizeForCartridge {
                cart_type,
                rom_size_code,
            });
        }

        let ram_size_code = rom[0x0149];
        let Some(ram_size_bytes) = ram_size_bytes_from_code(ram_size_code) else {
            return Err(CartridgeError::UnsupportedRamSizeCode(ram_size_code));
        };
        if spec.mapper == MapperType::Mbc2 && ram_size_code != 0x00 {
            return Err(CartridgeError::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            });
        }
        if !spec.has_ram && ram_size_bytes != 0 {
            return Err(CartridgeError::UnsupportedRamSizeForCartridge {
                cart_type,
                ram_size_code,
            });
        }

        let title = parse_title(&rom);
        let header_warnings = diagnose_header(&rom);
        // Keep a transient 8KB RAM window for ROM-only homebrew/test ROM protocols
        // that write pass/fail signatures into A000-BFFF without declaring cartridge RAM.
        let effective_ram_size = if spec.mapper == MapperType::Mbc2 {
            MBC2_RAM_BYTES
        } else if spec.has_ram || spec.mapper == MapperType::RomOnly {
            if ram_size_bytes > 0 {
                ram_size_bytes
            } else {
                RAM_BANK_BYTES
            }
        } else {
            0
        };
        let compatibility_ram =
            effective_ram_size > 0 && ram_size_bytes == 0 && spec.mapper != MapperType::Mbc2;
        let ram = if effective_ram_size > 0 {
            vec![0; effective_ram_size]
        } else {
            Vec::new()
        };
        let ram_bank_count = if ram.is_empty() {
            0
        } else {
            ram.len().div_ceil(RAM_BANK_BYTES)
        };
        let rtc = if spec.has_timer {
            Some(Mbc3Rtc::new(clock.now_epoch_secs()))
        } else {
            None
        };

        Ok(Self {
            rom,
            title,
            cart_type_code: cart_type,
            rom_size_code,
            ram_size_code,
            declared_ram_size_bytes: ram_size_bytes,
            compatibility_ram_mode: compatibility_ram,
            header_warnings,
            mapper: spec.mapper,
            rom_bank_count,
            ram,
            ram_bank_count,
            has_battery: spec.has_battery,
            has_timer: spec.has_timer,
            has_rumble,
            rumble_active: false,
            clock,
            host_rtc_epoch_secs: None,
            save_dirty: false,
            ram_enable_required: mapper_uses_ram_gate(spec.mapper) && !compatibility_ram,
            ram_enabled: !mapper_uses_ram_gate(spec.mapper) || compatibility_ram,
            mbc1_rom_bank_low5: 1,
            mbc1_bank_high2: 0,
            mbc1_mode: 0,
            mbc2_rom_bank_low4: 1,
            mbc3_rom_bank_low7: 1,
            mbc3_ram_bank_or_rtc: 0,
            rtc,
            mbc5_rom_bank: 1,
            mbc5_ram_bank: 0,
        })
    }
}
