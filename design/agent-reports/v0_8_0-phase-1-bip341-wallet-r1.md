# v0.8.0 Phase 1 — md-codec BIP-341 — R1 architect review + disposition

**Date:** 2026-05-13
**Reviewer:** `feature-dev:code-reviewer` (Sonnet 4.6), dispatched per
plan Phase 1 reviewer-loop discipline.
**Phase commit reviewed:** initial Phase 1 commit on branch
`v0_8_0-bip341-wallet-vectors` adding
`crates/md-codec/tests/bip341_wallet_vectors.rs` + fixture +
`[dev-dependencies]` for `serde_json` and `hex`.

## R1 verdict

**2I / 0C** — both Important findings folded in-cycle.

## R1 findings

### I-1 — FOLLOWUPS.md entries absent from disk (confidence 95)

Per CLAUDE.md cross-repo discipline + SPEC §5: each touched repo's
`design/FOLLOWUPS.md` must carry a `bip-vector-adoption-v0_8`
companion entry visible on disk, not only referenced in prose. The
initial Phase 1 commit referenced two FOLLOWUPS via the test file's
module-doc comment but did not actually create the entries.

**Fold:** added two entries to `design/FOLLOWUPS.md`:

1. `bip-vector-adoption-v0_8` — cross-repo cycle companion entry
   pointing at the SPEC, the plan, and this Phase 1 work. Will
   close when the cycle's audit-matrix successor doc lands at
   `design/agent-reports/v0_8_0-bip-test-vector-audit-matrix.md`
   (Phase 4) and the patch tag is cut (Phase E).
2. `bip341-keypath-signing-vector-coverage` — declares the
   companion `keyPathSpending` array (1 vector) OUT-OF-SCOPE-PER-LAYER.
   md-codec exposes no Schnorr signing surface; the cell would
   require a new surface and is deferred to v1+.

### I-2 — Unused `sha256d` import would fail clippy (confidence 90)

The test file's module-level `use bitcoin::hashes::sha256d;` was
"consumed" only via a `std::any::type_name::<sha256d::Hash>()` call
inside `fixture_sha256_pin`. `type_name` is a generic function;
its type parameter does not count as a real use for the
unused-imports lint. Under `cargo clippy -p md-codec --all-targets
-- -D warnings`, this would have fired as a new clippy error on
top of the two pre-existing errors in `src/error.rs`.

**Fold:** dropped the unused `sha256d` import and the
`type_name` workaround. The inside-fn `use bitcoin::hashes::{sha256,
Hash}` is correctly scoped and clippy-clean.

Post-fold verification:

```
$ cargo test -p md-codec --test bip341_wallet_vectors
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; …

$ cargo clippy -p md-codec --tests -- -D warnings
# … pre-existing 2 errors in src/error.rs (doc_lazy_continuation)
# remain, but no clippy error attributable to this commit
```

Pre-existing `src/error.rs` clippy errors are unchanged from main
and outside Phase 1's scope (they existed when v0.5.0 was tagged
with red CI).

## R1 nits (non-blocking)

- **N-1** SHA pin under CRLF: noted as a Windows-platform concern;
  current CI is Linux only. No fix required for v0.8.0.
- **N-2** Pre-existing clippy errors disposition (out of scope)
  remains defensible per the reviewer's own analysis.
- **N-3** `walk_tree` DFS order verified correct for all 4 tree
  shapes via reviewer's bitcoin-source cross-read. No issue.
- **N-4** `LeafVersion::from_consensus(0xFA)` verified safe (even,
  not annex, not TapScript → `Ok(FutureLeafVersion(0xFA))` per
  bitcoin v0.32 source line 1195). No issue.
- **N-5** Test isolation sound. No issue.

## R2 self-clear

Both Important folds applied and verified. R1 nits acknowledged as
non-blocking. **Phase 1 close gate: CLEAR.** Phase 4 (audit-matrix
successor) will reference this report as the Phase 1 R1 record.
