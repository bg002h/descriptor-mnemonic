//! Deterministic, re-runnable seed-corpus generator for the md-codec fuzz
//! targets. md phase of the constellation stress-fuzz program (Cycle C).
//!
//! Run with:
//!     cd fuzz && cargo +nightly-2026-04-27 test --test gen_corpus
//!
//! It (1) builds a fixed set of valid single- and multi-chunk md1
//! descriptors via the public structural API, (2) writes seed files into
//! the cargo-fuzz default `corpus/<target>/` layout, and (3) — THE GATE
//! (R0 [I6] / round-2 minor) — asserts every committed seed passes the
//! SAME split-then-call the corresponding target uses. A seed that does not
//! round-trip is a generation bug and fails the test loudly.
//!
//! Determinism: descriptors are built from fixed literals (no RNG); the
//! same run produces byte-identical seeds every time, so re-running never
//! churns the committed corpus.

use std::fs;
use std::path::{Path, PathBuf};

use md_codec::chunk::{decode_with_correction, reassemble, split};
use md_codec::decode::{decode_md1_string, decode_payload};
use md_codec::encode::{Descriptor, encode_md1_string, encode_payload};
use md_codec::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
use md_codec::tag::Tag;
use md_codec::tlv::TlvSection;
use md_codec::tree::{Body, Node};
use md_codec::use_site_path::UseSitePath;

// ---------------------------------------------------------------------------
// Structural builders (mirror crates/md-codec/tests/common/mod.rs).
// ---------------------------------------------------------------------------

fn keyarg(tag: Tag, index: u8) -> Node {
    Node {
        tag,
        body: Body::KeyArg { index },
    }
}

fn multikeys(tag: Tag, k: u8, indices: Vec<u8>) -> Node {
    Node {
        tag,
        body: Body::MultiKeys { k, indices },
    }
}

fn wrap(tag: Tag, inner: Node) -> Node {
    Node {
        tag,
        body: Body::Children(vec![inner]),
    }
}

/// 65-byte xpub: 32-byte chain-code (filled with `seed`) || 33-byte
/// compressed pubkey = G (the secp256k1 generator). Structurally valid, so
/// it passes `validate_xpub_bytes` (mirrors wallet_policy.rs::make_xpub).
fn make_xpub(seed: u8) -> [u8; 65] {
    let mut x = [0u8; 65];
    for b in x[0..32].iter_mut() {
        *b = seed;
    }
    // 0x02 || G.x — the compressed generator point.
    x[32] = 0x02;
    x[33..65].copy_from_slice(&[
        0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87, 0x0B,
        0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8,
        0x17, 0x98,
    ]);
    x
}

/// Shared origin path with `depth` hardened components (non-divergent).
fn shared_path(n: u8, depth: u8) -> PathDecl {
    PathDecl {
        n,
        paths: PathDeclPaths::Shared(OriginPath {
            components: (0..depth)
                .map(|i| PathComponent {
                    hardened: true,
                    value: u32::from(i) + 1,
                })
                .collect(),
        }),
    }
}

/// Assemble a template-only descriptor (no pubkeys TLV) from a tree.
fn descriptor(n: u8, path: PathDecl, tree: Node, tlv: TlvSection) -> Descriptor {
    Descriptor {
        n,
        path_decl: path,
        use_site_path: UseSitePath::standard_multipath(),
        tree,
        tlv,
    }
}

