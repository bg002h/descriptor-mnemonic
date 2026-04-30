# v0.10.0 Phase 6 review (opus)

**Date:** 2026-04-29
**Commits:** `b5f00f9` (docs main) + `d9a9b1c` (controller followup fixes)
**Baseline:** `d26f891` (Phase 5)
**Reviewer:** opus-4.7 (1M context)
**Branch:** `feature/v0.10-per-at-n-paths`

## 1. Verdict

**CLEAN — proceed to Phase 7 (v0.10.0 release).**

The BIP draft updates are accurate, comprehensive, and faithful to the
spec. CHANGELOG and MIGRATION read well, the "Why a wire-format break?"
framing is appropriate, the sed snippet works as advertised on literal-
bool call sites, and the README scope update is concise. POLICY_BACKUP's
RecoveryHints slot moves cleanly from `0x36` to `0x37`. The controller's
followup commit (d9a9b1c) cleanly closes the four rustdoc warnings that
Phases 1–5 introduced (verified: `cargo doc --workspace --all-features
--no-deps` emits zero warnings) and files a well-written FOLLOWUPS entry
for the pre-existing v0.5→v0.6 stale tag references in BIP byte-layout
examples — correctly deferred to the v0.10.0.1 cleanup tier (those
references pre-date v0.10 by multiple releases; sweeping them inside a
v0.10 docs commit would conflate two unrelated cleanups).

All 714 workspace tests pass; clippy + fmt + rustdoc all clean.

No blocking issues. No INLINE-FIX items. One Minor / FOLLOWUPS-able
item logged below for v0.10.0.1 cleanup batch (joining the existing
BIP byte-layout-examples entry).

## 2. Scope reviewed

- Commits `b5f00f9` and `d9a9b1c` against baseline `d26f891`.
- Files inspected (full diffs read; resulting state spot-checked against
  spec invariants):
  - `bip/bip-mnemonic-descriptor.mediawiki` (+95 / -12): five new/updated
    subsections; full read of lines 290–445 for the v0.10 surface.
  - `CHANGELOG.md` (+127 / -0): new `[0.10.0]` section.
  - `MIGRATION.md` (+123 / -0): new v0.9.x → v0.10.0 section.
  - `README.md` (+3 / -3): scope-paragraph update.
  - `design/POLICY_BACKUP.md` (+3 / -2): RecoveryHints slot 0x36 → 0x37.
  - `crates/md-codec/src/bytecode/path.rs` (+4 / -3): rustdoc link
    fully-qualifications.
  - `crates/md-codec/src/policy.rs` (+2 / -1): rustdoc reword.
  - `design/FOLLOWUPS.md` (+9): new
    `bip-byte-layout-examples-stale-v0_6-renumber` entry.
  - `design/agent-reports/v0-10-phase-5-review.md` (+66, new file).
- Cross-references checked:
  - `design/SPEC_v0_10_per_at_N_paths.md` §1–§6 for wire-format claim
    fidelity (count semantics, dispatch logic, error variants, header
    byte values, `MAX_PATH_COMPONENTS = 10`, `RESERVED_MASK = 0x03`).
  - `crates/md-codec/src/bytecode/header.rs` `RESERVED_MASK` constant.
  - `crates/md-codec/src/bytecode/path.rs` `MAX_PATH_COMPONENTS = 10`,
    `MAX_ORIGIN_PATHS = 32`.
  - `crates/md-codec/src/bytecode/tag.rs` Placeholder=0x33,
    SharedPath=0x34, Fingerprints=0x35, OriginPaths=0x36.
  - All four FOLLOWUPS handles (`md-per-at-N-path-tag-allocation`,
    `v010-p3-tier-2-kiv-walk-deferred`, `cli-policy-id-fingerprint-flag`,
    `bip-byte-layout-examples-stale-v0_6-renumber`) — all four real
    entries in `design/FOLLOWUPS.md`, lines 97 / 718 / 762 / 771.
  - Test name shift `decode_path_round_trip_multi_byte_component_count`
    → `decode_path_round_trip_multi_byte_child_index` (Phase 2; current
    name lives at `path.rs:847`).
