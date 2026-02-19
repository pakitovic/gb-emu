# gb-emu

Personal/hobby Game Boy emulator project written in Rust, focused on learning and incremental milestones.

Current scope:
- ROM-only plus basic MBC1/MBC5 support.
- CPU core with growing opcode coverage.
- Memory bus + timer/interrupt basics.
- Blargg ROM test integration in local scripts and CI.

## Project Structure

```text
src/
  cartridge/
  cpu/
  gameboy.rs
  hardware.rs
  memory/
  lib.rs
  main.rs
tests/
  integration_smoke.rs
scripts/
  blargg/
    fetch_blargg_roms.sh
    run_blargg.sh
    rom.txt
  gekkio/
    fetch_gekkio_roms.sh
    run_gekkio.sh
    roms_core.txt
    roms_boot_models.txt
    roms_acceptance_ppu.txt
  setup-hooks.sh
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

## Quality and Tests

Formatting/lint aliases are defined in `.cargo/config.toml`.

```bash
cargo fmt-check
cargo lint
cargo test --locked
```

Blargg suite:

```bash
scripts/blargg/fetch_blargg_roms.sh
# Runs all configured DMG Blargg ROMs:
scripts/blargg/run_blargg.sh
scripts/gekkio/fetch_gekkio_roms.sh
scripts/gekkio/run_gekkio.sh
# Default stable Gekkio core suite:
GEKKIO_SUITE=core scripts/gekkio/run_gekkio.sh
# Optional local WIP block (acceptance/ppu/*):
GEKKIO_SUITE=acceptance_ppu scripts/gekkio/run_gekkio.sh
# Boot matrix by hardware model (dmg0/dmg/mgb/sgb/sgb2):
GEKKIO_SUITE=boot_models scripts/gekkio/run_gekkio.sh
# Run a suite against a specific hardware model:
GB_MODEL=sgb GEKKIO_SUITE=core scripts/gekkio/run_gekkio.sh
GB_MODEL=mgb scripts/blargg/run_blargg.sh
```

## CI

Workflows:
- `.github/workflows/quality.yml`: format, lint, build, unit/integration tests.
- `.github/workflows/rom-tests.yml`: two independent jobs/checks for branch protection:
  - `rom-blargg`
  - `rom-gekkio` (runs `core` + `boot_models`; also runs `acceptance_ppu` as non-blocking)

## Test ROMs and Licensing Notes

`/roms` is intentionally ignored in `.gitignore` to avoid storing test ROM binaries in this repository.

CI and local setup use `scripts/blargg/fetch_blargg_roms.sh`, which pulls public test ROM sources at runtime.

Why:
- Keeps the repository lightweight.
- Avoids redistributing binaries with mixed or unclear licensing terms.

`scripts/gekkio/run_gekkio.sh` supports three profiles:
- `GEKKIO_SUITE=core` (default): stable acceptance set defined in `scripts/gekkio/roms_core.txt`.
- `GEKKIO_SUITE=acceptance_ppu`: WIP block for `acceptance/ppu/*` tests (kept separate from core).
- `GEKKIO_SUITE=boot_models`: dedicated boot-state matrix per model defined in `scripts/gekkio/roms_boot_models.txt`.

`scripts/blargg/run_blargg.sh` runs the full DMG Blargg set defined in:
- `scripts/blargg/rom.txt`

`scripts/blargg/fetch_blargg_roms.sh` mirrors upstream `retrio/gb-test-roms` layout for the selected ROMs and writes a local listing to:
- `roms/blargg's_test_roms/.blargg_listing.txt`

Current Blargg DMG selection intentionally excludes:
- `cgb_sound/*`
- `interrupt_time/*`

CI currently runs `GEKKIO_SUITE=core` and `GEKKIO_SUITE=boot_models` as required ROM checks, and runs `GEKKIO_SUITE=acceptance_ppu` as non-blocking.

When adding new ROM suites, document:
- Source repository URL.
- Upstream license/status.
- Which script/workflow consumes them.

## Development Notes

- Keep code identifiers/comments in English.
- Prefer small, safe refactors with tests.
- Add/adjust tests whenever behavior changes (unit + integration as needed).
