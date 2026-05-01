# SPEC — md-codec v0.11 wire format

> **Historical document.** This document is the original v0.11 design and is preserved for historical reference. Normative spec for current shipped behavior is `design/SPEC_v0_13_wallet_policy.md`. Hot-fix corrections from v0.12.1 are listed in the appendix.
>
> **Status:** Draft, derived from `design/BRAINSTORM_v0_11_wire_format.md` (decisions D1–D37). For implementation per the per-phase TDD-disciplined plan that will follow this spec.
>
> **Companion file:** `design/REJECTEDFORNOW.md` documents design alternatives that were considered and deferred to future versions.

---

## 1. Introduction

### 1.1 Purpose

md-codec is a wire-format codec for engraving BIP 388 wallet descriptor templates onto durable media (typically metal). The encoded form is a codex32 string with HRP `md1` and a BCH checksum (BIP 93 polynomial parameters). The codec produces compact, human-transcribable backups of wallet templates so signing parties can reconstruct policies from their own keys plus the engraved template.

v0.11 is a from-scratch redesign relative to v0.0–v0.10 (which all shared a byte-aligned legacy wire format). The redesign is permitted because v0.x has zero current users; the v0.10 → v0.11 break is free.

### 1.2 Optimization target

Relative, not absolute:

- Shortest reasonable engravings for common wallet types (BIP 84 single-sig, BIP 86 taproot, 2-of-3 BIP 48 multisig with shared origin path).
- Pay-for-what-you-use: rare features (divergent origins, multipath-shared `@N`, fingerprints, embedded xpubs) cost extra; common cases pay zero.
- Sane length limits where natural (max placeholders, max path components, max multipath cardinality).
- Common cases no worse than v0.10; uncommon cases free to be slightly longer if they buy cleaner v1.0 design.

### 1.3 Goal

Full BIP 388 wallet descriptor template grammar coverage. No deliberate scope cuts.

### 1.4 What v0.11 does NOT ship

- **Embedded xpubs (wallet-policy mode).** Reserved for v0.12 as the additive `Xpubs` TLV (tag 0x02). v0.11 wire form is forward-compat clean; v0.11 decoders skip the unknown TLV gracefully.
- **Wire-layer dictionaries** (path, use-site-path, shape). Considered and rejected for architectural cleanliness; preserved as a v0.12+/vendor-extension option per §6 of the brainstorm.
- **Display-layer shape dictionary.** Wallet UIs are free to recognize and label common shapes via their own conventions; spec doesn't curate names.
- **Keyless templates (n=0).** Disallowed; future keyless need handled via distinct HRP (e.g., `ks1`).

---

## 2. Vocabulary

Terminology aligns with BIP 388 / BIP 380 / BIP 32 / miniscript where possible. Coined md-codec terms are explicitly avoided.

| Term | Meaning | Source |
|---|---|---|
| **Wallet descriptor template** | The `@N`-placeholder form with use-site paths and script structure | BIP 388 |
| **Wallet policy** | (template + key information vector). The full thing | BIP 388 |
| **Key information vector** | Array of `[fp/origin]xpub` entries, one per `@N` | BIP 388 |
| **Output descriptor** | BIP 380 form — fully resolved, no `@N` | BIP 380 |
| **Account** | BIP 32 account level | BIP 32 / 44 |
| **Policy** (lowercase) | Reserved for miniscript Policy. NOT used for BIP 388 things | miniscript |
| **Key placeholder (KP)** / **key index (KI)** | The `@N` reference / the `i` in `@i` | BIP 388 |

### 2.1 v0.11 identifier names

- **`WalletDescriptorTemplateId`** — 128-bit hash over canonical BIP 388 wallet-descriptor-template content (use-site-path-decl + tree + `UseSitePathOverrides` TLV bits — see §8.1 for full domain definition). Always computable from any v0.11+ md1 string. 12-word phrase rendering attaches to this ID by default.
- **`Md1EncodingId`** — 128-bit hash over the full canonical md1 bit-stream (header + path-decls + tree + entire TLV section, excluding HRP and BCH checksum). Always computable. Identifies a specific engraving; basis for ChunkSetId derivation.
- **`WalletPolicyId`** — 128-bit hash over canonical BIP 388 wallet-policy content. Computable only when the `Xpubs` TLV is present (v0.12+).

### 2.2 Locked carry-overs from v0.x

| Item | What | Reference |
|---|---|---|
| codex32 BCH layer | BIP 173-style HRP-mixing + BIP 93 polymod | BIP 93 |
| Polynomial parameters | `POLYMOD_INIT`, `GEN_REGULAR`, shifts/masks | BIP 93 |
| `shibbolethnums` NUMS preimage | `SHA-256("shibbolethnums")` derives `MD_REGULAR_CONST` (top 65 bits) | v0.x |
| HRP `md` | Two-character HRP | v0.x |

---

## 3. Wire format overview

### 3.1 String layout

A v0.11 md1 string has the form:

```
HRP (md1) | payload | BCH checksum
```

- **HRP**: literal `md1` (3 ASCII chars: `md` + separator `1`).
- **Payload**: the bit-aligned encoding defined in §3.3–§3.7. Length variable; bit-packed MSB-first into codex32 5-bit symbols.
- **BCH checksum**: BIP 93 codex32 BCH (regular: 13 symbols / 65 bits). Long-form codex32 was dropped in v0.12.0; v0.11+ uses regular form only.

The HRP and checksum are governed by BIP 93's codex32 specification; they are not part of the payload encoding defined here. The target residue is `MD_REGULAR_CONST` (top 65 bits of `SHA-256("shibbolethnums")`); long-form codex32 (and its `MD_LONG_CONST`) was stripped in v0.12.0 and is not part of v0.11+ as shipped.

### 3.2 Payload structure

The payload is a bit-aligned stream consisting of (in order):

```
header                     5 bits
origin-path-decl           variable
use-site-path-decl         variable
tree                       variable
TLV section                variable (may be empty)
```

The TLV section ends when the codex32 total-length is exhausted; no explicit terminator.

### 3.3 Header (5 bits)

```
bit 4   bit 3   bit 2   bit 1   bit 0
[paths] [resv]  [version (3 bits)            ]
```

- **Bits 0–2:** version field. v0.11 = 0; future versions increment.
- **Bit 3:** reserved. v0.11 encoders MUST emit 0; decoders MUST reject 1 in payload-header position.
- **Bit 4:** all-keys-same-path flag. `0` = shared origin path (one path applies to all `@N`); `1` = divergent origin paths (per-`@N` paths follow).

**Note on bit-3 dual use across payload header and chunk header.** The payload header (this section) reserves bit 3 = 0. The chunk header (§9.3) places its `chunked` flag at the same bit-3 position; in chunk headers, this bit is set to 1 to indicate "this codex32 string is one chunk in a chunked set." Decoders dispatch chunked-vs-single-string by examining bit 3 of the first 5-bit symbol after the HRP — bit 3 = 0 means the symbol is a payload header (single-string mode); bit 3 = 1 means it's a chunk header (chunked mode). See §13.2 for decoder pseudocode.

