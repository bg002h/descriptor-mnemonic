# v0.5 spec holistic review — Opus 4.7

**Status:** DONE
**Subject:** `design/SPEC_v0_5_multi_leaf_taptree.md` at commit `fcef2a7`
**Codebase ground truth:** `crates/md-codec` at v0.4.1 (`270bf57`)
**Verdict:** **NEEDS-REVISION** — multiple factual errors against codebase ground truth and one off-by-one in the depth-gate semantics. Mechanically tractable; not a re-design.

## Executive summary

The spec is structurally sound and the design choices (pure admission, peek-before-recurse, family-stable SHA bump, no past-release deprecation banners) are correctly inherited from v0.4 precedent. However, the spec ships **five concrete factual errors** against the current codebase that an implementer would discover only at type-check time, plus **one logic bug** in the depth-gate semantics that would cause v0.5.0 to reject legal BIP 341 trees. The FOLLOWUPS accounting math (line 7 and line 400) is also wrong against the current `design/FOLLOWUPS.md` state.

These are not subjective ambiguities; they are claims about code that doesn't match what's in the repo. Fix list is mechanical: ~10 line edits in §3, §4, §5, plus one substantive correction to the depth gate in §3 + corresponding fixture text in H1/H2/§6.

After these are folded in, the spec should land at READY.

## Critical findings (block ship)

### C1 — `DecodeReport.tap_leaves` does not exist; `TapLeafReport` does not exist
**Spec lines:** 148-149 (§3); 230-241 (§4); 384 (§6 CHANGELOG); 328 (LI1).
**Issue:** Spec line 231 reads "Existing `decode_report.tap_leaves` was already a `Vec<TapLeafReport>` (populated with one entry for v0.4.x single-leaf case)". This is false. `crates/md-codec/src/decode_report.rs:111-120` defines `DecodeReport` with exactly four fields: `outcome`, `corrections`, `verifications`, `confidence`. There is no `tap_leaves` field today, and `grep -rn TapLeafReport` returns zero hits anywhere in the source tree (only the spec itself).
**Implication:** §3 leaf-index plumbing, §4 type definition, §6 CHANGELOG entry "decode_report.tap_leaves[] populated…", and LI1 test all stand on a non-existent field. The implementation plan must add this field (and the struct), not just "populate" it. This is a real new public-API surface — `DecodeReport` is `#[non_exhaustive]` so the addition is non-breaking, but it is an addition, not an augmentation.
**Fix:** In §4 around line 230, change wording to "v0.5 introduces a new `tap_leaves: Vec<TapLeafReport>` field on `DecodeReport` (additive on the `#[non_exhaustive]` struct — non-breaking)" and define `TapLeafReport` as a NEW struct. In §3 line 148, revise to "the new `decode_report.tap_leaves[]` field added in §4". In §6 CHANGELOG add: "Added: `DecodeReport.tap_leaves` field (new public type `TapLeafReport`)". LI1 still works as written.

