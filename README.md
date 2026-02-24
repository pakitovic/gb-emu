# gb-emu

Personal/hobby Game Boy emulator project written in Rust, focused on learning and incremental milestones.

## Current Scope

### Core Emulation (CPU / Bus / Timing / Input)
- CPU core with growing opcode coverage.
- Memory bus + timer/interrupt basics.
- Joypad input API in core with P1 register behavior and joypad interrupt edges.
- Core API bootstrap for portable frontends (frame stepping + framebuffer access).

### PPU / Video (DMG)
- DMG background layer rendering to a grayscale framebuffer.
- DMG window + sprite (OBJ) composition with priority/palette/flip handling.
- Mode 3 background/window pixel FIFO stepping per dot with a 6-dot BG fetch cadence and window trigger/restart timing (WX/WY mid-line writes affect only valid trigger windows).
- Mode 3 OBJ fetch stalls and sprite pixel mixing are stepped per dot with DMG priority/palette rules; OBJ fetch start now waits for BG fetch boundaries for more stable dot arbitration.
- Mode 3 window trigger comparator now queues pending restarts until a valid BG takeover boundary when OBJ fetch ownership delays immediate window restart.
- Mode 3 takeover arbitration now handles FIFO-stall boundaries and queued window-trigger release after active OBJ fetch windows, with regression coverage for VRAM/OAM blocking and STAT mode0 timing shifts under runtime contention.
- Mode 3 fetcher bus-phase modeling now includes an explicit one-dot BG push-stall recovery sleep micro-op before push resumes after FIFO overfill (`>8`), reducing premature OBJ/window handover on FIFO-stall dots and tightening associated STAT/VRAM/OAM timing corner cases.
- Mode 3 BG `Push` micro-op modeling now tracks an explicit latched `RecoverySleep` substate after FIFO stall resolution (before the later push-ready boundary), improving fetcher bus-phase state-machine clarity and tightening regression coverage for the `stalled -> recovery sleep -> push-ready -> TileIndex` sequence.
- Mode 3 BG `Push` takeover-boundary classification now distinguishes normal vs post-recovery push-ready states, reserving the `push-ready` handover boundary to the explicit post-`RecoverySleep` path and avoiding accidental classification on the normal (non-stalled) first-tile push flow.
- Mode 3 takeover arbitration now excludes the FIFO-recovery `Push` sleep dot as a valid BG/OBJ/window handover boundary (while still allowing the stalled `Push` boundary and later push-ready boundary), with regression coverage for window/OBJ timing and mode3 bus/STAT blocking on that edge.
- Mode 3 `Push` substate corner coverage now explicitly checks the `stalled -> recovery sleep -> push-ready` sequence, including shared window/OBJ arbitration and delayed takeover behavior on the first valid post-sleep boundary.
- On shared Mode 3 takeover boundaries, queued window restarts now defer to an immediately-eligible OBJ fetch start, with regression coverage for the resulting STAT/VRAM/OAM blocking behavior on that arbitration edge.
- Mode 3 OBJ/window arbitration now uses the same OBJ fetch-start lookahead as the OBJ fetcher path (including `Push` boundary handling), reducing window/OBJ overlap corruption seen in commercial scenes (e.g. mid-line window restarts around active sprites).
- Mode 3 window restarts now clear any remaining BG fine-scroll discard (`SCX & 7`) so WX-aligned HUD/window lines stay fixed instead of inheriting BG scroll jitter (e.g. Kirby's Dream Land HUD).
- Mode 3 line-start BG fine-scroll discard now advances the OBJ FIFO in lockstep with discarded BG pixels, fixing left-edge sprite column misalignment when `SCX` uses sub-tile offsets (e.g. Super Mario Land at the camera left boundary).
- Mode 3 line duration now grows from runtime OBJ fetch contention (including mid-line OBJ enable/disable effects), reducing reliance on static per-line penalty estimates.
- Additional PPU/DMA timing edge cases: mode0 STAT source enabled during mode3 triggers on HBlank entry, and DMA restart keeps prior transfer running through the full restart-delay window.

### APU / Audio Emulation
- APU core channel state-machine scaffolding: NR52 power control, NR50/NR51 mixer register gating, CH1/CH2/CH3/CH4 trigger/state progression, and DIV-driven frame sequencer stepping (length/sweep/envelope clocks).
- APU output path now supports real-device analog calibration profiles (model defaults plus custom per-device overrides) with per-channel DAC shaping/bias, routing matrix gains, stereo mixer drive, post-analog soft-clip/headroom limiting, low-pass + DC-blocking high-pass filtering, and selectable linear/cubic (Catmull-Rom with linear edge fallback) t-cycle-to-PCM resampling.
- APU frontend output now preserves stereo channel routing (NR50/NR51 left/right masks) end-to-end for SDL2 and WebAudio.
- APU length-enable edge behavior now includes immediate length clocking on non-length frame-sequencer steps when enabling length mid-playback.
- APU DMG quirk coverage now includes CH1 sweep overflow/negate-clear disable behavior, trigger+length-zero reload/decrement edges, envelope trigger reload offset on envelope-clock steps, documented/common envelope "zombie mode" writes (`NRx2` while active), CH3 wave sample-buffer retrigger semantics plus Wave RAM fetch-window access/retrigger corruption behavior, and CH4 `clock_shift >= 14` no-clock behavior.

### Cartridge / Save Persistence / Metadata
- ROM-only/ROM+RAM plus expanded MBC1/MBC2/MBC3/MBC5 support.
- Cartridge header ROM size decoding across standard size codes, mapper-specific RAM enable/banking behavior (including MBC5 rumble register semantics), and battery-backed persistence (`.sav`, plus `.rtc` for MBC3 timer cartridges) with atomic file replace writes.
- Core cartridge APIs expose import/export of battery save RAM bytes and MBC3 RTC persistence bytes for host adapters/runtime integration.
- Cartridge header diagnostics for Nintendo logo/header checksum/global checksum, exposed as non-blocking warnings in cartridge metadata.
- Cartridge metadata debug report consumed by CLI (`--cart-info`) and frontends (SDL2 `F1` cart-info panel, web debug panel).

### Audio Output Pipeline / Frontend Audio Integration
- Shared `runtime/` host utilities for frontend frame pacing, realtime audio queueing, adaptive buffering, t-cycle-to-PCM mixer bridging (SDL2/Web), and file-backed cartridge persistence adapters.
- Shared runtime audio mixer bridge from emulated APU t-cycle samples to frontend PCM rates (SDL2/Web).
- Realtime audio block API for backend callbacks (SDL queue/WebAudio) with silence padding when emulated audio budget is short.
- SDL2 frontend adaptive audio queue targeting with underrun estimation from queue depth and host time.
- Browser demo (`web/`) with AudioWorklet-based WebAudio hook using realtime mixer blocks.
- Minimal browser demo audio telemetry plus adaptive queue targeting for underrun recovery and latency tuning.

### Validation / CI
- Blargg + Gekkio ROM test integration in local scripts and CI.
- CPU unit regressions include explicit interrupt-control corner coverage (IME/EI/DI/RETI ordering, `EI->HALT` halt-bug sequencing, pending-interrupt preemption of `HALT`/`STOP`, current DMG-scope `STOP` characterization, and interrupt-dispatch stack-push side effects when `IE`/`IF` are overwritten mid-dispatch) to complement Blargg/Gekkio suites.

### Project Architecture / Workspace Layout
- The repository root is now a virtual Cargo workspace (`default-members = ["systems/gb"]`) and no longer owns a Rust package directly.
- Game Boy core/system package now lives in `systems/gb` (Cargo package name remains `gb-emu`).
- Shared frontend/host runtime helpers now live in the `runtime` workspace package (Cargo package name `gb-runtime`).
- Headless CLI frontend is extracted to `frontends/cli` (workspace package/path dependency on `systems/gb`) while preserving the CLI binary name `gb-emu`.
- SDL2 desktop frontend is extracted to `frontends/sdl2` (workspace package/path dependency on `systems/gb`).
- Rust/WASM frontend adapter is extracted to `frontends/wasm` (workspace package/path dependency on `systems/gb`), while `web/` remains the browser host assets/demo area.
- Boundary rule: keep hardware semantics in `systems/gb`, host/runtime helpers in `runtime`, and host platform bindings/UI code in `frontends/*` / `web`.

## Project Structure

```text
systems/
  gb/
    Cargo.toml
    src/
      audio.rs
      cartridge/
      cpu/
      gameboy.rs
      hardware.rs
      memory/
      timing.rs
      lib.rs
    tests/
      integration_smoke.rs
runtime/
  Cargo.toml
  src/
    audio.rs
    timing.rs
    lib.rs
  tests/
    integration_smoke.rs
frontends/
  cli/
    Cargo.toml
    src/
      main.rs
    tests/
      cli_cart_info.rs
  sdl2/
    Cargo.toml
    src/
      main.rs
  wasm/
    Cargo.toml
    src/
      lib.rs
scripts/
  dev/
    bootstrap.sh
    create_pr.sh
    run_audio_guard.sh
    run_sdl2_frontend.sh
    run_web_demo.sh
    setup-hooks.sh
  blargg/
    fetch_blargg_roms.sh
    run_blargg.sh
    rom.txt
  gekkio/
    fetch_gekkio_roms.sh
    run_gekkio.sh
    rom.txt
    roms_boot_models.txt
web/
  minimal/
    audio-worklet.js
    index.html
    main.js
```

## Workspace Layout Guide

- `systems/*`: hardware emulation packages and public system API surfaces.
  Current `systems/gb` owns CPU/APU/PPU/timer/interrupts/MMIO/DMA, cartridge/mappers and persistence-byte semantics (battery save RAM / MBC3 RTC import-export APIs), framebuffer generation, and emulated audio sample stream generation.
  It must not contain SDL2 backends, `wasm-bindgen` exports, browser DOM/JS integration, or libretro bindings.
- `runtime/`: frontend-shared host/runtime helpers that are not hardware semantics.
  Current `runtime` owns host-time frame pacing (`FramePacer`), frontend audio queue/adaptive buffering helpers, the frontend-facing t-cycle-to-PCM mixer bridge, and file-backed cartridge persistence adapters (`.sav` / `.rtc`).
- `frontends/*`: host adapters/UI entrypoints that depend on `systems/gb` and optionally `runtime`.
  - `frontends/cli`: CLI argument parsing, headless modes (`blargg`, `mooneye`, `cart-info`), CLI error formatting/wiring.
  - `frontends/sdl2`: SDL2 window/rendering, event loop, keyboard mapping, SDL2 audio queue/device integration.
  - `frontends/wasm`: `wasm-bindgen` exports, `WebEmulator` browser adapter API, WASM-only glue code.
- `web/`: browser host assets and demo pages (JavaScript/HTML/CSS/AudioWorklet/browser helper tests); no Rust package should live inside `web/`.

Future expansion rule:
- `CGB` support should remain inside `systems/gb` unless a stronger separation is proven necessary.
- A future Game Boy Advance implementation should be introduced as `systems/gba`.

## Workspace Command Examples

```bash
# Default workspace members (currently systems/gb core)
cargo build --locked
cargo test --locked

# All workspace packages (may require optional host dependencies like SDL2 dev libs)
cargo build --locked --workspace

# Package-targeted checks
cargo test --locked -p gb-runtime
cargo test --locked -p frontend-cli
cargo build --locked -p frontend-sdl2 --bin frontend-sdl2
cargo build --locked -p frontend-wasm --lib
```

Notes:
- `cargo lint` intentionally skips `frontend-sdl2` in the default alias to avoid requiring SDL2 system libraries in every CI/local environment.
- Prefer package-targeted commands for frontend work when optional host dependencies are not installed globally.

## Run

```bash
cargo run -p frontend-cli --bin gb-emu -- <path_to_rom.gb>
```

Useful flags:

```bash
cargo run -p frontend-cli --bin gb-emu -- --trace <path_to_rom.gb>
cargo run -p frontend-cli --bin gb-emu -- --blargg --max-steps 120000000 <path_to_rom.gb>
cargo run -p frontend-cli --bin gb-emu -- --mooneye --model dmg0 <path_to_rom.gb>
cargo run -p frontend-cli --bin gb-emu -- --cart-info <path_to_rom.gb>
```

Supported models for `--model`:
- `dmg0`
- `dmg` (default)
- `mgb`
- `sgb`
- `sgb2`

## Current Limitations

### Cartridge / Mapper / Persistence Limits
- Supported cartridge types:
  - ROM-only / ROM+RAM / ROM+RAM+BATTERY (0x00/0x08/0x09).
  - MBC1 family (0x01/0x02/0x03) with RAM enable and RAM banking mode support.
  - MBC2 family (0x05/0x06) with 512x4-bit internal RAM behavior.
  - MBC3 family (0x0F/0x10/0x11/0x12/0x13) with ROM/RAM banking and RTC register/latch support.
  - MBC5 family including rumble variants (0x19..0x1E), with ROM/RAM banking support and rumble control-bit tracking.
- Supported ROM size codes: 0x00..0x08 and 0x52/0x53/0x54 (validated against exact file length).
- Supported RAM size codes: 0x00..0x05 for supported cartridge families.
- For compatibility with legacy test ROM conventions, RAM-capable cartridge types declaring RAM size code `0x00` get a transient 8KB external RAM window.
- ROM-only and ROM+RAM cartridge family (no MBC) is limited to 32KB ROM by hardware design.
- Unsupported mappers (for example MBC6/MBC7/HuC variants, camera/tama) still fail fast when loading the ROM.
- MBC3 RTC persistence currently uses a sidecar `.rtc` file; this is emulator-specific metadata and not a hardware cartridge dump format.
- RTC clock source currently remains a core-local convenience (`SystemRtcClock`); public host-controlled RTC time-source injection is deferred unless determinism/libretro requirements make it necessary.

### Cartridge Header Diagnostics
- Header logo/checksum mismatches are reported as metadata warnings but do not block ROM loading.
- `CartridgeMetadata::debug_report()` remains a core convenience formatter for host/frontend debug UIs; moving presentation formatting out of `systems/gb` is deferred unless the cartridge metadata API grows enough to justify a stricter boundary.

### CPU / Core Fidelity
- CPU correctness and timing confidence are currently driven by the included Blargg + Gekkio suites and project integration tests; untested instruction/interrupt corner cases may still remain.
- The emulator is currently DMG-family focused (`dmg0`, `dmg`, `mgb`, `sgb`, `sgb2`); CGB-specific CPU/platform behavior (for example double-speed mode and CGB-only hardware interactions) is out of scope.
- Cross-subsystem cycle accuracy (CPU vs PPU/APU/DMA/bus contention) is implemented incrementally and is only guaranteed for the timing cases explicitly covered by current tests and documented PPU/DMA behavior.
- `GameBoy`/`Bus` currently expose a small set of persistence-byte bridge helpers for `gb_runtime::cartridge_persistence`; tightening or reshaping that host-facing boundary is deferred unless the core API surface grows significantly.

### Runtime / Host Utility Maintainability
- `runtime/src/audio.rs` is intentionally kept as a single module for now, but if runtime audio helpers continue to grow it should be split into `runtime/src/audio.rs` + `runtime/src/audio/*` submodules (for example `mixer`, `adaptive_queue`, `resampler`) as a maintenance refactor without behavioral changes.
- `web/audio-adaptive.mjs` and `gb_runtime` adaptive queue policy are intentionally separate today (browser demo tuning vs shared runtime helper tuning); if they continue to evolve, align tuning rules/tests or consolidate shared policy logic to avoid silent drift.

### PPU / Rendering / Timing Fidelity
- Framebuffer is DMG grayscale and currently focused on correctness over rendering performance optimizations.
- Dot-stepped OBJ fetch contention now extends Mode 3 at runtime and takeover boundaries include FIFO-stall arbitration; some DMG fetcher bus-phase details (for example full hardware sleep/push micro-ops) are still approximated.
- Recent Mode 3 BG `Push` fetcher work refines the internal state-machine (explicit latched `RecoverySleep` substate) and improves regression observability for the `stall/recovery` path, but it does not yet introduce additional hardware-visible micro-ops outside that `stall -> recovery sleep -> push-ready` flow.
- Remaining high-impact PPU fidelity work is concentrated in timing-sensitive Mode 3 corner cases (finer fetcher micro-ops / bus-phase modeling and additional DMA/STAT contention edge cases beyond the currently covered regressions).

### APU / Audio Fidelity
- Built-in analog calibration profiles are model-level references; full per-device fidelity requires supplying measured calibration values via `GameBoy::set_audio_analog_calibration(...)`.
- Envelope "zombie mode" (`NRx2` writes while a channel is active) is implemented using documented/common behavior; full unit/model-specific DMG variants are still not exhaustively modeled.
- CH3 Wave RAM active-access timing is modeled with a t-cycle fetch window approximation (sufficient for common DMG edge-cases), not a fully cycle-accurate bus arbitration model.
- VIN / external audio input routing (`NR50` VIN bits) is not currently modeled.
- Audio resampling remains interpolation-based (selectable `linear` or `cubic`, with linear fallback at cubic edges), not a band-limited/FIR resampler.

## Mapper Coverage Examples

- No MBC (ROM-only): `Tetris`, `Dr. Mario`.
- MBC1: `Super Mario Land`, `Kirby's Dream Land`.
- MBC2: `Pokemon Red/Blue`.
- MBC3: `Pokemon Gold/Silver`.
- MBC5: `Pokemon Pinball`.

## Local Requirements

- Rust stable toolchain (see `rust-toolchain.toml`).
- `git`, `curl`, `unzip`, `perl`, and `rg` (ripgrep) for ROM fetch/run scripts.
- Optional for SDL2 frontend: SDL2 runtime/dev libraries available in the OS.
- Optional for web frontend: `wasm-pack` (or equivalent wasm build tooling).
- Optional for web frontend unit tests: Node.js (for `node --test`).
- Optional for `scripts/dev/run_web_demo.sh`: `python3`.

Bootstrap helper:

```bash
# Check core + SDL2 + web dependencies.
scripts/dev/bootstrap.sh

# Same check, but install wasm-pack automatically when missing.
scripts/dev/bootstrap.sh --install-wasm-pack

# Skip one frontend when not needed on your machine.
scripts/dev/bootstrap.sh --skip-sdl2
scripts/dev/bootstrap.sh --skip-web
```

Common SDL2 install hints:
- macOS (Homebrew): `brew install sdl2`
- Debian/Ubuntu: `sudo apt-get update && sudo apt-get install -y libsdl2-dev`
- Fedora: `sudo dnf install -y SDL2-devel`
- Arch Linux: `sudo pacman -S --needed sdl2`

## Quality and Tests

Formatting/lint aliases are defined in `.cargo/config.toml`.

```bash
cargo fmt-check
cargo lint
cargo test --locked
cargo test --locked -p gb-runtime
```

`cartridge` tests include a mapper conformance matrix for all currently supported cartridge type codes (`0x0147`) plus integration smoke coverage through `GameBoy`.

Optional web frontend unit test:

```bash
node --test web/audio-adaptive.test.mjs
```

ROM test suites:

```bash
# Blargg
scripts/blargg/fetch_blargg_roms.sh
# Targeted performance guard used by CI to catch regressions earlier:
scripts/blargg/run_cpu_instrs_guard.sh
# Targeted audio/realtime mixer guard (local/dev, timeout-based):
scripts/dev/run_audio_guard.sh
# Runs all configured DMG Blargg ROMs:
scripts/blargg/run_blargg.sh

# Gekkio (Mooneye)
scripts/gekkio/fetch_gekkio_roms.sh
# Default is GEKKIO_SUITE=all:
scripts/gekkio/run_gekkio.sh
# Default stable Gekkio suite (core + acceptance/ppu):
GEKKIO_SUITE=all scripts/gekkio/run_gekkio.sh
# Boot matrix by hardware model (dmg0/dmg/mgb/sgb/sgb2):
GEKKIO_SUITE=boot_models scripts/gekkio/run_gekkio.sh
# Run a suite against a specific hardware model:
GB_MODEL=sgb GEKKIO_SUITE=all scripts/gekkio/run_gekkio.sh
GB_MODEL=mgb scripts/blargg/run_blargg.sh
```

Useful environment overrides for scripts:
- `GB_MODEL` (default: `dmg`) for both `run_blargg.sh` and `run_gekkio.sh`.
- `GEKKIO_SUITE` (`all`, `core`, `boot_models`) for `run_gekkio.sh`.
- `ROM_ROOT` to point to a custom ROM directory.
- `MAX_STEPS` and `TIMEOUT_SECS` to tune execution limits.
- `GEKKIO_VERSION` to fetch a specific Mooneye bundle version.
- `TEST_NAME` to override the integration test executed by `scripts/dev/run_audio_guard.sh`.

## Frontend Bootstrap (SDL2 + Web)

SDL2 desktop frontend (macOS / Windows / Linux):

```bash
cargo run -p frontend-sdl2 --bin frontend-sdl2 -- <path_to_rom.gb> [dmg0|dmg|mgb|sgb|sgb2]
```

SDL2 build/run helper (locks deps, prepares Homebrew SDL2 env on macOS, and clean-rebuilds by default):

```bash
# Build only (clean + locked SDL2 build)
scripts/dev/run_sdl2_frontend.sh --no-run

# Build and run in release mode (recommended for performance)
scripts/dev/run_sdl2_frontend.sh --release --no-clean -- <path_to_rom.gb>

# Build and run
scripts/dev/run_sdl2_frontend.sh -- <path_to_rom.gb> [dmg0|dmg|mgb|sgb|sgb2]

# Faster iteration without clean
scripts/dev/run_sdl2_frontend.sh --no-clean -- <path_to_rom.gb>
```

Web frontend bindings (wasm):

```bash
wasm-pack build frontends/wasm --target web --out-name gb_emu
```

Minimal browser demo (AudioWorklet + keyboard + ROM file loader):

```bash
scripts/dev/run_web_demo.sh
# Open http://localhost:8080/web/
```

Notes:
- The core remains frontend-agnostic and can be embedded by multiple frontends.
- MBC5 rumble status is exposed from core (`GameBoy::cartridge_has_rumble()`, `GameBoy::rumble_active()`), but no host haptics backend is wired yet.
- Cartridge metadata is exposed from core via `Cartridge::metadata()` and `GameBoy::cartridge_metadata()` (type code, mapper, ROM/RAM size codes, bank counts, declared/effective RAM, battery/timer/rumble flags, and header diagnostics warnings).
- Current web entrypoint is `WebEmulator` in `frontends/wasm/src/lib.rs`.
- `web/` contains browser host assets only; the Rust/WASM adapter crate lives in `frontends/wasm/`.
- Web builds use the core RTC wall-clock source for MBC3 RTC state.
- SDL2 key mapping: arrows=`D-Pad`, `Z`=`A`, `X`=`B`, `Backspace`=`Select`, `Enter`=`Start`.
- SDL2 debug panel: press `F1` to open a cartridge metadata/warnings popup.
- SDL2/Web pacing uses `gb_runtime::timing::FramePacer` (shared host/runtime pacing helper) to avoid frontend-specific timing drift.
- SDL2 audio uses the core mixer clock bridge and queues stereo interleaved PCM in real time (now from the `frontends/sdl2` workspace package).
- SDL2 queue refill is driven by emulated audio t-cycles; underruns are padded with silence (no synthetic emulated cycles).
- SDL2 queue target is auto-tuned over time windows (same policy as web) using estimated underruns from elapsed playback vs queued samples.
- `scripts/dev/run_sdl2_frontend.sh` is the recommended local command for SDL2 builds/runs (including a `--release` mode for performance) and consistent macOS/Homebrew linker env setup.
- SDL2 renderer uses accelerated rendering with `present_vsync()` by default to reduce visible tearing during scroll/camera movement; override with `GB_SDL2_VSYNC=0` for diagnostics/perf comparisons.
- Optional SDL2 debug tone: set `GB_AUDIO_TEST_TONE=1`.
- Optional SDL2 core APU resampler quality override: set `GB_AUDIO_RESAMPLER=linear` or `GB_AUDIO_RESAMPLER=cubic` (default).
- Optional SDL2 VSync override: set `GB_SDL2_VSYNC=1` (default) or `GB_SDL2_VSYNC=0`.
- Battery-backed cartridges loaded via `gb_runtime::cartridge_persistence` persist external RAM to a sibling `.sav` file; MBC3 timer carts also persist RTC metadata to `.rtc`. Save writes use atomic temp-file+rename replacement. Current CLI/SDL2 frontends flush saves on graceful exit through the shared runtime file adapter.
- Core helper: `GameBoy::set_audio_analog_calibration(profile)` to apply measured per-device analog calibration profiles from host/frontends.
- Web helpers:
  - `run_for_elapsed_micros(elapsed_micros)` to step as many emulated frames as host time allows.
  - `audio_clock_tcycles()` / `drain_audio_tcycles()` for raw emulated audio clock access.
  - `set_audio_sample_rate(rate_hz)` (preserves queued Core APU audio when reconfiguring WebAudio rate) and `drain_audio_samples(max_samples)` for WebAudio feeding (`Vec<f32>` stereo interleaved: `L,R,L,R,...`).
  - `set_audio_resampler_quality("linear" | "cubic")` and `audio_resampler_quality()` to compare interpolation quality/CPU tradeoffs from frontends.
  - `drain_audio_samples_realtime(block_samples)` for callback-style fixed-size WebAudio blocks (`block_samples` = frames, returned buffer is stereo interleaved).
  - `set_audio_test_tone_enabled(enabled)` for pipeline/debug validation.
  - `cartridge_debug_report()` and `cartridge_warning_count()` for frontend cartridge diagnostics panels.
- `web/` is intentionally small and uses `AudioWorkletNode` for lower-latency callback-style audio.
- `web/` surfaces cartridge metadata/warnings plus audio telemetry (`queued ms`, cumulative underrun samples/ms, and current resampler mode) and auto-adjusts the refill target queue based on recent underrun windows.

## CI

Workflows:
- `.github/workflows/quality.yml`: format, lint, build, unit/integration tests.
- `.github/workflows/rom-tests.yml`: two independent jobs/checks for branch protection:
  - `rom-blargg` (includes a realtime audio micro-guard and the `cpu_instrs` micro-guard before the full Blargg suite)
  - `rom-gekkio` (runs `all` + `boot_models`)

## Test ROMs and Licensing Notes

`/roms` is intentionally ignored in `.gitignore` to avoid storing test ROM binaries in this repository.

CI and local setup use both:
- `scripts/blargg/fetch_blargg_roms.sh`
- `scripts/gekkio/fetch_gekkio_roms.sh`

Both pull public test ROM sources at runtime.

Why:
- Keeps the repository lightweight.
- Avoids redistributing binaries with mixed or unclear licensing terms.

`scripts/gekkio/run_gekkio.sh` supports two profiles:
- `GEKKIO_SUITE=all` (default): stable acceptance set defined in `scripts/gekkio/rom.txt`.
- `GEKKIO_SUITE=boot_models`: dedicated boot-state matrix per model defined in `scripts/gekkio/roms_boot_models.txt`.

`scripts/blargg/run_blargg.sh` runs the full DMG Blargg set defined in:
- `scripts/blargg/rom.txt`

`scripts/blargg/fetch_blargg_roms.sh` mirrors upstream `retrio/gb-test-roms` layout for the selected ROMs and writes a local listing to:
- `roms/blargg's_test_roms/.blargg_listing.txt`

Current Blargg DMG selection intentionally excludes:
- `cgb_sound/*`
- `interrupt_time/*`

CI currently runs `GEKKIO_SUITE=all` and `GEKKIO_SUITE=boot_models` as required ROM checks.

When adding new ROM suites, document:
- Source repository URL.
- Upstream license/status.
- Which script/workflow consumes them.

## Development Notes

- Keep code identifiers/comments in English.
- Prefer small, safe refactors with tests.
- Add/adjust tests whenever behavior changes (unit + integration as needed).
- Optional local hook setup: `scripts/dev/setup-hooks.sh` (pre-commit runs `cargo fmt-check` and `cargo lint`).
- PR helper:
  - From a feature branch, run `scripts/dev/create_pr.sh` (or `scripts/dev/create_pr.sh main`).
  - PR title is set to the latest commit subject.
  - PR body is taken from the latest commit body, or falls back to `.github/pull_request_template.md`.
