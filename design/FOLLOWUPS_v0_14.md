# v0.14 follow-ups (carried from v0.13 post-ship audit)

This file collected items the v0.13 post-ship audit (commit `280c679`,
2026-04-30) flagged as latent issues or under-specifications. **All
four items L1–L4 closed in md-codec-v0.13.1.** Future-version concerns
that surface during v0.14 design will be added below as new sections.

## L1 — `Error::InvalidPresenceByte` consumer ✅ resolved in v0.13.1

`identity::validate_presence_byte(byte: u8) -> Result<(), Error>` added
as the parser-side enforcement of spec §5.3 "decoders MUST reject"
clause. Re-exported from `lib.rs`. Four unit tests cover legal
combinations and three reserved-bit patterns (lowest, highest, all).
The variant's `#[allow(dead_code)]` annotation was dropped; the helper
sits unused outside its tests, ready for v0.14+ canonical-record
consumers.

## L2 — Multi-chunk reassemble at the chunk-count boundary ✅ resolved in v0.13.1

`tests/chunking.rs` now exercises both ends of the chunk-count cap:

- `near_cap_descriptor_splits_to_at_most_64_chunks_and_round_trips`
  builds a single-sig descriptor with a ~2400-byte unknown TLV
  payload, splits to multiple chunks under the cap, reassembles, and
  asserts byte-identical round trip.
- `over_cap_descriptor_rejected_with_chunk_count_exceeds_max` inflates
  past the cap with a ~2700-byte unknown payload and asserts
  `Error::ChunkCountExceedsMax`.

## L3 — TLV bit_len strictness ✅ resolved in v0.13.1

The four sparse TLV body readers were consolidated into a generic
`read_sparse_tlv_body` helper in `tlv.rs` that bounds the
`BitReader`'s `bit_limit` to `start + bit_len` for the duration of
the body loop. Any over-read past the declared body errors with the
inner per-record validity variant rather than silently advancing the
outer cursor. Spec §3.2 has new "Strict body bounding" prose and §3.5
adds the broader invariant. Four hand-crafted-bad-wire tests assert
rejection across all four sparse TLV tags.

## L4 — `path_decl` always-populated invariant ✅ resolved in v0.13.1

Doc comments added to `expand_per_at_n` and
`compute_wallet_policy_id` capturing the Option A invariant — both
sites share the assumption that `path_decl.paths` is always populated
post-decode and never consult `canonical_origin` for path resolution
at hash time. Spec §3.5 (new "Invariants" subsection) formalizes the
contract and lists future-version tripwires.
