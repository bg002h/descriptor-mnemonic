# v0.2 Implementation Plan — Opus 4.7 architectural review

**Status:** APPLIED
**Subject:** `design/IMPLEMENTATION_PLAN_v0.2.md` (initial draft)
**Reviewer:** Opus 4.7 via general-purpose agent
**Verdict:** APPROVE WITH CHANGES
**Applied in commit:** (this commit)
**Role:** plan reviewer

## Summary

Senior-architect-style review of the v0.2 plan draft. Flagged 3 blockers, 6 important findings, 7 nits. All blockers and important findings applied to the plan in this commit; nits addressed as edits where they materially improved the document, otherwise noted but skipped.

## Verbatim review (as returned by the agent)

### Blockers

1. **Phase A bundles three breaking changes that aren't all foundational; one is purely additive and shouldn't gate B–F.** `5e-checksum-correction-fallback` is additive on `DecodedString`, not breaking. Bundling it in Phase A overstates the breaking-change footprint. **Fix:** Move `5e-checksum-correction-fallback` into Phase B; limit Phase A to the two genuinely shape-changing items (`p4-chunking-mode-enum`, `6a-bytecode-roundtrip-path-mismatch`).

2. **Phase A leaves the `6a` design decision unresolved at phase entry — a hard dependency for B's `7-encode-path-override`.** Whether `WalletPolicy` carries `decoded_shared_path` or `from_bytecode` returns a wrapper directly determines `to_bytecode`'s signature and the precedence rule. B cannot start until 6a is decided. **Fix:** Resolve the 6a shape in this plan before Phase A entry; document the precedence: `EncodeOptions.shared_path` > `decoded_shared_path` > BIP 84 fallback.

3. **Phase F sequencing is wrong relative to Phase C.** Phase C ships new error-class behavior (BCH correction); negative vectors that exercise 2/3/4-error inputs belong in F's regenerated negative corpus. F must come after C as well as D and E. **Fix:** State explicitly that F follows C, D, **and** E.

### Important

4. **Cargo.toml `[patch]` removal is the real Phase G blocker, not just an "external" risk.** `Cargo.toml` `[patch]`-redirects miniscript to a local sibling clone. A v0.2.0 tag must NOT ship with that block live. **Fix:** Add to v0.2.0 release gates: workspace `[patch]` block removed; miniscript pin is a published-crate version or upstream SHA. If PR #1 hasn't merged by Phase G entry, define an explicit fallback (vendor or fork-publish).

5. **Missing breaking-change accounting in B and E.** B's `7-encode-path-override` changes `WalletPolicy::to_bytecode` signature (breaking). E's behavior change from `PolicyScopeViolation` reject to accept is a behavioral break for callers relying on the v0.1 error contract. **Fix:** Tag B and E with explicit "breaking" notes; enumerate affected public symbols.

6. **Risk register omits two real showstoppers.** (a) BIP review/community feedback timing on Phases D and E. (b) miniscript fork divergence during v0.2 window. **Fix:** Add both with mitigations (early bitcoin-dev posting after D/E entry, periodic fork-rebase checks).

7. **MSRV / migration-guide / changelog policy unstated.** v0.1 inherits `rust-version = "1.85"`; plan never says whether v0.2 raises it. No `MIGRATION.md` or `CHANGELOG.md` plan despite Phase A breaking changes. **Fix:** Add to Phase G gates: MSRV unchanged or bumped (with rationale); CHANGELOG.md updated; MIGRATION.md present documenting Phase A breaking changes.

8. **Spec-impact tag for Phase C is understated.** "Clarification only" is wrong; BIP §"Test vectors" needs new 2/3/4-error positive correction vectors and a SHOULD-clause naming the canonical decoder algorithm (BM/Forney) for cross-implementation `Correction` consistency. **Fix:** Re-tag C as `minor`; enumerate BIP test-vector additions.

