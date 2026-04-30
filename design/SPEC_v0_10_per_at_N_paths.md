# v0.10 Design Spec: Per-`@N` Origin Path Declaration

**Brainstormed:** 2026-04-29 via conversational walkthrough; all 13 design questions locked in `design/BRAINSTORM_v0_10_per_at_N_paths.md`.
**Status:** Draft for opus review.
**Closes FOLLOWUPS at v0.10.0 ship:** `md-per-at-N-path-tag-allocation`.
**Filed during brainstorm (carry-forward):** `v2-design-questions`, `walletinstanceid-rendering-parity`.

**Wire-format-breaking release** at the BIP 388 wallet-policy template level: header bit 3 was reserved-must-be-zero in v0.x ≤ 0.9; v0.10 reclaims it as the `OriginPaths` flag. Pre-v0.10 decoders cleanly reject v0.10 OriginPaths-using encodings via `Error::ReservedBitsSet`. Existing shared-path encodings remain byte-identical (header byte `0x00` or `0x04` unchanged).

This is the next breaking-change axis bump from v0.9.1, not the v0.x→v0.x+1 patch pattern.

---

## §1. Scope and Goals

### Goal

v0.10 of `md-codec` admits **per-placeholder origin paths** in BIP 388 wallet policies. v0.x ≤ 0.9 carried only `Tag::SharedPath = 0x34`, declaring a single origin path applicable to every `@N` placeholder. This was lossy for the common multisig case where cosigners derive xpubs from different paths (e.g., distinct BIP 48 accounts per cosigner). v0.10 closes the lossy-flatten bug by adding `Tag::OriginPaths = 0x36`, a new path-declaration variant carrying one origin path per `@N` in placeholder-index order.

### Decision matrix (locked during brainstorming)

| # | Knob | Choice | Rationale |
|---|---|---|---|
| Q1 | Tag byte allocation | `Tag::OriginPaths = 0x36` | Next clean slot after the existing framing cluster (`0x33`-`0x35`); leaves `0x37+` free; first-shipper wins (`Tag::RecoveryHints` slated for `0x37` in `design/POLICY_BACKUP.md`). |
| Q2 | Encoding shape | Dense, count-prefixed (`u8`); one path-decl per `@N`; no deduplication | Simplest decoder (read N entries, done); no fallback or sentinel coupling; no wire-format index/redundancy machinery. Wire-size delta vs sparse encoding is single-digit bytes. |
| Q3 | SharedPath coexistence | Strict mutual exclusion at the path-decl slot | One canonical form per policy state; no advisory-vs-authoritative ambiguity; round-trip stability. |
| Q4 | Header flag bit | Bit 3 = OriginPaths flag; symmetric with bit 2 = Fingerprints | Self-descriptive header; pre-v0.10 decoders correctly reject (reserved-bit semantics); decoder dispatches without peek. |
| Q5 | Authority precedence with mk1 | Pure cross-reference to mk1 BIP §"Authority precedence" / SPEC §5.1 | mk1 owns the normative cross-format semantics; md1 acknowledges and points; orchestrator owns consistency check. |
| Q6 | Interaction with `Tag::Fingerprints` | Independent blocks; each independently flagged | Fingerprints identify seeds (path-invariant); paths identify derivation choices (seed-invariant). Orthogonal data → independent blocks. |
| Q7 | PolicyId impact | Route X — per-`@N` paths included in canonical bytecode | Two policies with same script but different per-cosigner accounts are *different* wallet shapes; PolicyId distinguishes them correctly. (See §6 typology subsection for Type 0/Type 1 framing.) |
| Q8 | Path component count cap | `MAX_PATH_COMPONENTS = 10` applied uniformly to `Tag::SharedPath` and `Tag::OriginPaths` | Aligns with mk1 SPEC §3.5; defense-in-depth; no real-world BIP path approaches 10 components. |
| Q9 | Encoder default behavior | Auto-detect; emit `Tag::SharedPath` if all `@N` paths agree, `Tag::OriginPaths` otherwise | Lossless by default; v0.10 fixes v0.9's silent path-divergence drop. |
| Q10 | Migration story | Wire-additive at decoder; encoder lossless under default options | Existing shared-path encodings byte-identical; divergent-path policies get correct (different) PolicyIds. |
| Q11 | Forward-compatibility hooks | None beyond existing slack (header bits 0/1, ~200 unused tag bytes, `#[non_exhaustive]`) | YAGNI; no speculative tag-byte preallocation. |
| Q12 | PolicyId typology | Light formalization — BIP teaching subsection for Type 0 (`WalletInstanceId`) / Type 1 (`PolicyId`); no code rename | Naming carries the type; new framing aids reader comprehension. |
| Q13 | PolicyId UX | BIP softens 12-word phrase to MAY-engrave; add `PolicyId::fingerprint() → [u8; 4]`; canonical PolicyId stays 128 bits / 12 BIP-39 words | The 12-word phrase is a Tier-3 anchor, optional for typical users; fingerprint API gives an 8-char short identifier for tools. |

### Non-goals (out of scope for v0.10)

- **BIP 393 recovery hints** (`Tag::RecoveryHints` at `0x37`): birthday, gap-limit, max-silent-payment-label-index. Slated for v1+ per `design/POLICY_BACKUP.md`. Header bit 1 likely reserved for the gating flag.
- **`WalletInstanceId::to_words()` BIP-39 rendering parity.** Filed as `walletinstanceid-rendering-parity` FOLLOWUPS; v1+.
- **Code-level rename** of `PolicyId` / `WalletInstanceId` to typed names (`Type1PolicyId` / `Type0PolicyId`). Q12 chose Light; existing names are descriptive.
- **PolicyId nonce / cryptographic instance distinguishing.** Skipped during brainstorm (per-customer or per-instance distinguishing handled out-of-band via labels or per-customer seeds).
- **Path-deduplication encoding** for `OriginPaths`. Q2-A locked dense; revisit at v2 (see `v2-design-questions` FOLLOWUPS).
- **Tag space rearrangement.** Captured in `v2-design-questions`; not a v0.10 concern.

