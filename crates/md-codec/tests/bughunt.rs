//! Bug-hunt triage rows (md-codec). Each test asserts the CORRECT behaviour,
//! so it FAILS on unmodified code when the reported bug is real.
//! Hunt report: design/agent-reports/bughunt-2026-10-31-md.md.

use md_codec::bitstream::{BitReader, BitWriter};
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

fn shared_path() -> PathDeclPaths {
    PathDeclPaths::Shared(OriginPath {
        components: vec![
            PathComponent {
                hardened: true,
                value: 48,
            },
            PathComponent {
                hardened: true,
                value: 0,
            },
        ],
    })
}

fn node(tag: Tag, body: Body) -> Node {
    Node { tag, body }
}

fn key(tag: Tag, index: u8) -> Node {
    node(tag, Body::KeyArg { index })
}

fn desc(n: u8, tree: Node, tlv: TlvSection) -> Descriptor {
    Descriptor {
        n,
        path_decl: PathDecl {
            n,
            paths: shared_path(),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree,
        tlv,
    }
}

/// wpkh(@0): the simplest card.
fn wpkh_card() -> Descriptor {
    desc(
        1,
        node(Tag::Wpkh, Body::KeyArg { index: 0 }),
        TlvSection::new_empty(),
    )
}

/// #2. `wsh(thresh(2,pk(@0),s:pk(@1),sln:older(210000)))`: 210000 has bits above
/// BIP-68's 16-bit value + type bit, so consensus enforces 210000 & 0xFFFF =
/// 13392. The timelock check must reach into `thresh` (a `Body::Variable`).
/// Control: the same `older` directly under `wsh(and_v(..))` is refused.
#[test]
fn bughunt_md_02_thresh_older_truncation_checked() {
    let older = node(Tag::Older, Body::Timelock(210_000));
    let direct = node(
        Tag::Wsh,
        Body::Children(vec![node(
            Tag::AndV,
            Body::Children(vec![key(Tag::PkK, 0), older.clone()]),
        )]),
    );
    assert!(
        md_codec::validate::validate_relative_timelocks(&direct).is_err(),
        "control: a bare older(210000) must be refused"
    );
    let nested = node(
        Tag::Wsh,
        Body::Children(vec![node(
            Tag::Thresh,
            Body::Variable {
                k: 2,
                children: vec![key(Tag::PkK, 0), key(Tag::PkK, 1), older],
            },
        )]),
    );
    let r = md_codec::validate::validate_relative_timelocks(&nested);
    assert!(
        matches!(r, Err(md_codec::Error::RelativeTimelockTruncated { .. })),
        "older(210000) inside thresh must be RelativeTimelockTruncated, got {r:?}"
    );
}

/// #5. `wsh(and_v(v:pk(@0),or_d(pk(@1),older(10))))`. The or_d alternatives are
/// separate spend paths: no single branch may need @1 AND the older(10) lock.
/// OPEN PRODUCT QUESTION: the exact expected branch shape (cross product
/// {@0,@1} / {@0,older 10}, or something else); only the wrong merged branch
/// is asserted absent.
#[test]
fn bughunt_md_05_split_branches_nested_or_not_merged() {
    let d = desc(
        2,
        node(
            Tag::Wsh,
            Body::Children(vec![node(
                Tag::AndV,
                Body::Children(vec![
                    node(Tag::Verify, Body::Children(vec![key(Tag::PkK, 0)])),
                    node(
                        Tag::OrD,
                        Body::Children(vec![
                            key(Tag::PkK, 1),
                            node(Tag::Older, Body::Timelock(10)),
                        ]),
                    ),
                ]),
            )]),
        ),
        TlvSection::new_empty(),
    );
    let shape = md_codec::policy_shape::policy_shape(&d);
    println!("{shape:?}");
    for b in &shape.branches {
        assert!(
            !b.slots.contains(&1) || b.locks.is_empty(),
            "a branch needs @1 AND older(10), but they are or_d alternatives: {b:?}"
        );
    }
}

/// #7. n=2, tree multi(@1,@0), fingerprints [(5, fp)]: index 5 is out of range
/// for n=2. Must be a typed error (or Ok), never a panic.
#[test]
fn bughunt_md_07_canonicalize_out_of_range_tlv_index_no_panic() {
    let mut tlv = TlvSection::new_empty();
    tlv.fingerprints = Some(vec![(5u8, [1, 2, 3, 4])]);
    let mut d = desc(
        2,
        node(
            Tag::Wsh,
            Body::Children(vec![node(
                Tag::Multi,
                Body::MultiKeys {
                    k: 2,
                    indices: vec![1, 0],
                },
            )]),
        ),
        tlv,
    );
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        md_codec::canonicalize::canonicalize_placeholder_indices(&mut d)
    }));
    assert!(
        r.is_ok(),
        "canonicalize panicked on an out-of-range TLV index"
    );
    assert!(
        r.unwrap().is_err(),
        "expected a typed error for index 5 with n=2"
    );
}

