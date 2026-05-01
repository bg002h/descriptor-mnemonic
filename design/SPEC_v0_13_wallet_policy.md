# SPEC v0.13 — wallet-policy-with-keys mode

Status: design-approved 2026-04-30. Pre-implementation; superseded by an
implementation plan.

This spec extends md1 v0.11 with concrete wallet keys (BIP 388 wallet
policy), preserving v0.11's bit-aligned wire and forward-compat semantics.
Pre-brainstorm input: `design/PRE_BRAINSTORM_v0_13_wallet_policy.md`.

## 1 Goal and scope

Encode BIP 388 wallet *policies* (template + concrete keys + origin), not
just templates. Preserve enough information to find the next address and
sign all addresses, while letting users elide as much as possible to keep
engravings small. Per-`@N` granularity throughout; different placeholders
may sit at different specification levels.

Out of scope: address-derivation logic, hardware-wallet PSBT integration,
non-mainnet coin support beyond the explicit-origin path. None of these
require wire-format changes.

## 2 Design pillars (locked from pre-brainstorm and brainstorm)

1. Assumed `path-from-master` = BIP-canonical for the top-level wrapper.
2. 16-cell design space: `master_fp × path-from-master × pubkey × use-site`,
   each axis independently elidable per `@N`.
3. Coin = mainnet (0) only as assumed; otherwise origin must be explicit.
4. Account = 0 only as assumed; otherwise origin must be explicit.
5. xpub on the wire = 65 bytes (32 chain code + 33 compressed pubkey).
   `parent_fingerprint` is dropped (cosmetic, not used by any restore/spend
   operation); `depth` and `child_index` are implied by origin path;
   `version` is implied by the network.
6. Optimize the dominant case (cell 7 = `[fp/path]xpub/<0;1>/*`).
7. Wallet-policy-mode dispatch is TLV-presence-driven, not header-bit-driven.
8. Wire encoding uses orthogonal per-axis TLVs; no header expansion.
9. Non-canonical wrappers (`sh(sortedmulti)`, `tr(@N, TapTree)`) force
   explicit origin per `@N`.
10. WalletPolicyId is a canonical-expanded hash, stable across elision on
    path/use-site, presence-significant on fp/xpub.

## 3 Wire format

### 3.1 Header

Unchanged from v0.11. `version = 0` (pre-1.0; we don't enumerate
generations until the v1.0 stability commitment).

### 3.2 New TLV tags

| Tag | Code | Body |
|---|---|---|
| `UseSitePathOverrides` | 0x00 | sparse `(idx, alts)` list (existing, v0.11) |
| `Fingerprints` | 0x01 | sparse `(idx, 4-byte fp)` list (existing, v0.11) |
| `Pubkeys` | **0x02** | sparse `(idx, 65-byte xpub)` list |
| `OriginPathOverrides` | **0x03 (new)** | sparse `(idx, OriginPath)` list |

`Pubkeys` claims tag 0x02, which v0.11 reserved as
`TLV_XPUBS_RESERVED_V0_12`. The implementation must rename/retire that
constant. `OriginPathOverrides` takes the next tag (0x03).

Each TLV body is a length-prefixed concatenation of records (per v0.11's
LP4-ext TLV framing). Each record begins with a `key_index_width`-bit
`idx` (1–4 bits, derived from N per v0.11 convention) followed by the
axis-specific value. Records pack until the TLV's bit-length is exhausted;
the inner value type is self-delimiting (e.g., `OriginPath::read` returns
when its own framing terminates). Within each TLV, idx values MUST be
strictly ascending — matching v0.11's existing `OverrideOrderViolation`
discipline for `UseSitePathOverrides`.

- `Pubkeys` value: 65 bytes = 32 chain code || 33 compressed pubkey.
- `OriginPathOverrides` value: v0.11 `OriginPath` / `PathComponent` types
  (self-delimiting). Variable-length, hardened-or-not flagged BIP 32 path
  components. Emitted *only* when the encoded origin differs from the
  BIP-canonical for the wrapper. A wpkh wallet at the canonical
  `m/84'/0'/0'` emits no entry; a wpkh wallet at `m/84'/0'/5'` (account 5)
  emits the full path.
- `Fingerprints` value: 4 raw bytes.

### 3.3 Mode dispatch

A wire is in **wallet-policy mode** iff the decoded `Pubkeys` TLV contains
at least one entry. The check is a post-TLV-decode predicate; no header
bit, no scan-ahead. Older v0.11 decoders reading v0.13 wire preserve the
`OriginPathOverrides` and `Pubkeys` TLVs as unknown blobs per v0.11 D6
forward-compat — they round-trip but cannot interpret.

### 3.4 Forward-compat

v0.11's D6 unknown-TLV-preservation property carries forward unchanged. A
v0.14+ decoder reading v0.13 wire keeps unknown TLVs verbatim and emits
them on re-encode. v0.13 decoders reading v0.11 wire see no `Pubkeys` TLV
and stay in template-only mode.

