# Phase A code review — r1

**Agent:** feature-dev:code-reviewer
**Date:** 2026-05-10
**Working tree:** uncommitted on `main` (parent = `6d58229` / md-codec-v0.30-spec-frozen)
**Files reviewed (4):**

- `crates/md-codec/src/tag.rs` (~199 lines changed)
- `crates/md-codec/src/error.rs` (~18 lines changed)
- `crates/md-codec/src/tree.rs` (12 lines added — `#[ignore]` annotations only)
- `design/FOLLOWUPS.md` (12 lines added — new entry `v0.30-phase-a-tree-tests-ignored-pending-corpus-regen`)

---

**Summary.** Phase A is correctly scoped and mechanically sound: the 6-bit/4-bit primary/extension widths match SPEC §3.1, all 35 variant codes match the §3.2 table exactly, `TagOutOfRange { primary: u8 }` is correct per §11.1, the extension prefix subcode is consumed before returning the error, the 4-arm `tag_reserved_range_rejected` test covers both boundaries of both branches, and the orphan-variant grep is clean. Two stale doc-comment artifacts in `error.rs` (outside Phase A's nominal scope but visible in the working tree) merit flagging as Important. No logic bugs.

---

**Findings table.**

| Priority | Finding | Where (file:line) | Recommendation |
|----------|---------|-------------------|----------------|
| Important | `error.rs` module doc and enum doc-comment both say "v0.11-specific" / "v0.11 wire-format". These are read by every consumer of the `Error` type and by `rustdoc`. Post-Phase A the file already carries `TagOutOfRange` which is a v0.30 concept; the "v0.11" framing is actively misleading now that both old and new variants coexist. The plan says `error.rs`'s prose rewrite completes in Phase B (`crates/md-codec/src/lib.rs:8-11` + `header.rs` prose listed there), but `error.rs` lines 1 and 5 are not listed in Phase B's surface-scan table — they may fall through the cracks. | `crates/md-codec/src/error.rs:1,5` | Drop or generalize the "v0.11" qualifier now (e.g., "Error variants for the md-codec wire-format codec."); alternatively, add `error.rs:1,5` to Phase B's scope table in IMPLEMENTATION_PLAN explicitly so it isn't missed. |
| Important | `ForbiddenTapTreeLeaf`'s field doc-comment says "Primary **5-bit** tag code of the forbidden leaf." Phase A widened bytecode primary tags to 6 bits. Any future reader using this doc to interpret the `tag` field will get the wrong bit-width. This variant is in `error.rs`, which Phase A touches. | `crates/md-codec/src/error.rs:172` | Change to "Primary **6-bit** tag code (bytecode space) of the forbidden leaf." One word fix. |
| Low | `ripemd160_extension_round_trip` in `tree.rs` (line 523) is not ignored but its test name contains `_extension_` — the same misleading suffix the plan called out for the three tag-level tests (`tag_hash256_extension` → `tag_hash256` etc.). The tree test will pass (no bit-count pin), so it's not a correctness issue, but the name implies Ripemd160 is still an extension-space tag, which it isn't in v0.30. | `crates/md-codec/src/tree.rs:524` | Rename to `ripemd160_round_trip` when tree.rs is touched in Phase C/F (add to FOLLOWUPS if not doing it now). |
| Nit | `error.rs` lines 17-18 (`ReservedHeaderBitSet`) and several other variants still contain "v0.11 requires …" in their `#[error]` format strings and doc-comments. Phase B will rewrite these; acceptable to leave until then, but the list at error.rs:37-38, 53-54, 72-73, 80, 176-177 are all "v0.11"-branded. Not a Phase A defect but confirms the Phase B scope must include `error.rs` prose globally. | `crates/md-codec/src/error.rs` (multiple lines) | Confirm Phase B scope explicitly covers all `error.rs` "v0.11" prose strings; add to IMPLEMENTATION_PLAN §3 Phase B surface-scan table if missing. |

---

**Detailed notes on review axes:**

1. **SPEC §3 conformance (tag.rs):** All 35 variant codes verified against the SPEC §3.2 table — exact match. `EXTENSION_PREFIX_6BIT = 0x3F` correct. `Tag::write` emits 6 bits primary + 4 bits extension. `Tag::read` reads 6 bits, hits the extension prefix branch first, consumes 4-bit subcode via `read_bits(4)` (cursor advanced — no alignment bug), returns `TagOutOfRange { primary: 0x3F }`. Reserved range 0x24..=0x3E falls through to the `_` wildcard arm and returns `TagOutOfRange { primary }` carrying the raw value. Module doc accurately describes v0.30 layout with no stale v0.11/v0.17/v0.18 references.

2. **Error taxonomy (error.rs):** `TagOutOfRange { primary: u8 }` is present with exact name and field type per SPEC §11.1. `UnknownPrimaryTag` / `UnknownExtensionTag` absent — workspace grep returns zero matches. `TagOutOfRange` doc-comment is accurate and complete. The Important findings above are the only issues.

3. **Extension subcode cursor discipline:** Confirmed — `let _subcode = r.read_bits(4)?` at tag.rs:160 consumes the 4-bit subcode unconditionally before returning the error. No cursor mis-alignment possible.

4. **Test coverage:** The 4-arm `tag_reserved_range_rejected` covers the two explicit boundaries the prior meta-review flagged as Important: primary 0x24 (low), 0x3E (high), extension 0x3F+0x00 (low), 0x3F+0x0F (high). Round-trip tests for 7 representative variants. No byte-level pin tests, but round-trips are sufficient for Phase A per plan.

5. **tree.rs `#[ignore]` discipline:** Exactly 12 annotations, all matching the FOLLOWUPS entry list. All have descriptive reason strings citing the lift phase. `true_round_trip` and `ripemd160_extension_round_trip` are correctly left un-ignored (no bit-count pins). `sortedmulti_2of3_bit_cost` is correctly ignored (has a pin). No logic-only tests were accidentally silenced.

6. **FOLLOWUPS entry:** All 12 test names match actual annotations. Phase attributions are consistent. Entry follows file format (short-id, Surfaced, Where, What, Why deferred, Status, Tier).

7. **Workspace impact:** Zero `UnknownPrimaryTag` / `UnknownExtensionTag` callers remain. TLV tag width at `tlv.rs:203,229` confirmed 5-bit — Q13 split preserved.

---

**Verdict: ITERATE (0 Critical, 2 Important)**

The two Important findings are both one-line doc fixes in `crates/md-codec/src/error.rs` (lines 1/5 and line 172). No logic changes required.
