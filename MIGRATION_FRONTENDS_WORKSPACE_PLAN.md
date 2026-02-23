# Frontends / Workspace Migration Plan

## Purpose

This document tracks the migration from the current single-crate mixed layout to a scalable multi-package workspace layout that cleanly separates:

- emulation systems (`GB` now, future `GBA`)
- frontend adapters (`CLI`, `SDL2`, `WASM`, future `libretro`)
- shared host/runtime utilities (pacing, frontend-facing audio bridge)
- browser host assets (`web/` JS/HTML)

This is a structural migration plan. Timing-sensitive emulator behavior must remain unchanged during layout refactors unless a phase explicitly says otherwise.

## Context (Current Repository State)

The current repository mixes core emulation code and frontend adapters in `/src`, while also having a separate browser host app in `/web`.

### Current entrypoints and frontend-related files

- `src/main.rs`
  - CLI/headless runner (ROM execution, `--blargg`, `--mooneye`, `--cart-info`, etc.)
- `src/bin/frontend_sdl2.rs`
  - SDL2 desktop frontend binary (window, events, input mapping, audio queue)
- `src/web.rs`
  - Rust/WASM bindings (`wasm-bindgen`) exporting `WebEmulator`
- `web/minimal/*`
  - Browser host app (HTML/JS/WebAudio/AudioWorklet) consuming the WASM build

### Current Cargo/dependency situation (why it feels mixed)

The root `Cargo.toml` currently holds:

- the emulator library (`[lib]`)
- CLI default binary (`src/main.rs`)
- SDL2 frontend binary (`[[bin]]` with `required-features = ["frontend-sdl2"]`)
- optional frontend dependencies and features:
  - `sdl2`
  - `wasm-bindgen`
  - `js-sys`

This works, but it blurs boundaries between:

- portable emulator core
- platform adapters
- browser host assets

### Naming collision that causes confusion

There are two different things called "web":

- `src/web.rs` = Rust/WASM frontend adapter
- `web/` = browser host app assets

This migration fixes that by moving the Rust/WASM adapter into `frontends/wasm/`, while keeping `web/` for browser assets only.

## Design Goals

### Functional goals

- Keep the emulator core frontend-agnostic.
- Make it easy to add future frontends (especially `libretro`).
- Prepare for future multiple systems/cores (e.g. `GB`, `GBA`).
- Avoid giant folders with unrelated files.

### Repository ergonomics goals

- Use simple folder names.
- Avoid a top-level `crates/` folder.
- Avoid repeating the project name in every folder name.
- Keep folder nesting shallow enough to avoid excessive editor folding.

### Refactor safety goals

- Prefer structural-only moves before logic changes.
- Keep timing-sensitive behavior unchanged in migration PRs.
- Preserve tests and validation scripts throughout the transition.

## Target Structure (Planned)

```text
gb-emu/
  Cargo.toml              # workspace root (orchestrates all packages)
  systems/
    gb/                   # Game Boy core (DMG/MGB/SGB/SGB2; future CGB)
      Cargo.toml
      src/
  runtime/                # shared host/frontend utilities (optional but recommended)
    Cargo.toml
    src/
  frontends/
    cli/
      Cargo.toml
      src/
    sdl2/
      Cargo.toml
      src/
    wasm/
      Cargo.toml
      src/
    libretro/             # future (create when implementation starts)
      Cargo.toml
      src/
  web/                    # browser host app assets (JS/HTML/CSS/worklets)
    minimal/
  scripts/
    dev/
    blargg/
    gekkio/
  tests/                  # integration tests (workspace-level, if needed)
  README.md
  MIGRATION_FRONTENDS_WORKSPACE_PLAN.md
```

## Why `systems/gb` Instead of Only `core/`

Separating the current Game Boy core into `systems/gb` helps future growth:

- `systems/gb` can keep all GB-family models (`dmg0`, `dmg`, `mgb`, `sgb`, `sgb2`, future `cgb`)
- `systems/gba` can be added later as a separate system without polluting the GB codebase
- frontends can depend on one or more systems cleanly
- testing and dependencies can stay isolated per system

