# Phase 10 Task 10.4 — Public API audit

**Status:** DONE_WITH_CONCERNS
**Commit:** `ab2f24a`
**File(s):**
- `crates/wdm-codec/src/lib.rs`
- `crates/wdm-codec/src/policy.rs`
- `crates/wdm-codec/src/wallet_id.rs`
- `crates/wdm-codec/src/options.rs`
- `crates/wdm-codec/src/encoding.rs`
- `crates/wdm-codec/src/decode_report.rs`
- `crates/wdm-codec/src/error.rs`
- `crates/wdm-codec/src/encode.rs`
- `crates/wdm-codec/src/decode.rs`
- `crates/wdm-codec/src/chunking.rs`
- `design/IMPLEMENTATION_PLAN_v0.1.md` (lines 129–336)
- `design/PHASE_2_DECISIONS.md` (Issue 3, line 133)
- `design/PHASE_5_DECISIONS.md` (D-9, line 151)
- `design/FOLLOWUPS.md`
**Role:** reviewer (api-audit)

## Summary

I performed a per-item walk-through of every type, function, method, and trait impl listed in `design/IMPLEMENTATION_PLAN_v0.1.md` §3 (lines 129–336) against the actual implementation under `crates/wdm-codec/src/`. The vast majority of the public contract is preserved exactly. Six items deviate from the spec: two are documented intentional deviations (the `From<miniscript::Error>` removal and the `compute_wallet_id` two-function split), and four are undocumented drift — most are harmless additions or signature tweaks, but one (the missing `WalletId::as_bytes`) is a small contract gap that should be patched before tag. Build/clippy/fmt/doc gates are all clean at HEAD; 440 tests pass.

## Per-section results

### Entry types (§3 lines 131–258)

| Spec item | Impl location | Status | Notes |
|---|---|---|---|
| `BchCode { Regular, Long }` (`Debug,Clone,Copy,PartialEq,Eq,Hash`) | `encoding.rs:10` | match | Re-exported from `lib.rs:158`. |
| `pub use BchCode` from `encoding` | `lib.rs:158` | match | |
| `pub use WalletPolicy, WalletId, WalletIdWords, ChunkWalletId` | `lib.rs:163` (policy) + `lib.rs:165` (wallet_id) | match | spec lumps these but the actual home modules differ; re-exports cover both `wdm_codec::WalletPolicy` and `wdm_codec::WalletId`. |
| `pub use Correction` | `lib.rs:152` | match | |
| `WalletPolicy` thin adapter | `policy.rs:170` (`#[non_exhaustive] #[derive(Debug,Clone,PartialEq,Eq)]`) | match (extra derives) | Spec only required existence; impl adds `Debug,Clone,PartialEq,Eq` — additive. |
| `WdmBackup` (`#[non_exhaustive]`, fields `chunks: Vec<EncodedChunk>`, `wallet_id_words: WalletIdWords`) | `policy.rs:436` | match | Derives `Debug,Clone,PartialEq,Eq`. |
| `WdmBackup::wallet_id() -> WalletId` | `policy.rs:458` | match | Reconstructs from BIP-39 mnemonic. |
| `EncodedChunk` (`#[non_exhaustive]`, fields `raw,chunk_index,total_chunks,code`) | `chunking.rs:418` | match | Derives `Debug,Clone,PartialEq,Eq`. |
| `DecodeReport` (`#[non_exhaustive]`, fields `outcome,corrections,verifications,confidence`) | `decode_report.rs:107` | match | |
| `DecodeOutcome { Clean, AutoCorrected, Failed }` (`#[non_exhaustive]`) | `decode_report.rs:14` | match | |
| `Verifications` (NOT `#[non_exhaustive]`, 5 bool fields) | `decode_report.rs:64` | match | Confirmed not marked non_exhaustive. |
| `Confidence { Confirmed, High, Probabilistic, Failed }` (`#[non_exhaustive]`) | `decode_report.rs:40` | match | |
| `DecodeResult { policy, report }` (`#[non_exhaustive]`) | `decode_report.rs:134` | match | |
| `Correction { chunk_index, char_position, original, corrected }` | `chunking.rs:443` | match | Derives `Debug,Clone,Copy,PartialEq,Eq`. |
| `WalletId([u8;16])` | `wallet_id.rs:48` | match | `Debug,Clone,Copy,PartialEq,Eq,Hash`. |
| `Display for WalletId` (hex) | `wallet_id.rs:98` | match | |
| `LowerHex for WalletId` | `wallet_id.rs:109` | match | |
| `AsRef<[u8]> for WalletId` | `wallet_id.rs:116` | match | |
| `From<[u8;16]> for WalletId` | `wallet_id.rs:123` | match | |
| `WalletId::as_bytes(&self) -> &[u8;16]` | (missing) | **gap** | Not implemented; only `AsRef<[u8]>` and `WalletId::new` available. See D-1 below. |
| `WalletId::to_words() -> WalletIdWords` | `wallet_id.rs:62` | match | |
| `WalletId::truncate() -> ChunkWalletId` | `wallet_id.rs:87` | match | |
| `WalletIdWords([String;12])` | `wallet_id.rs:199` | match | |
| `Display for WalletIdWords` | `wallet_id.rs:208` | match | |
| `IntoIterator for WalletIdWords` | `wallet_id.rs:225` | match | `Item = String`, 12-element array iter. |
| `ChunkWalletId(u32)` (upper 12 bits zero, debug-asserted) | `wallet_id.rs:267` | match | |
| `ChunkWalletId::MAX` const | `wallet_id.rs:271` | match | `(1 << 20) - 1`. |
| `ChunkWalletId::new(bits) -> Self` (panics if `> MAX`) | `wallet_id.rs:279` | match | Panics with `assert!`; covered by `#[should_panic]` test. |
| `ChunkWalletId::as_u32() -> u32` | `wallet_id.rs:290` | match | |
| `WalletIdSeed([u8;4])` (`Debug` redacted, `Hash`, `From<u32>`, `From<[u8;4]>`) | `wallet_id.rs:329` | match | |
| `EncodeOptions` (`#[non_exhaustive] #[derive(Default)]`, 3 fields) | `options.rs:20` | match | Adds `Debug,Clone,Copy,PartialEq,Eq`. |
| `DecodeOptions` (NOT `#[non_exhaustive]`, private `erasures`) | `options.rs:74` | match | Adds `Debug,Clone,Default,PartialEq,Eq`. |
| `DecodeOptions::new() -> Self` | `options.rs:82` | match | |

