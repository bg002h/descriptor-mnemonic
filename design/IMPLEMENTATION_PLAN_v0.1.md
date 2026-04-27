# wdm-codec v0.1 — Implementation Design

**Status:** Pre-Draft, AI only, not yet human reviewed.
**Created:** 2026-04-26.
**Author:** bg002h.

This document specifies the v0.1 reference implementation of the Wallet
Descriptor Mnemonic (WDM) format. It is the validated output of an
extended brainstorming and review process (multiple agent passes) and
is intended to drive the implementation work that follows.

The companion BIP draft is at `bip/bip-wallet-descriptor-mnemonic.mediawiki`.
The format-level design rationale is at `design/POLICY_BACKUP.md`. This
document is implementation-specific.

---

## 1. Scope and goals

**v0.1 scope:**

- Encoder + decoder for BIP 388 wallet policies (`WalletPolicy → string(s)`
  and `string(s) → WalletPolicy`)
- Single-string and chunked encoding
- Basic erasure decoding (user-supplied positions)
- `wsh()` only — no taproot
- No master fingerprints block
- No guided-recovery flow (BCH ECC only; structural-knowledge recovery
  is v0.3)

**v0.1 first consumer:** the user generating concrete test vectors for
the BIP draft. Test vectors are the load-bearing artifact; the API is
shaped around producing them.

**v0.1 explicitly out-of-scope (deferred to v0.2 / v0.3):**

- Taproot (`tr()` + tap-miniscript)
- Master fingerprints (tag `0x35`)
- Guided recovery (structure-aware candidate filtering, confidence
  calibration, blockchain verification)
- Foreign xpubs (Tier 2 Xpub Cards) and per-`@i` paths
- MuSig2 (`musig()` placeholder)
- Property-based testing (`proptest`)
- Fuzzing (`cargo-fuzz`)
- Cross-platform CI matrix beyond minimal sanity
- Performance benchmarks
- Mutation testing
- Cross-implementation interop testing

---

## 2. Architecture

### Crate layout

```
descriptor-mnemonic/
├── bip/                                   (existing BIP draft)
├── design/                                (existing design docs)
├── crates/
│   └── wdm-codec/
│       ├── Cargo.toml                     rust-version = "1.85"
│       ├── README.md
│       ├── src/
│       │   ├── lib.rs                     public API; re-exports
│       │   ├── policy.rs                  thin adapter over miniscript::descriptor::WalletPolicy
│       │   ├── bytecode/
│       │   │   ├── mod.rs                 top-level encode/decode
│       │   │   ├── tag.rs                 Tag enum 0x00–0x33; vendored from descriptor-codec
│       │   │   ├── key.rs                 WdmKey enum
│       │   │   ├── path.rs                path-dictionary stub for v0.2 expansion
│       │   │   ├── encode.rs              AST → bytes; reimplemented over WdmKey
│       │   │   └── decode.rs              bytes → AST; reimplemented over WdmKey
│       │   ├── encoding.rs                bech32 + BCH polynomials; HRP "wdm"
│       │   ├── chunking.rs                ChunkHeader + multi-string assembly + cross-chunk hash
│       │   ├── wallet_id.rs               Wallet ID derivation; known-vector unit test lives here
│       │   ├── error.rs                   Error enum + BytecodeErrorKind sub-enum; thiserror 2
│       │   ├── vectors.rs                 TestVectorFile schema (used by binary + tests)
│       │   └── bin/
│       │       ├── wdm.rs                 CLI binary
│       │       └── gen_vectors.rs         test-vector generator binary
│       └── tests/
│           ├── common/
│           │   └── mod.rs                 round_trip_assert, corrupt_n, load_vector
│           ├── corpus.rs                  C1–C5, E10, E12, E14 round-trip
│           ├── upstream_shapes.rs         9 descriptor-codec shapes rewritten in @i form
│           ├── chunking.rs                4 named hash tests
│           ├── ecc.rs                     deterministic BCH stress + fixed-seed sanity loop
│           ├── conformance.rs             18+ macro-expanded rejects_* cases
│           ├── vectors_schema.rs          deserialize committed JSON, assert structural invariants
│           └── error_coverage.rs          strum::EnumIter exhaustiveness over Error variants
└── Cargo.toml                             workspace root, single member
```

### Component responsibilities

