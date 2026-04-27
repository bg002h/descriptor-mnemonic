//! Wallet Descriptor Mnemonic (WDM) — engravable backup format for BIP 388 wallet policies.

#![cfg_attr(not(test), deny(missing_docs))]

pub mod bytecode;
pub mod chunking;
pub mod encoding;
pub mod error;
pub mod policy;
pub mod vectors;
pub mod wallet_id;

pub use chunking::{
    Chunk, ChunkCode, ChunkHeader, ChunkingPlan, MAX_BYTECODE_LEN, MAX_CHUNK_COUNT, chunk_bytes,
    chunking_decision, reassemble_chunks,
};
pub use encoding::{
    BchCode, DecodedString, bytes_to_5bit, decode_string, encode_string, five_bit_to_bytes,
};
pub use error::{BytecodeErrorKind, Error, Result};
pub use policy::WalletPolicy;
pub use wallet_id::{ChunkWalletId, WalletId, WalletIdWords, compute_wallet_id_for_policy};

/// Encode a `WalletPolicy` as canonical WDM bytecode.
///
/// Thin wrapper around [`WalletPolicy::to_bytecode`].
pub fn encode_bytecode(policy: &WalletPolicy) -> Result<Vec<u8>> {
    policy.to_bytecode()
}

/// Decode canonical WDM bytecode into a `WalletPolicy`.
///
/// Thin wrapper around [`WalletPolicy::from_bytecode`].
pub fn decode_bytecode(bytes: &[u8]) -> Result<WalletPolicy> {
    WalletPolicy::from_bytecode(bytes)
}
