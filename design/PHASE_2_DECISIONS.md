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

### D-2 (Task 2.3): `WdmKey::Key(DescriptorPublicKey)` shipped in v0.1 but unconstructible

**Context**: The plan file specifies `WdmKey::Key(DescriptorPublicKey)` as a forward-compat variant for v1+ foreign-xpub support. v0.1 only uses `Placeholder(u8)`.

**Decision**: Include the variant per plan and mark the enum `#[non_exhaustive]`. Operational contract:

- v0.1 encoders MUST NOT construct or emit `WdmKey::Key`. The current encoder (D-4) takes `Descriptor<DescriptorPublicKey>` + a placeholder map and never builds a `WdmKey` value at all, so this is enforced by absence rather than by a type guard.
- v0.1 decoders MUST reject the bytecode tag bytes that would produce `WdmKey::Key` — namely the inline-key tags 0x24–0x31 (the `Reserved*` set in `Tag`) — and return `Error::PolicyScopeViolation`. This rejection lives in `decode.rs`.
- The variant being publicly constructible in Rust is acceptable: `WdmKey` is not part of v0.1's public API surface (the encoder takes `Descriptor<DescriptorPublicKey>`, the decoder returns `Descriptor<DescriptorPublicKey>`), so external callers have no v0.1 surface that consumes a `WdmKey` value. Gating the constructor with `pub(crate)` would not change this and would block a future v1+ that legitimately constructs the variant.

**Rationale**: Including the variant gives v1+ implementations a clear extension point. `#[non_exhaustive]` lets us add fields/variants without breaking downstream code. The runtime check for "v0.1 rejects" lives in the decoder, not the type. Earlier wording said "unconstructible," which is imprecise — `WdmKey::Key(some_dpk)` compiles fine; what's actually true is that no v0.1 code path constructs it.

**Verify in code**: `crates/wdm-codec/src/bytecode/key.rs`. v0.1 decoder rejection lives in `bytecode/decode.rs` (a future task).

---

### D-3 (Task 2.1 review fix): `Tag` marked `#[non_exhaustive]`

**Context**: Task 2.1's code review flagged that `Tag` lacks `#[non_exhaustive]` despite the module-level doc forecasting new tags (e.g., fingerprints 0x35 in v0.2). Without the attribute, downstream `match tag { ... }` consumers will get a hard compile error when v0.2 adds variants.

**Decision**: Add `#[non_exhaustive]` to `Tag`. Same reasoning as D-2 for `WdmKey`. Zero runtime cost.

**Verify in code**: `crates/wdm-codec/src/bytecode/tag.rs`, `pub enum Tag` declaration.

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
- Output is the tree byte stream only. The 1-byte bytecode header (`0x00` or `0x04`) and the path declaration (`Tag::SharedPath`, 0x33) are NOT prepended by `encode_template`; they are added by the Phase 3 framing layer. Canonical bytecode = header || path-decl || tree, per BIP §"General structure".

**Re-export gate (concrete)**: re-export `encode_template` and `decode_template` from `lib.rs` immediately after Task 2.20 (the round-trip tests on the C1–C5 corpus subset reachable through the tree-only API), with `#[doc(hidden)]`. The `#[doc(hidden)]` comes off (or the re-export is dropped) at P5 once `encode_bytecode` / `decode_bytecode` from design §3 wraps them. `Tag` and `WdmKey` stay at `wdm_codec::bytecode::*` — not re-exported at the crate root — because they're representation details whose public surface should track the BIP's tag-table versioning, not the lib.rs API surface.

**Verify in code**: `crates/wdm-codec/src/bytecode/encode.rs` (Task 2.4 onwards). Public re-export from `crates/wdm-codec/src/lib.rs` once stable.

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
- `Error::InvalidBytecode { offset, kind: BytecodeErrorKind::* }` for malformed input (truncated, unknown tag, varint overflow, trailing bytes, **and** `BytecodeErrorKind::TypeCheckFailed(String)` for miniscript type-check failures during `Wsh::new(...)` reconstruction). The error carries the offset of the offending tag and the upstream miniscript error message.
- `Error::PolicyScopeViolation(...)` for v0.1-out-of-scope inputs (Reserved* key tags, taproot tags, placeholder index out of range relative to `keys.len()`).
- `Error::Miniscript(String)` MUST NOT escape the decoder boundary. The previously-existing blanket `From<miniscript::Error> for Error` impl in `error.rs` was removed (Issue 3 from the Phase 2 decision review); the decoder maps miniscript errors explicitly to `BytecodeErrorKind::TypeCheckFailed`. (`Error::Miniscript(String)` remains for `WalletPolicy::from_str` and other non-decode call sites, where the construction is now explicit at every site.)

