# gb-emu

Personal/hobby Game Boy emulator project written in Rust, focused on learning and incremental milestones.

## Current Scope

### Core Emulation (CPU / Bus / Timing / Input)
- CPU core with growing opcode coverage.
- Memory bus + timer/interrupt basics.
- Core timing contract now exposes an explicit DMG clock-ratio policy layer (CPU m-cycles -> base t-cycles) used at the CPU/bus boundary, keeping current DMG behavior unchanged while reducing future CGB double-speed refactor scope.
- CPU timing plumbing now derives control/jump/ALU/load/CB instruction return timings from the clock-ratio policy (`mcycles -> tcycles`), and non-instruction CPU timing paths (HALT idle step + interrupt service dispatch) also use explicit policy-derived timing returns, while keeping behavior unchanged in current DMG scope.
- Bus/memory access now routes VRAM/WRAM/OAM through internal segment helpers (CPU-visible vs hardware-internal access modes) with centralized VRAM/OAM blocking rules, reducing future CGB banking (`VBK`/`SVBK`) refactor scope.
- MMIO decode now includes DMG-noop scaffolding for CGB-only registers (`KEY1`, `VBK`, `SVBK`, `BGPI/BGPD`, `OBPI/OBPD`) with internal shadowed fields/state (including palette index/data shadow capture), and VRAM/WRAM bus helpers now resolve through DMG-fixed bank-selection scaffolding hooks backed by real multi-bank storage (CGB-sized VRAM/WRAM backing kept DMG-fixed by effective bank policy) so future CGB banking wiring can stay localized to the bus/MMIO layer.
- FFxx IO routing now uses a centralized declarative register-route decoder (including explicit reserved/unmapped DMG-family ranges and CGB scaffold register precedence) so adding future CGB MMIO cases stays localized to the IO router table instead of spreading FFxx side-effects across the bus.
- DMA is now modeled as a scheduler-style state machine with incremental `tick(tcycles)` advancement, formal mode/edge state, centralized DMA CPU access-block policy hooks (currently only DMG OAM DMA behavior is active), and DMG-noop scaffolding for future CGB DMA control registers (`HDMA1..HDMA5`) plus model-gated `GDMA/HDMA` scheduler paths (transfer-state/request wiring and HBlank-edge hook integration, inactive for current DMG-family models) to reduce future HDMA/GDMA integration refactor scope.
- Joypad input API in core with P1 register behavior and joypad interrupt edges.
- Core API bootstrap for portable frontends (frame stepping + framebuffer access).
- Optional boot ROM startup path in core (`PC=0x0000` + `0x0000..=0x00FF` boot-ROM mapping + `FF50` disable handling) with automatic fallback to existing post-boot defaults when no boot ROM is provided.
- Public API now exposes a `gb_emu::bus` alias module (`Bus`, `LCD_WIDTH`, `LCD_HEIGHT`, `LCD_FRAME_PIXELS`) while keeping existing `gb_emu::memory::*` paths stable.

### PPU / Video (DMG)
- DMG background layer rendering to a grayscale framebuffer.
- Video output now supports palette profiles over canonical DMG shade levels (`dmg` green, `mgb` gray), plus explicit `cgb`/`sgb` scaffold profiles to keep frontend wiring stable before real CGB/SGB color semantics are implemented.
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
- Mode 3 pixel composition now carries intermediate DMG pixel metadata (`source`, `color_id`, priority flags, palette selector) and applies DMG grayscale mapping in a final color step, reducing future CGB BG/OBJ palette/priority integration refactor scope.
- PPU now keeps an explicit formal mode state plus one-tick mode-entry edge events (`entered OAM/Transfer/HBlank/VBlank`) synchronized with STAT mode bits and used internally for mode-sensitive interrupt timing hooks (including VBlank entry), reducing future HBlank-DMA/HDMA integration refactor scope.
- PPU Mode 3 metadata scaffolding now also defines CGB-oriented BG tile attrs and OBJ palette/bank metadata placeholders, with runtime wiring model-gated off for current DMG-family models (test/debug paths can still exercise the scaffold) while current DMG color output behavior remains unchanged.
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
- Web demo host now persists battery-backed cartridge data in browser storage (SRAM + MBC3 RTC metadata) with dirty-flag autosave debounce and page-lifecycle flush hooks.
- Cartridge header diagnostics for Nintendo logo/header checksum/global checksum, exposed as non-blocking warnings in cartridge metadata.
- Cartridge metadata debug report consumed by CLI (`--cart-info`) and frontends (SDL2 `F1` cart-info panel, web debug panel).
- Cartridge core now exposes an internal `capabilities()` interface (mapper/ram/battery/timer/rumble/battery-save, compatibility-RAM mode, and normalized CGB/SGB header flag support) plus a model-aware cartridge compatibility policy (`header flags + selected hardware model`) so future DMG/CGB/SGB mode gating can query cartridge support without threading ad-hoc header checks through the bus.

