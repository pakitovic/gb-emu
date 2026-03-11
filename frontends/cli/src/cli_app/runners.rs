use super::MOONEYE_LOOP_WINDOW;
use gb_emu::gameboy::GameBoy;
use gb_emu::sgb::{
    CMD_ATRC_EN, CMD_ATTR_BLK, CMD_ATTR_CHR, CMD_ATTR_DIV, CMD_ATTR_LIN, CMD_ATTR_SET,
    CMD_ATTR_TRN, CMD_CHR_TRN, CMD_DATA_SND, CMD_DATA_TRN, CMD_ICON_EN, CMD_JUMP, CMD_MASK_EN,
    CMD_MLT_REQ, CMD_OBJ_TRN, CMD_PAL_PRI, CMD_PAL_SET, CMD_PAL_TRN, CMD_PAL01, CMD_PAL03,
    CMD_PAL12, CMD_PAL23, CMD_PCT_TRN, CMD_TEST_EN, SgbLink, SgbState,
    decode_sgb_transfer_from_framebuffer,
};
use std::collections::BTreeMap;

const SGB_TRANSFER_BLOCK_BYTES: usize = 0x1000;
const SGB_TRANSFER_CAPTURE_DELAY_FRAMES: u8 = 1;

fn bgr555_to_rgb888(color: u16) -> [u8; 3] {
    let red = ((color & 0x1F) as u8) << 3;
    let green = (((color >> 5) & 0x1F) as u8) << 3;
    let blue = (((color >> 10) & 0x1F) as u8) << 3;
    [red, green, blue]
}

pub(super) fn looks_like_tight_loop(pc_window: &[u16; MOONEYE_LOOP_WINDOW]) -> bool {
    let mut unique = [0u16; 4];
    let mut unique_len = 0usize;

    'outer: for &pc in pc_window {
        for &seen in unique.iter().take(unique_len) {
            if seen == pc {
                continue 'outer;
            }
        }

        if unique_len == unique.len() {
            return false;
        }
        unique[unique_len] = pc;
        unique_len += 1;
    }

    true
}

fn print_basic_trace(gb: &GameBoy, cycles: u8) {
    println!(
        "PC: {:04X}, A: {:02X}, cycles: {}",
        gb.cpu().registers().pc,
        gb.cpu().registers().a,
        cycles
    );
}

fn print_mooneye_trace(gb: &GameBoy, cycles: u8) {
    println!(
        "PC: {:04X}, A: {:02X}, B: {:02X}, C: {:02X}, D: {:02X}, E: {:02X}, H: {:02X}, L: {:02X}, cycles: {}",
        gb.cpu().registers().pc,
        gb.cpu().registers().a,
        gb.cpu().registers().b,
        gb.cpu().registers().c,
        gb.cpu().registers().d,
        gb.cpu().registers().e,
        gb.cpu().registers().h,
        gb.cpu().registers().l,
        cycles
    );
}

