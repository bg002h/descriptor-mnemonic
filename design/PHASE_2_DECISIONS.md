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