- Build/test verification:
  - `cargo doc --workspace --all-features --no-deps`: zero warnings.
  - `cargo test --workspace --all-features --no-fail-fast`: 714 tests
    pass (all green; no failures, no ignored).
  - Sed snippet hand-verified: `BytecodeHeader::new_v0(true)` →
    `BytecodeHeader::new_v0(true, false)`; `false` arg same shape.

## 3. Findings

### Finding 1 — MIGRATION test-rewrite note refers to a test name that has already been rewritten

- **Severity:** Minor / FOLLOWUPS-able.
- **Disposition:** Defer to v0.10.0.1 docs cleanup or no-op.
- **Description:** `MIGRATION.md` lines 96–110 ("Test rewrite — multi-byte
  LEB128 in the child-index dimension") describes the legacy
  `decode_path_round_trip_multi_byte_component_count` test name in past
  tense. This is fine for **consumer code** that copied the test
  pattern; the migration audience is downstream forks that did the
  same. The note is correct as-written for that audience: "Consumer
  code that copied this test pattern should rewrite analogously" — the
  test name is the locator a fork would use. The note is NOT misleading
  about md-codec's own state (the test was rewritten in Phase 2 and
  ships as `decode_path_round_trip_multi_byte_child_index`); but a
  reader who skims could conflate "must rewrite" with "is unfixed".
- **Why it matters (low):** ambiguous reading possible; cleaner phrasing
  would mark "(md-codec's own copy was rewritten in v0.10.0 Phase 2;
  the rewrite preserves the multi-byte LEB128 exercise)" so readers
  skimming the section know md-codec itself is not blocked.
- **Suggested fix (optional):** add a parenthetical to MIGRATION.md after
  the test-rewrite paragraph saying:
  ```
  (md-codec's own test was rewritten in v0.10.0 to
  `decode_path_round_trip_multi_byte_child_index`, exercising
  `m/16384` to surface a 2-byte LEB128 at the per-component level.)
  ```
- **Disposition:** Either leave as-is (the consumer-audience reading is
  the load-bearing one; the slight ambiguity costs the average reader
  ~0 confusion) or fold into the v0.10.0.1 docs cleanup batch alongside
  `bip-byte-layout-examples-stale-v0_6-renumber`. Logged below as a
  FOLLOWUPS candidate; not a blocker.

### Finding 2 — `MIGRATION.md` `Error::ReservedBitsSet` mask reference for pre-v0.10 decoders is technically `0x0B` (correct) but readers may double-take

- **Severity:** Minor / informational.
- **Disposition:** Acknowledge-only.
- **Description:** `MIGRATION.md` line 84 says: "Pre-v0.10 decoders
  reject with `Error::ReservedBitsSet { byte: 0x08 | 0x0C, mask:
  0x0B }` — intended forward-compat." The `mask: 0x0B` is correct for
  the pre-v0.10 reserved-mask perspective (the ReservedBitsSet error
  was raised by a v0.9 decoder, which DID see mask 0x0B). The CHANGELOG
  / BIP say `mask: 0x03` — also correct, for the v0.10 decoder
  perspective. This is **not** a contradiction; the two masks describe
  two different decoders rejecting the same byte for two different
  reasons (v0.9: bit 3 is reserved; v0.10: bit 3 is OriginPaths flag,
  but bit 1/0 is reserved). The MIGRATION mask is the correct "what
  pre-v0.10 decoders emit" form. No fix needed.

