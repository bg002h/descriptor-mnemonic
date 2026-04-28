# v0.2.1 full code audit — Opus 4.7

**Status:** DONE_WITH_CONCERNS
**Subject:** wdm-codec at commit 685bb61 (tag wdm-codec-v0.2.1)
**Verdict:** READY-WITH-CAVEATS — one BLOCKER for shell-impl parity / hostile-input safety; remainder of the code base is in unusually good shape.

## Executive summary

The crate is well-architected, well-documented, and well-tested. 564/564 tests pass; clippy clean; rustdoc clean (private items included). One real BLOCKER was discovered and reproduced: the top-level decoder panics on a class of malicious-but-BCH-valid Long-code input. Two IMPORTANT findings flag a load-bearing `expect()` whose justification is incorrect, plus a CLI-side mirror of the same issue. A handful of NITs and a substantial POSITIVE list round things out. The fix for the BLOCKER is mechanically small (replace one `expect()` with a structured error variant) and does not affect wire format. Once that lands, both v0.3 feature work and the planned bash shell-impl can proceed with confidence.

## Audit dimensions

### 1. Security

- **BLOCKER**: `crates/wdm-codec/src/decode.rs:135-136` — the `expect()` after `five_bit_to_bytes` is reachable with crafted input. The comment claims structural impossibility ("BCH layer emits length-aligned 5-bit data"), but this only holds for *encoder-produced* strings. The decoder accepts any 5-bit symbol sequence that satisfies the BCH polymod, so an attacker can construct a Long-code string whose final 5-bit symbol has a non-zero low bit, which forces `five_bit_to_bytes` to reject the trailing-padding-bits constraint and `expect()` to panic. **Reproduced**: a 93-symbol Long-code data part with `[0;92] ++ [1]` plus the legitimate 15-char checksum decodes cleanly through the BCH layer and panics in `decode()`. Fix: return `Err(Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::Truncated })` (or a new variant such as `MalformedPayloadPadding`) instead of `expect`. Wire format unaffected. The CLI's `wdm decode` and `wdm verify` subcommands, which call the lib `decode()` directly, inherit this panic.
- **IMPORTANT**: same root cause manifests in 4 sites in `encode.rs` (`five_bit_to_bytes(&decoded.data).expect("five-bit decode")` at lines 266/340/375/464). These are inside `#[cfg(test)]` so they are not user-reachable from a malicious input, but they pin the same false invariant; a future refactor that lifts those helpers to bin code (the CLI's `cmd_inspect` already handles `None` correctly via `.ok_or_else`) would re-leak the panic. Tighten the `decode_string → five_bit_to_bytes` contract centrally.
- POSITIVE: zero `unsafe` in the entire codebase; all `unwrap()` in lib code are inside `#[cfg(test)]` or behind a structural guarantee (`ChunkWalletId::new` `assert!` is fed only from pre-masked 20-bit values; `dummy_keys` panic is on a static table; `BytecodeHeader::new_v0(..)` reach is bounded). Varint decoder is correct (10-byte cap + bit-position guard at byte 10). All count/length fields read from input are single-byte → max 255 → no DoS via huge `Vec::with_capacity`. The fingerprints block uses `checked_add` for offset arithmetic. Cryptographic primitives are direct ports of BIP 93 with cross-validated constants and an algorithm-level `bch_verify_*` re-check after applying corrections (the explicit defense against `>4`-error inputs whose syndromes happen to factor as a degree-≤4 locator).
- POSITIVE: `WalletIdSeed` `Debug` impl deliberately redacts bytes — small but thoughtful detail.
- POSITIVE: privacy story for the fingerprints block is on-spec — encoder default is `None` → header byte `0x00` → no on-wire presence; opt-in only; CLI emits a stderr warning when `--fingerprint` is used (per BIP §"Fingerprints block" Privacy paragraph MUST clause).

### 2. Correctness

- POSITIVE: round-trip property holds across all 564 tests, including 3,200 randomized BCH round-trips at the t=4 boundary, 14 positive corpus vectors, and 34 negative vectors (most with byte-for-byte exact `input_strings`). The wire-format-stability gate (`v0_2_sha256_lock_matches_committed_file`) explicitly pins the v0.2.json SHA, and the v0.1 corpus also still verifies via a separate test path.
- POSITIVE: `from_bytecode` handles every edge I probed: empty input → `UnexpectedEnd`; truncated header → structured error; reserved bits set → typed `ReservedBitsSet`; missing children → `MissingChildren { expected, got }`; cross-chunk hash mismatch → `CrossChunkHashMismatch`; BIP32 child > 2³¹ encoded value → `InvalidPathComponent { encoded }`. The exhaustiveness gate (`tests/error_coverage.rs`) catches any new `Error` variant that lacks a `rejects_*` test.
- POSITIVE: integer-overflow-safe varint decoder; the 10-byte cap and the byte-10-payload-≤-1 check are both tested (`decode_rejects_overflow`).
- IMPORTANT: the BLOCKER above also has a correctness face — the BIP says (§"Payload" line 124) "Bytecode is converted to bech32 characters by the standard convert-bits algorithm (BIP 173 procedure with `frombits=8`, `tobits=5`, padding enabled on the encode side; reversed on decode)". The "reversed on decode" implies the decoder MUST reject non-zero padding bits with a structured error, not a panic. The current behavior is non-conformant.

### 3. Architecture

- POSITIVE: the layering is clean and the names are evocative — bytecode (operator AST + cursor + tags + varint + path + key + header) → chunking (header + plan + assembly) → encoding (codex32 + BCH) → decode_report (outcome + verifications + confidence) → policy (newtype + WdmBackup) → wallet_id → options → vectors → error. Each module has a top-of-file docstring that orients a new reader. The lib.rs preamble is a textbook example of a tutorial-grade module-graph map.
- POSITIVE: public API surface is well-curated. `Error` is `#[non_exhaustive]`. `EncodeOptions`, `WdmBackup`, `EncodedChunk`, `ChunkHeader`, `ChunkingPlan`, `Chunk`, `Tag`, and `CorrectionResult` are all `#[non_exhaustive]`. `BytecodeErrorKind` is also `#[non_exhaustive]`. Builder methods (`with_chunking_mode`, `with_force_chunking`, `with_force_long_code`, `with_seed`, `with_shared_path`, `with_fingerprints`) cover all knobs and let callers compose options without struct-literal access.
- POSITIVE: the dependency posture is conservative — `bitcoin` 0.32, `bip39` 2, `miniscript` (git-pinned to a specific SHA on apoelstra/2026-04, with the workspace `[patch]` redirect to the fork carrying the merged-but-not-yet-released hash-terminal translator fix). MSRV pinned at 1.85, edition 2024, resolver 3. The CI workflow installs the same MSRV across all three OSes and the `[patch]` story is documented inline in the root Cargo.toml.
- NIT: the workspace `[patch]` rewrite + sibling-clone CI step is a known temporary; tracked in FOLLOWUPS as `external-pr-1-hash-terminals`. Just confirming this doesn't need re-flagging.
- POSITIVE: type-state vs runtime-check tradeoffs are deliberate and documented. `ChunkHeader` is an enum (not struct-with-Option) precisely because the wire format encodes a `type` byte that determines which fields are present; the enum discriminant carries the invariant that `wallet_id`/`count`/`index` are only present in `Chunked`.

### 4. Test coverage

- POSITIVE: 564 tests across 16 binaries (391 lib unit tests + 8 main bin + 165 integration across 12 files + 5 doc-tests). The `error_coverage.rs` exhaustiveness gate forces any new `Error` variant to grow at least one `rejects_*` test in `conformance.rs`. The conformance suite has tests for all 26 variants in the mirror enum. Property-style coverage in `tests/bch_correction.rs` does 3,200 randomized round-trips.
- POSITIVE: negative vectors in v0.2.json are byte-for-byte exact, generated programmatically, and asserted at vector-build time via `debug_assert!` that decode returns the expected variant. Schema 2 includes the `provenance` field that documents which variants need lower-level API access (e.g., `EmptyChunkList`, `PolicyTooLarge`).
- IMPORTANT: there is NO test that pins the malicious-non-zero-padding-bit case described in the BLOCKER. Adding a `rejects_long_code_nonzero_trailing_pad_bit` test in `tests/conformance.rs` (or the BCH correction suite) would catch any future regression once the panic is converted to a structured error.
- NIT: test names are mostly excellent; a few that grew through phases (e.g., the renamed `chunking_mode_force_chunked_skips_single_string`) are now precise.

### 5. Documentation

- POSITIVE: rustdoc is genuinely informative, not decorative. `Error` variants document WHEN they fire and what the CALLER should do. Public structs document their `#[non_exhaustive]` rationale. Pipeline-flavored types name the stage they belong to. `lib.rs` carries a full pipeline diagram in ASCII art.
- POSITIVE: `bip/bip-wallet-descriptor-mnemonic.mediawiki` is consistent with the impl. Spot-checked: HRP, alphabet, target constants `T_REGULAR = 0x0815c07747a3392e7` and `T_LONG = 0x205701dd1e8ce4b9f47`, BCH 8-consecutive-roots windows (β^77..β^84 and γ^1019..γ^1026), generator polynomials, fingerprints byte layout, chunk header layout, and capacity arithmetic (32×53−4=1692) all match between BIP and code. The `Tag::TapTree (0x08)` v1+ deferral is reflected on both sides. Operator names use BIP 388 spelling on both sides via the `tag_to_bip388_name` helper.
- POSITIVE: README + CHANGELOG + MIGRATION are coherent. CHANGELOG documents every breaking change with cross-references to MIGRATION sections. MIGRATION provides before/after code examples and migration recipes (e.g., `&EncodeOptions::default()` for `to_bytecode` callers; `.clone()` for callers assuming `EncodeOptions: Copy`).
- IMPORTANT: BIP §"Payload" line 124 says "padding enabled on the encode side; reversed on decode" — this implies decoders MUST reject non-zero trailing pad bits, but neither the BIP nor the impl spell out the error variant the decoder should surface. The BIP could be tightened with a normative MUST clause; the impl currently panics.
- NIT: open FOLLOWUP `p10-bip-header-status-string` notes the BIP draft's `Status:` field is one revision behind the README. Stylistic.

### 6. Idiomatic Rust

- POSITIVE: the codebase is in unusually good shape. clippy `--workspace --all-targets -- -D warnings` is clean. `#[non_exhaustive]` is applied consistently to every public struct/enum that may grow fields/variants in v0.3+. Error variants have well-shaped fields (named struct variants where multi-field, tuple variants where one).
- POSITIVE: `From<ChunkCode> for BchCode` is implemented; `chunk_code_to_bch_code` helper is now a no-op shim that could be removed (already noted in inline comment).
- NIT: a few of the panics in test code use `panic!("expected ...")` rather than `assert!(matches!(...))`; not actually a problem since they are inside test fns, but slightly less informative on failure than a `matches!` match. The `match { Ok(_) => panic!(...), Err(e) if matches!(e, ...) => {} }` pattern in `bytecode/decode.rs:992`/`1186`/`1202`/`1234` is cromulent.
- NIT: `chunk_code_to_bch_code` private helper at `encode.rs:17-22` is now redundant given the public `From` impl; can be deleted in a future micro-cleanup.

### 7. Forward-looking concerns

- **The BLOCKER must be fixed before the bash shell-impl starts.** A non-Rust impl that mirrors the current Rust behavior for malicious inputs would either (a) replicate a bug that has already been documented as `expect("structurally impossible")`, or (b) silently diverge by handling the case differently — neither outcome is acceptable for a reference impl. Once the panic is converted to a structured error, both impls can produce byte-identical error variants on every input; that is the property a reference impl should guarantee. The fix is mechanical and isolated: replace the `expect` at `decode.rs:135-136` with `Err(Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::Truncated })` (or a new variant; see below).
- v0.3 feature work (taproot multi-leaf, foreign xpubs, MuSig2, BIP 393 recovery annotations) is well-scoped by the `#[non_exhaustive]` posture and the existing `Error::PolicyScopeViolation` /`TapLeafSubsetViolation` deferrals.
- The 7 open FOLLOWUPS are appropriately tiered. None is more urgent than tagged: `p2-inline-key-tags` (v1+), `external-pr-1-hash-terminals` (waiting on upstream), `p10-bip-header-status-string` (nice-to-have stylistic), `decoded-string-data-memory-microopt` (v0.3 breaking-window candidate), the two phase-D tap-leaf items (v0.3, evidence-driven), and `cli-json-debug-formatted-enum-strings` (v1+, JSON contract pin).
- For the bash shell-impl, two subtleties to watch:
  1. The Long code's data part is 93 5-bit symbols = 465 bits, leaving 1 bit of trailing padding when interpreting as bytes. Both impls must agree to reject non-zero trailing pad bits with the same error class. (See BLOCKER fix.)
  2. The polymod constants (`POLYMOD_INIT`, `GEN_REGULAR`, `GEN_LONG`, `WDM_REGULAR_CONST`, `WDM_LONG_CONST`, `REGULAR_SHIFT`/`LONG_SHIFT`, `REGULAR_MASK`/`LONG_MASK`) need to be reproduced bit-for-bit. The current implementation comments explicitly cite the NUMS-derivation `SHA-256(b"shibbolethnums")` for the target constants, so a shell impl can re-derive rather than copy.

## Specific findings

### BLOCKER — `decode.rs:135-136` panics on malicious non-zero-padding input

- **Severity**: BLOCKER
- **Where**: `crates/wdm-codec/src/decode.rs:135-136`
- **Description**: `let bytes = five_bit_to_bytes(&data_5bit).expect(...)` is reachable from external input. Constructing a Long-code WDM string with a final 5-bit data symbol whose lowest bit is set and a legitimate BCH checksum (over those data symbols) produces a string that passes Stage 2 (BCH validate/correct) and then panics in Stage 3 (header parse → byte conversion). Reproduction: 92 zero symbols + 1 one symbol + the genuine 15-char long-code BCH checksum yields `wdm1qq…qqp<checksum>` (112 chars total) which panics with the embedded message "five_bit_to_bytes failed after successful BCH decode — structurally impossible".
- **Recommended fix**: replace the `expect` with `?` after mapping `None` → an `Error` variant. Either reuse `Error::InvalidBytecode { offset: 0, kind: BytecodeErrorKind::Truncated }` for minimal API churn, or introduce a new `BytecodeErrorKind::MalformedPayloadPadding` (rust-dev-friendlier; `BytecodeErrorKind` is `#[non_exhaustive]` so additive). Add a `rejects_*` test in `tests/conformance.rs` and grow the exhaustiveness mirror in `tests/error_coverage.rs` if a new variant is added.

### IMPORTANT — same false-invariant pattern in 4 test sites + 1 CLI site mirror

- **Severity**: IMPORTANT (latent)
- **Where**: `crates/wdm-codec/src/encode.rs:266, 340, 375, 464` (test code) — and the contract at `decode.rs:135-136`
- **Description**: those four `expect("five-bit decode")` calls live inside `#[cfg(test)]` and only consume locally-encoded strings, so they are not user-reachable. But they encode the same incorrect invariant ("any string that round-trips through BCH decode is byte-aligned"). After fixing the BLOCKER, sweep these to either (a) be removed if they no longer apply, or (b) carry an updated comment that they only hold for encoder-produced inputs.
- **Recommended fix**: after the BLOCKER fix lands, regrep `expect("five.bit")` and align comments. The CLI's `cmd_inspect` already does the right thing (`.ok_or_else(|| anyhow::anyhow!("invalid 5-bit data in string"))`), confirming the structured-error pattern is already idiomatic.

### NIT — vestigial `chunk_code_to_bch_code` helper

- **Severity**: NIT
- **Where**: `crates/wdm-codec/src/encode.rs:17-22`
- **Description**: The helper is functionally identical to `BchCode::from(c)` (see `From<ChunkCode> for BchCode` in `chunking.rs:229-242`). Inline comment already flags this. Pure cleanup; no behavioral change.

### NIT — comment claims structural impossibility that isn't

- **Severity**: NIT (will go away with BLOCKER fix)
- **Where**: `crates/wdm-codec/src/decode.rs:130-134`
- **Description**: The pre-`expect` block-comment explains the invariant precisely the wrong way around. Update to acknowledge that Long-code decoded data may carry a single non-zero pad bit if the input was malicious, and that's the case the structured error covers.

## Forward-looking notes for v0.3 / shell impl

For v0.3 work, the codebase is solid groundwork: the `#[non_exhaustive]` posture across `Error`, `BytecodeErrorKind`, `EncodeOptions`, `DecodeResult`, `WdmBackup`, `EncodedChunk`, `ChunkHeader`, `ChunkingPlan`, `Tag`, and `CorrectionResult` makes additive changes safe; the per-tag dispatch in the bytecode decoder is regular enough that adding multi-leaf TapTree (`Tag::TapTree = 0x08`) is essentially "lift the v0.2 single-leaf decoder into a recursive version + add encode-side mirror." The fingerprints block is already wired through both sides; foreign xpubs would need a different path (likely a new tag in the inline-key-tags reserved range `0x24..=0x31`).

For the bash shell-impl, the audit-driven concrete recommendation is: do NOT start until the BLOCKER is fixed, because a reference impl needs to define the canonical error response to hostile inputs that exercise the BCH layer's tolerance for non-aligned 5-bit data. Specifically:
- Pin the polymod constants (and their NUMS derivation) into a module that is direct-translatable to bash arithmetic.
- Mirror the `bch_verify_*` defensive re-check after applying corrections (algorithm-level guard against >4-error inputs). The Rust impl does this; the BIP §"Error-correction guarantees" SHOULD-clause is the canonical reference.
- Lock in byte-identical `Correction.corrected` output by porting the same sorted-by-position output convention from `bch_decode.rs::decode_errors` (lines 578-581).

The 7 open FOLLOWUPS are appropriate; none is mis-tiered.

## Verdict

**READY-WITH-CAVEATS**: the codebase is genuinely in production-ready shape modulo the one BLOCKER. The fix is small, isolated, and does not affect the wire format. Once that single `expect()` becomes a structured error and a `rejects_*` test pins the new behavior, the crate should be promoted from `READY-WITH-CAVEATS` to `READY-FOR-V0.3-AND-SHELL-IMPL` and a v0.2.2 patch released. The "Bitcoin specs stay pre-release longer" caveat in the README remains the right framing — but the impl side, modulo this one finding, is genuinely ready.
