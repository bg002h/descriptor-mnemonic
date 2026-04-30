# v0.10.1 — Tier 2 KIV walk wire-up review

**Branch:** `feature/v0.10.1-tier-2-kiv-walk`
**Commits reviewed:** `c3a290d` (impl), `1b5b819` (FOLLOWUPS SHA backfill)
**Baseline:** `6178b3a`
**Date:** 2026-04-29

---

## 1. Verdict

**CLEAN — ready to ship.**

The Tier 2 wire-up is correct, well-tested, and the gate-on-`decoded_shared_path.is_none()` is justified by a real failure mode (dummy-key origin-path leak from `from_bytecode`-materialized policies). No critical or important findings. Two minor polish items optionally inline-fixable; none are blockers.

---

## 2. Scope reviewed

- `crates/md-codec/src/policy.rs` — Tier 2 walk implementation (`try_extract_paths_from_kiv`), new `origin_path_of` helper, gate insertion in `placeholder_paths_in_index_order`, rustdoc updates on `to_bytecode` and `try_extract_paths_from_kiv`, and 5 new tests.
- `crates/md-codec/Cargo.toml` — version bump to 0.10.1.
- `CHANGELOG.md` — `[0.10.1]` section.
- `design/FOLLOWUPS.md` — closure of `v010-p3-tier-2-kiv-walk-deferred` and new `to-bytecode-multipath-shared-at-n-set-key-info-mismatch` entry.
- Cross-checked against the fork at `/scratch/code/shibboleth/rust-miniscript-template-accessor/src/descriptor/wallet_policy/mod.rs` (v0.x with the `template()`/`key_info()` accessors).

Verification: `cargo test --package md-codec` → 484+ tests pass (including the 5 new ones); `cargo clippy --package md-codec --all-targets -- -D warnings` clean; `cargo fmt --package md-codec -- --check` clean.

---

## 3. Findings

### Finding 1 — `origin_path_of` placement splits the `impl WalletPolicy` block (Minor / polish)

**Severity:** Minor.
**Disposition:** Optional inline-fix or FOLLOWUP-able; non-blocking.
**Description:** The new free function `origin_path_of` is defined at lines 311–328, between two `impl WalletPolicy` blocks. The first block ends at line 309 (closing `inner()`) and a fresh `impl WalletPolicy {` reopens at line 330. Placing a free function between them makes the file structure harder to skim. Functionally fine; rustfmt accepts it.
**Suggested fix:** Move `origin_path_of` either above the first `impl WalletPolicy` block (~line 230) or below the second block (after line 852). Pure mechanical edit; no behavior change.

### Finding 2 — `shared_path()` could be DRY-ed against `origin_path_of` (Minor / polish)

**Severity:** Minor.
**Disposition:** FOLLOWUP-able.
**Description:** The match body in `WalletPolicy::shared_path` (lines 295–301) is byte-identical to the new `origin_path_of` helper. The implementer's docstring acknowledges this ("Mirrors the per-variant match in `WalletPolicy::shared_path` …"). `shared_path()` could call `origin_path_of(&first_key)` instead of inlining the match, eliminating one duplication.
**Suggested fix:** Either inline-fix in this PR (one-line change in `shared_path()`), or file a tiny FOLLOWUP. Doesn't affect correctness either way.

### Finding 3 — Aspirational docstring at `from_bytecode` mentions `set_key_info` but no public path exists (Minor / observation)

**Severity:** Minor.
**Disposition:** Pre-existing (predates v0.10.1); not introduced here. Note for future tracking.
**Description:** Line 633–634 says "Real key info must be supplied separately (e.g., during restore flow via `set_key_info`)." But md-codec exposes only `WalletPolicy::inner(&self) -> &InnerWalletPolicy` (immutable); there is no public way to call the fork's `set_key_info` on a decoded policy through md-codec's API. **This means the gate's design assumption — that `decoded_shared_path == Some` implies `key_info` is dummy-populated — is currently airtight in practice for external callers.** If a future md-codec release adds public mutation API for restore flow, the gate will need re-evaluation (see Tier-2 gate assessment §5).
**Suggested fix:** No action this release. Worth a FOLLOWUP "if/when public mutation lands, revisit Tier 2 gate semantics." (Captured in §7 below.)