Rule of thumb:

- another model/variant of the same platform -> same system package (`systems/gb`)
- another hardware platform -> new system package (`systems/gba`)

## Rust Workspace / `Cargo.toml` Notes (Important for This Migration)

This repo will use:

- one root `Cargo.toml` to orchestrate the workspace
- one `Cargo.toml` per package (system, frontend, runtime)

This is normal in Rust and does not imply a `crates/` folder.

### Why multiple `Cargo.toml` files are good here

- dependency isolation (`sdl2` only in `frontends/sdl2`, `wasm-bindgen` only in `frontends/wasm`)
- clearer compile targets
- simpler future growth (libretro/GBA/etc.)
- avoids feature flags in the system core for platform-specific code

### Folder names vs package names

Folder names are the priority for readability (`systems/gb`, `frontends/sdl2`, etc.).
Package names can be decided later and may differ slightly if needed to avoid naming conflicts.

Examples (not final decisions):

- folder `systems/gb` -> package name `gb-system` or `gb-core`
- folder `frontends/cli` -> package name `frontend-cli`
- folder `frontends/wasm` -> package name `frontend-wasm`

## Boundaries and Responsibilities

### `systems/gb`

Contains emulation logic and API surface for the Game Boy system:

- CPU/APU/PPU/timer/interrupts/MMIO/DMA
- cartridge loading/parsing/mappers/save persistence (system-level behavior)
- framebuffer and audio sample stream generation from emulation state
- public API used by frontends

Must not contain:

- SDL2/window/audio backend code
- `wasm-bindgen` exports
- browser DOM/JS integration
- libretro API bindings

### `runtime`

Contains frontend-shared host/runtime utilities that are not hardware behavior:

- host-time frame pacing (if separated from hardware timing)
- frontend-facing audio queue/adaptive buffering helpers
- reusable frontend orchestration helpers (optional)

This package is recommended, but it should be introduced only after the workspace/frontends split is stable.

### `frontends/cli`

Contains:

- CLI argument parsing
- headless run modes (`blargg`, `mooneye`, `cart-info`)
- CLI-specific error formatting and command wiring

### `frontends/sdl2`

Contains:

- SDL2 window/rendering
- SDL2 event loop
- keyboard input mapping
- SDL2 audio device/queue integration

### `frontends/wasm`

Contains:

- `wasm-bindgen` exports
- `WebEmulator` wrapper/adaptor API for browser JavaScript
- WASM-only glue code

### `web/`

Contains:

- browser host application
- JS/HTML/CSS
- AudioWorklet files
- browser integration tests (e.g. Node tests for helper modules)

No Rust package should live inside `web/`.

## Current -> Target Mapping (Initial Known Moves)

### Core/system migration

- `src/apu.rs` + `src/apu/` -> `systems/gb/src/apu.rs` + `systems/gb/src/apu/`
- `src/cpu/` -> `systems/gb/src/cpu/`
- `src/memory/` -> `systems/gb/src/memory/`
- `src/cartridge/` -> `systems/gb/src/cartridge/`
- `src/gameboy.rs` -> `systems/gb/src/gameboy.rs`
- `src/hardware.rs` -> `systems/gb/src/hardware.rs`
- `src/input.rs` -> `systems/gb/src/input.rs` (unless split later between system input API and frontend mappings)
- `src/lib.rs` -> `systems/gb/src/lib.rs`

### Frontend migration

- `src/main.rs` -> `frontends/cli/src/main.rs`
- `src/bin/frontend_sdl2.rs` -> `frontends/sdl2/src/main.rs`
- `src/web.rs` -> `frontends/wasm/src/lib.rs`

### Shared runtime candidates (to extract later)

The following are likely `runtime` candidates after frontends are split:

- frontend-facing audio mixing/resampling/queue helpers currently in `src/audio.rs`
- host-time pacing (`FramePacer`) currently used by SDL2/WASM

Important: only move code that is truly host/runtime behavior. Hardware semantics remain in `systems/gb`.

## Migration Strategy

This migration allows somewhat larger PRs than usual, but should still keep a clear separation between:

- structural moves/renames
- behavior changes

For timing-sensitive paths (CPU/PPU/APU/timer/interrupts/DMA), structural refactors should remain behavior-neutral.

## Roadmap (Phases / PR Plan)

Status values to use in this document:

- `TODO`
- `IN PROGRESS`
- `BLOCKED`
- `DONE`

### Phase 0 - Baseline Snapshot (Documentation + Validation)

Status: `DONE`

Goal:

- capture the current commands/entrypoints before structural changes
- establish a known-good baseline for comparison

Tasks:

- record current build/run commands in this file and/or `README.md`
- confirm current entrypoints and dependencies:
  - `src/main.rs`
  - `src/bin/frontend_sdl2.rs`
  - `src/web.rs`
  - `web/minimal/*`
- run validation baseline (full repo policy) and keep results attached to the migration PR or notes

Deliverable:

- baseline command matrix and results reference

### Phase 0 Baseline Snapshot (Executed)

Execution date:

- `2026-02-23` (local workspace session)

Working branch used for the migration baseline:

- `codex/phase0-baseline-snapshot`

Observed repository state before structural migration:

- Root package on `main` layout with mixed responsibilities under `src/`
- Untracked migration planning document present:
  - `MIGRATION_FRONTENDS_WORKSPACE_PLAN.md`
- Current frontend entrypoints confirmed:
  - `src/main.rs` (CLI/headless)
  - `src/bin/frontend_sdl2.rs` (SDL2 frontend binary)
  - `src/web.rs` (Rust/WASM adapter via `wasm-bindgen`)
  - `web/minimal/*` (browser host assets)
- Root `Cargo.toml` confirmed to contain:
  - root library + root CLI binary
  - optional frontend deps (`sdl2`, `wasm-bindgen`, `js-sys`)
  - root `[[bin]] frontend-sdl2`

ROM asset availability at baseline:

- `roms/blargg's_test_roms` present
- `roms/gekkio's_test_roms` present

#### Baseline command matrix and outcomes

| Command | Scope | Result | Notes |
| --- | --- | --- | --- |
| `cargo fmt-check` | Quality | PASS | No formatting diffs |
| `cargo lint` | Quality | PASS | `gb-emu` checked successfully |
| `cargo build --locked` | Quality | PASS | Root package builds |
| `cargo test --locked` | Quality | PASS | Unit + integration + CLI tests all passed |
| `node --test web/minimal/audio-adaptive.test.mjs` | Web helper test | PASS | 4/4 tests passed |
| `cargo build --locked --features frontend-web --lib` | WASM adapter compile path | PASS | Root package + `frontend-web` feature builds |
| `cargo build --locked --features frontend-sdl2 --bin frontend-sdl2` | SDL2 frontend compile path | PASS | SDL2 frontend binary builds successfully |
| `scripts/blargg/fetch_blargg_roms.sh` | ROM assets | PASS | ROMs already present locally |
| `scripts/blargg/run_blargg.sh` | ROM suite | PASS | `TOTAL=44 PASS=44 FAIL=0` |
| `scripts/gekkio/fetch_gekkio_roms.sh` | ROM assets | PASS | ROMs already present locally |
| `GEKKIO_SUITE=all scripts/gekkio/run_gekkio.sh` | ROM suite | PASS | `TOTAL=66 PASS=66 FAIL=0` |
| `GEKKIO_SUITE=boot_models scripts/gekkio/run_gekkio.sh` | ROM suite | PASS | `TOTAL=17 PASS=17 FAIL=0` |

#### `cargo test --locked` baseline summary (high level)

