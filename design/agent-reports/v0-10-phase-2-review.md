# v0.10.0 Phase 2 review (opus)

**Date:** 2026-04-29
**Commit:** `1936b19` (`feat(v0.10-p2): MAX_PATH_COMPONENTS cap + encode/decode_origin_paths`)
**Baseline:** `3b38242` (Phase 1)
**Reviewer:** opus-4.7 (1M context)
**Branch:** `feature/v0.10-per-at-n-paths`

## Verdict

INLINE-FIX — one minor wording fix recommended before P3, otherwise CLEAN.

Phase 2 implements every step (2.1–2.10) the plan specifies, the F5/F6/F15
findings are addressed verbatim, the cap is enforced uniformly across
`Tag::SharedPath` and `Tag::OriginPaths` per Q8, and all 9 new tests + 1
rewrite pass cleanly with no new clippy/fmt regressions. The implementer
flagged the Phase-1 FOLLOWUPS entry `v010-p1-origin-paths-count-too-large-zero-message`
for reviewer judgment; my recommendation is to fold the fix in here (one-line
`#[error(...)]` template change at `error.rs:524`) since Phase 2 is the
natural surfacing point and the change has zero blast radius. Two minor
items also worth noting: implementer's reported test count (692/1)
overstates reality (actual ~621 passing + 1 expected-fail), and the
Example B round-trip test pins only the prefix bytes (5 of 11) rather than
the full spec-pinned sequence.

## Scope reviewed

- Commit `1936b19` against baseline `3b38242`.
- Files inspected (full diffs read):
  - `crates/md-codec/src/bytecode/path.rs` (+406 / −48 effective)
  - `crates/md-codec/src/policy.rs` (1 line)
- Cross-references checked:
  - `design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md` Phase 2 steps 2.1–2.11.
  - `design/SPEC_v0_10_per_at_N_paths.md` §2 Example B (line 157).
  - Phase 1 review report (`design/agent-reports/v0-10-phase-1-review.md`).
  - Phase 1 FOLLOWUPS entry `v010-p1-origin-paths-count-too-large-zero-message`
    (FOLLOWUPS.md:672–681).
- Build/test verification:
  - `cargo build --workspace --all-features`: clean.
  - `cargo clippy --workspace --all-features --all-targets`: clean (no warnings).
  - `cargo fmt --check --all`: clean.
  - `cargo test --workspace --all-features`: 614 passing + 1 expected-fail
    (`every_error_variant_has_a_rejects_test_in_conformance` — Phase 4
    will land the rejection vectors).
  - `cargo test --workspace --all-features --doc`: 7 passing (6 + 1 across
    crates).
  - Path-module subset (`bytecode::path`): 44/44 passing — 9 new tests +
    1 rewrite all green.
- Call-site sweep:
  - `rg -n '\bencode_path\(' crates/md-codec/src/ crates/md-codec/tests/`
    → 18 sites; all use `?` (production) or `.expect("...")` (tests). Zero
    stale infallible calls.
  - `rg -n '\bencode_declaration\(' …` → all sites updated symmetrically.

## Findings

### 1 — `OriginPathsCountTooLarge` Display message awkward when `count = 0` (Phase-1 FOLLOWUPS now surfacing)

