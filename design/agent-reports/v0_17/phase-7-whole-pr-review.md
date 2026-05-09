# v0.17 Phase 7 — Whole-PR architect review + final cleanup (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.17-tap-multi-leaf-policy`

## Scope

Final integration review across all 7 phase commits before tag. Per-phase reviewer rounds had already converged 0C/0I; this pass looks for cross-phase concerns that only surface at the cycle level.

## Architect findings

`feature-dev:code-architect` dispatched on the full branch state.

- **C: 0**
- **I1** — `design/FOLLOWUPS.md` entry `v0.17-md-cli-tap-multi-leaf-policy-compile` was still marked `Status: open` and contained pre-implementation prose (referenced `--internal-key` flag name, claimed `policy_to_bytecode` substrate exists in md-codec, described the deliverable as `{a,b}` brace syntax which wasn't shipped). **Fixed inline** — entry now marked `Status: resolved 5d2de0f`, prose rewritten to describe what actually shipped (Tag::TrUnspendable wire-format addition, `--unspendable-key` flag name, AndV/Older/Verify walker arms), and explicit "did NOT ship" subsection records the deferred `{a,b}` brace syntax.
- **L1** (informational, not blocking) — `--unspendable-key` accepted-forms documented at three different levels of detail across surfaces: compile.rs doc-comment lists three forms (xpub, NUMS, None=auto-NUMS); README and manual mirror only describe two (NUMS, auto-NUMS). The xpub-as-fallback form is an advanced escape hatch. Architect noted this is not blocking and could be filed as a v0.17.1 followup if the xpub form is intended user-visible. **Decision: leave as-is for v0.17.** README/manual targeting common-case clarity is the right cut; advanced users find the third form via compile.rs doc-comment or `--help`.
- **L2** (informational) — Test count over-delivery: SPEC said +12 target; actual delivery is +21 net (reviewer-driven additions across phases). No action.

## Cross-phase consistency checks (all passed per architect)

- Canonicalization invariant ("MUST emit Tag::TrUnspendable iff NUMS internal key") consistent across md-codec CHANGELOG, MIGRATION.md, parse/template.rs doc-comment, format/text.rs comment.
- API surface stability: all three `compile_policy_to_template` call sites (main.rs, cmd/compile.rs, compile.rs tests) updated to three-arg form. No stale two-arg call sites.
- Wire-format compatibility: Tag::TrUnspendable's extension sub-code 0x05 cannot appear in pre-v0.17 payloads; "byte-identical decode" claim in CHANGELOG/MIGRATION is structurally sound.
- Manual mirror PR (toolkit #10) content matches md-cli/README.md verbatim; `manual-cli-surface-mirror` invariant satisfied.

## Final verification

- `cargo test --workspace --all-features` → **395 tests pass, 0 failures.**
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- Full git log verified — 7 phase commits + this Phase 7 cleanup commit, all green.

## Followups filed during this cycle

- `v0.17.1-from-policy-round-trip-integration` (filed Phase 5) — round-trip test deferred to v0.17.1 due to canonicity-gate constraint unrelated to v0.17.
- (No new followups from Phase 7.)

## Followups closed by this cycle

- `v0.17-md-cli-tap-multi-leaf-policy-compile` — resolved by commits 49739e5..5d2de0f. Entry updated with the resolved status + corrected prose.

## Exit gate

- ✅ Architect whole-PR review converged at 0C/0I after the FOLLOWUPS fix.
- ✅ `cargo test --workspace --all-features` clean (395 tests).
- ✅ `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- ✅ FOLLOWUPS entry marked resolved with v0.17 commit reference.
- ✅ Toolkit companion PR open and lint-clean (PR #10).

Phase 7 closed; proceeding to Phase 8 (release tagging + lockstep merge).
