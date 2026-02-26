pub(super) const HEADER_MIN_LEN: usize = 0x150;
pub(super) const ROM_ONLY: u8 = 0x00;
pub(super) const ROM_RAM: u8 = 0x08;
pub(super) const ROM_RAM_BATTERY: u8 = 0x09;
pub(super) const MBC1: u8 = 0x01;
pub(super) const MBC1_RAM: u8 = 0x02;
pub(super) const MBC1_RAM_BATTERY: u8 = 0x03;
pub(super) const MBC2: u8 = 0x05;
pub(super) const MBC2_BATTERY: u8 = 0x06;
pub(super) const MBC3_TIMER_BATTERY: u8 = 0x0F;
pub(super) const MBC3_TIMER_RAM_BATTERY: u8 = 0x10;
pub(super) const MBC3: u8 = 0x11;
pub(super) const MBC3_RAM: u8 = 0x12;
pub(super) const MBC3_RAM_BATTERY: u8 = 0x13;
pub(super) const MBC5: u8 = 0x19;
pub(super) const MBC5_RAM: u8 = 0x1A;
pub(super) const MBC5_RAM_BATTERY: u8 = 0x1B;
pub(super) const MBC5_RUMBLE: u8 = 0x1C;
pub(super) const MBC5_RUMBLE_RAM: u8 = 0x1D;
pub(super) const MBC5_RUMBLE_RAM_BATTERY: u8 = 0x1E;
pub(super) const ROM_BANK_BYTES: usize = 0x4000;
pub(super) const RAM_BANK_BYTES: usize = 0x2000;
pub(super) const MBC2_RAM_BYTES: usize = 512;
pub(super) const ROM_ONLY_ROM_BANK_COUNT: usize = 2;
pub(super) const HEADER_LOGO_START: usize = 0x0104;
pub(super) const HEADER_LOGO_END: usize = 0x0133;
pub(super) const HEADER_CHECKSUM_START: usize = 0x0134;
pub(super) const HEADER_CHECKSUM_END: usize = 0x014C;
pub(super) const HEADER_CHECKSUM_OFFSET: usize = 0x014D;
pub(super) const GLOBAL_CHECKSUM_HIGH_OFFSET: usize = 0x014E;
pub(super) const GLOBAL_CHECKSUM_LOW_OFFSET: usize = 0x014F;
pub(super) const NINTENDO_LOGO_BYTES: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];