### 3.4 Origin path declaration

The origin-path-decl block dispatches on header bit 4.

#### 3.4.1 Shared mode (bit 4 = 0)

```
[n: 5 bits]                         # key-count, range 1..32
[origin-path-encoding]               # one shared path
```

#### 3.4.2 Divergent mode (bit 4 = 1)

```
[n: 5 bits]                         # key-count
[origin-path-encoding × n]          # one path per @N, in @N-ascending order
```

#### 3.4.3 origin-path-encoding (explicit-only)

```
[depth: 4 bits]                     # 0..15 components
[component × depth]
```

#### 3.4.4 component

```
[hardened: 1 bit][value: LP4-ext varint]
```

`hardened = 1` indicates the BIP 32 hardened bit is set. `value` is the un-hardened-bit u31 value.

### 3.5 Use-site path declaration

Always present (one shared default, applied to all `@N` unless overridden by `UseSitePathOverrides` TLV).

```
[has-multipath: 1 bit]
[if has-multipath:
  [alt-count: 3 bits]               # encode count - 2; range 2..9
  [alternative × count]
]
[wildcard-hardened: 1 bit]
```

#### 3.5.1 alternative

```
[hardened: 1 bit][value: LP4-ext varint]
```

### 3.6 Tree

The tree is a recursive AST of operator nodes. Each node is `[tag][body]` where `tag` is a 5-bit primary code (or 5-bit extension prefix + 5-bit extension code) and `body` depends on the tag (see §6).

The tree's root is the wallet template's top-level constructor (one of `Wpkh`, `Tr`, `Wsh`, `Sh`, `Pkh`).

### 3.7 TLV section

The TLV section appears immediately after the tree and continues until the codex32 total-length is exhausted. Each entry is:

```
[tlv-tag: 5 bits]
[length: LP4-ext varint, in BITS]
[payload: length bits]
```

Entries appear in strict ascending tag-value order. Duplicate tags MUST be rejected by decoders. Unknown tags are skipped via length-prefix advancement (forward-compat).

#### 3.7.0 End-of-section detection (rollback-as-padding)

The TLV section has no explicit terminator: it runs until the codex32 symbol-aligned bit stream is exhausted. Because the wire is packed into 5-bit codex32 symbols, the final symbol can introduce up to 4 bits of trailing zero-padding that are not part of any TLV entry. Decoders therefore need a precise contract for distinguishing "end of section reached" from "malformed TLV entry."

Decoders SHOULD use a save/restore-position mechanism on the BitReader (the shipped reference uses `BitReader::save_position` / `restore_position`) so that partial-read tags — a tag prefix without enough remaining bits for a full record, an apparent ordering violation against trailing zero bits, an apparent zero-length entry, or any TLV-record parse failure that occurs near the end of the bit stream — do not consume bits prematurely. The contract is:

1. Before parsing each candidate TLV entry, save the BitReader position.
2. Attempt to read `[tag : 5][length : LP4-ext varint][payload : length bits]` plus any per-tag inner-record validation.
3. On any parse failure, restore the BitReader to the saved position and inspect the bits remaining at that position.
4. **If `remaining_bits ≤ 7`**, those bits are zero-padding from the codex32 symbol-aligned encoding and MUST be tolerated: the decoder treats the failure as a clean end-of-section marker and stops.
5. **If `remaining_bits > 7`**, the failure represents a genuinely malformed input and the original parse error is propagated.

The 7-bit padding tolerance accommodates the symbol-aligned wire's worst-case trailing slack. (Specifically: a codex32 wire's payload bit count is a multiple of 5; the chunk-header path adds 37 bits of header before whole-byte payloads, again ≤7 bits of slack at the byte boundary.) When fewer than 5 bits remain at a TLV-section boundary, those bits are unambiguously zero-padding and the decoder MUST stop without raising an error.

Encoders MUST omit empty TLVs (zero-length payload) per §3.7.2 / §3.7.3; decoders therefore treat any zero-length TLV at the end of stream as padding via the rollback path rather than as a valid empty entry.

#### 3.7.1 v0.11 starter TLV tag values

| Code | TLV entry | Spec section | In WDT-Id hash domain (§8.1)? |
|---|---|---|---|
| 0x00 | UseSitePathOverrides | §3.7.2 | yes |
| 0x01 | Fingerprints | §3.7.3 | **no** (advisory metadata) |
| 0x02 | RESERVED for v0.12 Xpubs TLV (superseded — see note below) | — | (see note below) |
| 0x03..0x1F | RESERVED for future TLVs | — | per-tag classification at the time it is specified |

**Note on tag 0x02:** v0.11 reserved this tag for a v0.12 `Xpubs` TLV. The v0.12 Xpubs design did not ship; the tag was reallocated in v0.13 to `Pubkeys` (sparse `(idx, 65-byte xpub)` list — see `design/SPEC_v0_13_wallet_policy.md` §3.2). v0.11 decoders treat tag 0x02 as an unknown TLV (forward-compat skip via length-prefix advancement). The constant `TLV_XPUBS_RESERVED_V0_12` was renamed/retired by v0.13's `Pubkeys` allocation.

Each future TLV tag is explicitly classified at the time of its specification as either *included in* or *excluded from* the `WalletDescriptorTemplateId` hash domain (§8.1). The default classification for advisory metadata (e.g., user-facing labels, signing-time hints) is *excluded*; the default for content that affects wallet identity / restoration semantics is *included*. There is no fixed tag-value range that determines classification — the classification is per-TLV-tag.

#### 3.7.2 UseSitePathOverrides (TLV tag 0x00)

Sparse list of per-`@N` use-site-path overrides. When present, listed `@N` indices use the override path instead of the shared default.

```
payload (length bits exhausted by these):
  while bits_remaining_in_payload > 0:
    [@N-index: ⌈log₂(n)⌉ bits]
    [use-site-path: per §3.5 (5-bit-or-more encoding)]
```

Validation:
- Each `@N`-index < n.
- No duplicate `@N`-index.
- Indices in canonical-ascending order.
- Entry MUST contain ≥ 1 override (encoder omits the TLV when none diverge).

#### 3.7.3 Fingerprints (TLV tag 0x01)

Sparse list of per-`@N` master-fingerprints. Optional metadata; helps signers identify their own keys at restore time.

```
payload (length bits exhausted by these):
  while bits_remaining_in_payload > 0:
    [@N-index: ⌈log₂(n)⌉ bits]
    [fingerprint: 32 bits, BIP 32 byte order]
```

Validation:
- Each `@N`-index < n.
- No duplicate `@N`-index.
- Indices in canonical-ascending order.
- Entry MUST contain ≥ 1 fingerprint (encoder omits the TLV when no fingerprints).
- "No entry for `@N`" is semantically equivalent to "no fingerprint for `@N`."

