//! Bytecode encoding and decoding for canonical MD bytecode.

pub mod cursor;
pub mod decode;
pub mod encode;
pub mod header;
pub mod key;
pub mod path;
pub mod tag;
pub mod varint;

pub use header::BytecodeHeader;
pub use key::MdKey;
pub use tag::Tag;