## 4 Canonical-origin map

For elided origin paths, the codec computes the canonical origin from the
top-level wrapper.

| Wrapper | Canonical `path-from-master` |
|---|---|
| `pkh(@N)` | `m/44'/0'/0'` |
| `wpkh(@N)` | `m/84'/0'/0'` |
| `tr(@N)` (key-path only) | `m/86'/0'/0'` |
| `wsh(multi/sortedmulti)` | `m/48'/0'/0'/2'` ¹ |
| `sh(wsh(multi/sortedmulti))` | `m/48'/0'/0'/1'` ¹ |
| `sh(sortedmulti)` | **none — must be explicit** |
| `tr(@N, TapTree)` | **none — must be explicit** |

The codec exposes `canonical_origin(wrapper) -> Option<OriginPath>`. `None`
return means the encoder must emit `OriginPathOverrides` entries for all
`@N` in those wrappers; the decoder rejects wires that omit them.

Coin (BIP 32 second component) = `0' / mainnet`; account = `0'`. Any
deviation forces full explicit origin via `OriginPathOverrides`.

¹ BIP 48 path layout is `m/48'/coin'/account'/script_type'`. The trailing
component (`2'` for segwit-multi, `1'` for nested-segwit-multi) is the
script-type field; account = `0'` is the third component.

## 5 Identity hashes

Three identity hashes total, each rendered as a 12-word BIP 39 phrase via
the existing `Phrase::from_id_bytes`.

### 5.1 Md1EncodingId (existing v0.11)

`SHA-256(full_wire_bytes)[0..16]`. Wire-level. Sensitive to TLV ordering,
padding, and elision choices. Stable for a specific engraving.

### 5.2 WalletDescriptorTemplateId (existing v0.11)

Hashes use-site-path-decl + tree + `UseSitePathOverrides` TLV bits. Captures
template + use-site shape only; ignores keys and origin. (See v0.11
`identity::WalletDescriptorTemplateId` for the exact construction.)

### 5.3 WalletPolicyId (new, v0.13)

Canonical-expanded hash. Stable across elision on path/use-site axes;
presence-significant on fp/xpub axes.

**Hash input** is the deterministic byte serialization, in order:

```
canonical_template_tree_bytes
||
for idx in 0..n (each unique placeholder index, ascending):
    canonical_record_for_@idx
```

where `n` is the placeholder count derived from the tree (one record per
unique `@N`, regardless of how many AST positions reference it), and each
`canonical_record_for_@idx` is:

```
[ presence_byte(1 byte)
| path_bit_len(LP4-ext varint) | path_bits  (zero-padded to byte boundary)
| use_site_bit_len(LP4-ext varint) | use_site_bits  (zero-padded to byte boundary)
| fp_4_bytes_if_present
| xpub_65_bytes_if_present ]
```

`presence_byte` bit 0 = `fp_present`, bit 1 = `xpub_present`, bits 2..7
reserved. Encoders MUST set reserved bits to 0; the hash is
implementation-defined for inputs with non-zero reserved bits, since
conforming encoders never produce them.

`canonical_template_tree_bytes` is the bit-aligned `Tree::write` output
of the placeholder-form template (no concrete keys), zero-padded to a
whole-byte boundary. This includes the wrapper tag, so wrapper context is
part of the hash and policies on different wrappers cannot collide on
identical per-`@N` records.

`path_bits` and `use_site_bits` are the bit-aligned `OriginPath::write` /
`UseSitePath::write` outputs respectively. The preceding `path_bit_len` /
`use_site_bit_len` LP4-ext varints (per v0.11 framing) are in *bits*, not
bytes; the bit stream is zero-padded to a byte boundary before the next
field. This makes the canonical record byte-aligned for hashing while
preserving exact bit-stream content (no canonicalization of internal
encoding choices within the BIP 32 path / multipath alternatives).

Field omission rules:

- `fp_present = 0` ⟹ `fp_4_bytes_if_present` is **omitted entirely** (zero
  bytes contributed).
- `xpub_present = 0` ⟹ `xpub_65_bytes_if_present` is **omitted entirely**.
- `path_bits` and `use_site_bits` are always present in canonical form
  (see below).

The other
two axes (path, use-site) are always present in canonical form because
their elided cases have canonical defaults:

- Path elided ⟹ fill in `canonical_origin(wrapper)`. If the wrapper has no
  canonical, the original wire was forced explicit (§4), so a path is
  always present.
- Use-site elided ⟹ fill in `<0;1>/*` (`standard_multipath`).

**Hash output:** `SHA-256(canonical_input)[0..16]`.

**Property: stability across origin-elision.** A user who engraves with
assumed-BIP-84 origin and another who explicitly encodes `m/84'/0'/0'`
produce identical canonical records → identical WalletPolicyId.

