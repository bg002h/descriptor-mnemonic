//! Wallet Descriptor Mnemonic (WDM) — engravable backup format for BIP 388 wallet policies.

#![cfg_attr(not(test), deny(missing_docs))]

pub mod bytecode;
pub mod chunking;
pub mod encoding;
pub mod error;
pub mod policy;
pub mod vectors;
pub mod wallet_id;

pub use error::{BytecodeErrorKind, ChunkWalletId, Error, Result};
