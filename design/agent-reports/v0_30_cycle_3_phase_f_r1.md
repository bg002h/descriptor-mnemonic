# Phase F — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `2e6918b`; not yet committed.

**Scope:** md-codec v0.30 Cycle 3 Phase F — replace `Body::Tr` `key_index = n` NUMS sentinel with explicit `is_nums: bool` flag; switch `key_index_width` from `⌈log₂(n+1)⌉` to `⌈log₂(n)⌉`; add `NUMSSentinelConflict` variant; lift 5 ignored tree.rs tests + 1 smoke pin.

**Files reviewed:** `crates/md-codec/src/{tree, encode, decode, validate, canonicalize, error}.rs`; `crates/md-cli/src/{parse/template, format/{text, json}}.rs`; `crates/md-codec/tests/{smoke, wallet_policy, address_derivation}.rs`; `design/FOLLOWUPS.md`; `design/SPEC_v0_30_wire_format.md` §7 + §11.

---

## Critical (block ship)

None.

## Important (must fix before ship)

### I-1 — `NUMSSentinelConflict` declared but never emitted (FIXED INLINE)

- **Where:** `crates/md-codec/src/canonicalize.rs:267-268`, `crates/md-codec/src/validate.rs:93-97`.
- **What:** Phase F added the variant to `error.rs` but left two `!is_nums && key_index >= n` check sites still raising `Error::PlaceholderIndexOutOfRange`. SPEC §11 line 339 documents:
  > `NUMSSentinelConflict | Inside tr() body: is_nums=0 with key_index ≥ n`
- The variant was dead-code as initially implemented, contradicting the sub-plan's stated reason for adding it in Phase F ("otherwise F can't validate").
- **Fix (applied):** Both sites now raise `Error::NUMSSentinelConflict`. Body::KeyArg's bounds check (validate.rs:45-51) stays on `PlaceholderIndexOutOfRange` — that's not "inside tr() body" per SPEC §11. Test `placeholder_usage_rejects_out_of_range_in_tr_key_index` updated to assert the new variant.

## Low (file as FOLLOWUP)

### L-1 — `tr_bip86_no_tree` test name is technically correct but slightly misleading

- **Where:** `tree.rs:444`.
- **What:** Test uses `is_nums: false, key_index: 0` — actually BIP-86-compatible at n=1 (key index 0), so name fits. Comment "synthetic width=0" is accurate. No action.

### L-2 — Sub-plan said `tr_nums_n_1_bare_round_trip` pins 7 bits; implementation correctly pins 8

- **Where:** `tree.rs:696-697`.
- **What:** SPEC §7 wire shape always emits `has_tree(1)`. Total: `Tag::Tr(6) + is_nums(1) + has_tree(1) = 8`. Sub-plan said 7 (omitted `has_tree`). Implementation is SPEC-correct.

### L-3 — `tap_tree_two_leaf_round_trip` (still `#[ignore]`) comment retains `Tag::Tr (5)` 5-bit prefix

- **Where:** `tree.rs:800-823`.
- **What:** Pre-Phase-A bit-width in an ignored test's comment. Phase H will fix as part of corpus regen.

## Nit (optional polish)

### N-1 — `#[error("NUMS sentinel conflict")]` is appropriately terse for a stub doc

- **Where:** `error.rs:344-345`.
- **What:** Comment "Phase G finalizes" is well-targeted. No change needed.

### N-2 — Updated FOLLOWUPS `Tier:` line could be tightened

- **Where:** `design/FOLLOWUPS.md:493`.
- **What:** "active; lift gated by Phase F/H" → "active; lift gated by Phase H" (Phase F's lifts already shipped in this commit). Minor.

---

## Correctness checks (all passed)

1. **kiw formula (`encode.rs:42`):** `(32 - (self.n as u32).saturating_sub(1).leading_zeros()) as u8`. Matches sub-plan exactly. Verified at n ∈ {0,1,2,3,4,5,7,8,9,15,16,31,32} via independent computation.

2. **Encoder/decoder mirror (`tree.rs:122-141` ↔ `tree.rs:250-273`):** Exact mirrors of `Tag::Tr | is_nums(1) | [key_index(kiw) iff !is_nums] | has_tree(1) | [tree iff has_tree]`.

3. **`debug_assert!` (`tree.rs:129-132`):** Present, matches sub-plan wording verbatim.

4. **Decoder formula mirror (`decode.rs:25`):** Identical expression to encoder.

5. **Walker NUMS detect (`template.rs:816-848`):** Compares `key_str == NUMS_H_POINT_X_ONLY_HEX`; on match `is_nums: true, key_index: 0`; else `is_nums: false, key_index`. No off-by-one.

6. **Renderer Tr arm (`text.rs:45-49`):** Branches on `*is_nums`. NUMS literal emitted via `NUMS_H_POINT_X_ONLY_HEX` constant when true.

7. **JSON (`json.rs:258-265`):** `JsonBody::Tr` carries `is_nums`; From<&Body> arm correct.

8. **Lifted tests:**
   - `tr_bip86_no_tree`: `is_nums: true, key_index: 0, tree: None`; 8-bit pin (Tag 6 + is_nums 1 + has_tree 1). ✓
   - `tr_nums_n_1_bare_round_trip`: 8-bit pin. ✓
   - `tr_nums_n_4_bare_round_trip`: `write_node(&mut w, &n, 2)` (was 3); ⌈log₂(4)⌉=2. ✓
   - `key_arg_n1_zero_bits`: pin = 6 (Tag 6 + 0 kiw). ✓
   - `key_arg_n3_two_bits`: pin = 8 (Tag 6 + 2 kiw). ✓

9. **New tests:** `tr_nums_flag_round_trip` + `tr_is_nums_false_round_trip` present at tree.rs:521+, both exercise encode→decode round-trips.

10. **bip84 smoke pin:** `smoke.rs:67-70` comment updated; 58 bits = header(5) + path-decl(31) + use-site(16) + Wpkh(6) + 0 kiw + 0 TLV. Test now GREEN.

11. **Body::Tr literal injection:** grep `Body::Tr {` across `crates/` — every literal carries `is_nums:`. No leftover `Body::Tr { key_index: n,` sentinel patterns.

12. **`key_index == n` / `key_index >= n` removal:** Only surviving `key_index >= n` is the bounds check at `canonicalize.rs:267` (now raising `NUMSSentinelConflict` post-I-1 fix).

13. **FOLLOWUPS entry update:** 6 lifted (C: 1 + F: 5); 6 remain (H). Math checks (12 original − 6 lifted = 6 remaining).

14. **`tap_tree_nested_four_leaf_round_trip` kiw at n=5:** kiw=3 (formula confirms); comment updated.

15. **Verification at HEAD + dirty tree:**
    - `cargo test -p md-codec --lib`: 215 / 0 / 6 ✓
    - `cargo test -p md-codec --test smoke`: 8 / 0 ✓
    - `cargo test -p md-codec --test chunking --test forward_compat --test wallet_policy --test address_derivation`: all green
    - `cargo test -p md-cli --bin md --tests`: all green except `help_examples` 2 fails (Phase H FOLLOWUP, unchanged)
    - `cargo clippy --workspace -- -D warnings`: clean

## Verdict

**Ship.** I-1 fixed inline (canonicalize.rs + validate.rs sites now wire NUMSSentinelConflict; test assertion updated to match). All other findings are documentation-only or no-action. Phase G inherits a now-live NUMSSentinelConflict variant ready for its taxonomy-sweep doc-comment finalization.