### What v0.10 ships

1. Header bit 3 reclaimed → OriginPaths flag (wire-format break for the bit; backward-compat for shared-path encodings since bit stays 0).
2. New `Tag::OriginPaths = 0x36` block in canonical bytecode.
3. `MAX_PATH_COMPONENTS = 10` enforced at both `Tag::SharedPath` and `Tag::OriginPaths`.
4. Encoder auto-detects path divergence and selects between `SharedPath` and `OriginPaths` accordingly.
5. New error machinery (split between bytecode-layer `BytecodeErrorKind::*` and policy-layer `Error::*` per the v0.6 strip-Layer-3 convention):
   - `BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 }` — bytecode-layer structural error: the count byte exceeds the BIP 388 placeholder cap.
   - `BytecodeErrorKind::UnexpectedTag { expected, got }` — already exists; reused for header-bit-vs-tag conflicts (Q3-A "0x36 with bit 3 clear" or "0x34 with bit 3 set" both surface here with `expected: 0x36 or 0x34` per the bit, `got: <other>`).
   - `Error::OriginPathsCountMismatch { expected, got }` — policy-layer semantic error: bytecode count doesn't match the tree's placeholder count after parse.
   - `Error::PathComponentCountExceeded { got, max: 10 }` — applies to both `Tag::SharedPath` and `Tag::OriginPaths` when an explicit-form path-decl declares too many components.
6. `PolicyId::fingerprint() -> [u8; 4]` API (top 32 bits as a short identifier, parallel to BIP 32 master-key fingerprints).
7. BIP draft updates: §"Path declaration" extended with §"Per-`@N` path declaration"; new §"PolicyId types" teaching subsection; §"Engraving the 12-word PolicyId phrase" softened to MAY-engrave.
8. mk1 BIP cross-reference: new §"Authority precedence with MK" subsection in md1's BIP pointing to mk1's normative wording.
9. Test vectors regenerate (both schemas) under family token `"md-codec 0.10"`. New positive vector exercising `Tag::OriginPaths`. New negative vectors for the new error variants.

---

## §2. Wire Format

### Header

The bytecode header byte gains bit 3 as the OriginPaths flag, parallel to bit 2 (Fingerprints):

```
Bits 7–4: version (0x0)
Bit 3:    OriginPaths flag (0x08)  ← NEW in v0.10
Bit 2:    Fingerprints flag (0x04)
Bits 1–0: reserved (must be 0)
```

`RESERVED_MASK` updates from `0x0B` (bits 3, 1, 0) to `0x03` (bits 1, 0 only).

Valid v0.10 header bytes are exactly: `0x00`, `0x04`, `0x08`, `0x0C`. Any other value is rejected:
- `version != 0` → `Error::UnsupportedVersion(nibble)`
- Reserved bits set → `Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::ReservedBitsSet { byte, mask: 0x03 } }`

### Bytecode layout

```
[header] [path-declaration] [Tag::Fingerprints + count + 4*N bytes]? [tree bytes]
```

The path-declaration slot holds **exactly one** of:

```
Tag::SharedPath (0x34) | indicator | [explicit-bytes]?       ← header bit 3 = 0
Tag::OriginPaths (0x36) | count: u8 | path_decl_0 | ... | path_decl_{N-1}   ← header bit 3 = 1
```

Strict mutual exclusion: header bit 3 dispatches the path-decl tag. Encountering `Tag::OriginPaths (0x36)` when bit 3 = 0 (or `Tag::SharedPath (0x34)` when bit 3 = 1) surfaces as `Error::InvalidBytecode { offset: 1, kind: BytecodeErrorKind::UnexpectedTag { expected, got } }`, where `expected` is the tag the header bit predicted (`0x34` if bit 3 clear, `0x36` if set). See §3 "Path-decl dispatch" for the full match logic.

Order is fixed: path-declaration MUST precede the optional fingerprints block, which MUST precede the tree bytes. This matches the v0.x convention.

### `Tag::OriginPaths` block

```
0x36 | count: u8 | path_decl_0 | path_decl_1 | ... | path_decl_{N-1}
```

- **`0x36`** — `Tag::OriginPaths` tag byte.
- **`count: u8`** — number of path declarations, MUST be in `1..=32` and MUST equal the placeholder count derivable from the tree (i.e., `max(@i) + 1` over all `@i` references). Two distinct rejection paths:
  - **Structural** (bytecode layer, before tree-walk): `count == 0` or `count > 32` rejects with `BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 }` — independent of the tree, applied during `decode_origin_paths`.
  - **Semantic** (policy-construction layer, after tree-walk): `count != tree_placeholder_count` rejects with `Error::OriginPathsCountMismatch { expected, got }`.
- **`path_decl_i`** — origin path for placeholder `@i`, encoded using the existing single-path format defined in `crates/md-codec/src/bytecode/path.rs`:
  - Single byte for dictionary-form paths: one of `0x01`–`0x07`, `0x11`–`0x17` (per the path dictionary table in BIP §"Path dictionary").
  - `0xFE` followed by `LEB128(component_count)` followed by per-component `2*index + hardened_bit` bytes for explicit paths. `component_count` MUST be ≤ `MAX_PATH_COMPONENTS = 10`.

Path-decls appear in placeholder-index order: `path_decl_0` is `@0`'s path, `path_decl_1` is `@1`'s, etc. There is no per-entry index — the position in the list IS the index.

