# v0.17 Phase 4 — compile.rs rewrite + `--unspendable-key` flag (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

User-facing feature work: `md compile` and `md encode --from-policy` now handle any miniscript-compileable Taproot policy via `Policy::compile_tr` with auto-NUMS fallback. The bare-pk gate is gone. New `--unspendable-key` flag exposes caller override.

## Artifacts

### compile.rs

- Function signature: `compile_policy_to_template(expr, ctx, unspendable_key: Option<&str>) -> Result<String, CompileError>`.
- Doc-comment expanded (paragraph form to satisfy clippy doc-list-indentation lint) describing the three forms: `Some("xpub")`, `Some("NUMS-hex")`, `None` (auto-NUMS).
- Tap arm: `Policy::compile_tr(unspendable_key.map(String::from).or_else(|| Some(NUMS_H_POINT_X_ONLY_HEX.to_string())))`. Auto-NUMS default; spike-verified strictly additive.
- Checksum strip: `desc.to_string().split_once('#').map(|(t, _)| t.to_string())`.
- SegwitV0 arm: `debug_assert!(unspendable_key.is_none())`; CLI rejects upstream.
- 7 new unit tests pinning spike-verified shapes (`compile_pk_tap_keypath_only`, `compile_or_two_keys_tap`, `compile_or_pk_and_pk_older_tap`, `compile_thresh_2_of_3_tap_auto_nums`, `compile_and_pk_pk_tap_auto_nums`, `compile_pk_tap_explicit_nums_extract_still_wins`, `compile_strips_descriptor_checksum`).

### main.rs (flag plumbing)

- `Command::Compile { expr, context, unspendable_key, json }` — new `unspendable_key: Option<String>` field with `#[arg(long, value_name = "KEY")]`.
- `Command::Encode { ... unspendable_key, ... }` — same field.
- Compile dispatch: rejects empty (`Some("")`) and segwitv0+flag combinations with clear errors.
- Encode dispatch: rejects empty, segwitv0+flag, and `unspendable_key` without `--from-policy` (template-only mode has no compile step).

### cmd/compile.rs

- Wrapper signature: `run(expr, ctx_str, unspendable_key: Option<&str>, json)`. Threads through.

### cmd_compile.rs (CLI integration tests)

- `compile_thresh_2_of_3_tap_auto_nums` — pinned `tr(<NUMS>,multi_a(2,@0,@1,@2))`.
- `compile_or_pk_and_pk_older_tap` — pinned `tr(@0,and_v(v:pk(@1),older(144)))`.
- `compile_segwitv0_rejects_unspendable_key` — flag rejection.
- `compile_pk_tap_with_explicit_nums_unspendable_key` — explicit NUMS with extract-first preserved.
- `compile_empty_unspendable_key_rejected` (reviewer-driven) — empty-string flag value rejected with actionable error.

## Verification

- `cargo test --workspace --all-features` → all pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- End-to-end smoke (live cargo run):
  - `md compile 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap` → `tr(50929b74...,multi_a(2,@0,@1,@2))` ✅
  - `md compile 'or(pk(@0),and(pk(@1),older(144)))' --context tap` → `tr(@0,and_v(v:pk(@1),older(144)))` ✅
  - `md compile 'pk(@0)' --context tap` → `tr(@0)` (no checksum, no v0.15 wording) ✅
  - `md compile 'pk(@0)' --context segwitv0 --unspendable-key <NUMS>` → "--unspendable-key is only valid for --context tap (segwitv0 has no internal key)" ✅
  - `md encode --from-policy 'thresh(2,pk(@0),pk(@1),pk(@2))' --context tap` → `md1qzq0j6qgjs54gk3aayrxh8yz7w` ✅

## Per-phase code-reviewer round

`feature-dev:code-reviewer` dispatched. Findings:

- **C: 0**
- **I1** — Empty-string `--unspendable-key` silently flowed into miniscript as `Some("")` instead of triggering auto-NUMS or a clear error. **Fixed inline** — added dispatch-level guard at both Compile and Encode call sites with actionable error message ("--unspendable-key must not be empty (omit the flag for auto-NUMS default)"). Companion test `compile_empty_unspendable_key_rejected` pins the rejection.
- **I2** — Encode segwitv0 rejection ordering (after context parsing). Reviewer confirmed not a bug; ordering acceptable.
- Reviewer confirmed: API shape `Option<&str>` correct; doc-comment clear; checksum-strip fallback handles missing-checksum case correctly; rejections of `--unspendable-key` without `--from-policy` are correct semantics.

Net: 0C/0I after fix.

## Exit gate

- ✅ Bare-pk gate dropped; `compile_tr` API used with auto-NUMS default.
- ✅ Checksum stripped from rendered output.
- ✅ `--unspendable-key` flag wired through Compile + Encode subcommands.
- ✅ Segwitv0 + empty-string + missing-`--from-policy` all rejected with clear errors.
- ✅ All 5 SPEC test-matrix cases pinned at unit level + 4 at CLI integration level.
- ✅ End-to-end compile + encode --from-policy verified live.
- ✅ Per-phase reviewer 0C/0I.

Phase 4 closed; proceeding to Phase 5 (end-to-end integration tests for `encode --from-policy` round-trip).
