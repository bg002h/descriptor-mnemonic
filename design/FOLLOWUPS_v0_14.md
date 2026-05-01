# v0.14 follow-ups (carried from v0.13 post-ship audit)

This file collects items the v0.13 post-ship audit (commit `280c679`,
2026-04-30) flagged as latent issues or under-specifications that v0.14
planning should pick up. Nothing here blocks v0.13.0 — these are forward-
looking concerns.

## L1 — `Error::InvalidPresenceByte` has no consumer

**State**: defined and `#[allow(dead_code)]`-marked in `crates/md-codec/src/error.rs`.

**Why it exists**: spec v0.13 §5.3 mandates "decoders MUST reject inputs
with non-zero reserved bits in `presence_byte`." But in v0.13 the
canonical-record preimage is hash-internal — no parser ever consumes it
from a wire — so the rejection path is unreachable. The variant is
defined as forward scaffolding for v0.14+.

**Risk for v0.14**: when canonical-record bytes become wire-visible
(e.g., new TLV embedding the format, or a verification-mode that surfaces
the preimage), the parser must rediscover the spec contract from
scratch. There's no reachable test that exercises rejection today.

**Action for v0.14 planning**:
- Decide whether v0.14 surfaces canonical-record bytes. If not, drop the
  variant; if yes, add (a) the parser, (b) a property test that mutates
  a known-good preimage's reserved bits and asserts rejection.
- The shipped `walletpolicyid_reserved_bits_masking_property` test
  (`identity.rs`) proves SHA-256 is mask-stable but does not exercise a
  rejection.

## L2 — Multi-chunk reassemble at the chunk-count boundary

**State**: `chunk::reassemble` (chunk.rs:303) calls
`decode_payload(&full_bytes, full_bytes.len() * 8)`, relying on TLV
rollback to absorb trailing padding (≤7 bits per the v0.12.0 contract).

**Why benign in v0.13**: encoder contract is chunk wire = 37 + 8N bits
(byte-boundary ends), so padding is only at the end of the final chunk.

**Risk for v0.14**: extensions reshaping chunk payloads (e.g., a chunk
header with non-byte payload size, or sub-chunk multiplexing) could
produce inter-chunk padding that the rollback budget can't absorb.

**Action for v0.14 planning**:
- Add a test that exercises `count = 64` (the v0.11 chunk-count maximum
  per BIP-93 long-form? actually count cap is per spec §9.8) so the
  boundary case is locked.
- If v0.14 reshapes chunk payloads, audit `reassemble` and `decode_payload`'s
  trailing-bit tolerance jointly.

## L3 — TLV bit_len strictness under-specified

**State**: spec §3.2 says "records pack until the TLV's bit-length is
exhausted; the inner value type is self-delimiting." For `Pubkeys`,
each record is a fixed 65 bytes; for `OriginPathOverrides`, `OriginPath::read`
self-terminates. But the spec doesn't say what happens when the declared
`bit_len` doesn't exactly match the record-pack length.

**Why benign today**: trailing slack inside a TLV body is benign — the
loop exits and the next iteration's `read_sparse_tlv_idx` will return
`BitStreamTruncated` if it tries to read past `bit_len`. The error
message is misleading but the rejection happens.

**Risk for v0.14**: malicious wires could declare `bit_len = N×65×8 + 2`
to consume two trailing bits silently before the next TLV; some future
parser that handles trailing slack as "ignore" instead of "reject" would
diverge from existing behavior.

**Action for v0.14 planning**:
- Add to spec §3.2: "after the last record is fully consumed, the decoder
  MUST verify `r.bit_position() - start == bit_len`; trailing slack
  inside a TLV body is rejected."
- Add the corresponding strict check in `tlv.rs` readers (one extra
  check per reader; ~5 lines each).
- Add a hand-crafted-bad-wire test for each sparse TLV.

## L4 — `path_decl` always-populated invariant

**State**: shipped Option A (v0.13 spec §3 Path-decl semantics) requires
`path_decl` populated on every wire. `expand_per_at_n` and
`compute_wallet_policy_id` rely on this — neither falls through to
`canonical_origin` at hash time.

**Risk for v0.14**: if some future version makes `path_decl` elidable
(e.g., a "minimal-bytes" mode that drops it for canonical wrappers),
`MissingExplicitOrigin` becomes the load-bearing gate, and
`compute_wallet_policy_id`'s simplification breaks (it would need to
re-add the `canonical_origin` lookup at hash time, regressing on the
plan's clean Option A).

**Action for v0.14 planning**:
- If `path_decl` elision is contemplated, audit `expand_per_at_n` and
  `compute_wallet_policy_id` jointly. Document the invariant they share.
- Consider a wire-format check at decode time: assert `path_decl.paths`
  resolves to a non-empty path for every `@N` whose canonical_origin is
  None (this is what `validate_explicit_origin_required` already does;
  the invariant is currently maintained by encoder discipline + decoder
  validation, not a structural wire field).