**Property: presence-significance on fp/xpub.** A user with no master_fp
and no concrete xpub (template-only) produces a different WalletPolicyId
than a user with both — by design, since they describe genuinely different
levels of policy specification.

**Edge case — partial keys.** `@0` with fp+xpub and `@1` without
produces presence_bytes 0b11 and 0b00 respectively. WalletPolicyId is
distinct from both "all keys present" and "all keys absent" cases.

## 6 Validation rules

### 6.1 Placeholder ordering (BIP 388)

The encoder canonicalizes placeholder indices so `@i` first appears in
the tree before `@j` for `j > i`. The decoder validates the same;
violations → `Error::PlaceholderOrderingViolation { placeholder_index: u8 }`.

This applies to v0.11 wires too (it was implicit in template-only mode),
but is now explicit in the v0.13 spec because TLV-driven keys make
ordering a wire-format concern, not just a tree-shape concern.

### 6.2 Index range

Every `(idx, ...)` entry in any TLV must have `idx < N`, where N is the
placeholder count derived from the tree. Out-of-range entries →
`Error::PlaceholderIndexOutOfRange`.

### 6.3 Required explicit origin

If the wrapper is non-canonical (§4) and any `@N` has no
`OriginPathOverrides` entry, decode fails with
`Error::MissingExplicitOrigin { idx: u8 }`.

### 6.4 xpub validity

`Pubkeys` xpub bytes that don't decode as a valid secp256k1 point fail
with `Error::InvalidXpubBytes`.

### 6.5 Sparseness allowed

Sparse TLVs are valid: a v0.13 wire may have keys for some `@N` and not
others (cell 1/3/7 mixed). WalletPolicyId reflects the per-`@N`
specification level via presence_byte.

## 7 Decoder dispatch

The v0.13 decoder reads in this order:

1. Header per v0.11 §3.1 → `version=0`, `divergent_paths`, `n`,
   `key_index_width`.
2. Tree → `n` placeholders.
3. TLV section → populates per-axis sparse maps.
4. **Mode predicate**: wallet-policy mode = `!Pubkeys.is_empty()`.
5. Per-`@N` axis resolution: TLV entry if present; else BIP-canonical
   default for path/use-site; else absent for fp/xpub.
6. Validate (§6).
7. Compute identity hashes lazily (Md1EncodingId on raw wire bytes;
   template + policy IDs on canonicalized inputs).

## 8 Error variants

Additive variants on `Error`:

- `MissingExplicitOrigin { idx: u8 }`
- `PlaceholderIndexOutOfRange { idx: u8, n: u8 }`
- `InvalidXpubBytes { idx: u8 }`
- `PlaceholderOrderingViolation { placeholder_index: u8 }`

## 9 Test coverage (informative)

- Smoke: 1-of-1 cell-7 round trip; 2-of-3 cell-7 round trip; 1-of-1 cell-1
  (template-only) decoded by v0.13 decoder, treated as v0.11-equivalent.
- Canonicalization: encode `wpkh(@0)` with assumed origin, encode same with
  explicit `m/84'/0'/0'`, assert identical WalletPolicyId.
- Partial keys: 2-of-3 with `@0` cell-7, `@1` cell-1, `@2` cell-1 round
  trips; WalletPolicyId distinct from full cell-7 version.
- Forced explicit: encode `sh(sortedmulti(...))` with assumed origin →
  encoder error.
- Cross-version: v0.11 decoder reads v0.13 wire, preserves `Pubkeys` and
  `OriginPathOverrides` as unknown blobs, round-trips back to identical
  bytes.
- BIP 388 ordering: decoder rejects wires with placeholder ordering
  violations.
- Divergent-paths × wallet-policy: a wpkh wallet with `divergent_paths=1`
  AND per-`@N` `OriginPathOverrides` round-trips, and its WalletPolicyId
  is stable when the same logical wallet is re-encoded with assumed paths
  where canonical (where applicable).
- Multi-chunk wallet-policy: a 2-of-3 cell-7 wallet (~5–7 codex32 chunks)
  splits, reassembles, and round-trips end-to-end; ChunkSetId is derived
  consistently across chunks.
- v0.11 forward-compat byte-exactness: re-encoding a v0.13 wire through a
  v0.11 decoder produces byte-identical output to the original wire.

## 10 References

- BIP 32, 380, 386, 388, 389
- v0.11 SPEC: archived in `debris/SPEC_v0_11_wire_format.md`
- v0.11 BRAINSTORM: in-tree at `design/BRAINSTORM_v0_11_wire_format.md`
- Pre-brainstorm: `design/PRE_BRAINSTORM_v0_13_wallet_policy.md`
- Upstream miniscript blockers (gate downstream consumption if/when
  wallet-policy materializes through `miniscript::WalletPolicy`):
  - `rust-miniscript#935` — hash-terminal support
  - `rust-miniscript#936` — `template()` / `key_info()` accessors
  - `rust-miniscript#934` — `set_key_info` AST-vs-placeholder ordering