- **Severity:** Minor
- **Disposition:** **INLINE-FIX in this phase** (recommended)
- **Description:** Phase 2 introduced
  `decode_origin_paths_rejects_count_zero` which now actively triggers
  `BytecodeErrorKind::OriginPathsCountTooLarge { count: 0, max: 32 }`. The
  rendered Display message under the current
  `#[error("OriginPaths count {count} exceeds maximum {max}")]` template
  reads `"OriginPaths count 0 exceeds maximum 32"`, which is grammatical
  but semantically weak — 0 doesn't arithmetically "exceed" 32; the value
  is structurally invalid as the lower-bound violation.

  Phase 1's reviewer note (Finding 2 in `v0-10-phase-1-review.md`) and the
  FOLLOWUPS entry both explicitly flagged Phase 2 as the natural revisit
  point: "Phase 2 introduces the actual `decode_origin_paths` callsite
  that surfaces this message; that's a natural revisit point."

  Recommended template per the Phase 1 suggestion (and FOLLOWUPS entry):
  `"OriginPaths count {count} is out of range (must be 1..={max})"`. This
  reads cleanly for both bounds:
  - count=0 → "OriginPaths count 0 is out of range (must be 1..=32)"
  - count=33 → "OriginPaths count 33 is out of range (must be 1..=32)"

  Why fold in now (vs. defer):
  1. Phase 1 explicitly flagged Phase 2 as the natural revisit point.
  2. One-line `#[error(...)]` template change.
  3. Zero blast radius: Display strings aren't wire format and aren't
     asserted in any test (the existing `decode_origin_paths_rejects_*`
     tests assert via `matches!` on the variant shape, not the Display
     output).
  4. Pre-v1.0 break freedom (per `feedback_pre_v1_break_freedom`) makes
     this strictly an improvement-of-defaults change, no migration cost.
  5. Deferring leaves a known-suboptimal user-facing message in the
     v0.10.0 release surface.

- **Suggested fix (exact edit):**
  - File: `crates/md-codec/src/error.rs`
  - Line: 524
  - Before:
    ```rust
    #[error("OriginPaths count {count} exceeds maximum {max}")]
    ```
  - After:
    ```rust
    #[error("OriginPaths count {count} is out of range (must be 1..={max})")]
    ```
  - After landing, mark the FOLLOWUPS entry
    `v010-p1-origin-paths-count-too-large-zero-message` as
    `resolved <NEW_COMMIT>` and move it into the Resolved section.

### 2 — Implementer's reported test count (692/1) overstates reality

- **Severity:** Minor
- **Disposition:** Acknowledge-only (no code change needed)
- **Description:** The hand-off reports "692/1." Actual workspace-wide
  measurement: 614 passing + 1 expected-fail in non-doc tests, plus 7
  doc tests = **621 passing + 1 expected-fail**. Trend matches plan
  expectation: P1 was 612 + 1; P2 net delta is +9 tests (9 new + 1 renamed
  rewrite, with the renamed test counted as -1 + 1 = net 0 contribution),
  so 621 is consistent.

  The Phase 1 reviewer flagged the same reporting drift (Finding 1 of P1
  review). Same recommendation here: implementer's tally process is
  systematically over-counting — useful to fix the methodology before
  Phase 3 hand-off (consider adopting `cargo test ... 2>&1 | grep
  '^test result' | awk '{sum+=$4} END {print sum}'` or equivalent).

- **Suggested fix:** None for code. Note for Phase 3 hand-off: have the
  implementer report tallies via a deterministic script rather than
  best-effort summation.

### 3 — Example B round-trip test pins only the prefix (5/11 bytes)

- **Severity:** Minor
- **Disposition:** Acknowledge-only OR FOLLOWUPS-able
- **Description:** `encode_origin_paths_round_trip_three_paths`
  (path.rs:1196–1227) asserts the first five bytes of the encoded output
  (`bytes[0..=4] = [0x36, 0x03, 0x05, 0x05, 0xFE]`) but does not pin the
  remaining 6 bytes (`04 61 01 01 C9 01`) that comprise the explicit-form
  third path declaration. Spec §2 line 157 pins the full 11-byte sequence
  `36 03 05 05 FE 04 61 01 01 C9 01`. The eq-comparison after decode
  (`assert_eq!(recovered, paths)`) provides indirect verification (an
  encoder bug in the component bytes would cause a decode mismatch), but
  doesn't catch byte-level drift in the explicit-path encoding (e.g., a
  mis-LEB128'd component count that happens to round-trip).