### Finding 3 — `MIGRATION.md` Mechanical sed for `BytecodeHeader::new_v0` covers literal-bool sites only; variable-bool sites flagged correctly

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:** Verified the sed snippet works as advertised on both
  literal `true` and literal `false` arguments. The MIGRATION explicitly
  notes that variable-bool sites need hand inspection (lines 56-58) and
  provides the grep one-liner to locate them. This is the right
  granularity — fully-automatic rewrite is unsafe (variable-bool sites
  may need contextual reasoning to choose `false` vs. a real
  `origin_paths` value).

### Finding 4 — BIP byte-layout-examples stale tag references (`0x32`, `0x33`-as-SharedPath) — correctly deferred

- **Severity:** N/A (already filed; deferral correct).
- **Disposition:** Acknowledge controller's filed FOLLOWUPS entry as
  well-written.
- **Description:** Verified the deferred references are real:
  - Line 549: `0x32 <index>` (Placeholder; should be `0x33`).
  - Line 579: full bytecode `04 33 03 35 02 ...` (should be `04 34 03
    35 02 ...` for SharedPath).
  - Line 589: `Tag::SharedPath (0x33)` annotation (should be `0x34`).
  - Lines 601, 603: `Tag::Placeholder (0x32)` annotations (should be
    `0x33`).
  - Line 549: `0x32 <index>` framing prose (should be `0x33`).
  Note the **second** byte-layout example in the BIP at lines 643–668
  (taproot) IS correct — it uses the v0.6+ `34 03` and `33 00` literals
  with correct `Tag::Placeholder (0x33)` annotations. So the stale-
  reference pattern is partial (one example correct, one stale, plus
  the framing prose stale).
  The controller's FOLLOWUPS entry
  (`bip-byte-layout-examples-stale-v0_6-renumber`, line 771) names the
  scope correctly, points at the rg-locator query, and tiers to
  `v0.10.0.1-cleanup` — exactly the right disposition.