---

## 4. Behavior assessment — wire-format change is correct + intentional

### Headline claim
Concrete-key descriptors with divergent origin paths (e.g.
`wsh(sortedmulti(2, [fp1/m/48'/0'/0'/2']xpubA/**, [fp2/m/48'/0'/0'/100']xpubB/**))`)
now emit `Tag::OriginPaths` (header bit 3 = 1, byte[1] = 0x36, byte[2] = count = 2),
instead of the v0.10.0 silent flattening to `Tag::SharedPath` via Tier 3.

### Verification

**(a) Tier 2 walk is correct.** Confirmed against fork accessor docs at
`/scratch/code/shibboleth/rust-miniscript-template-accessor/src/descriptor/wallet_policy/mod.rs:111–129`:
- `template().iter_pk()` and `key_info()[i]` are documented to be in matching AST order
  ("`key_info()[i]` is the concrete key at AST position `i`, matching the order of `self.template().iter_pk()`")
- The walk uses `enumerate()` over `iter_pk()` and indexes `key_info()[ast_pos]` — exactly the documented contract.
- `ke.index.0 as usize` correctly indexes the per-`@N` output slot.

**(b) BIP 388 contiguous-index invariant holds.** Confirmed at `wallet_policy/mod.rs:151–168`:
the fork's `validate()` enforces `prev.index.0 == curr.index.0 || prev.index.0 == curr.index.0 - 1`
(saturating). This means placeholder indices appear in monotone order with at most a single
+1 increment between distinct placeholders, so `max(index) + 1` is the placeholder count.
The implementer's reliance on this invariant is correct.

**(c) Multipath-shared `@N` handling is correct.** Same-placeholder AST positions yield the
same key (same logical `@N`), so `origin_path_of(key)` is the same path; the
`(Some(prev), Some(new)) if prev == &new` arm dedups them. Disagreement falls through
to Tier 3 (silent degrade); the BIP 388 invariant should make disagreement impossible
in valid inputs, but the safety arm is appropriate.

**(d) Encoder dispatch is exercised end-to-end.** Test
`tier_2_emits_origin_paths_for_concrete_divergent_descriptor` parses a divergent-path
descriptor, calls `to_bytecode`, and asserts `bytes[0] == 0x08`, `bytes[1] == 0x36`.
Then it round-trips through `from_bytecode` and verifies `decoded_origin_paths` contains
`[m/48'/0'/0'/2', m/48'/0'/0'/100']` in placeholder-index order. **This is a true
wire-level pin, not a helper-level assertion.**

**(e) v0.10.0-vs-v0.10.1 dispatch change is byte-pinned.** Test
`tier_2_drives_encoder_dispatch_change_from_v0_10_0` asserts `bytes[0] == 0x08`,
`bytes[1] == 0x36`, `bytes[2] == 0x02` — header bit 3 set, OriginPaths tag, count
byte 2. Comments document the v0.10.0-vs-v0.10.1 byte change. **This is a regression-pin
test for the headline behavior change.**

### Wire-format compatibility claim
The CHANGELOG claims v0.10.0 decoders can decode v0.10.1 encodings because the
wire format does not change (only widens the input set that flows through the
existing `Tag::OriginPaths` path). **This is correct** — `Tag::OriginPaths = 0x36`
and the OriginPaths block format were defined in v0.10.0; v0.10.1 only adjusts which
inputs trigger that path on the encoder side.

---

## 5. Tier-2 gate assessment (architectural surprise 1)

### What the gate is
At line 513:
```rust
if self.decoded_shared_path.is_none() {
    if let Some(paths) = self.try_extract_paths_from_kiv()? {
        return Ok(paths);
    }
}
```

