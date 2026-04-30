//! End-to-end round-trip smoke tests for v0.11.

use md_codec::v11::encode::{encode_payload, Descriptor};
use md_codec::v11::decode::decode_payload;
use md_codec::v11::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::v11::tag::Tag;
use md_codec::v11::tlv::TlvSection;
use md_codec::v11::tree::{Body, Node};
use md_codec::v11::use_site_path::UseSitePath;

fn bip84_path() -> OriginPath {
    OriginPath {
        components: vec![
            PathComponent { hardened: true, value: 84 },
            PathComponent { hardened: true, value: 0 },
            PathComponent { hardened: true, value: 0 },
        ],
    }
}

#[test]
fn bip84_single_sig_round_trip() {
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(bip84_path()),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: TlvSection::new_empty(),
    };
    let (bytes, total_bits) = encode_payload(&d).unwrap();
    let d2 = decode_payload(&bytes, total_bits).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn bip84_single_sig_payload_bit_count() {
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(bip84_path()),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: TlvSection::new_empty(),
    };
    let (_bytes, total_bits) = encode_payload(&d).unwrap();
    // Header(5) + path-decl(5+26=31) + use-site(16) + tree(5) + TLV(0) = 57 bits
    assert_eq!(total_bits, 57);
}

fn bip48_path() -> OriginPath {
    OriginPath {
        components: vec![
            PathComponent { hardened: true, value: 48 },
            PathComponent { hardened: true, value: 0 },
            PathComponent { hardened: true, value: 0 },
            PathComponent { hardened: true, value: 2 },
        ],
    }
}

#[test]
fn bip48_2of3_sortedmulti_round_trip() {
    let d = Descriptor {
        n: 3,
        path_decl: PathDecl {
            n: 3,
            paths: PathDeclPaths::Shared(bip48_path()),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: (0..3).map(|i| Node {
                        tag: Tag::PkK,
                        body: Body::KeyArg { index: i },
                    }).collect(),
                },
            }]),
        },
        tlv: TlvSection::new_empty(),
    };
    let (bytes, total_bits) = encode_payload(&d).unwrap();
    let d2 = decode_payload(&bytes, total_bits).unwrap();
    assert_eq!(d, d2);
}

#[test]
fn bip84_emit_md1_string() {
    let d = Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(bip84_path()),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        },
        tlv: TlvSection::new_empty(),
    };
    let s = md_codec::v11::encode::encode_md1_string(&d).unwrap();
    assert!(s.starts_with("md1"));
}
