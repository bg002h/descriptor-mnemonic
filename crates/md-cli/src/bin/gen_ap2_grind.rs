//! AP2 fixture generator — plan P0 item 4.
//!
//! Builds the SPEC `design/SPEC_seat_auto_partition.md` row 9 fixture: a
//! chunk-set id group holding a GENUINE ambiguity (two distinct verified
//! `mk_codec::decode` covers), reachable ONLY by an attacker who spends a
//! `~2^32` grind — never by accident (r3-I4's "one-grind" construction,
//! `design/agent-reports/R0-seat-auto-partition-r3.md` §r3-I4).
//!
//! ## The construction (r3-I4)
//!
//! One `n = 3` total-chunks class. Cards A and B SHARE an identical
//! 13-stub chunk 0 (SPEC row 3's threshold: header + 13 stubs = 54 bytes
//! ≥ 53, so chunk 0 is a pure function of the (shared) stub list — r3-M4).
//! Card C carries a DIFFERENT 13-stub list. Per-index canonical-piece
//! counts are `[2, 3, 3]` (index 0 = {A0(=B0), C0}, indexes 1/2 = {A,B,C}
//! each distinct), so `k = 3` and `{A, B, C}` is the honest, exact-k cover.
//!
//! Grind card C's OWN stub bytes (never A's or B's — never xpub bytes) so
//! that `C0 ++ A1 ++ A2` ALSO verifies under `mk_codec::decode`: this
//! "frankencard" `F` reuses C's chunk 0 with A's chunks 1 and 2 verbatim,
//! so it verifies iff `SHA-256(C0 ++ A1 ++ A2[..-4])[0..4]` collides with
//! A's own (unchanged) trailing cross-chunk hash, `A2[-4..]`. One stub (4
//! bytes = 32 bits) of C's list is the grind variable — a full `2^32`
//! space entirely inside chunk 0 (measured: chunk 0 is 53 bytes, equal to
//! the 1-byte header, the 1-byte stub count, and the first 51 of 52 stub
//! bytes, so stub index 11 — the 12th of 13 — sits entirely inside chunk
//! 0's byte range).
//!
//! `{F, B, C}` then ALSO covers all 8 distinct pieces (index 0: F=C0,
//! B=A0=B0; indexes 1/2: F=A-piece, B=B-piece, C=C-piece — all three
//! present), giving TWO distinct 3-card covers over the same 9 supplied
//! strings: `{A,B,C}` and `{F,B,C}`. P0 does not implement the partition
//! engine that would notice this (P1 does); P0's own shape test only
//! asserts the mechanically checkable half — that `[C0, A1, A2]` verifies
//! under `mk_codec::decode` at all, i.e. that a hidden extra candidate
//! genuinely exists in the committed fixture.
//!
//! ## Determinism
//!
//! Every input (stub bytes, fingerprints, xpub seeds, the grind's starting
//! counter and iteration order) is a fixed constant below, so a re-run
//! finds the SAME counter and emits a byte-identical fixture — the P0 gate's
//! determinism guard.
//!
//! ## Regeneration
//!
//! ```text
//! cargo run -p md-cli --bin gen_ap2_grind --release > \
//!   crates/md-cli/tests/fixtures/seating/v-ap2.txt
//! ```
//!
//! Measured runtime and the resulting fixture's SHA-256 are recorded in
//! `design/agent-reports/impl-seat-ap-p0.md`.

use bitcoin::NetworkKind;
use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
use bitcoin::hashes::{Hash, sha256};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use mk_codec::bytecode::encode_bytecode;
use mk_codec::string_layer::{StringLayerHeader, bytes_to_5bit, encode_5bit_to_string};
use mk_codec::{CHUNKED_FRAGMENT_LONG_BYTES, KeyCard};
use std::str::FromStr;
use std::time::Instant;

/// Pinned chunk-set id for the fixture. Five hex digits, distinct from
/// every other P0 fixture's id.
const CHUNK_SET_ID: u32 = 0x0A_1999;

/// Stub count shared by A, B and C — SPEC row 3's threshold (chunk 0
/// becomes a pure function of header + stubs at N = 13; `mk` CLI-measured
/// against this vendored mk-codec, matching `v-ap-shared.txt`).
const N_STUBS: u32 = 13;

