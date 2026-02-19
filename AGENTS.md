# AGENTS.md

## Mission
Maintain and evolve this Game Boy emulator with senior-level engineering standards, with special care for state machines, hardware behavior, CPU timing, registers, interrupts, and instruction-level correctness.

## Instruction Priority
- This file is authoritative for this repository.
- If there is ambiguity, choose the most conservative interpretation that preserves emulator correctness and test reliability.
- Do not apply personal style preferences if they conflict with repository conventions.

## Engineering Mindset
- Think and implement as a senior software engineer specialized in:
  - Finite state machines.
  - Low-level hardware emulation.
  - Assembly-level CPU behavior.
  - Register/memory-mapped I/O semantics.
  - Timing-sensitive logic (cycles, phases, edge-triggered behavior).
- Prioritize deterministic and explainable behavior over clever shortcuts.
- Prefer explicit models of hardware state transitions (well-defined enums/struct state) over implicit flag coupling.
- Keep the emulator core frontend-agnostic and portable; treat SDL2/web/mobile/libretro as adapter layers on top of the same core API.

## Sources and Research Before Changes
When behavior is unclear or hardware-accurate behavior is required:
- First consult official and primary references.
- Then compare with mature open-source emulators with similar scope.
- Document assumptions in code comments when needed (short and precise).

Recommended references:
- Pan Docs.
- Game Boy CPU manual/reference material.
- Mature open-source emulators (for comparison only), e.g. SameBoy, mGBA, Gambatte, BGB-related documentation.

## Code Rules (Rust)
- Keep code and code comments in English.
- Follow current Rust stable practices and idioms.
- Prefer small, composable functions with clear responsibilities.
- Model state explicitly; avoid hidden side effects.
- Use explicit integer types and explicit wrapping/overflow intent (`wrapping_*`, `checked_*`, etc.) when relevant.
- Avoid magic numbers in hardware logic: use named constants for registers, bit masks, timing windows, and memory ranges.
- Use `Result`/custom error types for fallible flows; avoid panics in runtime paths.
- `panic!` is acceptable in tests for assertion clarity.
- Keep `match` handling exhaustive and readable.
- Avoid `unsafe` unless strictly necessary; if used, justify with a focused safety comment.
- Respect lint rules (`clippy -D warnings`) and formatting (`rustfmt`).
- Avoid host-side I/O side effects inside the core (windowing, audio backend, stdout rendering); expose data through APIs for frontend adapters.

## Testing Policy (Mandatory)
For every behavior change:
- Add or update unit tests.
- Add or update integration tests when behavior crosses module boundaries, CLI behavior, cartridge loading, timing-sensitive interactions, or end-to-end emulator behavior.

For refactors:
- Keep refactors small.
- Add characterization/regression tests before changing behavior-sensitive code whenever practical.
- Verify refactor safety by keeping tests green before and after.

## Required Validation After Changes
After making changes, run everything aligned with CI and ROM validation scripts.

Quality checks:
```bash
cargo fmt-check
cargo lint
cargo build --locked
cargo test --locked
```

ROM suites:
```bash
scripts/blargg/fetch_blargg_roms.sh
scripts/blargg/run_blargg.sh
scripts/gekkio/fetch_gekkio_roms.sh
GEKKIO_SUITE=all scripts/gekkio/run_gekkio.sh
GEKKIO_SUITE=boot_models scripts/gekkio/run_gekkio.sh
```

If any step cannot run (environment/time/resource constraints), explicitly report:
- Which command was not run.
- Why.
- What risk remains.

## Documentation Policy
README must be kept up to date whenever relevant changes happen.

Always update README when changes affect:
- Features and current scope.
- Requirements/prerequisites.
- Limitations and known constraints.
- CLI flags/usage.
- Test workflows (quality, ROM suites, CI expectations).

## Change Scope and Safety
- Prefer incremental, reviewable changes.
- Avoid large speculative rewrites.
- Preserve existing architecture and naming patterns unless a change is justified and covered by tests.
- Keep new dependencies minimal and justified.

## Definition of Done (Task/PR)
Before considering a task complete, all of the following must hold:
- Behavior is implemented correctly and consistent with emulator architecture and hardware semantics.
- Unit and integration tests are added/updated where applicable and pass.
- Required validation commands (quality + ROM suites) are executed and pass, or non-executed steps are explicitly reported with risk.
- README is updated when features, requirements, limitations, workflows, or usage changed.
- Remaining risks, assumptions, and follow-ups are stated clearly in the final report.

## Performance and Regression Policy
- Treat timing/cycle behavior as first-class correctness.
- For timing-sensitive changes, verify no accidental regressions in cycle-sensitive test ROMs and existing tests.
- Prefer small measurable changes; avoid broad rewrites that make performance/correctness attribution difficult.
- If a tradeoff is necessary (readability vs speed, fidelity vs complexity), document why and what was validated.

## Git and PR Conventions
- Use small, focused commits with imperative commit messages.
- Keep PRs narrowly scoped and reviewable.
- If creating branches, use the `codex/` prefix.
- PR descriptions should include:
  - Problem statement.
  - Behavioral change summary.
  - Tests added/updated.
  - Validation commands and outcomes.
  - Risks or known limitations.

## Bug Traceability and Regression Rules
- Every bug fix must include a reproducible description (input ROM/test case + observed vs expected behavior).
- Add a regression test that would fail before the fix whenever practical.
- Prefer writing the failing test first for behavior-critical paths (CPU, PPU, timer, interrupts, DMA, memory map, boot profiles).
- Keep bug fixes minimal and isolated; avoid unrelated refactors in the same change.

## AGENTS.md Continuous Improvement
- If a PR introduces significant new patterns, tooling, workflows, hardware scope, or validation strategy, suggest updating `AGENTS.md`.
- If those changes become recurring practice, include the update in the same PR or in a follow-up PR.
- Keep `AGENTS.md` concise, practical, and aligned with how the project is actually maintained.

## Practical Checklist for Each Task
1. Understand affected hardware/state-machine behavior.
2. Read surrounding code and existing tests first.
3. Implement minimal, consistent change.
4. Add/update unit + integration tests as needed.
5. Update README if scope/usage/limits changed.
6. Run full quality and ROM validation commands.
7. Report results and remaining risks clearly.
