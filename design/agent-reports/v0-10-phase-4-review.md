# v0.10.0 Phase 4 review (opus)

**Date:** 2026-04-29
**Commit:** `2e61d38` (`feat(v0.10-p4): test corpus + conformance + hand-AST for OriginPaths`)
**Baseline:** `6913c5e` (Phase 3)
**Reviewer:** opus-4.7 (1M context)
**Branch:** `feature/v0.10-per-at-n-paths`

## Verdict

INLINE-FIX (description drift only) — recommend a one-line correction to o3's
description string + its `build_v0_10_origin_paths_vectors` doc comment, then
proceed to Phase 5. Wire format is correct, all 711 tests pass, conformance
gate now closes, regenerated JSON contains every new vector with the right
family token + SHA pin. The only finding is descriptive drift in o3 (claims
"0x05, 0x05, 0x04, explicit m/87'/0'/0'" but the actual encoded paths are
m/48'/0'/0'/2', m/48'/0'/0'/2', m/48'/0'/0'/1', m/87'/0'/0' which serialise
as dictionary indicators 0x05, 0x05, 0x06, 0x07 — no explicit-form path
involved). The drift is non-blocking and can be addressed inline before
Phase 5 commit, OR filed as FOLLOWUPS and fixed in Phase 6 (BIP/CHANGELOG)
since the JSON vector descriptions ship in v0.10.0 to downstream consumers.

## Scope reviewed

- Commit `2e61d38` against baseline `6913c5e`.
- Files inspected (full diffs read):
  - `crates/md-codec/src/vectors.rs` (+325 / -8): 3 positive vector helpers,
    6 negative vector generators, 1 inline byte-pin test.
  - `crates/md-codec/src/bytecode/hand_ast_coverage.rs` (+125): 4 hand-AST
    tests.
  - `crates/md-codec/tests/conformance.rs` (+111): 4 conformance tests.
  - `crates/md-codec/tests/vectors_schema.rs` (+4 / -4): corpus count 44→47,
    SHA pin update.
  - `crates/md-codec/tests/vectors/v0.1.json` (+1 / -1): family token
    `0.9` → `0.10`.
  - `crates/md-codec/tests/vectors/v0.2.json` (+149 / -1): family token +
    9 new vectors (3 positive, 6 negative).