**No deduplication.** Path-decls are written densely, even if multiple placeholders share the same path. This optimizes for the common case of dictionary-form paths (1 byte each) and sidesteps the indirection complexity of a per-entry path-table reference. (See `v2-design-questions` FOLLOWUPS item 2 for the deferred deduplication design discussion.)

### `Tag::SharedPath` block (unchanged from v0.x)

```
0x34 | indicator | [explicit-bytes]?
```

Same encoding as v0.x. The `MAX_PATH_COMPONENTS = 10` cap applies here too (Q8): explicit-form paths with `component_count > 10` are rejected. Dictionary-form encodings (single-byte indicator) are unaffected.

### Path component count cap

`MAX_PATH_COMPONENTS: usize = 10`, defined as a public constant in `crates/md-codec/src/bytecode/path.rs`. Applies to both `Tag::SharedPath` and `Tag::OriginPaths` decoded path-decls.

Decoder rejects with `Error::PathComponentCountExceeded { got, max: 10 }` when a decoded explicit-path declares more than 10 components. Encoder rejects symmetrically before serialization.

The cap aligns with mk1 SPEC §3.5 (mk1 closure decision Q-3). No real-world BIP path family exceeds 6 components (BIP 48 + change/index = 6); 10 leaves generous headroom.

### Wire-format examples

**Example A — 3-cosigner shared-path multisig (no fingerprints).** Policy: `wsh(sortedmulti(2, @0/**, @1/**, @2/**))`, all keys on `m/48'/0'/0'/2'` (dictionary `0x05`). v0.10 byte-identical to v0.9:

```
00                               ← header: bit 3 clear, bit 2 clear
| 34 05                          ← Tag::SharedPath, indicator 0x05
| <wsh sortedmulti 2 @0 @1 @2>   ← tree bytes
```

**Example B — 3-cosigner divergent-path multisig with fingerprints.** Same policy, but cosigner C has explicit path `m/48'/0'/0'/100'` (not in dictionary), with all 3 fingerprints present.

Explicit-path byte derivation for `m/48'/0'/0'/100'`:
- Component count = 4 → LEB128 `0x04`
- `48'` hardened → `2*48 + 1 = 97 = 0x61`
- `0'` hardened → `2*0 + 1 = 1 = 0x01`
- `0'` hardened → `0x01`
- `100'` hardened → `2*100 + 1 = 201`. 201 has bit 7 set, so LEB128 = `0xC9, 0x01` (two bytes — high-bit continuation).
- Full explicit path bytes: `FE 04 61 01 01 C9 01` (7 bytes including the `0xFE` indicator).

```
0C                                                                ← header: bits 2 + 3 set
| 36 03 05 05 FE 04 61 01 01 C9 01                                ← Tag::OriginPaths: count=3, paths={0x05, 0x05, explicit-7B}
| 35 03 deadbeef cafebabe d00df00d                                ← Tag::Fingerprints: count=3, 4 bytes each
| <wsh sortedmulti 2 @0 @1 @2>                                    ← tree bytes
```

Decoder reads header byte `0x0C` → bit 3 set + bit 2 set. Dispatches: read `Tag::OriginPaths` block, then `Tag::Fingerprints` block, then tree.

OriginPaths block size: 11 bytes (1 tag + 1 count + 1 + 1 + 7 explicit). Fingerprints block size: 14 bytes (1 + 1 + 12). Header: 1 byte. Total prefix before tree: 26 bytes.

**Example C — 3-cosigner divergent-path, no fingerprints.** Same as B but without the fingerprints block:

```
08                                                                ← header: bit 3 set, bit 2 clear
| 36 03 05 05 FE 04 61 01 01 C9 01                                ← Tag::OriginPaths
| <wsh sortedmulti 2 @0 @1 @2>                                    ← tree bytes
```

Header byte `0x08` was previously a `ReservedBitsSet` violation in v0.x ≤ 0.9; now it's a valid v0.10 header.

---

## §3. Decoder Design

### Header parse

`BytecodeHeader::from_byte(b)` updates:

```rust
const RESERVED_MASK: u8 = 0x03;       // was 0x0B in v0.9
const FINGERPRINTS_BIT: u8 = 0x04;
const ORIGIN_PATHS_BIT: u8 = 0x08;    // NEW

pub struct BytecodeHeader {
    version: u8,
    fingerprints: bool,
    origin_paths: bool,                // NEW
}
```

Decoder reads the byte, validates `version == 0`, validates `(b & RESERVED_MASK) == 0`, then extracts the two flag bits.

### Path-decl dispatch

After header parse, decoder reads byte at offset 1:

```rust
let expected = if header.origin_paths() { 0x36 } else { 0x34 };
match (header.origin_paths(), tag_byte) {
    (false, 0x34) => decode_shared_path(...),
    (true,  0x36) => decode_origin_paths(...),
    (_,     other) => Err(Error::InvalidBytecode {
        offset: 1,
        kind: BytecodeErrorKind::UnexpectedTag { expected, got: other },
    }),
}
```

`UnexpectedTag` is reused for both arbitrary unknown bytes (a stray `0x00`) and for the structural "header-bit-vs-tag conflict" case (Q3-A: bit 3 clear but tag is `0x36`, or bit 3 set but tag is `0x34`). The diagnostic reports the tag the header bit predicted versus the byte actually seen — sharper than a generic "header-bit-and-tag-disagree" variant. (Per F5: avoid introducing a redundant peer top-level `Error::ConflictingPathDeclarations` variant when `BytecodeErrorKind::UnexpectedTag` already covers this.)

### `decode_origin_paths`

