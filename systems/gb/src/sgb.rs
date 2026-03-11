use crate::memory::{LCD_FRAME_PIXELS, LCD_HEIGHT, LCD_WIDTH};
use crate::palette_db::SGB_HEADER_OVERRIDES;
use crate::palette_override::PaletteOverrideDb;
use crate::video::canonical_dmg_shade_id_for_luma;

const SGB_PACKET_BITS: usize = 128;
const SGB_PACKET_BYTES: usize = SGB_PACKET_BITS / 8;
const SGB_MAX_PACKETS: usize = 7;
const SGB_TILE_WIDTH: usize = 20;
const SGB_TILE_HEIGHT: usize = 18;
const SGB_ATTR_MAP_SIZE: usize = SGB_TILE_WIDTH * SGB_TILE_HEIGHT;
const SGB_ATTR_FILE_COUNT: usize = 45;
const SGB_ATTR_FILE_BYTES: usize = 90;
const SGB_ATTR_TRANSFER_BYTES: usize = 4096;
const SGB_SYSTEM_PALETTE_COUNT: usize = 512;
const SGB_BORDER_WIDTH: usize = 256;
const SGB_BORDER_HEIGHT: usize = 224;
const SGB_BORDER_TILEMAP_WIDTH: usize = 32;
const SGB_BORDER_TILEMAP_HEIGHT: usize = 28;
const SGB_BORDER_TILEMAP_VISIBLE_ENTRIES: usize =
    SGB_BORDER_TILEMAP_WIDTH * SGB_BORDER_TILEMAP_HEIGHT;
const SGB_BORDER_TILE_BYTES: usize = 32;
const SGB_BORDER_TILE_COUNT: usize = 256;
const SGB_BORDER_PALETTE_COLORS: usize = 16;
const SGB_BORDER_PALETTE_COUNT: usize = 3;
const SGB_OBJ_TILE_TRANSFER_BYTES: usize = 0x1000;
const SGB_OBJ_TILE_BYTES: usize = 32;
const SGB_OBJ_TILE_BLOCK_COUNT: usize = SGB_OBJ_TILE_TRANSFER_BYTES / SGB_OBJ_TILE_BYTES;
const SGB_OBJ_TILE_COUNT: usize = SGB_OBJ_TILE_BLOCK_COUNT * 2;
const SGB_OBJ_PALETTE_COUNT: usize = 4;
const SGB_OBJ_PALETTE_COLORS: usize = 16;
const SGB_OBJ_OAM_TRANSFER_BASE: usize = 0x0F90;
const SGB_OBJ_ENTRY_BYTES: usize = 4;
const SGB_OBJ_COUNT: usize = 24;
const SGB_OBJ_ATTRIBUTE_BYTES: usize = SGB_OBJ_COUNT * SGB_OBJ_ENTRY_BYTES;
const SGB_OBJ_ATTRIBUTE_EXTENSION_BASE: usize = SGB_OBJ_OAM_TRANSFER_BASE + SGB_OBJ_ATTRIBUTE_BYTES;
const SGB_OBJ_TILE_STRIDE_PER_ROW: usize = 16;

const P1_SELECT_MASK: u8 = 0x30;
const P1_RESET: u8 = 0x00;
const P1_BIT_ZERO: u8 = 0x20;
const P1_BIT_ONE: u8 = 0x10;
const P1_IDLE: u8 = 0x30;

pub const CMD_PAL01: u8 = 0x00;
pub const CMD_PAL23: u8 = 0x01;
pub const CMD_PAL03: u8 = 0x02;
pub const CMD_PAL12: u8 = 0x03;
pub const CMD_ATTR_BLK: u8 = 0x04;
pub const CMD_ATTR_LIN: u8 = 0x05;
pub const CMD_ATTR_DIV: u8 = 0x06;
pub const CMD_ATTR_CHR: u8 = 0x07;
pub const CMD_PAL_SET: u8 = 0x0A;
pub const CMD_PAL_TRN: u8 = 0x0B;
pub const CMD_ATRC_EN: u8 = 0x0C;
pub const CMD_TEST_EN: u8 = 0x0D;
pub const CMD_ICON_EN: u8 = 0x0E;
pub const CMD_DATA_SND: u8 = 0x0F;
pub const CMD_DATA_TRN: u8 = 0x10;
pub const CMD_MLT_REQ: u8 = 0x11;
pub const CMD_JUMP: u8 = 0x12;
pub const CMD_CHR_TRN: u8 = 0x13;
pub const CMD_PCT_TRN: u8 = 0x14;
pub const CMD_ATTR_TRN: u8 = 0x15;
pub const CMD_ATTR_SET: u8 = 0x16;
pub const CMD_MASK_EN: u8 = 0x17;
pub const CMD_OBJ_TRN: u8 = 0x18;
pub const CMD_PAL_PRI: u8 = 0x19;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgbCommand {
    pub command_id: u8,
    pub packet_count: u8,
    pub bytes: Vec<u8>,
}

