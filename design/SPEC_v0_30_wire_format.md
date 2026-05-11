# SPEC — md-codec v0.30 wire format

> **Status:** Design-frozen 2026-05-10. Implementation pending across Cycles 2–5 per `design/IMPLEMENTATION_PLAN_v0_30.md`. Pre-alpha redesign; clean break with v0.x — v0.x payloads are cleanly rejected, not silently mis-decoded.
>
> **Companion files:**
> - `design/IMPLEMENTATION_PLAN_v0_30.md` — per-phase rollout (Phases A–J across Cycles 2–5)
> - `design/agent-reports/spike-v0.30-{q9,q10,q11,q13,q6}-pre-spec.md` — Phase 0a empirical validation per `feedback_spike_before_locking_wire_format`
> - `design/SPEC_v0_11_wire_format.md` — historical baseline (v0.11–v0.18 wire format)
> - `design/FOLLOWUPS.md` `v2-design-questions` entry — original Q1–Q13 + 3 smaller wins catalog
> - `bip/bip-mnemonic-descriptor.mediawiki` — BIP draft; rewrite is a post-implementation deliverable (Phase I)

---

## 1. Introduction

### 1.1 Scope

A redesigned wire format for md-codec, replacing the v0.11–v0.18 wire format wholesale. Internal `version` field advances from 0 (v0.x) to 4 (v0.30). Crate version bumps from 0.18 to 0.30 (the +12 minor jump telegraphs "major change" without leaving the pre-1.0 sandbox per BIP draft framing).

Optimized for the engraving use case (template + origin paths only) where most multisig wallets save 2–14 codex32 chars on engraved output. Also wins (smaller margins) when fingerprints (TLV 0x01) and/or xpubs (TLV 0x02) are inlined.

### 1.2 What changes vs v0.x

Nine concrete changes, each addressing one or more numbered items from the `v2-design-questions` catalog (`design/FOLLOWUPS.md`):

| # | Change | Drivers |
|---|--------|---------|
| 1 | Header version field width: 3 → 4 bits (absorbs reserved bit) | Q8, SW1 |
| 2 | Header layout: in-band mode dispatch on bit 0 (chunked-flag); auto-dispatch without caller hint | Q10 |
| 3 | Wire-format `version` value = 4 (next breakage candidates: 8, 12) | Q10, clean-break |
| 4 | Bytecode primary tag width: 5 → 6 bits (64 slots; 28 free for future operators) | Q13 |
| 5 | Bytecode tag layout: semantic ranges (top-level / structure / multi / wrappers / binary ops / timelocks / hash leaves) | Q7 |
| 6 | Multi-family child packing: raw `key_index` only (no per-child `Tag::PkK`) | Q9 |
| 7 | Walker normalization: bare `Tag::PkK`/`Tag::PkH` always; `Tag::Check` reconstructed by renderer for key-leaf positions | Q12 |
| 8 | NUMS encoding in `tr()`: 1-bit `is_nums` flag; `kiw = ⌈log₂(n)⌉` (no sentinel widening) | SW3 |
| 9 | Decoder error taxonomy refinement (`WireVersionMismatch`, `OperatorContextViolation`, `NUMSSentinelConflict`, …) | §11 |

### 1.3 What does NOT change (preserved from v0.x)

| Aspect | v0.x and v0.30 both use |
|--------|--------------------------|
| Codex32 alphabet | bech32-style 32-char alphabet, 5 bits per symbol |
| HRP + separator | `md1` prefix |
| BCH polynomial | BIP-93 with per-HRP target residue (Q6 deferred) |
| BCH checksum | 13 symbols (65 bits) |
| Chunked envelope structure | 37-bit chunk header (chunk-set-id 20, count-1 6, index 6, version+flags 5); `SINGLE_STRING_PAYLOAD_BIT_LIMIT = 320 bits` |
| Path-decl section | origin paths per `@N` (BIP-32 derivation), LP4-ext varint components |
| Use-site path encoding | `<m;n>/...` multipath suffix; standard `<0;1>/*` = 16 bits |
| TLV section primary tag width | **5-bit** (Q13 split: bytecode tags grow to 6-bit; TLV tags retained at v0.x width) |
| TLV 0x00 use-site overrides | unchanged |
| TLV 0x01 fingerprints | per-entry `idx(kiw) + 32 bits` |
| TLV 0x02 xpubs | per-entry `idx(kiw) + 65 × 8 = 520 bits` (chain-code 32B + compressed-pubkey 33B) |
| TLV 0x03 origin-path overrides | unchanged |
| TLV section framing | implicit end + rollback-as-padding (≤7 bits) |
| Origin-path divergence flag | header bit 4 + per-`@N` inline block (Q11 wont-fix) |
| `k-1` and `n-1` in multi/thresh body | 5-bit fixed (Q4 lock per "most bit-efficient encoding in circumstances that matter") |
| Byte arrays | 8 bits per byte |
| Timelocks | 32-bit Bitcoin consensus encoding |
| Recursion depth cap | 128 |

