# Phase 4 Decision Log

Living document of decisions made during autonomous execution of Phase 4 (Chunking + Wallet ID). Each entry: what was decided, why, alternatives considered, where to verify in code.

## Open questions for review

(Populated as design questions arise that I picked a default for. Empty = no open questions.)

---

## Decisions made

### D-1 (Phase 4 scope and ordering)

**Context**: `design/IMPLEMENTATION_TASKS_v0.1.md` lines 2147–2158 list 10 subtasks for Phase 4 (4.1–4.10). Following Phase 3's pattern of bundling tightly-coupled subtasks for review economy.

**Decision**: Phase 4 executes in 5 task units, in dependency-flow order:

1. **4-B (plan 4.3+4.4+4.5+4.6): WalletId family** in `wallet_id.rs` — `WalletId([u8;16])`, `WalletIdWords([String;12])`, `ChunkWalletId(u32)` (moved from `error.rs` placeholder), `truncate`/`to_words`. **First** because `ChunkWalletId` is used by 4-A and 4-E, and `WalletId` is used by 4-C.
2. **4-A (plan 4.1+4.2): ChunkHeader + codec** in `chunking.rs` — struct with `version`/`type`/`wallet_id`/`count`/`index`; `to_bytes`/`from_bytes`. Uses `ChunkWalletId` from 4-B.
3. **4-C (plan 4.7): compute_wallet_id** — `pub fn compute_wallet_id(canonical_bytecode: &[u8]) -> WalletId` = `SHA-256(bytecode)[0..16]`. Phase-4 surface takes bytes (the `WalletPolicy`-aware wrapper is Phase 5 surface per IMPLEMENTATION_PLAN.md §3 line 276).
4. **4-D (plan 4.8): chunking_decision** — `ChunkingPlan` enum + decision function per BIP §"Chunking" capacities (single-string ≤56 B long, else chunked ≤45 B regular / ≤53 B long, max 32 chunks). Independent of other 4-* tasks.
5. **4-E (plan 4.9+4.10): chunk_bytes + reassemble_chunks** — inverse pair. `chunk_bytes` produces fragment-bearing chunks; `reassemble_chunks` validates and concatenates with cross-chunk SHA-256[0..4] check.

**Rationale**: The dependency-flow order minimizes "use stub, replace later" churn. 4-B builds the foundation; 4-A/4-C consume it; 4-D is independent (could go anywhere); 4-E composes everything.