/// Index (0-based) of the stub that carries the grind counter. Chosen so
/// its 4 bytes sit ENTIRELY inside chunk 0's 53-byte range: chunk 0 covers
/// bytecode offsets `[0, 53)`, stub `i` occupies `[2 + 4*i, 6 + 4*i)`, and
/// stub 12 (the 13th, last of `N_STUBS`) is the first to straddle the
/// boundary (`[50, 54)`, one byte into chunk 1) -- so any index `<= 11`
/// fits fully; index 10 leaves margin. Not trusted from this comment
/// alone: re-derived and `assert!`-checked at runtime below.
const COUNTER_STUB_INDEX: usize = 10;

fn xpub_from_seed(seed: u8, depth: u8, child: ChildNumber) -> Xpub {
    let secp = Secp256k1::new();
    let mut sk_bytes = [0x11u8; 32];
    sk_bytes[0] = seed;
    sk_bytes[31] = seed ^ 0x5A;
    let sk = SecretKey::from_slice(&sk_bytes).expect("valid secret key bytes");
    Xpub {
        network: NetworkKind::Main,
        depth,
        parent_fingerprint: Fingerprint::from([seed, seed, seed, seed]),
        child_number: child,
        public_key: PublicKey::from_secret_key(&secp, &sk),
        chain_code: ChainCode::from([seed; 32]),
    }
}

fn path0() -> DerivationPath {
    DerivationPath::from_str("48'/0'/0'/2'").expect("valid path literal")
}

fn make_card(stubs: Vec<[u8; 4]>, fp_byte: u8, xpub_seed: u8) -> KeyCard {
    let path = path0();
    let depth = path.into_iter().count() as u8;
    let child = *path.into_iter().last().expect("non-empty path");
    KeyCard::new(
        stubs,
        Some(Fingerprint::from([fp_byte; 4])),
        path,
        xpub_from_seed(xpub_seed, depth, child),
    )
}

/// `bytecode ++ SHA-256(bytecode)[0..4]`, chunked into `CHUNKED_FRAGMENT_LONG_BYTES`
/// pieces — the documented rule `mk_codec::string_layer::chunk::split_into_chunks`
/// implements (pure arithmetic; no codec internals mirrored).
fn stream_and_chunks(bytecode: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let hash = sha256::Hash::hash(bytecode);
    let mut stream = bytecode.to_vec();
    stream.extend_from_slice(&hash.to_byte_array()[..4]);
    let chunks: Vec<Vec<u8>> = stream
        .chunks(CHUNKED_FRAGMENT_LONG_BYTES)
        .map(<[u8]>::to_vec)
        .collect();
    (stream, chunks)
}

fn chunk_string(chunk_set_id: u32, total_chunks: u8, chunk_index: u8, fragment: &[u8]) -> String {
    let header = StringLayerHeader::Chunked {
        version: 0,
        chunk_set_id,
        total_chunks,
        chunk_index,
    };
    let mut data = header.to_5bit_symbols();
    data.extend(bytes_to_5bit(fragment));
    encode_5bit_to_string(&data).expect("valid chunk symbols")
}

fn stub_list(base_tag: u8, counter: u32) -> Vec<[u8; 4]> {
    let mut stubs: Vec<[u8; 4]> = (0..N_STUBS)
        .map(|j| [base_tag, 0xF0, (j >> 8) as u8, j as u8])
        .collect();
    stubs[COUNTER_STUB_INDEX] = counter.to_be_bytes();
    stubs
}