- `src/lib.rs` unit tests: `225 passed`
- `src/main.rs` unit tests (CLI parser): `4 passed`
- `tests/cli_cart_info.rs`: `1 passed`
- `tests/integration_smoke.rs`: `20 passed`
- doc tests: `0`

#### Phase 0 observations recorded for migration

- The root package currently mixes:
  - system core API (`src/lib.rs`, CPU/APU/memory/etc.)
  - root CLI binary (`src/main.rs`)
  - SDL2 frontend binary (`src/bin/frontend_sdl2.rs`)
  - Rust/WASM adapter module (`src/web.rs`)
- Root-level frontend features (`frontend-sdl2`, `frontend-web`) are currently used to gate platform-specific dependencies.
- The `web` naming collision is real and should be resolved by moving Rust/WASM code to `frontends/wasm`, while preserving `web/` for browser assets.
- SDL2 and WASM build paths are both healthy at baseline, so future migration regressions are easier to spot.

### Phase 1 - Introduce Workspace Root (No Functional Changes)

Status: `DONE`

Goal:

- establish a workspace root `Cargo.toml` that will orchestrate multiple packages

Tasks:

- convert the root Cargo layout to workspace mode
- decide whether this phase is:
  - `Option A`: workspace + keep current package in root temporarily (hybrid)
  - `Option B`: workspace + move system core immediately into `systems/gb`
- ensure all existing commands still work (or document temporary command changes)
- update `README.md` if command invocation changes

Recommended choice:

- `Option A` if minimizing churn
- `Option B` if prioritizing clean structure earlier and accepting a larger PR

Risks:

- command invocation changes during the transition
- scripts assuming root package layout

### Phase 1 Workspace Root (Executed)

Execution date:

- `2026-02-23` (local workspace session)

Working branch:

- `codex/workspace-phase1-root-hybrid`

Decision taken:

- `Option A` (hybrid): keep the current package in the repository root and add a workspace root in the same `Cargo.toml`.

Implementation summary:

- Root `Cargo.toml` now includes a workspace section with:
  - `members = ["."]`
  - `default-members = ["."]` (preserves root-package command behavior while future members are added incrementally)
  - `resolver = "3"`
- No package move/extraction is performed in this phase.
- No command invocation changes are introduced in this phase.

### Phase 2 - Extract `frontends/sdl2`

Status: `DONE`

Goal:

- move SDL2 frontend out of `src/bin` and isolate SDL2 dependencies

Tasks:

- create `frontends/sdl2/Cargo.toml`
- move `src/bin/frontend_sdl2.rs` -> `frontends/sdl2/src/main.rs`
- move `sdl2` dependency from root/core package to `frontends/sdl2`
- update scripts:
  - `scripts/dev/run_sdl2_frontend.sh`
  - any CI/local helper assuming `--bin frontend-sdl2` in the root package
- update `README.md` commands and structure section

Follow-up split inside `frontends/sdl2` (same or next PR depending size):

- `src/main.rs` -> thin entrypoint
- add:
  - `src/app.rs`
  - `src/audio.rs`
  - `src/video.rs`
  - `src/input.rs`
  - `src/args.rs`

Acceptance criteria:

- SDL2 frontend builds and runs
- behavior and controls remain unchanged
- no SDL2 dependency remains in the system package

### Phase 2 SDL2 Extraction (Executed)

Execution date:

- `2026-02-23` (local workspace session)

Working branch:

- `codex/workspace-phase2-extract-sdl2`

Implementation summary:

- Created `frontends/sdl2` workspace package and moved SDL2 frontend binary source:
  - `src/bin/frontend_sdl2.rs` -> `frontends/sdl2/src/main.rs`
- Added `frontends/sdl2/Cargo.toml` with:
  - path dependency on the root GB package (`gb-emu`)
  - local `sdl2` dependency
- Updated root `Cargo.toml` workspace members to include `frontends/sdl2`
- Removed SDL2-specific items from the root package:
  - `sdl2` dependency
  - `frontend-sdl2` feature
  - root `[[bin]] frontend-sdl2`
