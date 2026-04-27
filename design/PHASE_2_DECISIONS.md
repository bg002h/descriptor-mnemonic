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

| Task | Commit | Status notes |
|------|--------|--------------|
| _(populated as I go)_ | | |