- **Why deferred is right:** The stale references pre-date v0.10 by two
  releases (they're the v0.5→v0.6 sweep miss). Folding them into the
  v0.10 docs commit would (a) balloon the diff, (b) mix two unrelated
  doc-correction efforts in a single commit, and (c) tempt scope creep
  into a broader BIP-text audit that's outside Phase 6's plan. Filing
  separately at the v0.10.0.1 tier preserves diff focus.

### Finding 5 — `cargo doc` rustdoc warnings cleanly resolved

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:** Ran `cargo doc --workspace --all-features --no-deps`;
  zero warnings emitted. The four sites the controller commit fixes:
  - `bytecode/path.rs:78` — `[Error::PathComponentCountExceeded]` →
    `[crate::Error::PathComponentCountExceeded]` (proper crate-rooted
    path so the link resolves under the rustdoc lookup rules).
  - `bytecode/path.rs:218` — same fully-qualification on the
    `encode_declaration` Error link.
  - `bytecode/path.rs:318` — `pub MAX_ORIGIN_PATHS` doc previously linked
    to `decode_origin_paths`, which is `pub(crate)`. Rustdoc warns when
    a public item's docs link to a private item (private link target
    won't resolve in published docs). Fix: backticks-only, with explicit
    `pub(crate)` note. Right call.
  - `policy.rs:326` — `pub to_bytecode` doc previously linked to
    `placeholder_paths_in_index_order`, which is also private. Same fix
    pattern. Right call.
  All four fixes preserve the documentation intent; the link form is
  the only thing that changes.

### Finding 6 — Three controller-verified FOLLOWUPS handles confirmed real

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:** Spot-checked all three handles via `grep -n` on
  `design/FOLLOWUPS.md`:
  - `md-per-at-N-path-tag-allocation` — line 97 (currently open; v0.10
    closes this — see CHANGELOG "FOLLOWUPS closed" section).
  - `v010-p3-tier-2-kiv-walk-deferred` — line 718 (open;
    v0.11 follow-up per CHANGELOG).
  - `cli-policy-id-fingerprint-flag` — line 762 (open; v0.11 follow-up).
  All three exist in the open-items section. The CHANGELOG references
  align: line 124 closes the first; lines 130–135 defer the latter two
  to v0.11. Citations correct.

## 4. BIP draft assessment (section by section)

### 4a. Header table (lines 290–301)

- ✅ Bit 3 reframed as "OriginPaths flag (v0.10+)" with crisp prose
  describing the dispatch (1 = OriginPaths block at offset 1; 0 =
  SharedPath).
- ✅ "v0.x ≤ 0.9 this bit was reserved-must-be-zero" call-out.
- ✅ Cross-reference to "Per-`@N` path declaration" §.

### 4b. Valid header byte values (line 302)

- ✅ "`0x00`, `0x04`, `0x08`, `0x0C`" matches the spec §3 dispatch
  matrix and code (`RESERVED_MASK = 0x03`).
- ✅ Reserved-mask narrowing from `0x0B` to `0x03` documented.
- ✅ Forward-compat behavior (pre-v0.10 decoders reject via
  `ReservedBitsSet`) called out.

### 4c. Path declaration reframing (lines 304–311)

- ✅ Two-variant framing (SharedPath vs OriginPaths) presented cleanly.
- ✅ Mutual-exclusion language with `UnexpectedTag` rejection-form
  references the spec's chosen error variant (per spec F5 / §3
  "Path-decl dispatch"). Aligns with the code's `BytecodeErrorKind::
  UnexpectedTag` use-site.
- ✅ Example-driven (shared-path = original v0.x ≤ 0.9 form; OriginPaths
  = v0.10 multisig divergent-account case).

### 4d. Shared-path declaration format (lines 313–322)

- ✅ Stale `0x33` → `0x34` adjacent to the v0.10 insertions fixed
  inline (commit message documents this scope-limited fix; the rest
  remain as a separately-tiered cleanup).

### 4e. Component-count cap statement (line 378)

- ✅ States "MUST NOT exceed 10 components" matching `MAX_PATH_COMPONENTS
  = 10` constant.
- ✅ Names `Error::PathComponentCountExceeded { got, max: 10 }` matching
  the actual variant.
- ✅ Applies "uniformly to `Tag::SharedPath` and `Tag::OriginPaths`" —
  matches the implementation (both encode/decode paths share the cap).
- ✅ Defense-in-depth justification + cross-reference to mk1 SPEC §3.5.

### 4f. Per-`@N` path declaration subsection (lines 380–436)

- ✅ Block layout table accurate (offset 0 = `0x36` tag; offset 1 =
  `u8` count `1..=32`; offset 2..n = path-decls). Matches spec §2.
- ✅ Index-order semantics ("position in the list IS the index"; no
  per-entry index byte).
- ✅ "No deduplication" note matches the code's dense encoding.
- ✅ Encoder auto-detection rule ("emit `Tag::SharedPath` if all paths
  are equal, `Tag::OriginPaths` otherwise") matches the round-trip
  invariant: `encode(decode(encode(p))) == encode(p)`.
- ✅ Three rejection paths enumerated (structural / semantic / cursor
  exhaustion mid-list) — matches spec §3 + actual error variants.
- ✅ Example A (header `0x00`, shared-path) — bytes correct.
- ✅ Example B (header `0x0C`, divergent paths + fingerprints) —
  reproduces SPEC §2 byte sequence `36 03 05 05 FE 04 61 01 01 C9 01`
  exactly. The LEB128 derivation (`100' = 2*100+1 = 201` → `0xC9 0x01`
  two-byte encoding) is shown in-line, which is friendly to a reader.
- ✅ Example C (header `0x08`, divergent paths + no fingerprints) —
  matches SPEC §2 Example C byte-for-byte.
- ✅ "v0.10 valid header bytes are exactly: `0x00`, `0x04`, `0x08`,
  `0x0C`" enumeration.

### 4g. Authority precedence with MK subsection (lines 438–440)

- ✅ Three-sentence cross-reference framing per Q5/§5.1.
- ✅ Names MK as authoritative for xpub derivation; MD as descriptive
  for path layout.
