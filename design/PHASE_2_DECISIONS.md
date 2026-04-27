# Phase 2 Decision Log

Living document of decisions made during autonomous execution of Phase 2 (Bytecode Foundation). Each entry: what was decided, why, alternatives considered, where to verify in code.

## Open questions for review

(Populated as design questions arise that I picked a default for. Empty = no open questions.)

---

## Decisions made

### D-1 (Task 2.1): `Tag::from_byte` uses `match` instead of `unsafe transmute`

**Context**: The plan file specifies `Tag::from_byte` using `unsafe { std::mem::transmute(b) }` after a bounds check. The Pre-Execution Checklist (item 4) flagged this as a UB hazard if any future variant is added with a non-contiguous value (e.g., a hypothetical `Fingerprints = 0x35` would skip 0x34 and break the transmute, silently).

**Decision**: Use a `match` expression mapping each byte to its variant. ~52 arms but trivially correct, no `unsafe`, future-variant-safe.

**Rationale**: The cost is one-time enum maintenance (each new variant adds one match arm). The benefit is eliminating an entire class of UB that could be introduced by a careless future commit. Compiler-generated jump table is just as fast as transmute. v0.1 is the right time to make this choice; refactoring once 50 callers use `from_byte` is harder than getting it right now.

**Alternatives considered**:
- **Keep transmute with safety comment**: rejected — comment-as-safety is a fragile contract.
- **Use `num_enum` crate**: rejected — adds a dependency for one function.
- **`bytemuck::from_bytes`**: rejected — wrong abstraction, still relies on layout.

**Verify in code**: `crates/wdm-codec/src/bytecode/tag.rs`, `Tag::from_byte` body.

---

### D-5 (Task 2.12): Decoder output type and key substitution

**Context**: With Phase 2's encoder feature-complete, the decoder needs a public API. Three reasonable options for the output type:

- **(A)** `Result<Descriptor<DescriptorPublicKey>, Error>` — symmetric with encoder input. Caller must supply a `&[DescriptorPublicKey]` key info vector to substitute placeholders back into concrete keys.
- **(B)** `Result<DecodedTemplate, Error>` where `DecodedTemplate` is a custom intermediate type holding `WdmKey::Placeholder(u8)` references unchanged. Caller substitutes keys later.
- **(C)** Hybrid: decoder produces (B), with a separate `instantiate(keys: &[DescriptorPublicKey]) -> Result<Descriptor<...>, Error>` adapter.

**Decision**: **(A)** — public API:

```rust
pub fn decode_template(
    bytes: &[u8],
    keys: &[DescriptorPublicKey],
) -> Result<Descriptor<DescriptorPublicKey>, crate::Error>;
```

**Rationale**:
- Symmetric with `encode_template`. A caller who encoded with `placeholder_map: HashMap<Pk, u8>` can decode with `keys: Vec<Pk>` (just `keys[i]`) and round-trip.
- Reuses miniscript's existing types — no new public surface to maintain.
- For `Tag::Placeholder` + LEB128 idx: the decoder looks up `keys[idx]` and returns `Error::PolicyScopeViolation` with "placeholder index out of range" if `idx >= keys.len()`.
- For the v1+ `Reserved*` key tags (0x24..=0x31): the decoder rejects them with `Error::InvalidBytecode { kind: UnknownTag(b) }` since v0.1 cannot construct inline keys.

**Internal design**: cursor-based reader + per-tag dispatch.

```rust
struct Cursor<'a> { bytes: &'a [u8], offset: usize }
// fn read_byte / read_varint_u64 / read_array<const N: usize> / require_empty
```

Top-level reads `Tag::Wsh`, recursively parses Miniscript via tag dispatch, wraps in `Descriptor::Wsh(Wsh::new(...))`.

**Alternatives considered**:
- **(B) custom intermediate type**: rejected — adds public surface for marginal benefit. The 2-arg API in (A) is the natural inverse of `encode_template(d, &map)`.
- **(C) hybrid**: deferred to v0.2. If real callers need lazy key substitution we can add the intermediate then.

**Errors**:
- `Error::InvalidBytecode { offset, kind: BytecodeErrorKind::* }` for malformed input (truncated, unknown tag, varint overflow).
- `Error::PolicyScopeViolation(...)` for v0.1-out-of-scope inputs (Reserved* key tags, taproot tags if encountered).

**Verify in code**: `crates/wdm-codec/src/bytecode/decode.rs` (Task 2.12 onwards).

---

### D-4 (Task 2.4): Encoder takes `Descriptor<DescriptorPublicKey>` + placeholder map, mirrors `descriptor-codec` walker pattern

**Context**: Phase 2 needs an encoder turning a BIP 388 wallet policy into canonical bytecode. The plan said "walk a miniscript AST"; the upstream `descriptor-codec` (CC0) walks a `Descriptor<DescriptorPublicKey>` via a `trait EncodeTemplate` impl per fragment type. WDM differs only in that key positions get replaced by `Tag::Placeholder` + LEB128 index drawn from the wallet policy's key information vector — there is no separate payload byte stream as in descriptor-codec.

**Decision**: Public encoder API for v0.1:

```rust
pub fn encode_template(
    descriptor: &miniscript::Descriptor<miniscript::descriptor::DescriptorPublicKey>,
    placeholder_map: &std::collections::HashMap<miniscript::descriptor::DescriptorPublicKey, u8>,
) -> Result<Vec<u8>, crate::Error>;
```