### 1.4 Backward compatibility

**None.** v0.30 is a clean break. v0.30 codec does not read v0.x payloads; v0.x codec does not read v0.30 payloads. v0.x payloads fed to a v0.30 decoder raise `WireVersionMismatch` cleanly (proven by the auto-dispatch rejection trace in §2.1 below).

If a future cycle adds a migration story, it will live in a separate `md-codec-legacy` crate or a pinned-v0.18 binary. Out of scope here.

### 1.5 Audit reclassifications (v2-design-questions catalog items)

| Item | Disposition | Where addressed |
|------|-------------|------------------|
| Q1 (unified per-`@N` block) | Absorbed into Q11 | §6 |
| Q2 (path dictionary) | Resolved-by-v0.11 (already retired) | n/a |
| Q3 (md1+mk1 unification) | Foreclosed (sibling-format-bundle decision per `CLAUDE.md`) | n/a |
| Q4 (encoding uniformity) | LP4-ext for variable-range fields; fixed-5 for bounded `k`/`n` | §3 |
| Q5 (string-layer split) | Status quo retained (bytecode-layer metadata) | §8 |
| Q6 (BCH polynomial separation) | Deferred (per-HRP-residue is sufficient for hand-transcription threat model) | §11 |
| Q7 (semantic tag ranges) | Implemented within new 6-bit primary space | §3 |
| Q8 (version field width) | Widened to 4 bits | §2 |
| Q9 (multi child packing) | Implemented (raw `key_index`) | §4 |
| Q10 (header bit alignment + in-band discriminator) | Implemented (chunked-flag at bit 0) | §2 |
| Q11 (per-`@N` override unification) | Wont-fix (current bifurcated design empirically optimal) | §6 |
| Q12 (walker normalization) | Implemented (bare `PkK`/`PkH` on wire) | §5 |
| Q13 (tag-space rework) | Bytecode 6-bit / TLV 5-bit split | §3 |
| SW1 (reserved bit folded into version) | Absorbed by Q10/Q8 | §2 |
| SW2 (TLV section length prefix) | Reverted (rollback-as-padding retained from v0.x) | §9 |
| SW3 (NUMS-sentinel removal) | Implemented (1-bit `is_nums` flag) | §7 |

---

## 2. Header layout

Drivers: Q8, Q10, SW1.

### 2.1 Single-payload header (5 bits, MSB-first)

```
bit:  4       3   2   1   0
      paths   v3  v2  v1  v0
```

- Bit 4: `paths` flag (origin-path divergence; same role as v0.x).
- Bits 3..0: 4-bit `version` field. v0.30 uses **`version = 4`**.

### 2.2 Chunk header first 5-bit symbol

```
bit:  4    3    2    1    0
      v3   v2   v1   v0   chunked
```

- Bit 0: `chunked` flag (always 1 for chunk header).
- Bits 4..1: 4-bit `version` field.

Chunk header remainder (32 more bits): `chunk-set-id(20) | count-1(6) | index(6)`. Total chunk header = 37 bits (unchanged from v0.x in size).

### 2.3 Auto-dispatch (in-band, no caller hint)

Decoder reads first 5-bit symbol's bit 0:
- `bit 0 == 1` → chunked mode; consume 32 more bits as chunk header continuation.
- `bit 0 == 0` → single-payload mode; treat bits 4..1 as `paths` flag + version low bits.

### 2.4 Version value selection

The 4-bit `version` field has 16 representable values, but the auto-dispatch design constrains the usable subset:

- A WF-version `V` produces single-payload first-symbol bits 3..0 = binary representation of V. The auto-dispatch reads bit 0 (`v0`); if `v0 = 1`, the symbol is interpreted as chunked, and the WF-redesign decoder will mis-classify a single-payload as chunked.
- Therefore WF-redesign versions must have `v0 = 0` (i.e., even values).
- Additionally, `version = 0` collides with v0.x single-payload; `version = 2` collides with v0.x chunked-misread (see §2.5 below).
- **Usable WF-redesign versions: {4, 8, 12}.** v0.30 uses 4; future major breaks would use 8 then 12.
- After 12 is consumed, the next break requires a format-layer change (e.g., widening the version field to 5 bits, which would itself break the discriminator placement). The 3-version lifetime is intentional.

