# Phase 3 architect review — md-cli extraction

Date: 2026-05-03
Reviewer: feature-dev:code-architect (agent id `abc4848b4b7d400ee`)
Stage: post-Phase-3 review (Phase 3 commit `cccd1ea`, with prior hotfix `f9e01ee`)
Subject: CLI integration tests + snapshots moved from md-codec to md-cli

## Verdict

Phase 3 is correct and complete. All 15 CLI test files and the snapshots directory moved as pure renames with zero source edits. The 5 library tests and the `tests/vectors/` corpus remained in md-codec as required. The only residue is stale `source:` YAML headers in the 22 snapshot files — this is the expected insta behavior and resolves automatically on the next `cargo insta review` cycle. No issues require a fix before Phase 4.

## Critical issues

None.

## Important issues

None.

## Low/nit issues

**N1. Stale `source:` headers in moved snapshot files.** All 22 `.snap` files contain insta-generated metadata such as `source: crates/md-codec/tests/json_snapshots.rs` (confirmed in `crates/md-cli/tests/snapshots/json_snapshots__decode@wpkh_basic.snap` line 2 and `json_snapshots__inspect@wpkh_basic.snap` line 2). After the `git mv`, the source path no longer matches the file's location. This is a known insta behavior: `source:` is informational metadata, not a path insta reads at test runtime. Snapshot matching is keyed on the snapshot name, not the source header. The headers will be corrected the next time `cargo insta review` or `cargo insta accept` is run. No test failures result from this residue. File under `v0.16.x` as documentation noise with no functional impact.

**N2. Phase 2 review falsely confirmed include! hotfix landed in `9e12253`.** The Phase 2 review (Confirmation #2) stated all three corpus-path pre-fixes landed in `9e12253`. In fact, the `cmd/vectors.rs` `include!` edit was performed in the working tree but never staged; the hotfix at `f9e01ee` committed it between Phase 2 and Phase 3. The Phase 2 review mis-confirmed a working-tree state as a committed state. The remediation is in place (hotfix committed; a feedback memory was recorded — `feedback_verify_committed_content_not_working_tree.md`). Recommend no further action beyond this observation — the hotfix is on the branch and the review process now has a concrete example of why "confirm in git show, not in the working tree" is the verification standard. File under `v0.16.x` as a process note.

## Architect's confirmations

**1. All 15 CLI test files moved as pure renames.** Confirmed by checking each file at its new path in `crates/md-cli/tests/`: `cmd_address.rs`, `cmd_address_json.rs`, `cmd_bytecode.rs`, `cmd_compile.rs`, `cmd_decode.rs`, `cmd_encode.rs`, `cmd_inspect.rs`, `cmd_verify.rs`, `compile.rs`, `exit_codes.rs`, `help_examples.rs`, `json_snapshots.rs`, `scaffold.rs`, `template_roundtrip.rs`, `vector_corpus.rs`. All open with `use assert_cmd::Command` at line 3 — no source modifications relative to their pre-Phase-3 content.

**2. vector_corpus.rs path arithmetic correct.** `crates/md-cli/tests/vector_corpus.rs` line 13: `format!("{}/../md-codec/tests/vectors", env!("CARGO_MANIFEST_DIR"))`. Post-Phase-3, `CARGO_MANIFEST_DIR` = `.../crates/md-cli`; the path evaluates to `.../crates/md-cli/../md-codec/tests/vectors` = `.../crates/md-codec/tests/vectors`. Correct per Phase 2 review Confirmation #3 path arithmetic (still valid post-Phase-3).

**3. template_roundtrip.rs and json_snapshots.rs include! paths correct.** Both files use `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"))` (confirmed at lines 4 and 7 respectively). Same arithmetic — resolves to `crates/md-codec/tests/vectors/manifest.rs`, which still exists.

**4. Snapshots directory moved; old location absent.** `crates/md-codec/tests/snapshots/` no longer exists. `crates/md-cli/tests/snapshots/` exists and contains insta artifacts with correct snapshot content (body field, schema field, etc.).

**5. Five library tests remained in md-codec.** `address_derivation.rs`, `chunking.rs`, `forward_compat.rs`, `smoke.rs`, `wallet_policy.rs` all exist at `crates/md-codec/tests/` and open with `md_codec::*` imports, no `assert_cmd`. `vector_corpus.rs` does not exist there (correctly moved).

**6. vectors/ corpus stayed in md-codec.** `crates/md-codec/tests/vectors/manifest.rs` confirmed present.

**7. Dual smoke.rs setup is clean.** `crates/md-codec/tests/smoke.rs` (library smoke, pure `md_codec::*` API) and `crates/md-cli/tests/smoke.rs` (Phase-1 CLI scaffold smoke, one `cargo_bin("md")` test) are in separate crates with distinct test binary targets. No name collision.

**8. md-codec/Cargo.toml library-only posture intact.** Confirmed no `[[bin]]`, no `[features]`, no CLI optional deps, no `[dev-dependencies]` (all stripped in Phase 2 commit `9e12253`). The 15 moved test files no longer need to compile against md-codec, so the missing dev-dep break from Phase 2's interim state is fully resolved.

**9. md-cli/Cargo.toml has all required dev-dependencies.** `[dev-dependencies]` at `crates/md-cli/Cargo.toml` lines 35-39: `assert_cmd = "2.0"`, `predicates = "3.1"`, `insta = { version = "1.40", features = ["json"] }`, `tempfile = "3.13"`. All dev-deps needed by the 15 moved test files are present from Phase 1.

**10. Phase 2 hotfix disposition.** The hotfix `f9e01ee` committed the `include!` edit that was missing from `9e12253`. The fix is correct and sufficient. The feedback memory ("Phase 2 review mis-confirmed a working-tree edit as staged") is the right long-term remediation. No additional action is required.

## Architect's open questions

None.
