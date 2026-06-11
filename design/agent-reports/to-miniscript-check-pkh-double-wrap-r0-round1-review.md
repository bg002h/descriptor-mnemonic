# R0 Review — md-codec Check double-wrap (PART 2) — ROUND 1

**Source SHA:** `b93674f` (descriptor-mnemonic). **Verdict: 🟢 GREEN — 0 Critical / 0 Important / 3 Minor.**

## A1 correctness + no-regression INDEPENDENTLY VERIFIED
- **Double-Check mechanism confirmed** (`to_miniscript.rs:290-303` PkK/PkH arms re-apply `Check`; `:304-307` Check arm wraps another; type check fires at `from_ast` `:461`). miniscript typing confirmed in pinned 13.0.0: `pk_k`/`pk_h` = `Base::K`; `cast_check` K→B else `ChildBase1` (`correctness.rs:136-148,200-210`). `Check(Check(PkH))` = `c:` over B = guaranteed error; `Check(PkH)` once = the only valid form; both are `c:pk_h` → collapse loses nothing.
- **Crux 1 (exactness):** A1's `return node_to_miniscript(&children[0], keys)` re-enters the bare arm → byte-identical `Check(PkK/PkH)`. `Ctx` threaded generically (Segwitv0/Legacy/Tap).
- **Crux 2 (no regression):** `Tag::Check(bare PkK/PkH)` is ALWAYS an error today (all `?`-propagate; no error-tolerant fallback). NO md-codec test/vector exercises `Tag::Check` through the render path (only the wire-level `tree.rs:357-374` round-trip, untouched). Strictly error→success.
- **Crux 3 (ill-typed still errors):** a `Tag::Check` child is `Body::Children` not `Body::KeyArg` → guard misses → double-wraps → errors. Confirmed. Guard can't mis-fire on `Check(Wpkh/Pkh)` or malformed `(PkK,Body::Children)`.
- **Producers confirmed:** toolkit walker `parse_descriptor.rs:601-624` emits `Tag::Check(Tag::PkH/PkK)` non-tap (tap_context-gated); md-cli walker normalizes (`template.rs:602-628`); pre-v0.30 history `template.rs:1637-1638`. Toolkit refusal live at `restore.rs:869-875` → unblock is genuinely **zero-code** (the hint stops firing once render succeeds).
- **Precedent:** md-cli `format/text.rs:360-385` trailing-`c` collapse — A1 mirrors it.
- **Lockstep:** md-cli `Cargo.toml:28` `=0.35.0` → `=0.35.1` mandatory same-commit; md-cli has ZERO `to_miniscript` usage → no code change, version unchanged. Toolkit `md-codec = "0.35"` → `cargo update -p md-codec`. No hardcoded pins in CI/README.

## Critical / Important
**None.** Hunted the wrong-descriptor mode specifically: the only behavior change is gated on `(Tag::Check, [bare PkK/PkH KeyArg])`; every real producer means `c:pk_k`/`c:pk_h`; collapsed render is exactly that. Shape C still ERRORS post-A1 (never silently mis-renders).

## Minor (fold-at-will during TDD)
1. Pin the literal expected `wsh(pkh(<key>))` string in `wsh_check_pkh_renders_same_as_bare_pkh` (not equality-only; pattern at `tests/address_derivation.rs:700-722`).
2. Add a shape-C still-errors pin cell (`Check(OrI(PkK,PkK))` errors post-A1) citing the follow-up slug.
3. FOLLOWUPS path = `descriptor-mnemonic/design/FOLLOWUPS.md` (exists there, not repo root).

## A1 vs A2
**Ship A1; defer A2.** Flagship `wsh(andor(pkh(@0),after(N),or_i(and_v(v:pkh(@1),older(M)),and_v(v:pkh(@2),older(K)))))` FULLY unblocked by A1: all three keys are direct `Check(PkH)` (andor child 1; the two `v:pkh` = `Verify(Check(PkH))` → Check child is bare PkH → collapses → `Verify(B)` valid). No key sits in `or_i`-under-`c:` in any toolkit bundle shape. Shape C stays an error (funds-safe). A2's `want_k` touches 10+ arms — disproportionate for a PATCH. File shape C with Minor #2 as its RED cell.

## Scope
PATCH 0.35.1 (renderer-tolerance, error→success, no wire/API change). md-cli `=0.35.1` same-commit (zero md-cli code). A1 over A2 (shape C deferred). Publish-gated (irreversible — user confirm before `cargo publish`; toolkit then `cargo update -p md-codec` + new cells only). **GREEN — implementation may begin.**
