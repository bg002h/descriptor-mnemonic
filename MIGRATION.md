# Migration guide

Migration steps for upgrading between major releases of `md-codec` (formerly `wdm-codec`).

## v0.14.x → v0.15.0

v0.15.0 reintroduces the `md` CLI binary stripped in v0.12.0. **Library API is
unchanged** — no source changes required for downstream library consumers.

### What's new

- New `md` binary: `cargo install md-codec` produces it.
- Default features `cli` and `json` are on. Library-only consumers:

  ```toml
  md-codec = { version = "0.15", default-features = false }
  ```

- New opt-in `cli-compiler` feature pulls `miniscript/compiler` for the
  `compile` subcommand and `encode --from-policy`.

### What didn't change

- Wire format (v0.13/v0.14 unchanged).
- Library `Error` enum (CLI-specific errors live in the binary's own
  `CliError`).
- Public exports of `md_codec::*`.

### What's not coming back

- `--seed` flag for chunk-set-id override (v0.11 had it). The
  `derive_chunk_set_id` function is fully deterministic from the payload;
  if you need a known id for a test corpus, use `md vectors`.
- Separate `gen_vectors` binary — folded into `md vectors`.
- Testnet/regtest xpubs for `--key` — mainnet only in v0.15.0.

## v0.9.x → v0.10.0

v0.10.0 is a **wire-format-breaking release** at the BIP 388 wallet-policy
template level: header bit 3 was reserved-must-be-zero in v0.x ≤ 0.9; v0.10
reclaims it as the `OriginPaths` flag. v0.x ≤ 0.9 SharedPath-only encodings
are byte-identical under v0.10 (bit 3 stays `0`); new v0.10 OriginPaths-using
encodings (header byte `0x08` or `0x0C`) need v0.10+ decoders — pre-v0.10
decoders cleanly reject via `Error::ReservedBitsSet`.

### Why a wire-format break?

