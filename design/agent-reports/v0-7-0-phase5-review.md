# v0.7.0 Phase 5 review

**Status:** DONE_WITH_CONCERNS
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-29
**Commit reviewed:** `d2695d6` on `feature/v0.7.0-development`
**Files reviewed:**
- `crates/md-codec/Cargo.toml`
- `crates/md-codec/src/policy_compiler.rs`
- `crates/md-codec/src/lib.rs`
- `crates/md-codec/src/bin/md/main.rs`
- `crates/md-codec/tests/cli.rs`
**Role:** reviewer (Phase 5)

## Summary

**1 Important. 2 Nits. No Critical.** The Important finding (combined: missing NUMS-Tap-path test + CLI test gate too loose) is folded inline by the controller before tagging v0.7.0. Plan reviewer #1 Concern 2 is correctly satisfied; round-trip projection is sound; error mapping is defensible.

## Important

### IMP-1. None-branch test coverage gap + CLI test feature gate (Confidence: 80–90)

(a) `tap_pk_with_internal_key_compiles_and_encodes` only exercises `Some(internal)`. There was no test covering `ScriptContext::Tap` with `internal_key = None` — the very NUMS-unspendable path Plan reviewer #1 Concern 2 motivated.

(b) The CLI integration test was gated `#[cfg(feature = "compiler")]`, but `[[bin]] md` requires `cli`. Under `cargo test --no-default-features --features compiler` the test would compile but `cargo_bin("md")` would panic at runtime with a missing-binary error. Correct gate is `#[cfg(all(feature = "compiler", feature = "cli"))]`.

**Fix folded inline by controller:**
- New unit test `tap_pk_with_nums_internal_key_compiles_and_encodes` exercises the `internal_key = None` path end-to-end.
- CLI test gate tightened to `#[cfg(all(feature = "compiler", feature = "cli"))]`.

## Verification of the 6 specific concerns

| # | Concern | Verdict |
|---|---|---|
| 1 | Plan reviewer Concern 2 compliance (`internal_key: Option<DescriptorPublicKey>`) | PASS — exact API match; rustdoc explains None-NUMS semantics |
| 2 | Compile path correctness + None-branch coverage | IMPORTANT (folded — see IMP-1) |
| 3 | Stringify+reparse projection round-trip soundness | PASS — apoelstra/rust-miniscript PR 1 documents the contract; not coincidental |
| 4 | Error mapping | PASS with caveat — `PolicyScopeViolation` rustdoc still says "v0.1 scope"; widened semantic load is documented in `policy_compiler.rs` rustdoc but variant rustdoc could mention compiler use site |
| 5 | CLI argument handling | PASS with N-2 nit |
| 6 | Feature gating correctness | IMPORTANT (folded — see IMP-1) |

## Nits

### N-1. `PolicyScopeViolation` rustdoc still pinned to "v0.1 scope" (Confidence: 80)

`error.rs:180-188` first sentence is "Policy violates the v0.1 implementation scope" — pre-Layer-3-strip language. Phase 5's wrapper now also fires this for "compiler output MD can't encode," widening its load. Recommend a one-line note: "Also returned by `policy_to_bytecode` when the compiler emits a top-level shape MD does not encode."

### N-2. CLI `--context` error message omits `wsh`/`tr` aliases (Confidence: 80)

`main.rs::cmd_from_policy` accepts four forms (`segwitv0`, `wsh`, `tap`, `tr`) but the bail message says "must be one of: segwitv0, tap". User-facing inconsistency.

## FOLLOWUPS

1. **`v07-phase5-tap-none-test`** — RESOLVED inline (NUMS test added).
2. **`v07-phase5-cli-test-gate`** — RESOLVED inline (gate tightened to `all(compiler, cli)`).
3. **`v07-phase5-policyscopeviolation-rustdoc`** (Tier: v0.7.x) — refresh `Error::PolicyScopeViolation` rustdoc to mention `policy_to_bytecode` use site.
4. **`v07-phase5-cli-context-error-msg`** (Tier: v0.7.x) — update `--context` error message to enumerate all four accepted forms.

## Verdict

Phase 5 lands close to spec. After folding IMP-1 inline, **DONE_CLEAN.** The two nits are tracked as v0.7.x defensive cleanups. Controller proceeds to Phase 6 release plumbing.
