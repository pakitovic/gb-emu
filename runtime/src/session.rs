use crate::audio::{AudioMixer, AudioResamplerQuality, MixerSource};
use crate::timing::FramePacer;
use gb_emu::gameboy::{GameBoy, SCREEN_HEIGHT, SCREEN_WIDTH};
use gb_emu::palette_override::PaletteOverrideDb;
use gb_emu::sgb::{
    CMD_ATRC_EN, CMD_ATTR_SET, CMD_ATTR_TRN, CMD_CHR_TRN, CMD_DATA_SND, CMD_DATA_TRN, CMD_ICON_EN,
    CMD_JUMP, CMD_MASK_EN, CMD_OBJ_TRN, CMD_PAL_PRI, CMD_PAL_SET, CMD_PAL_TRN, CMD_PCT_TRN,
    CMD_TEST_EN, SgbBorderRenderer, SgbColorizer, SgbLink, SgbState,
    decode_sgb_transfer_from_framebuffer,
};
use std::time::Duration;

const SGB_TRANSFER_BLOCK_BYTES: usize = 0x1000;
const SGB_BORDER_WIDTH: usize = 256;
const SGB_BORDER_HEIGHT: usize = 224;
const SGB_TRANSFER_FRAME_COUNT: u8 = 5;
const SGB_TRANSFER_VISIBLE_TILEMAP_WIDTH: usize = 20;
const SGB_TRANSFER_VISIBLE_TILEMAP_HEIGHT: usize = 18;

#[derive(Debug, Clone)]
struct PendingSgbTransfer {
    command_id: u8,
    payload: Vec<u8>,
    frames_remaining: u8,
    sampled_transfer: [u8; SGB_TRANSFER_BLOCK_BYTES],
    sampled_transfer_valid: bool,
    used_exact_transfer_frame: bool,
    sampled_transfer_non_zero_bytes: usize,
}

impl PendingSgbTransfer {
    fn new(command_id: u8, payload: &[u8]) -> Self {
        Self {
            command_id,
            payload: payload.to_vec(),
            frames_remaining: SGB_TRANSFER_FRAME_COUNT,
            sampled_transfer: [0; SGB_TRANSFER_BLOCK_BYTES],
            sampled_transfer_valid: false,
            used_exact_transfer_frame: false,
            sampled_transfer_non_zero_bytes: 0,
        }
    }
}

fn update_pending_transfer_sample(
    entry: &mut PendingSgbTransfer,
    transfer: &[u8; SGB_TRANSFER_BLOCK_BYTES],
    non_zero_bytes: usize,
    exact_transfer_frame: bool,
) {
    if exact_transfer_frame {
        if !entry.used_exact_transfer_frame
            || !entry.sampled_transfer_valid
            || non_zero_bytes >= entry.sampled_transfer_non_zero_bytes
        {
            entry.sampled_transfer = *transfer;
            entry.sampled_transfer_valid = true;
            entry.used_exact_transfer_frame = true;
            entry.sampled_transfer_non_zero_bytes = non_zero_bytes;
        }
        return;
    }

    if entry.used_exact_transfer_frame {
        return;
    }

    if !entry.sampled_transfer_valid || non_zero_bytes > entry.sampled_transfer_non_zero_bytes {
        entry.sampled_transfer = *transfer;
        entry.sampled_transfer_valid = true;
        entry.sampled_transfer_non_zero_bytes = non_zero_bytes;
    }
}

const fn is_runtime_transfer_command(command_id: u8) -> bool {
    matches!(
        command_id,
        CMD_ATTR_TRN | CMD_PAL_TRN | CMD_DATA_TRN | CMD_CHR_TRN | CMD_PCT_TRN | CMD_OBJ_TRN
    )
}

const fn prefers_signal_transfer_fallback(command_id: u8) -> bool {
    matches!(command_id, CMD_PAL_TRN | CMD_ATTR_TRN)
}

const fn is_runtime_supported_sgb_command(command_id: u8) -> bool {
    matches!(
        command_id,
        0x00..=0x07
            | CMD_PAL_SET
            | CMD_PAL_TRN
            | CMD_ATRC_EN
            | CMD_TEST_EN
            | CMD_ICON_EN
            | CMD_DATA_SND
            | CMD_DATA_TRN
            | CMD_JUMP
            | CMD_CHR_TRN
            | CMD_PCT_TRN
            | CMD_ATTR_TRN
            | CMD_ATTR_SET
            | CMD_MASK_EN
            | CMD_OBJ_TRN
            | CMD_PAL_PRI
    )
}

/// Shared host/runtime session that wires a `GameBoy` instance with
/// frame pacing and frontend audio mixing.
pub struct RuntimeSession {
    gb: GameBoy,
    pacer: FramePacer,
    audio_mixer: AudioMixer,
    sgb_enabled: bool,
    sgb_command_transport_enabled: bool,
    sgb_active: bool,
    sgb_link: SgbLink,
    sgb_state: SgbState,
    sgb_colorizer: SgbColorizer,
    sgb_border_renderer: SgbBorderRenderer,
    pending_sgb_transfers: Vec<PendingSgbTransfer>,
}