- Updated SDL2 helper script and README commands to use workspace package invocation:
  - `cargo build/run -p frontend-sdl2 ...`
- Preserved SDL2 binary name (`frontend-sdl2`) and runtime behavior

### Phase 3 - Extract `frontends/wasm` (Rust/WASM Adapter)

Status: `DONE`

Goal:

- move the Rust/WASM adapter out of the system package and resolve the `web` naming collision

Tasks:

- create `frontends/wasm/Cargo.toml`
- move `src/web.rs` -> `frontends/wasm/src/lib.rs`
- move `wasm-bindgen` and `js-sys` dependencies to `frontends/wasm`
- remove `frontend-web` feature from the system package (or replace with package-level dependency structure)
- update build scripts/docs for WASM output
- update `README.md` to explicitly distinguish:
  - `frontends/wasm` (Rust adapter package)
  - `web/` (browser host app)

Optional internal split (later if needed):

- `src/lib.rs` facade
- `src/emulator.rs`
- `src/audio.rs`
- `src/api.rs`

Acceptance criteria:

- WASM package builds
- browser demo integration remains functional
- no `wasm-bindgen`/`js-sys` dependency remains in the system package

### Phase 3 WASM Extraction (Executed)

Execution date:

- `2026-02-23` (local workspace session)

Working branch:

- `codex/workspace-phase3-extract-wasm`

Implementation summary:

- Created `frontends/wasm` workspace package and moved Rust/WASM adapter source:
  - `src/web.rs` -> `frontends/wasm/src/lib.rs`
- Added `frontends/wasm/Cargo.toml` with:
  - path dependency on the root GB package (`gb-emu`)
  - local `wasm-bindgen` and `js-sys` dependencies
- Updated root `Cargo.toml` workspace members to include `frontends/wasm`
- Removed WASM-specific items from the root package:
  - `wasm-bindgen` dependency
  - `js-sys` dependency
  - `frontend-web` feature
  - `web` module export from `src/lib.rs`
- Updated WASM helper script and README commands to build from the workspace package:
  - `wasm-pack build frontends/wasm --target web --out-name gb_emu`
- Preserved the browser demo import contract by keeping the generated output basename `gb_emu` for `web/minimal/pkg/gb_emu.js`

### Phase 4 - Extract `frontends/cli`

Status: `DONE`

Goal:

- move the headless CLI runner out of the system package

Tasks:

- create `frontends/cli/Cargo.toml`
- move `src/main.rs` -> `frontends/cli/src/main.rs`
- preserve CLI behavior and parser tests
- update scripts and README commands for CLI execution

Recommended internal split (same PR or immediate follow-up):

- `src/main.rs` (thin)
- `src/args.rs`
- `src/commands.rs` or `src/modes.rs`

Acceptance criteria:

- `--trace`, `--blargg`, `--mooneye`, `--cart-info`, `--model`, `--max-steps` keep behavior
- existing CLI tests remain present and passing

### Phase 4 CLI Extraction (Executed)

Execution date:

- `2026-02-23` (local workspace session)

Working branch:

- `codex/workspace-phase4-extract-cli`

Implementation summary:

- Created `frontends/cli` workspace package and moved the CLI frontend source:
  - `src/main.rs` -> `frontends/cli/src/main.rs`
- Added `frontends/cli/Cargo.toml` with:
  - path dependency on the root GB package (`gb-emu`)
  - explicit binary target name `gb-emu` to preserve script/CLI invocation compatibility
- Moved CLI integration test with the CLI package:
  - `tests/cli_cart_info.rs` -> `frontends/cli/tests/cli_cart_info.rs`
- Updated CLI usage/help text and README commands to run through the workspace package:
  - `cargo run -p frontend-cli --bin gb-emu -- ...`