### Audio Output Pipeline / Frontend Audio Integration
- Shared `runtime/` host utilities for frontend frame pacing, realtime audio queueing, adaptive buffering, t-cycle-to-PCM mixer bridging (SDL2/Web), and file-backed cartridge persistence adapters.
- Shared `gb_runtime::session::RuntimeSession` now centralizes frontend wiring of `GameBoy + FramePacer + AudioMixer` (used by SDL2 and wasm adapters) so core-clock/audio capture plumbing does not need to be reimplemented per frontend.
- Shared `gb_runtime::audio_queue::AudioQueueController` now centralizes adaptive queue-targeting and underrun-estimation policy used by SDL2 and the wasm/web queue refill path.
- Shared runtime audio mixer bridge from emulated APU t-cycle samples to frontend PCM rates (SDL2/Web).
- Realtime audio block API for fixed-size callback backends, with silence padding when emulated audio budget is short.
- Queue-based frontends (SDL2/WebAudio queue feeder) now use the same runtime adaptive queue policy and enqueue only currently available emulated audio samples.
- Runtime queue defaults prioritize stable playback under host jitter (`initial/min/max = 4096/2048/16384`, `refill_block = 512`) while adaptive queueing remains shared across SDL2 and wasm/web frontends.
- SDL2 now uses runtime queue defaults directly (no frontend-specific queue constants), keeps a conservative host audio buffer request (`AudioSpecDesired.samples = 1024`), and the web demo uses an `8ms` queue-refill cadence.
- Browser demo (`web/`) with AudioWorklet-based WebAudio hook using realtime mixer blocks.
- Minimal browser demo audio telemetry plus adaptive queue targeting for underrun recovery and latency tuning.

### Validation / CI
- Blargg + Gekkio ROM test integration in local scripts and CI.
- CI ROM tests run the Blargg suites in `dev` profile and use relaxed per-ROM timeouts for the `blargg-all` suite / `cpu_instrs` guard on shared runners to reduce false-negative CI timeouts from debug-performance variance while keeping dedicated micro-guards in place.
- CPU unit regressions include explicit interrupt-control corner coverage (IME/EI/DI/RETI ordering, `EI->HALT` halt-bug sequencing, `HALT` wake/no-wake behavior when `IF`/`IE` change while halted, halt-bug latch behavior when pending interrupts are cleared/masked or the interrupt source changes before the next step, pending-interrupt preemption of `HALT`/`STOP`, current DMG-scope `STOP` characterization including delayed-service source changes and combined `IF/IE` priority re-evaluation after `EI->STOP`, and interrupt-dispatch stack-push side effects when `IE`/`IF` are overwritten mid-dispatch), plus `GameBoy` integration regressions for CPU-visible MMIO contention (`OAM DMA` OAM block and `PPU Mode 3` VRAM block) and `Bus::tick` t-cycle chunking characterization regressions (DIV/TIMA, LY/STAT, OAM DMA progress) to complement Blargg/Gekkio suites.

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
      bus.rs
      cartridge.rs
      cartridge/
      cpu.rs
      cpu/
      gameboy.rs
      gameboy/
      hardware.rs
      input.rs
      memory.rs
      memory/
      timing.rs
      lib.rs
    tests/
      integration_smoke.rs
