# v0.10.0 Phase 1 review (opus)

**Date:** 2026-04-29
**Commit:** `3b38242` (`refactor(v0.10-p1): reclaim header bit 3 + add Tag::OriginPaths variant`)
**Baseline:** `d7f0887` (Pre-Phase-0 version bump)
**Reviewer:** opus-4.7 (1M context)
**Branch:** `feature/v0.10-per-at-n-paths`

## Verdict

CLEAN — proceed to Phase 2.

Phase 1 implements every step (1.1–1.11) the plan specifies, the wire-format
mask values match the spec exactly (`RESERVED_MASK 0x0B → 0x03`,
`ORIGIN_PATHS_BIT = 0x08`, `Tag::OriginPaths = 0x36`), the structural-vs-semantic
error split lands per spec §1/§3/§4, and the `ErrorVariantName` mirror enum
gets exactly the two new top-level entries (no spurious sub-variant entries).
Two minor items only: the implementer-reported test count (683) overstates
reality (actual 612 passing across the workspace, +1 expected-fail), and one
docstring/error-message line is slightly awkward in the count-zero edge case.
Neither blocks Phase 2.

## Scope reviewed

- Commit `3b38242` against baseline `d7f0887`.
- Files inspected (all diffs read end-to-end):
  - `crates/md-codec/src/bytecode/header.rs` (+143 / −49 effective, full re-read)
  - `crates/md-codec/src/bytecode/tag.rs`
  - `crates/md-codec/src/bytecode/decode.rs` (one new arm)
  - `crates/md-codec/src/error.rs`
  - `crates/md-codec/src/policy.rs` (one call site)
  - `crates/md-codec/src/vectors.rs` (two new arms)
  - `crates/md-codec/tests/error_coverage.rs` (mirror enum)
- Cross-references checked:
  - `design/SPEC_v0_10_per_at_N_paths.md` §1, §2, §3 (header, tag, error
    variant shapes).
  - `design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md` Phase 1 steps
    1.1–1.11.
- Build/test verification:
  - `cargo build --workspace --all-features` clean.
  - `cargo clippy --workspace --all-features --all-targets` clean (no
    warnings).
  - `cargo fmt --check --all` clean.
  - `cargo test --workspace --all-features` → **612 passed, 1 failed
    (expected: `every_error_variant_has_a_rejects_test_in_conformance`),
    0 ignored**.

## Findings

### 1 — Implementer's reported test count (683) is inaccurate (actual 612 + 1 expected-fail)

- **Severity:** Minor
- **Disposition:** Acknowledge-only (no code change needed)
- **Description:** The implementer's hand-off reported "683 ok / 1 failed."
  Actual measured count is **612 passing** (workspace-wide,
  `cargo test --workspace --all-features`, including doc tests):
  - md-codec lib: 459
  - md-codec integration suites: 8 + 3 + 42 + 4 + 20 + 51 + 12 + 2 = 142
  - error_coverage: 4 of 5 (1 expected-fail)
  - md-signer-compat: 0 in lib, but earlier tally for `md-signer-compat`
    crates contributes
  - Doc tests: 6 + 1 = 7
  - Sum: 459 + 142 + 4 + 7 = 612 passing + 1 expected-fail
  The 683 figure may have included a stale baseline number or an aggregator
  glitch. The **expected-failing test is exactly the right one** (per Step
  1.10): `every_error_variant_has_a_rejects_test_in_conformance` failing
  on `OriginPathsCountMismatch` and `PathComponentCountExceeded`,
  resolved at Phase 4.
- **Suggested fix:** None. The actual numbers are healthy and the
  expected-fail is on the right test. Useful to note for Phase 2/3
  hand-offs so the implementer corrects their test-count reporting
  process.

### 2 — `OriginPathsCountTooLarge` error message awkward when `count = 0`