- Updated ROM scripts (`blargg`, `blargg cpu guard`, `gekkio`) to build `frontend-cli` explicitly while keeping the binary path/name (`target/debug/gb-emu`)
- Updated CI quality workflow to run `cargo test --locked -p frontend-cli` so CLI parser/integration tests remain covered after extraction
- Preserved CLI behavior and flags (`--trace`, `--blargg`, `--mooneye`, `--cart-info`, `--model`, `--max-steps`)

### Phase 5 - Move System Core to `systems/gb`

Status: `TODO`

Goal:

- establish the first system package in the final multi-system layout

Tasks:

- create `systems/gb/Cargo.toml`
- move current system core files from root `src/` into `systems/gb/src/`
- update path dependencies from frontends to `systems/gb`
- keep public API stable where practical to minimize frontend churn
- update `README.md` project structure section and build examples

Notes:

- this is the key step that prepares the repo for a future `systems/gba`
- package name can be decided independently from folder name

Acceptance criteria:

- all frontends build against `systems/gb`
- ROM suites still pass (structural change only)

### Phase 6 - Extract `runtime` (Shared Frontend/Host Utilities)

Status: `TODO`

Goal:

- remove frontend-host-specific logic from the system package and share it cleanly across frontends

Tasks:

- create `runtime/Cargo.toml`
- identify and move host/runtime logic used by multiple frontends:
  - frame pacing (`FramePacer`) if it represents host scheduling/pacing
  - frontend audio queue/adaptive buffering helpers
  - frontend-facing audio mixing bridge code that is not hardware semantics
- keep hardware emulation logic in `systems/gb`
- update `frontends/sdl2` and `frontends/wasm` to use `runtime`

Important review rule:

- verify the move is semantic-neutral for timing-sensitive behavior
- avoid changing algorithm behavior while moving files/modules

Acceptance criteria:

- shared frontend utility logic is not duplicated across frontends
- `systems/gb` no longer depends on platform-specific frontend packages

### Phase 7 - Cleanup, Documentation, and Future-Proofing

Status: `TODO`

Goal:

- finalize the migration and document the layout for future contributors

Tasks:

- update `README.md` fully:
  - project structure
  - build/run commands per frontend
  - workspace command examples
- decide package naming conventions (if still pending)
- remove obsolete features/scripts/paths
- add a short section describing:
  - `systems/*`
  - `frontends/*`
  - `runtime/`
  - `web/`
- record future expansion rule:
  - `CGB` stays under `systems/gb` unless proven otherwise
  - `GBA` becomes `systems/gba`

Acceptance criteria:

- no stale commands or paths in docs/scripts
- layout is discoverable for a new contributor

## Suggested PR Bundling (Larger PRs Allowed)

The migration can be done in smaller phases, but larger PRs are acceptable for this effort.

Recommended bundles if fewer PRs are preferred:

- Bundle A:
  - Phase 1 + Phase 2 (workspace root + SDL2 extraction)
- Bundle B:
  - Phase 3 + Phase 4 (WASM + CLI extraction)
- Bundle C:
  - Phase 5 (move core to `systems/gb`)
- Bundle D:
  - Phase 6 + Phase 7 (runtime extraction + cleanup/docs)

Do not bundle:

- structural migration of timing-sensitive modules with unrelated logic changes

## Validation Plan (Per Repository Policy)

The repository policy expects full validation after changes. During this migration, each PR should run as much of the following as practical and report any skipped steps explicitly.

### Quality checks

```bash
cargo fmt-check
cargo lint
cargo build --locked
cargo test --locked
```

### ROM suites

```bash
scripts/blargg/fetch_blargg_roms.sh
scripts/blargg/run_blargg.sh
scripts/gekkio/fetch_gekkio_roms.sh
GEKKIO_SUITE=all scripts/gekkio/run_gekkio.sh
GEKKIO_SUITE=boot_models scripts/gekkio/run_gekkio.sh
```

### Frontend-specific checks (when affected)

- SDL2 frontend build/run smoke check
- WASM package build
- browser helper tests:

```bash
node --test web/minimal/audio-adaptive.test.mjs
```

### Validation reporting template (for each migration PR)

