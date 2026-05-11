# Phase B code review — r1 (sign-off)

**Agent:** feature-dev:code-reviewer
**Date:** 2026-05-10
**Working tree:** uncommitted on `main` (parent = `0e93040` / Phase A commit)
**Files reviewed (4):**

- `crates/md-codec/src/header.rs` (~85 lines changed)
- `crates/md-codec/src/chunk.rs` (~67 lines changed)
- `crates/md-codec/src/error.rs` (~23 lines changed)
- `crates/md-codec/src/encode.rs` (~2 lines changed — scope expansion justified)

---

**Summary.** Phase B is well-executed and correctly implements the 4-bit version field, in-band chunked-flag layout, and `WireVersionMismatch` rejection per SPEC §2. Two issues were found: one stale doc-comment in `error.rs` describing the old 3-bit layout (Low), and several pre-existing v0.11 references in `lib.rs`, `codex32.rs`, etc. that are outside Phase B scope (pre-existing, not flagged). The two `version: 0` hits in `json.rs` are test-only struct literals exercising JSON serialization format, not wire encoders — no issue. Workspace orphan-caller checks pass cleanly on all four grep criteria.

---

**Findings table.**

| Priority | Finding | Where (file:line) | Recommendation |
|---|---|---|---|
| Low | `ChunkHeaderChunkedFlagMissing` doc-comment says "3-bit version field in a chunk header (see `chunk.rs` / spec §9.3)". The version field is now 4 bits per SPEC §2.2, and the current normative spec reference is §2.2 not §9.3. This is a stale doc-comment on a pre-existing variant that Phase B preserved unchanged. Not a blocker but is factually wrong post-Phase B. | `crates/md-codec/src/error.rs:207` | Change to "The chunked-flag bit follows the 4-bit version field in a chunk header (see `chunk.rs` / SPEC v0.30 §2.2) and MUST be 1." |
| Nit | `encode.rs` module doc (line 11) still reads "Top-level descriptor parsed/built from a v0.11 wire payload." Phase B's scope expansion to `encode.rs` touched only line 83 (the `Header { version: 0 }` literal); the module-level doc on `Descriptor` was not updated. Phase J owns the crate-level doc sweep, so this is correctly deferred — noting for FOLLOWUPS. | `crates/md-codec/src/encode.rs:11` | Defer to Phase J (crate-level doc sweep). File in FOLLOWUPS. |

---

**Detailed SPEC conformance verification (all pass):**

1. **header.rs §2.1**: `write` emits `(divergent_paths << 4) | (version & 0b1111)` = `[paths][v3][v2][v1][v0]` — correct. `read` extracts `bits & 0b1111` for version, `(bits >> 4) & 1` for paths — correct.

2. **header.rs §2.4**: `WF_REDESIGN_VERSION: u8 = 4` — correct.

3. **header.rs §2.5**: `WireVersionMismatch` triggered on `version != 4`. Reserved-bit check fully removed. 4-bit mask is `0b1111` not `0b0111` — correct.

4. **chunk.rs §2.2**: `write` emits `version(4 bits)` then `1(1 bit)` = `[v3][v2][v1][v0][chunked]` MSB-first — correct order. Old reserved-bit write (`write_bits(0, 1)`) is gone. `split()` uses `Header::WF_REDESIGN_VERSION` — correct.

5. **chunk.rs §2.5**: `read` checks version before chunked-flag — `WireVersionMismatch { got: 0 }` fires on v0.x chunk input before `ChunkHeaderChunkedFlagMissing` could fire — correct ordering.

6. **§2.5 rejection trace coverage:**
   - Row 1 (`got: 0`): `header_rejects_version_mismatch` arm 1, wire byte `0x00` — correct.
   - Row 2 (`got: 2`): `header_rejects_version_mismatch` arm 2, wire byte `0x10` — correct. The comment deconstructs `[0][0][0][1][0]` → version bits 3..0 = `0b0010 = 2` accurately.
   - Chunk-header path (`got: 0`): `chunk_header_rejects_v0x_version` uses `BitWriter` to construct the exact 37-bit wire (`version=0, chunked=1`) — correct; `assert_eq!(w.bit_len(), 37)` pins the construction.
   - Round-trip tests use `version: Header::WF_REDESIGN_VERSION` (=4) — confirms read-write symmetry on v0.30 inputs.

7. **error.rs §11.1**: `WireVersionMismatch { got: u8 }` — exact name and field match. `MalformedHeader { detail: String }` — exact name and `String` type match (runtime context composability preserved for Phase G). `ReservedHeaderBitSet` and `UnsupportedVersion` — zero hits across workspace.

8. **encode.rs scope expansion**: Change is minimal — only `version: 0` → `Header::WF_REDESIGN_VERSION` at line 83. No other version=0 / 3-bit / reserved-bit logic in `encode.rs`. The `kiw` formula change is correctly deferred to Phase F/SW3. Scope expansion is justified.

9. **Workspace orphan checks**: `ReservedHeaderBitSet` — 0 hits. `UnsupportedVersion` — 0 hits. `V0_11_VERSION` — 0 hits. `version: 0` in `crates/` — 2 hits, both in `md-cli/src/format/json.rs` test-only struct literals that construct `Header`/`ChunkHeader` solely to test JSON field serialization; they do not pass through `Header::write` or emit wire bytes. Not a wire correctness issue.

---

**Verdict: SHIP**