### C2 — `BytecodeErrorKind::TruncatedBytecode` is not a real variant
**Spec lines:** 294 (N1); 314 (H3).
**Issue:** Both fixtures expect `InvalidBytecode { kind: TruncatedBytecode }`. The actual variant set in `crates/md-codec/src/error.rs:353-444` is: `UnknownTag`, `Truncated`, `VarintOverflow`, `MissingChildren`, `UnexpectedEnd`, `TrailingBytes`, `ReservedBitsSet`, `TypeCheckFailed`, `InvalidPathComponent`, `UnexpectedTag`, `MalformedPayloadPadding`. There is no `TruncatedBytecode`.
**Implication:** N1 and H3 will not compile; an implementer would have to guess the intended variant. From the call-pattern in `decode_terminal` (lines 334-345), running off the cursor mid-parse surfaces as `UnexpectedEnd` (from `read_byte` failing) — which is then re-wrapped as `MissingChildren` by some call sites. For a `TapTree` recursion that hits EOF reading the right-child framing byte, the natural surface is `UnexpectedEnd` (which is what the spec's H3 description "[TapTree] then EOF" actually corresponds to).
**Fix:** Replace `TruncatedBytecode` with `UnexpectedEnd` in both N1 (line 294) and H3 (line 314). Optionally add a sentence acknowledging that an alternative `MissingChildren { expected: 2, got: <0|1> }` would also be a defensible rendering and the implementer should pick whichever surfaces best in the diagnostic — but the spec must commit to one expected variant per fixture for the test to be deterministic.

### C3 — N9 expected error is wrong (`UnknownTag` vs `PolicyScopeViolation`)
**Spec line:** 302 (N9).
**Issue:** N9 fixture: "`[TapTree]` as top-level descriptor (no `Tr` outer)" expecting `InvalidBytecode { kind: UnknownTag }`. Reality: top-level dispatch is `decode_descriptor` at `decode.rs:61-106`. `Tag::from_byte(0x08)` returns `Some(Tag::TapTree)` (verified at `tag.rs:148`), which falls through to the `Some(other) => Err(Error::PolicyScopeViolation(...))` arm at line 98-100, **not** the `None` arm at line 101-104. So a top-level `0x08` byte produces `PolicyScopeViolation("v0.4 does not support top-level tag TapTree")`, not `UnknownTag`.
**Implication:** N9 fixture would fail validation against actual decoder output.
**Fix:** Line 302 — change expected error to `PolicyScopeViolation` with the exact dispatcher message, OR (cleaner) move N9 to fire from the `decode_tap_subtree` recursive helper at depth 1 and document that "top-level `0x08` is rejected by the existing top-level dispatcher with `PolicyScopeViolation`; this fixture covers the *recursive-helper* unknown-tag path with a different bytecode shape". Note that v0.5 will need to update the top-level dispatcher's message text from "v0.4 does not support top-level tag TapTree" — the message refers to v0.4 but the tag is now active. Either update the message in the dispatcher fix (preferred), or accept the slightly-misleading message and document it.

### C4 — Depth-gate off-by-one rejects legal BIP 341 trees at depth 128
**Spec lines:** 104 (`if depth >= 128`); 312-313 (H1/H2 fixtures); 22 (decision-matrix "BIP 341 consensus depth (128)"); 313 (H2 description "128 framings").
**Issue:** BIP 341 / `TAPROOT_CONTROL_MAX_NODE_COUNT = 128` allows a script tree with leaves at depth ≤ 128. Rust-miniscript `TapTree::combine` (verified at `rust-miniscript-fork/src/descriptor/tr/taptree.rs:40-50`) accepts inputs with `depth ≤ 127` and produces output with `depth ≤ 128` — i.e., 128-deep leaves are legal at the upstream layer. The spec's helper starts `depth = 1` (line 133) and rejects `if depth >= 128` after consuming the framing byte (line 104). Trace:
  - depth=1 (first call from `decode_tr_inner`) sees TapTree → consume → check `1 >= 128` (no) → recurse to depth=2. ...
  - depth=127 sees TapTree → consume → check `127 >= 128` (no) → recurse to depth=128.
  - depth=128 sees TapTree byte → consume → check `128 >= 128` (yes) → REJECT.

So the helper allows at most **127** levels of `[TapTree]` framing. With 127 framings the deepest leaf — counted via `TapTree::combine`'s post-increment — ends up at miniscript-depth 127, **not** 128. This means MD v0.5 rejects the BIP-341-legal 128-deep boundary case.

The fixture text in H1 ("128-deep nested left-spine: 127 `[TapTree]` framings + 1 leaf at depth 128 + 127 trailing leaves") and H2 ("128 `[TapTree]` framings + 1 leaf") describes a legal-vs-illegal cut at 128/129 framings (i.e. leaves at depth 128 are legal, 129 illegal). That cut requires the gate to fire at `depth > 128`, not `depth >= 128`.

**Implication:** Either (a) the gate is wrong and the spec under-admits BIP 341 by one level, or (b) the fixture text and the decision-matrix wording are wrong and the gate is intentionally one-tighter than BIP 341. The spec's stated rationale at line 22 ("Same ceiling that the Bitcoin consensus rules impose on control-block paths. No tighter MD-specific cap.") strongly implies (a).
**Fix (preferred):** Change line 104 to `if depth > 128`. Update line 141's hostile-input commentary so the diagnostic-offset description still tracks the new gate. Verify H1 ("128-deep" passes) and H2 ("129-deep" rejects) against the new gate. If instead the implementer prefers the conservative one-tighter cap, the decision-matrix entry at line 22 must be amended to say so explicitly and the FAQ must explain the divergence from BIP 341.

### C5 — `decode_tap_miniscript` two-arg signature claim
**Spec line:** 145.
**Issue:** "This index is plumbed through to `decode_tap_miniscript`'s `Some(index)` argument, **which already exists on the v0.4.x signature for single-leaf (always `Some(0)`)**". False. Verified at `decode.rs:581-584`: `fn decode_tap_miniscript(cur: &mut Cursor<'_>, keys: &[DescriptorPublicKey]) -> Result<...>`. Two parameters, no `Option<usize>` index.
**Implication:** Implementer reading line 145 would think this is a no-signature-change addition. It is in fact a signature change to an internal function (`decode_tap_miniscript` and likely `decode_tap_terminal`), all callers must be updated, and `validate_tap_leaf_subset` likewise needs a signature change to accept `Option<usize>`. Both functions are `pub fn` on `validate_tap_leaf_subset` (line 468 of encode.rs is `pub`); the rest are private.
**Fix:** Rewrite line 144-150 to "v0.5 extends `decode_tap_miniscript` and `decode_tap_terminal` with a new `leaf_index: Option<usize>` parameter (currently absent at v0.4.x). The single-leaf decode path in `decode_tr_inner` passes `Some(0)`; the recursive helper threads the running counter as `Some(index)`. `validate_tap_leaf_subset`'s public signature also gains `leaf_index: Option<usize>` for routing into the extended `Error::TapLeafSubsetViolation { ..., leaf_index }`." Note this surfaces a pub-API change that should be in the CHANGELOG (line 380-388) — currently absent.

## Important findings (fix before ship)

### I1 — FOLLOWUPS accounting math is wrong
**Spec lines:** 7 (header summary); 400 (§6 net state).
**Issue:** Line 7 says "Carry-forward… 4 v0.3-deferred items, 5 wont-fix entries". Line 400 says "Net FOLLOWUPS state at v0.5.0 ship: 6 open + 5 wont-fix (was 7 open + 5 wont-fix at v0.4.1)".

Verified by direct count of `design/FOLLOWUPS.md` "Open items" section: **10 entries with status "open"** (counting `slip-0173-register-md-hrp` which is annotated "resolved (PR filed)" but lives in Open items as the originally-filed item; and counting `phase-5-cli-wdm1-assertion-sweep` which is "open (will be resolved in Phase 5 of the rename)") — and **3 entries with status "wont-fix"** (`legacy-pkh-permanent-exclusion`, `legacy-sh-multi-permanent-exclusion`, `legacy-sh-sortedmulti-permanent-exclusion`). Closing `v0-5-multi-leaf-taptree` brings the count to 9 open + 3 wont-fix at v0.5.0 ship, not 6 + 5.

The "5 wont-fix" claim has no support in the file; the v0.4 spec (lines 555-559) only filed 3 wont-fix entries. I cannot find a fourth or fifth wont-fix in the open-items section.

**Fix:** Re-count from FOLLOWUPS.md and rewrite lines 7 and 400 with the correct numbers. Suggested replacement for line 400: "Net FOLLOWUPS state at v0.5.0 ship: 9 open + 3 wont-fix (was 10 open + 3 wont-fix at v0.4.1; close 1 = `v0-5-multi-leaf-taptree`)."

### I2 — Test count "≥639" off-by-one
**Spec lines:** 272; 426.
**Issue:** "Sum: 29 NEW + 1 RENAMED listed in tables. Final passing count target ≥639 (609 baseline at v0.4.1 + at least 30…)". The RENAMED fixture (T2) is not a new test — it's the existing v0.4.x single-leaf fixture under a new filename. So it adds 0 to the test count, not 1. 609 + 29 = 638, not 639.

This is a small but factual arithmetic slip; the "≥639" target is one too high if the table count is the authority.
**Fix:** Either change the target to ≥638 (preferred — matches the math) or re-count and acknowledge that `gen_vectors` expansion of new positive fixtures into encode/decode test pairs justifies the "+30" rather than +29. Spec at line 263 already hedges with "may add 2-3 implicit per-fixture variants"; if the +1 is supposed to come from that hedge, say so explicitly at line 272.

### I3 — `TapLeafSubsetViolation` construction-site count
**Spec line:** 225.
**Issue:** "All 4 construction sites in the codebase (encoder validate, decoder validate, plus 2 in error-rendering helpers per Section 3 review)". Verified by grep: there are **3** `Err(Error::TapLeafSubsetViolation { ... })` construction sites: `encode.rs:443` (Terminal encoder catch-all), `encode.rs:487` (`validate_tap_leaf_terminal` catch-all), and `decode.rs:691` (`decode_tap_terminal` catch-all). The "2 in error-rendering helpers" is unattributable to anything in the code today.
**Fix:** Either document each of the 3 sites by file/line in the spec, or rephrase as "All construction sites of `Error::TapLeafSubsetViolation` (3 in the codebase today)…". Note: there are also 2 `validate_tap_leaf_subset` *call* sites (`encode.rs:154` and `decode.rs:276`), which need their plumbing updated — possibly the spec is conflating call sites with construction sites. The implementer plan should enumerate them explicitly.

### I4 — `validate_tap_leaf_subset` signature change is implicit
**Spec lines:** 174, 201; cross-ref to C5.
**Issue:** Spec uses `validate_tap_leaf_subset(ms, Some(leaf_index))?` at lines 174 and 201, implying a two-arg signature. Current signature (verified at `encode.rs:468`) is one arg. This is a public-API change because the function is `pub fn`. The CHANGELOG (lines 380-388) does not mention it.
**Fix:** Add a CHANGELOG entry: "Changed: `validate_tap_leaf_subset` public signature gains `leaf_index: Option<usize>` parameter — additive but breaking for any external caller (no known external callers exist)." Also flag in §4 type-wiring narrative.

### I5 — `decode_tap_subtree` initial-depth value is unstated rationale
**Spec lines:** 133 (caller invocation `depth=1`), 109-110 (recursive call `depth + 1`).
**Issue:** The caller passes `depth=1` from `decode_tr_inner`, but the spec doesn't explain why `1` rather than `0`. If `depth` represents "number of `[TapTree]` framings consumed so far on this path", then the initial value before any framing is consumed should be `0`, and the gate compares against the BIP 341 max. If `depth` represents "depth of leaves we'd accept here", then `1` is right because the caller has not consumed any framing yet but the leaves under `decode_tr_inner` start at miniscript-depth-1 (post-`combine`). The semantics are not explicit; combined with C4 the off-by-one becomes hard to reason about.
**Fix:** Add one sentence at §3 line 133 defining `depth` precisely. Suggested: "`depth` counts the `[TapTree]` framings consumed on the path from `decode_tr_inner` (initial = 1 because the first call sees byte 0x08 already; the gate fires when an additional framing would exceed BIP 341's 128-leaf-depth limit)." Reconcile with C4 fix.

### I6 — Spec doesn't address what happens to encoder-emitted depth-violating policies
**Spec lines:** 187-211 (encoder routing), 312-318 (H1/H2 hostile inputs).
**Issue:** Decoder side has explicit depth-128 gate. Encoder side relies on rust-miniscript's `TapTree::combine` rejecting overly deep inputs at construction time (since `Descriptor::Tr` is built via that API). But the spec doesn't say what happens if a caller hands in a `Descriptor::Tr` that somehow contains an over-deep tree (e.g. via deserialization that bypasses `combine`). Today rust-miniscript's `TapTree` constructors prevent this, but the encoder should emit a clean error if the invariant is somehow violated rather than silently producing oversized bytecode. Spec's encoder helper at lines 163-184 has no upper-bound check.
**Fix:** Either (a) add a depth check in `encode_tap_subtree` that mirrors the decoder's, (b) state explicitly that `TapTree::combine`'s upstream depth rejection is the guarantee and the encoder relies on it (defensive check unnecessary), or (c) add a debug-assertion. Recommend (b) with a sentence in §4. This also unblocks LI2's expected `leaf_index: Some(<expected index>)` because a depth-overflow path would never produce one.

### I7 — N3-N7 leaf-index expectation is loose
**Spec lines:** 296-300; 329 (LI2).
**Issue:** N3-N7 say "`leaf_index: Some(_)`" — i.e., "any non-None index". For a deterministic test, the expected `leaf_index` should be the exact integer (matching the DFS-pre-order index of the offending leaf in each fixture). LI2 says "exposes `leaf_index: Some(<expected index>)`" (no concrete number).
**Fix:** Pin each of N3-N7's `leaf_index` to the actual DFS index in the fixture's tree shape. E.g. if N3's hostile fixture is `{wpkh-leaf, pk(@1)}`, the wpkh leaf is `leaf_index = 0`. State the index per fixture so the assertion is exact.

### I8 — Single-leaf carve-out's "depth 0" guard is over-tight relative to the v0.4 path
**Spec lines:** 198, 213.
**Issue:** Spec says single-leaf carve-out is `leaves.len() == 1 && leaves[0].0 == 0`. v0.4 encoder path at `encode.rs:141-150` actually rejects single-leaf with `depth != 0` as `PolicyScopeViolation`. So under v0.5, a `TapTree` with one leaf at depth ≠ 0 (which rust-miniscript doesn't normally produce, but isn't structurally impossible) would now flow through the multi-leaf path and emit `0x08` framing — fine, but the spec doesn't note that the v0.4 explicit `PolicyScopeViolation("single-leaf TapTree must have depth 0…")` rejection is being removed (subsumed). Existing v0.4-rejecting tests for that path will break or need re-classification.
**Fix:** Add to §6 deliberate-NOT-do list (or §4): "v0.4's single-leaf-with-non-zero-depth `PolicyScopeViolation` is removed — under v0.5 such inputs are admitted via the multi-leaf path. No known producer emits this shape; the rejection was theoretical." If a v0.4 test asserts that rejection, list it in the §5 infrastructure-modifications section as needing removal.

## Minor findings (nice-to-have)

### M1 — BIP line numbers slightly imprecise
Spec line 249 cites BIP §"Top-level descriptor scope" 85-89; verified actual lines are 82-89 (the `tr(KEY)` admittance starts at 84 and the deferral note runs through 88). One-line drift.
**Fix:** Verify line numbers against `bip/bip-mnemonic-descriptor.mediawiki` HEAD before tag.

### M2 — Spec uses `&placeholder_map` once, no-`&` style elsewhere
Spec line 202 reads `leaves[0].1.encode_template(out, &placeholder_map)?;` but the surrounding context (and existing `encode.rs:155`) uses `placeholder_map` directly because it is already a `&HashMap`. Cosmetic.
**Fix:** Drop the `&` at line 202.

### M3 — Spec uses `Cursor` (no lifetime) in helper signature
Spec line 95 declares `cur: &mut Cursor`. Existing decoder uses `Cursor<'_>` everywhere (e.g. `decode.rs:62`). Will compile via lifetime elision but inconsistent with house style.
**Fix:** Spell as `Cursor<'_>` for consistency.

### M4 — "additive on `#[non_exhaustive]` enum" — backwards-compat claim is partially right
Spec line 215, 227. Adding a field to a struct variant on a `#[non_exhaustive]` enum is non-breaking ONLY for exhaustive `match` consumers if the struct variant itself was already field-exhaustive-bound (i.e., if a downstream wrote `Error::TapLeafSubsetViolation { operator }` they'll get a "missing field `leaf_index`" error). The spec's "they would need to update — but no external crate constructs MD's error variants" lampshades this for construction; for *destructuring* (e.g. `Error::TapLeafSubsetViolation { operator } => ...`), downstream consumers will see a soft warning unless the variant is also `#[non_exhaustive]` at the variant level (which it currently isn't). The struct variant in `error.rs:319-324` is a plain variant.
**Fix:** Either annotate the variant as `#[non_exhaustive]` (rust 1.84+ supports this; project MSRV is 1.85, so OK) and document, or acknowledge that destructuring downstream consumers will need to add `..` at the pattern site. State the chosen approach.

### M5 — Test name `t4_t5_bytecodes_differ_explicit` doesn't read naturally
Spec line 327. A more readable version: `t4_left_heavy_and_t5_right_heavy_emit_distinct_bytecodes`. Cosmetic.

### M6 — "≥639 tests + 0 ignored" sequence in §5 vs §6
§5 line 272 says ≥639 + 0 ignored; §6 line 426 same. Both ought to acknowledge the I2 off-by-one or restate the math. Fix together with I2.

### M7 — `decode_tap_subtree` returns leaf with `TapTree::leaf(leaf)` at line 119 — but this loses depth
Spec line 119: `Ok(TapTree::leaf(leaf))` always returns a depth-0 leaf, then the caller's `TapTree::combine` chain in lines 109-111 increments depth as we unwind. This is correct given how `combine` works (verified at `taptree.rs:40-50`), but the spec doesn't explain *why* the helper hands back a depth-0 leaf — readers familiar with TapTree semantics will trace the unwind. A one-sentence note would help.
**Fix:** Add a comment near line 119: "The depth-0 leaf is the seed; each enclosing `TapTree::combine` post-increments depth by 1 as the recursion unwinds, so a leaf encountered at recursion-depth N ends up at miniscript-depth N − 1 in the final tree."

### M8 — §6 phase 2 / phase 3 ordering question
Spec line 408 lists: "2. Type wiring → 3. Decoder → 4. Encoder". Adding the `leaf_index` field to `Error::TapLeafSubsetViolation` (phase 2) before changing decoder/encoder code (phases 3-4) means the phase-2 commit will leave call sites stale. The v0.4 plan (per `IMPLEMENTATION_PLAN_v0_4_*` style) typically lands type wiring + minimal call-site updates in one phase. Cosmetic for the spec; the real plan via `writing-plans` should fold them.

## Strengths

- **Wire-format-additive framing** mirrors v0.4 precedent exactly (line 9, line 360-365). Decision-matrix table at lines 19-24 is crisp and pre-locks the four big knobs.
- **Single-leaf preservation** is correctly carved out (lines 152-154, 198, 437) and the `leaves.len() == 1 && leaves[0].0 == 0` discriminator is *exactly* tight against rust-miniscript's `TapTree::leaf` constructor (which always yields `(0, ms)` per `taptree.rs:36`).
- **Sh-shape-parity rationale** (line 33, 91-127) correctly identifies that the v0.4 peek-before-recurse defense pattern transfers to the TapTree node level, with the right intuition about hostile-input cursor diagnostics.
- **Family-stable SHA framing** (lines 76-78, 364-365) faithfully reuses the v0.2.x and v0.4.x bump pattern.
- **Past-release framing** (line 369) correctly preserves v0.4.x as a smaller-surface subset — no deprecation banners. Matches v0.4 precedent.
- **Negative-fixture corrections folded inline** (lines 304, 318, 348-353) show that the per-section reviews materially improved the document.
- **Scope discipline** — §1 line 28 and §6 lines 433-438 are explicit about NOT broadening the per-leaf miniscript subset; the `validate_tap_leaf_subset` constants and call-site preservation note (line 85) confirms the carve-out holds. I checked all fixture descriptions in §5 — none implies admitting a leaf type currently rejected by `validate_tap_leaf_subset`. **Scope discipline holds.**
- **Carry-forward FOLLOWUPS list** (lines 7, 394-398) correctly identifies the two phase-d-tap-* entries as independent of v0.5 scope, the apoelstra PR pin, and the SLIP-0173 PR. The intention is right; only the *count* in I1 is wrong.
- **rust-miniscript v13 API claims** about `TapTree::leaf`, `TapTree::combine`, `tap_tree()`, and `leaves()` returning items with `.depth()`/`.miniscript()` — all verified correct at `rust-miniscript-fork/src/descriptor/tr/taptree.rs:33-200`.

## Cross-section consistency check

- **Decoder depth check vs encoder behavior** — see C4. Decoder rejects at depth 128 of recursion (i.e. at most 127 framings); encoder relies on upstream `TapTree::combine` to never produce > 128 leaf-depth. These are mutually consistent only if C4 is fixed (gate to `> 128`) — otherwise the encoder can produce trees the decoder will reject on round-trip. RT1-RT3 fixtures don't hit this boundary, but a hostile producer could.
- **Peek-before-recurse pattern** mirrors v0.4 Sh restriction matrix (`decode.rs:139-160`). Verified by reading both code paths — same idiom (`peek_byte` → tag-dispatch → `read_byte` after dispatch). **Internally consistent.**
- **Error::TapLeafSubsetViolation extension** — definition (§4 line 215-225), test expectations (§5 N3-N7 lines 296-300), CHANGELOG (§6 line 385) — all use the same `{ operator, leaf_index: Option<usize> }` shape. **Internally consistent.**
- **Test count math** — see I2 (off-by-one).
- **Single-leaf carve-out** — §4 lines 198-213 vs §3 lines 152-154 vs §6 line 437 — all align on "byte-identical to v0.4.x for the single-leaf case". **Internally consistent.**

## Summary table

| Severity | ID | Spec line | Fix size |
|---|---|---|---|
| CRITICAL | C1 | 148-149, 230-241, 384, 328 | Add new struct + field; ~15 lines spec edit |
| CRITICAL | C2 | 294, 314 | 2-line edit |
| CRITICAL | C3 | 302 | 1-line edit + dispatcher message decision |
| CRITICAL | C4 | 104, 22, 312-313 | 1-line code change + fixture text |
| CRITICAL | C5 | 145 | Rewrite §3 paragraph |
| IMPORTANT | I1 | 7, 400 | Re-count and rewrite |
| IMPORTANT | I2 | 272, 426 | Arithmetic correction |
| IMPORTANT | I3 | 225 | Re-attribute |
| IMPORTANT | I4 | 174, 201, 380-388 | CHANGELOG + narrative |
| IMPORTANT | I5 | 133 | One sentence |
| IMPORTANT | I6 | 187-211 | One sentence |
| IMPORTANT | I7 | 296-300, 329 | Pin 5 indices |
| IMPORTANT | I8 | 198, 213 | Note v0.4 path subsumption |
| MINOR | M1-M8 | various | Cosmetic |

## FOLLOWUPS appended

No FOLLOWUPS appended. All identified issues are within the immediate review-revision cycle (the spec must be corrected before writing-plans handoff). No items survive past the cycle.

---

## Confirmation pass — commit e6e8477

**Verdict:** **READY-WITH-MINOR-FIXES** — all 5 critical findings and 8 important findings resolved; selected minors resolved; only cosmetic residuals remain. Controller may proceed to user sign-off + writing-plans handoff. Two trivial fixes can be folded at any time before tag (or skipped without harm).

### Per-finding resolution

| ID | Status | Note |
|---|---|---|
| C1 | resolved | §3 line 162-164 + §4 lines 247-262 reframe `tap_leaves[]` and `TapLeafReport` as NEW additions on `#[non_exhaustive]` `DecodeReport`. |
| C2 | resolved | N1 (line 317) → `UnexpectedEnd`; H3 (line 337) → `UnexpectedEnd`. Variant verified real. |
| C3 | resolved | N9 (line 325) now expects `PolicyScopeViolation` with updated dispatcher message; CHANGELOG line 413 logs the message-text change. |
| C4 | resolved | Gate at line 104 is `if depth > 128`. Depth-semantics paragraph (line 142) defines `depth` precisely. H1 (line 335) and H2 (line 336) now describe the legal-128 / illegal-129 boundary. **Re-traced in confirmation pass — no residual off-by-one** (see C4 re-trace below). |
| C5 | resolved | §3 lines 153-159 explicitly mark `decode_tap_miniscript`, `decode_tap_terminal`, `validate_tap_leaf_subset` signatures as NEW at v0.5; CHANGELOG line 412 logs the public-API change. |
| I1 | resolved | Spec lines 7 and 429 now read "9 open + 3 wont-fix at v0.5.0 ship; 10 open + 3 wont-fix at v0.4.1". **Re-counted from FOLLOWUPS.md HEAD — confirmed correct** (see I1 re-count below). |
| I2 | resolved | Lines 293, 455 → "≥638 tests + 0 ignored (609 baseline + 29 new); ≥640 once gen_vectors expands". |
| I3 | resolved | Line 243 attributes 3 construction sites by file:line, distinguishes from 2 call sites. |
| I4 | resolved | CHANGELOG line 412 logs `validate_tap_leaf_subset` public-API change. |
| I5 | resolved | Depth-semantics paragraph at line 142 defines `depth = 1` rationale ("the framing-level this call is about to read"). |
| I6 | resolved | Encoder-depth-invariant paragraph at line 229 documents reliance on upstream `TapTree::combine`. |
| I7 | resolved | N3-N7 (lines 319-323) all pin `leaf_index: Some(0)`; pre-table note at line 313 explains the LEFT-leaf positioning. |
| I8 | resolved | Encoder narrative line 231 + infrastructure-modifications line 369 note v0.4 single-leaf-non-zero-depth subsumption. |
| M1 | not addressed | BIP line numbers not verified against `bip/bip-mnemonic-descriptor.mediawiki` HEAD; minor and the controller can verify at BIP-edit time. |
| M2 | resolved | Line 189 reads `placeholder_map` (no `&`). |
| M3 | resolved | Line 95 reads `Cursor<'_>`. |
| M4 | resolved | Line 245 + CHANGELOG line 411 commit to annotating the variant `#[non_exhaustive]`. |
| M5 | resolved | RT4 test name (line 350) reads `t4_left_heavy_and_t5_right_heavy_emit_distinct_bytecodes`. |
| M6 | resolved (with I2) | Both §5 and §6 quote ≥638 floor / ≥640 realistic. |
| M7 | resolved | Comment block at lines 116-119 explains depth-0 seed unwinding. |
| M8 | resolved | Phase plan at lines 433-445 folds type-wiring + decoder + call-site updates into a single Phase 2; new Phase 3 inserted for dispatcher-message update. |

### C4 re-trace (depth-gate confirmation)

Helper called from `decode_tr_inner` with `depth=1` to read the root-of-script-tree byte:
- depth=1 peeks TapTree → consume → `1 > 128`? no → recurse depth=2.
- ... left-spine ...
- depth=128 peeks TapTree → consume (128th framing) → `128 > 128`? no → recurse depth=129.
- depth=129 peeks LEAF → leaf-branch fires; depth-0 seed is created; unwinding through 128 enclosing `TapTree::combine` calls post-increments depth → final miniscript-depth = 128. **Legal — matches BIP 341 max.**

129-framing case (H2):
- depth=128 consumes 128th TapTree → recurse depth=129.
- depth=129 peeks TapTree (129th) → consume → `129 > 128`? yes → REJECT.

The recursion-depth vs miniscript-depth distinction is correctly captured in the depth-semantics paragraph at line 142 ("leaves discovered at depth=129 end up at miniscript-depth 128"). H1 fixture text speaks in miniscript-depth (correct: 128). H2 fixture text speaks in recursion-depth (correct: 129 framings → gate fires at recursion-depth 129). **No residual off-by-one.**

### I1 re-count (FOLLOWUPS confirmation)

`design/FOLLOWUPS.md` HEAD "Open items" section status-line tally:

| # | id | status |
|---|---|---|
| 1 | p2-inline-key-tags | open |
| 2 | external-pr-1-hash-terminals | open |
| 3 | decoded-string-data-memory-microopt | open |
| 4 | phase-d-tap-leaf-wrapper-subset-clarification | open |
| 5 | phase-d-tap-miniscript-type-check-parity | open |
| 6 | cli-json-debug-formatted-enum-strings | open |
| 7 | v0-5-multi-leaf-taptree | open ← closes at v0.5.0 ship |
| 8 | legacy-pkh-permanent-exclusion | wont-fix |
| 9 | legacy-sh-multi-permanent-exclusion | wont-fix |
| 10 | legacy-sh-sortedmulti-permanent-exclusion | wont-fix |
| 11 | cargo-toml-crates-io-metadata-fields | open |
| 12 | phase-5-cli-wdm1-assertion-sweep | open (Phase 5 will close) |
| 13 | rename-workflow-broad-sed-enumeration-lesson | open |
| 14 | slip-0173-register-md-hrp | listed in Open items but annotated "resolved (PR filed)" — counted as Open per the file's structural placement (matches v0.4 spec convention for in-flight external items) |

→ **10 open + 3 wont-fix at v0.4.1 ship** ✓
→ **9 open + 3 wont-fix at v0.5.0 ship** (close 1 = `v0-5-multi-leaf-taptree`) ✓

Spec lines 7 and 429 match. **Confirmed.**

### Residual issues (all minor / cosmetic)

- **R1 — line 103 inline comment is misleading.** The code reads `cur.read_byte()?;                   // commit consume only after the depth gate path` immediately followed by the depth-gate check on line 104. The comment suggests consume-after-gate, but the actual order is consume-then-gate. The §3 narrative at line 147 explains the actual order correctly. Fix: change the inline comment to `// commit consume after Tag::from_byte succeeded; depth gate fires below` or drop the comment entirely. Cosmetic; does not affect correctness.

- **R2 — line 142 "RT and H1 fixtures cover the legal-128 boundary" is slightly loose.** No RT fixture (RT1-RT4) hits depth-128; only H1 does. Fix: reword to "H1 fixture (§5) covers the legal-128 boundary; H2 covers the 129 rejection." One-word edit.

### New issues introduced by revisions

None of substance. R1 and R2 above are minor wording slips introduced by the revision pass, both cosmetic.

### Recommendation

The spec is ready for user sign-off and the `writing-plans` handoff. R1 + R2 can be folded as part of the writing-plans pass (which will likely re-touch §3 anyway) or deferred to the implementation-side commit that introduces the actual code — they are not contract-affecting. M1 (BIP line-number drift) should be re-verified at BIP-edit time during Phase 7 of the release plan, not now.
