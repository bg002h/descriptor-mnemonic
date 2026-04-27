//! Bytecode encoding and decoding for canonical WDM bytecode.

pub mod decode;
pub mod encode;
pub mod header;
pub mod key;
pub mod path;
pub mod tag;
pub mod varint;

pub use header::BytecodeHeader;
pub use key::WdmKey;
pub use tag::Tag;
