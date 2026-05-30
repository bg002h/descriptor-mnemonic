//! Theme 2 — BCH adversarial. Drive correction via the public decode_with_correction.
mod common;

use common::corrupt_chunk_at;
use md_codec::chunk::{decode_with_correction, split};
use md_codec::encode::Descriptor;
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

fn wpkh_descriptor(depth: u8) -> Descriptor {
    Descriptor {
        n: 1,
        path_decl: PathDecl {
            n: 1,
            paths: PathDeclPaths::Shared(OriginPath {
                components: (0..depth)
                    .map(|i| PathComponent {
                        hardened: true,
                        value: (i as u32) + 1,
                    })
                    .collect(),
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

fn multi_chunk_descriptor() -> Descriptor {
    // 6 Divergent cosigners × 15 hardened components → ≥4 chunks.
    let paths = (0..6u32)
        .map(|c| OriginPath {
            components: (0..15u32)
                .map(|i| PathComponent {
                    hardened: true,
                    value: c * 100 + i + 1,
                })
                .collect(),
        })
        .collect();
    Descriptor {
        n: 6,
        path_decl: PathDecl {
            n: 6,
            paths: PathDeclPaths::Divergent(paths),
        },
        use_site_path: UseSitePath::standard_multipath(),
        tree: Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::MultiKeys {
                    k: 2,
                    indices: (0..6).collect(),
                },
            }]),
        },
        tlv: TlvSection::new_empty(),
    }
}

// T2a — 1..=4-error correction across 3 lengths, through public decode_with_correction.
#[test]
fn t2a_correct_1_to_4_errors_across_lengths() {
    for d in [
        wpkh_descriptor(3),
        wpkh_descriptor(15),
        multi_chunk_descriptor(),
    ] {
        let chunks = split(&d).unwrap();
        for count in 1..=4usize {
            let mut cs = chunks.clone();
            for p in 1..=count {
                cs[0] = corrupt_chunk_at(&cs[0], p, 0x1F);
            }
            let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
            let (got, details) = decode_with_correction(&refs)
                .unwrap_or_else(|e| panic!("t={count} must correct: {e:?}"));
            assert_eq!(got, d, "t={count} recovered a different descriptor");
            assert!(details.len() >= count, "expected >= {count} corrections");
        }
    }
}

// T2b — correction inside the trailing 13-symbol checksum region.
#[test]
fn t2b_correct_checksum_region_errors() {
    let d = wpkh_descriptor(15);
    let chunks = split(&d).unwrap();
    let dp_len = chunks[0].chars().count() - 3; // post-HRP data-part length
    let mut cs = chunks.clone();
    cs[0] = corrupt_chunk_at(&cs[0], dp_len - 1, 0x1F);
    cs[0] = corrupt_chunk_at(&cs[0], dp_len - 7, 0x1F);
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    let (got, _) = decode_with_correction(&refs).expect("checksum-region errors correct");
    assert_eq!(got, d);
}

// A 5-error pattern (data-part positions) VERIFIED-UNCORRECTABLE for wpkh_descriptor(15)'s
// single chunk. A 5-error pattern is NOT guaranteed uncorrectable (Berlekamp-Massey may
// miscorrect). If T2d fires because this pattern starts to (mis)correct after a fixture/fmt
// change, pick another 5-position set (try [2,5,8,11,14] / [1,3,6,9,12] / …) until
// decode_with_correction errs, and update this const + comment.
const UNCORRECTABLE_5ERR: [usize; 5] = [1, 4, 7, 10, 13];

// T2c — randomized 5–8-error sweep. ASSERT != Ok(original) (NOT is_err — md miscorrects to a
// different codeword at ~2^-26). Seeded xorshift, no rand dep.
#[test]
fn t2c_five_to_eight_errors_never_return_original() {
    let d = wpkh_descriptor(15);
    let original = d.clone();
    let chunks = split(&d).unwrap();
    let dp_len = chunks[0].chars().count() - 3;
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for trial in 0..300u32 {
        for n_err in 5..=8usize {
            let mut positions = std::collections::BTreeSet::new();
            while positions.len() < n_err {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                positions.insert((x as usize) % dp_len);
            }
            let mut c0 = chunks[0].clone();
            for &p in &positions {
                c0 = corrupt_chunk_at(&c0, p, ((x as u8) | 1) & 0x1F);
            }
            let mut cs = chunks.clone();
            cs[0] = c0;
            let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
            if let Ok((got, _)) = decode_with_correction(&refs) {
                assert_ne!(
                    got, original,
                    "trial {trial} n_err {n_err}: 5-8 errors silently returned the original"
                );
            }
        }
    }
}

// T2d — the verified-uncorrectable deterministic 5-error pattern → Err.
#[test]
fn t2d_deterministic_five_error_is_err() {
    let d = wpkh_descriptor(15);
    let chunks = split(&d).unwrap();
    let mut c0 = chunks[0].clone();
    for p in UNCORRECTABLE_5ERR {
        c0 = corrupt_chunk_at(&c0, p, 0x1F);
    }
    let mut cs = chunks.clone();
    cs[0] = c0;
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    assert!(
        decode_with_correction(&refs).is_err(),
        "UNCORRECTABLE_5ERR must be uncorrectable — if this fires, the chunk symbols changed; \
         pick another 5-position pattern that errs and update the const (see its doc-comment)"
    );
}

// T2h — multi-chunk: 2 different chunks each ≤ 4 errors → Ok(original).
#[test]
fn t2h_multi_chunk_two_corrupted_within_t() {
    let d = multi_chunk_descriptor();
    let chunks = split(&d).unwrap();
    assert!(chunks.len() >= 2);
    let mut cs = chunks.clone();
    cs[0] = corrupt_chunk_at(&cs[0], 2, 0x1F);
    let li = cs.len() - 1;
    cs[li] = corrupt_chunk_at(&cs[li], 2, 0x1F);
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    let (got, _) = decode_with_correction(&refs).expect("each chunk within t corrects");
    assert_eq!(got, d);
}

// T2i — one chunk over t in a valid multi-chunk set: never silently yields the original
// (atomic-abort intent). Robust != Ok(original) invariant; Err is the expected abort, a rare
// chunk-0 miscorrection surfaces as Ok(different), still ≠ original. (if-let avoids Error: PartialEq.)
#[test]
fn t2i_one_chunk_over_t_never_returns_original() {
    let d = multi_chunk_descriptor();
    let chunks = split(&d).unwrap();
    let mut cs = chunks.clone();
    for p in [1usize, 4, 7, 10, 13] {
        cs[0] = corrupt_chunk_at(&cs[0], p, 0x1F);
    }
    let refs: Vec<&str> = cs.iter().map(String::as_str).collect();
    if let Ok((got, _)) = decode_with_correction(&refs) {
        assert_ne!(
            got, d,
            "a 5-error chunk-0 corruption must never reassemble to the original"
        );
    }
}