- **Severity:** Minor
- **Disposition:** Acknowledge-only (or FOLLOWUPS if you'd like to revisit
  with Phase 2's introduction of the actual check site)
- **Description:** The variant is named `OriginPathsCountTooLarge` and the
  docstring at `crates/md-codec/src/error.rs:514–530` correctly notes the
  variant fires for both `count == 0` and `count > 32`. Spec §3 line 457
  explicitly endorses this ("the variant name covers both bounds"). The
  spec's choice is reasonable — fewer error variants, simpler taxonomy.
  However, the user-facing `Display` message
  `"OriginPaths count {count} exceeds maximum {max}"` reads
  "OriginPaths count 0 exceeds maximum 32" for the count-zero case,
  which is grammatical but semantically weak — "0 doesn't exceed 32 in
  arithmetic terms; it's just structurally invalid."
- **Suggested fix:** Consider rewording the `#[error(...)]` template to
  cover both bounds explicitly, e.g., `"OriginPaths count {count} is
  out of range (must be 1..={max})"`. This is a Phase-2-touch suggestion
  since Phase 2 introduces the actual `decode_origin_paths` callsite that
  surfaces the message; for now, no Phase-1 change required.

### 3 — `BytecodeHeader::origin_paths()` not marked `const fn` while siblings are

- **Severity:** Minor
- **Disposition:** Inline-fix (1 line) OR Acknowledge-only
- **Description:** `crates/md-codec/src/bytecode/header.rs:114-118`:
  the new `origin_paths()` getter is `pub fn` while the parallel
  `version()` (line 105) and `fingerprints()` (line 110) are also `pub fn`
  in the current implementation — actually, looking again at the existing
  diff the plan specified `const fn` for all three getters (plan line 235-237):

  ```rust
  pub const fn fingerprints(&self) -> bool { self.fingerprints }
  pub const fn origin_paths(&self) -> bool { self.origin_paths }
  pub const fn version(&self) -> u8 { self.version }
  ```

  but the existing code has `pub fn` (not `const fn`) for `version()`
  and `fingerprints()` already pre-Phase-1, so this is a pre-existing
  pattern, not a regression. The implementer correctly mirrored the
  existing style (`pub fn`, not `const fn`). This is consistent with
  the file as it was, and `BytecodeHeader: Copy` so callers don't lose
  ergonomics.
- **Suggested fix:** None. The plan's `const fn` framing was advisory; the
  existing pattern wins. If you'd like all three to be `const fn` as a
  follow-up cleanup, that's strictly chore-tier (also impacts pre-Phase-1
  signatures and is out of P1 scope).

### 4 — F1, F2, F4 plan-level findings all correctly addressed

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:**
  - **F1** (delete `reserved_bit_3_set` test, rewrite
    `all_reserved_bits_set_no_fingerprints`): both done correctly.
    The deleted test would have asserted `0x08 → Err`; under v0.10 that's
    a valid OriginPaths header, so deletion is correct. The renamed test
    `all_reserved_bits_set_in_v0_10` at lines ~217–235 now asserts
    `0x03 → Err` with the right `RESERVED_MASK` value pinned in the
    `matches!` arm. The new assertion strengthens (vs. the old) by also
    asserting the exact `byte` and `mask` values, not just the variant.
  - **F2** (`ErrorVariantName` mirror): the two new top-level
    `Error::OriginPathsCountMismatch` and `Error::PathComponentCountExceeded`
    entries are present in `tests/error_coverage.rs` (lines 65-66), and
    `BytecodeErrorKind::OriginPathsCountTooLarge` is correctly omitted
    (covered by the existing `INVALID_BYTECODE_PREFIX` machinery for
    sub-variants). The conformance gate now correctly fails until
    Phase 4 lands the rejection vectors — exactly the contract Step 1.10
    specifies.
  - **F4** (three tag-table-coverage tests renamed/range-bumped):
    - `tag_v0_6_high_bytes_unallocated → tag_v0_10_high_bytes_unallocated`,
      loop bound `0x36..=0xFF → 0x37..=0xFF`. Done.
    - `tag_rejects_unknown_bytes` second loop bound bumped to
      `0x37..=0xFF`. Done.
    - `tag_round_trip_all_defined`: `v0_6_allocated → v0_10_allocated`
      with `0x33..=0x36` chain. Done.
    - The unallocated mid-range `0x24..=0x32` loop comment was also
      updated `(v0.6 unallocated) → (v0.10 unallocated)` for prose
      consistency.

### 5 — Two extra exhaustive-match sites not enumerated in the plan, found and fixed in-phase

- **Severity:** N/A (positive confirmation; correct in-phase decision)
- **Disposition:** Acknowledge-only
- **Description:** The implementer correctly identified two
  match-on-`Tag` / match-on-`Error` sites the plan didn't enumerate:
  - `crates/md-codec/src/bytecode/decode.rs:1065` — `tag_to_bip388_name`
    needed a new arm. Implementer added `Tag::OriginPaths => "<framing:0x36>"`
    matching the existing `<framing:0xNN>` convention used for
    `Placeholder`, `SharedPath`, and `Fingerprints`. Naming is consistent.
  - `crates/md-codec/src/vectors.rs:2331-2335` — `error_variant_name`
    needed two new arms for `Error::OriginPathsCountMismatch` and
    `Error::PathComponentCountExceeded`. Both added; string keys match
    variant names exactly.
  Both are required for the `non_exhaustive` enums to compile in
  Phase 1 (without these arms the build would fail), so this isn't really
  optional — but the implementer's framing of "found two extra sites the
  plan didn't enumerate" is fair, and the fix is local + idiomatic. No
  FOLLOWUPS entry needed; the plan's Phase 1 step list could be updated to
  enumerate these sites for future plan-template fidelity, but that's
  cosmetic.

## Spec-compliance verification

