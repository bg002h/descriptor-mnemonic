# Changelog — `mk-codec`

All notable changes to the `mk-codec` crate.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this crate follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(pre-1.0: a minor bump may break).

**This file starts at 0.5.0.** Earlier releases (0.1.0 … 0.4.2) were tagged
`mk-codec-v*` without a changelog; `git log --oneline mk-codec-v0.4.2` and the
tag list are the record for those.

## [0.5.0] — 2026-08-19

### Changed — BREAKING

- **`chunk_set_id` is now DERIVED from the payload, not drawn from entropy.**
  It is the top 20 bits of `SHA-256(canonical_bytecode)`, MSB-first — the hash
  the chunk layer already computes for its cross-chunk integrity suffix, so the
  derivation costs nothing.

  `encode()` previously drew a fresh 20-bit id from the OS CSPRNG on every
  call, so **encoding the same card twice emitted two different cards on the
  wire** (measured 2026-08-14: three `mk encode` runs on identical inputs, three
  different strings).

  SPEC §2.5 already forbade this — it required an encoder to "reuse the same
  value for all subsequent re-encodings of the same card", and a *stateless*
  encoder cannot honour that by drawing entropy per call, having nowhere to keep
  the value chosen at first encoding. **So this is a conformance fix rather than
  a wire-format change:** the format always meant this; the reference
  implementation did not do it.

  Mirrors the sibling format — `md-codec`'s `derive_chunk_set_id` takes the top
  20 bits of its payload hash by the same MSB-first expression.

  **Impact on callers:** a chunked card re-encoded from identical inputs now
  reproduces byte-for-byte. Any fixture, golden vector or transcript that
  recorded a *previous* random `chunk_set_id` will show a different value and
  must be regenerated. Consumers pinning `mk-codec = "0.4"` are unaffected until
  they move the pin.

### Added

- `chunk_set_id` determinism tests (`tests/chunk_set_id_determinism.rs`).
- T4 external oracles: BIP-84/86 addresses and BIP-32 compact-form vectors.
- T2-c `bch_correct ⇒ valid` proptest, mined KATs, and a third fuzz target.
