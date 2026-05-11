# Phase E — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `67c2b30`; not yet committed.

**Scope:** md-codec v0.30 Cycle 3 Phase E — walker emits BARE `Tag::PkK` / `Tag::PkH` at c:-positions (unconditional per SPEC §5.1 Q12); renderer's existing PkK/PkH arms already emit `pk(K)`/`pkh(K)` shorthand for bare-shape AST, so no behavioral renderer change required.

**Files reviewed:**
- `crates/md-cli/src/parse/template.rs` (walker — main change at `Terminal::Check` arm)
- `crates/md-cli/src/format/text.rs` (renderer — doc-comments only)
- `crates/md-cli/tests/template_roundtrip.rs` (1 updated assertion + 1 new test)
- 2 new unit tests in `parse/template.rs`

---

## Critical (block ship)

None.

## Important (must fix before ship)

### I-1 — `tap_context: bool` parameter on `walk_miniscript_node` is now fully dead

- **Where:** `crates/md-cli/src/parse/template.rs:573–576` (function signature); 15+ recursive call-sites; `let _ = tap_context;` suppressor at line 801 (the fallthrough catch-all arm).
- **What:** Phase E made the `Terminal::Check(PkK|PkH)` collapse unconditional, so the only branch on `tap_context` is gone. The parameter threads through every recursive call unchanged with no effect. Phase F's NUMS-flag changes don't need `tap_context` (they operate on `Body::Tr` struct fields). Leaving the parameter is a misleading API for the Phase F implementer.
- **Fix:** drop `tap_context: bool` parameter from `walk_miniscript_node`; drop the `, /*tap=*/ false` / `, /*tap=*/ true` / `, tap_context` from every call site; remove the `let _ = tap_context;` suppressor. Pure dead-code removal.
- **Decision:** apply inline in Phase E commit before ship.

## Low (file as FOLLOWUP — ship can proceed)

### L-1 — New unit tests inspect walker AST only (pre-encode)

- **Where:** `crates/md-cli/src/parse/template.rs` — `pkh_key_leaf_bare_on_wire` (`:1626`) and `tr_tap_leaf_bare_pk_on_wire` (`:1645`).
- **What:** Both tests call `walk_root` and assert the returned `Node` tree. They do NOT encode that tree to wire and decode back. The encode+decode round-trip is only covered by the integration test `tr_tap_leaf_bare_pk_round_trip` (and the pre-existing `wsh_pkh_shorthand_collapse_round_trips`).
- **Decision:** Low — integration tests cover end-to-end. File FOLLOWUP if/when post-Phase-F wire-shape scrutiny benefits from a unit-level encode+decode pin.

### L-2 — Sub-plan Phase E description narrower than SPEC

- **Where:** `/home/bcg/.claude/plans/noble-purring-pizza.md` Phase E "Goal" line says "At Tr tap-leaf c:-sites" — but SPEC §5.1 + the implementation are unconditional (tap + segwitv0). The sub-plan is not in the repo, so deferring to a future plan revision is acceptable.
- **Decision:** Low — plan-only drift; no repo file changes needed for Phase E ship.

## Nit (optional polish — not blocking)

### N-1 — `render_wrapper_chain` doc-comment cites Q12 ambiguously

- **Where:** `crates/md-cli/src/format/text.rs:297–303`.
- **What:** Doc mentions "v0.30 SPEC §5.1 (Q12 — walker normalization)" and then "unreachable on v0.30-produced wires". The `(Q12)` parenthetical implies the arm implements Q12 — which it doesn't (the walker does; this arm is defensive).
- **Decision:** Defer to opportunistic doc cleanup.

### N-2 — `wsh_pkh_shorthand_collapse_round_trips` 14-line doc-comment

- **Where:** `crates/md-cli/tests/template_roundtrip.rs:224–237`.
- **What:** Long prose vs the test's one round-trip body. Per memory "Write terse code in this repo".
- **Decision:** Defer to opportunistic cleanup.

---

## Correctness checks (all passed)

1. **Walker collapse correctness.** Lines 605–630 (`Terminal::Check` arm) unconditionally produces bare `Tag::PkK`/`Tag::PkH` for key terminals; emits `Tag::Check` wrapping `Body::Children(vec![child])` for non-key children. SPEC §5.1-correct.

2. **Renderer shorthand.** `render_node` PkK/PkH arms (`text.rs:60–81`) emit `pk(K)`/`pkh(K)` from bare `Body::KeyArg` directly; no context signal needed. The pre-existing arms already produce the desired output for Phase E's new wire shape.

3. **Defensive `render_wrapper_chain` retention.** The `Check(PkK)`/`Check(PkH)` collapse arm at `text.rs:357–381` is retained to handle foreign/legacy/codec-test wires; codec-layer `wrapper_chain_v_c_pk_round_trip` test still round-trips `Tag::Check`-wrapped key terminals at the AST level. Correct: the codec is shape-agnostic at the wire level.

4. **Q12 regression guard.** `wsh_pkh_shorthand_collapse_round_trips` (`template_roundtrip.rs:224+`) body unchanged; still exercises encode → wire → decode → render round-trip stability. Doc-comment update only.

5. **No Thresh/OrI/AndV/Swap walker drift.** All non-Check arms recurse via `walk_miniscript_node(...)` with identity-preserved semantics. Phase E changed only the Check arm.

6. **Phase F layering.** Phase F's NUMS-flag changes touch `Body::Tr` struct fields; Phase E's walker reshape doesn't conflict. Clean layering on top of Phase E.

7. **Stop condition met:**
   - `cargo test -p md-codec --lib`: 208 / 0 / 11 ✓
   - `cargo test -p md-cli --bin md`: 94 / 0 / 0 ✓
   - `cargo test -p md-cli --test template_roundtrip`: 8 / 0 ✓
   - `cargo test -p md-cli --test help_examples`: 2 failed (RED entering Phase E; deferred to Phase H per `v0.30-phase-c-help-examples-md1-strings-drift`)
   - `cargo clippy --workspace -- -D warnings`: clean

## Verdict

**Iterate** (1 item): apply I-1 (drop dead `tap_context` parameter). Then ship.

No r2 needed: I-1 fix is pure dead-code removal with no behavioral change.
