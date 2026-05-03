# Plan-stage architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a263c0133cf6ec5a3`)
Stage: written-plan review (pre-execution)
Subject: `design/IMPLEMENTATION_PLAN_md_cli_extraction.md`

## Verdict

Fix-then-execute. Two critical bug-finds in plan steps, several important issues, all fixed inline before commit.

## Critical issues (fixed inline)

**C1.** Two test files (`crates/md-codec/tests/template_roundtrip.rs` line 4, `crates/md-codec/tests/json_snapshots.rs` line 7) use `include!("vectors/manifest.rs")` — a relative path that resolves correctly while the file lives in `md-codec/tests/` but **breaks at compile time** the moment Phase 3's `git mv` lands the file in `md-cli/tests/` (the `vectors/` corpus stays in md-codec). Phase 3's `cargo test --workspace` step would not compile.

**Fix applied:** new Phase 2 Step 4 pre-fixes both files in place. The `concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs")` form resolves identically pre-move (`crates/md-codec/../md-codec/tests/...`) and post-move (`crates/md-cli/../md-codec/tests/...`). Both states evaluate to `crates/md-codec/tests/vectors/manifest.rs`. The plan's Phase 3 ("no source edits") invariant is preserved by doing the edit during Phase 2's source-edit window.

**C2.** Phase 2 Task 4 Step 2's three flat-file `git mv` commands target `crates/md-cli/src/{main,error,compile}.rs` — but after Step 1's `git rm crates/md-cli/src/main.rs`, the `crates/md-cli/src/` directory may not exist in the working tree (an empty directory with no tracked files is removed). `git mv` of a single file requires the destination's parent directory to exist, so Step 2 would fail. The directory moves (`cmd/`, `format/`, `parse/`) are unaffected — `git mv srcdir destdir` works whether or not `destdir` exists.

**Fix applied:** Phase 2 Step 2 now opens with `mkdir -p crates/md-cli/src` before the flat-file moves.

## Important issues (fixed inline)

**I2.** Task 4 Step 5's stripped `crates/md-codec/Cargo.toml` ended with a bare `[dev-dependencies]` heading. Cargo accepts that, but it's misleading — and the conditional note said "empty if Phase 0 found insta is CLI-only" while the template wrote the heading regardless.

**Fix applied:** the template now omits `[dev-dependencies]` entirely; the conditional note instructs the implementer to *add* the section header along with the `insta` line if Phase 0 said keep.

**I3.** CHANGELOG.md's existing preamble reads "All notable changes to `md-codec` are documented in this file." Once md-cli entries appear, that sentence is wrong. Plan didn't address the preamble.

**Fix applied:** Phase 4 Task 8 Step 2 now opens with an explicit preamble update before prepending the two version entries.

## Low/nit (fixed inline)

**L1.** Phase 2 Task 4 Step 5 had a defensive `git show HEAD~:crates/md-cli/Cargo.toml` diff to verify md-cli/Cargo.toml unchanged from Phase 1. `HEAD~` is brittle if Phase 1 produced a fix-up commit during its agent review.

**Fix applied:** removed the diff-check step entirely. The Phase-1 manifest is final by spec; if the implementer somehow edited it during Phase 2, the verification builds and tests would catch it (or wouldn't, but the diff check itself adds little).

**L4.** Phase 2 architect-review brief (Task 5 Step 1) said "verify md-cli/Cargo.toml is unchanged from Phase 1" — echoes spec language about a manifest-stub-then-fill pattern that the plan abandoned (Phase 1 creates the full manifest).

**Fix applied:** brief rewritten to accurately describe what Phase 2 touches in md-cli (zero — only md-codec/Cargo.toml + 3 source files).

## Architect's confirmations

- **TDD framing:** Phase 1's failing smoke test is a bisect anchor, not design-driving TDD. Plan is honest about this; not over-claiming. Fair.
- **Phase atomicity:** With C1+C2 fixes, every phase commit is independently buildable.
- **Task 12's `git switch --detach`:** safe; untracked files unaffected.
- **`git mv` with `git rm`-then-`git mv` sequence:** correct; no `--force` needed.
- **Plan ↔ spec coverage:** sampled and confirmed comprehensive.

## Architect's open question (resolved without code change)

- Architect raised L5: TDD theater vs. genuine TDD. Resolved as "fair framing" — the plan accurately calls Phase 1 a TDD invariant in the bisect-anchor sense, not in the design-driving sense. No edit needed.
