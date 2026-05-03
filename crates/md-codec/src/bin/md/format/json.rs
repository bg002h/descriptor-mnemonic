use serde::Serialize;
use md_codec::header::Header;
use md_codec::chunk::ChunkHeader;
use md_codec::identity::{Md1EncodingId, WalletDescriptorTemplateId, WalletPolicyId};

pub const SCHEMA: &str = "md-cli/1";

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes { use std::fmt::Write as _; write!(s, "{b:02x}").unwrap(); }
    s
}

#[derive(Serialize)]
pub struct JsonHeader {
    pub version: u8,
    pub divergent_paths: bool,
}
impl From<&Header> for JsonHeader {
    fn from(h: &Header) -> Self {
        Self { version: h.version, divergent_paths: h.divergent_paths }
    }
}

#[derive(Serialize)]
pub struct JsonChunkHeader {
    pub version: u8,
    pub chunk_set_id: String,
    pub count: u8,
    pub index: u8,
}
impl From<&ChunkHeader> for JsonChunkHeader {
    fn from(h: &ChunkHeader) -> Self {
        Self {
            version: h.version,
            chunk_set_id: format!("0x{:05x}", h.chunk_set_id),
            count: h.count,
            index: h.index,
        }
    }
}

#[derive(Serialize)]
pub struct JsonHash {
    pub hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}
impl From<&Md1EncodingId> for JsonHash {
    fn from(id: &Md1EncodingId) -> Self {
        Self { hex: hex(id.as_bytes()), fingerprint: None }
    }
}
impl From<&WalletDescriptorTemplateId> for JsonHash {
    fn from(id: &WalletDescriptorTemplateId) -> Self {
        Self { hex: hex(id.as_bytes()), fingerprint: None }
    }
}
impl From<&WalletPolicyId> for JsonHash {
    fn from(id: &WalletPolicyId) -> Self {
        // WalletPolicyId has no fingerprint() method — slice as_bytes() directly.
        let b = id.as_bytes();
        Self {
            hex: hex(b),
            fingerprint: Some(format!("0x{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_constant() {
        assert_eq!(SCHEMA, "md-cli/1");
    }

    #[test]
    fn header_serializes() {
        let h = Header { version: 0, divergent_paths: false };
        let v = serde_json::to_value(JsonHeader::from(&h)).unwrap();
        assert_eq!(v["version"], 0);
        assert_eq!(v["divergent_paths"], false);
    }

    #[test]
    fn chunk_header_csid_formatted() {
        let h = ChunkHeader { version: 0, chunk_set_id: 0xABCDE, count: 3, index: 1 };
        let v = serde_json::to_value(JsonChunkHeader::from(&h)).unwrap();
        assert_eq!(v["chunk_set_id"], "0xabcde");
    }
}