impl RuntimeSession {
    pub fn new(mut gb: GameBoy, audio_sample_rate_hz: u32) -> Self {
        let sgb_model_enabled = gb.hardware_model().supports_sgb_features();
        // Real cart-driven SGB command transport is gated by the cartridge SGB header flag.
        let sgb_command_transport_enabled =
            gb.cartridge_model_compatibility().sgb_features_requested;
        let sgb_enabled = sgb_model_enabled || sgb_command_transport_enabled;
        gb.set_audio_tcycle_stream_enabled(true);

        let mut audio_mixer = AudioMixer::new(audio_sample_rate_hz.max(1));
        audio_mixer.set_source(MixerSource::CoreApu);

        let mut session = Self {
            gb,
            pacer: FramePacer::default(),
            audio_mixer,
            sgb_enabled,
            sgb_command_transport_enabled,
            sgb_active: false,
            sgb_link: SgbLink::new(),
            sgb_state: SgbState::new(),
            sgb_colorizer: SgbColorizer::new(),
            sgb_border_renderer: SgbBorderRenderer::new(),
            pending_sgb_transfers: Vec::new(),
        };

        if sgb_model_enabled {
            let _ = session.sgb_state.apply_built_in_boot_palette(
                session.gb.rom_title(),
                session.gb.rom_header_crc32(),
                None,
            );
            session.sgb_active = true;
        }

        session
    }

    pub fn gameboy(&self) -> &GameBoy {
        &self.gb
    }

    pub fn gameboy_mut(&mut self) -> &mut GameBoy {
        &mut self.gb
    }

    pub fn push_host_time(&mut self, elapsed: Duration) {
        self.pacer.push_host_time(elapsed);
    }

    pub fn has_frame_budget(&self) -> bool {
        self.pacer.has_frame_budget()
    }

    pub fn frame_budget_count(&self) -> u32 {
        self.pacer.frame_budget_count()
    }

    pub fn duration_until_next_frame(&self) -> Duration {
        self.pacer.duration_until_next_frame()
    }

    pub fn audio_clock_tcycles(&self) -> u64 {
        self.pacer.audio_clock_tcycles()
    }

    pub fn drain_audio_tcycles(&mut self) -> u64 {
        self.pacer.drain_audio_tcycles()
    }

    pub fn run_frame_with_limit(&mut self, frame_step_limit: usize) -> Option<u64> {
        let cycles = self.gb.run_frame_with_limit(frame_step_limit)?;
        self.capture_pending_sgb_transfers();
        self.consume_emulated_cycles(cycles);
        Some(cycles)
    }

    pub fn sgb_active(&mut self) -> bool {
        self.process_pending_sgb_events();
        self.sgb_active
    }

    pub fn sgb_rgb_frame(&mut self) -> Option<&[u8]> {
        if !self.sgb_enabled {
            return None;
        }

        self.process_pending_sgb_events();
        if !self.sgb_active {
            return None;
        }
        self.refresh_live_obj_overlay_from_vram();

        Some(self.sgb_colorizer.colorize_rgb_frame(
            self.gb.framebuffer(),
            &self.sgb_state,
            (self.gb.bus.read_byte(0xFF40) & 0x80) != 0,
        ))
    }

    pub fn sgb_border_rgb_frame(&mut self) -> Option<&[u8]> {
        if !self.sgb_enabled {
            return None;
        }

        self.process_pending_sgb_events();
        if !self.sgb_active {
            return None;
        }
        self.refresh_live_obj_overlay_from_vram();

        let gb_rgb = self.sgb_colorizer.colorize_rgb_frame(
            self.gb.framebuffer(),
            &self.sgb_state,
            (self.gb.bus.read_byte(0xFF40) & 0x80) != 0,
        );
        self.sgb_border_renderer
            .compose_frame(gb_rgb, &self.sgb_state)
    }

    pub fn sgb_presented_rgb_frame(&mut self) -> Option<&[u8]> {
        if !self.sgb_enabled {
            return None;
        }

        self.process_pending_sgb_events();
        if !self.sgb_active {
            return None;
        }
        self.refresh_live_obj_overlay_from_vram();

        let gb_rgb = self.sgb_colorizer.colorize_rgb_frame(
            self.gb.framebuffer(),
            &self.sgb_state,
            (self.gb.bus.read_byte(0xFF40) & 0x80) != 0,
        );
        if self.sgb_state.has_presented_overlay() {
            return self
                .sgb_border_renderer
                .compose_frame(gb_rgb, &self.sgb_state);
        }

        Some(gb_rgb)
    }

    pub const fn sgb_border_frame_size() -> (usize, usize) {
        (SGB_BORDER_WIDTH, SGB_BORDER_HEIGHT)
    }

    pub const fn dmg_frame_size() -> (usize, usize) {
        (SCREEN_WIDTH, SCREEN_HEIGHT)
    }

    pub fn sgb_presented_frame_size(&mut self) -> Option<(usize, usize)> {
        if !self.sgb_enabled {
            return None;
        }

        self.process_pending_sgb_events();
        if !self.sgb_active {
            return None;
        }

        if self.sgb_state.has_presented_overlay() {
            Some(Self::sgb_border_frame_size())
        } else {
            Some(Self::dmg_frame_size())
        }
    }

