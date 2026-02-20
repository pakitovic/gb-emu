# gb-emu

Personal/hobby Game Boy emulator project written in Rust, focused on learning and incremental milestones.

Current scope:
- ROM-only plus basic MBC1/MBC5 support.
- CPU core with growing opcode coverage.
- Memory bus + timer/interrupt basics.
- Blargg + Gekkio ROM test integration in local scripts and CI.
- Core API bootstrap for portable frontends (frame stepping + framebuffer access).
- DMG background layer rendering to a grayscale framebuffer.
- DMG window + sprite (OBJ) composition with priority/palette/flip handling.
- Mode 3 background/window pixel FIFO stepping per dot (timing-sensitive SCX/window effects are now line-progressive).
- Mode 3 OBJ fetch stalls and sprite pixel mixing are stepped per dot with DMG priority/palette rules (mid-line register writes affect remaining pixels).
- Joypad input API in core with P1 register behavior and joypad interrupt edges.
- Portable real-time pacing clock (shared by SDL2/Web) with audio-tcycle clock accumulation.
- Core audio mixer clock bridge from emulated t-cycles to PCM samples.
- Realtime audio block API for backend callbacks (SDL queue/WebAudio) with silence padding when emulated audio budget is short.
- Minimal browser demo (`web/minimal`) with AudioWorklet-based WebAudio hook using realtime mixer blocks.

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
  integration_smoke.rs
scripts/
  dev/
    bootstrap.sh
    create_pr.sh
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
```

Supported models for `--model`:
- `dmg0`
- `dmg` (default)
- `mgb`
- `sgb`
- `sgb2`

## Current Limitations

- Supported cartridge types: ROM-only (0x00), MBC1 (0x01/0x02/0x03), MBC5 (0x19/0x1A/0x1B).
- Supported ROM size codes: 32KB (0x00) and 64KB (0x01).
- ROM-only cartridges must be 32KB.
- Unsupported cartridge/ROM size combinations fail fast when loading the ROM.
- Framebuffer is DMG grayscale and currently focused on correctness over rendering performance optimizations.
- Dot-stepped OBJ fetch stalls are modeled, but full cycle-exact BG/OBJ fetch contention is still approximated.
- APU emulation is not implemented yet; current audio path is timing-synchronized PCM generation (silence by default, optional test tone).

## Local Requirements

- Rust stable toolchain (see `rust-toolchain.toml`).
- `git`, `curl`, `unzip`, `perl`, and `rg` (ripgrep) for ROM fetch/run scripts.
- Optional for SDL2 frontend: SDL2 runtime/dev libraries available in the OS.
- Optional for web frontend: `wasm-pack` (or equivalent wasm build tooling).

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

Web frontend bindings (wasm):

```bash
wasm-pack build --target web --features frontend-web
```

Minimal browser demo (AudioWorklet + keyboard + ROM file loader):

```bash
wasm-pack build --target web --features frontend-web --out-dir web/minimal/pkg
python3 -m http.server 8080
# Open http://localhost:8080/web/minimal/
```

Notes:
- The core remains frontend-agnostic and can be embedded by multiple frontends.
- Current web entrypoint is `WebEmulator` in `src/web.rs`.
- SDL2 key mapping: arrows=`D-Pad`, `Z`=`A`, `X`=`B`, `Backspace`=`Select`, `Enter`=`Start`.
- SDL2/Web pacing uses `timing::FramePacer` from the core to avoid frontend-specific timing drift.
- SDL2 audio uses the core mixer clock bridge and queues PCM in real time.
- SDL2 queue refill is driven by emulated audio t-cycles; underruns are padded with silence (no synthetic emulated cycles).
- Optional SDL2 debug tone: set `GB_AUDIO_TEST_TONE=1`.
- Web helpers:
  - `run_for_elapsed_micros(elapsed_micros)` to step as many emulated frames as host time allows.
  - `audio_clock_tcycles()` / `drain_audio_tcycles()` for raw emulated audio clock access.
  - `set_audio_sample_rate(rate_hz)` and `drain_audio_samples(max_samples)` for WebAudio feeding.
  - `drain_audio_samples_realtime(block_samples)` for callback-style fixed-size WebAudio blocks.
  - `set_audio_test_tone_enabled(enabled)` for pipeline/debug validation.
- `web/minimal` is intentionally small and uses `AudioWorkletNode` for lower-latency callback-style audio.

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