/// The fixed valid-descriptor catalog. Mix of root tags + a wallet-policy
/// (pubkeys-TLV) descriptor whose 65-byte-per-key payload guarantees a
/// multi-chunk split (> SINGLE_STRING_PAYLOAD_BIT_LIMIT = 320 bits).
fn catalog() -> Vec<(&'static str, Descriptor)> {
    let mut out: Vec<(&'static str, Descriptor)> = Vec::new();

    // wpkh(@0) template-only.
    out.push((
        "wpkh",
        descriptor(
            1,
            shared_path(1, 1),
            keyarg(Tag::Wpkh, 0),
            TlvSection::new_empty(),
        ),
    ));

    // pkh(@0) template-only.
    out.push((
        "pkh",
        descriptor(
            1,
            shared_path(1, 1),
            keyarg(Tag::Pkh, 0),
            TlvSection::new_empty(),
        ),
    ));

    // wsh(multi(2,@0,@1)).
    out.push((
        "wsh_multi_2of2",
        descriptor(
            2,
            shared_path(2, 1),
            wrap(Tag::Wsh, multikeys(Tag::Multi, 2, vec![0, 1])),
            TlvSection::new_empty(),
        ),
    ));

    // wsh(multi(2,@0,@1,@2)).
    out.push((
        "wsh_multi_2of3",
        descriptor(
            3,
            shared_path(3, 1),
            wrap(Tag::Wsh, multikeys(Tag::Multi, 2, vec![0, 1, 2])),
            TlvSection::new_empty(),
        ),
    ));

    // wsh(sortedmulti(2,@0,@1,@2)).
    out.push((
        "wsh_sortedmulti_2of3",
        descriptor(
            3,
            shared_path(3, 1),
            wrap(Tag::Wsh, multikeys(Tag::SortedMulti, 2, vec![0, 1, 2])),
            TlvSection::new_empty(),
        ),
    ));

    // sh(wsh(multi(2,@0,@1))).
    out.push((
        "sh_wsh_multi_2of2",
        descriptor(
            2,
            shared_path(2, 1),
            wrap(
                Tag::Sh,
                wrap(Tag::Wsh, multikeys(Tag::Multi, 2, vec![0, 1])),
            ),
            TlvSection::new_empty(),
        ),
    ));

    // tr(@0) keyonly (non-NUMS).
    out.push((
        "tr_keyonly",
        descriptor(
            1,
            shared_path(1, 1),
            Node {
                tag: Tag::Tr,
                body: Body::Tr {
                    is_nums: false,
                    key_index: 0,
                    tree: None,
                },
            },
            TlvSection::new_empty(),
        ),
    ));

    // wsh(multi(2,@0,@1)) WALLET-POLICY (pubkeys TLV populated) — 2×65 bytes
    // of key material guarantees a multi-chunk split.
    let mut wp_tlv = TlvSection::new_empty();
    wp_tlv.pubkeys = Some(vec![(0u8, make_xpub(0x11)), (1u8, make_xpub(0x22))]);
    out.push((
        "wsh_multi_wallet_policy_chunked",
        descriptor(
            2,
            shared_path(2, 1),
            wrap(Tag::Wsh, multikeys(Tag::Multi, 2, vec![0, 1])),
            wp_tlv,
        ),
    ));

    out
}

fn corpus_dir(target: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join(target)
}

/// Write `bytes` to `dir/name`, creating the directory if needed.
fn write_seed(dir: &Path, name: &str, bytes: &[u8]) {
    fs::create_dir_all(dir).expect("create corpus dir");
    fs::write(dir.join(name), bytes).expect("write seed");
}

#[test]
fn gen_corpus() {
    let catalog = catalog();

    let dir_string = corpus_dir("md1_decode_string");
    let dir_payload = corpus_dir("md1_decode_payload");
    let dir_reassemble = corpus_dir("md1_reassemble");
    let dir_correction = corpus_dir("md1_decode_with_correction");

    let mut single_count = 0usize;
    let mut payload_count = 0usize;
    let mut reassemble_count = 0usize;
    let mut correction_count = 0usize;

    for (name, d) in &catalog {
        // --- md1_decode_string seed: the whole md1 string. ---
        let s = encode_md1_string(d)
            .unwrap_or_else(|e| panic!("gen-corpus: {name} failed to encode md1 string: {e}"));
        // GATE: the seed must decode via the SAME entry the target uses.
        decode_md1_string(&s)
            .unwrap_or_else(|e| panic!("gen-corpus GATE: {name} md1 string does not decode: {e}"));
        write_seed(&dir_string, &format!("{name}.md1"), s.as_bytes());
        single_count += 1;

        // --- md1_decode_payload seed: 2-byte LE total_bits prefix + payload
        //     bytes. The target clamps, so we prefix the EXACT canonical
        //     total_bits (always <= len*8) which the clamp leaves untouched. ---
        let (payload_bytes, total_bits) = encode_payload(d)
            .unwrap_or_else(|e| panic!("gen-corpus: {name} failed to encode payload: {e}"));
        // total_bits fits in u16 for these descriptors (sanity-check the gate
        // we rely on for the LE prefix).
        assert!(
            total_bits <= usize::from(u16::MAX),
            "gen-corpus: {name} total_bits {total_bits} exceeds u16 — adjust prefix encoding"
        );
        let clamp_budget = payload_bytes.len() * 8;
        let prefixed_bits = std::cmp::min(total_bits, clamp_budget);
        // GATE: the seed must decode via the target's clamp-then-call path.
        decode_payload(&payload_bytes, prefixed_bits).unwrap_or_else(|e| {
            panic!(
                "gen-corpus GATE: {name} payload does not decode at total_bits={prefixed_bits}: {e}"
            )
        });
        let mut seed = Vec::with_capacity(2 + payload_bytes.len());
        seed.extend_from_slice(&(total_bits as u16).to_le_bytes());
        seed.extend_from_slice(&payload_bytes);
        write_seed(&dir_payload, &format!("{name}.payload"), &seed);
        payload_count += 1;

        // --- Splitter targets: chunks joined by `\n` BETWEEN (no trailing). ---
        let chunks = split(d).unwrap_or_else(|e| panic!("gen-corpus: {name} failed to split: {e}"));
        let joined = chunks.join("\n");
        let parts: Vec<&str> = joined.split('\n').collect();
        // Sanity: re-split reproduces the exact chunk strings (no trailing "").
        assert_eq!(
            parts.len(),
            chunks.len(),
            "gen-corpus: {name} split/join/split changed part count"
        );

        // md1_reassemble GATE: split('\n') -> reassemble(&parts) is Ok.
        reassemble(&parts).unwrap_or_else(|e| {
            panic!("gen-corpus GATE: {name} \\n-joined seed does not reassemble: {e}")
        });
        write_seed(&dir_reassemble, &format!("{name}.parts"), joined.as_bytes());
        reassemble_count += 1;

        // md1_decode_with_correction GATE: split('\n') ->
        // decode_with_correction(&parts) is Ok (valid cards => empty details).
        let (_d2, details) = decode_with_correction(&parts).unwrap_or_else(|e| {
            panic!("gen-corpus GATE: {name} \\n-joined seed does not decode_with_correction: {e}")
        });
        assert!(
            details.is_empty(),
            "gen-corpus: {name} valid-class seed unexpectedly reported corrections: {details:?}"
        );
        write_seed(&dir_correction, &format!("{name}.parts"), joined.as_bytes());
        correction_count += 1;
    }

    // At least one MULTI-chunk seed must exist for the splitter targets to be
    // meaningful (the wallet-policy descriptor).
    let wp = &catalog
        .iter()
        .find(|(n, _)| *n == "wsh_multi_wallet_policy_chunked")
        .expect("wallet-policy descriptor present")
        .1;
    let wp_chunks = split(wp).expect("wallet-policy splits");
    assert!(
        wp_chunks.len() >= 2,
        "gen-corpus: wallet-policy descriptor produced {} chunk(s); expected multi-chunk",
        wp_chunks.len()
    );

    eprintln!(
        "gen-corpus wrote: md1_decode_string={single_count}, md1_decode_payload={payload_count}, \
         md1_reassemble={reassemble_count}, md1_decode_with_correction={correction_count} \
         (wallet-policy split into {} chunks)",
        wp_chunks.len()
    );
}
