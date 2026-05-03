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

use md_codec::encode::Descriptor;
use md_codec::tree::{Body, Node};
use md_codec::tlv::TlvSection;
use md_codec::origin_path::{OriginPath, PathDecl, PathDeclPaths};
use md_codec::use_site_path::UseSitePath;

#[derive(Serialize)]
pub struct JsonDescriptor {
    pub n: u8,
    pub path_decl: JsonPathDecl,
    pub use_site_path: JsonUseSitePath,
    pub tree: JsonNode,
    pub tlv: JsonTlv,
}
impl From<&Descriptor> for JsonDescriptor {
    fn from(d: &Descriptor) -> Self {
        Self {
            n: d.n,
            path_decl: (&d.path_decl).into(),
            use_site_path: (&d.use_site_path).into(),
            tree: (&d.tree).into(),
            tlv: (&d.tlv).into(),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "tag", content = "data")]
pub enum JsonPathDecl {
    Shared(String),
    Divergent(Vec<String>),
}
impl From<&PathDecl> for JsonPathDecl {
    fn from(p: &PathDecl) -> Self {
        match &p.paths {
            PathDeclPaths::Shared(op) => JsonPathDecl::Shared(format_origin_path(op)),
            PathDeclPaths::Divergent(v) => JsonPathDecl::Divergent(v.iter().map(format_origin_path).collect()),
        }
    }
}

/// Render an `OriginPath` as `m/...` notation. v0.14's `OriginPath` does not
/// implement `Display`, so we format it here.
fn format_origin_path(p: &OriginPath) -> String {
    let mut s = String::from("m");
    for c in &p.components {
        s.push('/');
        s.push_str(&c.value.to_string());
        if c.hardened { s.push('\''); }
    }
    s
}

#[derive(Serialize)]
pub struct JsonUseSitePath {
    pub multipath: Option<Vec<JsonAlt>>,
    pub wildcard_hardened: bool,
}
#[derive(Serialize)]
pub struct JsonAlt { pub hardened: bool, pub value: u32 }
impl From<&UseSitePath> for JsonUseSitePath {
    fn from(u: &UseSitePath) -> Self {
        Self {
            multipath: u.multipath.as_ref().map(|alts| alts.iter().map(|a| JsonAlt { hardened: a.hardened, value: a.value }).collect()),
            wildcard_hardened: u.wildcard_hardened,
        }
    }
}

#[derive(Serialize)]
pub struct JsonNode {
    pub tag: String,
    pub body: JsonBody,
}
impl From<&Node> for JsonNode {
    fn from(n: &Node) -> Self {
        Self { tag: format!("{:?}", n.tag), body: (&n.body).into() }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum JsonBody {
    KeyArg { index: u8 },
    Children(Vec<JsonNode>),
    Variable { k: u8, children: Vec<JsonNode> },
    Tr { key_index: u8, tree: Option<Box<JsonNode>> },
    Hash256Body(String),  // hex
    Hash160Body(String),  // hex
    Timelock(u32),
    Empty,
}
impl From<&Body> for JsonBody {
    fn from(b: &Body) -> Self {
        match b {
            Body::KeyArg { index } => JsonBody::KeyArg { index: *index },
            Body::Children(v) => JsonBody::Children(v.iter().map(JsonNode::from).collect()),
            Body::Variable { k, children } => JsonBody::Variable {
                k: *k, children: children.iter().map(JsonNode::from).collect()
            },
            Body::Tr { key_index, tree } => JsonBody::Tr {
                key_index: *key_index,
                tree: tree.as_ref().map(|n| Box::new(JsonNode::from(n.as_ref()))),
            },
            Body::Hash256Body(h) => JsonBody::Hash256Body(hex(h)),
            Body::Hash160Body(h) => JsonBody::Hash160Body(hex(h)),
            Body::Timelock(v) => JsonBody::Timelock(*v),
            Body::Empty => JsonBody::Empty,
        }
    }
}

#[derive(Serialize, Default)]
pub struct JsonTlv {
    pub use_site_path_overrides: Option<Vec<(u8, JsonUseSitePath)>>,
    pub fingerprints: Option<Vec<(u8, String)>>,
    pub pubkeys: Option<Vec<(u8, String)>>,
    pub origin_path_overrides: Option<Vec<(u8, String)>>,
    pub unknown: Vec<(u8, String, usize)>,  // (tag, hex(payload), bit_len)
}
impl From<&TlvSection> for JsonTlv {
    fn from(t: &TlvSection) -> Self {
        Self {
            use_site_path_overrides: t.use_site_path_overrides.as_ref().map(|v| v.iter().map(|(i, u)| (*i, u.into())).collect()),
            fingerprints: t.fingerprints.as_ref().map(|v| v.iter().map(|(i, fp)| (*i, hex(fp))).collect()),
            pubkeys: t.pubkeys.as_ref().map(|v| v.iter().map(|(i, p)| (*i, hex(p))).collect()),
            origin_path_overrides: t.origin_path_overrides.as_ref().map(|v| v.iter().map(|(i, op)| (*i, format_origin_path(op))).collect()),
            unknown: t.unknown.iter().map(|(tag, payload, bits)| (*tag, hex(payload), *bits)).collect(),
        }
    }
}

#[cfg(test)]
mod descriptor_json_tests {
    use super::*;
    use crate::parse::template::parse_template;

    #[test]
    fn wsh_multi_serializes() {
        let d = parse_template("wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))", &[], &[]).unwrap();
        let j = serde_json::to_value(JsonDescriptor::from(&d)).unwrap();
        assert_eq!(j["n"], 2);
        assert_eq!(j["tree"]["tag"], "Wsh");
        assert_eq!(j["tree"]["body"]["kind"], "Children");
        // Inner Multi node:
        assert_eq!(j["tree"]["body"]["data"][0]["tag"], "Multi");
        assert_eq!(j["tree"]["body"]["data"][0]["body"]["kind"], "Variable");
    }
}
