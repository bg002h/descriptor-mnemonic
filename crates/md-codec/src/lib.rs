//! # `md-codec`
//!
//! Reference implementation of the **Mnemonic Descriptor (MD)** format
//! — an engravable backup format for [BIP 388 wallet policies][bip388].
//!
//! [bip388]: https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki
//!
//! ## What this crate does
//!
//! Given a parsed BIP 388 wallet policy, [`encode()`] produces one or more
//! short codex32-derived strings (the "MD strings") that round-trip back to
//! the original policy via [`decode()`]. The format is designed for
//! hand-transcription onto durable physical media (engraved metal, paper) and
//! survives transcription errors via BCH error correction (BIP 93 codex32).
//!
//! MD is to *wallet structure* what BIP 39 is to *seed entropy*: a canonical
//! engravable backup format. A 24-word BIP 39 phrase restores a wallet's keys;
//! an MD string restores a wallet's spending policy — the miniscript template,
//! shared derivation path, and (in future versions) cosigner xpubs.
//!
//! ## Quick example
//!
//! ```
//! use std::str::FromStr;
//! use md_codec::{decode, encode, DecodeOptions, EncodeOptions, WalletPolicy};
//!
//! let policy = WalletPolicy::from_str("wsh(pk(@0/**))")?;
//! let backup = encode(&policy, &EncodeOptions::default())?;
//!
//! // backup.chunks holds 1+ codex32-derived strings ready to engrave.
//! // backup.wallet_id_words is the 12-word Tier-3 Wallet ID.
//! let inputs: Vec<&str> = backup.chunks.iter().map(|c| c.raw.as_str()).collect();
//! let result = decode(&inputs, &DecodeOptions::new())?;
//! assert_eq!(result.policy.to_canonical_string(), policy.to_canonical_string());
//! # Ok::<(), md_codec::Error>(())
//! ```
//!
//! ## Pipeline overview
//!
//! The encode pipeline is `WalletPolicy + EncodeOptions → MdBackup`; the
//! decode pipeline is `&[&str] + DecodeOptions → DecodeResult`. Each stage has
//! a counterpart on the decode side.
//!
//! 1. **Parse** — `WalletPolicy::from_str` (via [`std::str::FromStr`]) accepts
//!    a BIP 388 template (e.g., `wsh(pk(@0/**))`) or a full descriptor with
//!    concrete xpubs.
//! 2. **Bytecode** — [`WalletPolicy::to_bytecode`] emits canonical MD bytecode:
//!    a one-byte format header, a path declaration, and the operator tree (see
//!    [`bytecode`]).
//! 3. **Chunking decision** — [`chunking::chunking_decision`] selects a
//!    [`ChunkingPlan`]: single-string when the bytecode fits; otherwise
//!    1–32 chunks with a 4-byte cross-chunk integrity hash.
//! 4. **Codex32 wrap** — each chunk's bytes are wrapped in a codex32-derived
//!    string with HRP `md` and a BCH-encoded checksum (regular: 13 chars,
//!    long: 15 chars). See [`encoding`].
//! 5. **Tier-3 Wallet ID derivation** — `SHA-256(canonical_bytecode)[0..16]`,
//!    rendered as 12 BIP-39 words for human verification (see [`wallet_id`]).
//! 6. **Output** — a [`MdBackup`] holds the encoded chunks + words; on the
//!    decode side, a [`DecodeResult`] holds the recovered [`WalletPolicy`]
//!    plus a [`DecodeReport`] summarizing BCH corrections and verifications.
//!
//! ## Type-state graph
//!
//! ```text
//!                 EncodeOptions                    DecodeOptions
//!                       │                                │
//!                       ▼                                ▼
//!  WalletPolicy ──── encode() ──→ MdBackup ─[serialize chunks]
//!                                                        │
//!  WalletPolicy ←── decode() ──── DecodeResult ←── &[&str]
//!                       (also yields DecodeReport)
//! ```
//!
//! - Construct a [`WalletPolicy`] from a BIP 388 string with `FromStr`.
//! - Configure encoding with [`EncodeOptions`] (defaults are usually correct).
//! - Encode to a [`MdBackup`], whose `chunks: Vec<EncodedChunk>` is the
//!   engrave-ready output and `wallet_id_words: WalletIdWords` is the Tier-3
//!   12-word verifier.
//! - Decode by passing the raw strings as `&[&str]` to [`decode()`]; receive a
//!   [`DecodeResult`] containing the recovered policy and a [`DecodeReport`]
//!   that documents any BCH corrections, structural verifications, and the
//!   resulting [`Confidence`] level.
//!
//! ## Wallet identifiers
//!
//! MD uses **two distinct wallet identifiers** with different override
//! semantics. The two-WalletId story is load-bearing for the format's
//! verification story; see [`WalletId`], [`ChunkWalletId`], and
//! [`WalletIdSeed`] for the full semantics. Short version:
//!
//! - The **Tier-3 [`WalletId`]** (16 bytes, displayed as 12 BIP-39 words via
//!   [`WalletIdWords`]) is **always** content-derived from
//!   `SHA-256(canonical_bytecode)[0..16]`. It is **never** affected by
//!   [`EncodeOptions::wallet_id_seed`]. A user holding only the 12-word
//!   mnemonic can verify which seed corresponds to which `@i` placeholder.
//! - The **chunk-header [`ChunkWalletId`]** (20 bits embedded in every chunked
//!   string's header) is by default the first 20 bits of the same SHA-256
//!   (so the Tier-3 mnemonic predicts it). It can be overridden by setting
//!   [`EncodeOptions::wallet_id_seed`] for deterministic test-vector
//!   generation.
//!
//! See `IMPLEMENTATION_PLAN_v0.1.md` §4 "Wallet ID semantics" and the BIP
//! draft §"Wallet identifier" for the full rationale.
//!
//! ## Scope (v0.1)
//!
//! - Single user holding all seeds (no foreign xpubs)
//! - All `@i` placeholders share one derivation path
//! - `wsh()` segwit-v0 wallet policies
//!
//! Foreign xpubs (multi-party multisig where you don't hold all seeds), per-
//! placeholder paths, taproot, MuSig2, and BIP 393 recovery annotations are
//! deferred to v0.2+.
//!
//! ## Module map
//!
//! - [`bytecode`] — canonical bytecode encode/decode (operator tags,
//!   path declarations, varints).
//! - [`encoding`] — codex32-derived BCH layer (BIP 93 polymod, HRP, checksum).
//! - [`chunking`] — multi-string chunk header, plan selection, assembly,
//!   reassembly.
//! - [`policy`] — [`WalletPolicy`] newtype + [`MdBackup`] struct.
//! - [`wallet_id`] — Tier-3 [`WalletId`] / [`ChunkWalletId`] / [`WalletIdSeed`].
//! - [`options`] — [`EncodeOptions`] / [`DecodeOptions`] knobs.
//! - [`decode_report`] — [`DecodeResult`] / [`DecodeReport`] / [`Confidence`].
//! - [`error`] — [`Error`] enum (the public API contract for failures).
//! - [`vectors`] — JSON test-vector schema + builder used by `gen_vectors`.
//!
//! ## See also
//!
//! - [BIP draft][bip-draft] — authoritative format specification.
//! - [`design/POLICY_BACKUP.md`][design] — design rationale and decisions.
//!
//! [bip-draft]: https://github.com/bg002h/descriptor-mnemonic/blob/main/bip/bip-mnemonic-descriptor.mediawiki
//! [design]: https://github.com/bg002h/descriptor-mnemonic/blob/main/design/POLICY_BACKUP.md

