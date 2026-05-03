# Phase 2 architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a0bf92919a5df55df`)
Stage: post-Phase-2 review (Phase 2 commits `9e12253` + `931d2da`)
Subject: atomic source-move + manifest swap, with folded-in Phase-1 miniscript fix

## Verdict

Phase 2 is correct and complete; all critical and important items resolve cleanly from the source; one plan-text inaccuracy is nit-tier and goes to FOLLOWUPS.

## Critical issues

None.

## Important issues

None.

## Low/nit issues

**N1. Plan commit-message body falsely claims `cargo test --workspace` passes after Phase 2.** The plan's Task 4 Step 11 commit-message template states: "Workspace builds with default features and --all-features. The Phase-1 smoke test, which failed by design before this commit, now passes — TDD invariant satisfied." That is accurate. However, the explanatory note at line ~615 asserts `cargo test --workspace` passes because `cargo_bin("md")` resolves uniquely once md-codec drops `[[bin]]`. That claim is wrong in the interim state: md-codec/tests/ still contains 15 CLI integration tests (template_roundtrip.rs, json_snapshots.rs, vector_corpus.rs, cmd_*.rs, etc.) whose `[dev-dependencies]` (`assert_cmd`, `predicates`, `insta`, `tempfile`) were stripped from md-codec/Cargo.toml in this very commit. Those test targets fail to compile. The break is a known, expected, transient state resolved mechanically by Phase 3's `git mv` — but the plan's explanatory prose misrepresents it as passing. File a FOLLOWUPS entry under `v0.16.x` to correct the plan text.

## Architect's confirmations

**1. Atomicity.** The commit `9e12253` is a single atomic unit covering all manifest edits, source moves, and corpus-path pre-fixes simultaneously. The build is green at HEAD for the `cargo build --workspace` and `cargo build --workspace --all-features` targets. The Phase-1 smoke test at `crates/md-cli/tests/smoke.rs` (one encode round-trip) passes because md-cli/src/main.rs is now the real implementation, not the stub. The interim `cargo test --workspace` failure is expected (see N1) and Phase 3 resolves it mechanically.

**2. Phase-2 source-edit-window invariant.** All three corpus-path pre-fixes are confirmed in `9e12253`:
- `crates/md-codec/tests/template_roundtrip.rs` line 4: `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"))` — correct.
- `crates/md-codec/tests/json_snapshots.rs` line 7: same form — correct.
- `crates/md-codec/tests/vector_corpus.rs` line 13: `format!("{}/../md-codec/tests/vectors", env!("CARGO_MANIFEST_DIR"))` — correct `format!` variant.
No pre-fix was deferred.

**3. include!/format! path arithmetic.** The `CARGO_MANIFEST_DIR/../md-codec/tests/vectors/...` form is verified correct in both states. Pre-Phase-3: CARGO_MANIFEST_DIR = `…/crates/md-codec` (for the two `include!` files) so the path evaluates to `…/crates/md-codec/../md-codec/tests/vectors/…` = `…/crates/md-codec/tests/vectors/…` — correct. Post-Phase-3: CARGO_MANIFEST_DIR = `…/crates/md-cli`, path evaluates to `…/crates/md-cli/../md-codec/tests/vectors/…` = `…/crates/md-codec/tests/vectors/…` — correct. For md-cli/src/cmd/vectors.rs (already in md-cli post-Phase-2), CARGO_MANIFEST_DIR = `…/crates/md-cli` now and the `include!` resolves identically. One form, all states correct.

**4. Manifest strip.** `crates/md-codec/Cargo.toml` post-`9e12253` confirmed:
- No `[[bin]]` stanza — absent.
- No `[features]` block — absent.
- No CLI optional deps (clap, anyhow, miniscript, regex, serde, serde_json) — absent.
- No `[dev-dependencies]` block — absent (insta correctly dropped per Phase 0 verdict).
- Version is `0.15.2` — correct (Phase 4 bumps to 0.16.0).
- "with `md` CLI" in description and "command-line-utilities" in categories remain — deferred to FOLLOWUPS, as specified.

**5. `#[cfg(feature = "json")]` preservation.** `crates/md-cli/src/cmd/vectors.rs` line 42 has `#[cfg(feature = "json")]` intact. The `include!` substitution replaced lines 7-9 of the original `#[path]` block and did not disturb the json gate downstream.

**6. `#![allow(missing_docs)]` carryover.** `crates/md-cli/src/main.rs` line 1 is `#![allow(missing_docs)]` — confirmed present, auto-carried by `git mv`.

**7. Workspace lint compliance.** The `#![allow(missing_docs)]` at main.rs line 1 suppresses the workspace-lint `missing_docs = "warn"` for the entire md-cli crate. The three reported dead-code warnings (`CliError::Compile`, `JsonHeader`, `JsonChunkHeader`) are pre-existing: `CliError::Compile` is constructed only when `feature = "cli-compiler"` is active and Rust's dead-code analysis fires in the default build; `JsonHeader` and `JsonChunkHeader` are used only indirectly via serde serialization, which the lint cannot see through. None were introduced by Phase 2's edits — all three are structural properties of the moved source.

**8. Miniscript fix correctness.** `crates/md-cli/Cargo.toml` line 33: `miniscript = { workspace = true }` — unconditional, no `optional = true`. The `cli-compiler` feature at line 22 reads `cli-compiler = ["miniscript/compiler"]` (no `dep:miniscript` prefix, since miniscript is now a regular dep). This is consistent with the spec's intent: old md-codec had `default = ["cli", "json"]` with `cli = ["dep:miniscript", ...]`, meaning miniscript was always present in the default install. The fix preserves that behavior while eliminating the spurious `optional = true` that would have broken `cargo build -p md-cli --no-default-features` (it would have dropped miniscript, causing compile failures in `parse/template.rs` which calls miniscript unconditionally). Build matrix confirmed clean: default features (json, miniscript always), `--no-default-features` (no serde/serde_json; miniscript still present), `--features cli-compiler` (adds `miniscript/compiler`).

**9. Interim-state break characterization.** The `cargo test --workspace` failure is genuinely a missing-dev-deps issue: md-codec/tests/ still contains 15 CLI integration test files that `use assert_cmd::Command`, `use insta`, etc., but md-codec/Cargo.toml no longer declares those dev-deps. Cargo cannot build those test targets. This is not masking a deeper issue — `cargo test -p md-cli` works (md-cli has all dev-deps from Phase 1). Phase 3's `git mv` of the 15 test files and snapshots directory to md-cli/tests/ mechanically resolves the break with no new dev-dep declarations needed.

## Architect's open questions

None.