9. **Header bit 2 ambiguity in Phase E.** Plan should specify the encoder default (off unless caller opts in) and decoder accept-both behavior (`0x00` and `0x04`). **Fix:** Add to Phase E: "Encoder default is bit 2 = 0 (no fingerprints) unless `EncodeOptions::fingerprints` is `Some(_)`. Decoder accepts both 0x00 and 0x04."

### Nits

- **Line 7** "second component is the breaking-change axis pre-1.0" — Cargo's actual rule: leftmost non-zero component is the breaking axis. Tighten.
- **Line 23** call out the schema version bump explicitly: `1 → 2`.
- **Line 88** pick one number for the v0.2 gate (say 90%); stop discussing v0.1 history inline.
- **Line 117** "D and E are the tallest" — say *why*, not just relative ranking.
- **Phase G's "Public API audit"** — name the tool (`cargo public-api` / `cargo semver-checks`) so the gate is mechanically checkable.
- **Line 137** references `PHASE_v02_<X>_DECISIONS.md` while v0.1 used `PHASE_<N>_DECISIONS.md`. Pick one.
- **"Forward look" (line 146)** repeats `8-negative-fixture-dynamic-generation` as both v0.2 in-scope and a v0.3 surface. Disambiguate.

### Top-line verdict

**APPROVE WITH CHANGES** — plan is sound and substantially lighter than v0.1, but Blocker #2 (6a decision blocking B) and Blocker #3 (F-after-C) are sequencing bugs that will cause real rework if not fixed before Phase A starts. Blocker #1 is a smaller scope-correctness issue. Important #4 (Cargo `[patch]` as a release gate) must be in writing before tag.

## Disposition

| Finding | Status | Notes |
|---|---|---|
| Blocker 1 (move 5e to Phase B) | applied | Scope table updated; Phase A intro changed from "three" to "two"; 5e moved to Phase B with additive notation. |
| Blocker 2 (resolve 6a in plan; document precedence) | applied | Phase A entry now resolves 6a shape (newtype-field approach, returned-wrapper rejected) and documents the 4-tier shared-path precedence rule that Phase B builds on. |
| Blocker 3 (F after C, D, E) | applied | Sequencing dependencies section updated. |
| Important 4 (`[patch]` removal as release gate) | applied | Phase G gates expanded; explicit fallback options listed. |
| Important 5 (flag B and E breaking changes) | applied | Scope table tags updated; Phase B "Breaking surface" paragraph added; Phase E "Behavioral break" paragraph added. |
| Important 6 (BIP-review + miniscript-fork-divergence risks) | applied | Risk register expanded. |
| Important 7 (MSRV / CHANGELOG / MIGRATION policy) | applied | Phase G gates expanded with all three. |
| Important 8 (Phase C → minor spec impact) | applied | Scope table tag updated; Phase C body now enumerates BIP test-vector additions and the SHOULD-clause. |
| Important 9 (Phase E header bit 2 default + decoder accept) | applied | "Encoder default" and "Decoder behavior" paragraphs added under Phase E. |
| Nit 1 (SemVer language) | applied | "Leftmost non-zero version component" wording. |
| Nit 2 (schema 1 → 2) | applied | Scope table row for `8-negative-fixture-dynamic-generation` now says "bumps test-vector schema 1 → 2". |
| Nit 3 (drop v0.1 coverage history from gate) | applied | Phase G gate is now plainly "Coverage ≥ 90 % library line". |
| Nit 4 ("D and E are the tallest" — say why) | applied | Sequencing dependencies section now explains: "spec-substantial and D additionally has an external miniscript-fork dependency". |
| Nit 5 (name the API-audit tool) | applied | Phase G gate names `cargo public-api` and `cargo semver-checks`. |
| Nit 6 (decision-log naming) | applied | Workflow section pinned to `PHASE_v0_2_<X>_DECISIONS.md` consistent with v0.1's `PHASE_<N>_DECISIONS.md` template. |
| Nit 7 (Forward look disambiguation) | applied | "Forward look" entry now distinguishes the *item* (v0.2) from *schema-3 ideas surfaced during F* (v0.3). |

All findings closed in this same commit; no carry-forward FOLLOWUPS entries needed.
