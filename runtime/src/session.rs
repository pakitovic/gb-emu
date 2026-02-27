use crate::audio::{AudioMixer, AudioResamplerQuality, MixerSource};
use crate::timing::FramePacer;
use gb_emu::gameboy::GameBoy;
use std::time::Duration;

/// Shared host/runtime session that wires a `GameBoy` instance with
/// frame pacing and frontend audio mixing.
pub struct RuntimeSession {
    gb: GameBoy,
    pacer: FramePacer,
    audio_mixer: AudioMixer,
}

impl RuntimeSession {
    pub fn new(mut gb: GameBoy, audio_sample_rate_hz: u32) -> Self {
        gb.set_audio_tcycle_stream_enabled(true);

        let mut audio_mixer = AudioMixer::new(audio_sample_rate_hz.max(1));
        audio_mixer.set_source(MixerSource::CoreApu);

        Self {
            gb,
            pacer: FramePacer::default(),
            audio_mixer,
        }
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
        self.consume_emulated_cycles(cycles);
        Some(cycles)
    }

    /// Records consumed emulated cycles into pacing/audio clocks and captures
    /// newly produced core APU t-cycle samples into the runtime mixer queue.
    pub fn consume_emulated_cycles(&mut self, emulated_tcycles: u64) {
        self.pacer.consume_emulated_cycles(emulated_tcycles);
        let tcycle_samples = self.gb.drain_audio_tcycle_samples();
        self.audio_mixer.push_core_tcycle_samples(&tcycle_samples);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use gb_emu::cartridge::Cartridge;
    use gb_emu::timing::DMG_T_CYCLES_PER_SECOND;

    fn make_rom_32kb() -> Vec<u8> {
        let mut rom = vec![0; 32 * 1024];
        rom[0x0147] = 0x00;
        rom[0x0148] = 0x00;
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
}