impl SgbCommand {
    pub fn payload_bytes(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        for packet in self.bytes.chunks_exact(SGB_PACKET_BYTES) {
            payload.extend_from_slice(&packet[1..]);
        }
        payload
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SgbPalette {
    pub colors: [u16; 4],
}

impl Default for SgbPalette {
    fn default() -> Self {
        Self {
            colors: [0x7FFF, 0x5294, 0x294A, 0x0000],
        }
    }
}

const SGB_BUILT_IN_BOOT_PALETTES: [[u16; 4]; 32] = [
    [0x67BF, 0x265B, 0x10B5, 0x2866],
    [0x637B, 0x3AD9, 0x0956, 0x0000],
    [0x7F1F, 0x2A7D, 0x30F3, 0x4CE7],
    [0x57FF, 0x2618, 0x001F, 0x006A],
    [0x5B7F, 0x3F0F, 0x222D, 0x10EB],
    [0x7FBB, 0x2A3C, 0x0015, 0x0900],
    [0x2800, 0x7680, 0x01EF, 0x2FFF],
    [0x73BF, 0x46FF, 0x0110, 0x0066],
    [0x533E, 0x2638, 0x01E5, 0x0000],
    [0x7FFF, 0x2BBF, 0x00DF, 0x2C0A],
    [0x7F1F, 0x463D, 0x74CF, 0x4CA5],
    [0x53FF, 0x03E0, 0x00DF, 0x2800],
    [0x433F, 0x72D2, 0x3045, 0x0822],
    [0x7FFA, 0x2A5F, 0x0014, 0x0003],
    [0x1EED, 0x215C, 0x42FC, 0x0060],
    [0x7FFF, 0x5EF7, 0x39CE, 0x0000],
    [0x4F5F, 0x630E, 0x159F, 0x3126],
    [0x637B, 0x121C, 0x0140, 0x0840],
    [0x66BC, 0x3FFF, 0x7EE0, 0x2C84],
    [0x5FFE, 0x3EBC, 0x0321, 0x0000],
    [0x63FF, 0x36DC, 0x11F6, 0x392A],
    [0x65EF, 0x7DBF, 0x035F, 0x2108],
    [0x2B6C, 0x7FFF, 0x1CD9, 0x0007],
    [0x53FC, 0x1F2F, 0x0E29, 0x0061],
    [0x36BE, 0x7EAF, 0x681A, 0x3C00],
    [0x7BBE, 0x329D, 0x1DE8, 0x0423],
    [0x739F, 0x6A9B, 0x7293, 0x0001],
    [0x5FFF, 0x6732, 0x3DA9, 0x2481],
    [0x577F, 0x3EBC, 0x456F, 0x1880],
    [0x6B57, 0x6E1B, 0x5010, 0x0007],
    [0x0F96, 0x2C97, 0x0045, 0x3200],
    [0x67FF, 0x2F17, 0x2230, 0x1548],
];

// HLE copy of the built-in SGB BIOS title-to-palette assignments for
// monochrome carts that do not declare SGB support in the header.
// The table matches the current SameBoy and bsnes SGB HLE references.
const SGB_BUILT_IN_BOOT_PALETTE_ASSIGNMENTS: [(&str, usize); 26] = [
    ("ZELDA", 4),
    ("SUPER MARIOLAND", 5),
    ("MARIOLAND2", 19),
    ("SUPERMARIOLAND3", 1),
    ("KIRBY DREAM LAND", 10),
    ("HOSHINOKA-BI", 10),
    ("KIRBY'S PINBALL", 2),
    ("YOSSY NO TAMAGO", 11),
    ("MARIO & YOSHI", 11),
    ("YOSSY NO COOKIE", 3),
    ("YOSHI'S COOKIE", 3),
    ("DR.MARIO", 17),
    ("TETRIS", 16),
    ("YAKUMAN", 18),
    ("METROID2", 30),
    ("KAERUNOTAMENI", 8),
    ("GOLF", 23),
    ("ALLEY WAY", 21),
    ("BASEBALL", 14),
    ("TENNIS", 22),
    ("F1RACE", 29),
    ("KID ICARUS", 13),
    ("QIX", 24),
    ("SOLARSTRIKER", 6),
    ("X", 27),
    ("GBWARS", 20),
];

fn sgb_boot_palette_at(index: usize) -> SgbPalette {
    let colors = SGB_BUILT_IN_BOOT_PALETTES
        .get(index)
        .copied()
        .unwrap_or(SGB_BUILT_IN_BOOT_PALETTES[0]);
    SgbPalette { colors }
}

pub fn sgb_boot_palette_for_header_crc32(header_crc32: u32) -> Option<SgbPalette> {
    SGB_HEADER_OVERRIDES
        .iter()
        .find_map(|(candidate_crc32, palette_index)| {
            (*candidate_crc32 == header_crc32).then_some(sgb_boot_palette_at(*palette_index))
        })
}

pub fn sgb_boot_palette_for_title(title: &str) -> SgbPalette {
    let palette_index = SGB_BUILT_IN_BOOT_PALETTE_ASSIGNMENTS
        .iter()
        .find_map(|(candidate, palette_index)| (*candidate == title).then_some(*palette_index))
        .unwrap_or(0);
    sgb_boot_palette_at(palette_index)
}

pub fn sgb_boot_palette_for_cartridge(title: &str, header_crc32: u32) -> SgbPalette {
    sgb_boot_palette_for_header_crc32(header_crc32)
        .unwrap_or_else(|| sgb_boot_palette_for_title(title))
}

pub fn sgb_boot_palette_for_cartridge_with_overrides(
    title: &str,
    header_crc32: u32,
    overrides: Option<&PaletteOverrideDb>,
) -> SgbPalette {
    let base = sgb_boot_palette_for_cartridge(title, header_crc32);
    let Some(overrides) = overrides else {
        return base;
    };
    let Some(merged_rgb) =
        overrides.merged_sgb_boot_palette_rgb(header_crc32, base.colors.map(bgr555_to_rgb888))
    else {
        return base;
    };

    SgbPalette {
        colors: merged_rgb.map(rgb888_to_bgr555),
    }
}

pub fn sgb_has_title_boot_palette(title: &str) -> bool {
    SGB_BUILT_IN_BOOT_PALETTE_ASSIGNMENTS
        .iter()
        .any(|(candidate, _)| *candidate == title)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbPalSetState {
    pub palette_indices: [u16; 4],
    pub apply_attr_file: bool,
    pub attr_file_index: u8,
    pub mask_freeze_cancel: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SgbObjEntry {
    x: u16,
    y: u8,
    tile_number: u8,
    tile_table_high: bool,
    palette_number: u8,
    priority: u8,
    size_large: bool,
    x_flip: bool,
    y_flip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SgbObjControl {
    enabled: bool,
    change_palettes: bool,
    palette_indices: [u16; SGB_OBJ_PALETTE_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbAtrcState {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbTestState {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbIconState {
    pub mode: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SgbDataSendState {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgbDataTransferState {
    pub destination: u16,
    pub data: [u8; SGB_ATTR_TRANSFER_BYTES],
    pub loaded: bool,
}

impl Default for SgbDataTransferState {
    fn default() -> Self {
        Self {
            destination: 0,
            data: [0; SGB_ATTR_TRANSFER_BYTES],
            loaded: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbJumpState {
    pub target: u16,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SgbPalPriState {
    pub enabled: bool,
}

#[derive(Debug, Clone)]
struct SgbObjState {
    control: SgbObjControl,
    tiles: [[u8; SGB_OBJ_TILE_BYTES]; SGB_OBJ_TILE_COUNT],
    palettes: [[u16; SGB_OBJ_PALETTE_COLORS]; SGB_OBJ_PALETTE_COUNT],
    entries: [SgbObjEntry; SGB_OBJ_COUNT],
    data_loaded: bool,
    palettes_loaded: bool,
}

impl Default for SgbObjState {
    fn default() -> Self {
        Self {
            control: SgbObjControl::default(),
            tiles: [[0; SGB_OBJ_TILE_BYTES]; SGB_OBJ_TILE_COUNT],
            palettes: [[0; SGB_OBJ_PALETTE_COLORS]; SGB_OBJ_PALETTE_COUNT],
            entries: [SgbObjEntry::default(); SGB_OBJ_COUNT],
            data_loaded: false,
            palettes_loaded: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SgbState {
    backdrop_color: u16,
    gb_palettes: [SgbPalette; 4],
    system_palettes: [SgbPalette; SGB_SYSTEM_PALETTE_COUNT],
    system_palettes_loaded: bool,
    attr_map: [u8; SGB_ATTR_MAP_SIZE],
    border_tiles: [[u8; SGB_BORDER_TILE_BYTES]; SGB_BORDER_TILE_COUNT],
    border_tilemap: [u16; SGB_BORDER_TILEMAP_VISIBLE_ENTRIES],
    border_palettes: [[u16; SGB_BORDER_PALETTE_COLORS]; SGB_BORDER_PALETTE_COUNT],
    border_chr_low_loaded: bool,
    border_chr_high_loaded: bool,
    border_pct_loaded: bool,
    attr_files: [[u8; SGB_ATTR_FILE_BYTES]; SGB_ATTR_FILE_COUNT],
    attr_files_loaded: [bool; SGB_ATTR_FILE_COUNT],
    pal_set: SgbPalSetState,
    mask_mode: u8,
    obj: SgbObjState,
    atrc: SgbAtrcState,
    test: SgbTestState,
    icon: SgbIconState,
    data_send: SgbDataSendState,
    data_transfer: SgbDataTransferState,
    jump: SgbJumpState,
    pal_pri: SgbPalPriState,
    last_applied_command_id: Option<u8>,
}

impl Default for SgbState {
    fn default() -> Self {
        Self {
            backdrop_color: SgbPalette::default().colors[0],
            gb_palettes: [SgbPalette::default(); 4],
            system_palettes: [SgbPalette::default(); SGB_SYSTEM_PALETTE_COUNT],
            system_palettes_loaded: false,
            attr_map: [0; SGB_ATTR_MAP_SIZE],
            border_tiles: [[0; SGB_BORDER_TILE_BYTES]; SGB_BORDER_TILE_COUNT],
            border_tilemap: [0; SGB_BORDER_TILEMAP_VISIBLE_ENTRIES],
            border_palettes: [[0; SGB_BORDER_PALETTE_COLORS]; SGB_BORDER_PALETTE_COUNT],
            border_chr_low_loaded: false,
            border_chr_high_loaded: false,
            border_pct_loaded: false,
            attr_files: [[0; SGB_ATTR_FILE_BYTES]; SGB_ATTR_FILE_COUNT],
            attr_files_loaded: [false; SGB_ATTR_FILE_COUNT],
            pal_set: SgbPalSetState::default(),
            mask_mode: 0,
            obj: SgbObjState::default(),
            atrc: SgbAtrcState::default(),
            test: SgbTestState::default(),
            icon: SgbIconState::default(),
            data_send: SgbDataSendState::default(),
            data_transfer: SgbDataTransferState::default(),
            jump: SgbJumpState::default(),
            pal_pri: SgbPalPriState::default(),
            last_applied_command_id: None,
        }
    }
}

impl SgbState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gb_palettes(&self) -> &[SgbPalette; 4] {
        &self.gb_palettes
    }

    pub fn backdrop_color(&self) -> u16 {
        self.backdrop_color
    }

    pub fn pal_set_state(&self) -> SgbPalSetState {
        self.pal_set
    }

    pub fn attr_map(&self) -> &[u8; SGB_ATTR_MAP_SIZE] {
        &self.attr_map
    }

    pub fn palette_index_for_tile(&self, tile_x: usize, tile_y: usize) -> u8 {
        if tile_x >= SGB_TILE_WIDTH || tile_y >= SGB_TILE_HEIGHT {
            return 0;
        }
        self.attr_map[tile_y * SGB_TILE_WIDTH + tile_x]
    }

    pub fn mask_mode(&self) -> u8 {
        self.mask_mode
    }

    pub fn last_applied_command_id(&self) -> Option<u8> {
        self.last_applied_command_id
    }

    pub fn atrc_state(&self) -> SgbAtrcState {
        self.atrc
    }

    pub fn test_state(&self) -> SgbTestState {
        self.test
    }

    pub fn icon_state(&self) -> SgbIconState {
        self.icon
    }

    pub fn data_send_state(&self) -> &SgbDataSendState {
        &self.data_send
    }

    pub fn data_transfer_state(&self) -> &SgbDataTransferState {
        &self.data_transfer
    }

    pub fn jump_state(&self) -> SgbJumpState {
        self.jump
    }

    pub fn pal_pri_state(&self) -> SgbPalPriState {
        self.pal_pri
    }

    pub fn has_presented_overlay(&self) -> bool {
        self.has_border_data() || self.has_obj_overlay()
    }

    pub fn apply_built_in_boot_palette(
        &mut self,
        title: &str,
        header_crc32: u32,
        overrides: Option<&PaletteOverrideDb>,
    ) -> bool {
        self.gb_palettes[0] =
            sgb_boot_palette_for_cartridge_with_overrides(title, header_crc32, overrides);
        self.backdrop_color = self.gb_palettes[0].colors[0];
        self.sync_shared_color0();
        self.attr_map.fill(0);
        sgb_boot_palette_for_header_crc32(header_crc32).is_some()
            || sgb_has_title_boot_palette(title)
    }

    pub fn apply_command(&mut self, command: &SgbCommand) {
        let payload = command.payload_bytes();
        match command.command_id {
            CMD_PAL01 => self.apply_two_palette_command(&payload, 0, 1),
            CMD_PAL23 => self.apply_two_palette_command(&payload, 2, 3),
            CMD_PAL03 => self.apply_two_palette_command(&payload, 0, 3),
            CMD_PAL12 => self.apply_two_palette_command(&payload, 1, 2),
            CMD_ATTR_BLK => self.apply_attr_blk(&payload),
            CMD_ATTR_LIN => self.apply_attr_lin(&payload),
            CMD_ATTR_DIV => self.apply_attr_div(&payload),
            CMD_ATTR_CHR => self.apply_attr_chr(&payload),
            CMD_PAL_SET => self.apply_pal_set(&payload),
            CMD_ATRC_EN => self.apply_atrc_en(&payload),
            CMD_TEST_EN => self.apply_test_en(&payload),
            CMD_ICON_EN => self.apply_icon_en(&payload),
            CMD_DATA_SND => self.apply_data_snd(&payload),
            CMD_DATA_TRN => self.apply_data_trn(&payload),
            CMD_JUMP => self.apply_jump(&payload),
            CMD_ATTR_SET => self.apply_attr_set(&payload),
            CMD_CHR_TRN | CMD_PCT_TRN | CMD_PAL_TRN | CMD_ATTR_TRN => {}
            CMD_MASK_EN => self.apply_mask_en(&payload),
            CMD_OBJ_TRN => self.apply_obj_trn(&payload),
            CMD_PAL_PRI => self.apply_pal_pri(&payload),
            _ => {}
        }
        self.last_applied_command_id = Some(command.command_id);
    }

    pub fn load_attr_files_from_vram_transfer(&mut self, transfer: &[u8]) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        for file_index in 0..SGB_ATTR_FILE_COUNT {
            let src_start = file_index * SGB_ATTR_FILE_BYTES;
            let src_end = src_start + SGB_ATTR_FILE_BYTES;
            self.attr_files[file_index].copy_from_slice(&transfer[src_start..src_end]);
            self.attr_files_loaded[file_index] = true;
        }

        true
    }

    pub fn load_system_palettes_from_vram_transfer(&mut self, transfer: &[u8]) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        for palette_index in 0..SGB_SYSTEM_PALETTE_COUNT {
            let src_start = palette_index * 8;
            for color_index in 0..4 {
                let lo = transfer[src_start + color_index * 2] as u16;
                let hi = transfer[src_start + color_index * 2 + 1] as u16;
                self.system_palettes[palette_index].colors[color_index] = lo | (hi << 8);
            }
        }
        self.system_palettes_loaded = true;
        for (target, palette_id) in self
            .gb_palettes
            .iter_mut()
            .zip(self.pal_set.palette_indices.iter().copied())
        {
            if (palette_id as usize) < SGB_SYSTEM_PALETTE_COUNT {
                *target = self.system_palettes[palette_id as usize];
            }
        }
        self.backdrop_color = self.gb_palettes[0].colors[0];
        self.sync_shared_color0();
        self.refresh_obj_palettes_from_system_palettes();
        true
    }

    pub fn load_border_chr_from_vram_transfer(
        &mut self,
        transfer: &[u8],
        high_tile_block: bool,
    ) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        let base = if high_tile_block { 128 } else { 0 };
        for tile_index in 0..128 {
            let src_start = tile_index * SGB_BORDER_TILE_BYTES;
            let src_end = src_start + SGB_BORDER_TILE_BYTES;
            self.border_tiles[base + tile_index].copy_from_slice(&transfer[src_start..src_end]);
        }

        if high_tile_block {
            self.border_chr_high_loaded = true;
        } else {
            self.border_chr_low_loaded = true;
        }
        true
    }

    pub fn load_obj_chr_from_vram_transfer(
        &mut self,
        transfer: &[u8],
        high_tile_block: bool,
    ) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        let base = if high_tile_block { 128 } else { 0 };
        for tile_index in 0..SGB_OBJ_TILE_BLOCK_COUNT {
            let src_start = tile_index * SGB_OBJ_TILE_BYTES;
            let src_end = src_start + SGB_OBJ_TILE_BYTES;
            self.obj.tiles[base + tile_index].copy_from_slice(&transfer[src_start..src_end]);
        }
        self.obj.data_loaded = true;
        true
    }

    pub fn load_border_pct_from_vram_transfer(&mut self, transfer: &[u8]) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        for entry_index in 0..SGB_BORDER_TILEMAP_VISIBLE_ENTRIES {
            let src = entry_index * 2;
            self.border_tilemap[entry_index] =
                u16::from_le_bytes([transfer[src], transfer[src + 1]]);
        }

        let palette_base = 0x0800;
        for palette in 0..SGB_BORDER_PALETTE_COUNT {
            for color in 0..SGB_BORDER_PALETTE_COLORS {
                let src = palette_base + (palette * SGB_BORDER_PALETTE_COLORS + color) * 2;
                self.border_palettes[palette][color] =
                    u16::from_le_bytes([transfer[src], transfer[src + 1]]);
            }
        }

        self.backdrop_color = self.border_palettes[SGB_BORDER_PALETTE_COUNT - 1][0];
        self.sync_shared_color0();

        self.border_pct_loaded = true;
        true
    }

    pub fn has_border_data(&self) -> bool {
        self.border_pct_loaded && (self.border_chr_low_loaded || self.border_chr_high_loaded)
    }

    pub fn load_obj_from_vram_transfer(&mut self, transfer: &[u8]) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        for entry_index in 0..SGB_OBJ_COUNT {
            let src = SGB_OBJ_OAM_TRANSFER_BASE + entry_index * SGB_OBJ_ENTRY_BYTES;
            let attrs = transfer[src + 3];
            let extension = transfer[SGB_OBJ_ATTRIBUTE_EXTENSION_BASE + (entry_index / 4)];
            let extension_shift = (entry_index % 4) * 2;
            let x_msb = ((extension >> extension_shift) & 0x01) as u16;
            let size_large = ((extension >> (extension_shift + 1)) & 0x01) != 0;
            self.obj.entries[entry_index] = SgbObjEntry {
                x: transfer[src] as u16 | (x_msb << 8),
                y: transfer[src + 1],
                tile_number: transfer[src + 2],
                tile_table_high: (attrs & 0x01) != 0,
                palette_number: 4 + ((attrs >> 1) & 0x03),
                priority: (attrs >> 4) & 0x03,
                size_large,
                x_flip: (attrs & 0x40) != 0,
                y_flip: (attrs & 0x80) != 0,
            };
        }

        self.obj.data_loaded = true;
        self.refresh_obj_palettes_from_system_palettes();
        true
    }

    pub fn load_data_trn_from_vram_transfer(&mut self, transfer: &[u8], destination: u16) -> bool {
        if transfer.len() < SGB_ATTR_TRANSFER_BYTES {
            return false;
        }

        self.data_transfer.destination = destination;
        self.data_transfer
            .data
            .copy_from_slice(&transfer[..SGB_ATTR_TRANSFER_BYTES]);
        self.data_transfer.loaded = true;
        true
    }

    pub fn has_obj_overlay(&self) -> bool {
        self.obj.control.enabled && self.obj.data_loaded
    }

    fn apply_two_palette_command(&mut self, payload: &[u8], first: usize, second: usize) {
        let Some(colors) = decode_color_words(payload, 7) else {
            return;
        };

        self.gb_palettes[first].colors[0] = colors[0];
        self.gb_palettes[first].colors[1] = colors[1];
        self.gb_palettes[first].colors[2] = colors[2];
        self.gb_palettes[first].colors[3] = colors[3];

        self.gb_palettes[second].colors[0] = colors[0];
        self.gb_palettes[second].colors[1] = colors[4];
        self.gb_palettes[second].colors[2] = colors[5];
        self.gb_palettes[second].colors[3] = colors[6];
        self.backdrop_color = colors[0];
        self.sync_shared_color0();
    }

    fn apply_pal_set(&mut self, payload: &[u8]) {
        if payload.len() < 9 {
            return;
        }

        self.pal_set.palette_indices = [
            u16::from_le_bytes([payload[0], payload[1]]),
            u16::from_le_bytes([payload[2], payload[3]]),
            u16::from_le_bytes([payload[4], payload[5]]),
            u16::from_le_bytes([payload[6], payload[7]]),
        ];
        let flags = payload[8];
        self.pal_set.apply_attr_file = (flags & 0x80) != 0;
        self.pal_set.mask_freeze_cancel = (flags & 0x40) != 0;
        self.pal_set.attr_file_index = flags & 0x3F;

        if self.system_palettes_loaded {
            for (target, palette_id) in self
                .gb_palettes
                .iter_mut()
                .zip(self.pal_set.palette_indices.iter().copied())
            {
                if (palette_id as usize) < SGB_SYSTEM_PALETTE_COUNT {
                    *target = self.system_palettes[palette_id as usize];
                }
            }
            self.backdrop_color = self.gb_palettes[0].colors[0];
            self.sync_shared_color0();
        }

        if self.pal_set.apply_attr_file && self.pal_set.attr_file_index < SGB_ATTR_FILE_COUNT as u8
        {
            self.apply_attr_file_index(self.pal_set.attr_file_index as usize);
        }
        if self.pal_set.mask_freeze_cancel {
            self.mask_mode = 0;
        }
    }

    fn sync_shared_color0(&mut self) {
        for palette in self.gb_palettes.iter_mut() {
            palette.colors[0] = self.backdrop_color;
        }
        for palette in self.border_palettes.iter_mut() {
            palette[0] = self.backdrop_color;
        }
    }

    fn border_backdrop_rgb(&self) -> [u8; 3] {
        bgr555_to_rgb888_sgb(self.backdrop_color)
    }

    fn apply_mask_en(&mut self, payload: &[u8]) {
        let Some(mode) = payload.first().copied() else {
            return;
        };
        self.mask_mode = mode & 0x03;
    }

    fn apply_atrc_en(&mut self, payload: &[u8]) {
        self.atrc.enabled = payload.first().copied().unwrap_or(0) != 0;
    }

    fn apply_test_en(&mut self, payload: &[u8]) {
        self.test.enabled = payload.first().copied().unwrap_or(0) != 0;
    }

    fn apply_icon_en(&mut self, payload: &[u8]) {
        self.icon.mode = payload.first().copied().unwrap_or(0) & 0x03;
    }

    fn apply_data_snd(&mut self, payload: &[u8]) {
        self.data_send.payload.clear();
        self.data_send.payload.extend_from_slice(payload);
    }

    fn apply_data_trn(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }
        self.data_transfer.destination = u16::from_le_bytes([payload[0], payload[1]]);
    }

    fn apply_jump(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            self.jump = SgbJumpState::default();
            return;
        }
        self.jump = SgbJumpState {
            target: u16::from_le_bytes([payload[0], payload[1]]),
            valid: true,
        };
    }

    fn apply_obj_trn(&mut self, payload: &[u8]) {
        let Some(flags) = payload.first().copied() else {
            return;
        };
        self.obj.control = SgbObjControl {
            enabled: (flags & 0x01) != 0,
            change_palettes: (flags & 0x02) != 0,
            palette_indices: self.decode_obj_palette_indices(payload),
        };
        self.refresh_obj_palettes_from_system_palettes();
    }

    fn apply_pal_pri(&mut self, payload: &[u8]) {
        self.pal_pri.enabled = payload.first().copied().unwrap_or(0) != 0;
    }

    fn apply_attr_blk(&mut self, payload: &[u8]) {
        let Some(set_count) = payload.first().copied() else {
            return;
        };

        for set_index in 0..set_count as usize {
            let entry_start = 1 + set_index * 6;
            let Some(entry) = payload.get(entry_start..entry_start + 6) else {
                break;
            };
            self.apply_attr_blk_entry(entry);
        }
    }

    fn apply_attr_set(&mut self, payload: &[u8]) {
        let Some(raw_value) = payload.first().copied() else {
            return;
        };

        let file_index = (raw_value & 0x3F) as usize;
        if file_index < SGB_ATTR_FILE_COUNT {
            self.apply_attr_file_index(file_index);
        }

        if (raw_value & 0x40) != 0 {
            self.mask_mode = 0;
        }
    }

    fn apply_attr_lin(&mut self, payload: &[u8]) {
        let Some(set_count) = payload.first().copied() else {
            return;
        };

        for entry in payload.iter().copied().skip(1).take(set_count as usize) {
            let line = (entry & 0x1F) as usize;
            let palette = (entry >> 5) & 0x03;
            if (entry & 0x80) != 0 {
                if line >= SGB_TILE_HEIGHT {
                    continue;
                }
                for tile_x in 0..SGB_TILE_WIDTH {
                    self.attr_map[line * SGB_TILE_WIDTH + tile_x] = palette;
                }
            } else {
                if line >= SGB_TILE_WIDTH {
                    continue;
                }
                for tile_y in 0..SGB_TILE_HEIGHT {
                    self.attr_map[tile_y * SGB_TILE_WIDTH + line] = palette;
                }
            }
        }
    }

    fn apply_attr_div(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }

        let flags = payload[0];
        let division_line = (payload[1] & 0x1F) as usize;
        let after_or_right_palette = flags & 0x03;
        let before_or_left_palette = (flags >> 2) & 0x03;
        let division_palette = (flags >> 4) & 0x03;
        let horizontal_split = (flags & 0x40) != 0;

        for tile_y in 0..SGB_TILE_HEIGHT {
            for tile_x in 0..SGB_TILE_WIDTH {
                let selector = if horizontal_split { tile_y } else { tile_x };
                self.attr_map[tile_y * SGB_TILE_WIDTH + tile_x] = if selector < division_line {
                    before_or_left_palette
                } else if selector == division_line {
                    division_palette
                } else {
                    after_or_right_palette
                };
            }
        }
    }

    fn apply_attr_chr(&mut self, payload: &[u8]) {
        if payload.len() < 5 {
            return;
        }

        let mut tile_x = payload[0] as usize;
        let mut tile_y = payload[1] as usize;
        let count = u16::from_le_bytes([payload[2], payload[3]]) as usize;
        let vertical_mode = payload[4] != 0;
        if tile_x >= SGB_TILE_WIDTH || tile_y >= SGB_TILE_HEIGHT || count == 0 {
            return;
        }

        let packed_data = &payload[5..];
        if packed_data.len() < count.div_ceil(4) {
            return;
        }

        for index in 0..count {
            let shift = 6 - ((index & 0x03) * 2);
            let palette = (packed_data[index / 4] >> shift) & 0x03;
            self.attr_map[tile_y * SGB_TILE_WIDTH + tile_x] = palette;

            if vertical_mode {
                tile_y += 1;
                if tile_y == SGB_TILE_HEIGHT {
                    tile_x += 1;
                    tile_y = 0;
                    if tile_x == SGB_TILE_WIDTH {
                        break;
                    }
                }
            } else {
                tile_x += 1;
                if tile_x == SGB_TILE_WIDTH {
                    tile_y += 1;
                    tile_x = 0;
                    if tile_y == SGB_TILE_HEIGHT {
                        break;
                    }
                }
            }
        }
    }

    fn apply_attr_blk_entry(&mut self, entry: &[u8]) {
        let control = entry[0];
        let palettes = entry[1];
        let inside_palette = palettes & 0x03;
        let line_palette = (palettes >> 2) & 0x03;
        let outside_palette = (palettes >> 4) & 0x03;
        let effective_line_palette = if (control & 0x02) != 0 {
            Some(line_palette)
        } else if control == 0x01 {
            Some(inside_palette)
        } else if control == 0x04 {
            Some(outside_palette)
        } else {
            None
        };

        let x1 = (entry[2] as usize).min(SGB_TILE_WIDTH - 1);
        let y1 = (entry[3] as usize).min(SGB_TILE_HEIGHT - 1);
        let x2 = (entry[4] as usize).min(SGB_TILE_WIDTH - 1);
        let y2 = (entry[5] as usize).min(SGB_TILE_HEIGHT - 1);
        let min_x = x1.min(x2);
        let max_x = x1.max(x2);
        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        for y in 0..SGB_TILE_HEIGHT {
            for x in 0..SGB_TILE_WIDTH {
                let in_rect = x >= min_x && x <= max_x && y >= min_y && y <= max_y;
                let is_inside = x > min_x && x < max_x && y > min_y && y < max_y;
                let is_line = in_rect && !is_inside;
                let is_outside = !in_rect;
                let map_index = y * SGB_TILE_WIDTH + x;

                if is_inside && (control & 0x01) != 0 {
                    self.attr_map[map_index] = inside_palette;
                }
                if is_line && let Some(palette) = effective_line_palette {
                    self.attr_map[map_index] = palette;
                }
                if is_outside && (control & 0x04) != 0 {
                    self.attr_map[map_index] = outside_palette;
                }
            }
        }
    }

    fn apply_attr_file_index(&mut self, file_index: usize) {
        if !self.attr_files_loaded[file_index] {
            return;
        }

        let file = &self.attr_files[file_index];
        for tile_y in 0..SGB_TILE_HEIGHT {
            let line_start = tile_y * 5;
            for group in 0..5 {
                let packed = file[line_start + group];
                for sub in 0..4 {
                    let tile_x = group * 4 + sub;
                    let shift = 6 - (sub * 2);
                    let palette = (packed >> shift) & 0x03;
                    self.attr_map[tile_y * SGB_TILE_WIDTH + tile_x] = palette;
                }
            }
        }
    }

    fn decode_obj_palette_indices(&self, payload: &[u8]) -> [u16; SGB_OBJ_PALETTE_COUNT] {
        let mut indices = [0; SGB_OBJ_PALETTE_COUNT];
        for (slot, index) in indices.iter_mut().enumerate() {
            let src = 1 + slot * 2;
            if src + 1 >= payload.len() {
                break;
            }
            *index = u16::from_le_bytes([payload[src], payload[src + 1]]);
        }
        indices
    }

    fn refresh_obj_palettes_from_system_palettes(&mut self) {
        if !self.obj.control.change_palettes || !self.system_palettes_loaded {
            return;
        }

        for (palette_slot, base_palette_id) in
            self.obj.control.palette_indices.iter().copied().enumerate()
        {
            for group in 0..4 {
                let palette_id = (base_palette_id as usize + group) % SGB_SYSTEM_PALETTE_COUNT;
                let color_start = group * 4;
                self.obj.palettes[palette_slot][color_start..color_start + 4]
                    .copy_from_slice(&self.system_palettes[palette_id].colors);
            }
        }
        self.obj.palettes_loaded = true;
    }

    fn border_entry_at(&self, x: usize, y: usize) -> u16 {
        let tile_x = x / 8;
        let tile_y = y / 8;
        self.border_tilemap[tile_y * SGB_BORDER_TILEMAP_WIDTH + tile_x]
    }

    fn border_color_index_at(&self, x: usize, y: usize) -> u8 {
        let entry = self.border_entry_at(x, y);
        let tile_number = (entry & 0x03FF) as usize;
        let x_flip = (entry & 0x4000) != 0;
        let y_flip = (entry & 0x8000) != 0;

        let tile = &self.border_tiles[tile_number.min(SGB_BORDER_TILE_COUNT - 1)];
        let mut px = x & 7;
        let mut py = y & 7;
        if x_flip {
            px = 7 - px;
        }
        if y_flip {
            py = 7 - py;
        }
        snes_4bpp_tile_color_index(tile, px, py)
    }

    fn border_rgb_at(&self, x: usize, y: usize) -> [u8; 3] {
        let entry = self.border_entry_at(x, y);
        let palette_number = ((entry >> 10) & 0x07) as usize;
        let palette_slot = palette_number
            .saturating_sub(4)
            .min(SGB_BORDER_PALETTE_COUNT - 1);
        let color_index = self.border_color_index_at(x, y) as usize;
        if color_index == 0 {
            return self.border_backdrop_rgb();
        }
        let bgr555 = self.border_palettes[palette_slot][color_index];
        bgr555_to_rgb888_sgb(bgr555)
    }
}

#[derive(Debug, Clone)]
pub struct SgbColorizer {
    rgb_frame: Vec<u8>,
    frozen_rgb_frame: Vec<u8>,
    live_frame_valid: bool,
    frozen_frame_valid: bool,
    last_mask_mode: u8,
    last_lcd_enabled: bool,
}

impl Default for SgbColorizer {
    fn default() -> Self {
        Self {
            rgb_frame: vec![0; LCD_FRAME_PIXELS * 3],
            frozen_rgb_frame: vec![0; LCD_FRAME_PIXELS * 3],
            live_frame_valid: false,
            frozen_frame_valid: false,
            last_mask_mode: 0,
            last_lcd_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SgbBorderRenderer {
    rgb_frame: Vec<u8>,
}

impl Default for SgbBorderRenderer {
    fn default() -> Self {
        Self {
            rgb_frame: vec![0; SGB_BORDER_WIDTH * SGB_BORDER_HEIGHT * 3],
        }
    }
}

impl SgbBorderRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compose_frame<'a>(&'a mut self, gb_rgb24: &[u8], state: &SgbState) -> Option<&'a [u8]> {
        if gb_rgb24.len() != LCD_FRAME_PIXELS * 3 || !state.has_presented_overlay() {
            return None;
        }

        if state.has_border_data() {
            for y in 0..SGB_BORDER_HEIGHT {
                for x in 0..SGB_BORDER_WIDTH {
                    let border_rgb = state.border_rgb_at(x, y);
                    let out = (y * SGB_BORDER_WIDTH + x) * 3;
                    self.rgb_frame[out] = border_rgb[0];
                    self.rgb_frame[out + 1] = border_rgb[1];
                    self.rgb_frame[out + 2] = border_rgb[2];
                }
            }
        } else {
            self.rgb_frame.fill(0);
        }

        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        for y in 0..LCD_HEIGHT {
            for x in 0..LCD_WIDTH {
                let border_x = gb_origin_x + x;
                let border_y = gb_origin_y + y;
                let src = (y * LCD_WIDTH + x) * 3;
                let dst = (border_y * SGB_BORDER_WIDTH + border_x) * 3;
                self.rgb_frame[dst] = gb_rgb24[src];
                self.rgb_frame[dst + 1] = gb_rgb24[src + 1];
                self.rgb_frame[dst + 2] = gb_rgb24[src + 2];
            }
        }

        self.overlay_obj_frame(state, gb_origin_x, gb_origin_y);
        Some(&self.rgb_frame)
    }

    fn overlay_obj_frame(&mut self, state: &SgbState, gb_origin_x: usize, gb_origin_y: usize) {
        if !state.has_obj_overlay() {
            return;
        }

        for entry in state.obj.entries.iter().rev() {
            let palette_slot =
                (entry.palette_number.saturating_sub(4) as usize).min(SGB_OBJ_PALETTE_COUNT - 1);
            let palette = &state.obj.palettes[palette_slot];
            let object_size = if entry.size_large { 16usize } else { 8usize };
            for sprite_y in 0..object_size {
                let y = entry.y as usize + sprite_y;
                if y >= SGB_BORDER_HEIGHT {
                    continue;
                }
                for sprite_x in 0..object_size {
                    let x = entry.x as usize + sprite_x;
                    if x >= SGB_BORDER_WIDTH {
                        continue;
                    }
                    if entry.priority <= 0x01
                        && x >= gb_origin_x
                        && x < gb_origin_x + LCD_WIDTH
                        && y >= gb_origin_y
                        && y < gb_origin_y + LCD_HEIGHT
                    {
                        continue;
                    }

                    let local_x = if entry.x_flip {
                        object_size - 1 - sprite_x
                    } else {
                        sprite_x
                    };
                    let local_y = if entry.y_flip {
                        object_size - 1 - sprite_y
                    } else {
                        sprite_y
                    };
                    let tile = &state.obj.tiles[obj_tile_number_for_entry(entry, local_x, local_y)
                        .min(SGB_OBJ_TILE_COUNT - 1)];
                    let tile_x = local_x & 7;
                    let tile_y = local_y & 7;
                    let color_index = snes_4bpp_tile_color_index(tile, tile_x, tile_y) as usize;
                    if color_index == 0 {
                        continue;
                    }

                    let rgb = bgr555_to_rgb888_sgb(palette[color_index]);
                    let dst = (y * SGB_BORDER_WIDTH + x) * 3;
                    self.rgb_frame[dst] = rgb[0];
                    self.rgb_frame[dst + 1] = rgb[1];
                    self.rgb_frame[dst + 2] = rgb[2];
                }
            }
        }
    }
}

fn obj_tile_number_for_entry(entry: &SgbObjEntry, local_x: usize, local_y: usize) -> usize {
    let base_tile = ((entry.tile_table_high as usize) << 8) | entry.tile_number as usize;
    if !entry.size_large {
        return base_tile;
    }

    let tile_column = local_x / 8;
    let tile_row = local_y / 8;
    let aligned_base = base_tile & !0x01;
    aligned_base + tile_column + tile_row * SGB_OBJ_TILE_STRIDE_PER_ROW
}

impl SgbColorizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn colorize_rgb_frame<'a>(
        &'a mut self,
        dmg_frame: &[u8],
        state: &SgbState,
        lcd_enabled: bool,
    ) -> &'a [u8] {
        if dmg_frame.len() != LCD_FRAME_PIXELS {
            return &self.rgb_frame;
        }

        let entering_freeze = state.mask_mode == 0x01 && self.last_mask_mode != 0x01;
        let lcd_just_disabled = !lcd_enabled && self.last_lcd_enabled;
        if entering_freeze || lcd_just_disabled {
            if !self.live_frame_valid {
                self.render_live_rgb_frame(dmg_frame, state);
            }
            self.frozen_rgb_frame.copy_from_slice(&self.rgb_frame);
            self.frozen_frame_valid = true;
        }

        let output = match state.mask_mode {
            0x01 => {
                if self.frozen_frame_valid {
                    &self.frozen_rgb_frame
                } else {
                    self.render_live_rgb_frame(dmg_frame, state);
                    &self.rgb_frame
                }
            }
            0x02 => {
                self.rgb_frame.fill(0);
                &self.rgb_frame
            }
            0x03 => {
                self.render_color0_mask_rgb_frame(state);
                &self.rgb_frame
            }
            _ if !lcd_enabled && self.frozen_frame_valid => &self.frozen_rgb_frame,
            _ => {
                self.render_live_rgb_frame(dmg_frame, state);
                &self.rgb_frame
            }
        };

        self.last_mask_mode = state.mask_mode;
        self.last_lcd_enabled = lcd_enabled;
        output
    }

    fn render_live_rgb_frame(&mut self, dmg_frame: &[u8], state: &SgbState) {
        for y in 0..LCD_HEIGHT {
            for x in 0..LCD_WIDTH {
                let pixel_index = y * LCD_WIDTH + x;
                let tile_x = x / 8;
                let tile_y = y / 8;
                let palette_index = state.palette_index_for_tile(tile_x, tile_y) as usize;
                let shade_index = canonical_dmg_shade_id_for_luma(dmg_frame[pixel_index]) as usize;
                let color = state.gb_palettes[palette_index].colors[shade_index];
                let rgb = bgr555_to_rgb888_sgb(color);
                let out_index = pixel_index * 3;
                self.rgb_frame[out_index] = rgb[0];
                self.rgb_frame[out_index + 1] = rgb[1];
                self.rgb_frame[out_index + 2] = rgb[2];
            }
        }
        self.live_frame_valid = true;
    }

    fn render_color0_mask_rgb_frame(&mut self, state: &SgbState) {
        for y in 0..LCD_HEIGHT {
            for x in 0..LCD_WIDTH {
                let pixel_index = y * LCD_WIDTH + x;
                let tile_x = x / 8;
                let tile_y = y / 8;
                let palette_index = state.palette_index_for_tile(tile_x, tile_y) as usize;
                let color = state.gb_palettes[palette_index].colors[0];
                let rgb = bgr555_to_rgb888_sgb(color);
                let out_index = pixel_index * 3;
                self.rgb_frame[out_index] = rgb[0];
                self.rgb_frame[out_index + 1] = rgb[1];
                self.rgb_frame[out_index + 2] = rgb[2];
            }
        }
    }
}

fn decode_color_words(payload: &[u8], count: usize) -> Option<Vec<u16>> {
    if payload.len() < count * 2 {
        return None;
    }

    let mut colors = Vec::with_capacity(count);
    for i in 0..count {
        let lo = payload[i * 2] as u16;
        let hi = payload[i * 2 + 1] as u16;
        colors.push(lo | (hi << 8));
    }
    Some(colors)
}

pub(crate) fn bgr555_to_rgb888(color: u16) -> [u8; 3] {
    let red_5 = (color & 0x1F) as u8;
    let green_5 = ((color >> 5) & 0x1F) as u8;
    let blue_5 = ((color >> 10) & 0x1F) as u8;
    [
        expand_5bit_to_8bit(red_5),
        expand_5bit_to_8bit(green_5),
        expand_5bit_to_8bit(blue_5),
    ]
}

pub(crate) fn bgr555_to_rgb888_sgb(color: u16) -> [u8; 3] {
    let red_5 = (color & 0x1F) as u8;
    let green_5 = ((color >> 5) & 0x1F) as u8;
    let blue_5 = ((color >> 10) & 0x1F) as u8;
    [
        expand_5bit_to_8bit_sgb(red_5),
        expand_5bit_to_8bit_sgb(green_5),
        expand_5bit_to_8bit_sgb(blue_5),
    ]
}

pub fn decode_sgb_transfer_from_framebuffer(
    framebuffer: &[u8; LCD_FRAME_PIXELS],
) -> [u8; SGB_ATTR_TRANSFER_BYTES] {
    let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];

    for tile_index in 0..256usize {
        let tile_x = tile_index % SGB_TILE_WIDTH;
        let tile_y = tile_index / SGB_TILE_WIDTH;
        let framebuffer_tile_base = tile_y * 8 * LCD_WIDTH + tile_x * 8;
        let transfer_tile_base = tile_index * 16;

        for row in 0..8usize {
            let mut plane0 = 0u8;
            let mut plane1 = 0u8;
            let row_base = framebuffer_tile_base + row * LCD_WIDTH;
            for x in 0..8usize {
                let shade = framebuffer[row_base + x];
                let color = canonical_dmg_shade_id_for_luma(shade) & 0x03;
                let bit = 7 - x;
                plane0 |= (color & 0x01) << bit;
                plane1 |= ((color >> 1) & 0x01) << bit;
            }
            transfer[transfer_tile_base + row * 2] = plane0;
            transfer[transfer_tile_base + row * 2 + 1] = plane1;
        }
    }

    transfer
}

fn rgb888_to_bgr555(color: [u8; 3]) -> u16 {
    let red = compress_8bit_to_5bit(color[0]);
    let green = compress_8bit_to_5bit(color[1]);
    let blue = compress_8bit_to_5bit(color[2]);
    red | (green << 5) | (blue << 10)
}

fn expand_5bit_to_8bit(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}

fn expand_5bit_to_8bit_sgb(value: u8) -> u8 {
    const SGB_COLOR_CURVE: [u8; 32] = [
        0, 2, 5, 9, 15, 20, 27, 34, 42, 50, 58, 67, 76, 85, 94, 104, 114, 123, 133, 143, 153, 163,
        173, 182, 192, 202, 211, 220, 229, 238, 247, 255,
    ];
    SGB_COLOR_CURVE[value as usize]
}

fn compress_8bit_to_5bit(value: u8) -> u16 {
    ((value as u16 * 31) + 127) / 255
}

pub(crate) fn sgb_built_in_boot_palette_rgb888(index: usize) -> Option<[[u8; 3]; 4]> {
    SGB_BUILT_IN_BOOT_PALETTES
        .get(index)
        .copied()
        .map(|colors| colors.map(bgr555_to_rgb888))
}

fn snes_4bpp_tile_color_index(tile: &[u8; SGB_BORDER_TILE_BYTES], x: usize, y: usize) -> u8 {
    let bit = 7 - (x & 7);
    let row = y & 7;
    let p0 = (tile[row * 2] >> bit) & 0x01;
    let p1 = (tile[row * 2 + 1] >> bit) & 0x01;
    let p2 = (tile[16 + row * 2] >> bit) & 0x01;
    let p3 = (tile[16 + row * 2 + 1] >> bit) & 0x01;
    p0 | (p1 << 1) | (p2 << 2) | (p3 << 3)
}

#[derive(Debug)]
pub struct SgbLink {
    expected_packets: u8,
    command_id: u8,
    packet_bits: [u8; SGB_PACKET_BYTES],
    packet_bit_len: usize,
    receiving_packet: bool,
    pending_packet: Option<[u8; SGB_PACKET_BYTES]>,
    packets: Vec<[u8; SGB_PACKET_BYTES]>,
}

impl SgbLink {
    pub fn new() -> Self {
        Self {
            packets: Vec::with_capacity(SGB_MAX_PACKETS),
            ..Self::default()
        }
    }

    pub fn on_key_mmio_write(&mut self, addr: u16, value: u8) -> Option<SgbCommand> {
        if addr != 0xFF00 {
            return None;
        }
        self.on_p1_write(value)
    }

    pub fn on_p1_write(&mut self, value: u8) -> Option<SgbCommand> {
        match value & P1_SELECT_MASK {
            P1_RESET => {
                if self.packet_bit_len != 0 || self.pending_packet.is_some() {
                    self.reset_transfer();
                }
                self.start_packet();
                None
            }
            P1_BIT_ZERO => self.push_packet_value_bit(false),
            P1_BIT_ONE => self.push_packet_value_bit(true),
            P1_IDLE => None,
            _ => None,
        }
    }

    fn start_packet(&mut self) {
        self.packet_bits = [0; SGB_PACKET_BYTES];
        self.packet_bit_len = 0;
        self.receiving_packet = true;
        self.pending_packet = None;
    }

    fn reset_transfer(&mut self) {
        self.expected_packets = 0;
        self.command_id = 0;
        self.packet_bits = [0; SGB_PACKET_BYTES];
        self.packet_bit_len = 0;
        self.receiving_packet = false;
        self.pending_packet = None;
        self.packets.clear();
    }

    fn push_packet_value_bit(&mut self, bit: bool) -> Option<SgbCommand> {
        if let Some(packet) = self.pending_packet.take() {
            if bit {
                self.reset_transfer();
                return None;
            }
            return self.on_packet_complete(packet);
        }

        if !self.receiving_packet {
            return None;
        }

        let byte_index = self.packet_bit_len / 8;
        let bit_index = self.packet_bit_len % 8;
        if bit {
            self.packet_bits[byte_index] |= 1 << bit_index;
        }

        self.packet_bit_len += 1;
        if self.packet_bit_len < SGB_PACKET_BITS {
            return None;
        }

        let packet = self.packet_bits;
        self.packet_bits = [0; SGB_PACKET_BYTES];
        self.packet_bit_len = 0;
        self.receiving_packet = false;
        self.pending_packet = Some(packet);
        None
    }

    fn on_packet_complete(&mut self, packet: [u8; SGB_PACKET_BYTES]) -> Option<SgbCommand> {
        if self.expected_packets == 0 {
            let packet_count = if is_sgb_header_packet(packet[0]) {
                1
            } else {
                packet[0] & 0x07
            };
            if packet_count == 0 || packet_count as usize > SGB_MAX_PACKETS {
                self.reset_transfer();
                return None;
            }
            self.expected_packets = packet_count;
            self.command_id = packet[0] >> 3;
            self.packets.clear();
        }

        self.packets.push(packet);
        if self.packets.len() < self.expected_packets as usize {
            return None;
        }

        let packet_count = self.expected_packets;
        let command_id = self.command_id;
        let mut bytes = Vec::with_capacity(self.packets.len() * SGB_PACKET_BYTES);
        for packet in &self.packets {
            bytes.extend_from_slice(packet);
        }
        self.reset_transfer();

        Some(SgbCommand {
            command_id,
            packet_count,
            bytes,
        })
    }
}

const fn is_sgb_header_packet(first_byte: u8) -> bool {
    (first_byte & 0xF1) == 0xF1
}

impl Default for SgbLink {
    fn default() -> Self {
        Self {
            expected_packets: 0,
            command_id: 0,
            packet_bits: [0; SGB_PACKET_BYTES],
            packet_bit_len: 0,
            receiving_packet: false,
            pending_packet: None,
            packets: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(command_id: u8, packet_count: u8, seed: u8) -> [u8; SGB_PACKET_BYTES] {
        let mut packet = [0u8; SGB_PACKET_BYTES];
        packet[0] = (command_id << 3) | (packet_count & 0x07);
        for (offset, byte) in packet.iter_mut().enumerate().skip(1) {
            *byte = seed.wrapping_add(offset as u8);
        }
        packet
    }

    fn make_command_with_payload(command_id: u8, payload: &[u8]) -> SgbCommand {
        let packet_count = payload.len().div_ceil(SGB_PACKET_BYTES - 1).max(1) as u8;
        let mut bytes = vec![0u8; packet_count as usize * SGB_PACKET_BYTES];
        let mut payload_index = 0usize;

        for packet_idx in 0..packet_count as usize {
            let packet_start = packet_idx * SGB_PACKET_BYTES;
            bytes[packet_start] = (command_id << 3) | packet_count;
            for i in 1..SGB_PACKET_BYTES {
                if payload_index >= payload.len() {
                    break;
                }
                bytes[packet_start + i] = payload[payload_index];
                payload_index += 1;
            }
        }

        SgbCommand {
            command_id,
            packet_count,
            bytes,
        }
    }

    fn make_obj_transfer(x: u8, y: u8, attrs: u8) -> [u8; SGB_ATTR_TRANSFER_BYTES] {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        for row in 0..8 {
            transfer[row * 2] = 0xFF; // plane 0 -> color index 1
        }
        transfer[SGB_OBJ_OAM_TRANSFER_BASE] = x;
        transfer[SGB_OBJ_OAM_TRANSFER_BASE + 1] = y;
        transfer[SGB_OBJ_OAM_TRANSFER_BASE + 2] = 0;
        transfer[SGB_OBJ_OAM_TRANSFER_BASE + 3] = attrs;
        transfer
    }

    fn feed_packet(link: &mut SgbLink, packet: &[u8; SGB_PACKET_BYTES]) -> Option<SgbCommand> {
        let mut command = None;
        assert!(link.on_p1_write(P1_RESET).is_none());
        for byte in packet {
            for bit in 0..8 {
                assert!(link.on_p1_write(P1_IDLE).is_none());
                let bit_value = (byte >> bit) & 0x01;
                let write = if bit_value == 0 {
                    P1_BIT_ZERO
                } else {
                    P1_BIT_ONE
                };
                command = link.on_p1_write(write).or(command);
            }
        }
        command = link.on_p1_write(P1_BIT_ZERO).or(command);
        command
    }

    #[test]
    fn single_packet_command_is_decoded_from_p1_bit_stream() {
        let packet = make_packet(0x11, 1, 0x40);
        let mut link = SgbLink::new();

        let command = feed_packet(&mut link, &packet).expect("single packet should decode");
        assert_eq!(command.command_id, 0x11);
        assert_eq!(command.packet_count, 1);
        assert_eq!(command.bytes, packet.to_vec());
    }

    #[test]
    fn multi_packet_command_waits_until_all_packets_arrive() {
        let packet0 = make_packet(0x0A, 2, 0x10);
        let packet1 = make_packet(0x0A, 2, 0x80);
        let mut link = SgbLink::new();

        assert!(feed_packet(&mut link, &packet0).is_none());
        let command = feed_packet(&mut link, &packet1).expect("second packet completes command");

        let mut expected = Vec::with_capacity(2 * SGB_PACKET_BYTES);
        expected.extend_from_slice(&packet0);
        expected.extend_from_slice(&packet1);
        assert_eq!(command.command_id, 0x0A);
        assert_eq!(command.packet_count, 2);
        assert_eq!(command.bytes, expected);
    }

    #[test]
    fn reset_write_clears_partial_transfer() {
        let packet = make_packet(0x03, 1, 0x22);
        let mut link = SgbLink::new();

        assert!(link.on_p1_write(P1_RESET).is_none());
        for _ in 0..12 {
            assert!(link.on_p1_write(P1_IDLE).is_none());
            assert!(link.on_p1_write(P1_BIT_ONE).is_none());
        }

        assert!(link.on_p1_write(P1_RESET).is_none());
        let command = feed_packet(&mut link, &packet).expect("fresh packet should decode");
        assert_eq!(command.command_id, 0x03);
        assert_eq!(command.packet_count, 1);
    }

    #[test]
    fn non_p1_events_are_ignored_by_key_mmio_entry_point() {
        let packet = make_packet(0x12, 1, 0x33);
        let mut link = SgbLink::new();

        assert!(link.on_key_mmio_write(0xFF40, P1_BIT_ZERO).is_none());
        let command = feed_packet(&mut link, &packet).expect("packet should decode");
        assert_eq!(command.command_id, 0x12);
    }

    #[test]
    fn bit_stream_without_packet_start_is_ignored() {
        let packet = make_packet(0x11, 1, 0x40);
        let mut link = SgbLink::new();

        for byte in packet {
            for bit in 0..8 {
                let write = if ((byte >> bit) & 0x01) == 0 {
                    P1_BIT_ZERO
                } else {
                    P1_BIT_ONE
                };
                assert!(link.on_p1_write(write).is_none());
            }
        }
        assert!(link.on_p1_write(P1_BIT_ZERO).is_none());
    }

    #[test]
    fn packet_is_not_emitted_until_stop_bit_arrives() {
        let packet = make_packet(0x11, 1, 0x40);
        let mut link = SgbLink::new();
        assert!(link.on_p1_write(P1_RESET).is_none());

        for byte in packet {
            for bit in 0..8 {
                let write = if ((byte >> bit) & 0x01) == 0 {
                    P1_BIT_ZERO
                } else {
                    P1_BIT_ONE
                };
                assert!(link.on_p1_write(write).is_none());
            }
        }

        let command = link
            .on_p1_write(P1_BIT_ZERO)
            .expect("stop bit should complete packet emission");
        assert_eq!(command.command_id, 0x11);
        assert_eq!(command.bytes, packet.to_vec());
    }

    #[test]
    fn sgb_header_packet_f3_is_treated_as_single_packet_command() {
        let mut packet = [0u8; SGB_PACKET_BYTES];
        packet[0] = 0xF3;
        let mut link = SgbLink::new();

        let command = feed_packet(&mut link, &packet).expect("header packet should decode");

        assert_eq!(command.command_id, 0x1E);
        assert_eq!(command.packet_count, 1);
        assert_eq!(command.bytes, packet.to_vec());
    }

    #[test]
    fn sgb_header_packet_does_not_consume_following_regular_command_packets() {
        let mut header_packet = [0u8; SGB_PACKET_BYTES];
        header_packet[0] = 0xF3;
        let regular_packet = make_packet(0x0A, 1, 0x40);
        let mut link = SgbLink::new();

        let header_command =
            feed_packet(&mut link, &header_packet).expect("header packet should decode");
        let regular_command =
            feed_packet(&mut link, &regular_packet).expect("regular packet should still decode");

        assert_eq!(header_command.command_id, 0x1E);
        assert_eq!(regular_command.command_id, 0x0A);
        assert_eq!(regular_command.packet_count, 1);
    }

    #[test]
    fn raw_joyp_bit_polarity_matches_pandocs() {
        let packet = make_packet(0x11, 1, 0x00);
        let mut link = SgbLink::new();

        assert!(link.on_p1_write(0x00).is_none());
        for byte in packet {
            for bit in 0..8 {
                assert!(link.on_p1_write(0x30).is_none());
                let write = if ((byte >> bit) & 0x01) == 0 {
                    0x20
                } else {
                    0x10
                };
                assert!(link.on_p1_write(write).is_none());
            }
        }

        let command = link
            .on_p1_write(0x20)
            .expect("raw SGB stop bit should emit the command");
        assert_eq!(command.command_id, 0x11);
        assert_eq!(command.bytes, packet.to_vec());
    }

    #[test]
    fn payload_bytes_skip_packet_header_byte_for_each_packet() {
        let payload = (0u8..20).collect::<Vec<_>>();
        let command = make_command_with_payload(0x04, &payload);
        let decoded = command.payload_bytes();

        assert_eq!(decoded.len(), 30);
        assert_eq!(&decoded[..payload.len()], payload);
    }

    #[test]
    fn built_in_boot_palette_assignment_matches_known_titles() {
        assert_eq!(
            sgb_boot_palette_for_title("KIRBY DREAM LAND").colors,
            [0x7F1F, 0x463D, 0x74CF, 0x4CA5]
        );
        assert_eq!(
            sgb_boot_palette_for_title("ZELDA").colors,
            [0x5B7F, 0x3F0F, 0x222D, 0x10EB]
        );
        assert!(sgb_has_title_boot_palette("KIRBY DREAM LAND"));
        assert!(sgb_has_title_boot_palette("ZELDA"));
    }

    #[test]
    fn built_in_boot_palette_prefers_header_crc32_override_when_available() {
        assert_eq!(
            sgb_boot_palette_for_header_crc32(0x3020_17CC)
                .expect("Kirby header CRC32 should resolve")
                .colors,
            [0x7F1F, 0x463D, 0x74CF, 0x4CA5]
        );
        assert_eq!(
            sgb_boot_palette_for_cartridge("UNKNOWN GAME", 0x3020_17CC).colors,
            [0x7F1F, 0x463D, 0x74CF, 0x4CA5]
        );
    }

    #[test]
    fn built_in_boot_palette_allows_external_rgb_override_by_header_crc32() {
        let overrides = PaletteOverrideDb::parse_ini(
            "[gb.override.302017CC]\npal[0]=0x112233\npal[1]=0x445566\npal[2]=0x778899\npal[3]=0xAABBCC\n",
        )
        .expect("override INI should parse");

        let palette = sgb_boot_palette_for_cartridge_with_overrides(
            "KIRBY DREAM LAND",
            0x3020_17CC,
            Some(&overrides),
        );

        assert_eq!(palette.colors.map(bgr555_to_rgb888)[0], [0x10, 0x21, 0x31]);
        assert_eq!(palette.colors.map(bgr555_to_rgb888)[1], [0x42, 0x52, 0x63]);
        assert_eq!(palette.colors.map(bgr555_to_rgb888)[2], [0x73, 0x8C, 0x9C]);
        assert_eq!(palette.colors.map(bgr555_to_rgb888)[3], [0xAD, 0xBD, 0xCE]);
    }

    #[test]
    fn sgb_rgb_conversion_uses_sgb_curve_instead_of_linear_expansion() {
        assert_eq!(bgr555_to_rgb888_sgb(0x001F), [255, 0, 0]);
        assert_eq!(bgr555_to_rgb888_sgb(0x03E0), [0, 255, 0]);
        assert_eq!(bgr555_to_rgb888_sgb(0x7C00), [0, 0, 255]);
        assert_eq!(bgr555_to_rgb888_sgb(0x4210), [114, 114, 114]);
        assert_eq!(bgr555_to_rgb888(0x4210), [132, 132, 132]);
    }

    #[test]
    fn built_in_boot_palette_falls_back_to_title_when_crc_override_is_missing() {
        assert_eq!(
            sgb_boot_palette_for_cartridge("ZELDA", 0).colors,
            [0x5B7F, 0x3F0F, 0x222D, 0x10EB]
        );
    }

    #[test]
    fn built_in_boot_palette_assignment_list_matches_reference_titles() {
        assert_eq!(
            SGB_BUILT_IN_BOOT_PALETTE_ASSIGNMENTS,
            [
                ("ZELDA", 4),
                ("SUPER MARIOLAND", 5),
                ("MARIOLAND2", 19),
                ("SUPERMARIOLAND3", 1),
                ("KIRBY DREAM LAND", 10),
                ("HOSHINOKA-BI", 10),
                ("KIRBY'S PINBALL", 2),
                ("YOSSY NO TAMAGO", 11),
                ("MARIO & YOSHI", 11),
                ("YOSSY NO COOKIE", 3),
                ("YOSHI'S COOKIE", 3),
                ("DR.MARIO", 17),
                ("TETRIS", 16),
                ("YAKUMAN", 18),
                ("METROID2", 30),
                ("KAERUNOTAMENI", 8),
                ("GOLF", 23),
                ("ALLEY WAY", 21),
                ("BASEBALL", 14),
                ("TENNIS", 22),
                ("F1RACE", 29),
                ("KID ICARUS", 13),
                ("QIX", 24),
                ("SOLARSTRIKER", 6),
                ("X", 27),
                ("GBWARS", 20),
            ]
        );
    }

    #[test]
    fn built_in_boot_palette_falls_back_to_sgb_default_for_unknown_titles() {
        assert_eq!(
            sgb_boot_palette_for_title("UNKNOWN GAME").colors,
            [0x67BF, 0x265B, 0x10B5, 0x2866]
        );
        assert!(!sgb_has_title_boot_palette("UNKNOWN GAME"));
    }

    #[test]
    fn apply_built_in_boot_palette_merges_external_override_without_marking_cart_command() {
        let overrides = PaletteOverrideDb::parse_ini(
            "[gb.override.302017CC]\npal[0]=0x112233\npal[1]=0x445566\n",
        )
        .expect("override INI should parse");
        let mut state = SgbState::new();

        let matched =
            state.apply_built_in_boot_palette("KIRBY DREAM LAND", 0x3020_17CC, Some(&overrides));

        assert!(matched);
        assert_eq!(state.last_applied_command_id(), None);
        assert_eq!(
            state.gb_palettes()[0].colors.map(bgr555_to_rgb888)[0],
            [0x10, 0x21, 0x31]
        );
        assert_eq!(
            state.gb_palettes()[0].colors.map(bgr555_to_rgb888)[1],
            [0x42, 0x52, 0x63]
        );
    }

    #[test]
    fn pal01_updates_palette_zero_and_one_with_shared_color_zero() {
        let payload = [
            0x01, 0x00, // shared color
            0x02, 0x00, // pal0 c1
            0x03, 0x00, // pal0 c2
            0x04, 0x00, // pal0 c3
            0x05, 0x00, // pal1 c1
            0x06, 0x00, // pal1 c2
            0x07, 0x00, // pal1 c3
        ];
        let command = make_command_with_payload(CMD_PAL01, &payload);
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(
            state.gb_palettes()[0].colors,
            [0x0001, 0x0002, 0x0003, 0x0004]
        );
        assert_eq!(
            state.gb_palettes()[1].colors,
            [0x0001, 0x0005, 0x0006, 0x0007]
        );
        assert_eq!(state.last_applied_command_id(), Some(CMD_PAL01));
    }

    #[test]
    fn pal23_updates_palette_two_and_three() {
        let payload = [
            0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x66, 0x66, 0x77, 0x77,
        ];
        let command = make_command_with_payload(CMD_PAL23, &payload);
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(
            state.gb_palettes()[2].colors,
            [0x1111, 0x2222, 0x3333, 0x4444]
        );
        assert_eq!(
            state.gb_palettes()[3].colors,
            [0x1111, 0x5555, 0x6666, 0x7777]
        );
    }

    #[test]
    fn pal_set_updates_selected_palette_indices_and_attr_flags() {
        let payload = [
            0x04, 0x00, // palette id #0
            0x05, 0x00, // palette id #1
            0x06, 0x00, // palette id #2
            0x07, 0x00, // palette id #3
            0xE1, // apply-atf + cancel mask + attr index
        ];
        let command = make_command_with_payload(CMD_PAL_SET, &payload);
        let mut state = SgbState::new();

        state.apply_command(&command);

        let pal_set = state.pal_set_state();
        assert_eq!(pal_set.palette_indices, [4, 5, 6, 7]);
        assert!(pal_set.apply_attr_file);
        assert_eq!(pal_set.attr_file_index, 0x21);
        assert!(pal_set.mask_freeze_cancel);
    }

    #[test]
    fn mask_en_stores_lower_two_bits_only() {
        let command = make_command_with_payload(CMD_MASK_EN, &[0xFF]);
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.mask_mode(), 0x03);
    }

    #[test]
    fn system_control_commands_capture_state() {
        let mut state = SgbState::new();

        state.apply_command(&make_command_with_payload(CMD_ATRC_EN, &[1]));
        state.apply_command(&make_command_with_payload(CMD_TEST_EN, &[1]));
        state.apply_command(&make_command_with_payload(CMD_ICON_EN, &[2]));
        state.apply_command(&make_command_with_payload(
            CMD_DATA_SND,
            &[0x12, 0x34, 0x56],
        ));
        state.apply_command(&make_command_with_payload(CMD_DATA_TRN, &[0x78, 0x56]));
        state.apply_command(&make_command_with_payload(CMD_JUMP, &[0xCD, 0xAB]));
        state.apply_command(&make_command_with_payload(CMD_PAL_PRI, &[1]));

        assert!(state.atrc_state().enabled);
        assert!(state.test_state().enabled);
        assert_eq!(state.icon_state().mode, 2);
        assert_eq!(&state.data_send_state().payload[..3], &[0x12, 0x34, 0x56]);
        assert_eq!(state.data_transfer_state().destination, 0x5678);
        assert!(state.jump_state().valid);
        assert_eq!(state.jump_state().target, 0xABCD);
        assert!(state.pal_pri_state().enabled);
    }

    #[test]
    fn attr_trn_payload_loads_attribute_files_from_vram_transfer_bytes() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        for file_index in 0..SGB_ATTR_FILE_COUNT {
            let file_start = file_index * SGB_ATTR_FILE_BYTES;
            transfer[file_start] = file_index as u8;
        }

        let mut state = SgbState::new();
        assert!(state.load_attr_files_from_vram_transfer(&transfer));

        let attr_set = make_command_with_payload(CMD_ATTR_SET, &[4]);
        state.apply_command(&attr_set);
        assert_eq!(state.palette_index_for_tile(0, 0), 0x00);
        assert_eq!(state.palette_index_for_tile(2, 0), 0x01);
    }

    #[test]
    fn attr_set_applies_selected_attr_file_and_can_cancel_mask() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        // ATF #2 first line packs [0,1,2,3] in the first byte.
        let atf2_start = 2 * SGB_ATTR_FILE_BYTES;
        transfer[atf2_start] = 0b00_01_10_11;

        let mut state = SgbState::new();
        assert!(state.load_attr_files_from_vram_transfer(&transfer));
        state.mask_mode = 2;

        let command = make_command_with_payload(CMD_ATTR_SET, &[0x40 | 0x02]);
        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(0, 0), 0);
        assert_eq!(state.palette_index_for_tile(1, 0), 1);
        assert_eq!(state.palette_index_for_tile(2, 0), 2);
        assert_eq!(state.palette_index_for_tile(3, 0), 3);
        assert_eq!(state.mask_mode(), 0);
    }

    #[test]
    fn pal_set_uses_attr_file_index_when_available() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        let atf3_start = 3 * SGB_ATTR_FILE_BYTES;
        transfer[atf3_start] = 0b11_10_01_00;

        let mut state = SgbState::new();
        assert!(state.load_attr_files_from_vram_transfer(&transfer));
        let command = make_command_with_payload(
            CMD_PAL_SET,
            &[
                0, 0, // palette id #0
                0, 0, // palette id #1
                0, 0, // palette id #2
                0, 0,    // palette id #3
                0xC3, // apply ATF + cancel mask + attr file index #3
            ],
        );
        state.mask_mode = 3;
        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(0, 0), 3);
        assert_eq!(state.palette_index_for_tile(1, 0), 2);
        assert_eq!(state.palette_index_for_tile(2, 0), 1);
        assert_eq!(state.palette_index_for_tile(3, 0), 0);
        assert_eq!(state.mask_mode(), 0);
    }

