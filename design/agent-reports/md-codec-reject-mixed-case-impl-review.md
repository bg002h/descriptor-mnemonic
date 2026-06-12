# Impl Review — md-codec reject mixed-case md1 — Round 1 (+ fold)
Reviewer: Fable 5, 2026-06-12. Verified against the descriptor-mnemonic working tree.

## Verdict: GREEN (0C/0I) after the I1 fold.

Round 1 was RED (0C/1I): the CODE was correct, minimal, and matched every R0-resolved design point (both injection sites, identical `"mixes upper and lower case"` substring, `Codex32DecodeError` reuse / no new enum variant, first-statement placement, per-chunk semantics, all-upper preserved, 372-test suite + clippy + fmt clean, RED→GREEN proven via a two-file stash). The single Important was a TEST gap.

## I1 (folded) — the second injection site was suite-unpinned
Round-1 finding: the plan's correction-path cell (b) "mixed + 1 corrupted symbol → Err" was missing; the implemented (a) cell uses a residue==0 fixture that the FIRST site (`unwrap_string`, via the forwarded original string) catches — so reverting ONLY `chunk.rs` left all 7 cells green, leaving `parse_chunk_symbols`'s check unpinned (a regression could silently reintroduce the R0-I3 inconsistency: 0-error mixed rejects, 1-error mixed gets corrected+accepted).

**Fold:** added `decode_with_correction_rejects_mixed_with_symbol_error` — corrupts one data char of chunk 0 to a different codex32 char (a correctable 1-symbol error → the correction BRANCH, not the pass-through) + uppercases the HRP (mixed). VERIFIED: 8/8 cells green on the working tree; with ONLY `crates/md-codec/src/chunk.rs` stashed the new cell goes **RED** (proving it pins the second site); restored → green. fmt clean.

## Checks (round 1)
- Both injection sites correct (`is_mixed_case` ignores `-`/ws/digits/non-ASCII; first statement of `unwrap_string` over original `s`; first statement of `parse_chunk_symbols`; identical needle; `Codex32DecodeError` reused — toolkit exhaustive friendly-match safe).
- Semantics: cross-chunk heterogeneity ACCEPTED (legal per-chunk); internally-mixed REJECTS (reassemble + correction); all-upper round-trips both paths.
- No regression: full `cargo test -p md-codec` 372 passed/0 failed; clippy `-D warnings` clean; fmt clean; md-cli builds against the unbumped path-dep. No md-codec/md-cli test/golden/vector feeds mixed-case md1.
- Toolkit-tail heads-up (R0-I2): `mnemonic-toolkit/tests/cli_hrp_case_insensitive.rs::inspect_mixed_case_md1_accepted_characterization` asserts `.code(0)` → the 0.35.3 pin bump MUST invert it.
- Scope: diff = codex32.rs (+helper +check) + chunk.rs (+check) + the new test file. No stray churn.

## Minor
- M1: `unwrap_string`'s mixed-case error via `reassemble` carries no chunk index (toolkit `parse_md_chunk_index` falls back to 0) — matches every pre-existing `unwrap_string`-via-`reassemble` error; pre-existing pattern, no action.

Cleared to release-prep (0.35.3). NO new src change.
