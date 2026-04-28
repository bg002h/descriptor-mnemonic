# BIP draft

This directory contains the formal specification for the Mnemonic Descriptor
(MD) format in BIP MediaWiki format.

- [`bip-mnemonic-descriptor.mediawiki`](bip-mnemonic-descriptor.mediawiki) — the spec.

## Status

The BIP is in **Pre-Draft, AI only, not yet human reviewed** status. It has
not been assigned a BIP number; the placeholder `BIP: ?` will be replaced
once the spec is mature enough for community review and submission to the
BIP editor. The standard BIP 2 status `Draft` will be claimed only after
end-to-end human review.

## Reference implementation

A Rust reference implementation lives in
[`../crates/md-codec/`](../crates/md-codec/) and locks the v0.1 wire
format. It includes:

- Encode and decode pipelines with per-stage diagnostics.
- 565 unit + integration tests, including 12 corpus round-trips and 30+
  negative conformance vectors.
- BCH known-vectors verified against an independent Python implementation.
- A locked test-vector file at
  [`../crates/md-codec/tests/vectors/v0.1.json`](../crates/md-codec/tests/vectors/v0.1.json)
  that cross-implementations can consume directly. The schema is documented
  in [`../crates/md-codec/src/vectors.rs`](../crates/md-codec/src/vectors.rs).

See the crate's [`README.md`](../crates/md-codec/README.md) for a
quickstart and CLI reference.

## See also

- [`../design/POLICY_BACKUP.md`](../design/POLICY_BACKUP.md) — design
  rationale and open decisions.
- [`../design/PRIOR_ART.md`](../design/PRIOR_ART.md) — survey of related
  encoding schemes.
- [`../design/CORPUS.md`](../design/CORPUS.md) — reference miniscript
  test corpus.
