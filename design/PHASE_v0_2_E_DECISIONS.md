# Phase v0.2-E — Fingerprints block decisions

Resolves the open design questions for `p2-fingerprints-block` in advance of implementer dispatch.

## E-1 — `EncodeOptions::fingerprints` type

**Decision**: `pub fingerprints: Option<Vec<bitcoin::bip32::Fingerprint>>`

**Rationale**: `bitcoin::bip32::Fingerprint` is the standard 4-byte master-key-fingerprint type; reuses the existing dependency (`bitcoin = "0.32"`) without introducing a local newtype. `Vec` indexed by placeholder index (so `fps[i]` is the fingerprint for `@i`). Default `None` (no fingerprints block; matches v0.1 wire output).

**Builder method**: `pub fn with_fingerprints(self, fps: Vec<Fingerprint>) -> Self` — mirrors the `with_force_chunking` / `with_shared_path` builder pattern.

## E-2 — `WdmBackup.fingerprints` field shape

**Decision**: add `pub fingerprints: Option<Vec<Fingerprint>>` field to `WdmBackup` (`crates/wdm-codec/src/policy.rs:497`). Additive — `WdmBackup` is `#[non_exhaustive]`.

**Semantics**: `None` iff the bytecode header bit 2 was 0; `Some(fps)` iff bit 2 was 1 and the fingerprints block parsed successfully. The vec is in placeholder index order (`fps[i]` for `@i`), per BIP §"Fingerprints block".

**No accessor method needed**: the field is `pub` (matching the existing `chunks: Vec<EncodedChunk>` style).

## E-3 — Encoder default behavior

**Decision**:
- `EncodeOptions::fingerprints == None` → emit header byte `0x00` (bit 2 = 0); no fingerprints block. **Preserves v0.1 wire output for callers who don't opt in.**
- `EncodeOptions::fingerprints == Some(fps)` → validate `fps.len() == placeholder_count(policy)`; emit header byte `0x04` (bit 2 = 1); emit fingerprints block (`Tag::Fingerprints` + count byte + 4*count bytes) immediately after the path declaration, per BIP placement rule.

If `fps.len() != placeholder_count(policy)`, the encoder returns `Error::FingerprintsCountMismatch { expected, got }` (E-5).

## E-4 — Decoder validation

**Decision**: when header bit 2 is set, the decoder reads the next bytes and:

1. Expects exactly `Tag::Fingerprints = 0x35` as the first byte after the path declaration. If not, `Error::InvalidBytecode { kind: UnexpectedTag { expected: 0x35, got: <byte> } }`.
2. Reads the count byte. Validates `count == max(@i in template) + 1` (BIP MUST clause). If not, `Error::FingerprintsCountMismatch { expected: <derived from template>, got: <count from bytecode> }`.
3. Reads `count * 4` bytes; if the stream truncates mid-block, `Error::InvalidBytecode { kind: UnexpectedEnd }`.
4. Constructs `Vec<Fingerprint>` from the bytes (`Fingerprint::from(<[u8; 4]>)` or equivalent).
5. Continues parsing the rest of the bytecode (template tree) as before.

**Edge case**: header bit 2 = 0 + bytecode contains a `0x35` tag elsewhere. The decoder never looks for `0x35` if bit 2 = 0; if `0x35` appears mid-stream as an operator tag, it falls through to `UnknownTag(0x35)` (existing behavior). No new handling needed.

## E-5 — New error variant

**Decision**: `Error::FingerprintsCountMismatch { expected: usize, got: usize }`

**Rationale**: the BIP MUST clause for count == max(@i)+1 is an unambiguous constraint deserving its own variant. Reusable from both encode (caller-supplied vec wrong length) and decode (bytecode count byte wrong) paths. Reusing `InvalidBytecode` would conflate a structural error with a constraint violation.

**Register**: add to `tests/error_coverage.rs::ErrorVariantName` enum + add a `rejects_fingerprints_count_mismatch` conformance test.

**Other malformed cases** reuse existing variants (`InvalidBytecode { kind: UnexpectedTag }` for missing `0x35`; `InvalidBytecode { kind: UnexpectedEnd }` for truncation). No additional variants needed.

## E-6 — Remove the v0.1 PolicyScopeViolation rejection

**Decision**: at `crates/wdm-codec/src/policy.rs:416-420`, remove:

```rust
if header.fingerprints() {
    return Err(Error::PolicyScopeViolation(
        "v0.1 does not support the fingerprints block; use the no-fingerprints form (header byte 0x00)".to_string(),
    ));
}
```

Replace with the actual decode logic per E-4. Update the `from_bytecode` rustdoc Errors section (`policy.rs:402-405`) to drop the PolicyScopeViolation/fingerprints clause.

**Test cleanup**: search `crates/wdm-codec/` for tests that assert `PolicyScopeViolation` for header byte `0x04`. Either delete them (the path is now valid) or convert them to **positive** round-trip tests of the new fingerprints-block path. Phase 5-B added one such test at `policy.rs:787-791` (search "0x04 = version 0, fingerprints flag set"); convert it.

## E-7 — Privacy clause

