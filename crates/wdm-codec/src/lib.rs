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
pub use wallet_id::{ChunkWalletId, WalletId, WalletIdWords};