### 2.5 Safe v0.x rejection (auto-dispatch trace)

| Input | First-symbol bits 4..0 | v0.30 reads | Decoder verdict |
|-------|--------------------------|-------------|-----------------|
| v0.x single-payload (version=0) | `[paths][0][0][0][0]` | bit 0 = 0 → single-payload; version bits 3..0 = `0000` = 0 | `WireVersionMismatch { got: 0 }` |
| v0.x chunked (version=0, chunked=1) | `[0][0][0][1][0]` | bit 0 = 0 → single-payload; version bits 3..0 = `0010` = 2 | `WireVersionMismatch { got: 2 }` |
| v0.30 single-payload | `[paths][0][1][0][0]` | bit 0 = 0 → single-payload; version = 4 | accepted |
| v0.30 chunked | `[0][1][0][0][1]` | bit 0 = 1 → chunked; version bits 4..1 = `0100` = 4 | accepted |

No silent mis-decode is possible.

---

## 3. Bytecode tag space + encoding uniformity

Drivers: Q4, Q7, Q13.

### 3.1 Tag space (Q13 split)

- **Bytecode tags:** 6-bit primary (0x00–0x3F = 64 slots) + 4-bit extension subspace (0x0–0xF = 16 slots). Extension prefix = `0x3F`. Total capacity: 63 primary + 16 extension = 79 operator slots.
- **TLV section tags:** SEPARATE 5-bit primary tag space (matching v0.x). TLV tag values 0x00 (use-site overrides), 0x01 (fingerprints), 0x02 (xpubs), 0x03 (origin-path overrides) keep their v0.x widths. Reserved range and extension semantics in TLV space inherit v0.x design.

Decoder dispatches tag-width based on context: bytecode tree position → 6-bit; TLV section → 5-bit. The split eliminates Q13's +1/TLV-tag cost (the dominant Mode B/C overhead per Phase 3.5b empirical comparison) while preserving the 6-bit headroom in the bytecode space.

### 3.2 Bytecode primary tag allocation

| Hex | Operator | Semantic range |
|-----|----------|----------------|
| 0x00 | `wpkh` | top-level descriptor wrappers |
| 0x01 | `tr` | top-level descriptor wrappers |
| 0x02 | `wsh` | top-level descriptor wrappers |
| 0x03 | `sh` | top-level descriptor wrappers |
| 0x04 | `pkh` | top-level descriptor wrappers (admit-set narrow per `legacy-pkh-permanent-exclusion`; reserved if not admitted) |
| 0x05 | `TapTree` | structure |
| 0x06 | `multi` | multi family |
| 0x07 | `sortedmulti` | multi family |
| 0x08 | `multi_a` | multi family |
| 0x09 | `sortedmulti_a` | multi family |
| 0x0A | `pk_k` | key reference leaves |
| 0x0B | `pk_h` | key reference leaves |
| 0x0C | `c:` Check | miniscript wrappers |
| 0x0D | `v:` Verify | miniscript wrappers |
| 0x0E | `s:` Swap | miniscript wrappers |
| 0x0F | `a:` Alt | miniscript wrappers |
| 0x10 | `d:` DupIf | miniscript wrappers |
| 0x11 | `j:` NonZero | miniscript wrappers |
| 0x12 | `n:` ZeroNotEqual | miniscript wrappers |
| 0x13 | `and_v` | binary ops |
| 0x14 | `and_b` | binary ops |
| 0x15 | `andor` | binary ops |
| 0x16 | `or_b` | binary ops |
| 0x17 | `or_c` | binary ops |
| 0x18 | `or_d` | binary ops |
| 0x19 | `or_i` | binary ops |
| 0x1A | `thresh` | binary ops |
| 0x1B | `after` | timelocks |
| 0x1C | `older` | timelocks |
| 0x1D | `sha256` | hash/literal leaves |
| 0x1E | `hash160` | hash/literal leaves |
| 0x1F | `hash256` | hash/literal leaves (promoted from v0.x extension subspace) |
| 0x20 | `ripemd160` | hash/literal leaves (promoted from v0.x extension) |
| 0x21 | `raw_pkh` | hash/literal leaves (promoted from v0.x extension) |
| 0x22 | `0` False | hash/literal leaves (promoted from v0.x extension) |
| 0x23 | `1` True | hash/literal leaves (promoted from v0.x extension) |
| 0x24–0x3E | reserved | future operators (semantic sub-ranges per Q7) |
| 0x3F | extension prefix | 4-bit sub-code follows |