- ✅ Defers normative text to MK BIP §"Authority precedence (MK ↔ MD
  path information)" (the right cross-reference).

### 4h. Tag table (line 526–528)

- ✅ Row added: `0x36 = OriginPaths` with disposition "Reclaimed in
  v0.10 from the previously reserved range; gated by header bit 3".
- ✅ Reserved range narrowed: "Tags `0x37`–`0xFF` are reserved" (line
  535; was `0x36`–`0xFF`).

### 4i. Backwards-Compatibility section (lines 1014–1020)

- ✅ "v0.x ≤ 0.9 ↔ v0.10 wire-format break (header bit 3 reclamation)"
  paragraph present with the correct narrative:
  - SharedPath-only encodings remain byte-identical;
  - OriginPaths encodings need v0.10+ decoders;
  - Pre-v0.10 decoders reject cleanly via `ReservedBitsSet` (intended
    forward-compat behavior).
- ✅ Closes a v0.x ≤ 0.9 silent path-divergence drop bug — framing is
  consistent with CHANGELOG.

### 4j. PolicyId types subsection (lines 705–722)

- ✅ Type 1 (`PolicyId`) = `SHA-256(canonical_bytecode)[0..16]`.
- ✅ Type 0 (`WalletInstanceId`) =
  `SHA-256(canonical_bytecode || canonical_xpub_serialization)[0..16]`.
- ✅ "Type 1 answers 'what shape of wallet is this?'; Type 0 answers
  'which specific wallet instance is this?'" — clean teaching framing.
- ✅ v0.10 effect on Type 1 noted: "Two policies with same script
  template but different per-cosigner accounts ... hash to different
  Type-1 PolicyIds in v0.10". Right normative point — v0.10's path-
  divergence honesty closes a v0.9 silent collision.
- ✅ "Users who computed PolicyIds on divergent-path policies under v0.x
  ≤ 0.9 SHOULD re-engrave under v0.10" — appropriate guidance.

### 4k. 12-word phrase engraving softening (lines 695, 768)

- ✅ "A Policy ID is a derived identifier MAY be engraved on a separate
  medium for offline cross-verification" (line 687) — softened from
  the prior "optionally engraved" language.
- ✅ "12-word PolicyId phrase MAY be engraved" (line 768) — explicit
  MAY-not-SHOULD.
- ✅ Fingerprint API cross-reference: "PolicyId fingerprint (top 4
  bytes ... rendered as 8 lowercase hex characters, parallel to BIP 32
  master-key fingerprints) is offered as an 8-character display form.
  The reference implementation exposes this as `PolicyId::fingerprint()
  -> [u8; 4]` (v0.10+)" — accurate.

## 5. CHANGELOG / MIGRATION / README / POLICY_BACKUP assessment

### CHANGELOG.md `[0.10.0]` section

- ✅ Top blurb names the headline FOLLOWUPS closure
  (`md-per-at-N-path-tag-allocation`).
- ✅ "Why a wire-format break?" callout (lines 17–32) — accurate framing:
  describes the v0.x ≤ 0.9 silent-flatten bug, the v0.10 fix
  (`Tag::OriginPaths`), what stays byte-identical, and the API-break
  cost (`new_v0(bool)` → `new_v0(bool, bool)`; `encode_path` Result).
- ✅ Added section enumerates all spec-required additions:
  `Tag::OriginPaths = 0x36`, header bit 3 reclaim, `MAX_PATH_COMPONENTS`,
  `decoded_origin_paths`, `EncodeOptions::origin_paths`,
  `EncodeOptions::with_origin_paths`, `PolicyId::fingerprint()`, BIP
  subsections, path-component-cap statement, MAY-engrave softening.
- ✅ Changed section enumerates `BytecodeHeader::new_v0(bool, bool)`
  signature break + `encode_path -> Result` break (with cross-
  references to MIGRATION).