    pub fn frame_step_timeout_diagnostics(&self) -> String {
        let registers = self.gb.cpu().registers();
        let lcdc = self.gb.bus.read_byte(0xFF40);
        let stat = self.gb.bus.read_byte(0xFF41);
        let ly = self.gb.bus.read_byte(0xFF44);
        let lyc = self.gb.bus.read_byte(0xFF45);
        let ie = self.gb.bus.interrupt_enable();
        let iflags = self.gb.bus.interrupt_flags();
        let pending = self.gb.bus.pending_interrupts();
        let recent_pc = self
            .gb
            .recent_pc_trace()
            .into_iter()
            .map(|pc| format!("{pc:04X}"))
            .collect::<Vec<_>>()
            .join(",");
        let recent_mmio = self
            .gb
            .recent_key_mmio_writes()
            .into_iter()
            .map(|(addr, value)| format!("{addr:04X}:{value:02X}"))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "frame_counter={} pc={:04X} sp={:04X} af={:02X}{:02X} bc={:02X}{:02X} de={:02X}{:02X} hl={:02X}{:02X} lcdc={:02X} stat={:02X} ly={:02X} lyc={:02X} ie={:02X} if={:02X} pending={:02X} recent_pc=[{}] recent_mmio=[{}]",
            self.gb.frame_counter(),
            registers.pc,
            registers.sp,
            registers.a,
            registers.f,
            registers.b,
            registers.c,
            registers.d,
            registers.e,
            registers.h,
            registers.l,
            lcdc,
            stat,
            ly,
            lyc,
            ie,
            iflags,
            pending,
            recent_pc,
            recent_mmio
        )
    }

    /// Records consumed emulated cycles into pacing/audio clocks and captures
    /// newly produced core APU t-cycle samples into the runtime mixer queue.
    pub fn consume_emulated_cycles(&mut self, emulated_tcycles: u64) {
        self.pacer.consume_emulated_cycles(emulated_tcycles);
        self.process_pending_sgb_events();
        let tcycle_samples = self.gb.drain_audio_tcycle_samples();
        self.audio_mixer.push_core_tcycle_samples(&tcycle_samples);
    }

    fn process_pending_sgb_events(&mut self) {
        if !self.sgb_enabled || !self.sgb_command_transport_enabled {
            return;
        }

        for event in self.gb.drain_key_mmio_write_events() {
            let Some(command) = self.sgb_link.on_key_mmio_write(event.addr, event.value) else {
                continue;
            };
            if !is_runtime_supported_sgb_command(command.command_id) {
                continue;
            }
            if is_runtime_transfer_command(command.command_id) {
                let payload = command.payload_bytes();
                self.pending_sgb_transfers
                    .push(PendingSgbTransfer::new(command.command_id, &payload));
            }
            self.sgb_state.apply_command(&command);
            self.sgb_active = true;
        }
    }

    fn capture_pending_sgb_transfers(&mut self) {
        if self.pending_sgb_transfers.is_empty() {
            return;
        }

        let signal_transfer = decode_sgb_transfer_from_framebuffer(self.gb.framebuffer());
        let exact_transfer = self.capture_exact_sgb_transfer_block();
        let mut raw_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        let raw_transfer_valid = self.gb.copy_vram_hardware_block(0x8000, &mut raw_transfer);
        let raw_transfer_non_zero_bytes = raw_transfer.iter().filter(|&&byte| byte != 0).count();
        let signal_transfer_non_zero_bytes =
            signal_transfer.iter().filter(|&&byte| byte != 0).count();
        let mut pending = Vec::with_capacity(self.pending_sgb_transfers.len());

        for mut entry in self.pending_sgb_transfers.drain(..) {
            if let Some(transfer) = exact_transfer {
                let exact_non_zero_bytes = transfer.iter().filter(|&&byte| byte != 0).count();
                update_pending_transfer_sample(&mut entry, &transfer, exact_non_zero_bytes, true);
            } else if !entry.used_exact_transfer_frame {
                if prefers_signal_transfer_fallback(entry.command_id) {
                    update_pending_transfer_sample(
                        &mut entry,
                        &signal_transfer,
                        signal_transfer_non_zero_bytes,
                        false,
                    );
                } else if raw_transfer_valid
                    && raw_transfer_non_zero_bytes >= signal_transfer_non_zero_bytes
                {
                    update_pending_transfer_sample(
                        &mut entry,
                        &raw_transfer,
                        raw_transfer_non_zero_bytes,
                        false,
                    );
                } else {
                    update_pending_transfer_sample(
                        &mut entry,
                        &signal_transfer,
                        signal_transfer_non_zero_bytes,
                        false,
                    );
                }
            }

            if entry.frames_remaining > 0 {
                entry.frames_remaining -= 1;
            }
            if entry.frames_remaining != 0 || !entry.sampled_transfer_valid {
                pending.push(entry);
                continue;
            }

            match entry.command_id {
                CMD_ATTR_TRN => {
                    let _ = self
                        .sgb_state
                        .load_attr_files_from_vram_transfer(&entry.sampled_transfer);
                }
                CMD_PAL_TRN => {
                    let _ = self
                        .sgb_state
                        .load_system_palettes_from_vram_transfer(&entry.sampled_transfer);
                }
                CMD_DATA_TRN => {
                    let destination = if entry.payload.len() >= 2 {
                        u16::from_le_bytes([entry.payload[0], entry.payload[1]])
                    } else {
                        0
                    };
                    let _ = self
                        .sgb_state
                        .load_data_trn_from_vram_transfer(&entry.sampled_transfer, destination);
                }
                CMD_CHR_TRN => {
                    let high_tile_block = entry.payload.first().copied().unwrap_or(0) & 0x01 != 0;
                    let _ = self.sgb_state.load_border_chr_from_vram_transfer(
                        &entry.sampled_transfer,
                        high_tile_block,
                    );
                    let _ = self
                        .sgb_state
                        .load_obj_chr_from_vram_transfer(&entry.sampled_transfer, high_tile_block);
                }
                CMD_PCT_TRN => {
                    let _ = self
                        .sgb_state
                        .load_border_pct_from_vram_transfer(&entry.sampled_transfer);
                }
                CMD_OBJ_TRN => {
                    let _ = self
                        .sgb_state
                        .load_obj_from_vram_transfer(&entry.sampled_transfer);
                }
                _ => {}
            }
        }

        self.pending_sgb_transfers = pending;
    }

    fn capture_exact_sgb_transfer_block(&self) -> Option<[u8; SGB_TRANSFER_BLOCK_BYTES]> {
        let lcdc = self.gb.bus.read_byte(0xFF40);
        if (lcdc & 0x80) == 0 || (lcdc & 0x01) == 0 || (lcdc & 0x10) == 0 {
            return None;
        }
        if self.gb.bus.read_byte(0xFF42) != 0 || self.gb.bus.read_byte(0xFF43) != 0 {
            return None;
        }
        if self.gb.bus.read_byte(0xFF47) != 0xE4 {
            return None;
        }

        let map_base = if (lcdc & 0x08) != 0 { 0x9C00 } else { 0x9800 };
        let mut tilemap = [0u8; SGB_TRANSFER_VISIBLE_TILEMAP_HEIGHT * 32];
        if !self.gb.copy_vram_hardware_block(map_base, &mut tilemap) {
            return None;
        }
        for tile_y in 0..SGB_TRANSFER_VISIBLE_TILEMAP_HEIGHT {
            for tile_x in 0..SGB_TRANSFER_VISIBLE_TILEMAP_WIDTH {
                let expected = (tile_y * SGB_TRANSFER_VISIBLE_TILEMAP_WIDTH + tile_x) as u8;
                if tilemap[tile_y * 32 + tile_x] != expected {
                    return None;
                }
            }
        }

        let mut transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        self.gb
            .copy_vram_hardware_block(0x8000, &mut transfer)
            .then_some(transfer)
    }

    pub fn audio_sample_rate_hz(&self) -> u32 {
        self.audio_mixer.sample_rate_hz()
    }

    pub fn audio_source(&self) -> MixerSource {
        self.audio_mixer.source()
    }

    pub fn set_audio_source(&mut self, source: MixerSource) {
        self.audio_mixer.set_source(source);
    }

    pub fn audio_resampler_quality(&self) -> AudioResamplerQuality {
        self.audio_mixer.core_resampler_quality()
    }

    pub fn set_audio_resampler_quality(&mut self, quality: AudioResamplerQuality) {
        self.audio_mixer.set_core_resampler_quality(quality);
    }

    pub fn set_audio_sample_rate_hz(&mut self, sample_rate_hz: u32) {
        self.audio_mixer.set_sample_rate_hz(sample_rate_hz.max(1));
    }

    pub fn pending_audio_output_samples(&self) -> u64 {
        self.audio_mixer.pending_samples()
    }

    pub fn drain_audio_samples(&mut self, max_samples: usize) -> Vec<f32> {
        let pending_tcycles = self.pacer.drain_audio_tcycles();
        self.audio_mixer
            .drain_synced_samples(pending_tcycles, max_samples)
    }

    pub fn drain_audio_realtime_block(&mut self, block_samples: usize) -> Vec<f32> {
        let pending_tcycles = self.pacer.drain_audio_tcycles();
        self.audio_mixer
            .drain_realtime_block(pending_tcycles, block_samples)
    }

    pub fn apply_palette_overrides(&mut self, overrides: Option<&PaletteOverrideDb>) -> bool {
        if !self.gb.hardware_model().supports_sgb_features() {
            return false;
        }
        if self.sgb_state.last_applied_command_id().is_some() {
            return false;
        }

        let matched = self.sgb_state.apply_built_in_boot_palette(
            self.gb.rom_title(),
            self.gb.rom_header_crc32(),
            overrides,
        );
        self.sgb_active = true;
        let _ = matched;
        true
    }

    fn refresh_live_obj_overlay_from_vram(&mut self) {
        if !self.sgb_enabled || !self.sgb_active || !self.sgb_state.has_obj_overlay() {
            return;
        }

        let mut transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        let _ = self.gb.copy_vram_hardware_block(0x8000, &mut transfer);
        let _ = self.sgb_state.load_obj_from_vram_transfer(&transfer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_emu::cartridge::Cartridge;
    use gb_emu::hardware::HardwareModel;
    use gb_emu::sgb::{
        CMD_CHR_TRN, CMD_DATA_TRN, CMD_MASK_EN, CMD_OBJ_TRN, CMD_PAL_SET, CMD_PAL_TRN, CMD_PAL01,
        CMD_PCT_TRN,
    };
    use gb_emu::timing::DMG_T_CYCLES_PER_SECOND;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
        rom
    }

    fn make_rom_with_title_32kb(title: &str) -> Vec<u8> {
        let mut rom = make_rom_32kb();
        let title_bytes = title.as_bytes();
        let copy_len = title_bytes.len().min(16);
        rom[0x0134..0x0134 + copy_len].copy_from_slice(&title_bytes[..copy_len]);
        rom
    }

    fn make_sgb_enhanced_rom_32kb() -> Vec<u8> {
        let mut rom = make_rom_32kb();
        rom[0x0146] = 0x03;
        rom
    }

    #[test]
    fn runtime_session_routes_frame_audio_to_runtime_mixer() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new(cartridge);
        let mut session = RuntimeSession::new(gb, 48_000);

        {
            let gb = session.gameboy_mut();
            gb.bus.write_byte(0xFF26, 0x00);
            gb.bus.write_byte(0xFF26, 0x80);
            gb.bus.write_byte(0xFF24, 0x77);
            gb.bus.write_byte(0xFF25, 0x11);
            gb.bus.write_byte(0xFF11, 0x80);
            gb.bus.write_byte(0xFF12, 0xF0);
            gb.bus.write_byte(0xFF13, 0xFC);
            gb.bus.write_byte(0xFF14, 0x87);
        }

        let ran = session.run_frame_with_limit(250_000);
        assert!(ran.is_some());

        let samples = session.drain_audio_realtime_block(512);
        assert_eq!(samples.len(), 1_024);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().any(|sample| sample.abs() > 0.0));
    }

    #[test]
    fn runtime_session_audio_drain_consumes_pending_pacer_budget() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new(cartridge);
        let mut session = RuntimeSession::new(gb, 48_000);

        session.set_audio_source(MixerSource::TestTone);
        session.consume_emulated_cycles(DMG_T_CYCLES_PER_SECOND / 100);

        let samples = session.drain_audio_samples(10_000);
        assert!(!samples.is_empty());
        assert!(samples.iter().any(|sample| *sample != 0.0));
        assert_eq!(session.drain_audio_tcycles(), 0);
        assert_eq!(session.pending_audio_output_samples(), 0);
    }

    #[test]
    fn runtime_session_frame_timeout_diagnostics_exposes_core_state_fields() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new(cartridge);
        let session = RuntimeSession::new(gb, 48_000);

        let diagnostics = session.frame_step_timeout_diagnostics();

        assert!(diagnostics.contains("frame_counter="));
        assert!(diagnostics.contains("pc="));
        assert!(diagnostics.contains("lcdc="));
        assert!(diagnostics.contains("stat="));
        assert!(diagnostics.contains("ly="));
        assert!(diagnostics.contains("ie="));
        assert!(diagnostics.contains("if="));
        assert!(diagnostics.contains("pending="));
        assert!(diagnostics.contains("recent_pc=["));
        assert!(diagnostics.contains("recent_mmio=["));
    }

    fn feed_sgb_packet_via_p1(gb: &mut GameBoy, packet: &[u8; 16]) {
        gb.bus.write_byte(0xFF00, 0x00);
        for byte in packet {
            for bit in 0..8 {
                let bit_value = (byte >> bit) & 0x01;
                let p1_write = if bit_value == 0 { 0x20 } else { 0x10 };
                gb.bus.write_byte(0xFF00, p1_write);
            }
        }
        gb.bus.write_byte(0xFF00, 0x20);
    }

    fn make_single_packet_command(command_id: u8, payload: &[u8]) -> [u8; 16] {
        let mut packet = [0u8; 16];
        packet[0] = (command_id << 3) | 0x01;
        for (index, value) in payload.iter().copied().enumerate() {
            if index + 1 >= packet.len() {
                break;
            }
            packet[index + 1] = value;
        }
        packet
    }

    fn write_sgb_transfer_block(gb: &mut GameBoy, transfer: &[u8; SGB_TRANSFER_BLOCK_BYTES]) {
        for (offset, byte) in transfer.iter().copied().enumerate() {
            gb.bus.write_byte(0x8000 + offset as u16, byte);
        }
        for tile_index in 0..256u16 {
            let map_x = tile_index % 20;
            let map_y = tile_index / 20;
            gb.bus
                .write_byte(0x9800 + map_y * 32 + map_x, tile_index as u8);
        }
        gb.bus.write_byte(0xFF42, 0x00);
        gb.bus.write_byte(0xFF43, 0x00);
        gb.bus.write_byte(0xFF47, 0xE4);
        gb.bus.write_byte(0xFF40, 0x91);
    }

    fn advance_runtime_frame(session: &mut RuntimeSession) {
        let ran = session.run_frame_with_limit(250_000);
        assert!(ran.is_some(), "expected runtime frame to advance");
    }

    fn advance_runtime_transfer_window(session: &mut RuntimeSession) {
        for _ in 0..SGB_TRANSFER_FRAME_COUNT {
            advance_runtime_frame(session);
        }
    }

    fn make_obj_transfer(x: u8, y: u8, attrs: u8) -> [u8; SGB_TRANSFER_BLOCK_BYTES] {
        let mut transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        for row in 0..8 {
            transfer[row * 2] = 0xFF; // plane 0 -> color index 1
        }
        transfer[0x0F90] = x;
        transfer[0x0F91] = y;
        transfer[0x0F92] = 0;
        transfer[0x0F93] = attrs;
        transfer
    }

    #[test]
    fn runtime_session_decodes_sgb_packets_and_exposes_colored_frame() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let mut packet = [0u8; 16];
        packet[0] = (CMD_PAL01 << 3) | 0x01;
        packet[1..15].copy_from_slice(&[
            0x00, 0x00, // shared color
            0x1F, 0x00, // palette 0 color 1 (red)
            0x00, 0x00, // palette 0 color 2
            0x00, 0x00, // palette 0 color 3
            0x1F, 0x00, // palette 1 color 1
            0x00, 0x00, // palette 1 color 2
            0x00, 0x00, // palette 1 color 3
        ]);

        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);
        let frame_len = session
            .sgb_rgb_frame()
            .expect("SGB frame should be available after a decoded command")
            .len();

        assert!(session.sgb_active());
        assert_eq!(frame_len, 160 * 144 * 3);
        assert_eq!(session.sgb_presented_frame_size(), Some((160, 144)));
    }

    #[test]
    fn runtime_session_auto_detects_sgb_packets_for_sgb_header_cartridge() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Dmg);
        let mut session = RuntimeSession::new(gb, 48_000);
        let packet = make_single_packet_command(CMD_MASK_EN, &[0x00]);

        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);

        assert!(session.sgb_active());
        assert!(session.sgb_rgb_frame().is_some());
        assert_eq!(session.sgb_presented_frame_size(), Some((160, 144)));
    }

    #[test]
    fn runtime_session_ignores_sgb_packets_for_plain_dmg_cartridge() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Dmg);
        let mut session = RuntimeSession::new(gb, 48_000);
        let packet = make_single_packet_command(CMD_MASK_EN, &[0x00]);

        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);

        assert!(!session.sgb_active());
        assert!(session.sgb_rgb_frame().is_none());
        assert!(session.sgb_presented_frame_size().is_none());
    }

    #[test]
    fn runtime_session_ignores_sgb_packets_for_plain_dmg_cartridge_on_sgb_model() {
        let cartridge = Cartridge::from_bytes(make_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);
        let before = session
            .sgb_rgb_frame()
            .expect("SGB model should expose boot palette frame")
            .to_vec();
        let packet = make_single_packet_command(
            CMD_PAL01,
            &[
                0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        );

        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);

        let after = session
            .sgb_rgb_frame()
            .expect("boot palette frame should remain available");
        let after = after.to_vec();
        assert!(session.sgb_active());
        assert_eq!(before, after);
        assert_eq!(session.sgb_presented_frame_size(), Some((160, 144)));
    }

    #[test]
    fn runtime_session_ignores_unsupported_boot_like_packets() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);
        let before = session
            .sgb_rgb_frame()
            .expect("SGB model should expose boot palette frame")
            .to_vec();
        let packet = make_single_packet_command(0x1E, &[0x00]);

        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);

        let after = session
            .sgb_rgb_frame()
            .expect("unsupported packets should not disable boot palette output");
        let after = after.to_vec();
        assert!(session.sgb_active());
        assert_eq!(before, after);
        assert_eq!(session.sgb_presented_frame_size(), Some((160, 144)));
    }

    #[test]
    fn runtime_session_applies_title_specific_sgb_boot_palette_for_non_enhanced_kirby() {
        let cartridge = Cartridge::from_bytes(make_rom_with_title_32kb("KIRBY DREAM LAND"))
            .expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let rgb = session
            .sgb_rgb_frame()
            .expect("SGB boot palette frame should be available");
        let rgb = rgb.to_vec();

        assert!(session.sgb_active());
        assert_eq!(&rgb[0..3], &[0xFF, 0xC6, 0xFF]);
        assert_eq!(session.sgb_presented_frame_size(), Some((160, 144)));
    }

    #[test]
    fn runtime_session_applies_external_palette_override_to_sgb_boot_palette_before_cart_commands()
    {
        let cartridge = Cartridge::from_bytes(make_rom_with_title_32kb("UNKNOWN GAME"))
            .expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);
        let before = session
            .sgb_rgb_frame()
            .expect("SGB boot palette frame should be available")
            .to_vec();
        let overrides = PaletteOverrideDb::parse_ini(&format!(
            "[gb.override.{:08X}]\npal[0]=0x112233\n",
            session.gameboy().rom_header_crc32()
        ))
        .expect("override INI should parse");

        assert!(session.apply_palette_overrides(Some(&overrides)));
        let overridden = session
            .sgb_rgb_frame()
            .expect("override boot palette frame should remain available")
            .to_vec();
        assert_eq!(&overridden[0..3], &[0x10, 0x21, 0x31]);

        assert!(session.apply_palette_overrides(None));
        let restored = session
            .sgb_rgb_frame()
            .expect("default boot palette frame should remain available")
            .to_vec();
        assert_eq!(restored, before);
    }

    #[test]
    fn runtime_session_does_not_override_cart_driven_sgb_palette_state() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);
        let packet = make_single_packet_command(
            CMD_PAL01,
            &[
                0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        );
        feed_sgb_packet_via_p1(session.gameboy_mut(), &packet);
        let cart_driven = session
            .sgb_rgb_frame()
            .expect("cart-driven SGB frame should be available")
            .to_vec();
        let overrides = PaletteOverrideDb::parse_ini(&format!(
            "[gb.override.{:08X}]\npal[0]=0x112233\n",
            session.gameboy().rom_header_crc32()
        ))
        .expect("override INI should parse");

        assert!(!session.apply_palette_overrides(Some(&overrides)));
        let after = session
            .sgb_rgb_frame()
            .expect("cart-driven SGB frame should remain available")
            .to_vec();
        assert_eq!(after, cart_driven);
    }

    #[test]
    fn runtime_session_processes_pal_trn_before_pal_set() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);
        let mut transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        let palette2_start = 2 * 8;
        for offset in 0..8usize {
            transfer[palette2_start + offset] = if offset % 2 == 0 { 0x1F } else { 0x00 };
        }
        write_sgb_transfer_block(session.gameboy_mut(), &transfer);

        let pal_trn = make_single_packet_command(CMD_PAL_TRN, &[]);
        let pal_set = make_single_packet_command(
            CMD_PAL_SET,
            &[
                2, 0, // pal #0 = system #2
                2, 0, // pal #1 = system #2
                2, 0, // pal #2 = system #2
                2, 0, // pal #3 = system #2
                0, // flags
            ],
        );
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pal_trn);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pal_set);
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        let rgb = session
            .sgb_rgb_frame()
            .expect("SGB frame should be present after PAL_TRN/PAL_SET");
        assert_eq!(&rgb[0..3], &[0xFF, 0x00, 0x00]);
    }

    #[test]
    fn runtime_prefers_signal_transfer_fallback_for_palette_and_attr_commands() {
        assert!(prefers_signal_transfer_fallback(CMD_PAL_TRN));
        assert!(prefers_signal_transfer_fallback(CMD_ATTR_TRN));
        assert!(!prefers_signal_transfer_fallback(CMD_CHR_TRN));
        assert!(!prefers_signal_transfer_fallback(CMD_PCT_TRN));
        assert!(!prefers_signal_transfer_fallback(CMD_OBJ_TRN));
    }

    #[test]
    fn runtime_transfer_sampling_keeps_richest_non_exact_candidate() {
        let mut entry = PendingSgbTransfer::new(CMD_PAL_TRN, &[]);
        let mut rich_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        rich_transfer[0] = 0x12;
        rich_transfer[1] = 0x34;
        let poor_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];

        update_pending_transfer_sample(&mut entry, &rich_transfer, 2, false);
        update_pending_transfer_sample(&mut entry, &poor_transfer, 0, false);

        assert_eq!(entry.sampled_transfer[0], 0x12);
        assert_eq!(entry.sampled_transfer[1], 0x34);
        assert_eq!(entry.sampled_transfer_non_zero_bytes, 2);
        assert!(!entry.used_exact_transfer_frame);
    }

    #[test]
    fn runtime_transfer_sampling_allows_exact_frame_to_override_fallback_candidate() {
        let mut entry = PendingSgbTransfer::new(CMD_PAL_TRN, &[]);
        let mut fallback_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        fallback_transfer[0] = 0x12;
        let mut exact_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        exact_transfer[0] = 0xAB;
        exact_transfer[1] = 0xCD;

        update_pending_transfer_sample(&mut entry, &fallback_transfer, 1, false);
        update_pending_transfer_sample(&mut entry, &exact_transfer, 2, true);

        assert_eq!(entry.sampled_transfer[0], 0xAB);
        assert_eq!(entry.sampled_transfer[1], 0xCD);
        assert_eq!(entry.sampled_transfer_non_zero_bytes, 2);
        assert!(entry.used_exact_transfer_frame);
    }

    #[test]
    fn runtime_session_sgb_transfer_uses_next_frame_window_not_command_frame() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let mut initial_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        initial_transfer[16..24].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        write_sgb_transfer_block(session.gameboy_mut(), &initial_transfer);

        let pal_trn = make_single_packet_command(CMD_PAL_TRN, &[]);
        let pal_set = make_single_packet_command(
            CMD_PAL_SET,
            &[
                2, 0, // pal #0 = system #2
                2, 0, // pal #1 = system #2
                2, 0, // pal #2 = system #2
                2, 0, // pal #3 = system #2
                0, // flags
            ],
        );
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pal_trn);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pal_set);
        advance_runtime_frame(&mut session);

        let mut next_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        next_transfer[16..24].copy_from_slice(&[0x00, 0x7C, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x7C]);
        write_sgb_transfer_block(session.gameboy_mut(), &next_transfer);
        advance_runtime_transfer_window(&mut session);

        let rgb = session
            .sgb_rgb_frame()
            .expect("SGB frame should be present after delayed PAL_TRN/PAL_SET");
        assert_eq!(&rgb[0..3], &[0x00, 0x00, 0xFF]);
    }

    #[test]
    fn runtime_session_processes_chr_trn_and_pct_trn_for_border_frame() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let mut chr_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        for row in 0..8 {
            chr_transfer[row * 2] = 0xFF; // plane 0 -> color index 1
        }
        write_sgb_transfer_block(session.gameboy_mut(), &chr_transfer);
        let chr_trn = make_single_packet_command(CMD_CHR_TRN, &[0x00]);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &chr_trn);
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        let mut pct_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        for entry in 0..(32 * 28) {
            let offset = entry * 2;
            pct_transfer[offset..offset + 2].copy_from_slice(&0x1000u16.to_le_bytes());
        }
        let palette_base = 0x0800;
        pct_transfer[palette_base + 2..palette_base + 4].copy_from_slice(&0x001Fu16.to_le_bytes());
        write_sgb_transfer_block(session.gameboy_mut(), &pct_transfer);
        let pct_trn = make_single_packet_command(CMD_PCT_TRN, &[]);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pct_trn);
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        let border = session
            .sgb_presented_rgb_frame()
            .expect("SGB border frame should be available after CHR_TRN/PCT_TRN");
        let (width, height) = RuntimeSession::sgb_border_frame_size();
        assert_eq!(border.len(), width * height * 3);
        assert_eq!(&border[0..3], &[0xFF, 0x00, 0x00]);
        assert_eq!(session.sgb_presented_frame_size(), Some((width, height)));
    }

    #[test]
    fn runtime_session_processes_obj_trn_for_presented_overlay_frame() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let gb_origin_x = ((SGB_BORDER_WIDTH - SCREEN_WIDTH) / 2) as u8;
        let gb_origin_y = ((SGB_BORDER_HEIGHT - SCREEN_HEIGHT) / 2) as u8;
        let mut palette_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        write_sgb_transfer_block(session.gameboy_mut(), &palette_transfer);
        let pal_trn = make_single_packet_command(CMD_PAL_TRN, &[]);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &pal_trn);

        let obj_transfer = make_obj_transfer(gb_origin_x, gb_origin_y, 0x30);
        write_sgb_transfer_block(session.gameboy_mut(), &obj_transfer);
        let chr_trn = make_single_packet_command(CMD_CHR_TRN, &[0x00]);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &chr_trn);
        let obj_trn = make_single_packet_command(
            CMD_OBJ_TRN,
            &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        );
        feed_sgb_packet_via_p1(session.gameboy_mut(), &obj_trn);
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        let presented = session
            .sgb_presented_rgb_frame()
            .expect("OBJ_TRN should produce a composed SGB frame");
        let (width, height) = RuntimeSession::sgb_border_frame_size();
        assert_eq!(presented.len(), width * height * 3);
        let sprite_pixel = (gb_origin_y as usize * width + gb_origin_x as usize) * 3;
        assert_ne!(
            &presented[sprite_pixel..sprite_pixel + 3],
            &[0x00, 0x00, 0x00]
        );
        assert_eq!(session.sgb_presented_frame_size(), Some((width, height)));
    }

    #[test]
    fn runtime_session_processes_data_trn_transfer_bytes() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let mut transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        transfer[0] = 0x12;
        transfer[1] = 0x34;
        transfer[2] = 0x56;
        write_sgb_transfer_block(session.gameboy_mut(), &transfer);
        let data_trn = make_single_packet_command(CMD_DATA_TRN, &[0x78, 0x56]);
        feed_sgb_packet_via_p1(session.gameboy_mut(), &data_trn);
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        assert_eq!(session.sgb_state.data_transfer_state().destination, 0x5678);
        assert!(session.sgb_state.data_transfer_state().loaded);
        assert_eq!(
            session.sgb_state.data_transfer_state().data[0..3],
            [0x12, 0x34, 0x56]
        );
    }

    #[test]
    fn runtime_session_refreshes_obj_overlay_oam_from_live_vram() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let mut palette_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        write_sgb_transfer_block(session.gameboy_mut(), &palette_transfer);
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(CMD_PAL_TRN, &[]),
        );

        let initial_x = 16u8;
        let y = 16u8;
        let initial_transfer = make_obj_transfer(initial_x, y, 0x30);
        write_sgb_transfer_block(session.gameboy_mut(), &initial_transfer);
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(CMD_CHR_TRN, &[0x00]),
        );
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(
                CMD_OBJ_TRN,
                &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            ),
        );
        advance_runtime_frame(&mut session);
        advance_runtime_transfer_window(&mut session);

        let first = session
            .sgb_presented_rgb_frame()
            .expect("OBJ overlay should be available")
            .to_vec();
        let width = RuntimeSession::sgb_border_frame_size().0;
        let first_pixel = (y as usize * width + initial_x as usize) * 3;
        assert_ne!(&first[first_pixel..first_pixel + 3], &[0x00, 0x00, 0x00]);

        let moved_x = initial_x.saturating_add(8);
        let moved_pixel = (y as usize * width + moved_x as usize) * 3;
        assert_eq!(&first[moved_pixel..moved_pixel + 3], &[0x00, 0x00, 0x00]);
        let moved_transfer = make_obj_transfer(moved_x, y, 0x30);
        write_sgb_transfer_block(session.gameboy_mut(), &moved_transfer);
        let second = session
            .sgb_presented_rgb_frame()
            .expect("OBJ overlay should remain available")
            .to_vec();
        assert_eq!(&second[first_pixel..first_pixel + 3], &[0x00, 0x00, 0x00]);
        assert_ne!(&second[moved_pixel..moved_pixel + 3], &[0x00, 0x00, 0x00]);
    }

    #[test]
    fn runtime_session_renders_large_obj_overlay_tiles() {
        let cartridge =
            Cartridge::from_bytes(make_sgb_enhanced_rom_32kb()).expect("test ROM should load");
        let gb = GameBoy::new_with_model(cartridge, HardwareModel::Sgb);
        let mut session = RuntimeSession::new(gb, 48_000);

        let gb_origin_x = ((SGB_BORDER_WIDTH - SCREEN_WIDTH) / 2) as u8;
        let gb_origin_y = ((SGB_BORDER_HEIGHT - SCREEN_HEIGHT) / 2) as u8;

        let mut palette_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        palette_transfer[0..8].copy_from_slice(&[0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x1F, 0x00]);
        write_sgb_transfer_block(session.gameboy_mut(), &palette_transfer);
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(CMD_PAL_TRN, &[]),
        );

        let mut chr_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        for tile_index in [0usize, 1, 16, 17] {
            let tile_base = tile_index * 32;
            for row in 0..8 {
                chr_transfer[tile_base + row * 2] = 0xFF;
            }
        }
        write_sgb_transfer_block(session.gameboy_mut(), &chr_transfer);
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(CMD_CHR_TRN, &[0x00]),
        );

        let mut obj_transfer = [0u8; SGB_TRANSFER_BLOCK_BYTES];
        obj_transfer[0x0F90] = gb_origin_x;
        obj_transfer[0x0F91] = gb_origin_y;
        obj_transfer[0x0F92] = 0;
        obj_transfer[0x0F93] = 0x30;
        obj_transfer[0x0FFC] = 0b0000_0010;
        write_sgb_transfer_block(session.gameboy_mut(), &obj_transfer);
        feed_sgb_packet_via_p1(
            session.gameboy_mut(),
            &make_single_packet_command(
                CMD_OBJ_TRN,
                &[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            ),
        );

        let presented = session
            .sgb_presented_rgb_frame()
            .expect("large OBJ_TRN should produce a composed SGB frame");
        let width = RuntimeSession::sgb_border_frame_size().0;
        for (x, y) in [
            (gb_origin_x as usize, gb_origin_y as usize),
            (gb_origin_x as usize + 8, gb_origin_y as usize),
            (gb_origin_x as usize, gb_origin_y as usize + 8),
            (gb_origin_x as usize + 8, gb_origin_y as usize + 8),
        ] {
            let pixel = (y * width + x) * 3;
            assert_ne!(&presented[pixel..pixel + 3], &[0x00, 0x00, 0x00]);
        }
    }
}