| Module | Responsibility |
|---|---|
| `policy` | BIP 388 wallet policy types; `FromStr` parse via `miniscript::descriptor::WalletPolicy`; canonicalize (whitespace, `/**` expansion, hardened-component spelling); validate v0.1 scope (wsh-only, no foreign xpubs, ≤20 keys, contiguous indices, miniscript `sanity_check`) |
| `bytecode` | Encode AST → canonical bytecode; decode bytecode → AST. Forked from `descriptor-codec`. Encoder is generic over `WdmKey` (placeholder in v0.1; foreign xpub variant reserved for v1+) |
| `bytecode::tag` | Tag enum 0x00–0x33; values 0x00–0x31 vendored verbatim from descriptor-codec; 0x32 placeholder, 0x33 shared-path declaration. Tag 0x35 (fingerprints) reserved for v0.2 — NOT in v0.1's enum |
| `bytecode::key` | `WdmKey` enum with `Placeholder(u8)` variant; `Key(DescriptorPublicKey)` variant exists but unused in v0.1 |
| `bytecode::path` | Path dictionary const table; placeholder for v0.2 expansion |
| `encoding` | bech32 alphabet (32 chars) + BCH polynomials (regular `BCH(93,80,8)` and long `BCH(108,93,8)`, generator polynomials per BIP 93 §"Generator polynomial"; quoted inline in this module's source as named `const` arrays for traceability); HRP `"wdm"`; length validation (reject 94–95); case enforcement |
| `chunking` | `ChunkHeader` struct with explicit byte layout; cross-chunk SHA-256[0..4] hash; chunking decision (single-string capacities: ≤48 B regular, ≤56 B long; else chunked with per-chunk fragment ≤45 B regular or ≤53 B long); reassembly with wallet_id/count/index validation |
| `wallet_id` | 16-byte SHA-256[0..16] of canonical bytecode → 12-word BIP 39 mnemonic (Tier-3 Wallet ID); 20-bit chunk-header `wallet_id` is the first 20 bits of the same SHA-256 (truncation relationship). `WalletId::to_words(&self) -> WalletIdWords` exposes the BIP-39 conversion. |
| `error` | `Error` enum with `BytecodeErrorKind` sub-enum; granular variants for each rejection mode in the BIP |
| `vectors` | `TestVectorFile` / `Vector` / `NegativeVector` schema structs; `pub` (not feature-gated) since serde is already a dep |
| `bin::wdm` | CLI subcommands: `encode`, `decode`, `verify`, `inspect`, `bytecode`, `vectors` |
| `bin::gen_vectors` | `--output <path>` writes; `--verify <path>` deserialize-then-reserialize-then-typed-compare |

### Dependencies

- `bitcoin = "0.32"` — re-export of `bitcoin::hashes::sha256`, `bitcoin::hex`, BIP 32 types
- `miniscript = "12"` — `descriptor::WalletPolicy`, `KeyExpression`
- `bech32 = "0.11"` — alphabet conversion + Checksum trait extended for WDM polynomials
- `clap = "4"` with `derive` — CLI argument parsing for both binaries
- `serde` + `serde_json` — `TestVectorFile` schema + `vectors_schema.rs` deserialization
- `thiserror = "2"` — Error enum derives
- `indexmap = "2"` — deterministic insertion-order iteration in path emission
- `strum = "0.26"` (dev-dependency) — `EnumIter` for `Error` exhaustiveness CI gate
- `rand = "0.8"` (dev-dependency) — `SmallRng` for fixed-seed BCH stress
- `hex = "0.4"` (dev-dependency, transitive via bitcoin) — test fixtures

No `descriptor-codec` dependency — the fork lives inline in `bytecode/`.

---

## 3. Public API

### Entry types

```rust
// Each type's authoritative definition is in its module of record;
// the crate root only re-exports for convenient `wdm_codec::Foo` paths.
// Below shows definitions for clarity; in the actual source the type
// lives in only one place.

// In src/encoding.rs:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BchCode { Regular, Long }

// In src/lib.rs:
pub use crate::encoding::BchCode;
pub use crate::policy::{WalletPolicy, WalletId, WalletIdWords, ChunkWalletId};
pub use crate::chunking::Correction;

// In src/policy.rs:
pub struct WalletPolicy { /* thin adapter */ }

#[non_exhaustive]
pub struct WdmBackup {
    pub chunks: Vec<EncodedChunk>,
    pub wallet_id_words: WalletIdWords,
}
impl WdmBackup {
    pub fn wallet_id(&self) -> WalletId;
}

#[non_exhaustive]
pub struct EncodedChunk {
    pub raw: String,
    pub chunk_index: u8,
    pub total_chunks: u8,
    pub code: BchCode,
}

#[non_exhaustive]
pub struct DecodeReport {
    pub outcome: DecodeOutcome,
    pub corrections: Vec<Correction>,
    pub verifications: Verifications,
    pub confidence: Confidence,
}

#[non_exhaustive]
pub enum DecodeOutcome { Clean, AutoCorrected, Failed }

// NOT #[non_exhaustive] — callers SHOULD match exhaustively per BIP requirement
pub struct Verifications {
    pub cross_chunk_hash_ok: bool,
    pub wallet_id_consistent: bool,
    pub total_chunks_consistent: bool,
    pub bytecode_well_formed: bool,
    pub version_supported: bool,
}

#[non_exhaustive]
pub enum Confidence { Confirmed, High, Probabilistic, Failed }

#[non_exhaustive]
pub struct DecodeResult {
    pub policy: WalletPolicy,
    pub report: DecodeReport,
}

pub struct Correction {
    pub chunk_index: u8,
    pub char_position: usize,
    pub original: char,
    pub corrected: char,
}

/// Tier-3 Wallet ID: 16 bytes (128 bits), derived from canonical bytecode.
pub struct WalletId([u8; 16]);
impl Display for WalletId { /* hex */ }
impl LowerHex for WalletId { /* ... */ }
impl AsRef<[u8]> for WalletId { /* ... */ }
impl From<[u8; 16]> for WalletId { /* ... */ }
impl WalletId {
    pub fn as_bytes(&self) -> &[u8; 16];
    /// Convert to the 12-word BIP 39 mnemonic (Tier-3 representation).
    pub fn to_words(&self) -> WalletIdWords;
    /// Truncate to the 20-bit chunk-header form (used in chunked encoding).
    pub fn truncate(&self) -> ChunkWalletId;
}

pub struct WalletIdWords([String; 12]);
impl Display for WalletIdWords { /* space-joined */ }
impl IntoIterator for WalletIdWords { /* ... */ }

/// 20-bit chunk-header wallet identifier. Stored as `u32` with only the
/// lower 20 bits significant; serialized as 4 bech32 characters.
pub struct ChunkWalletId(u32);  // upper 12 bits MUST be zero; debug-asserted
impl ChunkWalletId {
    pub const MAX: u32 = (1 << 20) - 1;
    pub fn new(bits: u32) -> Self;  // panics if bits > MAX
    pub fn as_u32(&self) -> u32;
}

pub struct WalletIdSeed([u8; 4]);
impl Debug for WalletIdSeed { /* redacted */ }
impl Hash for WalletIdSeed;
impl From<u32> for WalletIdSeed;
impl From<[u8; 4]> for WalletIdSeed;

#[non_exhaustive]
#[derive(Default)]
pub struct EncodeOptions {
    pub force_chunking: bool,
    pub force_long_code: bool,
    pub wallet_id_seed: Option<WalletIdSeed>,
}

/// Opaque options struct. v0.1 has no public knobs; the type exists so
/// v0.2+ can add builder methods without breaking existing call sites.
/// Erasure decoding is supported internally for use by guided recovery
/// (v0.3); v0.1 callers do not invoke that path.
#[derive(Default)]
pub struct DecodeOptions {
    erasures: Vec<(usize, usize)>,  // private; v0.3 will expose via with_erasure_hint
}
impl DecodeOptions {
    pub fn new() -> Self;
}
// (No public builder methods in v0.1; v0.3 adds with_erasure_hint when guided
// recovery lands. The struct is forward-compatible because all fields are
// private and Default is the sole public constructor.)
```

### Free functions

```rust
pub fn encode(
    policy: &WalletPolicy,
    options: &EncodeOptions,
) -> Result<WdmBackup, Error>;

pub fn decode(
    strings: &[&str],
    options: &DecodeOptions,
) -> Result<DecodeResult, Error>;

pub fn encode_bytecode(policy: &WalletPolicy) -> Result<Vec<u8>, Error>;
pub fn decode_bytecode(bytes: &[u8]) -> Result<WalletPolicy, Error>;
pub fn compute_wallet_id(policy: &WalletPolicy) -> WalletId;
```

### Methods on `WalletPolicy`

```rust
impl std::str::FromStr for WalletPolicy {
    type Err = Error;
}

impl WalletPolicy {
    pub fn to_canonical_string(&self) -> String;
    pub fn key_count(&self) -> usize;
    pub fn shared_path(&self) -> Option<&DerivationPath>;
    pub fn to_bytecode(&self) -> Result<Vec<u8>, Error>;
    pub fn from_bytecode(bytes: &[u8]) -> Result<Self, Error>;
    #[doc(hidden)]
    pub fn inner(&self) -> &miniscript::descriptor::WalletPolicy;
}
```

### Error type

```rust
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    InvalidHrp(String),
    MixedCase,
    InvalidStringLength(usize),
    BchUncorrectable,
    InvalidBytecode { offset: usize, kind: BytecodeErrorKind },
    UnsupportedVersion(u8),
    UnsupportedCardType(u8),
    ChunkIndexOutOfRange { index: u8, total: u8 },
    DuplicateChunkIndex(u8),
    WalletIdMismatch { expected: ChunkWalletId, got: ChunkWalletId },
    TotalChunksMismatch { expected: u8, got: u8 },
    PolicyScopeViolation(String),
    CrossChunkHashMismatch,
    PolicyParse(String),
    Miniscript(String),  // wrapped, NOT #[from] miniscript::Error
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BytecodeErrorKind {
    UnknownTag(u8),
    Truncated,
    VarintOverflow,
    MissingChildren { expected: usize, got: usize },
    UnexpectedEnd,
    TrailingBytes,
}

impl From<miniscript::Error> for Error {
    fn from(e: miniscript::Error) -> Self {
        Error::Miniscript(e.to_string())
    }
}
```

### CLI commands

```bash
wdm encode <policy> [--path bip48|0x05|"m/48'/0'/0'/2'"] [--force-chunked] [--seed 0xdeadbeef]
wdm decode <string> [<string>...]
wdm verify <string> [<string>...] --policy <policy>
wdm inspect <string>                 # prints DecodeReport, no full decode
wdm bytecode <policy>                # hex-dump canonical bytecode
wdm vectors > vectors.json           # delegates to gen_vectors binary

gen_vectors --output <path>          # writes
gen_vectors --verify <path>          # read-only, deserialize-then-reserialize-then-typed-compare
```

---

## 4. Data flow

### Encode pipeline

```
Stage 1   BIP 388 policy string → WalletPolicy   (via miniscript::WalletPolicy::from_str)
Stage 1.5 canonicalize in place                   (whitespace, /** expansion, hardened spelling, key vector sort)
Stage 2   walk AST, emit canonical bytecode       (header + path-decl + tree)
Stage 3   chunking decision                       (≤48B → single regular; ≤56B → single long; else chunked)
Stage 4   if chunked:                             append SHA-256(bytecode)[0..4]; split fragments;
                                                   prepend ChunkHeader { version, type, wallet_id, count, index }
                                                   wallet_id (chunk-header, 20-bit ChunkWalletId): see "Wallet ID semantics" below
Stage 5   for each chunk:                         bech32 8→5 conversion; BCH checksum (regular or long polynomial);
                                                   prepend "wdm" + "1" separator; append checksum
Stage 6   Tier-3 wallet ID derivation             16-byte SHA-256(canonical_bytecode)[0..16];
                                                   ALWAYS content-derived (never overridden by seed);
                                                   converted to 12-word BIP 39 mnemonic for the WdmBackup
Result    WdmBackup { chunks, wallet_id_words }
```

### Decode pipeline

```
Stage 1   per-string parse                        case check; HRP="wdm"; length valid (reject 94–95); identify code variant
Stage 2   BCH validate + correct                  zero syndromes → Clean; ≤4 substitutions → AutoCorrected (record positions);
                                                   >4 substitutions → Error::BchUncorrectable
Stage 3   header parse                            version (reject ≠ 0); type (0=single, 1=chunked); if chunked: wallet_id, count, index
Stage 4   reassembly (chunked only)               verify wallet_id consistent; verify total_chunks consistent;
                                                   sort by index, no gaps/dupes; concat fragments;
                                                   strip trailing 4 bytes; verify SHA-256(reassembled)[0..4] match
Stage 5   bytecode decode                         read header byte, path declaration, walk operator tree;
                                                   reject trailing bytes; reject unknown tags
Stage 6   verifications + report                  populate DecodeReport from accumulated state
Result    DecodeResult { policy, report }
```

### Errors per stage

| Stage | Errors |
|---|---|
| Encode 1 | `PolicyParse`, `PolicyScopeViolation`, `Miniscript` |
| Encode 2-5 | (none expected; bytecode generation is total over canonical input) |
| Decode 1 | `InvalidHrp`, `MixedCase`, `InvalidStringLength` |
| Decode 2 | `BchUncorrectable` |
| Decode 3 | `UnsupportedVersion`, `UnsupportedCardType` |
| Decode 4 | `WalletIdMismatch`, `TotalChunksMismatch`, `ChunkIndexOutOfRange`, `DuplicateChunkIndex`, `CrossChunkHashMismatch` |
| Decode 5 | `InvalidBytecode { offset, kind: BytecodeErrorKind }` |

### Round-trip invariants

```rust
// Structural equality (NOT string equality — miniscript Display normalizes)
let p2 = decode(&encode(&p, &opts)?, &DecodeOptions::default())?.policy;
assert_structural_eq(&p, &p2);

// Bytecode round-trip
let p2 = WalletPolicy::from_bytecode(&p.to_bytecode()?)?;
assert_structural_eq(&p, &p2);

// Idempotency under fixed seed
let opts = EncodeOptions { wallet_id_seed: Some(seed), ..Default::default() };
assert_eq!(encode(&p, &opts)?, encode(&p, &opts)?);

// Encode-decode-encode (guards against silent loss-of-information bugs)
let s1 = encode(&p, &opts)?;
let s2 = encode(&decode(&s1, &DecodeOptions::default())?.policy, &opts)?;
assert_eq!(s1, s2);
```

### Wallet ID semantics

Two distinct artifacts:

| Artifact | Width | Source | Affected by `wallet_id_seed`? |
|---|---|---|---|
| Tier-3 Wallet ID (16 bytes, 12 BIP-39 words) | 128 bits | First 16 bytes of SHA-256(canonical_bytecode) | **Never** — always content-derived |
| Chunk-header `wallet_id` (`ChunkWalletId`) | 20 bits | Default: first 20 bits of the same SHA-256. With seed: replaced by `WalletIdSeed` cast to a 20-bit value. | **Yes** — seed overrides this only |

When `wallet_id_seed = None` (the normal case), the chunk-header `wallet_id` is a strict truncation of the Tier-3 Wallet ID. A user holding only the Tier-3 12-word mnemonic can recompute the expected chunk-header wallet_id and verify chunk consistency.

When `wallet_id_seed = Some(_)`, the chunk-header `wallet_id` diverges from the Tier-3 Wallet ID. This mode is provided for deterministic test-vector generation; it is exposed via the CLI `--seed` flag for reproducible debug encodes but is not the default encoding mode for production use. The Tier-3 Wallet ID continues to be content-derived in either mode, so the truncation relationship breaks only when a user explicitly opts in.

### Determinism

- `EncodeOptions::wallet_id_seed = None` → both Wallet IDs derived from SHA-256(canonical_bytecode); fully deterministic
- `EncodeOptions::wallet_id_seed = Some(_)` → chunk-header wallet_id replaced by seed; Tier-3 Wallet ID still content-derived
- Path emission: sorted-by-first-appearance during prefix-order AST walk, using `IndexMap` (no `HashMap` nondeterminism)
- Hardened path components: canonical form uses `'` (apostrophe), not `h` (per BIP 388 convention)
- JSON serialization: `serde_json::to_string_pretty` with 2-space indent; struct-field order via `serde(rename = "...")` declarations; trailing newline appended to every output file. `gen_vectors --verify` deserializes both files into typed structs and compares field-by-field, so JSON-formatting drift in `serde_json` releases doesn't break verification.

---

## 5. Test strategy

### Test categories

**Unit tests** (inline `#[cfg(test)] mod tests` per module):
- `bytecode/tag.rs`: tag round-trip via `from(u8)` / `as u8`; reject 0x36–0xFF
- `bytecode/encode.rs`: each operator individually
- `bytecode/decode.rs`: each operator individually; trailing bytes rejected
- `encoding.rs`: bech32 8↔5 pack/unpack; hand-computed BCH vectors; HRP/separator/case checks
- `chunking.rs`: `ChunkHeader` round-trip; cross-chunk hash determinism
- `policy.rs`: canonicalization; `FromStr` failure modes
- `wallet_id.rs`: known-vector test; expected hex computed once from impl, committed as `const`, treated as spec-by-reference

**Integration tests** (`tests/*.rs`):
- `tests/common/mod.rs` — shared helpers: `round_trip_assert`, `corrupt_n`, `load_vector`
- `tests/corpus.rs` — round-trip every entry in v0.1 corpus (**9 entries**: C1–C5, E10, E12, **E13** (HTLC with sha256 — exercises the inline 32-byte hash literal path), E14); plus encode-decode-encode idempotency; plus HRP-lowercase property in the round-trip loop; plus ≥1 Coldcard-exported BIP 388 wallet policy string. (CORPUS.md's C6 is a v0.2 placeholder; not in v0.1.)
- `tests/upstream_shapes.rs` — 9 descriptor-codec policy shapes rewritten in `@i` form (encoder coverage)
- `tests/chunking.rs` — 4 named hash tests:
  - `chunk_hash_mismatch_rejects`
  - `chunk_hash_correct_reassembly`
  - `chunk_out_of_order_reassembly`
  - `natural_long_code_boundary` (56-byte payload, single Long string, no hash)
- `tests/ecc.rs` — deterministic constructed BCH stress (named cases per code path) + fixed-seed `many_substitutions_always_rejected` loop (N=1000, `SmallRng::seed_from_u64(0xDEADBEEF)`)
- `tests/conformance.rs` — `macro_rules!`-expanded `rejects_*` cases (18+):
  - `rejects_mixed_case`, `rejects_invalid_hrp`, `rejects_length_94`, `rejects_length_95`,
  - `rejects_unknown_version`, `rejects_unknown_card_type`, `rejects_unknown_tag`,
  - `rejects_trailing_bytes`, `rejects_zero_keys`, `rejects_more_than_20_keys`,
  - `rejects_pre_expanded_multipath`, `rejects_foreign_xpubs_in_v0`, `rejects_inline_keys_in_v0_bytecode`,
  - `rejects_chunk_index_out_of_range`, `rejects_duplicate_chunk_index`,
  - `rejects_wallet_id_mismatch_across_chunks`, `rejects_total_chunks_mismatch_across_chunks`,
  - `rejects_cross_chunk_hash_mismatch`
- `tests/vectors_schema.rs` — deserialize committed `tests/vectors/v0.1.json` against typed struct; assert structural invariants. **Lives in P8** (depends on the JSON file, which P8 produces); not P6.
- `tests/error_coverage.rs` — `strum::EnumIter` over every `Error` variant; collects errors from conformance suite; asserts every variant produced by ≥1 negative test

### Test vector generation

`src/bin/gen_vectors.rs` — single binary with two modes:
- `--output <path>` — generates `tests/vectors/v0.1.json` from corpus (positive vectors) + conformance suite (negative vectors)
- `--verify <path>` — deserialize committed → regenerate in-memory → deserialize regenerated → typed-struct compare; exit non-zero on diff

CI runs `--verify` mode (no working-tree mutation).

### JSON schema

```json
{
  "version": "0.1",
  "vectors": [
    {
      "id": "C1",
      "description": "Single-key wsh wallet",
      "input_policy": "wsh(pk(@0/**))",
      "canonical_bytecode_hex": "...",
      "canonical_bytecode_len": 7,
      "encoding_code": "regular",
      "chunks": [
        {"raw": "wdm1q0...", "chunk_index": 0, "total_chunks": 1}
      ],
      "wallet_id_hex": "...",
      "wallet_id_words": ["abandon", "ability", ...]
    }
  ],
  "negative_vectors": [
    {
      "id": "N01",
      "description": "mixed case",
      "input": "wDm1qq...",
      "expected_error": "MixedCase",
      "note": "lowercase HRP and uppercase HRP both accepted; mixing is rejected"
    }
  ]
}
```

### Coverage targets

- **Hard requirement:** every `Error` variant produced by ≥1 negative test (enforced by `tests/error_coverage.rs` + `strum::EnumIter`)
- **Hard requirement:** every BCH correction code path exercised by ≥1 named test
- Public API: 100% of public functions called by ≥1 test
- Line coverage: ≥85% (informational, via `cargo-llvm-cov`; not a CI gate)
- Branch coverage: not measured in v0.1 (requires nightly)

### CI

- Linux (full): `fmt --check`, `clippy -D warnings`, `test`, `gen_vectors --verify`, `doc --no-deps`, `llvm-cov`
- Windows (sanity, every PR): `cargo build` + `cargo test --lib`
- macOS (sanity, every PR): `cargo build` + `cargo test --lib`
- Stub `cargo check`-only CI in P0; upgraded to full at P5

---

## 6. Build order — 11 phases, 9.5-day wall-clock minimum

| Phase | Duration | Parallel? | Description |
|---|---|---|---|
| P0 | 0.5 d | — | Workspace, Cargo.toml (`rust-version=1.85`), module skeletons, **Error enum skeleton committed**, **SHA-256 chosen for cross-chunk hash**, CI yml stub (`cargo check` only). Verify miniscript 12 + bitcoin 0.32 actually compile on MSRV 1.85; downgrade pin if needed before continuing. **Update POLICY_BACKUP.md** to mark the SHA-256 hash decision as RESOLVED (was DECIDE). |
| P1 | 1.5 d | ‖ P2 | Encoding layer: BCH polynomials (regular + long; coefficients quoted inline as `const` arrays); bech32 8↔5; HRP/separator/length/case validation. Cross-check first 3 BCH vectors against BIP 93 reference Python impl |
| P2 | **3.0 d** | ‖ P1 | Bytecode foundation (revised from 2.0d after holistic review): vendor descriptor-codec's tag table (0x00–0x31), LEB128, AST walker; reimplement key-terminal arms over `WdmKey` (replicate the ~40 structural arms, change only the 2 key arms). 5057 LOC of unfamiliar code is the dominant risk; budget includes reading time |
| P3 | 1.0 d | — | WDM extensions: `WdmKey::Placeholder` only; tags 0x32, 0x33; bytecode header byte; path dictionary const; sorted-by-first-appearance path emission via `IndexMap` |
| P4 | 1.0 d | — | Chunking: `ChunkHeader`; cross-chunk SHA-256[0..4]; `WalletId` 16-byte + `WalletId::to_words()`; `ChunkWalletId` 20-bit newtype; `WalletId::truncate() -> ChunkWalletId`; chunking decision + reassembly + verification |
| P5 | 0.5 d | — | Top-level API: `WalletPolicy` adapter; `FromStr`; encode/decode wiring; populate full Error variants; `#[non_exhaustive]` markers; CI upgraded from `cargo check` to `cargo test` |
| P5.5 | 0.5 d | — | **Spec reconciliation buffer (uncapped if needed).** Sweep `// TODO: spec` markers from P2–P5; update BIP and `design/POLICY_BACKUP.md`; commit BIP edits before any test vectors |
| P6 | 1.5 d | — | Test corpus (excluding vectors_schema.rs, which moves to P8): `tests/common/mod.rs`; `corpus.rs` (**9 entries** including E13 + ≥1 Coldcard-exported policy); `upstream_shapes.rs`; `chunking.rs` (4 named); `ecc.rs`; `conformance.rs` (18+ macros); `error_coverage.rs` |
| P7 | 1.0 d | ‖ P8 ‖ P9 | CLI binary (`wdm`) with all 6 subcommands; clap derive |
| P8 | 0.5 d | ‖ P7 ‖ P9 | `src/vectors.rs` schema; `src/bin/gen_vectors.rs` (write + verify); generate and commit `tests/vectors/v0.1.json`; `tests/vectors_schema.rs` (now in this phase since it depends on the JSON file); update BIP test vectors section to reference the JSON via permalink + content hash (frozen at v0.1.0) |
| P9 | 0.5 d | ‖ P7 ‖ P8 | rustdoc on every public item (under `#![deny(missing_docs)]`); README quickstart; status updates in BIP and root README |
| P10 | 0.5 d | — | Pre-release review; coverage check; CI green-bar across Linux/Windows/macOS; tag `wdm-codec-v0.1.0`; advance project status |

**Critical path:** P0 → P2 → P3 → P4 → P5 → P5.5 → P6 → P7 → P10 = **9.5 days** (P2 increased from 2.0 to 3.0).

**Slack:** ~0.5 day relative to a 10-day budget. The 1.5-week (~7.5 day) target requires cutting scope; recommend pushing v0.1 to a 2-week target. If overruns occur:
1. Cut P9 (doc polish) first — minimal rustdoc, defer README polish
2. Then cut P7 (CLI) to encode/decode-only (no `inspect`/`bytecode`/`verify` subcommands)
3. **Never cut P6, P5.5, or P8** — the test vectors are the deliverable

---

## 7. Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| P2 fork is more invasive than 3.0d | Low (was Medium at 2.0d) | Targeted extraction; replicate ~40 structural arms verbatim; only key arms diverge |
| BCH polynomial bug undetected by tests (P1) | Low | Cross-check first 3 vectors against BIP 93 reference Python impl during P1; `const` arrays in source |
| `miniscript 12` `KeyExpression` API surprises | Medium | Pin miniscript exactly; adapter trait insulates from upstream changes |
| Spec ambiguities surface during impl | High | **P5.5 is the explicit reconciliation slot (uncapped if needed)** |
| `WalletPolicy` round-trip is structural not string | Medium (expected) | Documented; tests use `assert_structural_eq` |
| `HashMap` nondeterminism breaks idempotency | Low | `IndexMap` from start in P3; no `HashMap` in canonical paths |
| Coverage gaps in Error variants | Low | `tests/error_coverage.rs` `strum::EnumIter` hard CI gate |
| 2-week budget overrun (revised from 1.5-week target) | Medium | Cut P9 doc polish first; cut P7 CLI subcommands second; never cut P6 / P5.5 / P8 |
| Coldcard exemplar policy strings unavailable | Low | Coldcard docs are public; Bitcoin Stack Exchange has examples |
| **MSRV drift from upstream** (miniscript 12 / bitcoin 0.32 may not actually compile on Rust 1.85) | Medium | P0 includes verification step; if MSRV doesn't hold, downgrade to 1.83 or lower per upstream's actual minimum |
| **`clap` MSRV creep into library MSRV** | Low | `clap` is a non-dev dependency only on the `wdm` and `gen_vectors` binary targets; library users don't pull it. Use `optional = true` + per-target enable if needed. |
| **Test vector format diverges from BIP reader expectations** | Medium | JSON schema explicit in §3; BIP "Test Vectors" section will quote schema verbatim during P8 |
| **JSON formatter drift (serde_json release changes pretty-print)** | Low | `--verify` mode does typed-struct compare, not raw byte compare; immune to whitespace drift |
| **POLICY_BACKUP.md DECIDE markers go stale** during impl | Medium | P0 propagates SHA-256 decision; P5.5 sweeps remaining DECIDEs |

---

## 8. Definition of done for v0.1

Every item must be true before tagging `wdm-codec-v0.1.0`:

- All 9 v0.1 corpus entries (C1–C5, E10, E12, E13, E14) round-trip (encode → decode → structural-equal)
- All 9 upstream-derived shapes round-trip
- ≥1 Coldcard-exported BIP 388 wallet policy string round-trips losslessly
- All 4 named chunking tests pass (`chunk_hash_mismatch_rejects`, `chunk_hash_correct_reassembly`, `chunk_out_of_order_reassembly`, `natural_long_code_boundary`)
- All 18+ conformance rejection tests in `tests/conformance.rs` pass with specific `Error` variants
- BCH stress tests in `tests/ecc.rs` pass (deterministic constructed cases + `many_substitutions_always_rejected` fixed-seed loop)
- `tests/upstream_shapes.rs` passes (encoder coverage on the 9 descriptor-codec-derived shapes)
- `tests/vectors_schema.rs` deserializes the committed JSON into typed structs without errors
- **`tests/error_coverage.rs` (`strum::EnumIter` exhaustiveness gate) passes** — every `Error` variant produced by ≥1 negative test
- `gen_vectors --verify tests/vectors/v0.1.json` succeeds in CI
- All public items have rustdoc (build clean under `#![deny(missing_docs)]`)
- CI green: Linux full + Windows sanity (`cargo build` + `cargo test --lib`) + macOS sanity (`cargo build` + `cargo test --lib`)
- Line coverage ≥85% (informational, via `cargo-llvm-cov`)
- Spec reconciliation commit (P5.5) made before any test vectors generated
- `design/POLICY_BACKUP.md` DECIDE markers reconciled (SHA-256 hash, HRP "wdm", `'` for hardened components, etc.) during P0 + P5.5
- BIP `bip-wallet-descriptor-mnemonic.mediawiki` "Test Vectors" section updated to reference `crates/wdm-codec/tests/vectors/v0.1.json` via:
  - GitHub permalink to the v0.1.0-tagged commit (durable across repo moves)
  - SHA-256 content hash of the JSON file (catches silent edits)
- `tests/vectors/v0.1.json` is **frozen at v0.1.0** — never regenerated for v0.1.x patch releases; v0.2 spawns a new `v0.2.json`
- Project status advanced from "Pre-Draft, AI only" to "Pre-Draft, AI + ref impl, awaiting human review"
- Commit tagged `wdm-codec-v0.1.0`

---

## 9. v0.2 / v0.3 staging (out-of-scope but flagged)

**v0.2:**
- Taproot (`tr()` + tap-miniscript): bytecode tag 0x06 + 0x08 (TapTree) + 0x1A (multi_a)
- Master fingerprints block (tag 0x35): `EncodeOptions::include_fingerprints: bool`
- Guided recovery: erasure decoding public API + structure-aware candidate filtering + confidence calibration
- `proptest` property-based testing
- `cargo-fuzz` adversarial harness
- Full macOS + Windows CI matrix
- Performance benchmarks
- v0.2-specific corpus additions

**v0.3:**
- Foreign xpubs (Tier 2 Xpub Cards)
- Per-`@i` paths (heterogeneous derivation paths in one policy)
- MuSig2 (`musig()` placeholder, contingent on Coldcard support)
- Public API stability commitment
- crates.io publish under name TBD

---

## 10. References

- BIP draft: `bip/bip-wallet-descriptor-mnemonic.mediawiki`
- Format design rationale: `design/POLICY_BACKUP.md`
- Prior-art survey: `design/PRIOR_ART.md`
- Reference miniscript corpus: `design/CORPUS.md`
- BIP 93 (codex32): https://bips.dev/93/
- BIP 388 (wallet policies): https://bips.dev/388/
- BIP 380 (output descriptors): https://bips.dev/380/
- `descriptor-codec` (joshdoman, CC0): https://github.com/joshdoman/descriptor-codec
- `rust-miniscript`: https://github.com/rust-bitcoin/rust-miniscript
- `rust-bitcoin`: https://github.com/rust-bitcoin/rust-bitcoin
- `rust-bech32`: https://github.com/rust-bitcoin/rust-bech32