- Commands run
- Commands skipped
- Reason for each skipped command
- Residual risk (if any)

## Progress Tracker

Use this table as the migration source of truth.

| Phase | Title | Status | Branch | PR | Notes |
| --- | --- | --- | --- | --- | --- |
| 0 | Baseline snapshot | DONE | `codex/phase0-baseline-snapshot` |  | Full quality + frontend checks + Blargg/Gekkio baseline green |
| 1 | Workspace root | DONE | `codex/workspace-phase1-root-hybrid` | `#102` | Hybrid workspace root (`Cargo.toml`) added with root package kept in place |
| 2 | Extract SDL2 frontend | DONE | `codex/workspace-phase2-extract-sdl2` | `#103` | SDL2 frontend moved to `frontends/sdl2`; root package SDL2 dependency/bin removed |
| 3 | Extract WASM frontend | DONE | `codex/workspace-phase3-extract-wasm` | `#104` | Rust/WASM adapter moved to `frontends/wasm`; root package `frontend-web` feature and wasm deps removed |
| 4 | Extract CLI frontend | DONE | `codex/workspace-phase4-extract-cli` |  | CLI frontend moved to `frontends/cli`; root package no longer contains `src/main.rs` |
| 5 | Move core to `systems/gb` | TODO |  |  |  |
| 6 | Extract `runtime` | TODO |  |  |  |
| 7 | Cleanup and final docs | TODO |  |  |  |

## Implementation Checklist (Detailed)

### Global checklist (applies to each phase)

- [ ] Keep behavior unchanged unless the phase explicitly includes behavior work
- [ ] Keep tests with moved code (do not drop coverage during file moves)
- [ ] Update `README.md` when commands/structure/workflows change
- [ ] Update affected scripts under `scripts/dev/`, `scripts/blargg/`, `scripts/gekkio/`
- [ ] Run validation and report skipped steps/risk
- [ ] Record phase status in the tracker table above

### Frontend extraction checklist

- [ ] Frontend package has only frontend-specific dependencies
- [ ] System package does not retain platform-specific optional dependencies/features
- [ ] Build commands are documented
- [ ] Script wrappers (if any) target the new package path/name

### System core move checklist (`systems/gb`)

- [ ] Public API remains stable enough for frontends
- [ ] Path dependency updates are correct in all frontends
- [ ] Integration tests still target the expected binaries/packages
- [ ] ROM suites pass after structural move

### Runtime extraction checklist

- [ ] Host/runtime code moved without semantic changes
- [ ] Hardware semantics remain in `systems/gb`
- [ ] No duplicate pacing/audio helper code remains in SDL2/WASM frontends

## Decisions Recorded in This Plan

- We will use a Rust workspace with multiple `Cargo.toml` files (root + packages).
- We will not use a top-level `crates/` folder.
- Folder names should remain simple and scoped (`systems/`, `frontends/`, `runtime/`, `web/`).
- `web/` remains the browser host asset area (JS/HTML), not a Rust package.
- The Rust/WASM adapter moves to `frontends/wasm`.
- The current GB core will move to `systems/gb`.
- Future `GBA` work should use `systems/gba` (new system package).
- `CGB` should start as part of `systems/gb`, not as a separate system package.

## Open Decisions (Can Be Deferred)

These do not block Phase 1, but should be resolved during the migration:

- Final Cargo package names (folder names are already fixed by this plan)
- Whether to introduce `runtime/` immediately or only after all frontends are split
- Whether to rename `web/minimal/` later (not required for this migration)
- Which phases to bundle into larger PRs

## References (Architecture Inspiration)

Used as structural inspiration only (not a requirement to mirror):

- mGBA repository and `src/platform/*` layout (frontend/platform grouping)
- mooneye-gb repository with separate `core/` package (workspace split core/app)

This plan intentionally adapts those ideas to the needs of this repository:

- simpler folder names
- no `crates/`
- explicit future multi-system path (`systems/`)