#![cfg_attr(not(test), deny(missing_docs))]

pub mod bytecode;
pub mod chunking;
pub mod decode;
pub mod decode_report;
pub mod encode;
pub mod encoding;
pub mod error;
pub mod options;
pub mod policy;
#[cfg(feature = "compiler")]
pub mod policy_compiler;
pub mod vectors;
pub mod wallet_id;

pub use chunking::{
    Chunk, ChunkCode, ChunkHeader, ChunkingMode, ChunkingPlan, Correction, EncodedChunk,
    MAX_BYTECODE_LEN, MAX_CHUNK_COUNT, chunk_bytes, chunking_decision, reassemble_chunks,
};
pub use decode::decode;
pub use decode_report::{
    Confidence, DecodeOutcome, DecodeReport, DecodeResult, TapLeafReport, Verifications,
};
pub use encode::encode;
pub use encoding::{
    BchCode, DecodedString, bytes_to_5bit, decode_string, encode_string, five_bit_to_bytes,
};
pub use error::{BytecodeErrorKind, Error, Result};
pub use options::{DecodeOptions, EncodeOptions};
pub use policy::{MdBackup, WalletPolicy};
#[cfg(feature = "compiler")]
pub use policy_compiler::{ScriptContext, policy_to_bytecode};
pub use vectors::{NegativeVector, TestVectorFile, Vector};
pub use wallet_id::{
    ChunkWalletId, WalletId, WalletIdSeed, WalletIdWords, compute_wallet_id_for_policy,
};

/// Encode a [`WalletPolicy`] as canonical MD bytecode.
///
/// Thin free-function wrapper around [`WalletPolicy::to_bytecode`]. Provided
/// for symmetry with [`decode_bytecode`] and for callers who prefer a
/// function-style API over a method call. The output is the canonical
/// `[header][path-declaration][tree]` byte sequence consumed by the chunking
/// + codex32 layers; see [`bytecode`] for the on-the-wire format.
///
/// Uses default [`EncodeOptions`] — no shared-path override and no other
/// knobs. Callers needing options should call [`WalletPolicy::to_bytecode`]
/// directly with a configured [`EncodeOptions`].
pub fn encode_bytecode(policy: &WalletPolicy) -> Result<Vec<u8>> {
    policy.to_bytecode(&EncodeOptions::default())
}

/// Decode canonical MD bytecode into a [`WalletPolicy`].
///
/// Thin free-function wrapper around [`WalletPolicy::from_bytecode`]. The
/// input is the same `[header][path-declaration][tree]` byte sequence
/// produced by [`encode_bytecode`].
///
/// # Errors
///
/// Returns the same errors as [`WalletPolicy::from_bytecode`]:
/// [`Error::InvalidBytecode`] on malformed input, [`Error::UnsupportedVersion`]
/// for non-zero version nibbles, and [`Error::PolicyScopeViolation`] for
/// inputs that fall outside the v0.1 scope.
pub fn decode_bytecode(bytes: &[u8]) -> Result<WalletPolicy> {
    WalletPolicy::from_bytecode(bytes)
}