### Why it's needed
`from_bytecode` (line 800) calls `decode_template(tree_bytes, &dummies)` with
`all_dummy_keys()`, then `from_descriptor` collects the actually-referenced dummy
keys into `key_info`. **Critically, the dummy keys carry real-looking origin paths**
(verified at lines 59–96: each entry has a `[fingerprint/path]` prefix like
`[6738736c/44'/0'/0']`). So a Tier 2 walk over a `from_bytecode`-materialized policy
would extract dummy-table origin paths (e.g., `m/44'/0'/0'`) and emit them as the
"recovered" per-`@N` paths — **silently corrupting the round-trip**.

### Why the gate sits at Tier 2 specifically
The gate cannot move higher (would lose Tier 1's `decoded_origin_paths` short-circuit)
nor lower (Tier 3 must still consume `decoded_shared_path` via the fallback chain).
**The placement is correct.** Both decode-path markers (`decoded_origin_paths == Some`
→ Tier 1 short-circuits; `decoded_shared_path == Some` → Tier 2 skipped, Tier 3
consumes) cover both decoded-bytecode shapes.

### Verification of the claim that "moving decoded_shared_path consumption ahead
of Tier 2 unconditionally would break `to_bytecode_override_wins_over_decoded_shared_path`"
Read the test (lines 1633–1685). Setup:
1. Decode bytecode with SharedPath = `m/84'/0'/0'` → `decoded_shared_path = Some(m/84'/0'/0')`.
2. Encode with `opts.shared_path = Some(m/48'/0'/0'/2')`.
3. Expect `bytes[2] == 0x05` (override wins over decoded path).

If `decoded_shared_path` were consumed before Tier 2 (i.e., directly returned without
checking `opts.shared_path` first), the override would be silently ignored.
**The current Tier 0/Tier 3 layering — `opts.origin_paths` (Tier 0 per-`@N`) at the top,
`opts.shared_path` consumed inside `resolve_shared_path_fallback` (Tier 3) — keeps the
override semantics correct.** The gate's placement (skip-Tier-2-not-Tier-3) preserves
this. Implementer's claim verified.

### Could the gate cause a silent bug?
**One scenario worth flagging** (Finding 3 above):
- A user decodes a SharedPath bytecode → `decoded_shared_path = Some`.
- They (somehow, via internal API mutation or a future public `set_key_info`)
  replace the dummy keys with real keys having different origin paths.
- They call `to_bytecode`. Tier 2 is skipped (gate). Tier 3 fires using
  `decoded_shared_path`. Result: emits the original on-wire shared path,
  **not** the new real-key paths.

**Today this scenario cannot be triggered through md-codec's public API** (no public
`set_key_info`). The gate is airtight in practice. If a future release adds
public mutation, the gate semantics need revisiting — captured as a new
FOLLOWUP below.

### Gate verdict: **CORRECT and well-justified.**

---

## 6. Test coverage assessment

| Test | Asserts what name claims? | Wire-level? | Strong? |
|---|---|---|---|
| `tier_2_emits_origin_paths_for_concrete_divergent_descriptor` | Yes — header byte 0x08, tag 0x36, plus round-trip path order check | Yes (bytes[0], bytes[1], + decoded round-trip) | Strong |
| `tier_2_falls_through_for_template_only_policy` | Yes — direct helper returns None + indirect encoder emits SharedPath | Yes (bytes[0]=0x00, bytes[1]=Tag::SharedPath) | Strong |
| `tier_2_falls_through_for_keys_without_origin_info` | Yes — bare-xpub case, helper returns None + encoder Tier-3-fallback | Yes (bytes[0]=0x00, bytes[1]=Tag::SharedPath) | Strong |
| `tier_2_multipath_shared_at_n_collapses_to_single_path` | Yes — paths.len() == 1, path matches expected | Helper-level only (end-to-end encode elided) | Acceptable, with explanation |
| `tier_2_drives_encoder_dispatch_change_from_v0_10_0` | Yes — pins header bit 3, OriginPaths tag, count byte | Yes (bytes[0]=0x08, bytes[1]=0x36, bytes[2]=0x02) | Strong (regression-pin) |

