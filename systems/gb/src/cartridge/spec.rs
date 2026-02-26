use super::{
    CartridgeMapper, CartridgeSpec, MBC1, MBC1_RAM, MBC1_RAM_BATTERY, MBC2, MBC2_BATTERY, MBC3,
    MBC3_RAM, MBC3_RAM_BATTERY, MBC3_TIMER_BATTERY, MBC3_TIMER_RAM_BATTERY, MBC5, MBC5_RAM,
    MBC5_RAM_BATTERY, MBC5_RUMBLE, MBC5_RUMBLE_RAM, MBC5_RUMBLE_RAM_BATTERY, MapperType,
    ROM_BANK_BYTES, ROM_ONLY, ROM_RAM, ROM_RAM_BATTERY,
};

pub(super) fn cartridge_spec(cart_type: u8) -> Option<CartridgeSpec> {
    let spec = match cart_type {
        ROM_ONLY => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        ROM_RAM => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        ROM_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::RomOnly,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC1 => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC1_RAM => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC1_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc1,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC2 => CartridgeSpec {
            mapper: MapperType::Mbc2,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC2_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc2,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC3 => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC3_RAM => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC3_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        MBC3_TIMER_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: false,
            has_battery: true,
            has_timer: true,
        },
        MBC3_TIMER_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc3,
            has_ram: true,
            has_battery: true,
            has_timer: true,
        },
        MBC5 | MBC5_RUMBLE => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: false,
            has_battery: false,
            has_timer: false,
        },
        MBC5_RAM | MBC5_RUMBLE_RAM => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: false,
            has_timer: false,
        },
        MBC5_RAM_BATTERY | MBC5_RUMBLE_RAM_BATTERY => CartridgeSpec {
            mapper: MapperType::Mbc5,
            has_ram: true,
            has_battery: true,
            has_timer: false,
        },
        _ => return None,
    };
    Some(spec)
}

pub(super) fn mapper_uses_ram_gate(mapper: MapperType) -> bool {
    matches!(
        mapper,
        MapperType::Mbc1 | MapperType::Mbc2 | MapperType::Mbc3 | MapperType::Mbc5
    )
}

pub(super) fn public_mapper(mapper: MapperType) -> CartridgeMapper {
    match mapper {
        MapperType::RomOnly => CartridgeMapper::RomOnly,
        MapperType::Mbc1 => CartridgeMapper::Mbc1,
        MapperType::Mbc2 => CartridgeMapper::Mbc2,
        MapperType::Mbc3 => CartridgeMapper::Mbc3,
        MapperType::Mbc5 => CartridgeMapper::Mbc5,
    }
}

pub(super) fn is_mbc5_rumble_type(cart_type: u8) -> bool {
    matches!(
        cart_type,
        MBC5_RUMBLE | MBC5_RUMBLE_RAM | MBC5_RUMBLE_RAM_BATTERY
    )
}

pub(super) fn rom_size_bytes_from_code(code: u8) -> Option<usize> {
    let bytes = match code {
        0x00 => 2 * ROM_BANK_BYTES,
        0x01 => 4 * ROM_BANK_BYTES,
        0x02 => 8 * ROM_BANK_BYTES,
        0x03 => 16 * ROM_BANK_BYTES,
        0x04 => 32 * ROM_BANK_BYTES,
        0x05 => 64 * ROM_BANK_BYTES,
        0x06 => 128 * ROM_BANK_BYTES,
        0x07 => 256 * ROM_BANK_BYTES,
        0x08 => 512 * ROM_BANK_BYTES,
        0x52 => 72 * ROM_BANK_BYTES,
        0x53 => 80 * ROM_BANK_BYTES,
        0x54 => 96 * ROM_BANK_BYTES,
        _ => return None,
    };
    Some(bytes)
}

pub(super) fn ram_size_bytes_from_code(code: u8) -> Option<usize> {
    let bytes = match code {
        0x00 => 0,
        0x01 => 2 * 1024,
        0x02 => 8 * 1024,
        0x03 => 32 * 1024,
        0x04 => 128 * 1024,
        0x05 => 64 * 1024,
        _ => return None,
    };
    Some(bytes)
}