Internal walker is a private `trait EncodeTemplate` mirroring descriptor-codec's structure but emitting only the `template` byte stream (no `payload`). For each leaf key encountered, look up `placeholder_map`; if present, emit `Tag::Placeholder` + `varint::encode_u64(index)`; if missing, return `Error::PolicyScopeViolation` with a descriptive message (v0.1 forbids inline keys in the wallet-policy framing).

**Rationale**:
- Reuses miniscript's existing types and parsing (no custom AST).
- Mirrors descriptor-codec line-by-line for the common operator arms — easier to verify correctness against the reference.
- Placeholder substitution at the leaf is the only WDM-specific divergence; one well-marked seam.
- Caller-provided `placeholder_map` keeps the encoder pure (no parsing of `@i` strings inside the encoder).

**Alternatives considered**:
- **Custom `WdmDescriptor`/`WdmAst` type**: rejected — would duplicate miniscript's type hierarchy. YAGNI.
- **Use `Descriptor<WdmKey>`**: rejected — requires implementing `MiniscriptKey`, `ToPublicKey`, and other traits on `WdmKey`, each with non-trivial bodies. Large surface.
- **Encoder returns `(template, payload)` mirroring descriptor-codec**: rejected — WDM v0.1 has no payload concept. The Template Card carries only the template; key material lives on the Xpub Cards (separate structure entirely).

**v0.1 scope reminders** (enforced inside the encoder; emit `PolicyScopeViolation` on violation):
- Only `Wsh()` top-level (no Sh, Bare, Pkh, Wpkh, Tr in v0.1).
- All keys MUST be in `placeholder_map` (no inline keys).
- No taproot.

**Verify in code**: `crates/wdm-codec/src/bytecode/encode.rs` (Task 2.4 onwards). Public re-export from `crates/wdm-codec/src/lib.rs` once stable.

---

### D-3 (Task 2.1 review fix): `Tag` marked `#[non_exhaustive]`

**Context**: Task 2.1's code review flagged that `Tag` lacks `#[non_exhaustive]` despite the module-level doc forecasting new tags (e.g., fingerprints 0x35 in v0.2). Without the attribute, downstream `match tag { ... }` consumers will get a hard compile error when v0.2 adds variants.

**Decision**: Add `#[non_exhaustive]` to `Tag`. Same reasoning as D-2 for `WdmKey`. Zero runtime cost.

**Verify in code**: `crates/wdm-codec/src/bytecode/tag.rs`, `pub enum Tag` declaration.

---

### D-2 (Task 2.3): `WdmKey::Key(DescriptorPublicKey)` shipped in v0.1 but unconstructible

**Context**: The plan file specifies `WdmKey::Key(DescriptorPublicKey)` as a forward-compat variant for v1+ foreign-xpub support. v0.1 only uses `Placeholder(u8)`.

**Decision**: Include the variant per plan, BUT mark the enum `#[non_exhaustive]` and document that v0.1 encoders MUST emit only `Placeholder` and v0.1 decoders MUST reject any non-placeholder key tag (these are the `Reserved*` tags 0x24–0x31 in `Tag`).

**Rationale**: Including the variant gives v1+ implementations a clear extension point. `#[non_exhaustive]` lets us add fields/variants without breaking downstream code. The runtime check for "v0.1 rejects" lives in the decoder, not the type.

**Verify in code**: `crates/wdm-codec/src/bytecode/key.rs`. v0.1 decoder rejection lives in `bytecode/decode.rs` (a future task).

---

(More decisions appended as Phase 2 progresses.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| 2.1 Tag enum | `c543389` / `1f963a7` | 52 variants 0x00–0x33; `match`-based `from_byte` (D-1); `#[non_exhaustive]` (D-3). |
| 2.2 LEB128 varint | `73e3501` / `046972a` | u64 encode/decode + 8 tests covering boundaries, overflow, truncation, trailing data. |
| 2.3 WdmKey enum | `3732af8` / `2723211` | `Placeholder(u8)` + `Key(DescriptorPublicKey)`; `#[non_exhaustive]` per D-2; derives include `Hash`. |
| 2.4 Encoder skeleton | `a92748c` / `5e4ba1a` | Public `encode_template`; private `EncodeTemplate` trait; Wsh-only at top level (D-4). |
| 2.5 WshInner + leaves | `d97bef5` / `df209e7` | True/False/PkK/PkH terminals; `encode_key` migrated to trait impl. |
| 2.6 Multisig | `1d3c594` / `cb68282` | sortedmulti, multi, multi_a; generic `Threshold` impl; `Arc` not yet needed. |
| 2.7 Logical ops | `e3441a5` / `11a39cf` | and_v/and_b/and_or/or_b/or_c/or_d/or_i; Arc forwarding impl. |
| 2.8 Threshold + timelocks | `6edafbb` (no fix needed) | thresh, after, older. |
| 2.9 Hash literals | `be13f6e` / `4af599f` | sha256/hash256/ripemd160/hash160; hash256 byte-order doc + asymmetric test pattern. |
| 2.10 Wrappers + RawPkH | `68e0173` / `ed1db96` | Alt/Swap/Check/DupIf/Verify/NonZero/ZeroNotEqual + RawPkH; Terminal coverage now complete; canary deleted. |
| 2.11 Taproot (Tr/TapTree) | _deferred_ | v0.2 scope; v0.1 rejects `Descriptor::Tr` at the top level (Task 2.4). |

**Phase 2 encoder is feature-complete as of `ed1db96`.** Next: decoder (Task 2.12 onwards).
