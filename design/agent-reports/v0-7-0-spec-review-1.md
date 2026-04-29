# v0.7.0 spec review #1 — pre-implementation-plan

**Status:** DONE_WITH_CONCERNS
**Commit:** 44c1a19 (on `feature/v0.7.0-development`)
**File(s):**
- `design/SPEC_v0_7_0.md`
- `design/MD_SCOPE_DECISION_2026-04-28.md`
- `design/FOLLOWUPS.md`
- `Cargo.toml` (workspace)
- `crates/md-codec/Cargo.toml`
- `crates/md-codec/src/bytecode/encode.rs`
**Role:** reviewer (spec)

## Summary

Spec is well-scoped and the four-track bundling reads coherently. Found one critical issue (acceptance criterion drift), one important contradiction (operator-naming inconsistency between §4.2 prose and the example list), one important Cargo-feature plumbing snag (§5.1 has two snippets where one is broken — needs deletion), and concrete recommendations on all six §9 open questions plus the 4.4-Option-A architecture choice. None are show-stoppers — the implementation plan can be drafted once §1.1, §4.2, §4.4, §5.1, and §8 are touched up.

## Critical

**C-1. §8 acceptance criterion #3 is too tight as written.** "`cargo test --workspace` passes 100%" papers over a Track A subtlety: of the 432 tests, 38 fail today on v0.5 byte literals and the rest pass. There's no language acknowledging the inverse risk: a test that passes today *despite* having stale v0.6-disagreeing literals (self-cancelling), or a test that becomes newly-fragile under the symbolic refactor.

**Fix:** strengthen criterion #3 with "no NEW failures introduced relative to the pre-rebaseline baseline; every test fixed in Track A documented in the agent report with old-bytes / new-bytes / file:line."

## Important

**I-1. §4.2 contradicts itself on operator naming.** Doc-comment says "BIP 388 / BIP 379 source-form spelling" but the example uses `pk_k` (desugared AST name). The validator walks the **AST**, not the source string — it sees `Terminal::PkK`, etc. So the allowlist must be desugared-AST names, not BIP 388 source names. The doc comment is wrong; the example list is right.

**Fix:** change §4.2 doc-comment to "rust-miniscript desugared AST node names (matching `Terminal::PkK` → `pk_k`, `Terminal::Check` → `c:`, `Terminal::Verify` → `v:`, etc.)." Also call out that admitting `pk(...)` requires both `"c:"` and `"pk_k"` in the allowlist. Plus: add explicit pointer to the existing `tag_to_bip388_name` adapter as the single-source-of-truth naming hook.

**I-2. §5.1 "two attempts" — delete the broken first snippet.** First snippet has `optional = true` on miniscript dep; this would break the build. Second snippet is correct (`compiler = ["miniscript/compiler"]` passthrough on a non-optional dep).

**Fix:** delete the first snippet entirely.

**I-3. §4.4 architectural choice (Option A vs B) needs trade-off paragraph.** Spec selects Option A (refactor `validate_tap_leaf_subset` to take allowlist) but doesn't document the trade-off. Plus Phase 3 should explicitly preserve the existing `validate_tap_leaf_subset` signature as a back-compat shim that calls into `_with_allowlist` with the historical hardcoded Coldcard list.

**Fix:** add 3-line trade-off paragraph at end of §4.4 articulating coupling (Option A wins on AST-walk-locality grounds; coupling concern is mild because the two crates co-release). Phase 3 in §7 should explicitly say "preserve `validate_tap_leaf_subset` signature and behavior."

**I-4. §5.2 runtime-enum-over-generic-monomorphize pattern.** The runtime `ScriptContext` enum dispatches to a generic `Concrete<...>::Policy::compile::<Ctx>()` call where `Ctx` is a *type* parameter. Implementation requires a `match` that monomorphizes both branches. API shape is fine; spec should acknowledge the pattern is intentional.

**Fix:** one sentence in §5.2: "the runtime-enum-over-generic-monomorphize pattern is intentional; both branches pull through the existing per-context `EncodeTemplate` impls already in encode.rs."

**I-5. §4.5 md-signer-compat `version = "0.1.0"` — keep as written.** Independent versioning is correct for a brand-new crate. The "they release together" mental model is captured by workspace tooling, not version-string parity.

**I-6. `&'static [&'static str]` vs typed enum — keep strings.** Strings win for vendor-doc-fidelity and future-proofing. Add a unit test that asserts every entry in `COLDCARD_TAP.allowed_operators` and `LEDGER_TAP.allowed_operators` is a value the validator's naming hook can actually emit (catches typos).

**Fix:** add to §4.6 acceptance: "unit test verifies every string in `COLDCARD_TAP.allowed_operators` and `LEDGER_TAP.allowed_operators` is recognized by the validator's operator-naming hook (catches typos)."

## Open question recommendations (§9)

1. **Q1 — `validate_tap_leaf_subset_with_allowlist` visibility:** `pub fn` (as spec leans). md-signer-compat is a sibling crate; `pub(crate)` doesn't reach across crates.

2. **Q2 — `&'static [&'static str]` vs typed enum:** strings, per I-6.

3. **Q3 — `ScriptContext` placement:** top-level `pub use ScriptContext` (as §5.2 has it), gated behind `#[cfg(feature = "compiler")]`.

4. **Q4 — `cli-compiler` feature naming:** `cli-compiler` is fine as written.

5. **Q5 — `md validate --signer ...` CLI:** defer to v0.7.x patch. Add FOLLOWUPS entry post-ship (`v07-cli-validate-signer-subset`).

6. **Q6 — CHANGELOG `[Unreleased]` discipline:** consolidated entry at release time only. Matches v0.6 practice.

## Other observations

**O-1. `GENERATOR_FAMILY` token roll without corpus content additions.** Implementation plan Phase 6 should explicitly verify by `gen_vectors --verify` against v0.6 corpus *before* rolling the family token.

**O-2. Phase 3 refactor size.** `validate_tap_leaf_terminal` keeps its current recursive shape; only the leaf-allowed-vs-rejected check changes from "hardcoded match" to "allowlist contains?" lookup. Phase 3 should be ~30-50 lines diff, not a rewrite.

**O-3. Wrapper colon convention.** Spec is consistent with codebase (`Terminal::Check` → `"c:"`, etc.). Good.

**O-4. `or_c` hand-AST round-trip is an open policy question.** v0.6 strip-Layer-3 means encode admits `Terminal::OrC`, but the decoder reconstructs miniscript via parser-equivalent typing rules and may reject an unwrapped `or_c` at the top level of a tap leaf (V-type at top is invalid). The hand-AST test in §3.1 demonstrates an *encoder* path; the round-trip back through the decoder needs explicit policy.

**Fix:** add to §3.1: "Decoder round-trip behavior for unwrapped V-type `or_c` is intentionally an open question — implementation plan resolves it (either wrap-test-only-checks-encode-bytes, or wrap or_c under a `v:` so the resulting fragment is well-typed at the top)."

## Follow-up items (for FOLLOWUPS.md)

- `v07-cli-validate-signer-subset` (Q5 deferral): post-ship CLI track for `md validate --signer <name> <bytecode>`.

## Status: DONE_WITH_CONCERNS

Implementation plan can be drafted against the touched-up spec without further spec rev.