- ✅ New error variants section lists all three:
  `OriginPathsCountTooLarge`, `OriginPathsCountMismatch`,
  `PathComponentCountExceeded` — matches code.
- ✅ "(`Error` is `#[non_exhaustive]`; adding variants is API-additive,
  not breaking.)" — correct framing for SemVer purposes.
- ✅ Wire format section explains valid header byte values + v0.x ≤ 0.9
  byte-stability + forward-compat rejection behavior.
- ✅ Generator family token note ("md-codec 0.9" → "md-codec 0.10")
  + corpus regen call-out + new positive/negative vector enumeration.
- ✅ FOLLOWUPS closed = `md-per-at-N-path-tag-allocation`.
- ✅ FOLLOWUPS deferred = `v010-p3-tier-2-kiv-walk-deferred` and
  `cli-policy-id-fingerprint-flag` — both real entries (verified §3
  Finding 6).
- ✅ MSRV unchanged (1.85).
- ✅ Style consistent with prior v0.7.x / v0.9.x entries (same
  Added/Changed/Wire-format/FOLLOWUPS section structure).

### MIGRATION.md v0.9.x → v0.10.0 section

- ✅ "Why a wire-format break?" framing brief; points at CHANGELOG for
  the full version (right scope split).
- ✅ Mechanical sed snippet hand-verified to work on literal-bool sites.
- ✅ Variable-bool grep one-liner provided for hand-inspection sites.
- ✅ Hand-rename items section covers `encode_path`/`encode_declaration`
  with concrete `?` and `.expect("...")` guidance.
