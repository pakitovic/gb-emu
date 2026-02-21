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

## Rust Module Organization and Visibility Policy
- Rust supports both `foo.rs` and `foo/mod.rs`; behavior is equivalent. This repository must use one consistent rule for new/refactored production modules.
- For new or refactored production modules with children, use:
  - Entry file: `<module>.rs`
  - Children: `<module>/...`
- Do not introduce new production `mod.rs` files.
- Existing legacy `mod.rs` files can remain as-is unless the task explicitly includes module layout migration.
- Do not partially migrate a module layout in behavior work:
  - If a module is touched for behavior, keep its current layout.
  - If layout migration is needed, do it as a dedicated structural refactor change.
- Keep re-exports in the module entry file only (`<module>.rs` or legacy `mod.rs`), and avoid long re-export chains from nested children.
- Visibility must be minimized and explicit:
  - Default to private items.
  - Use `pub(super)` for parent-only access.
  - Use `pub(in crate::<subsystem>)` for subsystem boundaries.
  - Use `pub(crate)` only when a cross-subsystem API is required.
  - Use `pub` only for intentionally public crate API surfaces.
- Refactor safety rule:
  - Structural refactors (split/move/rename/visibility tightening) must not change behavior.
  - Behavior changes must be in a separate commit/PR unless explicitly requested to combine.
  - For CPU/PPU/APU/timer/interrupt/DMA paths, keep or add characterization tests before/with structural refactors.
- Test-only module trees may keep using `mod.rs` when it improves fixture organization.

## Testing Policy (Mandatory)
For every behavior change:
- Add or update unit tests.
- Add or update integration tests when behavior crosses module boundaries, CLI behavior, cartridge loading, timing-sensitive interactions, or end-to-end emulator behavior.

For refactors:
- Keep refactors small.
- Do not unnecessarily constrain functional feature/bug work to tiny diffs; choose the scope needed to complete the requested behavior end-to-end.
- Add characterization/regression tests before changing behavior-sensitive code whenever practical.
- Before refactoring behavior-sensitive paths, verify existing tests cover the target behavior; if coverage is missing, add tests first.
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

## Script Organization Policy
- Never place scripts directly under `scripts/`.
- Scripts for development workflow (for example PR helpers, dependency bootstrap, local hook setup) must live in `scripts/dev/`.
- Domain/suite-specific scripts must live in their own subdirectory (for example `scripts/blargg/`, `scripts/gekkio/`).
- When introducing a new script category, create a dedicated subdirectory under `scripts/` and document it in `README`.

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
- Never implement changes directly on `main`; create a dedicated branch per logical change/feature.
- Create new branches from `main` using regular checkout flow (for example: `git checkout main` then `git checkout -b codex/<topic>`).
- Do not use `git worktree` for normal development unless explicitly requested by the user.
- Default to one PR per branch/change. If the request includes multiple sequential changes, split them into separate branches/PRs unless explicitly asked to combine.
- Parallel requests can use separate branches created from `main`; resolve conflicts later if they appear.
- Keep each branch incremental and small to minimize merge/rebase conflicts.
- Prepare commit title, PR title, and PR description in English.
- PR title must match the latest/main commit subject of the branch.
- For PR creation/update, use `scripts/dev/create_pr.sh` (default base: `main`), which pushes the branch and creates or updates the PR automatically.
- If the latest commit has no body, PR description should be initialized from `.github/pull_request_template.md`.
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