```rust
fn decode_origin_paths(cursor: &mut Cursor) -> Result<Vec<DerivationPath>, Error> {
    let count = cursor.read_u8()?;
    if count == 0 || count > 32 {
        // Bytecode-layer structural error — the count byte alone is malformed,
        // independent of the tree (BIP 388 caps placeholders at 32 and requires ≥1).
        return Err(Error::InvalidBytecode {
            offset: cursor.offset() - 1,
            kind: BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 },
        });
    }
    let mut paths = Vec::with_capacity(count as usize);
    for _ in 0..count {
        paths.push(decode_path(cursor)?);
        // decode_path enforces MAX_PATH_COMPONENTS = 10 internally and surfaces
        // BytecodeErrorKind::UnexpectedEnd on cursor exhaustion.
    }
    Ok(paths)
}
```

The structural count check (`count == 0 || count > 32`) is a bytecode-layer concern surfacing as `BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 }`. The semantic count check (`count == tree_placeholder_count`) is a policy-layer concern surfacing later as `Error::OriginPathsCountMismatch { expected, got }` — applied after tree-walk yields the actual placeholder count. Keeping these split aligns with the v0.6 strip-Layer-3 convention.

Cursor exhaustion mid-list (e.g., `count=3` with only 2 path-decls' worth of bytes before the tree) surfaces as `BytecodeErrorKind::UnexpectedEnd` from the inner `decode_path` call — consistent with `decode_path` / `decode_declaration` existing convention.

### Behavior on unknown tag at path-decl slot

If header bit 3 = 0 and offset-1 byte is anything other than `0x34`:

```
Error::InvalidBytecode {
    offset: 1,
    kind: BytecodeErrorKind::UnexpectedTag { expected: 0x34, got: byte },
}
```

If header bit 3 = 1 and offset-1 byte is anything other than `0x36`: same shape with `expected: 0x36`.

### Backwards-compat behavior

A v0.10 decoder MUST accept any v0.x ≤ 0.9 SharedPath-only encoding without behavior change. Specifically:

- Header byte `0x00` → no fingerprints, no origin paths → SharedPath at offset 1 (or rejected as `UnexpectedTag` if not 0x34).
- Header byte `0x04` → fingerprints, no origin paths → SharedPath at offset 1 + Fingerprints block following.

Pre-v0.10 decoders confronted with v0.10 OriginPaths-using encodings (header bit 3 set) reject cleanly via `Error::ReservedBitsSet`. This is the intended v0.x ≤ 0.9 ↔ v0.10 forward-compat behavior; no special handling required on either side.

---

## §4. Encoder Design + Type/Error Updates

### Round-trip stability — `decoded_origin_paths` field

v0.10's `WalletPolicy` gains a new field parallel to the existing `decoded_shared_path`:

```rust
pub struct WalletPolicy {
    // ... existing fields ...
    decoded_shared_path: Option<DerivationPath>,
    decoded_origin_paths: Option<Vec<DerivationPath>>,    // NEW in v0.10
}
```

Populated by `from_bytecode` when the source bytecode used `Tag::OriginPaths`. The encoder's source-of-truth precedence chain consults this field as Tier 1, parallel to the existing `decoded_shared_path` Tier 1. Without it, a `decode → encode` round-trip would lose per-`@N` path divergence and re-emit `Tag::SharedPath`, breaking byte-stability.

**Invariant:** at most one of `decoded_shared_path` and `decoded_origin_paths` is `Some`. `from_bytecode` populates exactly one, based on which path-decl tag was present on the wire (per Q3-A strict mutual exclusion). Documented on the field rustdoc; defense-in-depth check in `to_bytecode`.

### Encoder per-`@N`-path precedence chain

`placeholder_paths_in_index_order(&self, opts: &EncodeOptions) -> Result<Vec<DerivationPath>, Error>` returns the per-`@N` path in placeholder-index order, consulting (in order):

- **Tier 0 — `opts.origin_paths` override.** If `EncodeOptions::origin_paths: Some(Vec<DerivationPath>)`, use that. Tier 0 takes absolute precedence and is intended for deterministic test-vector generation. (No production code path uses this.)
- **Tier 1 — `decoded_origin_paths`** (new). If `from_bytecode` decoded a `Tag::OriginPaths` block, the per-`@N` paths it observed sit here. Round-trip stability source.
- **Tier 2 — Walk key information vector.** For each placeholder `@i` from `i=0` to `key_count-1`, extract the origin path from the corresponding xpub key in the KIV. Used when the policy was constructed from a concrete-key descriptor (e.g., parsed from a string).
- **Tier 3 — Fall through to single-shared-path tier chain** (existing v0.x logic). When the policy has no per-`@N` divergence information available, every placeholder gets the single shared path. This produces a `SharedPath` encoding by Q9-A's auto-detect rule.

The encoder dispatch then auto-detects whether all returned paths agree (→ emit `SharedPath`) or diverge (→ emit `OriginPaths`).

### Encoder dispatch

`WalletPolicy::to_bytecode(&self, opts: &EncodeOptions) -> Result<Vec<u8>, Error>` gains a path-divergence check:

```rust
let placeholder_paths: Vec<DerivationPath> = self.placeholder_paths_in_index_order()?;
let all_share = placeholder_paths.windows(2).all(|w| w[0] == w[1]);
let header = BytecodeHeader::new_v0(opts.fingerprints.is_some(), !all_share);

let mut out = Vec::new();
out.push(header.as_byte());

if all_share {
    out.extend_from_slice(&encode_declaration(&placeholder_paths[0]));
} else {
    out.push(Tag::OriginPaths.as_byte());
    // BIP 388 caps placeholder count at 32 upstream of to_bytecode (validated
    // when the WalletPolicy is constructed); the cast is infallible. expect()
    // documents the upstream invariant rather than silently masking a real bug.
    let count_u8 = u8::try_from(placeholder_paths.len())
        .expect("BIP 388 caps placeholder count at 32; upstream validation guarantees this");
    out.push(count_u8);
    for path in &placeholder_paths {
        out.extend_from_slice(&encode_path(path)?);
        // encode_path enforces MAX_PATH_COMPONENTS = 10 and surfaces
        // Error::PathComponentCountExceeded on violation (caught earlier
        // at policy construction in well-formed cases).
    }
}

if let Some(fps) = &opts.fingerprints {
    // existing fingerprints block emission, unchanged
}

out.extend_from_slice(&tree_bytes);
Ok(out)
```

`placeholder_paths_in_index_order` is a new helper that returns the per-`@N` path in placeholder-index order. Implementation detail: for a `WalletPolicy`, this walks the key information vector and extracts the origin path for each placeholder. If two placeholders refer to the same key (BIP 388 doesn't permit this in its current form, but defense-in-depth), the path is the same for both.

Each path is validated for `component_count <= MAX_PATH_COMPONENTS` before emission; encoder rejects with `Error::PathComponentCountExceeded` rather than emit a non-decodable byte sequence.

### Type updates

```rust
// crates/md-codec/src/bytecode/header.rs
pub struct BytecodeHeader {
    version: u8,
    fingerprints: bool,
    origin_paths: bool,    // NEW
}

impl BytecodeHeader {
    pub fn new_v0(fingerprints: bool, origin_paths: bool) -> Self { ... }   // signature change
    pub fn origin_paths(&self) -> bool { self.origin_paths }                 // NEW getter
}

// crates/md-codec/src/bytecode/tag.rs
pub enum Tag {
    // ... existing variants ...
    OriginPaths = 0x36,    // NEW
}

// crates/md-codec/src/bytecode/path.rs
pub const MAX_PATH_COMPONENTS: usize = 10;    // NEW
```

`BytecodeHeader::new_v0` signature change is a public-API break — callers must update from `new_v0(bool)` to `new_v0(bool, bool)`. Listed in MIGRATION.md for v0.9 → v0.10.

### Error updates

```rust
// crates/md-codec/src/error.rs

// Bytecode-layer structural errors (within BytecodeErrorKind enum):
pub enum BytecodeErrorKind {
    // ... existing variants ...

    /// The OriginPaths count byte is structurally invalid (zero or exceeds
    /// the BIP 388 placeholder cap of 32). Independent of tree placeholder
    /// count (which is checked separately at policy construction).
    OriginPathsCountTooLarge { count: u8, max: u8 },
}

// Policy-layer semantic errors (top-level Error enum):
pub enum Error {
    // ... existing variants ...

    /// The OriginPaths bytecode count doesn't match the tree's actual
    /// placeholder count after parse. Bytecode passed structural validation
    /// (count is 1..=32) but is inconsistent with the tree.
    #[error("OriginPaths count mismatch: tree has {expected} placeholders, OriginPaths declares {got}")]
    OriginPathsCountMismatch { expected: usize, got: usize },

    /// An explicit-form path declaration exceeded `MAX_PATH_COMPONENTS = 10`.
    /// Applies to both `Tag::SharedPath` and `Tag::OriginPaths`.
    #[error("path component count {got} exceeds maximum {max}")]
    PathComponentCountExceeded { got: usize, max: usize },
}
```

(The peer top-level `Error::ConflictingPathDeclarations` variant from the v1 spec draft is dropped per F5: `BytecodeErrorKind::UnexpectedTag` already covers the header-bit-vs-tag conflict case.)

`Error` is `#[non_exhaustive]`; adding variants is API-additive, not breaking.

### `EncodeOptions` updates

No new fields required. Q9-A locks auto-detect; the encoder examines the policy and selects the path-decl variant. No `with_per_at_n_paths(true)` opt-in toggle.

### `PolicyId::fingerprint()`

New helper added to `crates/md-codec/src/policy_id.rs`:

```rust
impl PolicyId {
    /// Return the first 32 bits of this PolicyId as a 4-byte array, parallel
    /// to BIP 32 master-key fingerprints. Suitable as a short identifier in
    /// CLI output, log lines, or as a minimal-cost engraving anchor.
    pub fn fingerprint(&self) -> [u8; 4] {
        let mut fp = [0u8; 4];
        fp.copy_from_slice(&self.0[0..4]);
        fp
    }
}
```

Pure additive API. CLI may render as `0x{fp[0]:02x}{fp[1]:02x}{fp[2]:02x}{fp[3]:02x}` (8 hex chars). No new error path; infallible.

### Encoder canonical-form determinism

Property: for a given `WalletPolicy` and `EncodeOptions`, the canonical bytecode is uniquely determined.

The encoder rule "all-`@N` agree → SharedPath; otherwise → OriginPaths" is a pure function of policy state. Round-trip tests assert `encode(decode(encode(p))) == encode(p)` byte-identically for any policy `p`.

Implication for PolicyId stability: same policy under v0.10 always hashes to the same PolicyId. v0.10 PolicyIds for *divergent-path* policies differ from v0.9's lossily-flattened encoding of the same policy, because the v0.9 canonical bytecode dropped path divergence info.

---

## §5. Test Corpus + Hostile-Input Fixtures

### Positive vectors

`crates/md-codec/src/vectors.rs` adds at least one new schema-2 positive vector exercising `Tag::OriginPaths`:

- **`o1_sortedmulti_2of3_divergent_paths`** — `wsh(sortedmulti(2, @0/**, @1/**, @2/**))` where `@0` and `@1` use shared `m/48'/0'/0'/2'` and `@2` uses `m/48'/0'/0'/100'` (explicit, not in dictionary). Built via `EncodeOptions::default()` (no fingerprints; auto-detect engages OriginPaths). Validates the bit-3 header set + dense per-`@N` encoding + explicit path embedded.

Optionally (not strictly required for v0.10):

- **`o2_sortedmulti_2of3_divergent_paths_with_fingerprints`** — same shape as o1 but with all 3 fingerprints, exercising header `0x0C` (both flags) and the OriginPaths-then-Fingerprints block ordering.

- **`o3_wsh_sortedmulti_2of4_divergent_paths`** — `wsh(sortedmulti(2, @0/**, @1/**, @2/**, @3/**))` with paths e.g. `{0x05, 0x05, 0x06, 0x07}` to exercise count=4 boundary in OriginPaths and verify multi-distinct-dictionary-form encoding. (Reshaped per F12 — `pkh()` is a single-key descriptor with exactly one `@N`, so a "4-`@N` pkh policy" was a category error in the spec draft.)

The existing v0.9 corpus vectors (M1, M2, M3, S1, S2, S3, S4, T1, Cs, etc.) are byte-identical regen — they all use shared paths, so v0.10 emits SharedPath as before. Vector count grows from 44 → 45 (or 46/47 if optional vectors land); SHA pin updates.

### Negative vectors

New negative vectors covering the new error variants:

- **`n_orig_count_mismatch`** — synthetic encoding with `Tag::OriginPaths count = 4` but the tree carries only 3 placeholders. Decoder rejects with `Error::OriginPathsCountMismatch { expected: 3, got: 4 }`.
- **`n_orig_path_components_too_long`** — `Tag::OriginPaths` with one `path_decl_i` declaring `component_count = 11`. Decoder rejects with `Error::PathComponentCountExceeded { got: 11, max: 10 }`.
- **`n_orig_paths_truncated`** — header bit 3 set, `Tag::OriginPaths count=3`, but only 2 path-decls follow before the tree bytes. Decoder hits cursor exhaustion mid-path-list inside `decode_path`, rejects with `Error::InvalidBytecode { kind: BytecodeErrorKind::UnexpectedEnd }` (matching the existing convention in `decode_path` / `decode_declaration`).
- **`n_orig_paths_count_zero`** — header bit 3 set, `Tag::OriginPaths count=0`. Decoder rejects at the bytecode layer with `BytecodeErrorKind::OriginPathsCountTooLarge { count: 0, max: 32 }` (the variant name covers both bounds — see F4 / §3 `decode_origin_paths`).
- **`n_orig_paths_count_too_large`** — header bit 3 set, `Tag::OriginPaths count=33`. Decoder rejects with `BytecodeErrorKind::OriginPathsCountTooLarge { count: 33, max: 32 }`.
- **`n_conflicting_path_declarations`** — header byte with bit 3 set but offset-1 byte is `0x34` (or vice versa). Decoder rejects with `Error::InvalidBytecode { offset: 1, kind: BytecodeErrorKind::UnexpectedTag { expected: 0x36, got: 0x34 } }` (or swapped per the bit). Per F5: reuses existing `UnexpectedTag` infrastructure rather than introducing a peer top-level variant.

Existing negative vectors (n01–n15, etc.) regenerate without semantic change.

### Hand-AST coverage

`crates/md-codec/src/bytecode/hand_ast_coverage.rs` adds:

- **`tag_origin_paths_byte_position`** — pin `Tag::OriginPaths` to byte `0x36`.
- **`header_origin_paths_flag_round_trip`** — assert `from_byte(0x08) → BytecodeHeader { version: 0, fingerprints: false, origin_paths: true }` and the round-trip back.
- **`encoder_emits_shared_path_when_all_paths_agree`** — policy with all-shared paths must NOT emit `Tag::OriginPaths` (header bit 3 clear). Asymmetric byte-fill assertion (per `v07-phase2-asymmetric-byte-order-test-inputs` lesson).
- **`encoder_emits_origin_paths_when_paths_diverge`** — policy with divergent paths MUST emit `Tag::OriginPaths` and header bit 3 set.
- **`max_path_components_boundary_10_passes_11_rejects`** — explicit-form path with 10 components round-trips; with 11 components both encoder and decoder reject.

### Defensive-corpus byte-literal pinning

Per `v07-decoder-arm-cursor-sentinel-pattern` (v0.7 P2 review): hand-AST coverage tests for the new OriginPaths decoder arm should use a trailing `0xFF` sentinel pattern to assert cursor-exhaustion correctness — the decoder must consume exactly the bytes it declared and leave the cursor at the byte following the OriginPaths block, ready for the optional Fingerprints block or tree bytes.

(F7: dropped the `origin_paths_walker_reports_first_violation` test from the v1 spec draft. The "walker-reports-first-violation" pattern in `hand_ast_coverage.rs` applies specifically to tap-leaf subset violations where a depth-first AST walk visits leaves in DFS pre-order. The OriginPaths decoder is a flat sequential `for _ in 0..count { decode_path(cursor)? }` loop with early `?` return on first error — the assertion would be trivially true by construction. No defensive value beyond the cursor-sentinel pin already covered.)

---

## §6. Migration + Release Framing

### Wire-format break summary

| Aspect | v0.9.1 | v0.10.0 |
|---|---|---|
| Header byte valid values | `0x00`, `0x04` | `0x00`, `0x04`, `0x08`, `0x0C` |
| Reserved-bits mask | `0x0B` (bits 3, 1, 0) | `0x03` (bits 1, 0) |
| Tag bytes allocated for path declarations | `0x34` only | `0x34` and `0x36` |
| Maximum path component count | unbounded (LEB128 limit) | 10 (enforced) |
| `BytecodeHeader::new_v0` signature | `new_v0(bool)` | `new_v0(bool, bool)` |
| `Tag::OriginPaths` exists | no | yes (`0x36`) |
| `Error::OriginPathsCountMismatch` exists | no | yes |
| `Error::PathComponentCountExceeded` exists | no | yes |
| `BytecodeErrorKind::OriginPathsCountTooLarge` exists | no | yes |
| `PolicyId::fingerprint()` exists | no | yes |
| `MAX_PATH_COMPONENTS` constant | n/a | `10` |

### Migration table for consumer code

| v0.9 consumer code | v0.10 equivalent |
|---|---|
| `BytecodeHeader::new_v0(true)` | `BytecodeHeader::new_v0(true, false)` (or `(true, true)` if also emitting OriginPaths) |
| `match err { Error::ReservedBitsSet { byte: 0x08, .. } => ... }` (catching v0.9 rejections of bit-3-set inputs) | Won't fire under v0.10 — those inputs now decode as OriginPaths-using encodings. |
| Reading the chunk-set-id from chunk headers | Unchanged. |

Most consumer code requires zero changes — the `BytecodeHeader::new_v0` signature change is the only forced edit. Mechanical via:

```bash
# Replace one-arg new_v0 calls with two-arg form (preserves fingerprints flag,
# sets origin_paths to false — typical v0.9 use case had no per-`@N` paths).
find . -type f -name '*.rs' -exec sed -i \
    -e 's/BytecodeHeader::new_v0(\([^)]*\))/BytecodeHeader::new_v0(\1, false)/g' \
    {} +
```

Callers that explicitly want the new behavior pass `true` for the second argument, but the typical pre-v0.10 caller wasn't using OriginPaths and the `false` default is correct.

### PolicyId Type 0 / Type 1 typology (BIP teaching subsection)

The BIP draft gains a §"PolicyId types" subsection (placement: under §"Naming and identifiers" or as its own §). Approximate prose:

> MD defines two cryptographic wallet-identifying hashes at different levels of specificity. We refer to them as **types of PolicyId** for ease of reference:
>
> * **Type 1 — `PolicyId`.** `SHA-256(canonical_bytecode)[0..16]`. Identifies the wallet *template + path layout*: the BIP 388 script structure plus the per-`@N` origin paths (which for v0.x ≤ 0.9 was a single shared path; for v0.10+ admits per-`@N` paths). Two wallets with the same template and same path layout but *different* concrete cosigner xpubs share a `PolicyId`. Engraved as the optional 12-word BIP-39 phrase (see §"Engraving the 12-word PolicyId phrase").
>
> * **Type 0 — `WalletInstanceId`.** `SHA-256(canonical_bytecode || canonical_xpub_serialization)[0..16]`. Identifies the wallet *instance*: template + paths + concrete cosigner xpubs. Two wallets are distinct even if they share a template and path layout, as long as their cosigner xpub sets differ. Computed at recovery time from policy + assembled xpubs; not engraved on any physical card. See [`compute_wallet_instance_id`](https://docs.rs/md-codec/0.10.0/md_codec/fn.compute_wallet_instance_id.html).
>
> Type 1 answers "what shape of wallet is this?"; Type 0 answers "which specific wallet instance is this?" Type 1 is the engraved anchor (Tier 3). Type 0 is the cryptographic check at recovery time.

(Exact placement and wording finalized when v0.10's BIP draft updates land.)

### PolicyId UX — engraving language softening

The BIP draft's existing language about engraving the 12-word phrase shifts from "SHOULD engrave" toward MAY-engrave-for-cross-verification:

> The 12-word PolicyId phrase MAY be engraved on a separate metal anchor for **offline cross-verification** with a digital backup of the codex32 string. Users who maintain only the codex32 Template Card itself need not engrave the phrase — the codex32 string carries the policy directly with BCH error correction; the phrase is a redundant integrity check rather than a recovery primitive.
>
> For users who want a minimal-cost identifier, the **PolicyId fingerprint** (top 4 bytes of `PolicyId`, rendered as 8 lowercase hex characters, parallel to BIP 32 master-key fingerprints) is offered as an 8-character display form.

### Authority precedence with mk1 (BIP cross-reference)

New md1 BIP subsection added under §"Per-`@N` path declaration":

> ===== Authority precedence with MK =====
>
> When an MD card with per-`@N` paths participates in recovery alongside one or more MK cards (xpub backups; see [bg002h/mnemonic-key]), MK's `origin_path` is **authoritative** for the xpub's derivation; MD's per-`@N` path is **descriptive** — the policy's expected path layout. Per-format decoders are not required to be aware of cross-format context; consistency-checking is the recovery orchestrator's responsibility. Mismatch MUST cause the orchestrator to reject the assembly. See MK's BIP §"Authority precedence" and SPEC §5.1 for the full normative semantics.

### CHANGELOG framing

`[0.10.0] — DATE` section leads with "Why a wire-format break?" callout:

> v0.x ≤ 0.9 silently flattened policies with divergent per-`@N` origin paths to a single shared path, losing information. The result was that `decode(encode(p))` could differ from `p` for any policy where cosigners derived xpubs from different paths — a real-world case for any multisig with cosigners using distinct BIP 48 accounts. v0.10 fixes this with a new `Tag::OriginPaths` block. Existing shared-path encodings remain byte-identical; divergent-path policies now round-trip correctly.

Followed by the standard sections (Added, Changed, Wire format, FOLLOWUPS closed, etc.), per the v0.9.0 release pattern.

### Family-token roll

Test-vector corpora regenerate under family token `"md-codec 0.10"`. SHA pins update in `crates/md-codec/tests/vectors_schema.rs`. Both schema-1 (`v0.1.json`) and schema-2 (`v0.2.json`) regenerate.

### Existing tests affected by `MAX_PATH_COMPONENTS = 10`

The existing `decode_path_round_trip_multi_byte_component_count` test in `crates/md-codec/src/bytecode/path.rs` exercises 128-component paths to validate multi-byte LEB128 round-trip in the count field. Under v0.10's cap of 10, this test must rewrite to exercise multi-byte LEB128 in the **child-index dimension** (e.g., `m/16384` where `16384 = 2 × 8192` requires 2-byte LEB128) rather than the **component-count dimension**. Plan-time work item; the test's defensive value (multi-byte LEB128 round-trip exercise) survives the rewrite.

### Sibling-repo coordination

mk1's main branch (currently behind its in-flight `feature/v0.1.0-implementation`) inherits the path dictionary's continued definition; v0.10 doesn't change the path dictionary itself. mk1's `feature/v0.1.0-implementation` work or its successor branch will land mk1 v0.1.0 alongside or after md1 v0.10.0; coordination per `design/RELEASE_PROCESS.md` lockstep checklist.

mk1 BIP §"Authority precedence" prose stays unchanged across v0.10 — it already declared the cross-format semantics that md1 v0.10 now references. mk1's existing forward-reference hedges (see `design/agent-reports/v0-9-phase-0-mk1-hedge-audit.md`) remain accurate; the post-v0.9 hedge cleanup carries naturally into v0.10.

---

## Appendix A — Open implementer questions

These are minor questions deferred to plan time; nothing wire-format-affecting:

1. **Should `WalletPolicy::placeholder_paths_in_index_order` be a public method or `pub(crate)`?** Public allows tooling to introspect the path layout; pub(crate) keeps the API surface tight. Default: pub(crate); promote later if a consumer use case surfaces.

2. ~~Per-`@N` path inheritance from key-information-vector?~~ **Resolved at spec time** per opus review F14: the precedence chain is documented in §4 "Encoder per-`@N`-path precedence chain" (Tier 0: `opts.origin_paths` override; Tier 1: `decoded_origin_paths`; Tier 2: walk KIV; Tier 3: shared-path fall-through). Plan-time finalizes the implementation details only.

3. **Should the `o1_*` corpus vector use synthetic dummy keys or a fingerprints-block-bearing variant?** Default: synthetic, no fingerprints. Fingerprints variant is optional `o2_*`.

4. **CLI display format for `PolicyId::fingerprint()` output.** `0x{:08x}` (8 hex chars with `0x` prefix) is the natural choice; document in `bin/md/main.rs` rustdoc.

---

## Self-review checklist (post-opus-review-pass-1)

- [x] All 13 brainstorm questions addressed in §1 decision matrix.
- [x] Wire-format examples cover shared-path, divergent-path, divergent-path-with-fingerprints. Byte sequences re-verified per F1 (`m/48'/0'/0'/100'` → `FE 04 61 01 01 C9 01`).
- [x] Header byte changes documented with old/new mask.
- [x] Encoder/decoder dispatch logic specified, including the new `decoded_origin_paths` round-trip stability field (F2) and the 4-tier per-`@N` path precedence chain (F14).
- [x] New error variants enumerated with shapes; structural-vs-semantic split into `BytecodeErrorKind::OriginPathsCountTooLarge` (bytecode layer) and `Error::OriginPathsCountMismatch` (policy layer) per F4. `Error::ConflictingPathDeclarations` dropped per F5; `BytecodeErrorKind::UnexpectedTag` reused.
- [x] Test-corpus additions enumerated (positive + negative + hand-AST). `o3_pkh_*` reshaped to `o3_wsh_sortedmulti_2of4_*` per F12 (pkh has only 1 placeholder).
- [x] Migration sed snippet provided. Test rewrite for `MAX_PATH_COMPONENTS` cap noted per F15.
- [x] BIP teaching subsections drafted (Type 0/1 typology, engraving softening, mk1 cross-reference).
- [x] Cross-format coordination addressed (mk1 path dictionary stability, mk1 BIP unchanged).
- [x] CHANGELOG and family-token framing specified.

## Opus review pass-1 findings status

- F1 (strong, byte sequence wrong) — fixed.
- F2 (strong, missing `decoded_origin_paths` field) — added in §4.
- F3 (nice-to-have, wrong error variant for truncation) — fixed.
- F4 (strong, wrong-shaped count>32 rejection) — split into `BytecodeErrorKind::OriginPathsCountTooLarge` (structural) + `Error::OriginPathsCountMismatch` (semantic).
- F5 (nice-to-have, redundant `ConflictingPathDeclarations`) — dropped; `BytecodeErrorKind::UnexpectedTag` reused.
- F6 (nice-to-have, count=0 semantics) — explicit `1..=32` MUST clause added in §2; `n_orig_paths_count_zero` negative vector filed.
- F7 (nice-to-have, premature walker test) — dropped from §5.
- F8 (nice-to-have, encoder try_from cleanup) — minor; folded into encoder code sketch.
- F9 (nice-to-have, cap-in-encode_path) — clarified in §4: cap lives in `encode_path` / `decode_path` for both `Tag::SharedPath` and `Tag::OriginPaths` reuse.
- F10 (nice-to-have, BytecodeHeader builder API) — deferred; would benefit from v2 design pass.
- F11 (nice-to-have, decoded_*_path mutual exclusion invariant) — added in §4.
- F12 (nice-to-have, o3_pkh_* category error) — fixed.
- F13 (confirmation, Type 0/1 prose accurate under Route X) — no change.
- F14 (nice-to-have, lift Appendix Q2 into §4) — done.
- F15 (nice-to-have, existing 128-component test rewrite) — migration note added in §6.