### Free functions (§3 lines 263–277)

| Spec item | Impl location | Status | Notes |
|---|---|---|---|
| `encode(policy, options) -> Result<WdmBackup, Error>` | `encode.rs:47` | match | Re-exported `lib.rs:157`. |
| `decode(strings, options) -> Result<DecodeResult, Error>` | `decode.rs:69` | match | Re-exported `lib.rs:155`. Impl signature uses `_options` (silently ignored in v0.1 per rustdoc). |
| `encode_bytecode(policy) -> Result<Vec<u8>, Error>` | `lib.rs:176` | match | Wrapper around `WalletPolicy::to_bytecode`. |
| `decode_bytecode(bytes) -> Result<WalletPolicy, Error>` | `lib.rs:192` | match | Wrapper around `WalletPolicy::from_bytecode`. |
| `compute_wallet_id(policy: &WalletPolicy) -> WalletId` | `wallet_id.rs:169` & `wallet_id.rs:183` | drift (documented) | Spec is single-fn taking `&WalletPolicy`; impl is two functions: `compute_wallet_id(&[u8]) -> WalletId` (Phase 4-C) AND `compute_wallet_id_for_policy(&WalletPolicy) -> Result<WalletId, Error>` (Phase 5-B). Tracked in `PHASE_5_DECISIONS.md` D-9 (line 151). See D-2 below. |

### `WalletPolicy` methods (§3 lines 281–295)

| Spec item | Impl location | Status | Notes |
|---|---|---|---|
| `impl FromStr for WalletPolicy` (`Err = Error`) | `policy.rs:176` | match | |
| `to_canonical_string(&self) -> String` | `policy.rs:197` | match | |
| `key_count(&self) -> usize` | `policy.rs:211` | match | |
| `shared_path(&self) -> Option<&DerivationPath>` | `policy.rs:241` | drift | Impl returns `Option<DerivationPath>` (owned) instead of `Option<&DerivationPath>`. The implementation materializes via `into_descriptor()` so a borrow into `self` is not naturally available; the cloned `DerivationPath` is what comes out. See D-3. |
| `to_bytecode(&self) -> Result<Vec<u8>, Error>` | `policy.rs:288` | match | |
| `from_bytecode(bytes) -> Result<Self, Error>` | `policy.rs:353` | match | |
| `inner(&self) -> &miniscript::descriptor::WalletPolicy` (`#[doc(hidden)]`) | `policy.rs:259` | match | |