runtime/
  Cargo.toml
  src/
    audio.rs
    audio_queue.rs
    bootrom.rs
    session.rs
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
  Current `runtime` owns host-time frame pacing (`FramePacer`), a shared frontend runtime session (`RuntimeSession`: `GameBoy + FramePacer + AudioMixer` wiring), frontend audio queue/adaptive buffering helpers, frontend boot-ROM autoload helpers, the frontend-facing t-cycle-to-PCM mixer bridge, and file-backed cartridge persistence adapters (`.sav` / `.rtc`).
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
cargo run -p frontend-cli --bin gb-emu -- --bootrom-dir roms/bootrom --model mgb <path_to_rom.gb>
cargo run -p frontend-cli --bin gb-emu -- --no-bootrom <path_to_rom.gb>
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
- RTC clock source currently remains a core-local convenience (`SystemRtcClock`) for native hosts, while host/frontends can inject RTC epoch time for portability-sensitive targets (web) and future deterministic/libretro integrations.
- Web demo battery persistence currently uses browser local storage (base64-encoded SRAM/RTC sidecar blobs keyed per ROM file/hash); browser quota/privacy/security settings can disable or evict stored data.
- Cartridge capabilities/compatibility policy now normalize the CGB (`0x0143`) and SGB (`0x0146`) header flags and combine them with the selected hardware model, but the result is currently advisory metadata only: current DMG-family scope does not enforce/reject by CGB header requirements, does not switch to CGB execution, and does not enable SGB-specific features from the header flags.

### Cartridge Header Diagnostics
- Header logo/checksum mismatches are reported as metadata warnings but do not block ROM loading.

### CPU / Core Fidelity
- CPU correctness and timing confidence are currently driven by the included Blargg + Gekkio suites and project integration tests; untested instruction/interrupt corner cases may still remain.
- The emulator is currently DMG-family focused (`dmg0`, `dmg`, `mgb`, `sgb`, `sgb2`); CGB-specific CPU/platform behavior (for example double-speed mode and CGB-only hardware interactions) is out of scope.
- Optional boot ROM startup currently supports DMG-family 256-byte boot ROM windows (`0x0000..=0x00FF`) only; CGB/AGB boot ROM mapping behavior is not implemented in current scope.
- Cross-subsystem cycle accuracy (CPU vs PPU/APU/DMA/bus contention) is implemented incrementally and is only guaranteed for the timing cases explicitly covered by current tests and documented PPU/DMA behavior.
- CPU timing plumbing is mostly policy-derived in current DMG scope, but some CPU timing work remains outside this migration (for example future CGB-specific timing behavior/policies and additional non-instruction edge cases not yet explicitly characterized).
- The bus/memory segment helper layer is currently DMG single-bank only; future CGB VRAM/WRAM bank selection (`VBK`/`SVBK`) and CGB-specific bus access rules are not implemented yet.
- `KEY1`/`VBK`/`SVBK` and CGB palette MMIO (`BGPI/BGPD`, `OBPI/OBPD`) scaffolding remain DMG-noop in current scope: reads behave as unmapped (`0xFF`), writes do not alter emulation behavior, and while internal placeholder shadows/state (including palette index/data shadow capture and auto-increment scaffold behavior) are recorded for future CGB wiring, no CGB-visible palette semantics or color output behavior is enabled.
- FFxx IO routing is now centralized/declarative, but the CGB-oriented portion remains scaffold-level only in current DMG scope: many CGB FFxx registers are intentionally documented as reserved/unmapped placeholders and still decode to DMG no-op behavior until real CGB features are implemented.
- DMA scheduling remains DMG-only in public behavior and implements only OAM DMA transfer rules for current models; `HDMA1..HDMA5` writes are DMG-visible no-ops, and while model-gated `GDMA/HDMA` transfer-state/HBlank-hook scaffolding now exists internally for future CGB wiring, CGB DMA enablement, timing accuracy, and CGB-specific DMA bus restrictions are not implemented yet.
- `GameBoy`/`Bus` currently expose a small set of persistence-byte bridge helpers for `gb_runtime::cartridge_persistence`; tightening or reshaping that host-facing boundary is deferred unless the core API surface grows significantly.

### Runtime / Host Utility Maintainability
- `runtime/src/audio.rs` is intentionally kept as a single module for now, but if runtime audio helpers continue to grow it should be split into `runtime/src/audio.rs` + `runtime/src/audio/*` submodules (for example `mixer`, `adaptive_queue`, `resampler`) as a maintenance refactor without behavioral changes.
- Queue-targeting policy is now runtime-owned via `gb_runtime::audio_queue`; browser host code should treat it as the source of truth to avoid SDL2/web drift.

