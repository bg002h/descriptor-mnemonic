# Phase 7 Decision Log

Living document of decisions made during execution of Phase 7 (CLI binary). Each entry: what was decided, why, alternatives considered, where to verify in code.

## Open questions for review

(Populated as design questions arise that an implementer picked a default for. Empty = no open questions.)

---

## Carry-forward notes from earlier phases

### CF-1 (from Phase 4): chunk byte-boundary detection — codex32 layer is the source of truth

**Context**: During Phase 4-E review (`f0d9346`) and its follow-up sweep (`2e735be`, `e7a7a16`), we considered adding a `Chunk::from_exact_bytes` helper alongside `Chunk::from_bytes` that would error on trailing bytes. That helper turned out to be **dead-equivalent to `from_bytes`** at the byte layer because:

- `ChunkHeader` (per BIP §"Chunk format") has fixed sizes (2 bytes for SingleString, 7 bytes for Chunked) — both validatable from header content alone.
- A chunk's *fragment* has no length field on the wire. The fragment size is implicit: it's whatever bytes follow the header in the chunk's byte buffer.
- `Chunk::from_bytes` therefore consumes the entire input slice as `header_bytes ++ fragment_bytes`. There's no in-buffer way to detect "this slice has more bytes than this chunk should have" because the chunk's own byte form has no length.

We removed the helper in `e7a7a16` rather than carry dead API surface.

**Decision for Phase 7**: when the codex32 string layer parses a chunk-bearing codex32 string into raw bytes, those bytes ARE one chunk by construction (the codex32 string is its own length-self-describing wrapper via the bech32 string boundary + checksum length). Phase 7's chunk-string parser:

1. Decodes a codex32-derived string into its 5-bit data part.
2. Strips the BCH checksum (13 chars regular, 15 chars long).
3. Re-packs the remaining 5-bit data into bytes (via `bytes_to_5bit`/`five_bit_to_bytes` in `encoding.rs`).
4. Hands the resulting byte buffer to `Chunk::from_bytes`.
5. Asserts (or trusts) that `consumed == bytes.len()` on the returned `(Chunk, usize)` pair. With the current `from_bytes` design this is a tautology, but the assertion documents the invariant for any future audit.

**No new wire-format field required**: the codex32 string boundary is the chunk boundary. The "trailing bytes within a chunk" failure mode does not exist at the byte layer; it only exists at the codex32 layer (e.g., a malformed string with extra characters before the checksum), and that's caught by the codex32 BCH checksum verification (Phase 1, `encoding.rs`), not by chunk parsing.

**When this would change**: if a future variant ever needs to embed multiple chunks back-to-back in a single byte stream (e.g., for a test corpus file or a batched-export format), THEN we'd need either:

- A fragment-length field added to the chunk header (wire-format change; new tag + bytecode-version bump territory), or
- A length-prefixed framing layer above the chunk (out-of-scope addition; resembles tar/cpio archives).

Neither is needed for v0.1 or any v0.2 work currently scoped. If a Phase 7 implementer feels tempted to reintroduce a `from_exact_bytes`-style helper, that's a signal to revisit this decision rather than add the helper — the helper would be inert API noise unless one of the two changes above is also made.

**Verify in code**: when Phase 7's chunk-string parser is implemented, look for:

- A function like `parse_chunk_string(s: &str) -> Result<Chunk, Error>` that combines `decode_string` (from `encoding.rs`) with `Chunk::from_bytes`.
- An inline `debug_assert_eq!(consumed, bytes.len())` (or equivalent comment) at the `from_bytes` call site that documents the codex32-layer-provides-boundary invariant.
- No new error variants for "trailing bytes inside a chunk byte buffer" (the codex32 layer's `Error::InvalidChar` / `Error::InvalidStringLength` / BCH-correction errors cover the analogous failure modes at the string layer).

---

## Decisions made

(Populated as Phase 7 executes.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| _none yet_ | | |