### `Error` variants (§3 lines 301–318)

| Spec variant | Impl variant | Status | Notes |
|---|---|---|---|
| `InvalidHrp(String)` | `error.rs:54` | match | |
| `MixedCase` | `error.rs:63` | match | |
| `InvalidStringLength(usize)` | `error.rs:71` | match | |
| `BchUncorrectable` | `error.rs:95` | match | |
| `InvalidBytecode { offset, kind }` | `error.rs:105` | match | |
| `UnsupportedVersion(u8)` | `error.rs:119` | match | |
| `UnsupportedCardType(u8)` | `error.rs:127` | match | |
| `ChunkIndexOutOfRange { index, total }` | `error.rs:136` | match | |
| `DuplicateChunkIndex(u8)` | `error.rs:149` | match | |
| `WalletIdMismatch { expected, got }` (both `ChunkWalletId`) | `error.rs:158` | match | |
| `TotalChunksMismatch { expected, got }` | `error.rs:172` | match | |
| `PolicyScopeViolation(String)` | `error.rs:187` | match | |
| `CrossChunkHashMismatch` | `error.rs:198` | match | |
| `PolicyParse(String)` | `error.rs:298` | match | |
| `Miniscript(String)` (wrapped, NOT `#[from]`) | `error.rs:306` | match | |
| (additional, undocumented in spec) `InvalidChar { ch, position }` | `error.rs:80` | drift (additive) | See D-4. |
| (additional) `InvalidChunkCount(u8)` | `error.rs:206` | drift (additive) | See D-4. |
| (additional) `InvalidChunkIndex { index, count }` | `error.rs:215` | drift (additive) | See D-4. |
| (additional) `ReservedWalletIdBitsSet` | `error.rs:228` | drift (additive) | See D-4. |
| (additional) `ChunkHeaderTruncated { have, need }` | `error.rs:237` | drift (additive) | See D-4. |
| (additional) `PolicyTooLarge { bytecode_len, max_supported }` | `error.rs:254` | drift (additive) | See D-4. |
| (additional) `EmptyChunkList` | `error.rs:266` | drift (additive) | See D-4. |
| (additional) `MissingChunkIndex(u8)` | `error.rs:275` | drift (additive) | See D-4. |
| (additional) `MixedChunkTypes` | `error.rs:283` | drift (additive) | See D-4. |
| (additional) `SingleStringWithMultipleChunks` | `error.rs:290` | drift (additive) | See D-4. |

`Error` is `#[non_exhaustive]`, so additive variants are not a SemVer break. The base set required by the spec is fully present.

### `BytecodeErrorKind` variants (§3 lines 320–329)

| Spec variant | Impl variant | Status | Notes |
|---|---|---|---|
| `UnknownTag(u8)` | `error.rs:315` | match | |
| `Truncated` | `error.rs:319` | match | |
| `VarintOverflow` | `error.rs:323` | match | |
| `MissingChildren { expected, got }` | `error.rs:327` | match | |
| `UnexpectedEnd` | `error.rs:336` | match | |
| `TrailingBytes` | `error.rs:340` | match | |
| (additional) `ReservedBitsSet { byte, mask }` | `error.rs:347` | drift (additive) | See D-5. |
| (additional) `TypeCheckFailed(String)` | `error.rs:359` | drift (additive) | Replaces the removed `From<miniscript::Error>` blanket impl per Phase 2 Issue 3 — documented. |
| (additional) `InvalidPathComponent { encoded }` | `error.rs:371` | drift (additive) | See D-5. |
| (additional) `UnexpectedTag { expected, got }` | `error.rs:383` | drift (additive) | See D-5. |

`BytecodeErrorKind` is `#[non_exhaustive]`, so additive variants are not a SemVer break.

### `From<miniscript::Error> for Error` (§3 lines 331–335)

