//! Multi-card chunking round-trip tests.

use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::encode::Descriptor;
use md_codec::identity::compute_md1_encoding_id;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

fn small_descriptor() -> Descriptor {
    Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(OriginPath {
                components: vec![
                    PathComponent { hardened: true, value: 84 },
                    PathComponent { hardened: true, value: 0 },
                    PathComponent { hardened: true, value: 0 },
                ],
            }),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: TlvSection::new_empty(),
    }
}

#[test]
fn small_descriptor_splits_into_one_chunk() {
    let d = small_descriptor();
    let chunks = split(&d).unwrap();
    assert_eq!(chunks.len(), 1);
    for c in &chunks {
        assert!(c.starts_with("md1"));
    }
}

#[test]
fn chunk_set_id_matches_md1_encoding_id_top_20_bits() {
    let d = small_descriptor();
    let md1_id = compute_md1_encoding_id(&d).unwrap();
    let derived = derive_chunk_set_id(&md1_id);
    let bytes = md1_id.as_bytes();
    let expected = ((bytes[0] as u32) << 12) | ((bytes[1] as u32) << 4) | ((bytes[2] as u32) >> 4);
    assert_eq!(derived, expected);
}

#[test]
fn small_descriptor_split_then_reassemble() {
    use md_codec::chunk::reassemble;
    let d = small_descriptor();
    let chunks = split(&d).unwrap();
    let chunk_refs: Vec<&str> = chunks.iter().map(|s| s.as_str()).collect();
    let d2 = reassemble(&chunk_refs).unwrap();
    assert_eq!(d, d2);
}
