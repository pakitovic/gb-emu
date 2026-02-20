# gb-emu

Personal/hobby Game Boy emulator project written in Rust, focused on learning and incremental milestones.

Current scope:
- ROM-only/ROM+RAM plus expanded MBC1/MBC2/MBC3/MBC5 support.
- CPU core with growing opcode coverage.
- Memory bus + timer/interrupt basics.
- Blargg + Gekkio ROM test integration in local scripts and CI.
- Core API bootstrap for portable frontends (frame stepping + framebuffer access).
- DMG background layer rendering to a grayscale framebuffer.
- DMG window + sprite (OBJ) composition with priority/palette/flip handling.
- Mode 3 background/window pixel FIFO stepping per dot (timing-sensitive SCX/window effects are now line-progressive).
- Mode 3 OBJ fetch stalls and sprite pixel mixing are stepped per dot with DMG priority/palette rules (mid-line register writes affect remaining pixels).
- Mode 3 line duration now grows from runtime OBJ fetch contention (including mid-line OBJ enable/disable effects), reducing reliance on static per-line penalty estimates.
- Additional PPU/DMA timing edge cases: mode0 STAT source enabled during mode3 triggers on HBlank entry, and DMA restart keeps prior transfer running through the full restart-delay window.
- APU core channel state-machine scaffolding: NR52 power control, NR50/NR51 mixer register gating, CH1/CH2/CH3/CH4 trigger/state progression, and DIV-driven frame sequencer stepping (length/sweep/envelope clocks).
- Joypad input API in core with P1 register behavior and joypad interrupt edges.
- Portable real-time pacing clock (shared by SDL2/Web) with audio-tcycle clock accumulation.
- Core audio mixer clock bridge from emulated t-cycles to PCM samples.
- Realtime audio block API for backend callbacks (SDL queue/WebAudio) with silence padding when emulated audio budget is short.
- Cartridge header ROM size decoding across standard size codes, mapper-specific RAM enable/banking behavior (including MBC5 rumble register semantics), and battery-backed persistence (`.sav`, plus `.rtc` for MBC3 timer cartridges) with atomic file replace writes.
- Cartridge header diagnostics for Nintendo logo/header checksum/global checksum, exposed as non-blocking warnings in cartridge metadata.
- Cartridge metadata debug report consumed by CLI (`--cart-info`) and frontends (SDL2 `F1` cart-info panel, web debug panel).
- SDL2 frontend adaptive audio queue targeting with underrun estimation from queue depth and host time.
- Minimal browser demo (`web/minimal`) with AudioWorklet-based WebAudio hook using realtime mixer blocks.
- Minimal browser demo audio telemetry plus adaptive queue targeting for underrun recovery and latency tuning.

## Project Structure

```text
src/
  audio.rs
  bin/
    frontend_sdl2.rs
  cartridge/
  cpu/
  gameboy.rs
  hardware.rs
  memory/
  timing.rs
  web.rs
  lib.rs
  main.rs
tests/
  cli_cart_info.rs
  integration_smoke.rs
scripts/
  dev/
    bootstrap.sh
    create_pr.sh
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

## Run

```bash
cargo run -- <path_to_rom.gb>
```

Useful flags:

```bash
cargo run -- --trace <path_to_rom.gb>
cargo run -- --blargg --max-steps 120000000 <path_to_rom.gb>
cargo run -- --mooneye --model dmg0 <path_to_rom.gb>
cargo run -- --cart-info <path_to_rom.gb>
```

Supported models for `--model`:
- `dmg0`
- `dmg` (default)
- `mgb`
- `sgb`
- `sgb2`

## Current Limitations

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
- Header logo/checksum mismatches are reported as metadata warnings but do not block ROM loading.
- Framebuffer is DMG grayscale and currently focused on correctness over rendering performance optimizations.
- Dot-stepped OBJ fetch contention now extends Mode 3 at runtime, but full cycle-exact BG/OBJ fetch arbitration is still approximated.
- APU channel synthesis (CH1/CH2/CH3/CH4) now advances in core state/timing, but SDL2/Web PCM output is still fed by the existing frontend mixer path (silence by default, optional test tone), so synthesized APU stream is not yet audible in frontends.

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
```

`cartridge` tests include a mapper conformance matrix for all currently supported cartridge type codes (`0x0147`) plus integration smoke coverage through `GameBoy`.