/// #8. An unknown TLV the writer cannot represent faithfully must be refused
/// at write, or round-trip equal. OPEN PRODUCT QUESTION: refuse vs. error type
/// (only "silent wrong round-trip" is asserted absent).
fn tlv_roundtrip_ok(unknown: Vec<(u8, Vec<u8>, usize)>) -> Result<(), String> {
    let mut t = TlvSection::new_empty();
    t.unknown = unknown;
    let mut w = BitWriter::new();
    if t.write(&mut w, 1).is_err() {
        return Ok(()); // refused at write: acceptable
    }
    let bits = w.bit_len();
    let bytes = w.into_bytes();
    let mut r = BitReader::with_bit_limit(&bytes, bits);
    match TlvSection::read(&mut r, 1, 2) {
        Ok(back) if back == t => Ok(()),
        Ok(back) => Err(format!("round-trip changed it: wrote {t:?}, read {back:?}")),
        Err(e) => Err(format!("wrote it, then could not read it back: {e:?}")),
    }
}

#[test]
fn bughunt_md_08_unknown_tlv_tag_above_31_not_silently_truncated() {
    // 0x21 = 33; the tag field is 5 bits, so it is read back as 1 (fingerprints).
    let r = tlv_roundtrip_ok(vec![(0x21, vec![0xAB], 8)]);
    assert!(r.is_ok(), "{}", r.unwrap_err());
}

#[test]
fn bughunt_md_08b_unknown_tlv_tag_duplicating_known_tag_not_written() {
    // Tag 1 (fingerprints) as an "unknown" entry.
    let r = tlv_roundtrip_ok(vec![(1, vec![0xAB], 8)]);
    assert!(r.is_ok(), "{}", r.unwrap_err());
}

/// #9. LP4-ext extension form carrying 5 in 22 bits (canonical is L=3, 3 bits).
/// OPEN PRODUCT QUESTION: strict-canonical vs. lenient reader policy; the
/// report says one value must have one wire form.
#[test]
fn bughunt_md_09_varint_noncanonical_extension_refused() {
    let mut w = BitWriter::new();
    w.write_bits(15, 4); // L = 15: extension
    w.write_bits(8, 4); // l_high = 8
    w.write_bits(5, 14); // payload_low = 5
    w.write_bits(0, 8); // payload_high = 0 (8 bits)
    let bits = w.bit_len();
    let bytes = w.into_bytes();
    let mut r = BitReader::with_bit_limit(&bytes, bits);
    let v = md_codec::varint::read_varint(&mut r);
    assert!(
        v.is_err(),
        "non-canonical 22-bit encoding of 5 must be refused, decoded as {v:?}"
    );
}

/// #16 (lead L16). `c as u8` truncates a non-ASCII char to its low byte, so
/// U+01xx (xx = a real codex32 symbol) must NOT decode as symbol xx.
fn swap_first_data_char(s: &str) -> String {
    let (head, rest) = s.split_at(3); // "md1"
    let mut it = rest.chars();
    let c = it.next().unwrap();
    let alias = char::from_u32(0x100 + c as u32).unwrap();
    format!("{head}{alias}{}", it.as_str())
}

#[test]
fn bughunt_md_16_single_string_non_ascii_alias_refused() {
    let s = md_codec::encode::encode_md1_string(&wpkh_card()).unwrap();
    let bad = swap_first_data_char(&s);
    assert_ne!(bad, s);
    let r = md_codec::decode::decode_md1_string(&bad);
    assert!(
        r.is_err(),
        "non-ASCII alias {bad:?} decoded as if it were {s:?}"
    );
}

#[test]
fn bughunt_md_16b_chunk_non_ascii_alias_refused() {
    let chunks = md_codec::chunk::split(&wpkh_card()).unwrap();
    let mut bad: Vec<String> = chunks.clone();
    bad[0] = swap_first_data_char(&chunks[0]);
    assert_ne!(bad[0], chunks[0]);
    let refs: Vec<&str> = bad.iter().map(String::as_str).collect();
    let r = md_codec::chunk::reassemble(&refs);
    assert!(
        r.is_err(),
        "non-ASCII alias in a chunk decoded: {:?}",
        bad[0]
    );
}
