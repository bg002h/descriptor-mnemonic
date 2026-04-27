# Phase v0.2 A — Bucket B: bytecode round-trip path mismatch

**Status:** DONE
**Closed short-id:** `6a-bytecode-roundtrip-path-mismatch`
**Commit:** `86ca5df` — `fix(policy)!: WalletPolicy.decoded_shared_path enables first-pass byte-stable round-trip (closes 6a-bytecode-roundtrip-path-mismatch)`

## Summary

Made `encode → decode → encode` byte-stable on the FIRST round-trip pass for
template-only policies. Added a private `decoded_shared_path:
Option<DerivationPath>` field to `WalletPolicy`, populated by `from_bytecode`
from the on-wire `Tag::SharedPath` declaration and consulted by `to_bytecode`
under the Phase A precedence rule:

1. `self.decoded_shared_path` (`Some` after `from_bytecode`)
2. `self.shared_path()` (real-keys descriptor case)
3. BIP 84 mainnet fallback (template-only via `parse()`)

Phase B will layer `EncodeOptions::shared_path` as the highest-precedence
override on top of this stack, per the resolved design in
`design/IMPLEMENTATION_PLAN_v0.2.md` §"Phase A".

## Files changed

- `crates/wdm-codec/src/policy.rs`
  - Added `decoded_shared_path: Option<DerivationPath>` field to `WalletPolicy`
    struct with rustdoc explaining provenance and precedence.
  - `FromStr::from_str` and the existing struct-literal paths now initialize the
    field to `None`.
  - `from_bytecode` keeps the previously-discarded `_shared_path` from
    `decode_declaration` and stashes it on the returned policy as
    `Some(shared_path)`.
  - `to_bytecode` selects the path declaration via the Phase A precedence
    chain (`decoded_shared_path > shared_path() > BIP 84`).
  - Updated `to_bytecode` rustdoc: replaced the v0.1 D-10 fallback section
    with a Phase A precedence section that points forward to Phase B's
    `EncodeOptions::shared_path` extension.
  - Added two inline tests:
    - `from_bytecode_populates_decoded_shared_path_consulted_by_to_bytecode`
      — round-trips a hand-built bytecode whose path declaration is
      `m/48'/0'/0'/2'` (BIP 48 named indicator `0x05`) and asserts both that
      `decoded_shared_path` is `Some` after decode AND that the re-encoded
      bytes are byte-identical to the original. The chosen path
      distinguishes the correct behavior from both the BIP 84 default
      (`0x03`) and any leakage of the dummy-key origin paths
      (`m/44'/0'/0'`).
    - `parse_does_not_set_decoded_shared_path` — pins the FromStr invariant.

- `crates/wdm-codec/tests/corpus.rs`
  - Tightened `corpus_encode_decode_encode_idempotency`: the test now
    asserts FIRST-pass raw-byte equality between `encode1` and `encode2`
    (new Invariant 2). The old "second-pass determinism" check is
    preserved (renamed to Invariant 4) as a regression guard against
    future encoder non-determinism.
  - Replaced the comment block that previously documented the v0.1 D-10
    deferral ("raw-byte equality across the FIRST and SECOND encodes is
    NOT asserted here") with a Phase A reference.

## Quality gates

All required gates green:

| Gate | Result |
|---|---|
| `cargo test -p wdm-codec` | 463 tests passing across lib + integration + doctests (370 lib + 88 integration + 5 doctests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all --check` | clean |
| `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --document-private-items` | clean |
| `cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` | PASS (10 positive, 30 negative) |

## Test count

The pre-Bucket-B baseline was 368 lib tests; this bucket adds 2 inline
tests in `policy.rs` (`from_bytecode_populates_decoded_shared_path_consulted_by_to_bytecode`
and `parse_does_not_set_decoded_shared_path`), bringing the lib total to
370. The corpus integration test count is unchanged (the existing
idempotency test was tightened in place).

## Verification of key behavior

The new inline test exercises the precise byte-equality property the
fix delivers: a hand-built bytecode `[0x00, 0x33, 0x05, 0x05, 0x0C,
0x1B, 0x32, 0x00]` (header + SharedPath(BIP 48 indicator) + Wsh +
Check + PkK + Placeholder + index 0) decodes and re-encodes to the
exact same byte sequence, with the path-indicator byte at offset 2
preserved as `0x05` (not the BIP 84 fallback `0x03` and not the
dummy-key-origin-derived `m/44'/0'/0'`).

The corpus idempotency test exercises the same property over all 11
canonical corpus policies via the full `encode → decode → encode`
pipeline (not just the lower-level `to_bytecode`/`from_bytecode`
methods); it now asserts byte-equality on the FIRST pass between the
two encoded `WdmBackup.chunks` raw strings, which is the strongest
form of byte-stability achievable for template-only policies.

## Public-API impact

`WalletPolicy` is `#[non_exhaustive]`, so the field addition is not
externally observable in pattern matches. The behavioral change is in
`to_bytecode`: for `WalletPolicy` values produced by `from_bytecode`,
the emitted shared-path declaration now matches the on-wire path
instead of the dummy-key origin fallback. This is the breaking aspect
that justifies the `!` in the conventional-commits subject. Callers
who relied on the v0.1 behavior of "decode-then-re-encode emits the
dummy-key origin path" must migrate; migration is documented inline
in the `to_bytecode` rustdoc and will be summarized in `MIGRATION.md`
at Phase G.

## Coordination notes (parallel batch context)

The working tree contained uncommitted in-progress edits from BUCKET A
(`crates/wdm-codec/src/chunking.rs`, `crates/wdm-codec/src/lib.rs`,
`crates/wdm-codec/src/options.rs`) at the time this bucket was
dispatched. To execute my disjoint-file scope without entangling
Bucket A's WIP into my commit, I temporarily stashed those edits,
ran my implementation + test cycle + quality gates against a clean
underlying tree, committed Bucket B, then restored Bucket A's WIP to
the working tree. My commit (`86ca5df`) touches only the two files in
my scope (`crates/wdm-codec/src/policy.rs`,
`crates/wdm-codec/tests/corpus.rs`); no Bucket A files appear in the
diff.

After the controller's review checkpoint, Bucket A will land its own
commit on top of mine. The two buckets are file-disjoint per the v0.2
plan, so a clean rebase or fast-forward should suffice; no merge
conflict on shared files is expected.

## Deferred minor items

None. Bucket B closes one item (`6a-bytecode-roundtrip-path-mismatch`)
and surfaces no new ones. Per the parallel-batch rule the controller
aggregates the FOLLOWUPS.md update.

## Notes for the controller

- This commit is intentionally `!`-flagged under conventional commits
  because callers exercising `from_bytecode → to_bytecode` will see a
  different shared-path byte than they did in v0.1.1. The pattern of
  observable bytes shifts from `m/44'/0'/0'` (dummy-key origin leakage)
  to whatever path was on the wire — which is the correct behavior, but
  is observable to consumers.
- The `to_bytecode` doc-comment now references
  `design/IMPLEMENTATION_PLAN_v0.2.md` for the Phase B
  `EncodeOptions::shared_path` extension; once Phase B lands, the
  precedence list in that rustdoc should grow a `0.` entry for the
  `EncodeOptions::shared_path` override.
