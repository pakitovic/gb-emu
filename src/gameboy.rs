use crate::cartridge::Cartridge;
use crate::cartridge::CartridgeError;
use crate::cartridge::CartridgeMetadata;
use crate::cpu::Cpu;
use crate::hardware::HardwareModel;
use crate::input::Button;
use crate::memory::Bus;

const MOONEYE_LOOP_WINDOW: usize = 8;
pub const SCREEN_WIDTH: usize = crate::memory::LCD_WIDTH;
pub const SCREEN_HEIGHT: usize = crate::memory::LCD_HEIGHT;

fn looks_like_tight_loop(pc_window: &[u16; MOONEYE_LOOP_WINDOW]) -> bool {
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

pub struct GameBoy {
    pub cpu: Cpu,
    pub bus: Bus,
}

impl GameBoy {
    pub fn new(cartridge: Cartridge) -> Self {
        Self::new_with_model(cartridge, HardwareModel::default())
    }

    pub fn new_with_model(cartridge: Cartridge, model: HardwareModel) -> Self {
        Self {
            cpu: Cpu::new_with_model(model),
            bus: Bus::new_with_model(cartridge, model),
        }
    }

    // Execute one CPU step
    pub fn step(&mut self) -> u8 {
        self.cpu.step(&mut self.bus)
    }

    pub fn rom_title(&self) -> &str {
        self.bus.rom_title()
    }

    pub fn serial_output(&self) -> &str {
        self.bus.serial_output()
    }

    pub fn frame_counter(&self) -> u64 {
        self.bus.frame_counter()
    }

    pub fn framebuffer(&self) -> &[u8; SCREEN_WIDTH * SCREEN_HEIGHT] {
        self.bus.framebuffer()
    }

    pub fn flush_battery_save(&mut self) -> Result<(), CartridgeError> {
        self.bus.flush_battery_save()
    }

    pub fn cartridge_metadata(&self) -> CartridgeMetadata {
        self.bus.cartridge_metadata()
    }

    pub fn cartridge_has_rumble(&self) -> bool {
        self.bus.cartridge_has_rumble()
    }

    pub fn rumble_active(&self) -> bool {
        self.bus.rumble_active()
    }

    pub fn drain_audio_tcycle_samples(&mut self) -> Vec<f32> {
        self.bus.drain_audio_tcycle_samples()
    }

    pub fn run_frame_with_limit(&mut self, trace: bool, max_steps: usize) -> Option<u64> {
        let start_frame = self.frame_counter();
        let mut total_cycles = 0u64;
        for _ in 0..max_steps {
            let cycles = self.step();
            total_cycles = total_cycles.wrapping_add(cycles as u64);
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, cycles: {}",
                    self.cpu.registers.pc, self.cpu.registers.a, cycles
                );
            }
            if self.frame_counter() != start_frame {
                return Some(total_cycles);
            }
        }
        None
    }

    pub fn set_button_pressed(&mut self, button: Button, pressed: bool) {
        self.bus.set_button_pressed(button, pressed);
    }

    // Main execution loop
    pub fn run(&mut self, trace: bool) {
        println!("ROM: {}", self.bus.rom_title());
        loop {
            let cycles = self.step();
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, cycles: {}",
                    self.cpu.registers.pc, self.cpu.registers.a, cycles
                );
            }
        }
    }

    pub fn run_blargg(&mut self, max_steps: usize, trace: bool) -> Option<String> {
        println!("ROM: {}", self.bus.rom_title());
        for _ in 0..max_steps {
            let cycles = self.step();
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, cycles: {}",
                    self.cpu.registers.pc, self.cpu.registers.a, cycles
                );
            }

            let serial = self.bus.serial_output();
            if serial.contains("Passed") {
                return Some("Passed".to_string());
            }
            if serial.contains("Failed") {
                return Some("Failed".to_string());
            }

            // Blargg memory protocol fallback:
            // A001..A003 == DE B0 61, A000 == status (0 pass, non-zero fail, 0x80 running).
            let sig_ok = self.bus.read_byte(0xA001) == 0xDE
                && self.bus.read_byte(0xA002) == 0xB0
                && self.bus.read_byte(0xA003) == 0x61;
            if sig_ok {
                let status = self.bus.read_byte(0xA000);
                if status == 0x00 {
                    return Some("Passed".to_string());
                }
                if status != 0x80 {
                    return Some("Failed".to_string());
                }
            }
        }
        None
    }

    pub fn run_mooneye(&mut self, max_steps: usize, trace: bool) -> Option<String> {
        println!("ROM: {}", self.bus.rom_title());
        let mut pc_window = [0u16; MOONEYE_LOOP_WINDOW];
        let mut pc_window_len = 0usize;
        let mut pc_window_pos = 0usize;

        for _ in 0..max_steps {
            let cycles = self.step();
            if trace {
                println!(
                    "PC: {:04X}, A: {:02X}, B: {:02X}, C: {:02X}, D: {:02X}, E: {:02X}, H: {:02X}, L: {:02X}, cycles: {}",
                    self.cpu.registers.pc,
                    self.cpu.registers.a,
                    self.cpu.registers.b,
                    self.cpu.registers.c,
                    self.cpu.registers.d,
                    self.cpu.registers.e,
                    self.cpu.registers.h,
                    self.cpu.registers.l,
                    cycles
                );
            }

            let pc = self.cpu.registers.pc;
            pc_window[pc_window_pos] = pc;
            pc_window_pos = (pc_window_pos + 1) % MOONEYE_LOOP_WINDOW;
            if pc_window_len < MOONEYE_LOOP_WINDOW {
                pc_window_len += 1;
            }

            // Mooneye acceptance convention:
            // - Success signature in B,C,D,E,H,L: 3,5,8,13,21,34
            // - Failure signature in B,C,D,E,H,L: 0x42,0x42,0x42,0x42,0x42,0x42
            //
            // Final signatures are expected to be observed in a tight loop.
            // This avoids false negatives in tests where intermediate values
            // can temporarily match the failure signature.
            let regs = (
                self.cpu.registers.b,
                self.cpu.registers.c,
                self.cpu.registers.d,
                self.cpu.registers.e,
                self.cpu.registers.h,
                self.cpu.registers.l,
            );
            let in_tight_loop =
                pc_window_len == MOONEYE_LOOP_WINDOW && looks_like_tight_loop(&pc_window);
            if regs == (3, 5, 8, 13, 21, 34) && in_tight_loop {
                return Some("Passed".to_string());
            }
            if regs == (0x42, 0x42, 0x42, 0x42, 0x42, 0x42) && in_tight_loop {
                return Some("Failed".to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::Cartridge;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00; // ROM-only
        rom[0x0148] = 0x00; // 32KB
        rom
    }

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

    #[test]
    fn run_frame_with_limit_returns_none_if_budget_is_too_small() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let mut gb = GameBoy::new(cartridge);
        let start = gb.frame_counter();

        let result = gb.run_frame_with_limit(false, 1);

        assert!(result.is_none());
        assert_eq!(gb.frame_counter(), start);
    }

    #[test]
    fn run_frame_with_limit_advances_frame_counter() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let mut gb = GameBoy::new(cartridge);
        let start = gb.frame_counter();

        let cycles = gb
            .run_frame_with_limit(false, 50_000)
            .expect("frame should be produced within step budget");

        assert!(cycles > 0);
        assert!(gb.frame_counter() > start);
    }
}
