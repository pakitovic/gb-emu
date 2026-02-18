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
  run_blargg.sh
  run_gekkio.sh
  fetch_blargg_roms.sh
  fetch_gekkio_roms.sh
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
scripts/fetch_blargg_roms.sh
scripts/run_blargg.sh
scripts/fetch_gekkio_roms.sh
scripts/run_gekkio.sh
# Optional local expansion (includes incremental ROMs):
GEKKIO_SUITE=incremental scripts/run_gekkio.sh
# Boot matrix by hardware model (dmg0/dmg/mgb/sgb/sgb2):
GEKKIO_SUITE=boot_models scripts/run_gekkio.sh
# Run a suite against a specific hardware model:
GB_MODEL=sgb GEKKIO_SUITE=incremental scripts/run_gekkio.sh
GB_MODEL=mgb scripts/run_blargg.sh
```

## CI

Workflows:
- `.github/workflows/quality.yml`: format, lint, build, unit/integration tests.
- `.github/workflows/rom-tests.yml`: two independent jobs/checks for branch protection:
  - `rom-blargg`
  - `rom-gekkio` (runs `incremental` + `boot_models`)

## Test ROMs and Licensing Notes

`/roms` is intentionally ignored in `.gitignore` to avoid storing test ROM binaries in this repository.

CI and local setup use `scripts/fetch_blargg_roms.sh`, which pulls public test ROM sources at runtime.

Why:
- Keeps the repository lightweight.
- Avoids redistributing binaries with mixed or unclear licensing terms.

`run_gekkio.sh` supports three profiles:
- `GEKKIO_SUITE=core` (default): stable timer set + `acceptance/instr/daa.gb`.
- `GEKKIO_SUITE=incremental`: `core` plus growing interrupt coverage (`acceptance/interrupts/ie_push.gb`, `acceptance/ei_sequence.gb`, `acceptance/ei_timing.gb`, `acceptance/di_timing-GS.gb`, `acceptance/if_ie_registers.gb`, `acceptance/intr_timing.gb`, `acceptance/rapid_di_ei.gb`, `acceptance/reti_intr_timing.gb`, `acceptance/reti_timing.gb`), OAM DMA coverage (`acceptance/oam_dma/basic.gb`, `acceptance/oam_dma/reg_read.gb`, `acceptance/oam_dma/sources-GS.gb`), bit-behavior coverage (`acceptance/bits/mem_oam.gb`, `acceptance/bits/reg_f.gb`, `acceptance/bits/unused_hwio-GS.gb`), boot-state coverage for current DMGABC profile (`acceptance/boot_regs-dmgABC.gb`, `acceptance/boot_div-dmgABCmgb.gb`, `acceptance/boot_hwio-dmgABCmgb.gb`), and instruction timing coverage (`acceptance/add_sp_e_timing.gb`, `acceptance/ld_hl_sp_e_timing.gb`, `acceptance/push_timing.gb`, `acceptance/pop_timing.gb`, `acceptance/call_timing.gb`, `acceptance/call_cc_timing.gb`, `acceptance/call_timing2.gb`, `acceptance/call_cc_timing2.gb`, `acceptance/jp_timing.gb`, `acceptance/jp_cc_timing.gb`, `acceptance/ret_timing.gb`, `acceptance/ret_cc_timing.gb`, `acceptance/rst_timing.gb`).
- `GEKKIO_SUITE=boot_models`: dedicated boot-state matrix per model defined in `scripts/gekkio_roms_boot_models.txt`.

CI currently runs `GEKKIO_SUITE=incremental` and `GEKKIO_SUITE=boot_models` as required ROM checks.

When adding new ROM suites, document:
- Source repository URL.
- Upstream license/status.
- Which script/workflow consumes them.

## Development Notes

- Keep code identifiers/comments in English.
- Prefer small, safe refactors with tests.
- Add/adjust tests whenever behavior changes (unit + integration as needed).