- **Suggested fix:** Strengthen the assertion to pin the full 11-byte
  sequence per spec §2:
  ```rust
  assert_eq!(
      bytes,
      vec![0x36, 0x03, 0x05, 0x05, 0xFE, 0x04, 0x61, 0x01, 0x01, 0xC9, 0x01],
      "must match spec §2 Example B byte sequence"
  );
  ```
  This is strictly an additive strengthening — keep the existing
  prefix-byte asserts as documentation-of-layout, or replace them with
  the single full-sequence assert. Defer-able to FOLLOWUPS as a v0.10
  nice-to-have if not folded in with Finding 1.

### 4 — `MAX_ORIGIN_PATHS = 32` const addition is sensible

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** The plan didn't explicitly call for a
  `MAX_ORIGIN_PATHS` const — it specified `> 32` checks inline. The
  implementer added `pub const MAX_ORIGIN_PATHS: u8 = 32` at path.rs:319
  and used it as the single source of truth in both
  `encode_origin_paths` (line 345) and `decode_origin_paths` (lines 401,
  406). This is good code hygiene: the magic number is named, the cap
  has a docstring citing BIP 388, and Phase 3+ (which will need to
  cross-check against `placeholder_count`) will have a stable symbol
  to import. The choice of `u8` is correct (the wire byte is a u8, and
  32 fits trivially). Not spec drift; consistent with the existing
  `MAX_PATH_COMPONENTS` pattern. Approve.