### 3.3 Encoding uniformity for small unsigned ints (Q4)

- **LP4-ext varint** (cost = 4 + ⌈log₂(value+1)⌉ bits, 4 bits for value=0): TLV section length values, path components, depth counts.
- **Fixed 5 bits**: `k-1` and `n-1` in `multi`/`sortedmulti`/`multi_a`/`sortedmulti_a`/`thresh` body. Bounded fields (`k ≤ n ≤ ~15` for realistic Bitcoin multisigs); fixed-5 wins on bit-efficiency for every realistic value 1..31; ties at value=0 (which doesn't arise; 1-of-1 multisig is not a deployed shape). Saves 11 bits across the 17-wallet validation corpus vs LP4-ext throughout.
- **Fixed 8 bits per byte**: byte arrays (hashes, fingerprints, pubkey halves).
- **Fixed 32 bits**: timelock values (Bitcoin consensus).
- **Fixed `kiw` bits**: key indices (where `kiw = ⌈log₂(n)⌉` per §7).

---

## 4. Multi-family child packing

Driver: Q9.

For parent ∈ {`multi`, `sortedmulti`, `multi_a`, `sortedmulti_a`}, children are encoded as raw `kiw`-bit key indices. No per-child tag (the parent operator implies all children are keys).

```
Tag(6) | k-1(5 fixed) | n-1(5 fixed) | key_index_0(kiw) | ... | key_index_{n-1}(kiw)
```

`Thresh` children remain full `Node` (children can be arbitrary operators); `Thresh`'s `k-1` and `n-1` are also 5-bit fixed.

### 4.1 AST shape

- `Body::MultiKeys { k: u8, indices: Vec<u8> }` — for multi-family operators
- `Body::Variable { k: u8, children: Vec<Node> }` — for `Thresh` only

### 4.2 Wire-shape examples

`sortedmulti(2, @0, @1, @2)` at `kiw=2`: `Tag:6 + k-1:5 + n-1:5 + 3×idx:2 = 22 bits`. (v0.x: 36 bits; saves 14.)

`sortedmulti(2, @0, @1)` at `kiw=1`: `Tag:6 + k-1:5 + n-1:5 + 2×idx:1 = 18 bits`. (v0.x: 27 bits; saves 9.)

`sortedmulti(7, @0..@10)` at `kiw=4` (BIP-388 7-of-11 P2WSH): `Tag:6 + k-1:5 + n-1:5 + 11×idx:4 = 60 bits`. (v0.x: 119 bits; saves 59.)

### 4.3 Decoder branch

In `read_node`, on encountering a tag in the multi-family set, skip per-child tag reads; consume `n` raw `kiw`-bit indices directly.

---

## 5. Walker normalization for `c:pk_k` / `c:pk_h`

Driver: Q12.

### 5.1 Wire invariant

Walker emits bare `Tag::PkK` (or `Tag::PkH`) at every key-check position regardless of context. `Tag::Check` is never emitted wrapping a key leaf on the wire. (`Tag::Check` may still wrap non-key children in valid encodings.)

### 5.2 Renderer reconstruction

When rendering, the renderer detects `c:`-position bare `PkK`/`PkH` and emits `pk(K)` / `pkh(K)` (BIP 379 sugar) or `c:pk_k(K)` / `c:pk_h(K)` (canonical) per output preference.

### 5.3 Wire savings

For `c:pk_k(@N)` previously `Tag::Check(5/6) + Tag::PkK(5/6) + idx(kiw)` → now bare `Tag::PkK(6) + idx(kiw)`. Saves ~5 bits per `c:`-position site. Eliminates wire-shape audit-surface duplication.

---

## 6. Per-`@N` override encoding (Q11 — intentionally not unified)

Retained from v0.x:
- **Origin-path divergence:** signaled by header bit 4 (`paths` flag); per-`@N` block packed inline after path-decl. Dense (covers all `n` paths in one bit).
- **Use-site-path overrides:** TLV tag 0x00, sparse per-override entry.
- **Origin-path overrides:** TLV tag 0x03, sparse.

### 6.1 Why not unified

Unifying both into TLV costs +8 bits per full-divergence encoding (5-bit tag + ≥3-bit length) vs the saved 1-bit header slot. Moving use-site overrides into a dense block loses 6–12 bits per sparse pattern. Empirical analysis (Phase 0a Spike Q11) confirmed that the asymmetric design is structurally optimal for both representative dense and sparse override patterns.

---

## 7. NUMS-sentinel removal (SW3)

### 7.1 Body change

`Body::Tr` adds `is_nums: bool`. `key_index` is present iff `is_nums = false`.

### 7.2 Wire encoding

```
Tag::Tr(6) | is_nums(1) | key_index(kiw, present iff !is_nums) | has_tree(1) | [tree if has_tree]
```

Where `kiw = ⌈log₂(n)⌉` (no widening for sentinel).

### 7.3 Walker / renderer

- Walker sets `is_nums = true` when the internal key matches the BIP-341 NUMS H-point.
- Renderer emits `tr(NUMS_H_POINT_HEX, ...)` when `is_nums = true`; else emits `tr(@N/..., ...)`.

### 7.4 Savings

Saves 1 bit per key index in any descriptor where `n` is a power of 2 (where the v0.x +1 widening was a full extra bit). Adds 1 bit per `tr()` for the flag. Net usually slightly positive; NUMS-internal `tr()` cases additionally save the v0.x sentinel kiw cost.

---

## 8. String-layer / bytecode-layer metadata split (Q5)

**Status quo retained.** Metadata stays in the bytecode layer; the string layer carries only HRP + version-discriminator first symbol + payload + 13-symbol BCH checksum.

Alternative (move metadata to string-layer fields) would require new field delimiters, two-pass decoding, and harm streaming-decoder support. No scenario identified where moving metadata reduces total payload or simplifies the decoder.

---

## 9. TLV section framing (SW2 reverted)

**Status quo retained.** TLV section uses implicit end-of-stream framing via the rollback-as-padding contract (≤7 bits trailing zero). No explicit length prefix.

```
[bytecode header][path-decl][use-site-path][tree][tlv-body...][padding ≤7 bits]
```

### 9.1 Why SW2 was reverted

Phase 3.5b empirical analysis showed the SW2 length prefix added 10–17 bits per descriptor with TLV content (Mode B and Mode C use cases), reducing v0.30's effectiveness against v0.x by 47–118 bits across the 17-wallet validation corpus. Per "most bit-efficient encoding in the circumstances that matter," the SW2 framing benefit (cleaner truncation detection) does not justify the per-descriptor overhead.

### 9.2 Decoder error semantics

TLV truncation indistinguishable from valid trailing padding within the ≤7-bit codex32 padding tolerance — same as v0.x; acknowledged tradeoff.

---

## 10. BCH polynomial layer (Q6 — unchanged from v0.11)

Retain shared BIP-93 polynomial with per-HRP target residues (`crates/md-codec/src/bch.rs:7-17`). Per-HRP-residue + HRP-mixing is sufficient for the hand-transcription threat model where user errors dominate over cryptographic attacks. Cross-repo coordination burden of distinct polynomials per HRP outweighs the formal cryptographic gain at this threat tier (per Phase 0a Spike Q6).

---

## 11. Decoder error taxonomy + well-formedness invariants

### 11.1 Error categories

| Error | Trigger |
|-------|---------|
| `WireVersionMismatch { got: u8 }` | Single-payload or chunked version field ≠ accepted v0.30 value (4 in this release) |
| `MalformedHeader { detail }` | Chunked-flag inconsistent with caller context; chunk header out-of-range fields; chunk version ≠ payload version |
| `TagOutOfRange { primary: u8 }` | 6-bit primary in reserved range 0x24–0x3E; extension sub-code unrecognized |
| `BCHResidueMismatch` | BCH verify returns false (HRP mismatch, transcription error, wrong format) |
| `OperatorContextViolation { tag: Tag, context: ContextKind }` | Tag in forbidden position. `ContextKind` enum: `TopLevel` (e.g., bare `PkK` as top-level descriptor), `TapLeaf` (e.g., `Multi` instead of `MultiA` inside tap leaf), `MultiBody` (non-key tag among multi-family children) |
| `NUMSSentinelConflict` | Inside `tr()` body: `is_nums=0` with `key_index ≥ n` |
| `DecodeRecursionDepthExceeded { depth: usize, max: usize }` | Recursion depth ≥ 128 |
| `PlaceholderIndexOutOfRange { idx: u8, n: u8 }` | Key index ≥ `n` (non-NUMS context; narrowed trigger per SW3) |

### 11.2 Well-formedness invariants

1. **Version**: first 5-bit symbol's bits 3..0 (single-payload) or bits 4..1 (chunked) = WF version (4); else `WireVersionMismatch`.
2. **Tag range**: every primary tag in 0x00–0x23 maps to an allocated operator; 0x24–0x3E primary → `TagOutOfRange`; 0x3F triggers extension subcode read.
3. **Multi-family body**: exactly `n` raw `kiw`-bit key indices ∈ `[0, n)`; no child tags; any tag → `OperatorContextViolation { context: MultiBody }`.
4. **NUMS isolation**: `is_nums` flag exists only in `Body::Tr` (structurally absent elsewhere). In-`tr()` overflow `is_nums=0 ∧ key_index ≥ n` → `NUMSSentinelConflict`.
5. **TLV framing**: TLV body consumes all remaining bits via the rollback-as-padding contract; decoder reads TLV entries sequentially until ≤7 trailing bits remain (treated as codex32 padding).
6. **Sparse TLV ordering**: within each TLV entry, `@N` indices strictly ascending.
7. **Depth cap**: recursion depth < 128.
8. **Chunk consistency**: all chunks in a set share `version`, `chunk-set-id`, `count-1`; indices form complete `0..count-1`; reassembled `chunk-set-id` matches wire value.

---

## 12. BIP-draft impact (post-implementation deliverable)

Sections of `bip/bip-mnemonic-descriptor.mediawiki` invalidated by v0.30 (full rewrite required during Phase I per `IMPLEMENTATION_PLAN_v0_30.md`):

| BIP section | Invalidation reason |
|-------------|---------------------|
| `====Header====` (line 178) | §2: new 4-bit version, bit-0 discriminator |
| `====Checksum====` + `====Length envelope====` (lines 217, 261) | §2: first-symbol structure change |
| `====Bytecode header====` (line 297) | §2 |
| `=====Tag table=====` (line 382) | §3: 6-bit primary space, new slot assignments |
| `=====Operator children layout=====` (line 481) | §4: raw key indices |
| `=====tr() NUMS sentinel rule (v0.18)=====` (line 465) | §7: replaced by `is_nums` flag |
| `=====Key references=====` (line 485) | §7: new `kiw` formula |
| `====LP4-ext varint====` (line 605) | §3: Q4 lock — LP4-ext for variable-range fields; fixed-5 retained for bounded `k`/`n` |
| `====Chunk header (37 bits)====` (line 694) | §2: chunked-flag moves to bit 0 |
| `=====Bit-layout example=====` (line 543) | All bit-layout examples need full regen with new bit widths |
| `===Decoder requirements===` (line 783) | §2, §11: new version field + error taxonomy |

Sections **partially affected** (amendment sufficient):
- `====Round-trip canonical form====` (line 679) — Q12 walker normalization invariant
- `===Decoder reporting===` (line 802) — new error categories
- `==Test Vectors==` (line 1000) — all schema vectors invalidated; full regen

Sections **NOT affected** despite touching wire layer:
- `====TLV section====` + `=====End-of-section detection=====` (lines 491, 529) — SW2 reverted; rollback-as-padding contract retained from v0.x; BIP wording stays as-is.
- `=====TLV tag allocations=====` (line 501) — TLV tags retained at v0.x 5-bit width per Q13 split.

---

## 13. Empirical validation (17-wallet × 3-mode comparison)

Validation corpus: 12 Claude-proposed basic wallets + 5 user-supplied complex shapes (recovery patterns with `andor`, `or_i`, `thresh`, `pkh`, `after`, `older`). See `design/agent-reports/spike-v0.30-q9-pre-spec.md` and the Phase 3.5/3.5b/3.6 sections of the originating plan file (`/home/bcg/.claude/plans/typed-rolling-spindle.md` § Phase 3.5b–3.6) for full per-wallet bit and char counts.

| Mode | v0.x corpus bits | v0.30 corpus bits | Δ corpus bits |
|------|-------------------|--------------------|----------------|
| A (template + paths) | 2,893 | 2,734 | **−159** |
| B (+ fingerprints TLV 0x01) | 5,175 | 5,003 | **−172** |
| C (+ xpubs TLV 0x02) | 35,829 | 35,647 | **−182** |

v0.30 wins in all three modes. No wallet costs more chars under v0.30 in Mode A. Edge cases in chunked-multi-string Mode C (#8/#10/#16/#17) have small +1/+2/+4/+6 char regressions due to chunk-byte-boundary realignment despite bit savings; these affect only the non-engraving Mode C use case.