- Cross-references checked:
  - `design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md` Phase 4 §§4.1–4.9.
  - `design/SPEC_v0_10_per_at_N_paths.md` §2 wire-format examples B and C.
  - `design/FOLLOWUPS.md` — `v010-p2-origin-paths-round-trip-spec-byte-pin`
    (Phase 2 deferred byte-pin entry).
  - `crates/md-codec/tests/error_coverage.rs` — `INVALID_BYTECODE_PREFIX`
    machinery + `ErrorVariantName` mirror enum.
  - `crates/md-codec/src/bytecode/path.rs:18-24` — dictionary indicator
    table (0x04=m/86'/0'/0', 0x05=m/48'/0'/0'/2', 0x06=m/48'/0'/0'/1',
    0x07=m/87'/0'/0').
- Build/test verification:
  - `cargo test --workspace --all-features --no-fail-fast`: **711 ok / 0
    failed / 0 ignored** (matches implementer claim).
  - `cargo test --package md-codec --lib vectors::`: 2 passed including
    `o2_vector_origin_paths_block_matches_spec_example_b` and
    `every_v2_negative_generator_fires_expected_variant`.
  - `cargo test --package md-codec --test conformance` (filtered to the 4
    new tests): all 4 pass.
  - `cargo test --package md-codec --test error_coverage`: 5/5 passing,
    including `every_error_variant_has_a_rejects_test_in_conformance` (was
    failing in Phase 3).
  - `cargo clippy --workspace --all-features --all-targets -- -D warnings`:
    clean.
  - `cargo fmt --all -- --check`: clean.
  - `sha256sum tests/vectors/v0.2.json`:
    `31ef8a1662a7768a1a7aaeb1fb04fef92580cb13ba5b016472fab97366926886` —
    matches `V0_2_SHA256` constant.

## Findings

### 1 — `o3` description and doc-comment misdescribe the actual encoded paths

- **Severity:** Important (descriptive drift — wire format correct, but
  the JSON `description` field ships in `tests/vectors/v0.2.json` and is
  user-facing for downstream conformance consumers).
- **Disposition:** Recommend INLINE-FIX before Phase 5 commit.
- **Description:** `vectors.rs:1180` and the `build_v0_10_origin_paths_vectors`
  doc comment at `:1149-1150` both describe o3 as covering "(0x05, 0x05,
  0x04, explicit m/87'/0'/0')". Reading the actual code:
  - `:1183-1186`: `mainnet.clone()` (m/48'/0'/0'/2'), `mainnet`
    (m/48'/0'/0'/2'), `m/48'/0'/0'/1'`, `m/87'/0'/0'`.
  - Per the dictionary table at `bytecode/path.rs:20-23`: m/48'/0'/0'/2' =
    0x05, m/48'/0'/0'/1' = **0x06**, m/87'/0'/0' = **0x07** (not explicit).
    0x04 maps to m/86'/0'/0' (BIP 86 P2TR), which is NOT used in o3.
  - Confirmed by the actual encoded hex `08 36 04 05 05 06 07 …` in
    `tests/vectors/v0.2.json`.
- **Why it matters:** Two issues:
  1. Description claims o3 includes an "explicit `m/87'/0'/0'`" path-decl,
     but `m/87'/0'/0'` is in the dictionary at indicator 0x07 — so o3
     does NOT exercise the explicit-form encoder path inside the
     OriginPaths block. If a reader (or consumer of v0.2.json) audits o3
     to verify count-4 boundary AND explicit-form coverage, they'll be
     surprised to find o3 covers only the dictionary-form case at count=4.
  2. The "0x04" indicator claim is straight-up wrong — would mislead a
     reviewer trying to decode the corpus by hand.
- **Wire format is correct:** the encoder produced valid bytes that round-
  trip; the count=4 boundary IS exercised. Only the prose drifts.
- **Suggested fix:** edit `crates/md-codec/src/vectors.rs:1149-1150` (doc
  comment for `build_v0_10_origin_paths_vectors`) and `:1180` (the o3
  description string) to reflect the actual encoding:
  ```
  - o3: 2-of-4 sortedmulti exercising count=4 boundary with four distinct
    dictionary-form indicators (0x05, 0x05, 0x06, 0x07).
  ```
  ```rust
  let o3 = build_origin_paths_vector(
      "o3_wsh_sortedmulti_2of4_divergent_paths",
      "O3 — wsh(sortedmulti(2,...)) 2-of-4 exercising count=4 boundary with four distinct dictionary-form path indicators (0x05, 0x05, 0x06, 0x07)",
      …
  );
  ```
  Then regenerate `tests/vectors/v0.2.json` and update the `V0_2_SHA256`
  pin (the description string is part of the JSON, so the SHA changes).
  Plus regenerate `v0.1.json` for consistency (schema 1 includes the same
  `description` field).
- **Alternative:** if the implementer wanted o3 to genuinely cover both a
  4-count boundary AND an explicit-form path-decl inside OriginPaths,
  swap one of the dictionary paths for an explicit-only path (e.g.,
  `m/48'/0'/0'/100'` from the dictionary table absence). This would change
  the wire bytes substantively and require a SHA pin update. The minimal
  fix is just the description.

### 2 — Phase 4's o2 inline test is the deferred Phase-2 byte-pin coverage

- **Severity:** N/A (positive confirmation; resolves the deferred FOLLOWUPS
  entry per the prompt's explicit guidance).
- **Disposition:** Mark `v010-p2-origin-paths-round-trip-spec-byte-pin` as
  resolved by Phase 4 (per the prompt).
- **Description:** Phase 2's reviewer flagged that
  `encode_origin_paths_round_trip_three_paths` at `bytecode/path.rs:1199`
  pinned only the 5-byte prefix (`bytes[0..=4]`) of SPEC §2 Example B's
  11-byte block, leaving the explicit-path tail bytes
  `04 61 01 01 C9 01` covered only indirectly via `assert_eq!(recovered,
  paths)`. The FOLLOWUPS entry's "Why deferred" pre-acknowledged that
  "Phase 4 conformance vectors will also pin Example B's full byte
  sequence as a fixture, providing a second line of defense."
  Phase 4's `o2_vector_origin_paths_block_matches_spec_example_b` test
  (vectors.rs:2693-2707) does exactly that: asserts that o2's
  `expected_bytecode_hex` contains the full 11-byte sequence
  `36030505fe04610101c901`. Verified the test is wired into the lib test
  suite and passes. The byte-pin coverage now exists.
- **Note:** the path.rs round-trip test itself was NOT strengthened to
  pin all 11 bytes inline — the prefix-pin pattern there remains. The
  prompt explicitly authorizes marking the FOLLOWUPS entry as resolved
  if Phase 4 covers the spec byte sequence somewhere in the corpus.
  Phase 4 does. (If a future maintainer wants the path.rs test ALSO
  strengthened, that's a separate polish item; the spec byte sequence
  is pinned at the corpus layer now, which is the durable artifact.)

### 3 — `rejects_invalid_bytecode_origin_paths_count_zero` is over-and-above the plan

- **Severity:** N/A (positive — defense-in-depth beyond plan).
- **Disposition:** Acknowledge-only.
- **Description:** Plan §4.4 specified 3 conformance tests (count_too_large,
  count_mismatch, path_component_count_exceeded). Implementer added a
  fourth: `rejects_invalid_bytecode_origin_paths_count_zero`. This is
  good — the `BytecodeErrorKind::OriginPathsCountTooLarge` variant
  structurally covers BOTH the count==0 and count>32 boundaries (per
  spec §2 line 109), and a hostile decoder ought to be tested at both
  boundaries. Both tests verify the variant fires with their respective
  count values (0 and 33). Not redundant with each other — they cover
  the bottom and top of the cap. Not redundant with the conformance gate
  either — `INVALID_BYTECODE_PREFIX` only requires ONE test name starting
  with `rejects_invalid_bytecode_`, and the existing v0.x tests already
  satisfy that floor.

### 4 — Negative vector `n_orig_paths_truncated` exercises the right truncation path

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:** Per the prompt's red-flag list ("does it actually trigger
  truncation at the right offset, or could it accidentally pass for a
  different reason?"). The vector is `[0x08, 0x36, 0x03, 0x05, 0x05]` —
  count=3 declared, only 2 dictionary-indicator path-decls follow. The
  `decode_origin_paths` loop reads 3 path-decls; the third hits
  `Cursor::read_byte` on an empty buffer, which returns
  `BytecodeErrorKind::UnexpectedEnd` (verified via the build-time
  `debug_assert_decode_matches(&[s.as_str()], "InvalidBytecode")`).
  The truncation triggers at the third iteration, exactly as intended.
  No accidental success path (e.g., the bytes don't accidentally also
  fail header parse or tag validation — header `0x08` is a valid v0.10
  header, tag `0x36` is the expected OriginPaths tag).

### 5 — `n_orig_paths_count_mismatch` defense-in-depth construction is real

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:** Per the prompt's claim. Read `vectors.rs:2225-2237`:
  ```rust
  let policy: WalletPolicy = "wsh(sortedmulti(2,@0/**,@1/**,@2/**))".parse().unwrap();
  let valid = policy.to_bytecode(&EncodeOptions::default()).unwrap();
  let tree_bytes = &valid[3..];   // strip [header, SharedPath, indicator]
  let mut bytecode: Vec<u8> = vec![0x08, Tag::OriginPaths.as_byte(), 0x04];
  bytecode.extend_from_slice(&[0x05, 0x05, 0x05, 0x05]);
  bytecode.extend_from_slice(tree_bytes);
  ```
  This is a real round-trip-compatible 3-placeholder tree extracted from
  a real encoder run, not a synthetic stub. The constructed bytecode
  parses through the entire OriginPaths block (count=4 + 4 indicators),
  then walks into the (real) tree which decodes to 3 placeholders, and
  surfaces `Error::OriginPathsCountMismatch { expected: 3, got: 4 }`.
  Build-time `debug_assert_decode_matches` confirms the variant.

### 6 — Hand-AST tests cover all four claimed contracts

- **Severity:** N/A (positive verification).
- **Disposition:** Acknowledge-only.
- **Description:**
  - `header_origin_paths_flag_round_trip` — pins `BytecodeHeader::new_v0(false, true).as_byte() == 0x08`, then asserts `from_byte(0x08).origin_paths() == true` and `fingerprints() == false`. ✓
  - `encoder_emits_shared_path_when_all_paths_agree` — encodes a 3-placeholder template with default opts, asserts `bytes[0] & 0x08 == 0` and `bytes[1] == Tag::SharedPath` (0x34) and not 0x36. ✓
  - `encoder_emits_origin_paths_when_paths_diverge` — encodes the same template with `with_origin_paths` Tier 0 override carrying divergent paths, asserts `bytes[0] == 0x08` and `bytes[1] == Tag::OriginPaths` (0x36). ✓
  - `max_path_components_boundary_10_passes_11_rejects` — `encode_path` with a 10-component path passes; with an 11-component path returns `Error::PathComponentCountExceeded { got: 11, max: 10 }`. ✓
  All four assertions reflect the spec text and verify the right contracts.

## Spec-compliance verification

| Plan step | Status | Notes |
|---|---|---|
| 4.1: Add positive vectors o1, o2, o3 + inline o2 spec-byte-pin test | ✅ | All three vectors present with correct IDs; inline test asserts full 11-byte spec sequence is contained in o2.expected_bytecode_hex. o3 description has drift (Finding 1) but bytes are correct. |
| 4.2: Add 6 negative vectors | ✅ | All 6 IDs present and matching plan; build-time `debug_assert_decode_matches` confirms each fires its declared variant. |
| 4.3: Update `vectors_schema.rs` corpus count assertion | ✅ | 44 → 47 with explanatory message. |
| 4.4: Add conformance tests | ✅ + (one bonus test) | Plan asked for 3; implementer added 4. The fourth (`rejects_invalid_bytecode_origin_paths_count_zero`) is good defense-in-depth. All four pass. The two `rejects_invalid_bytecode_origin_paths_*` names satisfy `INVALID_BYTECODE_PREFIX`; `rejects_origin_paths_count_mismatch` and `rejects_path_component_count_exceeded` satisfy their PascalCase-derived expected substrings. |
| 4.5: Add hand-AST coverage tests | ✅ | All 4 tests present and correct. |
| 4.6: Regenerate v0.1.json + v0.2.json + update SHA pin | ✅ | Both JSONs carry `"generator": "md-codec 0.10"`; `V0_2_SHA256` updated to `31ef8a1662a7768a1a7aaeb1fb04fef92580cb13ba5b016472fab97366926886`; `sha256sum` of the file confirms the pin matches. |
| 4.7: Full test gate | ✅ | 711 ok / 0 failed; was 701 ok / 1 failed in Phase 3 (the single failing test was `every_error_variant_has_a_rejects_test_in_conformance`, now passing). |
| 4.8: Commit Phase 4 | ✅ | `2e61d38` with descriptive message + Co-Authored-By. |
| 4.9: Opus reviewer pass on Phase 4 | (in progress — this report) | |

Spec byte sequence verification:

- **SPEC §2 Example B** (`36 03 05 05 FE 04 61 01 01 C9 01`) — present in
  o2's encoded bytes (`0c 36 03 05 05 fe 04 61 01 01 c9 01 35 03 …`),
  verified by `o2_vector_origin_paths_block_matches_spec_example_b`. ✓
- **SPEC §2 Example C** (`08 | 36 03 05 05 FE 04 61 01 01 C9 01`) — present
  in o1's encoded bytes (`08 36 03 05 05 fe 04 61 01 01 c9 01 …`). ✓
- **Header byte 0x08** for divergent paths without fingerprints — verified
  via o1 hex prefix and `encoder_emits_origin_paths_when_paths_diverge`. ✓
- **Header byte 0x0C** for divergent paths with fingerprints — verified
  via o2 hex prefix. ✓

## Test corpus assessment

**Positive vectors (3):** o1 (header 0x08, no fps, mirrors Example C),
o2 (header 0x0C, with 3 fps, mirrors Example B), o3 (header 0x08, no fps,
count-4 boundary). Wire format correct in all three. Description drift
in o3 (Finding 1).

**Negative vectors (6):** all 6 IDs match the plan. Each generator calls
`debug_assert_decode_matches` to verify the expected variant fires at
build time. The `every_v2_negative_generator_fires_expected_variant` test
re-verifies at test time. All pass.

**Conformance gate closure:** the four new tests cover the two new
variants (`OriginPathsCountMismatch`, `PathComponentCountExceeded`) plus
two new sub-variants under `BytecodeErrorKind::OriginPathsCountTooLarge`.
The `every_error_variant_has_a_rejects_test_in_conformance` test now
passes — verified directly via `cargo test --package md-codec --test
error_coverage`. The conformance gate is closed.

**Coverage gap check:** read `error_coverage.rs:37-67` `ErrorVariantName`
mirror enum. The two new entries are `OriginPathsCountMismatch` (line 65)
and `PathComponentCountExceeded` (line 66). Their pascal-to-snake
conversions yield `origin_paths_count_mismatch` and
`path_component_count_exceeded`, which are exactly the test name
suffixes. No coverage gap; no test that LOOKS right but fails to match
the parser pattern.

## Hand-AST coverage assessment

The 4 tests pin (a) header bit 3 round trip via `BytecodeHeader::new_v0`
and `from_byte`, (b) encoder dispatch in both directions (shared paths →
SharedPath tag; divergent paths → OriginPaths tag, header 0x08), and (c)
the `MAX_PATH_COMPONENTS=10` boundary at the encoder API. Together they
provide hand-AST guard coverage for every load-bearing wire-format
decision Phase 4 introduces. No gaps observed.

The use of `BytecodeHeader::origin_paths()` accessor in
`header_origin_paths_flag_round_trip` (rather than asserting `h == h2`
which the plan sketch suggested) is slightly more rigorous: it directly
verifies the field round-trips through the byte representation, rather
than relying on `PartialEq` to capture the field — a subtle improvement
over the plan sketch.

## JSON regeneration verification

- **Family token:** Both `v0.1.json` and `v0.2.json` show
  `"generator": "md-codec 0.10"` — the token reflects the version bump,
  not the prior `"md-codec 0.9"`. ✓
- **SHA pin:** `V0_2_SHA256` constant in `tests/vectors_schema.rs` updated
  to `31ef8a1662a7768a1a7aaeb1fb04fef92580cb13ba5b016472fab97366926886`,
  which exactly matches `sha256sum` of the committed `v0.2.json`. ✓
- **Corpus count:** `vectors_schema.rs` asserts `v2.vectors.len() == 47`
  with explanatory message. Plan specifies `44 → 47` (+3 for o1/o2/o3).
  The actual JSON has 47 positive + 22 negative IDs (from `grep -c
  '"id":'` returning 101, of which the existing v0.9 corpus contributed
  ~92 lines). Verified by spot-checking the v0.2.json that all 9 new
  IDs are present:
  - `o1_sortedmulti_2of3_divergent_paths` ✓
  - `o2_sortedmulti_2of3_divergent_paths_with_fingerprints` ✓
  - `o3_wsh_sortedmulti_2of4_divergent_paths` ✓
  - `n_orig_paths_count_zero` ✓
  - `n_orig_paths_count_too_large` ✓
  - `n_orig_paths_truncated` ✓
  - `n_orig_paths_count_mismatch` ✓
  - `n_path_components_too_long` ✓
  - `n_conflicting_path_declarations_bit_set_tag_shared` ✓

## Recommended action

**Proceed to Phase 5 after one inline doc-string fix in vectors.rs (or
defer the fix to a Phase 6 housekeeping commit if the implementer prefers
to land the SHA-pin churn in one place with the BIP/CHANGELOG updates).**

### Required edits (INLINE-FIX scope)

The o3 description and helper doc comment misdescribe the wire bytes.
Either:

**(Option A) Fix inline before Phase 5 ships:**

1. Edit `crates/md-codec/src/vectors.rs:1149-1150` (doc comment):
   ```
   - o3: 2-of-4 sortedmulti exercising count=4 boundary with four distinct
     dictionary-form indicators (0x05, 0x05, 0x06, 0x07; paths
     m/48'/0'/0'/2', m/48'/0'/0'/2', m/48'/0'/0'/1', m/87'/0'/0').
   ```

2. Edit `crates/md-codec/src/vectors.rs:1180` (description):
   ```rust
   "O3 — wsh(sortedmulti(2,...)) 2-of-4 exercising count=4 boundary with four distinct dictionary-form path indicators (0x05, 0x05, 0x06, 0x07)",
   ```

3. Regenerate `v0.1.json` + `v0.2.json` (the description ships in the
   JSON), update `V0_2_SHA256` to the new SHA, run the full test suite.

**(Option B) Defer to FOLLOWUPS:** file as a v0.10.0.x or v0.11
description-cleanup item; the wire format is correct and shipping with a
slightly-misleading description string is non-fatal (the corpus is
defended by the wire-format SHA pin and round-trip tests).

I recommend **Option A** — the SHA churn is one commit, the description
ships in the JSON to downstream consumers, and Phase 5 hasn't started
yet so no in-flight work is disturbed.

### Optional polish (non-blocking)

- Phase 2 FOLLOWUPS entry `v010-p2-origin-paths-round-trip-spec-byte-pin`
  also asks to strengthen the `encode_origin_paths_round_trip_three_paths`
  test in `path.rs:1199` to inline-pin the full 11-byte sequence. Phase 4's
  o2 byte-pin in `vectors.rs` provides the spec byte coverage at the
  corpus layer (per the prompt's explicit resolution authorization), but
  if a future maintainer wants belt-and-suspenders the path.rs test could
  also be strengthened. Not Phase 4's responsibility per the prompt.

### FOLLOWUPS handling

**Resolved:** `v010-p2-origin-paths-round-trip-spec-byte-pin` — Phase 4's
`o2_vector_origin_paths_block_matches_spec_example_b` test asserts the
full 11-byte spec byte sequence (`36030505fe04610101c901`) is contained
in o2's encoded bytecode, which is the second-line-of-defense coverage
the FOLLOWUPS entry pre-acknowledged. Per the prompt's explicit
authorization, marking the entry as `resolved by md-codec-v0.10.0 phase 4
(commit 2e61d38)` and Tier as `(closed)`.

**Newly filed:** depends on disposition of Finding 1.
- If Option A: no new entry; fix inline.
- If Option B: file `v010-p4-o3-vector-description-drift` at tier
  `v0.10-housekeeping` with the corrected description text.

### Phase 5 entry conditions

After Finding 1 is resolved (either inline or via FOLLOWUPS), Phase 5
may proceed. Phase 5 owns the `PolicyId::fingerprint() -> [u8; 4]` API.
The conformance gate is now closed; build/clippy/fmt are clean; corpus
count + SHA pin are correct. No Phase-4-derived blockers for Phase 5.