### 5 — `decode_origin_paths` `#[allow(dead_code)]` deferral correctly documented

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** The `#[allow(dead_code)]` at path.rs:393 has an
  accompanying 5-line comment (lines 388–392) explaining the deferral
  ("standalone helper landed in Phase 2; Phase 3 wires it into
  `WalletPolicy::from_bytecode`"). The annotation is on the
  `pub(crate)` item only; the test module exercises the function so the
  test build sees it as live. Acceptable.

### 6 — `encode_declaration` signature change is correctly symmetric

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** `encode_declaration(&DerivationPath) -> Vec<u8>`
  becomes `Result<Vec<u8>, Error>` (path.rs:220). This is forced by the
  F6 propagation: `encode_declaration` calls `encode_path` with `?`
  (line 224). The change is mechanically required, the rustdoc Errors
  section is added, and the single non-test call site (`policy.rs:421`)
  is updated with `?` propagation. Symmetric and correct.

### 7 — Cursor-sentinel test verifies exact consumption

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** `decode_origin_paths_consumes_exact_bytes`
  (path.rs:1295–1308) uses two dictionary paths (`m/48'/0'/0'/2'` ×2 →
  encoded as `[0x36, 0x02, 0x05, 0x05]`), appends a sentinel `0xFF`, and
  passes `&bytes[1..]` (skipping the Tag byte). Decoder consumes
  count(1) + 2 dictionary indicators(1+1) = 3 bytes; cursor is then
  positioned at offset 3 in the slice, which is the sentinel `0xFF`.
  The `cur.read_byte() == 0xFF` assert fires on under-read (cursor at
  index ≤2) AND over-read (cursor past sentinel → `read_byte()` would
  fail with `UnexpectedEnd`, also failing the test correctly). Both
  off-by-one directions caught. Solid pattern.

### 8 — F5/F6/F15 plan-level findings all addressed verbatim

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:**
  - **F5** (cap-check ordering): cap check at path.rs:157 is BEFORE the
    component-decode loop at path.rs:163. The
    `decode_path_cap_check_fires_before_component_decode` test
    (path.rs:1182–1193) actively pins this with synthesized count=11 and
    NO component bytes — must surface `PathComponentCountExceeded`, not
    `UnexpectedEnd`. Test passes. The F5 priority is locked.
  - **F6** (encode_path infallible→fallible): `encode_path` signature is
    now `Result<Vec<u8>, Error>` (path.rs:83). Mechanical updates at all
    18 call sites with `?` (production: 2 sites in path.rs lines
    224/357, plus policy.rs:421) or `.expect("...within cap")` (tests:
    15 sites). No stale call sites; build clean. The signature break is
    correctly the cleaner-design choice (per
    `feedback_pre_v1_break_freedom`).
  - **F15** (rewrite multi-byte component-count test):
    `decode_path_round_trip_multi_byte_component_count` is gone; replaced
    by `decode_path_round_trip_multi_byte_child_index` (path.rs:849–878)
    using `m/16384` (encoded value `2*16384 = 32768 = 0x8000`, requiring
    3-byte LEB128 `[0x80, 0x80, 0x02]`). The test pins all 5 expected
    bytes individually and round-trips through `decode_all`. Stronger
    than the original 128-component test in that it now also pins the
    explicit count=1 byte position.

## Spec-compliance verification

| Step | Status | Notes |
|---|---|---|
| 2.1: Write failing tests for MAX_PATH_COMPONENTS cap | ✅ | All 4 plan-specified tests present at path.rs:1126–1194; names + assertions match plan verbatim. |
| 2.2: Run tests; verify they fail (TDD) | ✅ | (Implementer's word; can't retroactively verify pre-impl, but the impl matches plan's pass-bar.) |
| 2.3: Add `MAX_PATH_COMPONENTS` cap enforcement | ✅ | Const at path.rs:42 with rustdoc citing BIP §3.5 + Q8; cap enforced in `encode_path` (line 90) and `decode_path` (line 157). F6 break taken; symmetric `encode_declaration` change at line 220. |
| 2.4: Run cap tests; verify they pass | ✅ | All 4 cap tests pass; verified via `cargo test --package md-codec bytecode::path`. |
| 2.5: Address `decode_path_round_trip_multi_byte_component_count` (F15) | ✅ | Test renamed `_component_count → _child_index`, rewritten to use `m/16384` exercising 3-byte LEB128 in the child-index dimension. Old test removed (not legacy-disabled — clean replacement). |
| 2.6: Write failing tests for `encode/decode_origin_paths` | ✅ | All 4 plan-specified tests present at path.rs:1196–1283; names + assertions match plan verbatim. |
| 2.7: Implement `encode_origin_paths` + `decode_origin_paths` | ✅ | Both functions present at path.rs:342 / path.rs:394 with full rustdoc Errors sections; defense-in-depth count check; correct prefix layout `[Tag::OriginPaths, count, …path-decls…]`; count range `1..=32` enforced; `decode_origin_paths` correctly pub(crate) with `#[allow(dead_code)]` until Phase 3. |
| 2.8: Run all path-module tests; verify pass | ✅ | 44/44 path-module tests pass. |
| 2.9: Add cursor-sentinel + asymmetric-byte-fill defensive test | ✅ | `decode_origin_paths_consumes_exact_bytes` at path.rs:1287–1308; correctly asserts cursor positions at sentinel after decode. |
| 2.10: Commit Phase 2 | ✅ | Commit `1936b19` lands with detailed message; matches plan template (with one prose note: message says "Phase 4 wires them" but the plan actually wires them in Phase 3 — mild commit-message drift, no code impact). |

## Test coverage assessment

- **Cap enforcement:** Strong. Both encode-side (`_rejects_11_components`)
  and decode-side (`_rejects_11_components_in_explicit_form`) caps are
  asserted. The boundary case (`_accepts_10_components`) pins inclusivity
  at the cap. F5 priority pin
  (`_cap_check_fires_before_component_decode`) is genuinely defensive
  against future refactor risk — a regression that moved the cap check
  after the decode loop would surface as `UnexpectedEnd` and the test
  would correctly fail with a descriptive message.
- **OriginPaths helpers:** Strong. Round-trip with mixed dictionary +
  explicit paths; both bounds rejected (count=0 AND count=33);
  truncation surfaces `UnexpectedEnd`; cursor-sentinel pins exact
  consumption. The only minor gap is the partial spec-byte pinning in
  the round-trip test (Finding 3) — the prefix is asserted but the
  explicit-path tail bytes (5–10) aren't byte-pinned. Indirect
  verification via decode round-trip is sufficient for correctness but
  weaker than spec-pinning would be.
- **Multi-byte LEB128 round-trip:** Stronger than pre-Phase-2. The new
  `_child_index` test pins all 5 expected bytes individually
  (`0xFE, 0x01, 0x80, 0x80, 0x02`), proving 3-byte LEB128 in the
  child-index dimension. The pre-Phase-2 component-count variant pinned
  3 bytes (count LSB + count MSB + total length); the rewrite pins more
  positions overall.
- **API-break propagation:** Strong via the call-site sweep. 18
  `encode_path` call sites + 11 `encode_declaration` call sites all
  updated. Build is warning-free, so no missed sites or partially-applied
  `?` propagation.
- **Coverage gaps acknowledged:** the plan-out-of-scope items (Phase 3
  policy-layer integration with `decoded_origin_paths`; Phase 4
  conformance vectors for `OriginPathsCountMismatch` and
  `PathComponentCountExceeded` rejection paths) are correctly deferred —
  the conformance gate fails on exactly those two missing rejection
  tests, which is the contract handed off to Phase 4.

## Decision on Phase-1 FOLLOWUPS fold-in vs defer

**Decision: INLINE-FIX in this phase.**

The Phase-1 FOLLOWUPS entry
`v010-p1-origin-paths-count-too-large-zero-message` was filed in P1 with
explicit reviewer guidance that Phase 2 was the natural revisit point.
Phase 2 has now landed the active call site
(`decode_origin_paths_rejects_count_zero`) that triggers the
known-awkward Display rendering. The fix is a one-line `#[error(...)]`
template change at `crates/md-codec/src/error.rs:524`:

- Before: `#[error("OriginPaths count {count} exceeds maximum {max}")]`
- After: `#[error("OriginPaths count {count} is out of range (must be 1..={max})")]`

Why fold in now:
1. Phase 1 reviewer explicitly flagged Phase 2 as the revisit point.
2. One-line, zero-blast-radius change. Display strings aren't wire
   format; no test asserts the Display output (only the variant shape
   via `matches!`).
3. `feedback_pre_v1_break_freedom` memory: pre-v1.0 the project has
   freedom to make cleaner-default changes without weighing
   migration costs.
4. Deferring leaves a known-suboptimal error message in the v0.10.0
   release. With v0.10.0 likely tagged at end of this multi-phase
   sequence, this is the last natural opportunity to inline-fix
   without it becoming "next-release" baggage.

After the fix lands, the controller should mark the FOLLOWUPS entry as
`resolved <NEW_COMMIT>` and move it to the Resolved section of
FOLLOWUPS.md.

## Recommended action

**INLINE-FIX, then proceed to Phase 3.**

### Required edit

- File: `/scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/error.rs`
- Line: 524
- Change:
  ```diff
  -    #[error("OriginPaths count {count} exceeds maximum {max}")]
  +    #[error("OriginPaths count {count} is out of range (must be 1..={max})")]
  ```
- Verification: `cargo test --workspace --all-features` should still
  pass with 614 + 1 expected-fail (no test asserts the Display string
  textually, so the variant-shape `matches!` arms remain green).
- After: mark FOLLOWUPS entry
  `v010-p1-origin-paths-count-too-large-zero-message` as resolved with
  the new commit SHA and move to Resolved section.

### Optional edit (if desired)

Strengthen `encode_origin_paths_round_trip_three_paths` (path.rs:1196)
to pin the full 11-byte spec §2 Example B sequence. Defer-able to
FOLLOWUPS as a v0.10 nice-to-have if the controller prefers to keep
this commit minimal.

### Phase 3 entry conditions

After the inline fix lands and FOLLOWUPS is updated, Phase 3 may
proceed without further re-review. The implementer should:
- Carry forward the test-count-reporting hygiene note from Phase 1
  (use a deterministic tally script).
- Wire `decode_origin_paths` into `WalletPolicy::from_bytecode` per
  Phase 3 plan; the `#[allow(dead_code)]` annotation can come off in the
  same commit.
- Implement the 4-tier precedence chain for path resolution per Phase 3
  plan steps 3.1–3.10.