| Spec item | Impl status | Notes |
|---|---|---|
| `impl From<miniscript::Error> for Error { Error::Miniscript(e.to_string()) }` | **deliberately removed** | Documented in `PHASE_2_DECISIONS.md` line 133 ("Issue 3 from the Phase 2 decision review"); call sites construct `Error::Miniscript` explicitly. The spec text at line 331 still shows the impl, so the spec is stale here. See D-6. |

## Drift / gap inventory

### Drift D-1: `WalletId::as_bytes(&self) -> &[u8; 16]` is missing

- **Spec says:** line 211 — `pub fn as_bytes(&self) -> &[u8; 16];`
- **Impl is:** `WalletId` exposes its bytes only via `AsRef<[u8]>` (returns `&[u8]`, not the array). There is no `as_bytes` accessor. The constructor `WalletId::new([u8; 16])` exists.
- **Recommendation:** **v0.1-blocker** (small additive fix).
- **Rationale:** This is a 3-line addition (`pub fn as_bytes(&self) -> &[u8; 16] { &self.0 }`) that preserves the spec'd contract. Returning the typed array reference (vs. the untyped `&[u8]` slice that `AsRef` gives) is occasionally load-bearing for callers who need a fixed-size array (e.g. to feed into `<[u8; 16]>::from(*id.as_bytes())` or to copy into another fixed array without a length-checked panic). Cheap to add now; awkward to add later if external callers have started leaning on `AsRef`.

### Drift D-2: `compute_wallet_id` is split into two functions

- **Spec says:** line 276 — `pub fn compute_wallet_id(policy: &WalletPolicy) -> WalletId;`
- **Impl is:** Two functions: `compute_wallet_id(canonical_bytecode: &[u8]) -> WalletId` (`wallet_id.rs:169`) and `compute_wallet_id_for_policy(&WalletPolicy) -> Result<WalletId, Error>` (`wallet_id.rs:183`). Both re-exported.
- **Recommendation:** **spec-update**.
- **Rationale:** Documented intentional deviation in `PHASE_5_DECISIONS.md` D-9 ("Rust does not support function overloading"). The two-function split is the right shape; the spec just needs to be updated to match. A spec edit at line 276 listing both names with a one-line cross-reference to D-9 closes this.

### Drift D-3: `WalletPolicy::shared_path` returns owned, not borrowed

- **Spec says:** line 289 — `pub fn shared_path(&self) -> Option<&DerivationPath>;`
- **Impl is:** `policy.rs:241` returns `Option<DerivationPath>` (owned).
- **Recommendation:** **spec-update**.
- **Rationale:** The implementation materializes a `Descriptor` via `into_descriptor()` to extract the path; the resulting `DerivationPath` is not a borrow into `&self` and cannot be without holding additional state. The spec's `Option<&DerivationPath>` would force the impl either to (a) cache the materialized descriptor in `WalletPolicy` (additional state, breaks the "thin newtype" property) or (b) use unsafe self-referential tricks. Neither is desirable; the owned return is the right shape. Update spec line 289 to match.

### Drift D-4: `Error` enum has 11 additional variants beyond the spec

