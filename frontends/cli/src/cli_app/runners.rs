use super::MOONEYE_LOOP_WINDOW;
use gb_emu::gameboy::GameBoy;

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
