# Migration guide

Migration steps for upgrading between major releases of `wdm-codec`.

## v0.1.x → v0.2.0

v0.2.0 ships several breaking changes alongside additive features. This guide focuses on the breaking surface; for the full feature list see [`CHANGELOG.md`](./CHANGELOG.md).

### Wire format compatibility

**v0.1.0 backups remain valid v0.2.0 inputs.** The wire format for the no-fingerprints, no-taproot, no-correction-changes path is unchanged. `v0.1.json` test vectors verify byte-identical against v0.2.0 (`cargo run --bin gen_vectors -- --verify crates/wdm-codec/tests/vectors/v0.1.json` PASS). If you have v0.1.x-encoded backups in steel, they decode under v0.2.0 with the same output.

The breaking changes are at the **API surface**, not the wire format.

### §1 — `WalletPolicy::to_bytecode` signature change + `EncodeOptions` lost `Copy`

**Before (v0.1.x):**

```rust
let policy: WalletPolicy = "wsh(pk(@0/**))".parse()?;
let bytecode = policy.to_bytecode()?;
```

**After (v0.2.0):**

```rust
let policy: WalletPolicy = "wsh(pk(@0/**))".parse()?;
let bytecode = policy.to_bytecode(&EncodeOptions::default())?;
```

Callers needing no override should pass `&EncodeOptions::default()`. Callers wanting an override (a custom shared path or fingerprints) construct `EncodeOptions` via the builder:

```rust
use bitcoin::bip32::DerivationPath;
use std::str::FromStr;

let opts = EncodeOptions::default()
    .with_shared_path(DerivationPath::from_str("m/48'/0'/0'/2'")?)
    .with_force_chunking(true);
let bytecode = policy.to_bytecode(&opts)?;
```

#### `EncodeOptions: !Copy`

`EncodeOptions` no longer derives `Copy` because the new `shared_path: Option<DerivationPath>` field's type isn't `Copy`. It still derives `Clone + Default + PartialEq + Eq`.

**Before (v0.1.x):**

```rust
fn use_options(opts: EncodeOptions) {  // takes by value, Copy semantics
    let bytecode_a = policy_a.to_bytecode_with_opts(opts);
    let bytecode_b = policy_b.to_bytecode_with_opts(opts);  // re-uses by Copy
}
```

**After (v0.2.0):**

```rust
fn use_options(opts: &EncodeOptions) {  // take by reference, the standard pattern
    let bytecode_a = policy_a.to_bytecode(opts)?;
    let bytecode_b = policy_b.to_bytecode(opts)?;  // re-uses by &
}
```

Callers that genuinely need to mutate per-call: `.clone()` explicitly.

### §2 — `WalletPolicy` `PartialEq` semantics

`WalletPolicy` gained an internal `decoded_shared_path: Option<DerivationPath>` field (Phase A). The field is populated by `from_bytecode` (so `Some(...)`) and not by `parse()` / `FromStr` (so `None`). The derived `PartialEq` compares all fields; therefore two **logically-equivalent** policies — one constructed each way — now compare **unequal**.

**Before (v0.1.x):**

```rust
let a: WalletPolicy = "wsh(pk(@0/**))".parse()?;
let bytecode = a.to_bytecode()?;
let b = WalletPolicy::from_bytecode(&bytecode)?;
assert_eq!(a, b);  // worked in v0.1.x for template-only policies
```

**After (v0.2.0):**

```rust
let a: WalletPolicy = "wsh(pk(@0/**))".parse()?;
let bytecode = a.to_bytecode(&EncodeOptions::default())?;
let b = WalletPolicy::from_bytecode(&bytecode)?;
// assert_eq!(a, b);  // FAILS — a.decoded_shared_path = None; b.decoded_shared_path = Some(...)

// Recommended workaround: compare canonical string form
assert_eq!(a.to_canonical_string(), b.to_canonical_string());
```

`.to_canonical_string()` is the construction-path-agnostic equality test; it serializes both policies to the same BIP 388 string form regardless of construction history.

If you derived `Hash` on a wrapper struct containing `WalletPolicy`, the same caveat applies — the new field participates in the hash. Switch to a manual `Hash` impl that ignores `decoded_shared_path`, or to using the canonical string as the hash key.

### §3 — Header bit 2 `PolicyScopeViolation` no longer fires

v0.1 rejected any bytecode with header bit 2 = 1 (the fingerprints flag) with `Error::PolicyScopeViolation("v0.1 does not support the fingerprints block; use the no-fingerprints form (header byte 0x00)")`. v0.2 implements the fingerprints block; that error variant for that path no longer fires.

**Before (v0.1.x):**

```rust
match WalletPolicy::from_bytecode(&bytes) {
    Err(Error::PolicyScopeViolation(msg)) if msg.contains("fingerprints") => {
        // v0.1 used this as a way to detect "the input is from a v0.2+ encoder"
        eprintln!("This backup needs a v0.2+ wallet to read");
    }
    Ok(_) => { /* ... */ }
    Err(_) => { /* ... */ }
}
```

**After (v0.2.0):**

The header bit 2 = 1 path is now valid. Inspect the parsed fingerprints directly:

```rust
let result = decode(&strings, &DecodeOptions::new())?;
if let Some(fps) = &result.fingerprints {
    eprintln!("Backup carries {} fingerprints (privacy-sensitive)", fps.len());
} else {
    eprintln!("Backup has no fingerprints block");
}
```

`WdmBackup.fingerprints` (set by the encoder when `EncodeOptions::fingerprints` is `Some(_)`) and `DecodeResult.fingerprints` (populated by the decoder when header bit 2 = 1) are the new authoritative APIs.

### §4 — `force_chunking: bool` → `chunking_mode: ChunkingMode`

`pub fn chunking_decision(usize, bool)` is now `(usize, ChunkingMode)`; `EncodeOptions.force_chunking: bool` is renamed to `chunking_mode: ChunkingMode`.

**Before (v0.1.x):**

```rust
let plan = chunking_decision(bytecode_len, false)?;  // auto
let plan = chunking_decision(bytecode_len, true)?;   // force chunked

let opts = EncodeOptions { force_chunking: true, ..Default::default() };
```

**After (v0.2.0):**

```rust
let plan = chunking_decision(bytecode_len, ChunkingMode::Auto)?;
let plan = chunking_decision(bytecode_len, ChunkingMode::ForceChunked)?;

let opts = EncodeOptions { chunking_mode: ChunkingMode::ForceChunked, ..Default::default() };
```

For source compatibility, the `with_force_chunking(self, force: bool)` builder method **is preserved** as a `bool → enum` shim. Callers using the builder need no migration:

```rust
// Works in both v0.1.1 and v0.2.0
let opts = EncodeOptions::default().with_force_chunking(true);
```

### §5 — `Correction.corrected` value for checksum-region positions

v0.1 reported `Correction.corrected = 'q'` (the bech32 alphabet's first character) as a placeholder when the BCH ECC corrected a substitution **inside the 13/15-char checksum region**. v0.2 reports the **actual corrected character** at every position via the new `DecodedString::corrected_char_at(usize) -> char` accessor.

If you displayed `correction.corrected` to users as "we changed your transcribed character X to Y", the displayed Y is now correct for checksum-region corrections. If you had downstream code that assumed `correction.corrected == 'q'` meant "the correction is in the checksum region", switch to inspecting `correction.char_position` against the data-part length to determine region:

```rust
let data_part_len = chunk.raw.len() - "wdm1".len() - checksum_len;  // 13 or 15
let in_checksum_region = correction.char_position >= data_part_len;
```

### §6 — Test vector schema bumped 1 → 2

`crates/wdm-codec/tests/vectors/v0.1.json` is locked at SHA `1957b542ed0388b51f01a7b467c8e802942dc6d6507abffaefaf777c90f3cd2c` — the v0.1.0 contract. v0.2.0 ships an additional `crates/wdm-codec/tests/vectors/v0.2.json` at SHA `3c208300f57f1d42447f052499bab4bdce726081ecee139e8689f6dedb5f81cb`.

Schema 2 is **additive** over schema 1; readers that ignore unknown fields parse v0.2.json cleanly. New fields:

- `Vector.expected_fingerprints_hex: Option<Vec<String>>` — present iff the vector encoded with fingerprints
- `Vector.encode_options_fingerprints: Option<Vec<[u8; 4]>>` — the fingerprints to pass to `EncodeOptions::with_fingerprints` when regenerating
- `NegativeVector.provenance: Option<String>` — one-sentence note on how the negative fixture was generated

If your conformance suite verified against v0.1.json, that file is still authoritative; your suite continues to work. To exercise v0.2.0's new features (taproot, fingerprints), verify against v0.2.json additionally.

### §7 — Workspace `[patch]` block

v0.2.0 ships with the same workspace `[patch."https://github.com/apoelstra/rust-miniscript"]` block as v0.1.0 + v0.1.1, redirecting to a local fork at `../rust-miniscript-fork`. Downstream consumers of `wdm-codec` as a dependency need to either:

1. **Use a git-dep** with the same `[patch]` redirect in their workspace (see the comment in our root `Cargo.toml` for the exact form), OR
2. **Wait for `apoelstra/rust-miniscript#1` to merge upstream**, after which `wdm-codec-v0.2.1` will drop the `[patch]` block and bump the `rev =` pin to the merged SHA.

This is the same downstream UX as v0.1.x. Tracked as `external-pr-1-hash-terminals` in `design/FOLLOWUPS.md`.

### Compiling — quick checklist

If you're upgrading a v0.1.x consumer to v0.2.0, the minimum mechanical changes are:

1. Add `&EncodeOptions::default()` to every `policy.to_bytecode()` call site.
2. If you `match`'d on `Error::PolicyScopeViolation(msg) if msg.contains("fingerprints")`, replace with `result.fingerprints.is_some()` inspection on `WdmBackup` / `DecodeResult`.
3. If you used `EncodeOptions { force_chunking: true, ..Default::default() }` literal-init, change `force_chunking` to `chunking_mode: ChunkingMode::ForceChunked`. (If you used the builder, no change needed.)
4. If you compared `WalletPolicy` instances via `==` across `parse()` and `from_bytecode` construction paths, switch to comparing via `.to_canonical_string()`.
5. If you took `EncodeOptions` by value into a closure, switch to `&EncodeOptions` or add explicit `.clone()`.

`cargo build` will surface the items needing migration; the compile errors map directly to the migration steps.