**v0.1 scope reminders**:
- `decode_template` consumes the tree byte stream only. The Phase 3 framing layer is responsible for stripping the bytecode header byte (0x00 / 0x04) and the path declaration (Tag::SharedPath, 0x33), then handing the remaining tree bytes to `decode_template`. The decoder's `Tag::Wsh` start point matches this contract.

**Verify in code**: `crates/wdm-codec/src/bytecode/decode.rs` (Task 2.12 onwards).

---

### D-6 (Phase 2 framing boundary): bytecode header + path declaration are Phase 3, not Phase 2

**Context**: The BIP describes canonical bytecode as `header || path-decl || tree`. The first reviewer's open question OQ-1 asked whether Phase 2's `encode_template` / `decode_template` produce/consume canonical bytecode or just the tree subtree. The IMPLEMENTATION_TASKS plan schedules the bytecode header for Task 3.1 and the path declaration for Tasks 3.2–3.4 — i.e., Phase 3 — but D-4 and D-5 didn't make the boundary explicit.

**Decision**: Phase 2 owns the operator tree byte stream only. Phase 3 owns the framing (header byte + path declaration). `encode_template` produces the tree; `decode_template` consumes the tree. The framing layer wraps/unwraps both at the appropriate boundary. The path module (`bytecode/path.rs`) remains a stub through end of Phase 2 and is fleshed out in Tasks 3.2–3.4.

**Rationale**: The operator-tree encoder/decoder is reusable independently of framing (e.g., for future variants that change framing without changing the tree), and keeping framing concerns out of `encode_template` / `decode_template` keeps the tree walker focused. The trade-off — that tree-only output isn't a complete canonical bytecode — is documented explicitly in D-4 and D-5 amendments.

**Spec ambiguity flagged for P5.5 BIP review** (user-confirmed MUST-present): the BIP §"Path declaration" should clarify that the shared-path declaration is MUST-present in v0 even for placeholder-free policies (e.g., `wsh(after(N))` with no keys at all). Current BIP wording is consistent with MUST-present but not explicit; P5.5 should make this explicit.

**Verify in code**: `crates/wdm-codec/src/bytecode/{encode,decode}.rs` — both files' module docs and the public API doc comments mention the tree-only contract. `crates/wdm-codec/src/bytecode/path.rs` remains a stub.

---

### D-7 (Phase 2 spec compliance): single-byte encoding for placeholder index, k, and n

**Context**: The BIP draft §"LEB128 encoding" (line 417) is explicit that `multi`, `thresh`, `sortedmulti`, `multi_a` threshold and count fields use **single-byte** encoding, not LEB128. The tag table also documents `Tag::Placeholder` (0x32) operator data as "1-byte placeholder index (0–255)". The first reviewer's Issue 1 caught that the encoder emitted these as LEB128 — coinciding with single-byte for values 0–127 (so all current corpus tests passed) but diverging for values ≥128.

**Decision**: Single-byte encoding is the **wire-format permanent contract** for v0.1 + v0.2 + foreseeable extensions. Affected sites in `crates/wdm-codec/src/bytecode/encode.rs`:

1. `impl EncodeTemplate for DescriptorPublicKey` — placeholder index emitted as `out.push(index)` (no varint).
2. `impl<T: EncodeTemplate, const MAX: usize> EncodeTemplate for Threshold<T, MAX>` — `k` and `n` emitted as `out.push(u8::try_from(self.k())?)` and parallel for `n`. The `try_from` returns `PolicyScopeViolation` if the value exceeds 255 (defensive guard; miniscript's bounds prevent reaching this in practice).
3. `impl EncodeTemplate for SortedMultiVec<DescriptorPublicKey, Segwitv0>` — same pattern as Threshold.

**Wire-format upgrade path** (user-confirmed): future support for >255-key multisig (theoretical taproot extension beyond BIP 388's current 32-key cap) ships as a **new tag** (e.g., `Tag::LargeMulti = 0x36` with LEB128 fields), not a width change to the existing `Tag::Multi` field. This preserves the v0.1 wire-format invariant and gives versioning a clean handle.

**Test pinning**: `encode_placeholder_index_above_127_uses_single_byte` exercises an index of 200 and asserts the emitted bytes are `[0x32, 0xC8]` (Tag::Placeholder + 200) rather than `[0x32, 0xC8, 0x01]` (Placeholder + LEB128 of 200). Threshold k/n ≥128 are unreachable through miniscript's bounds; the `u8::try_from` conversion is a defensive guard documented in code rather than tested.

**Verify in code**: `crates/wdm-codec/src/bytecode/encode.rs` — three impl bodies (DescriptorPublicKey, Threshold, SortedMultiVec). `tests::encode_placeholder_index_above_127_uses_single_byte` pins the placeholder case.

---

### D-8 (Task 2.13 inner-fragment dispatcher): error-handling pattern

**Context**: The encoder's `Terminal` match in `encode.rs` carries `#[allow(unreachable_patterns)]` because miniscript v12's `Terminal` is not `#[non_exhaustive]` — the wildcard guards against a future miniscript upgrade that adds variants. The future Task 2.13+ inner-fragment decoder dispatcher will face an analogous match, but on `Tag` (which IS `#[non_exhaustive]` per D-3), so the wildcard arm is genuinely reachable today and the situation is different.

**Decision**: Task 2.13+ inner-fragment dispatcher MUST NOT use `#[allow(unreachable_patterns)]`. Two error paths handle the two failure modes:

1. `Tag::from_byte(b)` returns `None` → `Error::InvalidBytecode { offset, kind: BytecodeErrorKind::UnknownTag(b) }`. (Same as `decode_descriptor` at the top level today.)
2. `Tag::from_byte(b)` returns `Some(t)` but `t` isn't a valid inner-fragment tag (e.g., `Tag::Wsh` mid-tree, `Tag::SharedPath` outside the framing layer, `Tag::Fingerprints` in v0.1, or a v0.2+ tag) → `Error::PolicyScopeViolation(format!("tag {t:?} (0x{b:02x}) is not valid as an inner fragment"))`.

**Rationale**: The wildcard arm is reachable today for top-level tags appearing mid-tree, framing-layer tags, Reserved* inline-key tags, and any future tag. `#[allow(unreachable_patterns)]` would be incorrect (clippy would flag it as a reachable arm) and would mask the real diagnostic. The two-error-path discipline matches the top-level dispatcher's pattern (`decode_descriptor` already uses this structure).

**Verify in code**: `crates/wdm-codec/src/bytecode/decode.rs` — the future inner dispatcher (introduced in Task 2.13). The current top-level dispatcher already follows the pattern.

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
| **review fix bundle** | `36ac8f3` | D-7 single-byte k/n/index + Issue 3 `BytecodeErrorKind::TypeCheckFailed` + decision-log amendments (D-2/D-4/D-5 amended; D-6/D-7/D-8 added). |
| 2.12 Decoder skeleton | `133e016` / `a0ad5aa` | `decode_template` + Cursor + top-level dispatch (D-5). |
| 2.13 Decoder Wsh inner + leaves | `ee508d0` / `cb7e117` | True/False/PkK/PkH; `decode_placeholder` reads single-byte index per D-7. |
| 2.14 Decoder multisig + thresh | `1d3e0a6` / `1c5e4cb` | SortedMulti/Multi/MultiA/Thresh; wires `decode_miniscript` recursion. |
| 2.15 Decoder wrappers + RawPkH | `9cf9068` / `990ac54` | Alt/Swap/Check/DupIf/Verify/NonZero/ZeroNotEqual + RawPkH; first parser-driven `wsh(pk(K))` round-trip. |
| 2.16 Decoder After/Older | `988f5d7` / `26edd17` | Timelocks via `read_varint_u64`; `from_consensus(0)` rejection coverage. |
| 2.17 Decoder hash literals | `7362113` (no fix needed) | sha256/hash256/ripemd160/hash160; asymmetric-pattern wire-format pinning. |
| 2.18 Decoder logical ops | `dd305b5` (no fix needed) | and_v/and_b/and_or/or_b/or_c/or_d/or_i; final Phase 2 task. Catch-all migrates from "(Task 2.X+)" deferred-stub form to structural-reason guard. |
| 2.19–2.20 | _absorbed_ | The original plan had 2.18 cover encoder logical ops only and 2.19–2.20 cover decoder mirrors. The decoder mirror landed in Task 2.18 (this file's 2.18 = original plan's 2.18 + 2.19 + 2.20 combined for the logical-op group). All operator groups now have both encoder and decoder coverage. |

**Phase 2 is feature-complete as of `dd305b5`.** Both encoder and decoder cover every Segwitv0 Terminal variant in miniscript v12. 169 lib tests passing. Catch-all in `decode_terminal` is now a structural defensive guard (no more deferred-task references).

Next: Phase 3 (framing layer — bytecode header + path declaration), Phase 4 (chunking + wallet ID), or P5 (high-level public API: `encode_bytecode` / `decode_bytecode` from design §3) — see `design/IMPLEMENTATION_PLAN_v0.1.md` for the phase outline.
