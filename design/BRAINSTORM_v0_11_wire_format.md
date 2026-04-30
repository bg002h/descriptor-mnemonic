# Brainstorm — v0.11 wire-format redesign (partial, in-progress)

> **Status:** Foundation sub-area mostly complete; sub-areas 2-4 not yet started. Decisions tagged ✅ are locked; ⏸ are deferred to a later brainstorm pass; 🟡 are open questions captured here for the eventual spec.
>
> **Output target:** v0.11 ships next (after this brainstorm → SPEC → PLAN → implement). v1.0 promotion later after a soak period.

---

## 0. Goal & framing

Redesign the md-codec wire format from-scratch where v0.x choices are accumulated baggage. Pre-v1.0 break freedom is real — zero current users, so wire-format break in v0.11 is a free move.

**Optimization target (relative, not absolute):**
- Shortest reasonable engravings for common wallet types (BIP 84 single-sig, BIP 86 taproot, 2-of-3 BIP 48 multisig with shared origin path)
- Pay-for-what-you-use: rare features (divergent origins, multipath-shared `@N`, fingerprints, etc.) cost extra; common cases pay zero
- Sane length limits where natural (max placeholders, max path components, max multipath cardinality)
- Common cases no worse than v0.10; uncommon cases free to be slightly longer if they buy cleaner v1.0 design

**Goal:** full BIP 388 wallet descriptor template grammar coverage. **No deliberate scope cuts.** If BIP 388 admits it grammatically, md-codec encodes it.

---

## 1. Locked carry-overs (kept from v0.x)

These are the load-bearing pieces from v0.x that v0.11 inherits unchanged:

| Item | What | Status |
|---|---|---|
| codex32 BCH layer | BIP 173-style HRP-mixing + BIP 93 polymod algorithm + per-format target residues | ✅ locked |
| Polynomial parameters | `POLYMOD_INIT`, `GEN_REGULAR`, `GEN_LONG`, `REGULAR_SHIFT`/`MASK`, `LONG_SHIFT`/`MASK` | ✅ locked |
| `shibbolethnums` NUMS preimage | `SHA-256("shibbolethnums")` derives `WDM_REGULAR_CONST` (top 65 bits) and `WDM_LONG_CONST` (top 75 bits). Symbols may be renamed `MD_*` per FOLLOWUPS `wdm-symbol-rename-md`; values locked. | ✅ locked |
| Header byte with version field | Format reserves a version field at the top of the encoded payload; specific bit allocation 🟡 (see Foundation Q5). Pre-v1.0 sandbox: version=0 throughout v0.x including v0.11. | ✅ locked |
| HRP `md` | Two-character human-readable prefix | ✅ locked |

---

## 2. Vocabulary (industry-aligned)

**Discipline:** don't invent words; adopt BIP 388 / BIP 380 / BIP 32 / miniscript terminology where it exists. Coined md-codec terms (`shape`, `instance`, `Type 0/Type 1 PolicyId`, `WalletId`) are explicitly dropped.

| Term | Meaning | Source |
|---|---|---|
| **Wallet descriptor template** (or just **template**) | The `@N`-placeholder form. | BIP 388 verbatim |
| **Wallet policy** | Pair of (template + key information vector). The full thing. | BIP 388 verbatim |
| **Key information vector** / **key info** | Array of concrete xpubs in a wallet policy. | BIP 388 verbatim |
| **Output descriptor** / **descriptor** | BIP 380 form — fully resolved, no `@N`. What `WalletPolicy::into_descriptor()` produces. | BIP 380 verbatim |
| **Account** | BIP 32 account level. One wallet policy = one account. | BIP 32 / 44 verbatim |
| **Policy** (lowercase, naked) | **Reserved** for miniscript Policy. NOT to be used for BIP 388 things. | miniscript / BIP 388 line 45 explicit reservation |
| **Key placeholder (KP)** / **key index (KI)** | The `@N/<...>/*` syntax / the `i` in `@i`. | BIP 388 verbatim |