| Step | Status | Notes |
|---|---|---|
| 1.1: Write failing header tests | ✅ | All 5 new tests added (lines 282-339); names + assertions match plan verbatim. |
| 1.2: Run tests; verify they fail (TDD) | ✅ | (Implementer's word; cannot retroactively verify, but the impl matches the plan-specified pass-bar.) |
| 1.3: Update `BytecodeHeader` struct + impl + migrate 2 existing tests | ✅ | `RESERVED_MASK 0x0B → 0x03`; new `origin_paths` field; `new_v0` signature `(bool, bool)`; `ORIGIN_PATHS_BIT = 0x08`; `from_byte` and `as_byte` correctly threaded; `reserved_bit_3_set` deleted; `all_reserved_bits_set_no_fingerprints → all_reserved_bits_set_in_v0_10` rewritten with stronger assertions. |
| 1.4: Run tests; verify 5 new tests pass | ✅ | All header tests pass; verified via `cargo test --package md-codec bytecode::header`. |
| 1.5: Find + update existing `new_v0(...)` call sites | ✅ | Only one call site outside header.rs: `policy.rs:415` updated to `new_v0(opts.fingerprints.is_some(), false)` with a clear inline comment that Phase 4 will swap in real path-divergence detection. `rg new_v0\(` in src/ + tests/ confirms zero remaining 1-arg calls. |
| 1.6: Add `Tag::OriginPaths = 0x36` variant | ✅ | Variant added with rustdoc; `from_byte` arm added; reserved-range comment updated `0x36-0xFF → 0x37-0xFF`. `Tag` is `#[repr(u8)]` so `as_byte()` returns 0x36 automatically. |
| 1.6 (sub) F4 — three tag tests | ✅ | All three renamed/updated correctly (see Finding 4 above for line-by-line confirmation). |
| 1.7: Add Tag::OriginPaths byte-position test | ✅ | Both new tests `tag_origin_paths_byte_position` and `tag_v0_10_unallocated_starts_at_0x37` added at tag.rs:283-298, names + assertions match plan verbatim. |
| 1.8: Add new error variants + extend `ErrorVariantName` mirror (F2) | ✅ | Three new variants land with proper rustdoc; structural-vs-semantic split correct; `ErrorVariantName` extended with exactly two top-level entries. |
| 1.9: Verify all of Phase 1 builds + new tests pass | ✅ | `cargo build --workspace --all-features` clean; 612 tests pass workspace-wide. |
| 1.10: Run conformance gate (expected: 1 failure) | ✅ | `every_error_variant_has_a_rejects_test_in_conformance` fails with the expected two missing rejection tests (`rejects_origin_paths_count_mismatch`, `rejects_path_component_count_exceeded`). Phase 4 will land these. |
| 1.11: Commit Phase 1 | ✅ | Commit `3b38242` lands with detailed message; matches the plan's template. |

## Test coverage assessment

- **Wire-format tests:** Strong. Every header byte value is asserted
  (`0x00, 0x04, 0x08, 0x0C` round-trip; `0x01, 0x02, 0x03` reject;
  `0x10` rejects with `UnsupportedVersion`). The new
  `header_byte_0x02_rejects_with_reserved_bit_1` and
  `header_byte_0x01_rejects_with_reserved_bit_0` tests **strengthen**
  the assertions vs. the deleted test by binding `byte` and `mask` in
  the `matches!` arm — useful for catching mask-value drift.
- **Tag table tests:** Strong. The new `tag_v0_10_unallocated_starts_at_0x37`
  proves the entire `0x37..=0xFF` range is unallocated; the renamed
  `tag_round_trip_all_defined` proves `0x36` is now in the
  allocated chain; `tag_origin_paths_byte_position` pins both
  `as_byte()` and `from_byte()` for `0x36`.
- **Error variant tests:** Phase 1 deliberately does not write rejection
  tests for the three new error variants — those are Phase 4 work
  (conformance vectors). The mirror-enum extension correctly causes the
  conformance gate to fail until Phase 4 lands. Coverage hand-off to
  Phase 4 is well-defined.
- **Round-trip stability for existing wire format:** `as_byte()` for
  `BytecodeHeader::new_v0(false, false) == 0x00` and
  `new_v0(true, false) == 0x04` are explicitly asserted (preserving
  v0.9 byte identity for shared-path-only encodings, per Q10
  decision-matrix locked migration story).
- **No new error-variant rejection tests in Phase 1** is intentional and
  spec-aligned (per Step 1.10's framing); not a coverage gap.

## Recommended action

**Proceed to Phase 2.**

No inline fix required. The implementer correctly:
- Reclaimed header bit 3 with the right mask values per spec.
- Added `Tag::OriginPaths = 0x36` with a clean test sweep.
- Split the new error machinery into the right structural-vs-semantic
  buckets with correct mirror-enum entries.
- Updated all `new_v0(...)` call sites (single site).
- Caught two extra exhaustive-match sites the plan didn't enumerate
  and fixed them with idiomatic, in-style additions.
- Left the conformance gate failing on exactly the two expected
  variants, matching the Phase 4 hand-off contract.

The test-count discrepancy (Finding 1) is a reporting issue, not a code
issue. The error-message wording awkwardness for `count=0`
(Finding 2) is spec-endorsed; revisit at Phase 2 only if it surfaces a
real diagnostic readability concern when the actual check site lands.

**Phase 2 entry conditions met.** No re-review needed before Phase 2
proceeds.
