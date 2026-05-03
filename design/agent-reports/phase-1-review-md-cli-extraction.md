# Phase 1 architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `ac1219e029f20b4c7`)
Stage: post-Phase-1 review (Phase 1 commit `6e339b8`)
Subject: Phase 1 of the md-cli extraction (scaffold + failing smoke)

## Verdict

Proceed to Phase 2.

## Critical issues

None.

## Important issues

None.

## Low/nit issues

1. `cargo_bin("md")` in smoke.rs resolves by binary output name, not package. In a workspace with two `[[bin]] name = "md"` targets (md-codec and md-cli), assert_cmd's `cargo_bin("md")` builds the binary associated with the test's own package context when run via `cargo test -p md-cli`. This works correctly today. However, once Phase 3 moves the full CLI test suite into md-cli and the old md-codec CLI tests are retired, a residual `[[bin]] name = "md"` in md-codec (guarded by `required-features = ["cli"]`) will still exist until Phase 5 removes it. During that window — Phases 3-4 — `cargo test --workspace` will compile two `md` binaries to the same `target/debug/md` path, with the last-linked one winning. In practice `cargo test -p md-cli` (the prescribed invocation) pins context correctly, but any naive `cargo test --workspace` that runs md-cli tests immediately after md-codec tests could pick up the wrong binary. Recommend adding a comment to smoke.rs noting the `-p md-cli` invocation requirement, or documenting this in the Phase 3 checklist. Tier: v0.16.x. Source: phase-1-review.

2. `crates/md-cli/Cargo.toml` omits `readme`, `homepage`, and `documentation` fields that are present in md-codec's manifest. This is appropriate for a v0.1.0 pre-release crate (no README exists yet, no docs.rs page). When md-cli becomes the primary published crate at v0.16.0, these fields should be added. Tier: v0.16.x. Source: phase-1-review.

## Architect's confirmations

- `crates/md-cli/Cargo.toml` matches the spec template at `design/SPEC_md_codec_v0_16_library_only.md` lines 99-138 verbatim. No drift.
- `Cargo.toml` workspace `members` line matches plan Task 2 Step 1 prescription exactly: `["crates/md-codec", "crates/md-cli"]`.
- `crates/md-cli/src/main.rs` matches plan Task 2 Step 3 prescription exactly: `#![allow(missing_docs)]` + `fn main() { unimplemented!("md-cli scaffold; replaced atomically in Phase 2") }`.
- `crates/md-cli/tests/smoke.rs` matches plan Task 2 Step 4 prescription exactly, including the canonical phrase `md1qqpqqxqxkceprx7rap4t` sourced from the existing `md encode --help` after_long_help text at `crates/md-codec/src/bin/md/main.rs:59`.
- TDD failure mode is correct: `unimplemented!()` triggers a panic, which exits the process with a non-zero status code. `assert_cmd`'s `.assert().success()` fails on any non-zero exit. The test cannot pass spuriously — the canonical phrase is the codec's deterministic output for the given template, and the stub binary never reaches any encoding logic.
- Workspace inheritance is correctly applied: `edition.workspace`, `rust-version.workspace`, `license.workspace`, `repository.workspace`, and `[lints] workspace = true` all delegate to `[workspace.package]` fields in the root `Cargo.toml`.
- The `miniscript = { workspace = true, optional = true }` dep correctly inherits `version = "13.0.0", default-features = false, features = ["std"]` from `[workspace.dependencies]`. No version skew.
- All declared deps (`clap`, `anyhow`, `regex`, `bitcoin`, `serde`, `serde_json`, `miniscript`) are consumed by the existing bin source in `crates/md-codec/src/bin/md/` and will be needed when Phase 2 moves that source.
- Commit message subject for `6e339b8` matches the plan's Task 2 Step 8 prescription character-for-character.

## Architect's open questions

None.

---

Both low/nit items deferred to FOLLOWUPS (filed in Task 9, v0.16.x tier).