- ✅ Wire format section explains bit-3 reclaim + byte-stability of
  v0.x ≤ 0.9 SharedPath encodings + pre-v0.10 decoder rejection behavior
  with the correct mask (`0x0B` from the v0.9 decoder's perspective).
- ✅ Test-rewrite note for the LEB128 dimension shift — accurate for
  the consumer-fork audience (Finding 1 is a minor phrasing nit; not a
  blocker).
- ✅ "What consumer code does NOT need to change" closing list — sets
  expectations correctly for the typical pre-v0.10 case.

### README.md scope update (lines 80–88)

- ✅ Bullet 2 reads "Shared paths ... AND per-`@N` divergent paths
  (one path per placeholder, in placeholder-index order) — v0.10+".
- ✅ The "covers single-key wallets ... typical Coldcard self-custody
  setups" sentence updated to mention "(including those where each
  cosigner derives from a distinct BIP 48 account)" — concrete +
  recognizable use-case.
- ✅ Final paragraph: "Foreign xpubs ... deferred to v1+. Per-`@N`
  divergent paths shipped in v0.10 (header bit 3 reclaimed;
  `Tag::OriginPaths = 0x36`); see CHANGELOG.md and MIGRATION.md".
  Concise; cross-references both target files.

### POLICY_BACKUP.md RecoveryHints relocation (lines 826–832)

- ✅ Verified by `grep -n '0x36|0x37|RecoveryHints'`: only two matches,
  both at lines 828–829 in the relocated form. The 0x36 reference is
  the contextual call-out ("the adjacent 0x36 slot was reclaimed in
  v0.10 as Tag::OriginPaths") — correct citation.
- ✅ The slot moved to `0x37`, parenthetical notes the relocation
  rationale ("the adjacent 0x36 slot was reclaimed").
- ✅ This is the right slot pick: the OriginPaths spec §"Q1" pre-
  flagged that "Tag::RecoveryHints slated for 0x37 in
  design/POLICY_BACKUP.md" — the controller commit and POLICY_BACKUP
  edit close that loop atomically with v0.10 ship.

## 6. Followup-fix commit (`d9a9b1c`) assessment

### 6a. Rustdoc warnings (4 warnings → 0)

- ✅ Verified via `cargo doc --workspace --all-features --no-deps`: zero
  warnings emitted.
- ✅ All four sites use the right fix pattern:
  - Public item linking to public item but the link target is in a
    different module: fully-qualify (path.rs:78, :218 →
    `crate::Error::PathComponentCountExceeded`).
  - Public item linking to `pub(crate)` item: replace link with plain
    backticks + an explanatory parenthetical (path.rs:318, policy.rs:326).
  Both patterns preserve the documentation intent; only the link form
  changes.
- ✅ Doctest (`cargo test --doc`) shows 7 passed in the doc-test runner
  including `policy_id::PolicyId::fingerprint`.

### 6b. New FOLLOWUPS entry: `bip-byte-layout-examples-stale-v0_6-renumber`

- ✅ Stable handle (slug-form, project convention).
- ✅ "Surfaced" line points at the Phase 6 implementer + commit hash.
- ✅ "Where" gives both the file and the rg-locator query.
- ✅ "What" is concrete (sweep `0x32` → `0x33` for Placeholder; `0x33`
  → `0x34` for SharedPath).
- ✅ "Why deferred" gives a clean reason (pre-dates v0.10 by two
  releases; sweep would balloon docs commit; v0.10.0.1 is a natural
  cleanup tier).
- ✅ "Status: open" + "Tier: v0.10.0.1-cleanup" — correctly tiered.
- ✅ Spot-checked the entry's claim by `grep` on the BIP file — the
  stale references are at lines 549, 579, 589, 601, 603 (and
  framing prose 549) — exactly as the entry describes.

### 6c. Verifications of three controller-checked handles

All three exist (verified §3 Finding 6).

## 7. Recommended action

**Proceed to Phase 7 (v0.10.0 release).**

No inline fixes required. Phase 6 cleanly closes the docs-track work
needed to ship v0.10.0:

- BIP draft has the five required v0.10 sections + tag-table row +
  bytecode-header table update + Backwards-Compatibility paragraph +
  PolicyId types teaching subsection + MAY-engrave softening.
- CHANGELOG has the "Why a wire-format break?" framing + Added/Changed/
  Wire-format/FOLLOWUPS sections in the project's established style.
- MIGRATION has the sed snippet + hand-rename items + wire-format
  description + consumer-doesn't-need-to-change closing list.
- README scope updated.
- POLICY_BACKUP RecoveryHints atomically renumbered.
- Rustdoc clean.
- 714 tests passing.

Phase 7 (release) can proceed with these gates already satisfied:
- Wire format claims accurate against `RESERVED_MASK = 0x03`,
  `MAX_PATH_COMPONENTS = 10`, `MAX_ORIGIN_PATHS = 32`,
  `Tag::OriginPaths = 0x36` constants in code.
- All three v0.10 spec-required FOLLOWUPS handles real and
  appropriately marked (closed / deferred / deferred).
- One newly-filed FOLLOWUPS entry well-tiered for v0.10.0.1 cleanup.

### Optional polish (non-blocking; v0.10.0.1 candidates)

If a v0.10.0.1 cleanup batch ships, fold these in:

1. The `bip-byte-layout-examples-stale-v0_6-renumber` entry already
   filed.
2. Optional: add a parenthetical to `MIGRATION.md` test-rewrite note
   making explicit that md-codec's own copy of the test was rewritten
   in Phase 2 (Finding 1). Low-priority phrasing nit; not a blocker.

### FOLLOWUPS handling

**No new entries from this review beyond what the controller already
filed.** Finding 1 is small enough that I do not recommend a separate
FOLLOWUPS entry; it can fold into the v0.10.0.1 cleanup batch alongside
`bip-byte-layout-examples-stale-v0_6-renumber` if a maintainer wants
to tighten the MIGRATION phrasing, or stay as-is.

### Phase 7 entry conditions

All Phase 6 deliverables shipped and gated. Phase 7 can begin with the
release-tag + version-bump + crate publish workflow. No Phase-6-derived
blockers.