pub(super) fn run_forever(gb: &mut GameBoy, trace: bool) -> ! {
    println!("ROM: {}", gb.rom_title());
    loop {
        let cycles = gb.step();
        if trace {
            print_basic_trace(gb, cycles);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SgbCommandStats {
    count: usize,
    first_step: usize,
    last_step: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingReportTransfer {
    command_id: u8,
    payload: Vec<u8>,
    frames_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransferCaptureSummary {
    command_id: u8,
    non_zero_bytes: usize,
    first_bytes: [u8; 16],
    palette_window_bytes: [u8; 32],
}

pub(super) fn run_sgb_report(gb: &mut GameBoy, max_steps: usize, trace: bool) {
    println!("ROM: {}", gb.rom_title());
    println!("Model: {}", gb.hardware_model());

    let mut sgb_link = SgbLink::new();
    let mut sgb_state = SgbState::new();
    let mut immediate_vram_state = SgbState::new();
    let mut pending_transfer_commands = Vec::<PendingReportTransfer>::new();
    let mut command_stats = BTreeMap::<u8, SgbCommandStats>::new();
    let mut recent_commands = Vec::<String>::new();
    let mut transfer_summaries = Vec::<TransferCaptureSummary>::new();
    let mut last_frame_counter = gb.frame_counter();

    for step_index in 0..max_steps {
        let cycles = gb.step();
        if trace {
            print_basic_trace(gb, cycles);
        }

        let frame_counter = gb.frame_counter();
        if frame_counter != last_frame_counter {
            for entry in &mut pending_transfer_commands {
                if entry.frames_remaining > 0 {
                    entry.frames_remaining -= 1;
                }
            }
            let ready_count = pending_transfer_commands
                .iter()
                .filter(|entry| entry.frames_remaining == 0)
                .count();
            if ready_count != 0 {
                let transfer = decode_sgb_transfer_from_framebuffer(gb.framebuffer());
                let mut remaining =
                    Vec::with_capacity(pending_transfer_commands.len() - ready_count);
                for entry in pending_transfer_commands.drain(..) {
                    if entry.frames_remaining == 0 {
                        let non_zero_bytes = transfer.iter().filter(|&&byte| byte != 0).count();
                        let mut first_bytes = [0u8; 16];
                        first_bytes.copy_from_slice(&transfer[..16]);
                        let mut palette_window_bytes = [0u8; 32];
                        palette_window_bytes.copy_from_slice(&transfer[0x640..0x660]);
                        transfer_summaries.push(TransferCaptureSummary {
                            command_id: entry.command_id,
                            non_zero_bytes,
                            first_bytes,
                            palette_window_bytes,
                        });
                        apply_transfer_command_for_report(
                            &mut sgb_state,
                            entry.command_id,
                            &entry.payload,
                            &transfer,
                        );
                    } else {
                        remaining.push(entry);
                    }
                }
                pending_transfer_commands = remaining;
            }
            last_frame_counter = frame_counter;
        }

        for event in gb.drain_key_mmio_write_events() {
            let Some(command) = sgb_link.on_key_mmio_write(event.addr, event.value) else {
                continue;
            };
            let entry = command_stats
                .entry(command.command_id)
                .or_insert(SgbCommandStats {
                    count: 0,
                    first_step: step_index,
                    last_step: step_index,
                });
            entry.count += 1;
            entry.last_step = step_index;
            let payload = command
                .payload_bytes()
                .iter()
                .take(12)
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            recent_commands.push(format!(
                "{:02X}:{} payload=[{}]",
                command.command_id,
                sgb_command_name(command.command_id),
                payload
            ));
            let payload_bytes = command.payload_bytes();
            if is_transfer_command_for_report(command.command_id) {
                let mut raw_vram = [0u8; SGB_TRANSFER_BLOCK_BYTES];
                let _ = gb.copy_vram_hardware_block(0x8000, &mut raw_vram);
                apply_transfer_command_for_report(
                    &mut immediate_vram_state,
                    command.command_id,
                    &payload_bytes,
                    &raw_vram,
                );
                pending_transfer_commands.push(PendingReportTransfer {
                    command_id: command.command_id,
                    payload: payload_bytes.clone(),
                    frames_remaining: SGB_TRANSFER_CAPTURE_DELAY_FRAMES,
                });
            }
            sgb_state.apply_command(&command);
        }
    }

    println!("SGB report:");
    println!("  steps: {max_steps}");
    println!("  unique_commands: {}", command_stats.len());
    let lcdc = gb.bus.read_byte(0xFF40);
    let stat = gb.bus.read_byte(0xFF41);
    let mut shade_counts = [0usize; 256];
    for &shade in gb.framebuffer().iter() {
        shade_counts[shade as usize] += 1;
    }
    println!("  final_lcdc: {lcdc:02X}");
    println!("  final_stat: {stat:02X}");
    println!(
        "  framebuffer_luma: [FF]={} [AA]={} [55]={} [00]={}",
        shade_counts[0xFF], shade_counts[0xAA], shade_counts[0x55], shade_counts[0x00]
    );
    println!("  final_mask_mode: {:02X}", sgb_state.mask_mode());
    println!(
        "  final_pal_set: [{:04X}, {:04X}, {:04X}, {:04X}] flags=apply_atf:{} cancel_mask:{} attr_file:{}",
        sgb_state.pal_set_state().palette_indices[0],
        sgb_state.pal_set_state().palette_indices[1],
        sgb_state.pal_set_state().palette_indices[2],
        sgb_state.pal_set_state().palette_indices[3],
        sgb_state.pal_set_state().apply_attr_file,
        sgb_state.pal_set_state().mask_freeze_cancel,
        sgb_state.pal_set_state().attr_file_index,
    );
    for (palette_index, palette) in sgb_state.gb_palettes().iter().enumerate() {
        let rgb = palette.colors.map(bgr555_to_rgb888);
        println!(
            "  gb_palette[{palette_index}]: #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X}",
            rgb[0][0],
            rgb[0][1],
            rgb[0][2],
            rgb[1][0],
            rgb[1][1],
            rgb[1][2],
            rgb[2][0],
            rgb[2][1],
            rgb[2][2],
            rgb[3][0],
            rgb[3][1],
            rgb[3][2],
        );
    }
    let mut attr_histogram = [0usize; 4];
    for &palette_index in sgb_state.attr_map() {
        if let Some(bucket) = attr_histogram.get_mut(palette_index as usize) {
            *bucket += 1;
        }
    }
    println!(
        "  attr_map_histogram: [0]={} [1]={} [2]={} [3]={}",
        attr_histogram[0], attr_histogram[1], attr_histogram[2], attr_histogram[3]
    );
    println!("  immediate_vram_palettes:");
    for (palette_index, palette) in immediate_vram_state.gb_palettes().iter().enumerate() {
        let rgb = palette.colors.map(bgr555_to_rgb888);
        println!(
            "    gb_palette[{palette_index}]: #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X} #{:02X}{:02X}{:02X}",
            rgb[0][0],
            rgb[0][1],
            rgb[0][2],
            rgb[1][0],
            rgb[1][1],
            rgb[1][2],
            rgb[2][0],
            rgb[2][1],
            rgb[2][2],
            rgb[3][0],
            rgb[3][1],
            rgb[3][2],
        );
    }
    if !transfer_summaries.is_empty() {
        println!("  transfer_captures:");
        for summary in &transfer_summaries {
            let first_bytes = summary
                .first_bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let palette_window_bytes = summary
                .palette_window_bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!(
                "    {:02X} {:<8} non_zero_bytes={} first_bytes=[{}] palette_window_0x640=[{}]",
                summary.command_id,
                sgb_command_name(summary.command_id),
                summary.non_zero_bytes,
                first_bytes,
                palette_window_bytes
            );
        }
    }
    if command_stats.is_empty() {
        println!("  commands: none");
        return;
    }

    println!("  commands:");
    for (command_id, stats) in command_stats {
        println!(
            "    {command_id:02X} {:<8} count={} first_step={} last_step={}",
            sgb_command_name(command_id),
            stats.count,
            stats.first_step,
            stats.last_step
        );
    }

    let tail_start = recent_commands.len().saturating_sub(16);
    let tail = recent_commands[tail_start..].join(", ");
    println!("  recent: [{tail}]");
}

fn is_transfer_command_for_report(command_id: u8) -> bool {
    matches!(
        command_id,
        CMD_ATTR_TRN | CMD_PAL_TRN | CMD_DATA_TRN | CMD_CHR_TRN | CMD_PCT_TRN | CMD_OBJ_TRN
    )
}

fn apply_transfer_command_for_report(
    state: &mut SgbState,
    command_id: u8,
    payload: &[u8],
    transfer: &[u8; SGB_TRANSFER_BLOCK_BYTES],
) {
    match command_id {
        CMD_ATTR_TRN => {
            let _ = state.load_attr_files_from_vram_transfer(transfer);
        }
        CMD_PAL_TRN => {
            let _ = state.load_system_palettes_from_vram_transfer(transfer);
        }
        CMD_DATA_TRN => {
            let destination = if payload.len() >= 2 {
                u16::from_le_bytes([payload[0], payload[1]])
            } else {
                0
            };
            let _ = state.load_data_trn_from_vram_transfer(transfer, destination);
        }
        CMD_CHR_TRN => {
            let high_tile_block = payload.first().copied().unwrap_or(0) & 0x01 != 0;
            let _ = state.load_border_chr_from_vram_transfer(transfer, high_tile_block);
            let _ = state.load_obj_chr_from_vram_transfer(transfer, high_tile_block);
        }
        CMD_PCT_TRN => {
            let _ = state.load_border_pct_from_vram_transfer(transfer);
        }
        CMD_OBJ_TRN => {
            let _ = state.load_obj_from_vram_transfer(transfer);
        }
        _ => {}
    }
}

pub(super) fn run_blargg(gb: &mut GameBoy, max_steps: usize, trace: bool) -> Option<&'static str> {
    println!("ROM: {}", gb.rom_title());
    for _ in 0..max_steps {
        let cycles = gb.step();
        if trace {
            print_basic_trace(gb, cycles);
        }

        let serial = gb.serial_output();
        if serial.contains("Passed") {
            return Some("Passed");
        }
        if serial.contains("Failed") {
            return Some("Failed");
        }

        let sig_ok = gb.bus.read_byte(0xA001) == 0xDE
            && gb.bus.read_byte(0xA002) == 0xB0
            && gb.bus.read_byte(0xA003) == 0x61;
        if sig_ok {
            let status = gb.bus.read_byte(0xA000);
            if status == 0x00 {
                return Some("Passed");
            }
            if status != 0x80 {
                return Some("Failed");
            }
        }
    }
    None
}

pub(super) fn run_mooneye(gb: &mut GameBoy, max_steps: usize, trace: bool) -> Option<&'static str> {
    println!("ROM: {}", gb.rom_title());
    let mut pc_window = [0u16; MOONEYE_LOOP_WINDOW];
    let mut pc_window_len = 0usize;
    let mut pc_window_pos = 0usize;

    for _ in 0..max_steps {
        let cycles = gb.step();
        if trace {
            print_mooneye_trace(gb, cycles);
        }

        let pc = gb.cpu().registers().pc;
        pc_window[pc_window_pos] = pc;
        pc_window_pos = (pc_window_pos + 1) % MOONEYE_LOOP_WINDOW;
        if pc_window_len < MOONEYE_LOOP_WINDOW {
            pc_window_len += 1;
        }

        let regs = (
            gb.cpu().registers().b,
            gb.cpu().registers().c,
            gb.cpu().registers().d,
            gb.cpu().registers().e,
            gb.cpu().registers().h,
            gb.cpu().registers().l,
        );
        let in_tight_loop =
            pc_window_len == MOONEYE_LOOP_WINDOW && looks_like_tight_loop(&pc_window);
        if regs == (3, 5, 8, 13, 21, 34) && in_tight_loop {
            return Some("Passed");
        }
        if regs == (0x42, 0x42, 0x42, 0x42, 0x42, 0x42) && in_tight_loop {
            return Some("Failed");
        }
    }
    None
}

fn sgb_command_name(command_id: u8) -> &'static str {
    match command_id {
        CMD_PAL01 => "PAL01",
        CMD_PAL23 => "PAL23",
        CMD_PAL03 => "PAL03",
        CMD_PAL12 => "PAL12",
        CMD_ATTR_BLK => "ATTR_BLK",
        CMD_ATTR_LIN => "ATTR_LIN",
        CMD_ATTR_DIV => "ATTR_DIV",
        CMD_ATTR_CHR => "ATTR_CHR",
        CMD_PAL_SET => "PAL_SET",
        CMD_PAL_TRN => "PAL_TRN",
        CMD_ATRC_EN => "ATRC_EN",
        CMD_TEST_EN => "TEST_EN",
        CMD_ICON_EN => "ICON_EN",
        CMD_DATA_SND => "DATA_SND",
        CMD_DATA_TRN => "DATA_TRN",
        CMD_MLT_REQ => "MLT_REQ",
        CMD_JUMP => "JUMP",
        CMD_CHR_TRN => "CHR_TRN",
        CMD_PCT_TRN => "PCT_TRN",
        CMD_ATTR_TRN => "ATTR_TRN",
        CMD_ATTR_SET => "ATTR_SET",
        CMD_MASK_EN => "MASK_EN",
        CMD_OBJ_TRN => "OBJ_TRN",
        CMD_PAL_PRI => "PAL_PRI",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tight_loop_detector_accepts_small_repeating_pc_sets() {
        let one_pc = [0x1234; MOONEYE_LOOP_WINDOW];
        let two_pc = [
            0x2000, 0x2001, 0x2000, 0x2001, 0x2000, 0x2001, 0x2000, 0x2001,
        ];
        assert!(looks_like_tight_loop(&one_pc));
        assert!(looks_like_tight_loop(&two_pc));
    }

    #[test]
    fn tight_loop_detector_rejects_wide_pc_ranges() {
        let wide = [
            0x1000, 0x1001, 0x1002, 0x1003, 0x1004, 0x1005, 0x1006, 0x1007,
        ];
        assert!(!looks_like_tight_loop(&wide));
    }
}
