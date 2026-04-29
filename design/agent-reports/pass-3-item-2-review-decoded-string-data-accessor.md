# Pass 3 Item 2: Review — DecodedString.data() Accessor Refactor

**Status:** DONE_WITH_CONCERNS
**Commit:** (uncommitted at review time — reviewing working-tree changes; will be committed by controller after addressing the documentation finding below)
**File(s):**
- `crates/md-codec/src/encoding.rs`
- `crates/md-codec/src/decode.rs`
- `crates/md-codec/src/encode.rs`
- `crates/md-codec/src/bin/md/main.rs`
- `crates/md-codec/tests/chunking.rs`
- `CHANGELOG.md`
- `MIGRATION.md`
- `design/FOLLOWUPS.md`
**Role:** reviewer (code-quality)

## What was reviewed

The `DecodedString.data` field-to-method refactor: correctness of the `data()` accessor, panic safety, allocation accounting in the consumer loop, absence of unintended clones at call sites, and accuracy of the MIGRATION.md and CHANGELOG.md documentation for downstream consumers.

## Findings

### Critical

None.

### Important

**MIGRATION.md third alternative pattern is semantically misleading (confidence: 90)**

File: `MIGRATION.md`, the third bullet of the "How to upgrade" code block:

```rust
// OR — if the consumer doesn't need data_with_checksum afterwards —
// take the underlying buffer:
let owned_with_checksum: Vec<u8> = decoded.data_with_checksum;
```

This is presented as an alternative to `let owned: Vec<u8> = decoded.data().to_vec()` for consumers who previously wrote `let owned: Vec<u8> = decoded.data`. The problem: `data_with_checksum` is NOT the same data as the old `data` field. The original `data` stored the checksum-stripped prefix — `data_with_checksum[..len - 13/15]`. `data_with_checksum` includes the trailing 13 or 15 BCH checksum symbols.

The variable name chosen in the guide (`owned_with_checksum`) signals the difference, but the surrounding prose does not. The comment "if the consumer doesn't need data_with_checksum afterwards" addresses only the ownership/move concern (can I move the Vec?), not the semantic content difference. A downstream consumer who reads "take the underlying buffer" and substitutes `data_with_checksum` as a drop-in for the old `data` in code that then passes the result to `five_bit_to_bytes` or any other processor of the 5-bit payload symbols will silently produce wrong output: the decoded byte string will have additional bytes decoded from the trailing checksum symbols.

The same issue appears in `CHANGELOG.md`: "or take the underlying buffer via `decoded.data_with_checksum` if the checksum-stripped slice is no longer needed" — the phrase "checksum-stripped slice" likely refers to whether the caller still needs `data()` (i.e., the checksum-stripped slice), but can be read as characterizing what `data_with_checksum` returns.

**Fix:** The third alternative should carry an explicit warning that `data_with_checksum` is LONGER than the old `data` field and must not be used as a payload-processing substitute. The safest fix is to add a sentence: "Note: `data_with_checksum` includes the trailing 13- or 15-symbol BCH checksum and is NOT a drop-in replacement for `data` in payload-processing contexts (e.g., do not pass it to `five_bit_to_bytes` — use `decoded.data()` or `decoded.data().to_vec()` for that)." Alternatively, remove the third pattern from the migration guide entirely, since it is not a replacement for the removed field — it is a different piece of data.

## Passing checks

1. **Accessor correctness**: `data()` returns `&self.data_with_checksum[..self.data_with_checksum.len() - checksum_len]` with `checksum_len = 13 (Regular) / 15 (Long)`. This exactly mirrors what the old `data` field stored. Verified by tracing `decode_string` construction: `data_with_checksum` is populated from `CorrectionResult.data`, which holds the full data-part (header + payload + checksum), and the pre-refactor `data` field was explicitly computed as `result.data[..result.data.len() - checksum_len].to_vec()`.

2. **Panic safety**: `bch_code_for_length` requires total data-part length `14..=93` for Regular and `96..=108` for Long before `decode_string` proceeds. The `CorrectionResult.data` output has the same length as the input `values` slice. Therefore `data_with_checksum.len() >= 14 >= 13` for Regular and `>= 96 >= 15` for Long. The subtraction `data_with_checksum.len() - checksum_len` cannot underflow on any reachable `DecodedString`.

3. **Allocation accounting**: The stage-2 buffer change from `Vec<(Vec<u8>, BchCode)>` to `Vec<DecodedString>` correctly preserves one allocation per decoded string. The consumer loop `for decoded in decoded_strings { five_bit_to_bytes(decoded.data()) ... }` moves the `DecodedString` (not a clone), calls `data()` to get a `&[u8]` slice into the existing buffer, and passes it to `five_bit_to_bytes`. No clone is introduced.

4. **No clones at call sites**: All six changed call sites (`encode.rs` 4 sites, `main.rs` 1 site, `tests/chunking.rs` 1 site) pass `decoded.data()` directly as `&[u8]` to `five_bit_to_bytes`. None introduce `.to_vec()`. Semantically identical to the previous `&decoded.data` (which was `&Vec<u8>` auto-dereffed to `&[u8]`).

5. **Import cleanup**: `BchCode` is correctly removed from the parent module's `use crate::{...}` and added to the test module's `use crate::{...}`. `BchCode` is not used in the parent module's production code post-refactor; it is still needed in the test module at line 307.

6. **CHANGELOG placement**: `[Unreleased]` section correctly placed above `[0.5.0]`. Change is correctly classified as breaking (public field removal). The planned version (0.6.0) is consistent with project versioning conventions.

## Follow-up items

If the controller chooses to defer rather than fix the documentation issue inline, the following FOLLOWUPS.md entry would be appropriate:

- `migration-data-with-checksum-alternative-clarification`: MIGRATION.md third alternative pattern (`let owned_with_checksum = decoded.data_with_checksum`) needs an explicit warning that `data_with_checksum` includes BCH checksum symbols and is not a payload-processing substitute for the removed `data` field. Affects `MIGRATION.md` and the parallel wording in `CHANGELOG.md`. Not a blocker (no internal call sites use this pattern; the concern is purely downstream-consumer documentation quality), but should be fixed before 0.6.0 ships.