### PPU / Rendering / Timing Fidelity
- Framebuffer is DMG grayscale and currently focused on correctness over rendering performance optimizations.
- `cgb`/`sgb` palette selections are currently display-side scaffold profiles only; they do not implement hardware CGB palette RAM semantics or SGB command-driven border/palette behavior.
- Dot-stepped OBJ fetch contention now extends Mode 3 at runtime and takeover boundaries include FIFO-stall arbitration; some DMG fetcher bus-phase details (for example full hardware sleep/push micro-ops) are still approximated.
- The Mode 3 pixel pipeline now carries DMG-oriented intermediate metadata plus CGB attr/palette scaffolding fields, but CGB BG tile attributes / OBJ palette metadata are not yet used to change rendering rules or final color output (current behavior remains DMG-only, and runtime scaffold wiring is model-gated off for current DMG-family models).
- Formal PPU mode-entry edge hooks now exist for future HBlank-DMA/HDMA wiring, but no HDMA/HBlank DMA behavior is implemented in current DMG scope.
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
node --test web/*.test.mjs
```

ROM test suites:

```bash
# Blargg
scripts/blargg/fetch_blargg_roms.sh
# Targeted performance guard used by CI to catch regressions earlier:
scripts/blargg/run_cpu_instrs_guard.sh
# Targeted audio/realtime mixer guard (local/dev, timeout-based):
scripts/dev/run_audio_guard.sh
# Targeted DMA scheduler debug guard (local/dev and CI, timeout-based):
scripts/dev/run_dma_guard.sh
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
- `TEST_NAME` to override the unit test executed by `scripts/dev/run_dma_guard.sh`.

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
- Web builds inject host wall-clock epoch time (`Date.now()`) into the core RTC path for MBC3 RTC state, avoiding browser target traps from direct wall-clock queries inside the core.
- Web demo now uses a retro DMG-inspired shell layout centered on the Game Boy screen, with a floating `Settings` panel hidden by default and toggled from a top-right two-line button near the `DOT MATRIX WITH STEREO SOUND` strip. Branding in the shell reads `Emulator GAME BOY TM`; runtime status text remains in `Debug` (below cartridge info). The left `BATTERY` indicator lights when a ROM is loaded and turns off when no ROM is active. `Settings` includes sections (`Save Data`, `System`, `Audio`, `Debug`) plus `Load ROM` without a persistent side column. Canvas scaling is manual via `System > Video Size` with integer steps (`x1` to `x4`) from the native DMG resolution (160x144), defaulting to `x4` and never changing automatically on responsive resize. The canvas shows a click target only while no cartridge is loaded (`Load ROM`), mapped to the same action as the settings control. `Reset ROM` lives in `Save Data` near SAV/RTC operations. The demo auto-starts emulation after `Load ROM` (and after `Reset ROM`) and keeps per-model boot ROM import/persistence in browser storage (dmg0/dmg/mgb/sgb/sgb2). `Load Boot ROM(s)` accepts one or many files, classifies each through the shared known-hash normalizer, stores only valid DMG-family matches by their canonical hardware slot, and shows a green validity check for the currently selected hardware when a valid boot ROM is present in browser storage.
- Web demo video controls now include a palette selector (`auto|dmg|mgb|cgb|sgb`), where `auto` maps from the loaded hardware model and scaffold options (`cgb`/`sgb`) remain display-only placeholders for future hardware-accurate color paths.
- SDL2 key mapping: arrows=`D-Pad`, `Z`=`B`, `X`=`A`, `Backspace`=`Select`, `Enter`=`Start`.
- SDL2 debug panel: press `F1` to open a cartridge metadata/warnings popup.
- SDL2/Web runtime wiring now uses `gb_runtime::session::RuntimeSession` (shared `GameBoy + FramePacer + AudioMixer` orchestration) to reduce frontend-specific timing/audio drift.
- SDL2 audio uses the core mixer clock bridge and queues stereo interleaved PCM in real time (now from the `frontends/sdl2` workspace package).
- SDL2 queue refill is driven by emulated audio t-cycles and enqueues only available core samples (no synthetic silence padding in refill blocks).
- SDL2 queue target is auto-tuned over time windows (same policy as web) using estimated underruns from elapsed playback vs queued samples.
- `scripts/dev/run_sdl2_frontend.sh` is the recommended local command for SDL2 builds/runs (including a `--release` mode for performance) and consistent macOS/Homebrew linker env setup.
- SDL2 renderer uses accelerated rendering with `present_vsync()` by default to reduce visible tearing during scroll/camera movement; override with `GB_SDL2_VSYNC=0` for diagnostics/perf comparisons.
- Optional SDL2 debug tone: set `GB_AUDIO_TEST_TONE=1`.
- Optional SDL2 core APU resampler quality override: set `GB_AUDIO_RESAMPLER=linear` or `GB_AUDIO_RESAMPLER=cubic` (default).
- Optional SDL2 VSync override: set `GB_SDL2_VSYNC=1` (default) or `GB_SDL2_VSYNC=0`.
- Optional SDL2 video palette override: set `GB_VIDEO_PALETTE=auto|dmg|mgb|cgb|sgb` (default: `auto`, model-based mapping).
- Battery-backed cartridges loaded via `gb_runtime::cartridge_persistence` persist external RAM to a sibling `.sav` file; MBC3 timer carts also persist RTC metadata to `.rtc`. Save writes use atomic temp-file+rename replacement. Current CLI frontend flushes saves on graceful exit; SDL2 also performs dirty-flag autosave with a short debounce window plus a flush on window focus loss, while keeping the graceful-exit flush. The web demo mirrors this policy with browser-side autosave debounce and page visibility/navigation flush hooks using local storage (browser quota/security policies may still block persistence).
- Web demo audio control uses a toggle button (`Enable audio` / `Disable audio`); audio is auto-enabled when a ROM is loaded/reset (user gesture path) when possible and can still be manually disabled/re-enabled during a session.
- Core helper: `GameBoy::set_audio_analog_calibration(profile)` to apply measured per-device analog calibration profiles from host/frontends.
- Web helpers:
  - `run_for_elapsed_micros(elapsed_micros)` to step as many emulated frames as host time allows.
  - `audio_clock_tcycles()` / `drain_audio_tcycles()` for raw emulated audio clock access.
  - `set_audio_sample_rate(rate_hz)` (preserves queued Core APU audio when reconfiguring WebAudio rate) and `drain_audio_samples(max_samples)` for queue-based WebAudio feeding (`Vec<f32>` stereo interleaved: `L,R,L,R,...`).
  - `observe_audio_queue_target(now_ms, queued_samples)`, `audio_queue_clear_required()`, and `commit_audio_queue_refill(now_ms, queued_samples_after_refill)` to drive queue-based host refill using shared runtime policy.
  - `audio_queue_refill_block_samples()` / `audio_queue_max_refill_blocks()` to read runtime-owned queue refill limits from wasm/web hosts.
  - `set_audio_resampler_quality("linear" | "cubic")` and `audio_resampler_quality()` to compare interpolation quality/CPU tradeoffs from frontends.
  - `drain_audio_samples_realtime(block_samples)` for callback-style fixed-size backends (`block_samples` = frames, returned buffer is stereo interleaved and zero-padded when short).
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

Boot ROM placement policy (local-only):
- Store boot ROM binaries under `roms/bootrom/`.
- Only `roms/bootrom/.gitkeep` is versioned; boot ROM binaries stay ignored.
- Recommended model file names: `dmg0_boot.bin`, `dmg_boot.bin`, `mgb_boot.bin`, `sgb_boot.bin`, `sgb2_boot.bin`.
- CLI/SDL2 frontends auto-load `<bootrom_dir>/<model>_boot.bin` when present and otherwise keep the existing post-boot default startup path.
- On frontend boot ROM lookup, the directory is normalized by known SHA-256 hashes (first 256 bytes): recognized dumps are renamed to canonical file names, and unknown/invalid blobs are prefixed with `invalid_` so they are excluded from autoload.
- CLI adds explicit per-run controls: `--no-bootrom` (force post-boot defaults) and `--bootrom-dir <path>` (override boot ROM lookup root).
- CLI boot ROM directory resolution order: `--bootrom-dir <path>` (if provided) -> `GB_BOOTROM_DIR` (if set) -> `roms/bootrom`.

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
