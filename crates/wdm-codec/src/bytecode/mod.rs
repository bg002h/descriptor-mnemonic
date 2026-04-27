//! Bytecode encoding and decoding for canonical WDM bytecode.

pub mod decode;
pub mod encode;
pub mod key;
pub mod path;
pub mod tag;
pub mod varint;

pub use tag::Tag;