---

## 4. Encoding primitives

### 4.1 LP4-ext varint

Variable-length integer encoding for path component values and TLV length fields. Bit-aligned.

```
[L: 4 bits]                   # length of payload in bits
[payload: L bits]             # value, MSB-first
```

If L = 15 (maximum), an extension hop follows:

```
[L: 4 bits = 15]
[L_high: 4 bits]              # additional payload bits = L_high
[payload_low: 14 bits]
[payload_high: L_high bits]
total payload = (payload_high << 14) | payload_low
```

For values requiring more than 28 bits (covered by L=15 + L_high=14), a recursive extension applies (rare for path components; covers full u32 timelocks via 32-bit fixed encoding instead — see §4.3).

#### 4.1.1 Bit costs

| Value | Encoding | Bits |
|---|---|---|
| 0 | L=0 | 4 |
| 1 | L=1 + payload(1) | 5 |
| 84 | L=7 + payload(0b1010100) | 11 |
| 1024 | L=11 + payload(0b10000000000) | 15 |
| 16383 | L=14 + payload(14 bits) | 18 |
| 16384 | L=15 + L_high=1 + payload (extension) | 23 |

### 4.2 Fixed-width fields

| Field | Width | Encoded value | Logical range |
|---|---|---|---|
| Header version | 3 bits | raw value | 0..7 |
| Header reserved | 1 bit | 0 (MUST) | — |
| Header all-keys-same-path | 1 bit | raw value | 0 or 1 |
| Key-count `n` | 5 bits | encodes `n − 1` | logical 1..32 |
| `k` in k-of-n (multisig, thresh) | 5 bits | encodes `k − 1` | logical 1..32 |
| Variable-arity child count (in tree, e.g., `thresh`'s `n` field) | 5 bits | encodes `count − 1` | logical 1..32 |
| Path-component depth | 4 bits | raw value | 0..15 |
| Multipath alt-count | 3 bits | encodes `count − 2` | logical 2..9 |
| TLV tag | 5 bits | raw value | 0..31 |
| Hardened bit | 1 bit | raw | 0 or 1 |
| Has-multipath / has-tree / wildcard-hardened | 1 bit | raw | 0 or 1 |
| Primary tag | 5 bits | raw | 0x00..0x1F |
| Extension tag (after 0x1F primary) | 5 bits | raw | 0x00..0x1F |

**Count-field offset encoding:** all fixed-width count/cardinality fields whose logical minimum is non-zero use offset encoding (encode `logical_value − offset`) so that the full 2^width logical-range space is reachable. Specifically:

- `n` (key-count), `k` (threshold), variable-arity child count: 5-bit field encoding `value − 1`. Logical 1 → encoded 0; logical 32 → encoded 31. Logical value 0 is unrepresentable (and disallowed per §7.2 / §7.4).
- Multipath alt-count: 3-bit field encoding `value − 2`. Logical 2 → encoded 0; logical 9 → encoded 7. Logical values 0 and 1 are unrepresentable (alt-count ≥ 2 by definition).

Encoders MUST emit the encoded value (i.e., `logical − offset`); decoders MUST add the offset back to recover the logical value.

### 4.3 Bitcoin-native u32 (timelocks)

`after(n)` and `older(n)` arguments use Bitcoin-native u32 encoding: 32 bits, MSB-first, byte-identical to nLockTime / nSequence in transactions.

```
[after-arg: 32 bits, MSB-first]
[older-arg: 32 bits, MSB-first]
```

This sacrifices ~10-20 bits per timelock occurrence vs varint encoding but provides 1:1 cross-tool legibility with Bitcoin transaction values.

### 4.4 Hash literals

Hash literal arguments to `Sha256`, `Hash256`, `Ripemd160`, `Hash160`, `RawPkH`:

| Op | Hash size |
|---|---|
| Sha256, Hash256 | 256 bits, raw |
| Ripemd160, Hash160, RawPkH | 160 bits, raw |

Hash bits are emitted MSB-first within each byte; byte order is the natural BIP 32 / Bitcoin order (big-endian for HASH160 outputs as Bitcoin uses them).

### 4.5 Placeholder index encoding

At every key position in the tree:

```
[index: ⌈log₂(n)⌉ bits]
```

`n` is the key-count parsed at the head of the path-decl block (§3.4). For `n = 1` (single-sig), index width is 0 bits — the placeholder reference is implicit (no bits emitted; @0 is the only valid value).

### 4.6 Bit-packing convention

The payload bit stream is packed into codex32 5-bit symbols MSB-first:

- The first bit of the payload occupies the most-significant bit of the first codex32 symbol.
- Bits proceed through symbols left-to-right.
- The final symbol may be partially used; remaining bits are zero-padded to fill the symbol.

For SHA-256 input (used by ID hashing in §8), the same bit stream is packed into bytes MSB-first:

- The first bit of the canonical bit stream is the MSB of byte 0.
- Bits proceed through bytes left-to-right.
- The final byte is zero-padded if necessary.

---

## 5. Tag-space allocation

### 5.1 Primary 5-bit space

| Code | Op | Body |
|---|---|---|
| 0x00 | Wpkh | key-arg |
| 0x01 | Tr | key-arg + has-tree:1 + [optional tree] |
| 0x02 | Wsh | child × 1 |
| 0x03 | Sh | child × 1 |
| 0x04 | Pkh | key-arg |
| 0x05 | TapTree | child × 2 |
| 0x06 | Multi | k:5 + n:5 + key-arg × n |
| 0x07 | SortedMulti | k:5 + n:5 + key-arg × n |
| 0x08 | MultiA | k:5 + n:5 + key-arg × n |
| 0x09 | SortedMultiA | k:5 + n:5 + key-arg × n |
| 0x0A | PkK | key-arg |
| 0x0B | PkH | key-arg |
| 0x0C | Check (`c:`) | child × 1 |
| 0x0D | Verify (`v:`) | child × 1 |
| 0x0E | Swap (`s:`) | child × 1 |
| 0x0F | Alt (`a:`) | child × 1 |
| 0x10 | DupIf (`d:`) | child × 1 |
| 0x11 | NonZero (`j:`) | child × 1 |
| 0x12 | ZeroNotEqual (`n:`) | child × 1 |
| 0x13 | AndV | child × 2 |
| 0x14 | AndB | child × 2 |
| 0x15 | AndOr | child × 3 |
| 0x16 | OrB | child × 2 |
| 0x17 | OrC | child × 2 |
| 0x18 | OrD | child × 2 |
| 0x19 | OrI | child × 2 |
| 0x1A | Thresh | k:5 + n:5 + child × n |
| 0x1B | After | u32 timelock |
| 0x1C | Older | u32 timelock |
| 0x1D | Sha256 | 256-bit hash |
| 0x1E | Hash160 | 160-bit hash |
| 0x1F | **Extension prefix** | — |

### 5.2 Extension space (after 0x1F primary)

| Code | Op | Body |
|---|---|---|
| 0x00 | Hash256 | 256-bit hash |
| 0x01 | Ripemd160 | 160-bit hash |
| 0x02 | RawPkH | 160-bit hash |
| 0x03 | False | — (no body) |
| 0x04 | True | — (no body) |
| 0x05..0x1F | RESERVED for future fragments | — |

Extension reads as: `[primary: 0x1F][extension: 5 bits]`. Total wire cost for an extension op is 10 bits.

---

## 6. Operator body encoding (per-tag rules)

### 6.1 Class 1 — Fixed-arity, no body fields

After the tag, body is N child encodings (recursive):

| Op | Children |
|---|---|
| Wrappers (Alt, Swap, Check, DupIf, Verify, NonZero, ZeroNotEqual) | 1 |
| AndV, AndB, OrB, OrC, OrD, OrI | 2 |
| AndOr | 3 |
| Sh, Wsh | 1 |
| Pkh, Wpkh, PkK, PkH | 1 (key-arg per §4.5) |
| RawPkH | 1 (160-bit hash literal per §4.4) |
| TapTree | 2 (each child is a leaf or another TapTree node) |
| False, True | 0 |

### 6.2 Class 2 — Variable-arity (Multi, SortedMulti, MultiA, SortedMultiA, Thresh)

```
[tag][k: 5 bits][n: 5 bits][child × n]
```

For multisig family (Multi, SortedMulti, MultiA, SortedMultiA), each child is a key-arg (§4.5). For Thresh, each child is a miniscript fragment (recursive tree node).

`k`, `n` are 5-bit fixed; range 1..32. Decoder validates `1 ≤ k ≤ n`.

### 6.3 Class 3 — Tr (taproot top-level)

```
Tr body:
  [key-arg]                # encoding per §4.5
  [has-tree: 1 bit]
  [if has-tree: tap-script-tree]
```

`tap-script-tree` is recursive: either a leaf miniscript fragment or a `TapTree`-tag inner node with two children (each child itself a `tap-script-tree`).

#### 6.3.1 Tap-script-tree leaf restriction

A leaf in a tap-script-tree MUST be a miniscript fragment that is type-valid in the tap-script (taproot leaf) execution context per BIP 388 / miniscript typing. Concretely:

**Permitted leaf tags:**
- `MultiA` (0x08), `SortedMultiA` (0x09) — taproot k-of-n multisig forms
- `PkK` (0x0A), `PkH` (0x0B) — key-spend fragments
- All wrappers (0x0C–0x12), all logical operators (0x13–0x1A), `After` (0x1B), `Older` (0x1C), `Sha256` (0x1D), `Hash160` (0x1E)
- Extension-space: `Hash256` (0x00), `Ripemd160` (0x01), `RawPkH` (0x02), `False` (0x03), `True` (0x04)

**Forbidden leaf tags:**
- Top-level constructors: `Wpkh` (0x00), `Tr` (0x01), `Wsh` (0x02), `Sh` (0x03), `Pkh` (0x04) — these are top-level descriptor wrappers, not script-context fragments
- `Multi` (0x06), `SortedMulti` (0x07) — wsh-only multisig, forbidden in tap-script context per miniscript typing

Decoders MUST reject a tap-script-tree containing a forbidden leaf tag (see §7.4).

`has-tree = 0` is the BIP 86 case (single-sig taproot, dominant). `has-tree = 1` enters multi-leaf taproot territory.

### 6.4 Class 4 — Argument terminals

| Argument | Encoding | Bits |
|---|---|---|
| Key arg (template mode) | placeholder index per §4.5 | ⌈log₂(n)⌉ |
| Key arg with embedded xpub (v0.12+) | resolved via Xpubs TLV using `@N` index | (TBD in v0.12) |
| Hash literal — 256 bits | bit-aligned raw | 256 |
| Hash literal — 160 bits | bit-aligned raw | 160 |
| Timelock — After, Older | Bitcoin-native u32 per §4.3 | 32 |

### 6.5 Wrapper chains

Miniscript wrapper chains like `vc:pk(@0)` parse as `Verify(Check(PkK(@0)))` — three nested AST nodes, each with its own 5-bit primary tag. One-tag-per-wrapper; no wrapper-stack compression.

Note: miniscript wrappers `t:`, `l:`, `u:` are syntactic sugar that desugar to existing AST nodes; they have no wire-distinct tags.

---

## 7. Validation invariants

Decoders MUST enforce all of the following. Failure to enforce any is a spec violation.

### 7.1 Header

- Version field equals 0 (v0.11). Other version values are reserved for future spec revisions.
- Bit 3 (reserved) equals 0. Bit-3 = 1 is rejected.
- Bit 4 must be a valid flag value (0 or 1) — both are valid.

### 7.2 Path declaration

- `n ≥ 1` (no keyless templates).
- `n` matches `max(@N seen in tree) + 1` (BIP 388 well-formedness).
- Each `@i` for `0 ≤ i < n` appears at least once in the tree.
- First occurrences of `@i` in the tree are in canonical ascending order (BIP 388).
- Path-component depth ≤ 15.
- All path component values fit in u31 (hardened bit accounted for separately).

### 7.3 Use-site path declaration

- alt-count (when has-multipath = 1) is in 2..9 (encoded as count − 2 = 0..7).
- All use-site paths in a template (shared default + UseSitePathOverrides) that have multipaths share the same alt-count (BIP 388 multipath consistency).

### 7.4 Tree

- Every `@N` index in the tree is < n.
- Operator arities match §6.
- For Thresh and multisig: `1 ≤ k ≤ n`.
- TapTree nodes appear only inside Tr's tree body (recursive tap-script-tree).
- Tap-script-tree leaves use only permitted tags per §6.3.1; forbidden tags rejected.
- Tag values not in §5.1 or §5.2 are rejected.

### 7.5 TLV section

- TLV tags appear in strictly ascending order.
- No duplicate TLV tags.
- Each TLV entry's payload length is ≥ 0 bits (zero-payload TLVs are valid in principle but encoder MUST omit empty TLVs per §3.7.2 / §3.7.3).
- Within `UseSitePathOverrides`: `@N`-index ascending, < n, no duplicates; ≥ 1 entry.
- Within `Fingerprints`: `@N`-index ascending, < n, no duplicates; ≥ 1 entry.
- Unknown tags are skipped via length-prefix advancement; do not cause rejection.

### 7.6 Cross-cutting

- The codex32 BCH checksum must validate (BIP 93).
- The HRP must be `md1`.
- For chunked encodings (§9), all chunks must share version, chunk-set-id, and count; indices must be 0..count-1 with no duplicates or gaps; reassembled `Md1EncodingId[0..20]` must match the chunk-set-id.

---

## 8. Identity

### 8.0 TLV hash domain summary

Each identity hash in §8 is computed over a different subset of the wire. To make the per-TLV classification explicit, the table below lists each TLV defined in v0.11 (and the v0.13-allocated additions on tag 0x02 and 0x03) and the identity-hash domain(s) in which it participates.

| TLV | Tag | In `WalletDescriptorTemplateId` domain (§8.1)? | In `Md1EncodingId` domain (§8.2)? | In `WalletPolicyId` domain (v0.13 §5.3)? |
|---|---|---|---|---|
| `UseSitePathOverrides` | 0x00 | **yes** (only TLV included) | yes (full payload) | yes (canonical-expanded use-site axis) |
| `Fingerprints` | 0x01 | no (advisory metadata) | yes (full payload) | yes (presence-significant fp axis) |
| `Pubkeys` (v0.13) | 0x02 | no | yes (full payload) | yes (presence-significant xpub axis) |
| `OriginPathOverrides` (v0.13) | 0x03 | no | yes (full payload) | yes (canonical-expanded path axis) |

In short:

- **`WalletDescriptorTemplateId` (§8.1)** hashes ONLY `UseSitePathOverrides` (tag 0x00) among TLVs. Everything else — `Fingerprints`, and the v0.13-allocated `Pubkeys` / `OriginPathOverrides` — is excluded. The WDT-Id captures BIP 388 template + use-site shape and is invariant to fingerprints, embedded keys, and origin-path metadata.
- **`Md1EncodingId` (§8.2)** hashes the entire payload including all TLVs verbatim, so any TLV change (including adding/removing `Fingerprints`, `Pubkeys`, or `OriginPathOverrides`) yields a different Md1EncodingId.
- **`WalletPolicyId` (v0.13 §5.3)** is a canonical-expanded hash defined in the v0.13 spec. It belongs to v0.13's domain, not v0.11's. Its hash input is built per-`@N` from `(canonical_origin, canonical_use_site, fp?, xpub?)`, drawing values from `OriginPathOverrides`, `UseSitePathOverrides`, `Fingerprints`, and `Pubkeys` after canonical-fill. For current behavior, see `design/SPEC_v0_13_wallet_policy.md` §5.3.

### 8.1 WalletDescriptorTemplateId

```
WalletDescriptorTemplateId = SHA-256(canonical_template_bits)[0..16]
```

`canonical_template_bits` is the bit stream consisting of the following segments, **concatenated in wire order**:

1. Use-site-path-decl bits (§3.5).
2. Tree bits (§3.6 + §6).
3. Bits of every TLV entry classified as "in WDT-Id hash domain" per §3.7.1, in the same order they appear in the wire form (i.e., ascending tag-value order).

`canonical_template_bits` EXCLUDES:
- The header (§3.3).
- The origin-path-decl (§3.4) — origin paths are part of BIP 388's key information vector, not the template.
- TLV entries classified as "not in WDT-Id hash domain" per §3.7.1. In v0.11 base spec, this means **`Fingerprints` (TLV tag 0x01) is excluded** from the WDT-Id hash. Future TLV tags are individually classified at specification time.
- HRP, BCH checksum, chunk headers.

For v0.11 specifically, since `UseSitePathOverrides` is the only structural TLV included in the hash domain, `canonical_template_bits` is:

```
use-site-path-decl bits || tree bits || [if UseSitePathOverrides TLV present:
  [tlv-tag: 5 bits = 0x00] || [length: LP4-ext varint] || [overrides payload bits]
]
```

The bits are concatenated in this order, packed into bytes MSB-first per §4.6. SHA-256 is computed over the resulting byte stream; the first 16 bytes are the `WalletDescriptorTemplateId`.

#### 8.1.1 Properties

- **Always computable** from any v0.11+ md1 string.
- **Shape-identifying.** Two engravings of the same logical wallet (same template structure + same use-site paths) yield the same WalletDescriptorTemplateId, regardless of differences in origin paths or fingerprints.
- **Stable across metadata changes.** Adding/removing a Fingerprints TLV does not change the WalletDescriptorTemplateId.

### 8.2 Md1EncodingId

```
Md1EncodingId = SHA-256(canonical_full_payload_bits)[0..16]
```

`canonical_full_payload_bits` is the entire payload bit stream:
- Header (§3.3).
- Origin-path-decl (§3.4).
- Use-site-path-decl (§3.5).
- Tree (§3.6 + §6).
- Entire TLV section (§3.7) including all entries.

Excludes only HRP, BCH checksum, and chunk headers.

#### 8.2.1 Properties

- **Always computable.**
- **Engraving-specific.** Differs across engravings of the same wallet that include different metadata (e.g., one with fingerprints, one without).
- **Basis for ChunkSetId** (§9.2): `ChunkSetId = Md1EncodingId[0..20]`.

### 8.3 WalletPolicyId

> **Superseded.** v0.11 sketched a `WalletPolicyId` to be defined when v0.12 shipped an `Xpubs` TLV. v0.12 did not ship that TLV; the canonical `WalletPolicyId` definition lives in `design/SPEC_v0_13_wallet_policy.md` §5.3 and is defined against v0.13's `Pubkeys` (TLV 0x02) and `OriginPathOverrides` (TLV 0x03). The v0.11 sketch below is preserved for historical context only.

```
WalletPolicyId = SHA-256(canonical_wallet_policy_bits)[0..16]
```

Defined only when concrete wallet keys are present in the encoding. The hash domain covers BIP 388 wallet-policy content: template + key-information-vector content (xpubs + origin paths + fingerprints).

For v0.11: not computable. v0.11 encodings cannot embed xpubs.

For current behavior: see `design/SPEC_v0_13_wallet_policy.md` §5.3 (canonical-expanded hash, stable across origin and use-site elision, presence-significant on fp/xpub).

### 8.4 Phrase rendering

All three IDs render to a 12-word phrase via standard BIP-39:

```
Phrase = BIP-39 mnemonic from 128-bit ID input
       (4-bit checksum appended → 132 bits → 12 × 11-bit words → English BIP-39 word list)
```

- **Word list:** English canonical (BIP-39 standard 2048-word English list).
- **Display:** single ASCII space between words, lowercase, no leading/trailing whitespace.
- **Determinism:** same ID always produces the same phrase.

#### 8.4.1 Phrase types

| Phrase | Source ID | Computable |
|---|---|---|
| `WalletDescriptorTemplateIdPhrase` | WalletDescriptorTemplateId | always |
| `Md1EncodingIdPhrase` | Md1EncodingId | always |
| `WalletPolicyIdPhrase` | WalletPolicyId | only when keys are present; canonical definition in v0.13 (`Pubkeys` TLV 0x02 — see `design/SPEC_v0_13_wallet_policy.md` §5.3) |

The default user-facing phrase is `WalletDescriptorTemplateIdPhrase` (shape-identifying, BIP-388-aligned semantic).

### 8.5 Naming canonicalization

No spec-curated dictionary in v0.11 base spec. Wallet UIs are free to recognize and label common shapes via their own conventions, but the canonical phrase rendering for the WalletDescriptorTemplateId is uniformly the BIP-39 12-word phrase.

---

## 9. Chunking

### 9.1 Activation threshold

Chunking is required when the encoding exceeds the codex32 regular-form single-string limit (per BIP 93). v0.11+ ships regular form only; long-form codex32 was dropped in v0.12.0. The data-symbol budget per single string is therefore the regular-form maximum (80-char data part: 3 HRP + 1 separator + 64 data symbols + 13 checksum, i.e. 64 × 5 = 320 payload bits). See the appendix entry F2 for the v0.12.1 hot-fix that corrected the threshold constant.

```
encoding ≤ codex32 regular limit (320 payload bits / 64 data symbols): single string
encoding > regular limit: chunked, multi-card
```

### 9.2 ChunkSetId

```
ChunkSetId = Md1EncodingId[0..20]
```

20-bit identifier that groups chunks of the same engraving for reassembly.

#### 9.2.1 ChunkSetIdSeed (override)

For deterministic test-vector generation, encoders MAY override the default ChunkSetId via an explicit seed. Carry-over from v0.10 (`EncodeOptions::chunk_set_id_seed`).

```
ChunkSetId = ChunkSetIdSeed[top 20 bits]
```

The override does NOT affect WalletDescriptorTemplateId, Md1EncodingId, or any other identifier — only the chunk-header field.

### 9.3 Chunk header (37 bits)

Per chunk, prepended to the chunk's payload before BCH-checksumming:

```
[version: 3 bits]                # matches main format version (must be 0 for v0.11)
[chunked: 1 bit]                  # 1 (this is a chunk in a chunked set)
[reserved: 1 bit]                 # matches D9 bit 3 reservation; must be 0
[chunk-set-id: 20 bits]           # Md1EncodingId[0..20]
[count: 6 bits]                   # 1..64 chunks total
[index: 6 bits]                   # 0..63, this chunk's position
```

Total: 37 bits (≈ 8 codex32 chars per chunk header overhead).

### 9.4 Single-string vs chunked

When the encoding fits in one codex32 string (no chunking needed), the wire form has no chunk header — just `HRP | payload | checksum`.

When chunking is in effect, each chunk is a self-contained codex32 string of the form:

```
md1 | chunk-header | chunk-payload | BCH-checksum
```

### 9.5 Chunk payload split

The full canonical payload (§3.2) is split into N chunks where N is chosen to fit each chunk (after adding the 37-bit chunk header) within the codex32 regular-form single-string limit (320 bits per chunk wire). The first chunk's chunk-payload contains the start of the canonical payload; subsequent chunks continue.

Spec phase TODO: define exact split algorithm (suggested: byte-boundary splits where possible, padded to a chunk-payload size that divides evenly; encoder picks N to minimize total chars).

### 9.6 Reassembly

Decoders reassemble by:

1. Sorting all received chunks by their `index` field.
2. Validating consistency: all chunks share `version`, `chunk-set-id`, `count`. Indices form 0..count-1 with no duplicates or gaps.
3. Concatenating chunk-payloads in order to reconstruct the canonical full payload.
4. Computing `Md1EncodingId` over the reassembled payload and verifying its first 20 bits equal the chunk-set-id (cross-chunk integrity).

If any check fails, reassembly fails with a specific error indicating which check.

### 9.7 Per-chunk integrity

Each chunk has its own codex32 BCH checksum. A user can typo-check an individual card without reassembling the full backup. The reassembled `Md1EncodingId` provides cross-chunk integrity at the reassembly step.

### 9.8 Maximum chunks

A chunk-set has at most 64 chunks (count field is 6 bits). For exotic scenarios that require >64 chunks (extreme multisig with many embedded xpubs and divergent paths), a future spec revision would extend the count field width or introduce hierarchical chunking. Deferred until concrete demand.

---

## 10. Engraving layout and display

### 10.1 HRP placement

Every codex32 string carries its own `md1` HRP at the front. For chunked backups, each card's chunk has its own HRP — required by per-chunk BCH integrity (HRP-mixing in BIP 93 polymod).

### 10.2 Visual separators

Within a single codex32 string:
- A `-` or space MAY appear between the payload and the BCH checksum (D11).
- A `-` or space MAY appear every 4-5 chars throughout, for human transcription aid.

Between chunks of a chunked backup:
- Physical card boundaries serve as separator.
- Digital displays MAY use newlines or `||` between chunks.

Decoders MUST tolerate stripped whitespace and `-` between any positions on input.

### 10.3 Multi-card layout (recommended, non-normative)

```
Card 1 of N:
  [optional] WalletDescriptorTemplateIdPhrase: <12 BIP-39 words>
  [optional] Md1EncodingId fingerprint: <8 hex chars>  (for disambiguation)
  Md1 chunk 1/N:
  md1qpz9-... <codex32 string with BCH checksum>

Card 2 of N:
  Md1 chunk 2/N:
  md1qpz9-... <codex32 string with BCH checksum>

...
```

The phrase and fingerprint (when included) appear on card 1; subsequent cards have only their chunks.

### 10.4 Phrase rendering alongside the md1 string

Engraving recommendations (non-normative):

- **Always engrave the md1 string(s)** — required for decoding.
- **Optionally engrave `WalletDescriptorTemplateIdPhrase`** — 12 BIP-39 words (~30-50 metal chars), shape verification anchor at restore.
- **Optionally engrave `Md1EncodingId` fingerprint** — 4 bytes / 8 hex chars (~8 metal chars), engraving disambiguation for users with multiple wallets sharing a shape.

UIs and CLIs SHOULD default to displaying all three at encode time so users can choose which to engrave.

### 10.5 Tooling tolerance (recap of BIP 173 + D11)

- Decoders MUST tolerate stripped whitespace and `-` between any positions.
- Decoders MUST tolerate uppercase/lowercase variation in the codex32 alphabet (BIP 173 standard).
- Decoders MUST reject characters outside the codex32 alphabet (BIP 173 standard).

---

## 11. Forward-compat and versioning

### 11.1 Version field

The 3-bit version field in the header (§3.3) provides 8 generations of major format versions. v0.11 = 0; future major reformations increment.

A major version bump is reserved for changes that require entirely new wire-format semantics (e.g., reallocating header bits, changing tag-space structure, switching primitive encodings). Pre-v1.0 sandbox: version stays at 0 throughout v0.x series.

### 11.2 Additive features (no version bump)

The following are purely additive and do not require a version bump:

- New TLV tags (forward-compat by D6: unknown tags skipped via length-prefix advancement).
- New primary or extension tag-space allocations within the existing tag space (forward-compat by tag-space reservation discipline).
- New phrase types or rendering conventions (display-layer; not in wire form).

### 11.3 Reserved bits and slots

| Reservation | Purpose |
|---|---|
| Header bit 3 | freed; reserved for future use |
| Primary tag 0x1F | extension prefix (locked) |
| Extension tag 0x05–0x1F | reserved for future fragments |
| TLV tag 0x02 | originally reserved for v0.12 Xpubs; reallocated by v0.13 to `Pubkeys` (see `design/SPEC_v0_13_wallet_policy.md` §3.2) |
| TLV tag 0x03 | allocated by v0.13 to `OriginPathOverrides` |
| TLV tag 0x04–0x0F | reserved structural TLVs |
| TLV tag 0x10–0x1F | reserved metadata TLVs |
| Path-component depth > 15 | reserved for future TLV-based extension |
| Multipath alt-count > 9 | reserved for future ExtendedMultipath TLV |
| Chunk count > 64 | reserved for future spec rev (hierarchical chunking) |

### 11.4 v0.11 → v0.12 → v0.13 transition

v0.11 anticipated a v0.12 `Xpubs` TLV at tag 0x02. v0.12 instead shipped a cleanup release (v0.12.0 stripped v0.x and flattened the v11 module surface; v0.12.1 hot-fixed three audit findings — see appendix). The wallet-policy-with-keys feature shipped in v0.13, which allocated tag 0x02 to `Pubkeys` and tag 0x03 to `OriginPathOverrides`. This is purely additive — no v0.11 wire-format change. v0.11 decoders skip the unknown TLVs gracefully (recovering template + paths only); v0.13 decoders interpret the TLVs and reconstruct the full wallet policy with embedded xpubs and per-`@N` origin overrides. See `design/SPEC_v0_13_wallet_policy.md` for the current spec.

### 11.5 Validation strictness

Decoders MUST validate all invariants in §7 at decode time. Strict validation prevents malformed inputs from yielding ambiguous templates. Implementations MAY offer a "lax" mode for forensic / debugging use cases, but MUST default to strict.

---

## 12. Test vectors

To be defined during implementation phase. Test vectors will cover:

- Each top-level constructor (Pkh, Sh, Wpkh, Wsh, Tr) in single-sig and multisig forms.
- Each multisig variant (Multi, SortedMulti, MultiA, SortedMultiA) at varying k/n.
- Each wrapper (Alt, Swap, Check, DupIf, Verify, NonZero, ZeroNotEqual) at the leading position of a chained example.
- Each logical operator (AndV, AndB, AndOr, OrB, OrC, OrD, OrI, Thresh).
- Timelocks (After, Older) at boundary values (0, 1, 500_000_000-1, 500_000_000, 2³¹-1).
- Hashes (Sha256, Hash256, Ripemd160, Hash160, RawPkH).
- Constants (False, True) — extension-space encoding.
- Path-decl shared and divergent modes.
- Use-site path encoding for `<0;1>/*`, `*`, `*'`, `<0;1>/*'`, `<0;1;2>/*`, custom variants.
- Multi-multipath consistency (shared default multipath alt-count = override multipath alt-count).
- TLV section with UseSitePathOverrides and Fingerprints, individually and combined.
- Validation rejections: every invariant in §7.
- Chunking: 1-chunk, 2-chunk, max-chunk (64) cases; ChunkSetId derivation; reassembly.
- Identity: WalletDescriptorTemplateId stability across metadata changes; Md1EncodingId differentiation.

Test vectors will be machine-generated against the reference implementation and persisted as canonical fixtures.

---

## 13. Implementation guidance

### 13.1 Bit-stream library

A bit-aligned encoder/decoder library is foundational. Design notes:

- Read/write at bit granularity (no byte-padding mid-payload).
- Track current bit position; support "peek N bits" and "consume N bits."
- Cleanly handle the boundary into TLV section (where bits remaining = total - position so far).
- Pack codex32 5-bit symbols MSB-first into the bit stream.

### 13.2 Decoder structure (recommended)

```
1. Strip HRP, validate it equals "md1".
2. Parse codex32 5-bit symbols, validate BCH checksum.
3. Examine bit 3 of the first 5-bit symbol (the symbol immediately after HRP, before any payload parsing):
     - bit 3 = 0: this is a payload header (single-string mode).
                  Goto step 4 with the same symbol stream.
     - bit 3 = 1: this is a chunk header (chunked mode).
                  Parse chunk header (§9.3), buffer the chunk-payload, await
                  remaining chunks, then reassemble (§9.6) and recurse with
                  the reassembled bit stream from step 4.
4. Parse payload header (5 bits, §3.3). Validate bits 0-2 match a known version,
   bit 3 = 0, bit 4 ∈ {0, 1}.
5. Parse origin-path-decl (§3.4, dispatched on bit 4).
6. Parse use-site-path-decl (§3.5).
7. Parse tree (recursive, §6).
8. Parse TLV section until bits exhausted (validating invariants from §7.5).
9. Run all validation invariants from §7.
10. Compute IDs as needed (§8).
```

The dispatch in step 3 works because v0.11 reserves payload-header bit 3 = 0 (§3.3), while chunk-header bit 3 is the `chunked` flag set to 1 (§9.3). Encoders never produce a payload header with bit 3 = 1, so seeing bit 3 = 1 unambiguously indicates chunked mode.

### 13.3 Encoder structure (recommended)

Inverse of the decoder, with canonicalization:
- Sort UseSitePathOverrides and Fingerprints entries by ascending `@N` index.
- Sort TLV tags ascending.
- Emit a single regular-form codex32 string when the encoding fits the single-string limit; chunk if not. (Long-form codex32 was dropped in v0.12.0.)

### 13.4 Validation strategy

Encoder rejects malformed inputs at encode time (preventing bad backups). Decoder validates everything at decode time (preventing bad inputs from yielding ambiguous templates).

### 13.5 Test discipline

Per-phase TDD per project conventions (`crates/md-codec/CLAUDE.md`-style). Tests written before implementation; per-phase opus reviewer gate workflow mirrors v0.10's playbook.

---

## 14. Appendix A — Common-case wire-cost worked examples

### 14.1 BIP 84 single-sig

Template: `wpkh(@0/<0;1>/*)`. Origin path: m/84'/0'/0'.

| Field | Bits |
|---|---|
| Header (version=0, reserved=0, all-keys-same-path=0) | 5 |
| n=1 | 5 |
| origin-path-encoding: depth=3, 84' (1+11), 0' (1+4), 0' (1+4) | 4+12+5+5 = 26 |
| use-site-path-decl: has-mp=1, count=2, alt 0 (1+4), alt 1 (1+5), wildcard=0 | 1+3+5+6+1 = 16 |
| Tree: Wpkh tag (5) + key-arg (n=1 ⇒ 0 bits) | 5 |
| TLV section: empty | 0 |
| **Total payload** | **57 bits ≈ 12 codex32 chars** |
| Codex32 BCH checksum (regular) | 13 chars |
| HRP `md1` | 3 chars + 1 separator = 4 chars |
| **Total engraved length** | **~29 codex32 chars** |

### 14.2 2-of-3 BIP 48 sortedmulti (shared origin path)

Template: `wsh(sortedmulti(2, @0/<0;1>/*, @1/<0;1>/*, @2/<0;1>/*))`. Origin path: m/48'/0'/0'/2'.

| Field | Bits |
|---|---|
| Header (all-keys-same-path=0) | 5 |
| n=3 | 5 |
| origin-path-encoding: depth=4, 48' (1+10), 0' (1+4), 0' (1+4), 2' (1+6) | 4+11+5+5+7 = 32 |
| use-site-path-decl: same as §14.1 = 16 | 16 |
| Tree: Wsh tag (5), SortedMulti tag (5), k=2 (5), n=3 (5), 3× key-arg (n=3 ⇒ 2 bits each) | 5+5+5+5+6 = 26 |
| TLV section: empty | 0 |
| **Total payload** | **84 bits ≈ 17 codex32 chars** |
| Codex32 BCH (regular) | 13 chars |
| HRP | 4 chars |
| **Total engraved length** | **~34 codex32 chars** |

### 14.3 BIP 86 taproot single-sig

Template: `tr(@0/<0;1>/*)`. Origin path: m/86'/0'/0'.

| Field | Bits |
|---|---|
| Header | 5 |
| n=1 | 5 |
| origin-path: depth=3, 86' (1+11), 0' (1+4), 0' (1+4) | 4+12+5+5 = 26 |
| use-site-path: 16 | 16 |
| Tree: Tr tag (5), key-arg (0 bits), has-tree=0 (1) | 5+0+1 = 6 |
| TLV: empty | 0 |
| **Total payload** | **58 bits ≈ 12 codex32 chars** |
| BCH | 13 chars |
| HRP | 4 chars |
| **Total** | **~29 chars** |

---

## 15. Appendix B — Decision log cross-reference

This spec implements decisions D1–D37 from `design/BRAINSTORM_v0_11_wire_format.md`, with revoked decisions (D24, D31, D32, D35, D19, D26) replaced by their successor decisions (D31′, D32′, D35′, D19′, D26′).

For rejected design alternatives and conditions for future reconsideration, see `design/REJECTEDFORNOW.md`.

---

## 16. Open items for implementation phase

The following items are flagged for resolution during early implementation. None block spec acceptance.

1. **Exact codex32 regular-form character limits** (per BIP 93). Spec text in §9.1 cites BIP 93 limits abstractly; implementation must verify and document the precise thresholds (regular max, chunk-payload max post-chunk-header overhead) against BIP 93 normative parameters. (Long-form codex32 was dropped in v0.12.0; v0.11+ uses regular form only.)
2. **Chunk payload split algorithm.** §9.5 leaves the exact split TODO. Implementation phase picks an algorithm (suggested: byte-boundary splits where possible; padded to chunk-payload bit-counts that pack evenly into codex32 5-bit symbols; encoder picks N to minimize total chars). The choice does not affect interop as long as encoder and decoder agree (the canonical concatenation of chunk-payloads in index order reconstructs the full payload).
3. **LP4-ext varint extension semantics for very large values.** §4.1 covers a single extension hop (covers up to ~2²⁸). Recursive extension for >2²⁸ is undefined in v0.11 and not needed for path-component values (BIP 32 limits child indices to u31, comfortably within single-extension reach). Implementation rejects values requiring a second extension hop in v0.11; spec revision adds recursive semantics if a future use case needs >2²⁸ varint values.
4. **Concrete polynomial constant value** for `MD_REGULAR_CONST`. Cited symbolically in §2.2; implementation text should include the hexadecimal value derived from the top 65 bits of `SHA-256("shibbolethnums")`. Present in shipped source at `crates/md-codec/src/bch.rs`. (`MD_LONG_CONST` was removed when long-form codex32 was dropped in v0.12.0.)
5. **Test-vector generation tooling.** Implementation phase defines a deterministic test-vector generator and persists canonical fixtures covering §12's enumerated coverage list. Test vectors include the NUMS xpub pattern for taproot internal-key tests when applicable.

---

## Appendix: v0.12.1 hot-fixes

Three divergences between this v0.11 spec and the shipped reference implementation were caught by a post-v0.12.0 audit and shipped as a maintenance release (`md-codec-v0.12.1`, commit `b0efc6e`). Readers of the v0.11 spec MUST treat the corrected behavior described below as the normative-as-shipped wire-format reality. The audit-finding labels (F2, F3, F4) are preserved here for cross-reference.

### F2 — Chunk single-string payload bit limit (§9.1, §9.5)

The original v0.11 spec assumed a long-form codex32 single-string budget. v0.12.0 dropped long-form codex32; the legal data-symbol budget per single regular-form codex32 string is therefore 64 symbols (320 bits), not the 75-symbol / 375-bit long-form figure that earlier code constants (`SINGLE_STRING_PAYLOAD_BIT_LIMIT = 75 * 5 = 375`) had carried over.

Wire emissions in the 320..375 payload-bit band would have exceeded BIP 93 codex32 regular's 80-character limit. v0.12.1 corrected `SINGLE_STRING_PAYLOAD_BIT_LIMIT` to `64 * 5 = 320`. See `crates/md-codec/src/chunk.rs`.

### F3 — LP4-ext varint overflow handling (§4.1)

The original §4.1 left "values requiring more than 28 bits" unspecified beyond a "rare for path components" footnote. The shipped `varint::write_varint` initially expressed this as `assert!(l_high <= 15)`, which panicked for value ≥ 2²⁹ instead of returning a structured error.

v0.12.1 replaced the panic with `Err(Error::VarintOverflow)`, cascaded the `Result` return through `OriginPath::PathComponent::write`, `UseSitePath::Alternative::write`, and `TlvSection::write` call sites, and added boundary tests (max single-extension `2^29 − 1` succeeds; `2^29` returns the error).

### F4 — Placeholder index `< n` enforcement on decode (§7.4)

§7.4 states "Every `@N` index in the tree is < n," but the shipped decoder's enforcement was a `debug_assert!((*index as usize) < seen.len())` that silenced in release builds, after which the conditional update at the next line silently skipped the violation. For non-power-of-2 `n` values (3, 5, 6, 7, 9–15), an index `≥ n` but `< 2^key_index_width` slipped through.

v0.12.1 replaced the silent skip with explicit `Err(Error::PlaceholderIndexOutOfRange)` on both the `KeyArg` and `Tr::key_index` decode paths. Tests added cover `n=3`, `n=5`, `n=15`, and the `Tr` key-index path.

### Reference

- Tag: `md-codec-v0.12.1`
- Commit: `b0efc6e`
- Companion audit findings: F2, F3, F4 from the v0.13 pre-implementation audit (full report saved to plan-agent transcript).

---

## End of spec