    #[test]
    fn attr_lin_updates_horizontal_and_vertical_lines() {
        let command = make_command_with_payload(
            CMD_ATTR_LIN,
            &[
                2, 0xC1, // horizontal line y=1 palette 2
                0x23, // vertical line x=3 palette 1
            ],
        );
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(0, 1), 2);
        assert_eq!(state.palette_index_for_tile(3, 0), 1);
        assert_eq!(state.palette_index_for_tile(3, 1), 1);
        assert_eq!(state.palette_index_for_tile(4, 1), 2);
    }

    #[test]
    fn attr_div_splits_attr_map_on_requested_division_line() {
        let command = make_command_with_payload(
            CMD_ATTR_DIV,
            &[
                0x79, // below/right=1 above/left=2 division=3 horizontal split
                5,
            ],
        );
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(0, 4), 2);
        assert_eq!(state.palette_index_for_tile(0, 5), 3);
        assert_eq!(state.palette_index_for_tile(0, 6), 1);
    }

    #[test]
    fn attr_chr_writes_left_to_right_and_wraps_to_next_row() {
        let command = make_command_with_payload(
            CMD_ATTR_CHR,
            &[
                19, 0, // x, y
                2, 0,    // two tiles
                0,    // horizontal mode
                0x60, // palettes [1,2]
            ],
        );
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(19, 0), 1);
        assert_eq!(state.palette_index_for_tile(0, 1), 2);
    }

    #[test]
    fn attr_chr_writes_top_to_bottom_and_wraps_to_next_column() {
        let command = make_command_with_payload(
            CMD_ATTR_CHR,
            &[
                0, 17, // x, y
                2, 0,    // two tiles
                1,    // vertical mode
                0x70, // palettes [1,3]
            ],
        );
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(0, 17), 1);
        assert_eq!(state.palette_index_for_tile(1, 0), 3);
    }

    #[test]
    fn pal_trn_loads_system_palettes_and_pal_set_applies_them() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        // Palette #2 colors as 16-bit little-endian words.
        let palette2_start = 2 * 8;
        transfer[palette2_start..palette2_start + 8].copy_from_slice(&[
            0x11, 0x11, // c0
            0x22, 0x22, // c1
            0x33, 0x33, // c2
            0x44, 0x44, // c3
        ]);

        let mut state = SgbState::new();
        assert!(state.load_system_palettes_from_vram_transfer(&transfer));
        let command = make_command_with_payload(
            CMD_PAL_SET,
            &[
                2, 0, // palette id #0
                2, 0, // palette id #1
                2, 0, // palette id #2
                2, 0, // palette id #3
                0, // flags
            ],
        );

        state.apply_command(&command);

        assert_eq!(
            state.gb_palettes()[0].colors,
            [0x1111, 0x2222, 0x3333, 0x4444]
        );
        assert_eq!(
            state.gb_palettes()[3].colors,
            [0x1111, 0x2222, 0x3333, 0x4444]
        );
    }

    #[test]
    fn pal_set_uses_palette_zero_color0_for_all_effective_palettes() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        let palette0_start = 0;
        transfer[palette0_start..palette0_start + 8]
            .copy_from_slice(&[0x10, 0x00, 0x11, 0x00, 0x12, 0x00, 0x13, 0x00]);
        let palette1_start = 8;
        transfer[palette1_start..palette1_start + 8]
            .copy_from_slice(&[0x20, 0x00, 0x21, 0x00, 0x22, 0x00, 0x23, 0x00]);
        let palette2_start = 2 * 8;
        transfer[palette2_start..palette2_start + 8]
            .copy_from_slice(&[0x30, 0x00, 0x31, 0x00, 0x32, 0x00, 0x33, 0x00]);
        let palette3_start = 3 * 8;
        transfer[palette3_start..palette3_start + 8]
            .copy_from_slice(&[0x40, 0x00, 0x41, 0x00, 0x42, 0x00, 0x43, 0x00]);

        let mut state = SgbState::new();
        assert!(state.load_system_palettes_from_vram_transfer(&transfer));
        state.apply_command(&make_command_with_payload(
            CMD_PAL_SET,
            &[
                0, 0, // palette id #0
                1, 0, // palette id #1
                2, 0, // palette id #2
                3, 0, // palette id #3
                0, // flags
            ],
        ));

        assert_eq!(
            state.gb_palettes()[0].colors,
            [0x0010, 0x0011, 0x0012, 0x0013]
        );
        assert_eq!(
            state.gb_palettes()[1].colors,
            [0x0010, 0x0021, 0x0022, 0x0023]
        );
        assert_eq!(
            state.gb_palettes()[2].colors,
            [0x0010, 0x0031, 0x0032, 0x0033]
        );
        assert_eq!(
            state.gb_palettes()[3].colors,
            [0x0010, 0x0041, 0x0042, 0x0043]
        );
        assert_eq!(state.backdrop_color(), 0x0010);
    }

    #[test]
    fn pct_trn_updates_global_backdrop_color_and_syncs_gb_color0() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        let palette_base = 0x0800;
        let last_border_palette_color0 = 0x7FFFu16;
        let last_palette_slot = SGB_BORDER_PALETTE_COUNT - 1;
        let last_palette_color0_offset =
            palette_base + last_palette_slot * SGB_BORDER_PALETTE_COLORS * 2;
        transfer[last_palette_color0_offset..last_palette_color0_offset + 2]
            .copy_from_slice(&last_border_palette_color0.to_le_bytes());
        let mut state = SgbState::new();
        state.gb_palettes[0].colors[0] = 0x0010;
        state.gb_palettes[1].colors[0] = 0x0020;

        assert!(state.load_border_pct_from_vram_transfer(&transfer));

        assert_eq!(state.backdrop_color(), last_border_palette_color0);
        assert_eq!(state.gb_palettes()[0].colors[0], last_border_palette_color0);
        assert_eq!(state.gb_palettes()[1].colors[0], last_border_palette_color0);
    }

    #[test]
    fn decode_sgb_transfer_from_framebuffer_reconstructs_visible_tiles() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        transfer[0x640..0x648].copy_from_slice(&[0x1F, 0x00, 0x03, 0x7C, 0x00, 0x42, 0x10, 0x21]);
        let mut framebuffer = [0xFFu8; LCD_FRAME_PIXELS];

        for tile_index in 0..256usize {
            let tile_x = tile_index % SGB_TILE_WIDTH;
            let tile_y = tile_index / SGB_TILE_WIDTH;
            let framebuffer_tile_base = tile_y * 8 * LCD_WIDTH + tile_x * 8;
            let transfer_tile_base = tile_index * 16;

            for row in 0..8usize {
                let plane0 = transfer[transfer_tile_base + row * 2];
                let plane1 = transfer[transfer_tile_base + row * 2 + 1];
                let row_base = framebuffer_tile_base + row * LCD_WIDTH;
                for x in 0..8usize {
                    let bit = 7 - x;
                    let color = ((plane0 >> bit) & 0x01) | (((plane1 >> bit) & 0x01) << 1);
                    framebuffer[row_base + x] = match color {
                        0 => 0xFF,
                        1 => 0xAA,
                        2 => 0x55,
                        _ => 0x00,
                    };
                }
            }
        }

        let decoded = decode_sgb_transfer_from_framebuffer(&framebuffer);

        assert_eq!(decoded, transfer);
    }

    #[test]
    fn attr_blk_updates_inside_line_and_outside_regions() {
        let payload = [
            1,    // one dataset
            0x07, // update inside/line/outside
            0x39, // inside=1 line=2 outside=3
            5, 4, 7, 6, // rectangle
        ];
        let command = make_command_with_payload(CMD_ATTR_BLK, &payload);
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(6, 5), 1);
        assert_eq!(state.palette_index_for_tile(5, 5), 2);
        assert_eq!(state.palette_index_for_tile(0, 0), 3);
    }

    #[test]
    fn attr_blk_inside_only_also_colors_surrounding_line() {
        let command = make_command_with_payload(
            CMD_ATTR_BLK,
            &[
                1,    // one dataset
                0x01, // inside only
                0x01, // inside palette = 1
                5, 4, 7, 6,
            ],
        );
        let mut state = SgbState::new();

        state.apply_command(&command);

        assert_eq!(state.palette_index_for_tile(6, 5), 1);
        assert_eq!(state.palette_index_for_tile(5, 5), 1);
        assert_eq!(state.palette_index_for_tile(0, 0), 0);
    }

    #[test]
    fn colorizer_uses_attr_tile_palette_and_dmg_shade() {
        let mut state = SgbState::new();
        state.gb_palettes[2].colors[1] = 0x7C00; // blue
        state.attr_map[0] = 2;
        let mut dmg_frame = [0u8; LCD_FRAME_PIXELS];
        dmg_frame[0] = 0xAA; // shade id 1
        let mut colorizer = SgbColorizer::new();

        let rgb = colorizer.colorize_rgb_frame(&dmg_frame, &state, true);

        assert_eq!(&rgb[0..3], &[0x00, 0x00, 0xFF]);
    }

    #[test]
    fn colorizer_mask_black_mode_forces_black_frame() {
        let mut state = SgbState::new();
        state.mask_mode = 0x02;
        let dmg_frame = [0xFFu8; LCD_FRAME_PIXELS];
        let mut colorizer = SgbColorizer::new();

        let rgb = colorizer.colorize_rgb_frame(&dmg_frame, &state, true);
        assert!(rgb.iter().all(|value| *value == 0));
    }

    #[test]
    fn colorizer_mask_freeze_mode_latches_previous_picture() {
        let mut state = SgbState::new();
        state.gb_palettes[0].colors[1] = 0x001F; // red
        let mut first_frame = [0u8; LCD_FRAME_PIXELS];
        first_frame[0] = 0xAA; // shade id 1
        let mut colorizer = SgbColorizer::new();

        let first_rgb = colorizer.colorize_rgb_frame(&first_frame, &state, true);
        assert_eq!(&first_rgb[0..3], &[0xFF, 0x00, 0x00]);

        state.mask_mode = 0x01;
        state.gb_palettes[0].colors[1] = 0x03E0; // green
        let mut second_frame = [0u8; LCD_FRAME_PIXELS];
        second_frame[0] = 0xAA;
        let frozen_rgb = colorizer.colorize_rgb_frame(&second_frame, &state, true);

        assert_eq!(&frozen_rgb[0..3], &[0xFF, 0x00, 0x00]);
    }

    #[test]
    fn colorizer_auto_freezes_when_lcd_is_disabled() {
        let mut state = SgbState::new();
        state.gb_palettes[0].colors[1] = 0x001F; // red
        let mut frame = [0u8; LCD_FRAME_PIXELS];
        frame[0] = 0xAA;
        let mut colorizer = SgbColorizer::new();

        let live_rgb = colorizer.colorize_rgb_frame(&frame, &state, true);
        assert_eq!(&live_rgb[0..3], &[0xFF, 0x00, 0x00]);

        state.gb_palettes[0].colors[1] = 0x03E0; // green
        let mut changed_frame = [0u8; LCD_FRAME_PIXELS];
        changed_frame[0] = 0xAA;
        let frozen_rgb = colorizer.colorize_rgb_frame(&changed_frame, &state, false);
        assert_eq!(&frozen_rgb[0..3], &[0xFF, 0x00, 0x00]);

        let resumed_rgb = colorizer.colorize_rgb_frame(&changed_frame, &state, true);
        assert_eq!(&resumed_rgb[0..3], &[0x00, 0xFF, 0x00]);
    }

    #[test]
    fn colorizer_mask_color0_mode_forces_palette_zero_color() {
        let mut state = SgbState::new();
        state.attr_map[0] = 2;
        state.gb_palettes[2].colors[0] = 0x03E0; // green
        state.gb_palettes[2].colors[1] = 0x001F; // red
        state.mask_mode = 0x03;
        let mut dmg_frame = [0u8; LCD_FRAME_PIXELS];
        dmg_frame[0] = 0xAA;
        let mut colorizer = SgbColorizer::new();

        let rgb = colorizer.colorize_rgb_frame(&dmg_frame, &state, true);

        assert_eq!(&rgb[0..3], &[0x00, 0xFF, 0x00]);
    }

    #[test]
    fn chr_trn_loads_low_and_high_border_tile_blocks() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        transfer[0] = 0xAA;
        transfer[SGB_BORDER_TILE_BYTES] = 0xBB;
        let mut state = SgbState::new();

        assert!(state.load_border_chr_from_vram_transfer(&transfer, false));
        assert_eq!(state.border_tiles[0][0], 0xAA);
        assert_eq!(state.border_tiles[1][0], 0xBB);
        assert_eq!(state.border_tiles[128][0], 0x00);
        assert!(!state.has_border_data());

        transfer[0] = 0xCC;
        assert!(state.load_border_chr_from_vram_transfer(&transfer, true));
        assert_eq!(state.border_tiles[128][0], 0xCC);
    }

    #[test]
    fn chr_trn_also_loads_obj_chr_blocks() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        transfer[0] = 0xAA;
        transfer[SGB_OBJ_TILE_BYTES] = 0xBB;
        let mut state = SgbState::new();

        assert!(state.load_obj_chr_from_vram_transfer(&transfer, false));
        assert_eq!(state.obj.tiles[0][0], 0xAA);
        assert_eq!(state.obj.tiles[1][0], 0xBB);

        transfer[0] = 0xCC;
        assert!(state.load_obj_chr_from_vram_transfer(&transfer, true));
        assert_eq!(state.obj.tiles[128][0], 0xCC);
    }

    #[test]
    fn pct_trn_loads_border_tilemap_and_palettes() {
        let mut transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        transfer[0..2].copy_from_slice(&0x1234u16.to_le_bytes());
        let palette_base = 0x0800;
        transfer[palette_base + 2..palette_base + 4].copy_from_slice(&0x001Fu16.to_le_bytes());
        let mut state = SgbState::new();

        assert!(state.load_border_chr_from_vram_transfer(&transfer, false));
        assert!(state.load_border_pct_from_vram_transfer(&transfer));
        assert_eq!(state.border_tilemap[0], 0x1234);
        assert_eq!(state.border_palettes[0][1], 0x001F);
        assert!(state.has_border_data());
    }

    #[test]
    fn border_color_zero_uses_global_backdrop_color() {
        let mut state = SgbState::new();
        state.backdrop_color = 0x7FFF;
        state.border_palettes[0][0] = 0x001F;

        let rgb = state.border_rgb_at(0, 0);

        assert_eq!(rgb, bgr555_to_rgb888_sgb(0x7FFF));
    }

    #[test]
    fn obj_trn_stores_enable_and_palette_selection_flags() {
        let mut state = SgbState::new();

        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00],
        ));
        assert!(state.obj.control.enabled);
        assert!(state.obj.control.change_palettes);
        assert_eq!(state.obj.control.palette_indices, [2, 3, 4, 5]);
    }

    #[test]
    fn obj_trn_loads_documented_oam_and_palette_blocks() {
        let mut transfer = make_obj_transfer(0x23, 32, 0xF6);
        let palette_words: [u16; 16] = [
            0x001F, 0x03E0, 0x7C00, 0x7FFF, 0x0210, 0x0420, 0x0630, 0x0840, 0x0A50, 0x0C60, 0x0E70,
            0x1080, 0x1290, 0x14A0, 0x16B0, 0x18C0,
        ];
        for (index, word) in palette_words.iter().copied().enumerate() {
            let base = index * 2;
            transfer[base..base + 2].copy_from_slice(&word.to_le_bytes());
        }
        transfer[SGB_OBJ_ATTRIBUTE_EXTENSION_BASE] = 0b0000_0011;
        let mut state = SgbState::new();

        assert!(state.load_system_palettes_from_vram_transfer(&transfer));
        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00],
        ));
        assert!(state.load_obj_from_vram_transfer(&transfer));

        assert_eq!(state.obj.entries[0].x, 0x123);
        assert_eq!(state.obj.entries[0].y, 32);
        assert_eq!(state.obj.entries[0].tile_number, 0);
        assert_eq!(state.obj.entries[0].palette_number, 7);
        assert_eq!(state.obj.entries[0].priority, 0x03);
        assert!(state.obj.entries[0].size_large);
        assert!(state.obj.entries[0].x_flip);
        assert!(state.obj.entries[0].y_flip);
        assert_eq!(
            state.obj.palettes[0][0..8],
            [
                0x001F, 0x03E0, 0x7C00, 0x7FFF, 0x0210, 0x0420, 0x0630, 0x0840
            ]
        );
        assert!(state.has_obj_overlay());
    }

    #[test]
    fn border_renderer_always_overlays_gb_viewport_over_border_window() {
        let mut state = SgbState::new();

        let mut chr_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        for row in 0..8 {
            chr_transfer[row * 2] = 0xFF; // plane 0 set -> color index 1
            chr_transfer[row * 2 + 1] = 0x00;
            chr_transfer[16 + row * 2] = 0x00;
            chr_transfer[16 + row * 2 + 1] = 0x00;
        }
        assert!(state.load_border_chr_from_vram_transfer(&chr_transfer, false));

        let mut pct_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        for entry in 0..SGB_BORDER_TILEMAP_VISIBLE_ENTRIES {
            let offset = entry * 2;
            pct_transfer[offset..offset + 2].copy_from_slice(&0x1000u16.to_le_bytes());
        }
        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        let center_tile_x = gb_origin_x / 8;
        let center_tile_y = gb_origin_y / 8;
        let center_entry = center_tile_y * SGB_BORDER_TILEMAP_WIDTH + center_tile_x;
        let center_offset = center_entry * 2;
        pct_transfer[center_offset..center_offset + 2].copy_from_slice(&0x1000u16.to_le_bytes());
        let palette_base = 0x0800;
        pct_transfer[palette_base + 2..palette_base + 4].copy_from_slice(&0x001Fu16.to_le_bytes());
        assert!(state.load_border_pct_from_vram_transfer(&pct_transfer));

        let mut renderer = SgbBorderRenderer::new();
        let mut gb_rgb = vec![0u8; LCD_FRAME_PIXELS * 3];
        gb_rgb[0] = 0x12;
        gb_rgb[1] = 0x34;
        gb_rgb[2] = 0x56;

        let composed = renderer
            .compose_frame(&gb_rgb, &state)
            .expect("border should compose when CHR and PCT are loaded");

        assert_eq!(composed.len(), SGB_BORDER_WIDTH * SGB_BORDER_HEIGHT * 3);
        assert_eq!(&composed[0..3], &[0xFF, 0x00, 0x00]);
        assert_ne!(state.border_color_index_at(gb_origin_x, gb_origin_y), 0);
        let center_pixel = (gb_origin_y * SGB_BORDER_WIDTH + gb_origin_x) * 3;
        assert_eq!(
            &composed[center_pixel..center_pixel + 3],
            &[0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn border_renderer_can_render_obj_overlay_without_border_data() {
        let mut state = SgbState::new();
        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        let mut palette_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        let transfer = make_obj_transfer(gb_origin_x as u8, gb_origin_y as u8, 0x30);
        assert!(state.load_system_palettes_from_vram_transfer(&palette_transfer));
        assert!(state.load_obj_chr_from_vram_transfer(&transfer, false));
        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ));
        assert!(state.load_obj_from_vram_transfer(&transfer));

        let mut renderer = SgbBorderRenderer::new();
        let gb_rgb = vec![0x00; LCD_FRAME_PIXELS * 3];
        let composed = renderer
            .compose_frame(&gb_rgb, &state)
            .expect("OBJ overlay should compose a presented SGB frame");

        assert_eq!(composed.len(), SGB_BORDER_WIDTH * SGB_BORDER_HEIGHT * 3);
        let sprite_pixel = (gb_origin_y * SGB_BORDER_WIDTH + gb_origin_x) * 3;
        assert_eq!(
            &composed[sprite_pixel..sprite_pixel + 3],
            &[0xFF, 0x00, 0x00]
        );
    }

    #[test]
    fn border_renderer_keeps_low_priority_obj_behind_gb_viewport() {
        let mut state = SgbState::new();
        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        let mut palette_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        let transfer = make_obj_transfer(gb_origin_x as u8, gb_origin_y as u8, 0x00);
        assert!(state.load_system_palettes_from_vram_transfer(&palette_transfer));
        assert!(state.load_obj_chr_from_vram_transfer(&transfer, false));
        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ));
        assert!(state.load_obj_from_vram_transfer(&transfer));

        let mut renderer = SgbBorderRenderer::new();
        let mut gb_rgb = vec![0x00; LCD_FRAME_PIXELS * 3];
        gb_rgb[0] = 0x12;
        gb_rgb[1] = 0x34;
        gb_rgb[2] = 0x56;
        let composed = renderer
            .compose_frame(&gb_rgb, &state)
            .expect("OBJ overlay should compose a presented SGB frame");

        let sprite_pixel = (gb_origin_y * SGB_BORDER_WIDTH + gb_origin_x) * 3;
        assert_eq!(
            &composed[sprite_pixel..sprite_pixel + 3],
            &[0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn border_renderer_draws_priority_two_obj_over_gb_viewport() {
        let mut state = SgbState::new();
        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        let mut palette_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        let transfer = make_obj_transfer(gb_origin_x as u8, gb_origin_y as u8, 0x20);
        assert!(state.load_system_palettes_from_vram_transfer(&palette_transfer));
        assert!(state.load_obj_chr_from_vram_transfer(&transfer, false));
        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ));
        assert!(state.load_obj_from_vram_transfer(&transfer));

        let mut renderer = SgbBorderRenderer::new();
        let mut gb_rgb = vec![0x00; LCD_FRAME_PIXELS * 3];
        gb_rgb[0] = 0x12;
        gb_rgb[1] = 0x34;
        gb_rgb[2] = 0x56;
        let composed = renderer
            .compose_frame(&gb_rgb, &state)
            .expect("OBJ overlay should compose a presented SGB frame");

        let sprite_pixel = (gb_origin_y * SGB_BORDER_WIDTH + gb_origin_x) * 3;
        assert_ne!(
            &composed[sprite_pixel..sprite_pixel + 3],
            &[0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn border_renderer_renders_large_obj_as_two_by_two_tile_block() {
        let mut state = SgbState::new();
        let gb_origin_x = (SGB_BORDER_WIDTH - LCD_WIDTH) / 2;
        let gb_origin_y = (SGB_BORDER_HEIGHT - LCD_HEIGHT) / 2;
        let mut palette_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);

        let mut chr_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        for tile_index in [0usize, 1, 16, 17] {
            let tile_base = tile_index * SGB_OBJ_TILE_BYTES;
            for row in 0..8 {
                chr_transfer[tile_base + row * 2] = 0xFF;
            }
        }

        let mut oam_transfer = [0u8; SGB_ATTR_TRANSFER_BYTES];
        oam_transfer[SGB_OBJ_OAM_TRANSFER_BASE] = gb_origin_x as u8;
        oam_transfer[SGB_OBJ_OAM_TRANSFER_BASE + 1] = gb_origin_y as u8;
        oam_transfer[SGB_OBJ_OAM_TRANSFER_BASE + 2] = 0;
        oam_transfer[SGB_OBJ_OAM_TRANSFER_BASE + 3] = 0x30;
        oam_transfer[SGB_OBJ_ATTRIBUTE_EXTENSION_BASE] = 0b0000_0010;

        assert!(state.load_system_palettes_from_vram_transfer(&palette_transfer));
        assert!(state.load_obj_chr_from_vram_transfer(&chr_transfer, false));
        state.apply_command(&make_command_with_payload(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ));
        assert!(state.load_obj_from_vram_transfer(&oam_transfer));

        let mut renderer = SgbBorderRenderer::new();
        let gb_rgb = vec![0x00; LCD_FRAME_PIXELS * 3];
        let composed = renderer
            .compose_frame(&gb_rgb, &state)
            .expect("large OBJ overlay should compose a presented SGB frame");

        for (x, y) in [
            (gb_origin_x, gb_origin_y),
            (gb_origin_x + 8, gb_origin_y),
            (gb_origin_x, gb_origin_y + 8),
            (gb_origin_x + 8, gb_origin_y + 8),
        ] {
            let pixel = (y * SGB_BORDER_WIDTH + x) * 3;
            assert_ne!(&composed[pixel..pixel + 3], &[0x00, 0x00, 0x00]);
        }
    }
}
