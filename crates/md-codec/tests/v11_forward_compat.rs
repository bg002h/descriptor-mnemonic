//! Forward-compat tests: unknown TLV tags are skipped/preserved per D6.

use md_codec::v11::bitstream::BitWriter;
use md_codec::v11::encode::{encode_payload, Descriptor};
use md_codec::v11::decode::decode_payload;
use md_codec::v11::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::v11::tag::Tag;
use md_codec::v11::tlv::TlvSection;
use md_codec::v11::tree::{Body, Node};
use md_codec::v11::use_site_path::UseSitePath;

fn bip84_descriptor_with_unknown_tlv() -> Descriptor {
    // Synthesize a v0.12-style Xpubs TLV (tag 0x02) raw bytes, embed as unknown.
    let mut sub = BitWriter::new();
    sub.write_bits(0x42, 8);  // arbitrary payload byte
    sub.write_bits(0x99, 8);
    let payload_bit_len = sub.bit_len();
    let payload = sub.into_bytes();

    let mut tlv = TlvSection::new_empty();
    tlv.unknown.push((0x02, payload, payload_bit_len));

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
        tlv,
    }
}

#[test]
fn unknown_tlv_round_trip_preserved() {
    let d = bip84_descriptor_with_unknown_tlv();
    let (bytes, total_bits) = encode_payload(&d).unwrap();
    let d2 = decode_payload(&bytes, total_bits).unwrap();
    assert_eq!(d.tlv.unknown, d2.tlv.unknown);
}