**Decision**:
- `EncodeOptions::fingerprints` rustdoc gets a `# Privacy` section: "Fingerprints leak which seeds match which `@i` placeholders. The fingerprints block is **optional** — only set this field if the recovery flow benefits from the disclosure (e.g., a multisig recovery tool that needs to match seeds to placeholder positions before deriving). Recovery tools SHOULD warn before encoding fingerprints, especially for solo-user single-seed wallets where the leak is unnecessary."
- BIP §"Fingerprints block" (line 397+) gets a corresponding privacy clause as a normative paragraph.

## E-8 — `Tag::Fingerprints = 0x35`

**Decision**: add to `crates/wdm-codec/src/bytecode/tag.rs::Tag` enum. Currently the enum's rustdoc at line 9 says "Tag 0x35 (fingerprints block) is reserved for v0.2 and is not in the v0.1 enum." — Phase E adds it. The enum is `#[non_exhaustive]` so the addition is additive.

**Update**: also add `0x35 => Some(Tag::Fingerprints)` to the `Tag::from_byte` match arm.

## E-9 — Behavioral break + MIGRATION.md note

**Decision**: this is a behavioral break: v0.1 callers pattern-matching on `Error::PolicyScopeViolation` for header bit 2 = 1 will no longer see that error variant. File a FOLLOWUPS entry: `phase-e-fingerprints-behavioral-break-migration-note` for the Phase G MIGRATION.md addition.

**Per the v0.2 plan note** at line 84 ("any v0.1 caller pattern-matching on PolicyScopeViolation for header bit 2 inputs will no longer see that error variant... The exhaustiveness gate at tests/error_coverage.rs may need its rejects_* test for that path retired or repurposed"), this is acknowledged in the plan as part of v0.2 scope. The MIGRATION.md tracker captures it for the Phase G release-prep doc.

## E-10 — CLI exposure

**Decision**: defer. The library API (`EncodeOptions::fingerprints`, `WdmBackup::fingerprints`) is the Phase E deliverable. A `wdm encode --fingerprint @0=<hex> --fingerprint @1=<hex>` CLI flag is a v0.2-nice-to-have follow-up; file as `phase-e-cli-fingerprint-flag`.

**Phase E CLI behavior**: `bin/wdm.rs::cmd_encode` does NOT add a fingerprint flag in this phase. The library API is fully functional via direct `EncodeOptions::default().with_fingerprints(...)` use; CLI users get fingerprints support in v0.2.1 or later.

## E-11 — BIP edits required

In `bip/bip-wallet-descriptor-mnemonic.mediawiki`:

1. **§"Fingerprints block (optional)"** (line 397): heading is fine. Body already specifies the format. Add:
   - A privacy clause (E-7) as a normative paragraph
   - A concrete byte-layout example for a 2-key policy with both fingerprints provided, regenerated via `cargo run --bin wdm bytecode 'wsh(multi(2, @0/**, @1/**))'` after running with fingerprints supplied (since `wdm bytecode` doesn't take fingerprints today, the agent may need to construct the example via a unit test that emits the bytecode hex and copy it into the BIP).
2. **Tag table at line 374** (`0x35` Fingerprints block): note "now implemented in v0.2" if the table style permits, or leave as-is.
3. **Header byte values** at line 220 (where valid v0 values `0x00` and `0x04` are listed): no change — both were already documented.

## E-12 — Placeholder-count helper

**Decision**: the encoder needs to know the placeholder count (max `@i` index + 1) to validate `fps.len()` and to emit the count byte. The Phase 5-B work (commit `48809b7`) deleted the old `count_placeholder_indices` byte-scan in favor of re-deriving from the descriptor structure. The Phase E agent should:

1. Search for an existing helper in `crates/wdm-codec/src/policy.rs` or `crates/wdm-codec/src/bytecode/` that returns the placeholder count from a `WalletPolicy` or `Descriptor<DescriptorPublicKey>`.
2. If one exists (e.g., from `from_descriptor`'s key_info reconstruction), reuse.
3. If not, write a small one in `policy.rs` — walk the descriptor AST, collect unique `@i` indices, return `max + 1` (or the `key_info.len()` if available).

The helper does NOT need to be `pub` (controller's call); `pub(crate)` is fine.

## Out of scope (deferred)

- CLI `--fingerprint` flag (E-10 → `phase-e-cli-fingerprint-flag`)
- MIGRATION.md write (E-9 → `phase-e-fingerprints-behavioral-break-migration-note`, Phase G)
- Multi-fingerprint-per-placeholder (e.g., for joint-spend signers) — the BIP spec is one-fingerprint-per-placeholder; multi-fingerprint is not on any roadmap
- Fingerprint validation against actual master keys — fingerprints are unverifiable without the private keys, so the encoder/decoder treats them as opaque 4-byte values; verification is a recovery-tool concern

## Reference

- BIP draft `bip/bip-wallet-descriptor-mnemonic.mediawiki:374` (tag 0x35 row), `:397-409` (Fingerprints block section)
- `crates/wdm-codec/src/bytecode/header.rs` — `FINGERPRINTS_BIT = 0x04`, `Header::new_v0(fingerprints: bool)` (already in place)
- `crates/wdm-codec/src/bytecode/tag.rs:9` — comment noting `0x35` reserved for v0.2
- `crates/wdm-codec/src/policy.rs:416-420` — current rejection site (Phase E removes)
- `crates/wdm-codec/src/policy.rs:497-505` — `WdmBackup` struct (Phase E adds field)
- `design/IMPLEMENTATION_PLAN_v0.2.md` Phase E section (the precedence rule + scope summary)