Optional web frontend unit test:

```bash
node --test web/minimal/audio-adaptive.test.mjs
```

ROM test suites:

```bash
# Blargg
scripts/blargg/fetch_blargg_roms.sh
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

## Frontend Bootstrap (SDL2 + Web)

SDL2 desktop frontend (macOS / Windows / Linux):

```bash
cargo run --features frontend-sdl2 --bin frontend-sdl2 -- <path_to_rom.gb> [dmg0|dmg|mgb|sgb|sgb2]
```

SDL2 build/run helper (locks deps, prepares Homebrew SDL2 env on macOS, and clean-rebuilds by default):

```bash
# Build only (clean + locked SDL2 build)
scripts/dev/run_sdl2_frontend.sh --no-run

# Build and run
scripts/dev/run_sdl2_frontend.sh -- <path_to_rom.gb> [dmg0|dmg|mgb|sgb|sgb2]

# Faster iteration without clean
scripts/dev/run_sdl2_frontend.sh --no-clean -- <path_to_rom.gb>
```

Web frontend bindings (wasm):

```bash
wasm-pack build --target web --features frontend-web
```

Minimal browser demo (AudioWorklet + keyboard + ROM file loader):

```bash
scripts/dev/run_web_demo.sh
# Open http://localhost:8080/web/minimal/
```

Notes:
- The core remains frontend-agnostic and can be embedded by multiple frontends.
- MBC5 rumble status is exposed from core (`GameBoy::cartridge_has_rumble()`, `GameBoy::rumble_active()`), but no host haptics backend is wired yet.
- Cartridge metadata is exposed from core via `Cartridge::metadata()` and `GameBoy::cartridge_metadata()` (type code, mapper, ROM/RAM size codes, bank counts, declared/effective RAM, battery/timer/rumble flags, and header diagnostics warnings).
- Current web entrypoint is `WebEmulator` in `src/web.rs`.
- SDL2 key mapping: arrows=`D-Pad`, `Z`=`A`, `X`=`B`, `Backspace`=`Select`, `Enter`=`Start`.
- SDL2 debug panel: press `F1` to open a cartridge metadata/warnings popup.
- SDL2/Web pacing uses `timing::FramePacer` from the core to avoid frontend-specific timing drift.
- SDL2 audio uses the core mixer clock bridge and queues PCM in real time.
- SDL2 queue refill is driven by emulated audio t-cycles; underruns are padded with silence (no synthetic emulated cycles).
- SDL2 queue target is auto-tuned over time windows (same policy as web) using estimated underruns from elapsed playback vs queued samples.
- `scripts/dev/run_sdl2_frontend.sh` is the recommended local command for clean SDL2 rebuilds and consistent macOS/Homebrew linker env setup.
- Optional SDL2 debug tone: set `GB_AUDIO_TEST_TONE=1`.
- Battery-backed cartridges loaded via `Cartridge::from_file(...)` persist external RAM to a sibling `.sav` file; MBC3 timer carts also persist RTC metadata to `.rtc`. Save writes use atomic temp-file+rename replacement. Current CLI/SDL2 frontends flush saves on graceful exit.
- Web helpers:
  - `run_for_elapsed_micros(elapsed_micros)` to step as many emulated frames as host time allows.
  - `audio_clock_tcycles()` / `drain_audio_tcycles()` for raw emulated audio clock access.
  - `set_audio_sample_rate(rate_hz)` and `drain_audio_samples(max_samples)` for WebAudio feeding.
  - `drain_audio_samples_realtime(block_samples)` for callback-style fixed-size WebAudio blocks.
  - `set_audio_test_tone_enabled(enabled)` for pipeline/debug validation.
  - `cartridge_debug_report()` and `cartridge_warning_count()` for frontend cartridge diagnostics panels.
- `web/minimal` is intentionally small and uses `AudioWorkletNode` for lower-latency callback-style audio.
- `web/minimal` surfaces cartridge metadata/warnings plus audio telemetry (`queued ms` and cumulative underrun samples/ms) and auto-adjusts the refill target queue based on recent underrun windows.

## CI

Workflows:
- `.github/workflows/quality.yml`: format, lint, build, unit/integration tests.
- `.github/workflows/rom-tests.yml`: two independent jobs/checks for branch protection:
  - `rom-blargg`
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