**v0.11 identifier names** (replacing v0.x's `PolicyId`):

- `WalletDescriptorTemplateId` — hash over canonical wallet-descriptor-template bytes. (What current `PolicyId` actually computes; renamed in v0.11.)
- `WalletPolicyId` — hash over canonical wallet-policy bytes (template + key information vector). Computed only when v0.11 admits wallet-policy encoding (see Foundation Q5).

The Type 0 / Type 1 PolicyId typology shipped in v0.10 is dropped — it was md-codec coinage, not industry-standard. Direct names suffice.

**Closes (on v0.11 ship):** `wallet-id-is-really-template-id` FOLLOWUPS entry.

---

## 3. Foundation sub-area decisions

### Q1 — Header version field for v0.11

✅ **Pre-v1.0 sandbox, version stays at 0.** Don't accept inherited bit allocation as baggage; version=0 throughout v0.x. v1.0 promotion later will introduce stable version-field semantics. Same byte-layout slot, no commitment to bit boundaries yet.

### Q2 — Feature/extension model

✅ **Hybrid: hardwired mode bits + tail-TLV section for extensions.** (Option D in the brainstorm Q&A.) Specifically D1-flavored: zero hardwired bits *for optional features* (everything optional goes through TLV); a small number of hardwired bits *for mode flags* (shape vs policy, all-keys-same-path). The two are conceptually distinct — mode flags affect how the rest of the encoding is interpreted; extensions are append-only optional data.

**TLV section:** lives at the tail of the encoded payload (after the tree). Implicit start — codex32 layer provides total length, parser walks header → path-decl → tree, remaining bytes are TLV section.

**TLV entry format:** to be specified — (tag, length, value) tuples; tag width and length encoding are deferred to spec phase.

**Forward-compat property:** decoders skip unknown TLV tags via length-prefix advancement. Adding a new feature tag is not a wire-format break for existing decoders (they'll skip it). This is the key win of TLV vs flag-bit-per-feature.

### Q3 — Bit alignment

✅ **Fully bit-aligned wire format.** (Option C in the brainstorm Q&A.) Tags, indices, counts, and small-int fields are bit-width-optimized rather than byte-padded. Approximate 22% common-case length reduction vs byte-aligned.

**Concrete savings for representative cases:**

| Case | Byte-aligned (v0.10) | Bit-aligned (v0.11) |
|---|---|---|
| BIP 84 single-sig | 24 codex32 chars total | 21 chars total |
| BIP 86 taproot single-sig | 24 chars | 21 chars |
| 2-of-3 BIP 48 multisig (shared path) | 37 chars | 29 chars |
| Same + 3 master fingerprints | 60 chars | ~50 chars |
| 2-of-4 with divergent origins | 47 chars | ~35 chars |

### Q4 — Tag space width

✅ **(C-prefix): 5-bit primary tags + 5-bit extension prefix.** Primary tag space holds the common ops (32 values); a designated "extension prefix" tag value indicates "next 5 bits are a secondary tag" for rare/wrapper ops. Common encodings stay 5-bit-tag width; rare encodings pay 10 bits per tag.

**Why this works:**
- 7 wire-distinct wrappers (Alt, Swap, Check, DupIf, NonZero, ZeroNotEqual, Verify) — `t:`, `l:`, `u:` are syntactic sugar that desugar to existing AST nodes
- Top-level constructors: 5 (`pkh`, `wpkh`, `sh(wpkh)`, `wsh`, `tr`)
- Miniscript fragments: 19
- Framings: 3-5 (placeholder-ref, path-decl, TLV-tag, possibly more)
- Total ~34-36 wire-distinct tags. Tightening common ones into 5-bit primary, deferring 4-6 rare ones to 10-bit extension form, fits comfortably.

**Concrete tag-space allocation deferred to spec phase.** This brainstorm just commits to the (C-prefix) shape.

### Q5 — Header bit allocation

✅ **5-bit header = 3-bit version + 2 mode flags.**

```
bit 4   bit 3   bit 2   bit 1   bit 0
[paths] [s/p]   [version (3 bits, 8 generations)]
```

- **Bits 0-2 (3 bits, 8 values):** version field. v0.11=0, v1.0=1, etc. 8 generations is plenty for the foreseeable future — major format breaks should be rare.
- **Bit 3:** shape vs policy. `0` = wallet descriptor template (placeholder-only); `1` = wallet policy (template + key information vector embedded).
- **Bit 4:** all-keys-same-path. `0` = shared-path (one path applies to all `@N`); `1` = divergent paths (per-`@N` paths follow). Common case is `0` — saves one tag in the path-declaration position.

**Common case** (BIP 84 single-sig, 2-of-3 BIP 48 shared, etc.): `header = 0b00000` (shape-encoding, shared path, version 0) — single 5-bit symbol = `q` in codex32. Predictable, recognizable.

**Mode flag rationale:**
- **shape vs policy:** wallet descriptor template (placeholder-only) vs full wallet policy (template + embedded key information vector). The latter is much larger payload (xpubs are ~78 bytes each); affects encoding and engraving cost dramatically. Whether v0.11 actually emits wallet-policy form (bit=1) is a Sub-area 4 question; allocating the bit now keeps the option open.
- **all-keys-same-path:** common case (every `@N` shares one origin path) vs divergent (per-`@N` origin paths). The bit dispatches the path-declaration block format. Saves a tag in the common case (~1 byte / ~2 chars on every shared-path encoding).

**Future mode bits** (e.g., partial-wallet, wallet-collection): would require a v1.0+ version bump + reallocation. 3-bit version space is the budget for major reformations.

---

## 4. Sub-areas 2-4 (not yet started)

The following are queued for future brainstorm passes once Foundation is fully nailed down:

### Sub-area 2 — Template structure
- Concrete tag-space allocation (which 32 primary 5-bit values map to which ops)
- Operator encoding (wrappers, conjunctions, disjunctions, thresholds)
- Key placeholder reference encoding (`@N` syntax in bytes)
- Tree shape encoding (variable-arity ops, count fields)
- Path-declaration block format(s) for the shared and divergent cases
- Field width conventions for indices, counts, k/n in bit-aligned form
- Variable-length value encoding (varints in bit-aligned form)

### Sub-area 3 — Per-key dimensions (the v0.10/v0.11 multipath gap closure)
- Origin paths: per-`@N` divergence (resolved by v0.10 OriginPaths block; v0.11 may streamline)
- Multipath suffixes: per-AST-position support to admit BIP 388's full multipath grammar (the v0.10 limitation `to-bytecode-multipath-shared-at-n-set-key-info-mismatch`, now subsumed into v0.11 redesign)
- Wildcards: `*` (unhardened) vs `*'` (hardened) per AST position
- Fingerprints: optional per-`@N` master-key fingerprints
- Unified design: how these dimensions compose without four separate optional blocks each consuming a tag

### Sub-area 4 — Identity & chunking
- `WalletDescriptorTemplateId` computation (canonical-bytes hash; SHA-256 truncation)
- `WalletPolicyId` computation (if v0.11 admits wallet-policy encoding)
- chunk-set-id / chunking semantics (how a multi-card backup partitions the encoded form)
- 12-word phrase rendering (existing PolicyId word list — likely renamed `WalletDescriptorTemplateId phrase`)
- NUMS-derived bit allocations for chunk-set-id, etc.
- Engraving layout / segmentation rules

---

## 5. Display conventions

✅ **HRP at front.** Every codex32 string starts `md1...`. HRP `md` immediately followed by separator `1`. Convention pinned for spec rigor.

✅ **Visual separator between payload and checksum.** Implementations SHOULD insert `-` or ` ` between payload and checksum for human-facing display (CLI, GUI, engraving template, printed backup). Both characters are non-codex32-alphabet so parsers strip cleanly.

**Parser requirement:** decoders MUST tolerate codex32 strings with whitespace and `-` separators stripped on input. SHOULD on output, MUST on input.

**Engraving layout templates** are deferred to Sub-area 4 brainstorm — chunking, line-wrap, multi-card layouts, where HRP appears on each card vs once total, etc.

---

## 6. Future considerations (post-v0.11)

- **Run-length encoding / compression.** Worth a future brainstorm pass — opportunities exist for repeated paths in OriginPaths-equivalent blocks, repeated keys in multipath-shared-`@N`, and recurring operator subtrees. Defer to v0.12 or post-v1.0.
- **Aggressive dictionaries.** Beyond v0.x's path dictionary, future versions could add multipath dictionaries, operator-combination codes (`wsh(sortedmulti(...))` as one tag), template-shape codes (whole-template single-byte code for BIP 84 single-sig, etc.). The aggressive variant could shrink common cases by another 30-50%. Trade-off: dictionary becomes a versioned spec object requiring careful curation.
- **Wallet policy encoding** (with embedded xpubs). v0.x only encodes templates; v0.11 may stay template-only. Future versions could admit encoding the full wallet policy for "engrave the whole wallet" use cases.

---

## 7. Open questions for spec phase

These are answered-in-principle but need concrete spec-time decisions:

1. **Concrete 5-bit primary tag-space allocation.** Which ops get which 5-bit codes? Frequency analysis of expected wire output should drive this.
3. **TLV entry format details.** Tag width (5-bit primary + extension? own prefix?), length encoding (varint? fixed?), terminator vs section header.
4. **Variable-length value encoding** in bit-aligned form. Bit-aligned varint (continuation-bit idiom)? Or different encoding?
5. **Path-declaration block format(s).** Shared case (one path, dictionary indicator or explicit-form). Divergent case (per-`@N` paths). How they're distinguished structurally given the all-keys-same-path header bit.
6. **Multipath-shared-`@N` block format.** Sparse vs dense, how it interacts with origin-paths and wildcards. (Sub-area 3.)
7. **Chunking and identity.** (Sub-area 4.)

---

## 8. Process notes

- Each sub-area gets its own brainstorm pass before spec writing.
- Spec phase produces `design/SPEC_v0_11_wire_format.md` (or split per sub-area if too long).
- Implementation plan in `design/IMPLEMENTATION_PLAN_v0_11_wire_format.md` (or per sub-area).
- Per-phase opus reviewer gate workflow for the implementation phases (mirror of v0.10's playbook).
- Phased subagent-driven development with TDD discipline per existing project conventions.

**Effort estimate:** roughly 2-3× the v0.10 effort due to wire-format-break magnitude (full bit-aligned re-encoder/-decoder, full BIP 388 grammar coverage, tag space re-allocation, identity rename, BIP draft re-write).

---

## 9. Decision log

| # | Decision | Status | Captured |
|---|---|---|---|
| D1 | v0.11 label (not v1.0 directly); v1.0 promotion later after soak | ✅ | §0 |
| D2 | Vocabulary: BIP 388/380 verbatim; drop shape/instance/Type 0/Type 1 coinage | ✅ | §2 |
| D3 | `PolicyId` renamed to `WalletDescriptorTemplateId` in v0.11 | ✅ | §2 |
| D4 | Drop Type 0/Type 1 PolicyId typology | ✅ | §2 |
| D5 | Header version stays at 0 for v0.11 (pre-v1.0 sandbox) | ✅ | §3 Q1 |
| D6 | Extension model: hybrid hardwired mode bits + tail-TLV (D + D1 flavor) | ✅ | §3 Q2 |
| D7 | Fully bit-aligned wire format | ✅ | §3 Q3 |
| D8 | Tag space: 5-bit primary + 5-bit extension prefix (C-prefix) | ✅ | §3 Q4 |
| D9 | Header layout: 3-bit version + bit 3 shape/policy + bit 4 all-keys-same-path | ✅ | §3 Q5 |
| D10 | HRP at front of codex32 string | ✅ | §5 |
| D11 | Visual separator between payload and checksum (display convention) | ✅ | §5 |
| D12 | Mode flags: shape vs policy + all-keys-same-path | ✅ | §3 Q5 |
| D13 | RLE/compression deferred to future brainstorm | ⏸ | §6 |
| D14 | Full BIP 388 grammar coverage (no deliberate scope cuts) | ✅ | §0 |