v0.x ≤ 0.9 silently flattened policies with divergent per-`@N` origin paths
to a single shared path, losing information. `decode(encode(p))` could differ
from `p` for any policy where cosigners derived xpubs from different paths.
v0.10 fixes this with the new `Tag::OriginPaths = 0x36` block. See
[`CHANGELOG.md`](CHANGELOG.md#0100--2026-04-29) for the full framing.

### What renamed/added/changed

Public-API break surface is small — two function signatures and one
additional error variant family:

| Before (v0.9.x) | After (v0.10.0) | Notes |
| --- | --- | --- |
| `BytecodeHeader::new_v0(fingerprints: bool)` | `BytecodeHeader::new_v0(fingerprints: bool, origin_paths: bool)` | Second arg defaults to `false` for typical pre-v0.10 use cases (no per-`@N` paths). |
| `encode_path(&DerivationPath) -> Vec<u8>` | `encode_path(&DerivationPath) -> Result<Vec<u8>, Error>` | Surfaces `Error::PathComponentCountExceeded` when path > 10 components. Symmetric change on `encode_declaration`. |
| (no field) | `WalletPolicy::decoded_origin_paths: Option<Vec<DerivationPath>>` | Round-trip stability when `from_bytecode` decoded `Tag::OriginPaths`. Additive. |
| (no field) | `EncodeOptions::origin_paths: Option<Vec<DerivationPath>>` | Tier 0 override for deterministic test-vector generation. Additive. |
| (no method) | `EncodeOptions::with_origin_paths(...)` | Builder for `origin_paths`. Additive. |
| (no method) | `PolicyId::fingerprint() -> [u8; 4]` | Short-identifier API; top 32 bits. 8-char hex display alternative to the 12-word phrase. Additive. |

New error variants (`Error` is `#[non_exhaustive]`; additive):

- `Error::OriginPathsCountMismatch { expected: usize, got: usize }` — policy-layer semantic error.
- `Error::PathComponentCountExceeded { got: usize, max: usize }` — applies to both `Tag::SharedPath` and `Tag::OriginPaths` explicit-form path-decls when component count > 10.
- `BytecodeErrorKind::OriginPathsCountTooLarge { count: u8, max: u8 }` — bytecode-layer structural error.

### Mechanical sed for consumer code

```bash
# Add false default for the new origin_paths arg in BytecodeHeader::new_v0:
find . -type f -name '*.rs' -exec sed -i \
    -e 's/BytecodeHeader::new_v0(\(true\|false\))/BytecodeHeader::new_v0(\1, false)/g' \
    {} +
```

The literal-bool form covers most call sites (test code typically passes
`true` or `false` directly). For variable-bool sites, inspect each:

```bash
# Find variable-bool BytecodeHeader::new_v0 call sites for hand-review:
grep -rn 'BytecodeHeader::new_v0(' --include='*.rs'
```

### Hand-rename items — three breaks requiring per-call-site review

1. **`BytecodeHeader::new_v0(bool)` → `new_v0(bool, bool)`** — signature
   gains an `origin_paths: bool` argument (parallel to the existing
   `fingerprints: bool`). The sed above covers literal-bool call sites; for
   variable-bool sites, inspect each and pass `false` for the
   `origin_paths` argument unless the caller is explicitly emitting an
   OriginPaths-using encoding.

2. **`encode_path(&DerivationPath) -> Vec<u8>` becomes
   `encode_path(&DerivationPath) -> Result<Vec<u8>, Error>`.** The
   function may now surface `Error::PathComponentCountExceeded` when the
   path declares more than `MAX_PATH_COMPONENTS = 10` components.
   Consumer call-site updates:

   - Append `?` to propagate the error if the caller is itself fallible.
   - `.expect("validated upstream")` where the caller has already
     pre-validated component count (e.g., test code with literal short
     paths). Document the upstream invariant in the `expect` message.

   `encode_declaration(...)` changes symmetrically.

3. **`md encode --fingerprint <@INDEX=HEX>` →
   `md encode --master-key-fingerprint <@INDEX=HEX>`.** **CLI break.**
   The flag still embeds BIP 32 master-key fingerprints into the
   bytecode's fingerprints block; the more explicit name disambiguates
   it from the new `--policy-id-fingerprint` output flag (see "Added"
   below). No deprecation alias was added — pre-v1.0 break freedom.
   Consumer scripts:

   ```bash
   # Sed for shell scripts that pass --fingerprint to `md encode`:
   find . -type f \( -name '*.sh' -o -name '*.bash' -o -name 'Makefile' \) \
     -exec sed -i 's/--fingerprint /--master-key-fingerprint /g' {} +
   ```

   Validate by running the script — the CLI rejects the old flag with
   a clap "unexpected argument" error.

### Added

- **`md encode --policy-id-fingerprint`** — additive output flag.
  When set, prints the freshly-computed PolicyId in its 4-byte /
  8-hex-char short form (`0x{:08x}`, via the new
  `PolicyId::fingerprint()` API) on a second line after the existing
  12-word phrase. The 12-word phrase is always printed; this flag is
  strictly additive. Use cases: CLI scripts, log lines, and minimal-
  cost engraving anchors for users who don't want the full phrase.
  Not on by default — opt-in to keep the default `md encode` output
  minimal.

### Wire format

- Header bit 3 reclaimed as the OriginPaths flag (`0x08`).
- `RESERVED_MASK` narrows from `0x0B` (bits 3, 1, 0) to `0x03`
  (bits 1, 0).
- Valid v0.10 header bytes: `0x00`, `0x04`, `0x08`, `0x0C`.
- v0.x ≤ 0.9 SharedPath-only encodings are byte-identical under v0.10
  (regenerate with `md-codec 0.10` and you get identical bytes; the
  `GENERATOR_FAMILY` family-token roll alone doesn't churn these
  vectors).
- New OriginPaths encodings (header bit 3 set) need v0.10+ decoders.
  Pre-v0.10 decoders reject with `Error::ReservedBitsSet { byte: 0x08
  | 0x0C, mask: 0x0B }` — intended forward-compat.
- Test-vector corpora (`v0.1.json` and `v0.2.json`) regenerate;
  family-token rolls `"md-codec 0.9"` → `"md-codec 0.10"`. New positive
  vector `o1_sortedmulti_2of3_divergent_paths` (and optional `o2`/`o3`)
  exercises the OriginPaths block. New negative vectors cover each new
  error variant.

### Test rewrite — multi-byte LEB128 in the child-index dimension

The pre-existing `decode_path_round_trip_multi_byte_component_count`
test in `crates/md-codec/src/bytecode/path.rs` exercised a 128-component
path to validate multi-byte LEB128 round-trip in the count field. Under
v0.10's `MAX_PATH_COMPONENTS = 10` cap, this test no longer compiles
(the encoder rejects the 128-component path before the decoder ever
sees it).

The defensive value (multi-byte LEB128 round-trip exercise) survives by
shifting the test from the **component-count dimension** to the
**child-index dimension** (e.g., `m/16384`, where `16384 = 2 × 8192`
requires a 2-byte LEB128 in the per-component bytes). Consumer code
that copied this test pattern should rewrite analogously.

### What consumer code does NOT need to change

- BIP 388 wallet policies whose placeholders all share a single origin
  path (the typical pre-v0.10 case): zero source changes beyond the
  `BytecodeHeader::new_v0` signature change. The encoder auto-detects
  path agreement and emits `Tag::SharedPath` as before; bytes are
  byte-identical.
- Decode of v0.x ≤ 0.9 strings: zero source changes; v0.10 decoders
  accept all v0.x ≤ 0.9 SharedPath-only encodings without behavior
  change.
- Chunk-set identifier surface, `PolicyId` / `WalletInstanceId` types,
  fingerprints block: unchanged.

## v0.8.x → v0.9.0

v0.9.0 is a **chunk-header-naming-cleanup release**. v0.8.0's mechanical
`WalletId → PolicyId` sweep accidentally renamed the chunk-header 20-bit
field family to a "PolicyId" name when it actually identifies a chunk-set
assembly (not a Policy ID, not a Wallet Instance ID). v0.9.0 corrects the
chunk-header sub-domain to `ChunkSetId` / `ChunkSetIdMismatch` /
`ReservedChunkSetIdBitsSet` etc. v0.8's Tier-3 `PolicyId` and derived
`WalletInstanceId` are stable and unchanged.

### Why a rename, *again*?

The same rationale that justified the v0.8 rename ("the value identifies
a template, not a wallet instance") applies one level deeper to the
chunk-header field: it identifies neither — it identifies the chunk-set
assembly. We expect this to be the last identifier rename in this family.

### What renamed

Chunk-header sub-domain only (~150 references in md-codec; small surface
in consumer code):

| Before (v0.8.x) | After (v0.9.0) |
| --- | --- |
| `md_codec::ChunkPolicyId` | `md_codec::ChunkSetId` |
| `md_codec::PolicyIdSeed` | `md_codec::ChunkSetIdSeed` |
| `EncodeOptions::policy_id_seed` field | `EncodeOptions::chunk_set_id_seed` |
| `EncodeOptions::with_policy_id_seed(seed)` | `EncodeOptions::with_chunk_set_id_seed(seed)` |
| `Error::PolicyIdMismatch { expected, got }` | `Error::ChunkSetIdMismatch { expected, got }` |
| `Error::ReservedPolicyIdBitsSet` | `Error::ReservedChunkSetIdBitsSet` |
| `ChunkHeader::Chunked.policy_id` field | `ChunkHeader::Chunked.chunk_set_id` |
| `Chunk.policy_id` field | `Chunk.chunk_set_id` |
| `Verifications.policy_id_consistent` field | `Verifications.chunk_set_id_consistent` |
| CLI JSON output: `policy_id_consistent` | `chunk_set_id_consistent` |

`PolicyId`, `PolicyIdWords`, `WalletInstanceId`, `compute_policy_id`,
`compute_policy_id_for_policy`, `compute_wallet_instance_id`,
`MdBackup::policy_id()` are intentionally unchanged.

### Mechanical sed for consumer code

```bash
# Type names (CamelCase)
find . -type f -name '*.rs' -exec sed -i \
  -e 's/\bChunkPolicyId\b/ChunkSetId/g' \
  -e 's/\bPolicyIdSeed\b/ChunkSetIdSeed/g' \
  -e 's/\bPolicyIdMismatch\b/ChunkSetIdMismatch/g' \
  -e 's/\bReservedPolicyIdBitsSet\b/ReservedChunkSetIdBitsSet/g' \
  {} +

# Snake-case bare names
find . -type f -name '*.rs' -exec sed -i \
  -e 's/\bchunk_policy_id\b/chunk_set_id/g' \
  -e 's/\bpolicy_id_seed\b/chunk_set_id_seed/g' \
  -e 's/\bpolicy_id_consistent\b/chunk_set_id_consistent/g' \
  {} +

# Snake-case test-name compound forms (P1 sed lesson — \b doesn't
# match between word-chars; explicit suffix forms required)
find . -type f -name '*.rs' -exec sed -i \
  -e 's/chunk_policy_id\(_[a-z]\)/chunk_set_id\1/g' \
  -e 's/policy_id_seed\(_[a-z]\)/chunk_set_id_seed\1/g' \
  {} +
```

**Hand-rename only:** `ChunkHeader::Chunked.policy_id` and
`Chunk.policy_id` *struct field* accesses — sed can't disambiguate from
`compute_policy_id` or `MdBackup::policy_id()` (the Tier-3 getter, which
intentionally stays). Consumers grepping `\.policy_id\b` should review
hits manually.

### Wire format

Unchanged for the rename portion. The chunk-header is bit-identical
across the v0.8→v0.9 boundary; only names move. The `expected_error_variant`
strings in the test-vector JSON corpora rename in lockstep with the code.

### Wire-additive: testnet `0x16`

v0.9.0 also adds `0x16 = m/48'/1'/0'/1'` to the path dictionary (BIP 48
testnet P2SH-P2WSH, mirror of mainnet `0x06`). Encoders that previously
encoded the testnet path via the explicit-path fallback (`0xFE` form) will
now select the single-byte dictionary form. Old encodings remain valid;
new encodings using `0x16` require v0.9+ decoders.

## v0.7.x → v0.8.0

v0.8.0 is a **naming-cleanup release**. The 16-byte template-only hash
that md-codec previously called "wallet ID" was renamed to "Policy ID"
to reflect what it actually identifies, and a new derived
`WalletInstanceId` quantity is introduced for per-wallet
disambiguation. **Wire format byte-identical to v0.7.x**; existing MD
chunks decode unchanged.

### Why the rename

The 16-byte hash `SHA-256(canonical_bytecode)[0..16]` covers the BIP
388 wallet-policy *template* only — no concrete cosigner xpubs — so
two distinct wallets that share an identical policy template (same
multisig shape and shared path, *different* cosigner sets) collide on
this value. The "wallet ID" name treated a one-to-many relationship as
one-to-one. The new "Policy ID" name makes the template-level scope
explicit, and a new derived `WalletInstanceId` (computed at recovery
time from policy + assembled xpubs) provides per-wallet disambiguation
for tools that need it.

### What changed (breaking for code consumers)

Every `WalletId*` and `wallet_id*` identifier renamed to
`PolicyId*` / `policy_id*`. Concretely:

| Old | New |
|---|---|
| `md_codec::WalletId` | `md_codec::PolicyId` |
| `md_codec::WalletIdSeed` | `md_codec::PolicyIdSeed` |
| `md_codec::WalletIdWords` | `md_codec::PolicyIdWords` |
| `md_codec::ChunkWalletId` | `md_codec::ChunkPolicyId` |
| `md_codec::wallet_id::compute_wallet_id` | `md_codec::policy_id::compute_policy_id` |
| `md_codec::wallet_id::compute_wallet_id_for_policy` | `md_codec::policy_id::compute_policy_id_for_policy` |
| Module `md_codec::wallet_id` | Module `md_codec::policy_id` |
| `Error::WalletIdMismatch` | `Error::PolicyIdMismatch` |
| `Error::ReservedWalletIdBitsSet` | `Error::ReservedPolicyIdBitsSet` |
| `Verifications::wallet_id_consistent` | `Verifications::policy_id_consistent` |
| `EncodeOptions::wallet_id_seed` | `EncodeOptions::policy_id_seed` |
| `EncodeOptions::with_wallet_id_seed(seed)` | `EncodeOptions::with_policy_id_seed(seed)` |
| Vector JSON field `wallet_id_words` | `policy_id_words` |
| Vector JSON field `wallet_id_*` (any) | `policy_id_*` |
| CLI output: `Wallet ID:` | `Policy ID:` |
| CLI JSON output: `wallet_id*` | `policy_id*` |

### What's new

`md_codec::WalletInstanceId` — the per-wallet identifier:

```rust
pub fn compute_wallet_instance_id(
    canonical_bytecode: &[u8],
    xpubs: &[bitcoin::bip32::Xpub],
) -> WalletInstanceId
```

Computes `SHA-256(canonical_bytecode || encode(xpubs[0]) || encode(xpubs[1]) || ...)[0..16]`,
where `encode(xpub)` is the canonical 78-byte BIP 32 serialization.
Xpubs are concatenated in placeholder-index order (`@0`, then `@1`,
..., `@(N-1)`); ordering is significant.

Wallet Instance IDs are **not** carried by any physical card or wire
structure — they are recovery-time derivations. Tools that have the
policy card (template) plus the cosigner xpubs (whether from a digital
backup, the wallet itself, or engraved `mk1` key cards in foreign-xpub
multisig) compute it on demand.

### What didn't change

- **Wire format byte-identical to v0.7.x.** Existing MD chunks decode
  unchanged.
- The 16-byte value, the 4-byte `ChunkPolicyId` stub field width, and
  the 12-word BIP 39 mnemonic anchor encoding — all unchanged.
- All public API behaviour: only names changed. The same bytes
  produce the same values.
- MSRV unchanged: 1.85.

### How to upgrade

The rename is mechanical:

```bash
# In your project's source:
sed -i '
  s/WalletIdMismatch/PolicyIdMismatch/g
  s/WalletIdBitsSet/PolicyIdBitsSet/g
  s/WalletIdSeed/PolicyIdSeed/g
  s/WalletIdWords/PolicyIdWords/g
  s/ChunkWalletId/ChunkPolicyId/g
  s/\bWalletId\b/PolicyId/g
  s/\bcompute_wallet_id_for_policy\b/compute_policy_id_for_policy/g
  s/\bcompute_wallet_id\b/compute_policy_id/g
  s/wallet_id\([_a-zA-Z0-9]\)/policy_id\1/g
  s/\bwallet_id\b/policy_id/g
  s/Wallet ID/Policy ID/g
  s/wallet ID/policy ID/g
' your-files-here
```

`cargo update -p md-codec --precise 0.8.0` to pull the new crate.

If you parse MD's CLI JSON output, update field-name expectations
(`wallet_id*` → `policy_id*`).

If you parse `tests/vectors/v0.2.json` for cross-implementation
testing, regenerate it from the v0.8 reference impl; field names
changed in lockstep.

## v0.6.x → v0.7.0

v0.7.0 is **purely additive**. No breaking changes since v0.6.0.

### What's new

1. **NEW workspace crate `md-signer-compat`.** Add as a dependency if
   you want named-subset validation (Coldcard, Ledger):
   ```toml
   [dependencies]
   md-signer-compat = "0.1"
   ```
   API:
   ```rust
   use md_signer_compat::{COLDCARD_TAP, LEDGER_TAP, validate, validate_tap_tree};
   // single-leaf
   validate(&COLDCARD_TAP, &leaf_ms, Some(0))?;
   // multi-leaf with DFS-pre-order leaf_index threading
   validate_tap_tree(&COLDCARD_TAP, &tap_tree)?;
   ```

2. **NEW pub function `md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist`.**
   Caller-supplied operator allowlist; existing
   `validate_tap_leaf_subset` is now a back-compat shim around this
   function with the historical Coldcard list.

3. **NEW cargo features on md-codec:**
   - `compiler` (default-off): exposes `ScriptContext`,
     `policy_to_bytecode`. Pulls in rust-miniscript's `compiler`
     (heavyweight ILP-style enumeration in the Tap branch).
   - `cli-compiler`: enables `md from-policy <expr> --context
     <tap|segwitv0> [--internal-key <KEY>]` subcommand.

4. **Tap-context internal key for `policy_to_bytecode` is
   caller-supplied.** Pass `Some(key)` to use a specific internal key,
   or `None` to defer to rust-miniscript's NUMS-unspendable default
   for script-path-only spends.

### What didn't change

- **Wire format byte-identical to v0.6.x.** Existing MD chunks decode
  unchanged.
- All existing public API. No removals; no renames; back-compat shims
  preserve every v0.6 caller.
- MSRV: 1.85.

### How to upgrade

```bash
cargo update -p md-codec --precise 0.7.0
# (Optional) Add the new signer-compat crate:
cargo add md-signer-compat
# (Optional) Enable the policy-compiler feature:
# (in your Cargo.toml)
# md-codec = { version = "0.7", features = ["compiler"] }
```

### Behavioral change worth knowing

`validate_tap_leaf_subset` (and its new generic sibling) walks
operator-tree children **depth-first leaf-first** — it reports the
deepest out-of-subset operator instead of the shallowest. For a tree
like `or_b(sha256(...), pk(...))` with neither `or_b` nor `sha256` in
the allowlist, v0.6 reported `"or_b"`; v0.7 reports `"sha256"`. The
back-compat allowlist (HISTORICAL_COLDCARD_TAP_OPERATORS) is unchanged
so no test outcome flips for the historical Coldcard subset; if you've
written tests against custom allowlists that asserted the shallower
operator name, this change may surface there.

## v0.5.x → v0.6.0

v0.6.0 strips MD's signer-compatibility curation layer. MD's scope is now
encoding-only: it serializes any BIP 388 wallet policy losslessly, without
enforcing a hardware-signer-specific operator subset. **Wire format breaks
at the v0.5.x → v0.6.0 boundary** (different Tag bytes for almost every
tap-leaf operator); v0.5.x-encoded MD strings are NOT decodable under v0.6.

See [`design/MD_SCOPE_DECISION_2026-04-28.md`](./design/MD_SCOPE_DECISION_2026-04-28.md)
for the full rationale.

### Breaking changes (8)

#### 1. Tag enum reorganized — wire-format-breaking

Every tap-leaf-bearing bytecode regenerates. Consumers depending on specific
Tag byte values (e.g., parsing MD bytecode in non-Rust tooling) need to
update their Tag tables to the v0.6 layout. See the BIP draft §"Tree
operators" tag table or `crates/md-codec/src/bytecode/tag.rs` for the
authoritative mapping.

Notable byte changes:
- `Tag::TapTree`: 0x08 → 0x07 (adjacent to `Tr=0x06`)
- `Tag::Multi`: 0x19 → 0x08 (multisig family now contiguous)
- `Tag::SortedMulti`: unchanged at 0x09
- `Tag::MultiA`: 0x1A → 0x0A
- **`Tag::SortedMultiA`** (NEW in v0.6): 0x0B
- Wrappers and logical operators shift by 2 positions (0x0A→0x0C, etc.)
- `Tag::Placeholder`: 0x32 → 0x33 (byte 0x32 left intentionally unallocated
  to surface v0.5→v0.6 transcoder mistakes as `from_byte=None` rather than
  data corruption)
- `Tag::SharedPath`: 0x33 → 0x34
- Constants/top-level descriptors/keys/timelocks/hashes/Fingerprints
  byte-identical from v0.5.

#### 2. Validator default flip — encoder/decoder no longer enforce signer subset

The default-path encoder no longer calls `validate_tap_leaf_subset`; the
decoder no longer rejects out-of-subset operators by default. Callers
depending on this rejection for safety must invoke `validate_tap_leaf_subset`
explicitly:

```rust
use md_codec::bytecode::encode::validate_tap_leaf_subset;

let ms = /* parse a Miniscript<DescriptorPublicKey, Tap> */;
validate_tap_leaf_subset(&ms, Some(0))?;  // explicit-call validation
```

The function and `validate_tap_leaf_terminal` helper remain `pub fn` for
this use case. Named signer subsets (Coldcard, Ledger) are tracked
separately as a future layered crate (`md-signer-compat`); see the v0.6
FOLLOWUPS for status.

#### 3. `Error::TapLeafSubsetViolation` renamed to `Error::SubsetViolation`

Field shape unchanged: `{ operator: String, leaf_index: Option<usize> }`.
Mechanical sed `s/TapLeafSubsetViolation/SubsetViolation/g` over consumer
code is sufficient.

#### 4. `Tag::Bare` variant removed

Code matching on `Tag::Bare` will not compile under v0.6. The byte 0x07 is
reused for `Tag::TapTree`. `Descriptor::Bare` continues to be rejected at
the top-level encoder via `Error::PolicyScopeViolation` (unchanged
behaviour); only the unused enum variant is gone.

#### 5. `Reserved*` Tag variants removed (14 variants 0x24-0x31)

Code matching on `Tag::ReservedOrigin`, `Tag::ReservedNoOrigin`, etc. will
not compile under v0.6. `Tag::from_byte` returns `None` for these bytes
in v0.6. Code that defensively matched these to error out can simply rely
on the `None` arm; the compile error catches the safety concern.

#### 6. New `BytecodeErrorKind::TagInvalidContext` variant

The decoder catch-all (for "Tag valid in some context but not as a tap-leaf
inner") now produces:

```rust
Error::InvalidBytecode {
    offset,
    kind: BytecodeErrorKind::TagInvalidContext {
        tag: u8,
        context: &'static str,
    },
}
```

Callers matching exhaustively on `BytecodeErrorKind` need to handle this
new variant (or rely on `#[non_exhaustive]` to keep the catch-all arm).

#### 7. `DecodedString.data` field removed

(Already shipped in commit `d79125d` ahead of the v0.6.0 release.)

The `DecodedString.data` field has been removed. Use `DecodedString::data() -> &[u8]`
instead (a method that returns a slice into the existing `data_with_checksum`
buffer rather than a separately-allocated `Vec<u8>`).

```rust
// BEFORE (v0.5.x)
let data: &Vec<u8> = &decoded.data;
let owned: Vec<u8> = decoded.data;

// AFTER (v0.6.0+)
let data: &[u8] = decoded.data();
let owned: Vec<u8> = decoded.data().to_vec();
```

Do **not** substitute `decoded.data_with_checksum` for the removed `data`
field — `data_with_checksum` is longer (includes the trailing 13/15-symbol
BCH checksum), so passing it to `five_bit_to_bytes` (or any other
payload-processing path) silently emits extra bytes decoded from the
checksum region.

#### 8. Wire format break — clean cut at v0.6.0

v0.5.x-encoded MD strings are NOT decodable under v0.6 (different Tag
bytes for almost every tap-leaf operator). v0.6 is a clean break; pre-1.0
+ no users yet means no deprecation cycle and no v0.5-compat decoder shim.
v0.1.json and v0.2.json fully regenerate at the v0.5.x → v0.6.0 boundary;
SHA pins update once. v0.6.x patch line stable thereafter (family-stable
SHA promise applies again from v0.6.0).

### What didn't change

- HRP `md` (unchanged).
- BCH error correction polynomial constants (unchanged).
- Top-level descriptor admit set (`wsh`/`tr`/`wpkh`/`sh(wsh)`/`sh(wpkh)`).
- Wallet-policy framing (`@i/<a;b>/*` placeholders, key-info-vector).
- BIP 341 depth-128 ceiling enforcement on TapTree decode.
- `DecodedString::corrected_char_at(char_position)` behaviour.
- MSRV: 1.85.

### How to upgrade

```bash
cargo update -p md-codec --precise 0.6.0
```

For consumers who only read decoded shapes (no Tag-byte parsing, no
TapLeafSubsetViolation matches): the rename `TapLeafSubsetViolation` →
`SubsetViolation` is mechanical via sed. No other action needed.

For consumers who emit/parse MD bytecode directly: regenerate against the
v0.6 Tag table.

For consumers who depended on encoder default-validator rejection of
out-of-subset operators: switch to explicit `validate_tap_leaf_subset(...)`
calls per #2 above.

For consumers who used `decoded.data` field-style: switch to `decoded.data()`
method-style per #7 above.

## v0.4.x → v0.5.0

v0.5.0 is wire-format-additive over v0.4.x. Multi-leaf `tr(KEY, TREE)` descriptors
are now admitted. v0.4.x-produced strings (`tr(KEY)` and single-leaf `tr(KEY, leaf)`)
decode byte-identical under v0.5.0. v0.5.0-produced strings containing multi-leaf
TapTree bytecode (`Tag::TapTree = 0x08`) are rejected by v0.4.x decoders with
`PolicyScopeViolation`.

### What changed

- `tr(KEY, TREE)` admittance with non-trivial script trees (BIP 388 §"Taproot tree" subset)
- New types: `TapLeafReport`; new field: `DecodeReport.tap_leaves: Vec<TapLeafReport>`
- `Error::TapLeafSubsetViolation` gains `leaf_index: Option<usize>` field; variant marked `#[non_exhaustive]`
- `validate_tap_leaf_subset` public signature gains `leaf_index: Option<usize>` parameter
- BIP 341 depth-128 enforcement: trees exceeding 128 levels are now rejected at decode time

### What didn't change

- Wire format for v0.4.x-shaped inputs is byte-identical
- KeyOnly `tr(KEY)` bytecode unchanged
- Single-leaf `tr(KEY, leaf)` bytecode unchanged
- Per-leaf miniscript subset unchanged (`validate_tap_leaf_subset` admissibility constants and call sites preserved)
- MSRV: 1.85 (unchanged)

### How to upgrade

```bash
cargo update -p md-codec --precise 0.5.0
```

For most callers, no code changes are required. Multi-leaf encoding now succeeds;
multi-leaf decoding now produces a populated `tap_leaves` report.

### If you destructure `Error::TapLeafSubsetViolation`

Pattern-match consumers that wrote:

```rust
match err {
    Error::TapLeafSubsetViolation { operator } => { /* ... */ }
}
```

Must change to:

```rust
match err {
    Error::TapLeafSubsetViolation { operator, .. } => { /* ... */ }
}
```

The `..` future-proofs against further field additions. The variant is now
`#[non_exhaustive]`, so the compiler will reject field-exhaustive destructures.

### If you call `validate_tap_leaf_subset` directly

Add a `leaf_index: Option<usize>` argument. Pass `None` if you don't have
leaf-index context:

```rust
// Before (v0.4.x):
validate_tap_leaf_subset(&ms)?;

// After (v0.5.0):
validate_tap_leaf_subset(&ms, None)?;
// or, if you have per-leaf context:
validate_tap_leaf_subset(&ms, Some(leaf_index))?;
```

### New encoder behavior

`Descriptor::Tr` with non-trivial `TapTree` (anything other than `TapTree::leaf(ms)`
or KeyOnly) now encodes successfully instead of returning `PolicyScopeViolation`.
The emitted bytecode uses `[Tr=0x06][Placeholder][key_index][TapTree=0x08][LEFT][RIGHT]`
recursive framing.

### New decoder behavior

Bytecode containing `Tag::TapTree (0x08)` now decodes successfully instead of
returning `PolicyScopeViolation("multi-leaf TapTree reserved for v1+")`. The
`decode_report.tap_leaves` field is populated for all `tr(...)` decodes (empty
`Vec` for KeyOnly; one entry per leaf for single-leaf and multi-leaf trees).

### Test vector SHAs

`v0.2.json` SHA changed from `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770`
to `4206cce1f1977347e795d4cc4033dca7780dbb39f5654560af60fbae2ea9c230` (Phase 6
added multi-leaf TapTree fixtures and Phase 11 rolled the family generator
token to `"md-codec 0.5"`). `v0.1.json` SHA changed from
`bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` to
`6d5dd831d05ab0f02707af117cdd2df5f41cf08457c354c871eba8af719030aa` — vector
content is byte-identical aside from the family generator string updating from
`"md-codec 0.4"` → `"md-codec 0.5"`.

The family-stable promise resets at v0.5.0: `"md-codec 0.5"` is the new family
token, and future v0.5.x patch releases will produce byte-identical SHAs.

---

## v0.3.x → v0.4.0

v0.4.0 is wire-format-additive over v0.3.x. Three previously-rejected
top-level descriptor types are now accepted: `wpkh(@0/**)`, `sh(wpkh(@0/**))`,
and `sh(wsh(...))`. v0.3.x-produced strings continue to validate identically
in v0.4.0; v0.4.0-produced strings using the new types will be rejected by
v0.3.x decoders with `PolicyScopeViolation`.

1. **Cargo dependency**: bump `md-codec = "0.3"` → `md-codec = "0.4"`. No
   API changes; no library `use` statement updates needed.
2. **CLI**: `md encode <policy>` now accepts the three new top-level types.
   Existing `wsh(...)`, `tr(...)` invocations unchanged.
3. **Test vector SHAs**: BOTH `v0.1.json` and `v0.2.json` SHA pins changed:
   - `v0.1.json` SHA: `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26` (was `aac3677fd84f06915c7bb5148a25ed80c399daa4f9bf56c8052ed84f83c9b71b`)
   - `v0.2.json` SHA: `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770` (was `18804929d54f94fe4b83a135f3e53d3a26b6ae3565729970ce02ef38f74e9909`)
   - Conformance suites pinning v0.3.x SHAs need a one-time update.
4. **No public API changes**: `MdBackup`, `EncodeOptions`, `WalletPolicy`,
   `Error::PolicyScopeViolation` all unchanged. `PolicyScopeViolation` simply
   fires for fewer inputs.
5. **CLI `--path` ergonomics**: new optional name `bip48-nested` maps to
   indicator `0x06` (BIP 48/1' nested-segwit multisig). Hex (`--path 0x06`)
   and literal-path (`--path "m/48'/0'/0'/1'"`) forms also work.
6. **Restriction matrix is normative**: hardware wallets and other implementers
   producing `sh(...)` strings MUST adhere to the §"Sh wrapper restriction
   matrix" in the BIP — `sh(multi(...))`, `sh(sortedmulti(...))`,
   `sh(pkh(...))`, etc. are permanently REJECTED.

---

## v0.2.x → v0.3.0

v0.3.0 renames the project from "Wallet Descriptor Mnemonic" (WDM) to "Mnemonic Descriptor" (MD). This is a **wire-format-breaking change** because the HRP enters the polymod via HRP-expansion. Strings starting with `wdm1...` are invalid v0.3.0 inputs.

### §1 — Wire format: HRP `wdm` → `md`

The bech32 HRP changes from `wdm` to `md`. Any stored string starting with `wdm1...` cannot be decoded by v0.3.0. To migrate, re-encode from the original descriptor source:

```bash
# v0.2.x: produced wdm1... strings
wdm encode 'wsh(pk(@0/**))'

# v0.3.0: produces md1... strings
md encode 'wsh(pk(@0/**))'
```

The HRP-expansion bytes change from `[3, 3, 3, 0, 23, 4, 13]` (length 7, for HRP `wdm`) to `[3, 3, 0, 13, 4]` (length 5, for HRP `md`), so the polymod-input prefix shrinks by 2 bytes. All checksums are therefore different.

### §2 — Crate name: `wdm-codec` → `md-codec`

Update `Cargo.toml`:

```toml
# Before (v0.2.x):
[dependencies]
wdm-codec = "0.2"

# After (v0.3.0):
[dependencies]
md-codec = "0.3"
```

### §3 — Library import + identifier renames

Update `use` statements and type references:

```rust
// Before (v0.2.x):
use wdm_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};
use wdm_codec::policy::WdmBackup;
use wdm_codec::bytecode::key::WdmKey;
use wdm_codec::encoding::{WDM_REGULAR_CONST, WDM_LONG_CONST};

// After (v0.3.0):
use md_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};
use md_codec::policy::MdBackup;
use md_codec::bytecode::key::MdKey;
use md_codec::encoding::{MD_REGULAR_CONST, MD_LONG_CONST};
```

Type renames: `WdmBackup` → `MdBackup`, `WdmKey` → `MdKey`.

Constant renames: `WDM_REGULAR_CONST` → `MD_REGULAR_CONST`, `WDM_LONG_CONST` → `MD_LONG_CONST`.

### §4 — CLI binary: `wdm` → `md`

The CLI binary is renamed from `wdm` to `md`. The subcommand surface and flags are unchanged:

```bash
# Before (v0.2.x):
wdm encode 'wsh(pk(@0/**))'
wdm decode <string>...
wdm verify <string>... --policy <policy>

# After (v0.3.0):
md encode 'wsh(pk(@0/**))'
md decode <string>...
md verify <string>... --policy <policy>
```

### §5 — Test vector SHAs: both `v0.1.json` and `v0.2.json` changed

Because the HRP-expansion bytes changed, all bech32 checksums in the test vectors changed. Both JSON files were regenerated with new SHA-256 digests:

- `crates/md-codec/tests/vectors/v0.1.json` — new SHA-256: `aac3677fd84f06915c7bb5148a25ed80c399daa4f9bf56c8052ed84f83c9b71b`
- `crates/md-codec/tests/vectors/v0.2.json` — new SHA-256: `18804929d54f94fe4b83a135f3e53d3a26b6ae3565729970ce02ef38f74e9909`

Conformance suites pinning the v0.2.x family-stable SHA `b403073b8a925bdda37adb92daa8521d527476aa7937450bd27fcbe0efdfd072` need a one-time update to the new v0.2.json SHA above. The family-stable promise resets at v0.3.0: future v0.3.x patches will produce byte-identical SHAs (per the design from v0.2.1).

### §6 — Repository URL: unchanged

The repository URL `https://github.com/bg002h/descriptor-mnemonic` is unchanged. Only the crate name and format name changed.

---

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
