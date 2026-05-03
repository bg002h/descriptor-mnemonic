# Final whole-PR architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `a67c3d612ac298a6c`)
Stage: full-branch review of feat/md-cli-extraction at HEAD `f78a93f`
Subject: complete md-cli extraction (13 commits, 5 phases)

## Verdict

Ready to merge. (One CHANGELOG accuracy fix landed inline post-review at `7ecbdd2`; see L1 below.)

## Acceptance criteria

1. **cargo build --workspace:** PASS — confirmed by Phase 2 review (Confirmation #1) and Phase 3 review. Workspace Cargo.toml line 3 lists both members; md-cli/src/main.rs and all submodules present; md-codec library-only with no bin source dependency.

2. **cargo build --workspace --all-features:** PASS — confirmed Phase 2 Confirmation #1; `cli-compiler = ["miniscript/compiler"]` in md-cli features; miniscript unconditional in md-cli deps; no features defined in md-codec so --all-features has nothing additional to activate there.

3. **cargo test --workspace:** PASS — Phase 3 review confirms all 15 CLI tests + 22 snapshots moved; 5 lib tests remain in md-codec; vector_corpus.rs CARGO_MANIFEST_DIR path arithmetic verified correct (Phase 3 Confirmation #2). Implementer-reported count of 358. The Phase 2 transient break (md-codec/tests/ files missing dev-deps) is fully resolved by Phase 3's git mv. No test in md-codec/tests/ requires assert_cmd after Phase 3.

4. **md binary parity infra:** PASS — md-cli/Cargo.toml lines 15–17: `[[bin]] name = "md" path = "src/main.rs"`. All 15 CLI tests at crates/md-cli/tests/ (Phase 3 Confirmation #1). Snapshots at crates/md-cli/tests/snapshots/ (Phase 3 Confirmation #4). main.rs line 50: `#[command(name = "md")]` explicit. One --version string phrasing note flagged in Low/nit below; all other parity infrastructure is correct.

5. **cargo check zero new warnings:** PASS — Phase 2 Confirmation #7: three dead-code warnings (CliError::Compile, JsonHeader, JsonChunkHeader) are pre-existing structural properties of the moved source, not regressions. `#![allow(missing_docs)]` at main.rs line 1 covers md-cli workspace-lint suppression. md-codec has no bin and no source changes; zero new warnings expected there.

6. **md-codec library-only manifest:** PASS — crates/md-codec/Cargo.toml lines 1–24: no `[[bin]]`, no `[features]` block, no CLI deps (clap, anyhow, miniscript, regex, serde, serde_json all absent), no `[dev-dependencies]`. Only bitcoin, thiserror, bip39 remain under `[dependencies]`.

7. **CHANGELOG entries:** PASS — CHANGELOG.md: preamble updated (line 3); md-cli [0.1.0] entry (line 7) with feature-carryover notes and --version behavioral note; md-codec [0.16.0] entry (line 24) with breaking-changes enumeration and Unchanged section. Both dated 2026-05-03.

8. **Per-phase reports persisted:** PASS — confirmed present: phase-0-audit-md-cli-extraction.md, phase-1-review-md-cli-extraction.md, phase-2-review-md-cli-extraction.md, phase-3-review-md-cli-extraction.md, phase-4-review-md-cli-extraction.md — all under design/agent-reports/.

9. **FOLLOWUPS entries filed:** PASS — 11 entries total: 4 spec-mandated (description-stale, categories-stale, vectors-out-dir-cwd-relative, path-dep-needs-version-for-publish) + 7 reviewer-surfaced. Tiers v0.16.1 or external, all Status: open, per Phase 4 Confirmation #11.

## Critical issues

None.

## Important issues

None.

## Low/nit issues

**L1. `md --version` reports `md 0.1.0`, not `md-cli 0.1.0`.** The CHANGELOG md-cli [0.1.0] entry described the version-output change as "md --version now reports `md-cli 0.1.0` instead of `md-codec 0.15.x`." The actual runtime output is `md 0.1.0` (was `md 0.15.2` pre-PR) because main.rs line 50 sets `#[command(name = "md")]` explicitly and clap derives --version from that name + CARGO_PKG_VERSION. The code is correct (binary should identify as `md`, not the package name); the CHANGELOG phrasing was inaccurate. **Resolved inline in commit `7ecbdd2`** — the CHANGELOG entry now correctly describes the change as "the binary still identifies as `md`; only the version number drops from `md 0.15.2` to `md 0.1.0`."

**L2. Stale description and categories in md-codec/Cargo.toml.** Already tracked under `v0.16-md-codec-cargo-toml-description-stale` and `v0.16-md-codec-cargo-toml-categories-stale`. Confirmed open; no action in this PR.

## Cross-phase observations

**Phase 2 hotfix (`f9e01ee`) is the one process anomaly.** The `include!` substitution in cmd/vectors.rs was in the working tree but unstaged in commit `9e12253`. Phase 2's architect review mis-confirmed it as committed. The hotfix at `f9e01ee` is correct and sufficient; the anomaly is documented in FOLLOWUPS (`v0.16-phase2-review-mis-confirmation-process-note`) and a feedback memory was recorded. No residue on the final branch state.

**No other missing-staged-edits pattern found.** Phase 3 moves are pure renames with zero source edits required (corpus-path pre-fixes were done in Phase 2 as planned). Phase 4 touches only Cargo.toml version line, CHANGELOG.md, and FOLLOWUPS.md — all independent documents. No evidence of additional unstaged edits surviving to HEAD.

**Spec patched mid-stream (`5d6485c`, `931d2da`) — no residual drift.** `5d6485c` reclassified vector_corpus.rs from lib to CLI (Phase 0 audit ground-truth). `931d2da` folded in the miniscript non-optional fix. Both patches are correctly reflected in the final implementation: vector_corpus.rs is in md-cli/tests/; miniscript is unconditional in md-cli/Cargo.toml.

**Plan Task 4 Step 11 prose claim (incorrect).** The plan's commit-message template incorrectly claims `cargo test --workspace` passes after Phase 2. Already tracked in FOLLOWUPS (`v0.16-plan-task4-step11-cargo-test-claim-incorrect`). No code impact; plan is historical at this point.

## Test count audit

Pre-PR: md-codec had 5 lib test files + 15 CLI integration test files. Phase 1 adds 1 scaffold smoke test in md-cli. Phase 3 moves 15 CLI tests to md-cli — conserving count. Post-PR: md-codec has 5 lib test targets; md-cli has 16 test targets (1 scaffold smoke + 15 moved CLI tests). Implementer-reported total of 358 is consistent with this breakdown. No unexplained additions or removals.

## Binary parity hazards (preview for Task 12)

**`md --version` differs by design:** pre-PR `md 0.15.2`; post-PR `md 0.1.0`. Only intentional diff.

**No feature-gate flips:** `json` defaults on in both pre-PR and post-PR (`default = ["json"]`). `cli-compiler` is opt-in in both. All `#[cfg(feature = "json")]` blocks preserved (cmd/vectors.rs line 42, json_snapshots.rs line 3, etc.).

**No command additions or removals:** the 8-subcommand dispatch (encode, decode, verify, inspect, bytecode, vectors, compile, address) is identical. Confirmed from main.rs lines 56–178.

**No exit-code changes:** main.rs lines 180–193: exit 0 on Ok, exit 2 on BadArg, exit 1 on other errors. Identical to pre-PR logic.

**json_snapshots.rs gating:** `#![cfg(feature = "json")]` at line 3 — active under `default = ["json"]`, same as pre-PR.

No behavioral hazards identified. Task 12's diff should show only the version-string line.

## Architect's confirmations

1. md-codec/Cargo.toml: library-only, version 0.16.0, no [[bin]], no [features], no CLI deps, no [dev-dependencies]. Confirmed lines 1–24.
2. md-cli/Cargo.toml: [[bin]] name = "md", version 0.1.0, correct features, all required deps and dev-deps. Confirmed lines 1–39.
3. Workspace Cargo.toml: resolver = "3", members = ["crates/md-codec", "crates/md-cli"]. Confirmed lines 1–3.
4. All 5 per-phase reports present in design/agent-reports/ with correct naming.
5. 11 FOLLOWUPS entries filed; all Status: open, appropriate tiers.
6. CHANGELOG preamble updated; both crate entries present with correct dates.
7. cmd/vectors.rs include! path arithmetic correct (CARGO_MANIFEST_DIR/../md-codec/tests/vectors/manifest.rs); #[cfg(feature = "json")] gate preserved.
8. `#![allow(missing_docs)]` at md-cli/src/main.rs line 1.
9. Phase 2 hotfix at `f9e01ee` is on the branch; anomaly is resolved and process-documented.
10. 15 CLI tests at md-cli/tests/; 5 lib tests at md-codec/tests/; vectors corpus stays at md-codec/tests/vectors/.

## Architect's open questions

None. Task 12 (binary parity check) is the remaining operational verification and can proceed immediately.
