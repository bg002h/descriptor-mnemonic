# v0.11 Phase 21 Review — Multi-card Chunking with codex32 Integration

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Phase:** 21 (Multi-card chunking — split + reassemble)
- **Status:** DONE
- **Commits:**
  - Task 21.1 — `acdf9e6` feat(v0.11): chunk split with codex32 integration
  - Task 21.2 — `89fedf2` feat(v0.11): reassemble multi-chunk md1 strings into Descriptor

## Scope

Phase 21 wires the Phase 18 ChunkHeader plumbing and Phase 20 single-string codex32 codec into a full multi-card lifecycle:

- `split(d: &Descriptor) -> Result<Vec<String>, V11Error>` — emits one md1 codex32 string per chunk; each chunk carries a 37-bit ChunkHeader (3-bit version, 2 mode flags, 20-bit ChunkSetId, 6-bit index, 6-bit total) followed by payload bytes; HRP+BCH wrap per chunk.
- `reassemble(strings: &[&str]) -> Result<Descriptor, V11Error>` — accepts arbitrary order; sorts by index; validates header consistency, completeness, and cross-chunk integrity; concatenates payload and decodes via the existing v0.11 byte decoder; verifies recovered `Md1EncodingId` against the ChunkSetId top-20 bits.

Spec citations: §9 (chunking) for the ChunkSetId derivation, per-chunk header layout, and reassembly contract; §3.7 (forward-compat / TLV behavior) for the unknown-tag skip discipline preserved across the byte-stream pipeline that chunking sits on top of.

## Verification

```
cargo test -p md-codec --test v11_chunking
running 3 tests
test chunk_set_id_matches_md1_encoding_id_top_20_bits ... ok
test small_descriptor_splits_into_one_chunk ... ok
test small_descriptor_split_then_reassemble ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Cumulative v0.11 test count: **103** (100 carried in + 3 chunking).

## Chunk-byte recovery formula

Reassembly recovers the exact payload byte count using `(symbol_aligned_bit_count - 37) / 8`, where `symbol_aligned_bit_count` is the bit length implied by the codex32 symbol count after stripping the BCH checksum. This is correct for **all** chunk sizes including the previously-broken N=3, 8, 11 cases that would mismatch under naive `bytes.len() * 8` accounting (because the final symbol's pad bits must be discarded before subtracting the 37-bit header). The encoder pads to symbol alignment on the way out; the decoder strips those pad bits before splitting header from payload, so round-trip is exact regardless of whether N straddles a 5/8 LCM boundary.

## API surface

- `split(d: &Descriptor) -> Result<Vec<String>, V11Error>`
- `reassemble(strings: &[&str]) -> Result<Descriptor, V11Error>`

New error variants (5):

- `ChunkSetEmpty` — input slice empty
- `ChunkSetInconsistent` — chunks disagree on version/mode/ChunkSetId/total
- `ChunkSetIncomplete` — fewer chunks than declared total
- `ChunkIndexGap` — duplicate or missing index in [0, total)
- `ChunkSetIdMismatch` — recovered `Md1EncodingId` top-20 bits do not match the ChunkSetId stamped in chunk headers

## Milestone

**Multi-card chunking operational.** v0.11 can now split a Descriptor into N md1 codex32 strings and reassemble them in any order back into the original Descriptor with cross-chunk integrity verified. End-to-end pipeline: Descriptor -> bytes -> chunks -> 37-bit-header-prefixed payloads -> codex32 strings (HRP+BCH per chunk) -> reverse.

## Carry-forward deferred items

Same set as prior phases: P1, P2, P4, P5, P13a, P13b. (P12 resolved in Phase 19.) No new deferrals introduced in Phase 21.

## Next

Phase 22 — end-to-end engrave/restore tests for BIP 86 + vault scenarios, exercising `split`/`reassemble` against realistic descriptor fixtures.

---

DONE — commits `acdf9e6` (split) and `89fedf2` (reassemble); 3/3 chunking tests pass; cumulative v0.11 = 103.