The multipath-shared test's elided end-to-end encode is well-explained: it documents
that the architectural surprise 2 (`set_key_info` length mismatch in `to_bytecode`)
is pre-existing and out-of-scope. The Tier 2 helper-level assertion is meaningful
in isolation because Tier 2 IS the contract under test for this PR. The comment
at line 2403–2410 points to the new FOLLOWUP for the encoder fix. **Acceptable.**

**Note:** The four edge cases I would have wanted to see — divergent paths, equal
paths (collapses to SharedPath), template-only, missing-origin — are all covered.
Multipath-shared `@N` (same-key disagreement) is the only uncovered edge case, and
that's by-design (BIP 388 forbids it; falling-through to Tier 3 is the safety net).
Coverage is appropriate for the scope.

---

## 7. CHANGELOG / FOLLOWUPS / version bump verification

### CHANGELOG.md `[0.10.1]` section
- ✓ Title + date present.
- ✓ FOLLOWUPS link (`v010-p3-tier-2-kiv-walk-deferred`) cited.
- ✓ "Required" subsection cites `apoelstra/rust-miniscript#2` and the `[patch]` mechanism.
- ✓ "Behavior change" subsection identifies the headline `wsh(sortedmulti(...))` case
  and the v0.10.0 silent-flatten regression that's fixed.
- ✓ "Wire-format compatibility" subsection correctly notes v0.10.0 decoders work,
  v0.10.0 encodings byte-stable for the unchanged input set, and multipath-shared `@N`
  caveat.
- ✓ "Internal — Tier 2 dummy-key safety" subsection explains the gate.
- ✓ FOLLOWUPS closure listed.

### FOLLOWUPS.md
- ✓ `v010-p3-tier-2-kiv-walk-deferred` marked resolved with commit `c3a290d`
  (the SHA backfill in commit `1b5b819`).
- ✓ Resolution note describes what was wired up, the gate, and the
  pinning test by name.
- ✓ Tier annotation: `v0.11 → v0.10.1 (closed; folded into v0.10.1)`.
- ✓ New entry `to-bytecode-multipath-shared-at-n-set-key-info-mismatch` is well-scoped:
  - Surfaced context (where, while writing which test).
  - Where to fix (file + function + line range, dummy-key materialization block).
  - What to do (rework `dummy_keys(count)` → walk `iter_pk()` and broadcast per-`@N`
    dummy across AST positions). Pure encoder fix, no wire-format change.
  - Why deferred (pre-existing v0.x gap; better as its own focused commit).
  - Status: open. Tier: v0.10.2 or v0.11.

### Cargo.toml
- ✓ `version = "0.10.1"`.

### Cargo.lock
- ✓ Updated to match.

### New FOLLOWUP I'd like to see appended
**`tier-2-gate-revisit-when-public-set-key-info-lands`** — see Finding 3. If md-codec
ever adds a public mutation API allowing replacement of `key_info` on a
decoded policy, the `decoded_shared_path.is_none()` gate's design assumption
(that `decoded_shared_path = Some` implies dummy `key_info`) becomes wrong.
The fix (when needed) would be a real "is dummy?" flag rather than the
current proxy via `decoded_shared_path`. **Today the gate is airtight; this is
forward-looking only.** Tier: vNext-API-expansion. Append to `design/FOLLOWUPS.md`
before merging this PR or as a quick housekeeping commit.

---

## 8. Recommended action: ready to ship

The patch is correct, well-tested, well-documented, and the Tier-2 gate is
justified and explained at multiple levels (commit message, rustdoc on the call
site, CHANGELOG "Internal — Tier 2 dummy-key safety" subsection). The
`origin_path_of` helper-placement nit (Finding 1) and the `shared_path()` DRY
opportunity (Finding 2) are pure polish; the forward-looking gate-revisit
FOLLOWUP (Finding 3 / §7) is a tracking item, not a code change.

**Recommended:** ship as-is. Optionally append the
`tier-2-gate-revisit-when-public-set-key-info-lands` FOLLOWUP entry before
merging (one-line operation; non-blocking).
