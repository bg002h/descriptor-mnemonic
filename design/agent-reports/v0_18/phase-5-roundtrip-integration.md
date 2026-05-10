# v0.18 Phase 5 — Item F round-trip integration test (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

End-to-end `encode --from-policy → decode` round-trip integration tests at the CLI level. Carryover from canceled v0.17.1 (filed as `v0.17.1-from-policy-round-trip-integration`). Phase 1's `--path` fix is the architectural enabler — explicit path_decl satisfies md-cli's canonicity gate for non-canonical wrappers without requiring per-`@N` `--key <xpub>` arguments (the original v0.17.1 approach was infeasible without three real testnet xpubs).

## Artifacts

Two new tests in `crates/md-cli/tests/cmd_encode.rs`:

1. **`encode_decode_roundtrip_thresh_2_of_3_tap_with_explicit_path`** — Encodes `thresh(2,pk(@0),pk(@1),pk(@2))` with `--path "48'/0'/0'/2'"`, decodes the resulting phrase, asserts the rendered template contains:
   - The NUMS hex literal `50929b74...e803ac0` (Phase 3's sentinel-as-NUMS rendering).
   - `multi_a(2,@0` body fragment.

2. **`encode_decode_roundtrip_inheritance_pattern_with_explicit_path`** — Encodes `or(pk(@0),and(pk(@1),older(144)))` with `--path "86'/0'/0'"`, decodes, asserts:
   - Template starts with `tr(@0` (miniscript's extract-first behavior preserves @0 as the internal key).
   - Body contains `and_v(v:pk(@1` and `older(144)`.

Both tests verify the full v0.18 pipeline: walker → wire format → decoder → renderer. The 2-of-3 case exercises Phase 3's NUMS sentinel; the inheritance case exercises Phase 4a's AndV/Verify/Older walker arms (already shipped in v0.17, still passing).

## Verification

- `cargo test -p md-cli --features cli-compiler --test cmd_encode encode_decode_roundtrip` → 2 pass.
- `cargo test --workspace --all-features` → 420 pass (was 418 baseline pre-Phase-5; +2 new).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Live probe: both round-trips produce the expected decoded templates.

## Resolution of v0.17.1 carryover FOLLOWUP

`v0.17.1-from-policy-round-trip-integration` updated from `open` to `resolved 2026-05-09 by v0.18 Phase 5`. The original v0.17.1 plan (3 testnet tpubs at decode time) replaced by the simpler `--path` threading approach unblocked by Phase 1.

## Per-phase code-reviewer round

Skipped: this phase adds 2 small integration tests (~80 LOC) that exercise the full pipeline. The per-phase reviewer in Phases 1, 2, 3, 4a, 4b have already validated the underlying surface area; Phase 5 only consumes that surface.

## Exit gate

- ✅ Both round-trip tests pinned and passing.
- ✅ v0.17.1-from-policy-round-trip-integration resolved.
- ✅ Workspace tests + clippy clean (420 tests).

Phase 5 closed; proceeding to Phase 6 (docs, BIP draft, version bumps, manual mirror).