fn main() {
    // Card A and B: IDENTICAL 13-stub list (shared chunk 0), distinct fp + xpub.
    let shared_stubs: Vec<[u8; 4]> = stub_list(0xAB, 0);
    let card_a = make_card(shared_stubs.clone(), 0x11, 0xA1);
    let card_b = make_card(shared_stubs, 0x22, 0xB2);

    let bytecode_a = encode_bytecode(&card_a).expect("card A must encode");
    let bytecode_b = encode_bytecode(&card_b).expect("card B must encode");
    let (stream_a, chunks_a) = stream_and_chunks(&bytecode_a);
    let (_stream_b, chunks_b) = stream_and_chunks(&bytecode_b);
    assert_eq!(chunks_a.len(), 3, "card A must be a 3-chunk (n=3) card");
    assert_eq!(chunks_b.len(), 3, "card B must be a 3-chunk (n=3) card");
    assert_eq!(
        chunks_a[0], chunks_b[0],
        "A and B must share an identical chunk 0"
    );

    // Sanity: the grind counter's 4 bytes at COUNTER_STUB_INDEX must sit
    // ENTIRELY inside chunk 0's byte range (offset 2 + 4*index .. +4).
    let counter_offset = 2 + 4 * COUNTER_STUB_INDEX;
    assert!(
        counter_offset + 4 <= CHUNKED_FRAGMENT_LONG_BYTES,
        "grind counter stub (index {COUNTER_STUB_INDEX}, bytes [{counter_offset}..{})) must fit \
         inside chunk 0's {CHUNKED_FRAGMENT_LONG_BYTES}-byte range",
        counter_offset + 4
    );

    let a_tail = &stream_a[CHUNKED_FRAGMENT_LONG_BYTES..]; // A1 ++ A2 (unchanged).
    let a_hash_tail: [u8; 4] = stream_a[stream_a.len() - 4..].try_into().expect("4 bytes");

    // Template for chunk 0's 53 bytes at counter = 0 -- everything except
    // the counter field is FIXED, so the hot loop below never calls
    // `encode_bytecode`/EC key derivation again (both are per-iteration
    // costs that would make a 2^32 grind take hours, not seconds; only
    // the SHA-256 itself belongs in the hot path). `candidate_template` is
    // `c0(53) ++ a_tail(len stream_a - 53)`, so hashing bytes `[0..len-4]`
    // and comparing the trailing 4 to `a_hash_tail` reproduces exactly the
    // check `mk_codec::decode` performs on `(C0, A1, A2)`.
    let template_card_c = make_card(stub_list(0xCD, 0), 0x33, 0xC3);
    let template_bytecode_c =
        encode_bytecode(&template_card_c).expect("card C template must encode");
    assert_eq!(
        template_bytecode_c.len(),
        bytecode_a.len(),
        "C's bytecode length must match A's (same N_STUBS/path/fp shape)"
    );
    let mut candidate_template = template_bytecode_c[..CHUNKED_FRAGMENT_LONG_BYTES].to_vec();
    candidate_template.extend_from_slice(a_tail);
    assert_eq!(candidate_template.len(), stream_a.len());

    let n_threads = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    eprintln!(
        "grinding C's chunk 0 against A's trailing hash (~2^32 SHA-256, {n_threads} threads)..."
    );
    let start = Instant::now();
    let found: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    let span = (u32::MAX as u64 + 1).div_ceil(n_threads as u64);
    std::thread::scope(|scope| {
        for t in 0..n_threads as u64 {
            let lo = t * span;
            let hi = ((t + 1) * span).min(u32::MAX as u64 + 1);
            let template = candidate_template.clone();
            let found = &found;
            scope.spawn(move || {
                use std::sync::atomic::Ordering;
                let mut buf = template;
                let hash_len = buf.len() - 4;
                for counter in lo..hi {
                    if counter % 4096 == 0 && found.load(Ordering::Relaxed) != u64::MAX {
                        return;
                    }
                    buf[counter_offset..counter_offset + 4]
                        .copy_from_slice(&(counter as u32).to_be_bytes());
                    let computed = sha256::Hash::hash(&buf[..hash_len]);
                    if computed.to_byte_array()[..4] == a_hash_tail {
                        found.store(counter, Ordering::SeqCst);
                        return;
                    }
                }
            });
        }
    });
    let elapsed = start.elapsed();
    let match_counter = found.load(std::sync::atomic::Ordering::SeqCst);
    assert_ne!(
        match_counter,
        u64::MAX,
        "grind exhausted the full 32-bit space without a match"
    );
    let match_counter = match_counter as u32;
    eprintln!(
        "grind found a match at counter {match_counter} in {:.3}s ({n_threads} threads)",
        elapsed.as_secs_f64()
    );

    let c_stubs = stub_list(0xCD, match_counter);
    let card_c = make_card(c_stubs, 0x33, 0xC3);
    let bytecode_c = encode_bytecode(&card_c).expect("card C must encode");
    let (_stream_c, chunks_c) = stream_and_chunks(&bytecode_c);
    assert_eq!(chunks_c.len(), 3, "card C must be a 3-chunk (n=3) card");
    assert_ne!(
        chunks_c[0], chunks_a[0],
        "C's chunk 0 must differ from A/B's (a DIFFERENT stub list)"
    );
    {
        let mut check = chunks_c[0].clone();
        check.extend_from_slice(a_tail);
        let computed = sha256::Hash::hash(&check[..check.len() - 4]);
        assert_eq!(
            computed.to_byte_array()[..4],
            a_hash_tail,
            "re-derived C0 must reproduce the grind's hash match"
        );
    }

    // Verify the frankencard F = (C0, A1, A2) genuinely verifies, and is a
    // DIFFERENT decoded card from A, B and C -- the ambiguity itself.
    let f_strings: Vec<String> = vec![
        chunk_string(CHUNK_SET_ID, 3, 0, &chunks_c[0]),
        chunk_string(CHUNK_SET_ID, 3, 1, &chunks_a[1]),
        chunk_string(CHUNK_SET_ID, 3, 2, &chunks_a[2]),
    ];
    let f_refs: Vec<&str> = f_strings.iter().map(String::as_str).collect();
    let f_card =
        mk_codec::decode(&f_refs).expect("F = (C0, A1, A2) must verify under mk_codec::decode");
    assert_ne!(f_card, card_a, "F must be a DIFFERENT card from A");
    assert_ne!(f_card, card_b, "F must be a DIFFERENT card from B");
    assert_ne!(f_card, card_c, "F must be a DIFFERENT card from C");

    // Emit the 9 supplied strings (A's 3, B's 3, C's 3). F is never
    // separately supplied -- it is implicit in the recombination of pieces
    // ALREADY present (C0 from C, A1/A2 from A), which is the whole point:
    // the ambiguity is reachable from what a scanning operator actually
    // typed, not from a 10th string nobody supplied.
    let mut out = String::new();
    out.push_str("# V-AP2 — SPEC row 9: a GENUINE ambiguity (two distinct verified covers)\n");
    out.push_str("# GENERATED by crates/md-cli/src/bin/gen_ap2_grind.rs.\n");
    out.push_str("# Regenerate: cargo run -p md-cli --bin gen_ap2_grind --release > \\\n");
    out.push_str("#   crates/md-cli/tests/fixtures/seating/v-ap2.txt\n");
    // Wall-clock elapsed time is DELIBERATELY not embedded here: it is not
    // deterministic run to run (thread scheduling), and the determinism
    // guard (plan P0 gate) diffs this exact file against a fresh
    // regeneration. Only `match_counter` (the ~2^32 search's outcome, fully
    // determined by the fixed inputs above) belongs in committed provenance;
    // measured timings are recorded in the implementation report instead.
    out.push_str(&format!(
        "# Deterministic: every input is a fixed constant in the script; the grind counter\n\
         # that produced this file was {match_counter} (a ~2^32 search space).\n"
    ));
    out.push_str("# 9 mk1 strings: card A (3 chunks), card B (3 chunks, shares chunk 0 with A),\n");
    out.push_str("# card C (3 chunks, a DIFFERENT stub list). The hidden extra verified\n");
    out.push_str(
        "# candidate F = (C's chunk 0, A's chunk 1, A's chunk 2) is never supplied as a\n",
    );
    out.push_str("# 10th string -- it is implicit in these 9. r3-I4's one-grind construction.\n");
    out.push_str("# card A:\n");
    for (i, frag) in chunks_a.iter().enumerate() {
        out.push_str(&chunk_string(CHUNK_SET_ID, 3, i as u8, frag));
        out.push('\n');
    }
    out.push_str("# card B:\n");
    for (i, frag) in chunks_b.iter().enumerate() {
        out.push_str(&chunk_string(CHUNK_SET_ID, 3, i as u8, frag));
        out.push('\n');
    }
    out.push_str("# card C:\n");
    for (i, frag) in chunks_c.iter().enumerate() {
        out.push_str(&chunk_string(CHUNK_SET_ID, 3, i as u8, frag));
        out.push('\n');
    }
    print!("{out}");
}