**Out of scope (explicitly deferred to Phase 5)**:
- `WalletIdSeed([u8; 4])` user-controlled override (P5 — `EncodeOptions` plumbing)
- `WalletPolicy`-aware `compute_wallet_id` wrapper (P5)
- `WdmBackup` / `EncodedChunk` / `DecodeReport` types (P5)
- Codex32 string wrapping of chunks (P7 + Phase 1's encoding.rs)

### D-2 (Phase 4 dep policy): `bip39` crate added for `WalletIdWords::to_words`

**Context**: Phase 4-B converts the 16-byte `WalletId` to 12 BIP-39 words for the Tier-3 Wallet ID. Two options were considered:

- **(a)** Add `bip39 = "2"` as a dep (validated wordlist, ~1 KLOC saved)
- **(b)** Embed the 2048-word English wordlist as a `const &[&str; 2048]` (no new dep, ~25 KB binary growth)

**Decision**: Option (a). User-confirmed via `/effort` continuation prompt.

**Rationale**: `bip39` is a small, well-maintained crate with the exact functionality needed. Embedding the wordlist would duplicate canonical reference data and be harder to keep in sync with the BIP 39 spec. Tradeoff accepted: one more transitive dependency.

**Verify in code**: `crates/wdm-codec/Cargo.toml` `[dependencies]` section gains `bip39 = "2"`. `crates/wdm-codec/src/wallet_id.rs` calls into the crate's mnemonic conversion API.

---

(More decisions appended as Phase 4 progresses.)

---

## Tasks completed

| Task | Commit (feat / fix) | Status notes |
|------|---------------------|--------------|
| 4-B WalletId family | `f4519ef` (no fix needed) | `WalletId([u8;16])` + `WalletIdWords([String;12])` + `ChunkWalletId(u32)`; `truncate` uses big-endian first-20-bits packing `(b[0]<<12)|(b[1]<<4)|(b[2]>>4)`; `ChunkWalletId::new` uses release-checked `assert!` (not debug_assert) for wire-format integrity; `bip39 = "2"` added; placeholder removed from `error.rs` and re-exports cleaned up in `lib.rs` per option (b); 10 new tests (211 → 221). First crate commit to include `Cargo.lock` (workspace has `[[bin]]` targets, `.gitignore` doesn't exclude). Code-review minor follow-ups (not blocking, address opportunistically): (a) duplicate-assertion in `wallet_id_display_is_hex` (the 34-char literal sliced to 32) — drop one assertion or use a 32-char literal; (b) consider `WalletIdWords::as_slice(&self) -> &[String; 12]` for borrow-iteration; (c) collapse `WalletId::new` and `From<[u8;16]>` to a single definition site. |
| 4-A ChunkHeader + codec | `aefdf3f` (no fix needed) | `#[non_exhaustive] enum ChunkHeader` with `SingleString { version }` and `Chunked { version, wallet_id, count, index }` variants; `to_bytes` produces 2 or 7 bytes; `from_bytes(&[u8]) -> Result<(Self, usize), Error>` performs all validation before `Ok` (length → version → type → length-2 → wallet_id high-bits-zero → count 1..=32 → index < count); 4 new top-level `Error::*` variants (`InvalidChunkCount`, `InvalidChunkIndex`, `InvalidWalletIdEncoding`, `ChunkHeaderTruncated`); avoids panic on untrusted input by checking `bytes[2] & 0xF0` before calling `ChunkWalletId::new`; 16 new tests (221 → 237). Follow-ups (defer to 4-E or beyond): (a) add named truncation tests for chunked-but-2-byte / chunked-but-5-byte (current code is correct by inspection but missing those exact lengths from coverage); (b) add `{ have, need }` payload to `ChunkHeaderTruncated` for diagnostics; (c) consider renaming `InvalidWalletIdEncoding` → `ReservedWalletIdBitsSet` (more diagnostic; non-breaking via `#[non_exhaustive]`); (d) reconsider `from_bytes` shape (slice+usize vs `&mut Cursor`) when 4-E lands; (e) potential `chunking/` directory split (current 384-line file projects to ~700 after 4-D + 4-E). |
| 4-C compute_wallet_id | `e43dc8d` (no fix needed) | `pub fn compute_wallet_id(&[u8]) -> WalletId` = `SHA-256(bytecode)[0..16]` via `bitcoin::hashes::sha256::Hash::hash`; matches the SHA-256/16-byte decision (POLICY_BACKUP §8 line 742); 5 new tests + 1 doctest pin the empty-input vector and the cross-task `truncate → 0xe3b0c` 20-bit relationship; (237 → 242 lib + 1 doctest). Follow-ups (style only): (a) align constructor with file convention by switching `WalletId::new(bytes)` → `WalletId::from(bytes)` (the rest of `wallet_id.rs`'s call sites use `From`); (b) doctest comment could put a space at the 16-byte boundary in the SHA-256 hex literal for self-evident truncation; (c) intra-doc-link `WalletId::truncate` instead of code-block reference. |
| 4-D chunking_decision | `1fe9505` (no fix needed; approve-with-followup) | `ChunkCode { Regular, Long }` (`#[non_exhaustive]`, `const fn` capacities 48/56 single-string + 45/53 fragment) + `ChunkingPlan { SingleString, Chunked }` (`#[non_exhaustive]`); `chunking_decision(bytecode_len, force_chunked)` selects via try-Regular-then-Long fallthrough at each tier; uses `usize::div_ceil`; new `Error::PolicyTooLarge { bytecode_len, max_supported }` with `max_supported = 32*53-4 = 1692`; 12 new tests at all spec boundaries (48/49, 56/57, 1436/1437, 1692/1693); (242 → 254). Follow-ups for 4-E to address opportunistically — most addressed in 4-E (`f0d9346`) or in chunking-bucket followup (`6b93e79`): (a) ✅ `MAX_BYTECODE_LEN` extracted in 4-E; (b) ✅ `MAX_CHUNK_COUNT` extracted in 4-E (collapsed to single `u8` const in `2e735be`); (c) ✅ test #1 strengthened in 4-E; (d) ✅ explicit 49-byte test added in `6b93e79`; (e) ✅ Regular-first doc note added in 4-E; (f) ⏸ `force_chunked` → enum deferred until call sites multiply; (g) ✅ 0-byte degenerate test added in `6b93e79`; (h) ⏸ `chunking.rs` directory split deferred. |
| 4-E chunk_bytes + reassemble_chunks | `f0d9346` (approve-with-followup; followups in `2e735be`) | `Chunk { header, fragment }` (`#[non_exhaustive]`); `chunk_bytes(bytecode, plan, wallet_id)` produces fragments with cross-chunk SHA-256[0..4] appended (chunked) or fragment=bytecode (single); `reassemble_chunks` performs all 7 BIP §"Reassembly" validations (empty, mixed-type, single-with-multiple, wallet_id, count, index range, dup, missing, hash); 4 new top-level Error variants (`EmptyChunkList`, `MissingChunkIndex(u8)`, `MixedChunkTypes`, `SingleStringWithMultipleChunks`); 4-D bonus follow-ups bundled (consts extracted, test #1 strengthened, doc note); 22 new tests (254 → 276). Code-review follow-ups all applied in `2e735be`: (1) ✅ eliminated per-chunk `clone` via `into_iter()` + `HashMap::remove`+`Vec::append`; (2) ✅ collapsed `MAX_CHUNK_COUNT_U8` into single `pub const MAX_CHUNK_COUNT: u8 = 32`; (3) ✅ added `Chunk::new(header, fragment)` constructor; (4) ⚠ added `Chunk::from_exact_bytes` + `Error::TrailingChunkBytes { consumed, total }` — but the error path is **unreachable in practice** because `Chunk::from_bytes` consumes all remaining bytes as the fragment (no fragment-length field; codex32 string layer provides the boundary in Phase 7). Helper kept as self-documenting alternative; error variant is dead code today; revisit if/when fragment length acquires its own wire-format presence. (5) ✅ fixed misleading `max_supported` in `chunk_bytes` capacity error to report plan-derived value (was always `MAX_BYTECODE_LEN`); (6) ✅ added hash-byte-corruption reassembly test (corrupts last byte of last fragment = stream[53] = 4th hash byte). (276 → 289). |
| _Phase 4 followup buckets_ | `df7bb4e` + `bdc0c3f` + `82277ce` + `6b93e79` (parallel) | Phase 3 + Phase 4 review nits swept up in 4 file-disjoint parallel commits: header.rs (Bucket A — 6 nits, no test count change), path.rs (Bucket B — 4 nits + 2 tests), wallet_id.rs (Bucket C — 6 nits + 1 test, single concern noted about pre-commit working tree state from parallel race, resolved at HEAD), chunking.rs/error.rs (Bucket D — 5 items + 5 tests including truncation-payload `{have, need}` and `InvalidWalletIdEncoding` → `ReservedWalletIdBitsSet` rename). |

---

## Phase 4 closure

Phase 4 is feature-complete as of `2e735be` (289 lib tests + 1 doctest passing, clippy `-D warnings` clean, fmt clean). The chunking primitives — `ChunkHeader`, `ChunkCode`, `ChunkingPlan`, `chunking_decision`, `Chunk`, `chunk_bytes`, `reassemble_chunks`, plus `WalletId`, `WalletIdWords`, `ChunkWalletId`, `compute_wallet_id` — are all implemented, tested, and ready for Phase 5 to compose into the top-level `WalletPolicy` API and the `encode` / `decode` entry points.

**Outstanding deferrals carried into Phase 5+:**
- `force_chunked: bool` → `ChunkingMode { Auto, ForceChunked }` enum (premature; defer until non-test call sites)
- `chunking.rs` directory split (file is 1500+ lines; section-banner organization remains navigable)
- `Chunk::from_exact_bytes` / `Error::TrailingChunkBytes` is currently unreachable; either remove (cleanup) or rejustify when (if) Phase 7 codex32 wrapping introduces a need for explicit chunk boundary detection
- `decode_declaration_from_bytes(&[u8]) -> Result<(DerivationPath, usize), Error>` (Phase 3 carryover; defer until non-Cursor consumer surfaces)

**Source-level v0.2 deferrals (not v0.1 follow-ups):**
- `encoding.rs:379` BCH 4-error correction (Berlekamp-Massey / Forney) — documented v0.2 scope per implementation plan risk register
- Taproot `Tr` / `TapTree` operators in bytecode encoder/decoder — Phase 2 D-2 / D-4 / D-5 deferrals to v0.2
