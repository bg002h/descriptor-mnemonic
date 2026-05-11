# Phase I — code review r1 (2026-05-10)

**Working tree:** dirty at HEAD `e86fb78`; not yet committed.

**Scope:** md-codec v0.30 Cycle 5 Phase I — BIP draft rewrite per SPEC §12. 12 sections fully rewritten + 3 partially amended; 6 worked bit-by-bit examples regenerated; 25 v0.x prose occurrences purged (0 active remaining; 8 preserved as historical context). 1099 → 1168 LOC (+230/-161; 391 LOC churn).

**Files reviewed:** `bip/bip-mnemonic-descriptor.mediawiki`, `crates/md-codec/src/{tag, header, encode, tree, error}.rs`, `design/SPEC_v0_30_wire_format.md` §12.

---

## Critical (block ship) — FIXED INLINE

### C-1 — `Error::PolicyScopeViolation` cited normatively at 4 BIP sites but does not exist in `error.rs`

- **Where:** `bip/bip-mnemonic-descriptor.mediawiki:105, 128, 504, 673` (pre-fix).
- **What:** The variant `Error::PolicyScopeViolation(String)` is mentioned as the canonical rejection error for top-level scope violations, sh-wrapper scope violations, bare(SCRIPT) descriptors, and tap-tree depth overflow. None of these match an actual variant in `crates/md-codec/src/error.rs`. Cross-implementations following the BIP MUST clauses literally would produce a variant the reference implementation never emits.
- **Fix (applied):**
  - Line 105 (Top-level scope rejection): reframed to cite actual variants (`Error::TagOutOfRange`, `Error::ForbiddenTapTreeLeaf`, `Error::DecodeRecursionDepthExceeded`) + note that pre-decode parse/walker rejections are reference-implementation-specific (md-cli surface, not md-codec).
  - Line 128 (Sh inner-tag matrix): replaced specific variant cite with "MUST reject with a structured error per §Top-level descriptor scope".
  - Line 504 (Bare descriptor): "rejects it as PolicyScopeViolation" → "rejects it at the parse layer per §Top-level descriptor scope".
  - Line 673 (TapTree depth overflow): "MUST reject ... with Error::PolicyScopeViolation" → "MUST reject ... with `Error::DecodeRecursionDepthExceeded { depth, max: 128 }`" (the actual variant).

### C-2 — `Error::BCHResidueMismatch` cited in error taxonomy table but does not exist

- **Where:** `bip/bip-mnemonic-descriptor.mediawiki:859`.
- **What:** The error taxonomy table cited `BCHResidueMismatch` as the canonical BCH-failure variant. The actual variant in `crates/md-codec/src/error.rs:245` is `Codex32DecodeError(String)`.
- **Fix (applied):** Row updated to `Codex32DecodeError(String)` with note that BCH residue mismatch is one of several conditions it covers (alongside HRP mismatch, etc.).

## Important (must fix before ship)

None blocking.

## Low (file as FOLLOWUP — addressed inline)

### L-1 — Error taxonomy table omitted 4 MUST-distinguishable variants (FIXED INLINE)

- **Where:** Error taxonomy table (originally lines 850–868).
- **What:** Table claimed to list all "structurally-defined error categories" but omitted `PathDepthExceeded`, `KGreaterThanN`, `TlvOrderingViolation`, `ChunkSetIdMismatch` — all of which are MUST-reject conditions in the SPEC.
- **Fix (applied):** Added 4 rows with field shapes pulled from `crates/md-codec/src/error.rs`.

## Low (filed as FOLLOWUP — pre-existing scope)

### Filed: `v0.30-phase-i-tag-rs-operator-count-off-by-one`

- **Where:** `crates/md-codec/src/tag.rs:3`.
- **What:** Module doc-comment says "35 operators in primary 6-bit space (0x00..=0x23)" but the range `0x00..=0x23` is 36 slots; the BIP rewrite at line 403 correctly states 36. tag.rs is pre-existing (predates Phase I); fix deferred per "don't refactor beyond scope" memory.

## Nit (no action)

### N-1 — TLV length varint nibble grouping (line 637)

LP4 layout `0111 1000010` for the 11-bit `[L:4][payload:7]` field has a space at the boundary; visually correct but could confuse readers. Not blocking.

### N-2 — `wsh_multi_chunked` corpus vector framing

The chunked example was generated via encoder override, not by payload-size triggering. A comment in the BIP test-vector section would clarify, but the corpus filename `wsh_multi_chunked` already conveys intent.

---

## Verdict

**Ship** (C-1, C-2 fixed inline; L-1 fixed inline; tag.rs FOLLOWUP filed). All 6 bit-layout examples cross-verified with `md encode`; tag table maps to `crates/md-codec/src/tag.rs` exactly; kiw formula and NUMS flag prose are SPEC-faithful; historical-context labeling is correct (8 preserved mentions all in clearly-marked history blocks).