- **Spec says:** lines 301–318 list 15 variants.
- **Impl is:** `error.rs:47–306` defines 26 variants (15 spec'd + 11 additional). All additions are documented inline; common patterns are stage-3 header errors (`InvalidChar`, `InvalidChunkCount`, `InvalidChunkIndex`, `ReservedWalletIdBitsSet`, `ChunkHeaderTruncated`) and reassembly errors (`EmptyChunkList`, `MissingChunkIndex`, `MixedChunkTypes`, `SingleStringWithMultipleChunks`, `PolicyTooLarge`).
- **Recommendation:** **spec-update**.
- **Rationale:** `Error` is `#[non_exhaustive]`, so adding variants is non-breaking. The spec's 15-variant list reflects an early estimate; the 11 additions emerged organically from Phases 3–6. Update §3 to enumerate the actual 26 variants OR add a sentence "the implementation may carry additional `#[non_exhaustive]` variants for finer-grained reporting; the contract is that the listed variants exist".

### Drift D-5: `BytecodeErrorKind` has 4 additional variants beyond the spec

- **Spec says:** lines 320–329 list 6 kinds.
- **Impl is:** `error.rs:312–388` defines 10 kinds (6 spec'd + `ReservedBitsSet`, `TypeCheckFailed`, `InvalidPathComponent`, `UnexpectedTag`).
- **Recommendation:** **spec-update**.
- **Rationale:** Same as D-4: `BytecodeErrorKind` is `#[non_exhaustive]`. `TypeCheckFailed` is the documented Issue 3 substitute for the removed `From<miniscript::Error>` blanket impl. The other three are emergent from bytecode parser hardening. Update §3 enumerative list or replace with "non-exhaustive contract".

### Drift D-6: Spec still shows `From<miniscript::Error> for Error` impl that has been removed

- **Spec says:** lines 331–335 show the impl.
- **Impl is:** Removed per Phase 2 Issue 3 (documented in `PHASE_2_DECISIONS.md` line 133). Test `error.rs:404` (`miniscript_error_can_be_wrapped_explicitly`) pins the removal: callers explicitly construct `Error::Miniscript(ms_err.to_string())`.
- **Recommendation:** **spec-update**.
- **Rationale:** Documented intentional deviation. Spec text at lines 317 and 331 should be updated: line 317 already has the comment "wrapped, NOT `#[from]` miniscript::Error" but lines 331–335 still show the impl. Either delete lines 331–335 or replace with a note "Originally proposed; removed per Phase 2 Issue 3 — see PHASE_2_DECISIONS.md".

## Follow-up items (controller appends to FOLLOWUPS.md)

- `p10-walletid-as-bytes`: Add `pub fn as_bytes(&self) -> &[u8; 16]` to `WalletId` per `IMPLEMENTATION_PLAN_v0.1.md` line 211. **Tier:** v0.1-blocker. **Where:** `crates/wdm-codec/src/wallet_id.rs` immediately above or below `to_words` (line 62). Three lines plus a doctest. (D-1)
- `p10-spec-compute-wallet-id-split`: Update `IMPLEMENTATION_PLAN_v0.1.md` §3 line 276 to list both `compute_wallet_id(&[u8])` and `compute_wallet_id_for_policy(&WalletPolicy)`, cross-referencing `PHASE_5_DECISIONS.md` D-9. **Tier:** v0.1-blocker (documentation drift). (D-2)
- `p10-spec-shared-path-owned`: Update `IMPLEMENTATION_PLAN_v0.1.md` §3 line 289 from `Option<&DerivationPath>` to `Option<DerivationPath>`. **Tier:** v0.1-blocker (documentation drift). (D-3)
- `p10-spec-error-additions`: Update `IMPLEMENTATION_PLAN_v0.1.md` §3 lines 301–318 either enumerating the actual 26 `Error` variants or adding a non-exhaustive contract clause. **Tier:** v0.1-nice-to-have (the contract is preserved; this is editorial cleanup). (D-4)
- `p10-spec-bytecode-error-additions`: Same treatment for `BytecodeErrorKind` lines 320–329 (10 actual variants vs. 6 spec'd). **Tier:** v0.1-nice-to-have. (D-5)
- `p10-spec-from-miniscript-removal`: Delete or annotate `IMPLEMENTATION_PLAN_v0.1.md` lines 331–335 to reflect the documented Phase 2 Issue 3 removal. **Tier:** v0.1-blocker (documentation drift; the spec currently promises an impl that does not exist). (D-6)

## Gate state at audit time

- `cargo test -p wdm-codec --lib`: PASS (361 tests in wdm-codec lib).
- `cargo test --workspace`: PASS (440 tests total).
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo fmt --check`: PASS.
- `RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps`: PASS.

## Final verdict

**needs-1-fix-and-spec-touch-ups before tag.**

The public contract is overwhelmingly preserved. Only one substantive code gap blocks the tag: **D-1 (`WalletId::as_bytes`)** is a tiny missing accessor that the spec explicitly promises and is cheap to add — three lines plus a doctest. After that, the remaining items are documentation drift in `IMPLEMENTATION_PLAN_v0.1.md` §3:

- **Tag-blocking spec edits** (because the spec currently promises behavior that doesn't exist): D-2, D-3, D-6.
- **Editorial spec edits** (additive only, contract preserved): D-4, D-5.

If the controller is willing to amend §3 in the same release commit that adds `WalletId::as_bytes`, v0.1.0 is tag-ready immediately afterward. None of the deviations break SemVer commitments; the additions are all guarded by `#[non_exhaustive]`.

No build/test gate failures — implementation correctness is solid.
