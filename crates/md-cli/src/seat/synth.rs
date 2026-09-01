//! P0 item 1 — the synthetic chunker (test-support only, never shipped).
//!
//! It CONSTRUCTS inputs; it is not an oracle (nothing here decides whether a
//! candidate should verify — that is `mk_codec::decode`'s job, called by
//! callers of this module, never by it). Built ONLY on mk-codec 0.5.0's
//! PUBLIC API: [`encode_bytecode`], [`derive_chunk_set_id`],
//! [`decode_string`], and [`encode_5bit_to_string`] (the string-layer encode
//! entry) — no local mirroring of codec internals, and no reliance on
//! `ChunkFragment`'s constructor, which does not exist externally
//! (`#[non_exhaustive]`, no `pub fn new` — r3-I1 P-d, measured `E0639`).
//!
//! `#[cfg(test)]`-gated and `pub(crate)` so every `#[cfg(test)] mod tests`
//! block across `src/seat/*.rs` can reach it (plan P0 item 1: "re-exported
//! for integration rows"; `tests/common` is not importable from unit tests
//! inside a binary-only crate — plan-r1 M2 — so the shared helper has to
//! live on the `src/` side instead).
//!
//! # Why a chunker is needed at all
//!
//! SPEC row 5 (the over-budget acceptance row) needs a header set at
//! `total_chunks = 32`; mk-codec's own encoder cannot mint one (its stub
//! cap of 255 tops out at n = 21 — r3 P-i, `crates/mk-codec` `u8::MAX`
//! stub-count field). This module builds the chunk STRINGS directly at the
//! 5-bit-symbol layer, bypassing `mk_codec::encode`'s bytecode-length path
//! entirely, so `total_chunks` and each fragment's content are fully
//! caller-controlled.

use mk_codec::string_layer::{
    StringLayerHeader, bytes_to_5bit, decode_string, encode_5bit_to_string,
};

/// Encode one synthetic chunk directly at the string layer: build the
/// 8-symbol `Chunked` header, append `bytes_to_5bit(fragment)`, and run it
/// through [`encode_5bit_to_string`] (the string-layer encode entry) —
/// mk-codec's OWN low-level primitive, the same one `mk_codec::string_layer`
/// composes into `encode`/`encode_with_chunk_set_id`.
pub(crate) fn synth_string(
    chunk_set_id: u32,
    total_chunks: u8,
    chunk_index: u8,
    fragment: &[u8],
) -> String {
    let header = StringLayerHeader::Chunked {
        version: 0,
        chunk_set_id,
        total_chunks,
        chunk_index,
    };
    let mut data = header.to_5bit_symbols();
    data.extend(bytes_to_5bit(fragment));
    encode_5bit_to_string(&data)
        .expect("valid synthetic chunk symbols (49-byte fragment, long code)")
}

/// Deterministic, pairwise-distinct-per-`card` fragment bytes.
///
/// `card` and `chunk_index` together select 49 bytes (the long-code
/// fragment ceiling, [`mk_codec::CHUNKED_FRAGMENT_LONG_BYTES`]) that differ
/// from every OTHER `card` value at the SAME `chunk_index` — the "distinct
/// stub lists" property SPEC row 5 depends on (r3-M1/NEW-M1: shared content
/// would collapse the per-index piece count below `card`-many, defeating
/// the fixture's whole point). Varying by `chunk_index` too is not required
/// for that property but keeps the bytes from being a constant repeat.
pub(crate) fn synth_fragment(card: u8, chunk_index: u8) -> Vec<u8> {
    (0u8..49)
        .map(|b| {
            card.wrapping_mul(97)
                .wrapping_add(chunk_index.wrapping_mul(13))
                .wrapping_add(b)
        })
        .collect()
}

/// Build one synthetic "card"'s full set of mk1 strings: `total_chunks`
/// pieces, each carrying [`synth_fragment`]`(card, index)`.
///
/// The `card` byte is the only thing distinguishing sibling cards built at
/// the SAME `chunk_set_id`/`total_chunks` (SPEC row 5's construction: 5
/// synthetic cards, `card = 0..5`, all sharing one id).
pub(crate) fn synth_card_strings(chunk_set_id: u32, total_chunks: u8, card: u8) -> Vec<String> {
    (0..total_chunks)
        .map(|i| synth_string(chunk_set_id, total_chunks, i, &synth_fragment(card, i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::NetworkKind;
    use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
    use bitcoin::hashes::{Hash, sha256};
    use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
    use mk_codec::bytecode::encode_bytecode;
    use mk_codec::{CHUNKED_FRAGMENT_LONG_BYTES, KeyCard};
    use std::str::FromStr;

    fn xpub(seed: u8) -> Xpub {
        let secp = Secp256k1::new();
        let mut sk_bytes = [0x11u8; 32];
        sk_bytes[31] = seed;
        let sk = SecretKey::from_slice(&sk_bytes).unwrap();
        Xpub {
            network: NetworkKind::Main,
            depth: 4,
            parent_fingerprint: Fingerprint::from([seed; 4]),
            child_number: ChildNumber::Hardened { index: 2 },
            public_key: PublicKey::from_secret_key(&secp, &sk),
            chain_code: ChainCode::from([seed; 32]),
        }
    }

    /// The chunker's primitive (`synth_string`) faithfully reproduces what
    /// `mk_codec::encode`'s own pipeline would emit for a REAL card: chunk
    /// `encode_bytecode`'s output by the documented rule (53-byte fragments
    /// of `bytecode ++ SHA-256(bytecode)[0..4]` — pure arithmetic, no codec
    /// internals mirrored, matching `chunk::split_into_chunks`'s own doc
    /// comment) and round-trip it through `mk_codec::decode`. Exercised at
    /// n=7 and n=21 (the largest a real card can mint, r3 P-i) — part of
    /// the "n = 7/11/12/21/32" range plan-r1 measured; n=32 (no real card
    /// reaches it) is exercised separately below, which is the module's
    /// reason to exist (SPEC row 5).
    fn real_card_round_trips_through_the_chunker(n_stubs: u32) {
        let path = DerivationPath::from_str("48'/0'/0'/2'").unwrap();
        let card = KeyCard::new(
            (0..n_stubs).map(u32::to_be_bytes).collect(),
            Some(Fingerprint::from([0xAA, 0xBB, 0xCC, 0xDD])),
            path,
            xpub(0x42),
        );
        let bytecode = encode_bytecode(&card).expect("card must encode");
        let chunk_set_id = mk_codec::derive_chunk_set_id(&bytecode);
        let hash = sha256::Hash::hash(&bytecode);
        let mut stream = bytecode.clone();
        stream.extend_from_slice(&hash.to_byte_array()[..4]);

        let frag_len = CHUNKED_FRAGMENT_LONG_BYTES;
        let total_chunks = stream.len().div_ceil(frag_len) as u8;
        let strings: Vec<String> = stream
            .chunks(frag_len)
            .enumerate()
            .map(|(i, frag)| synth_string(chunk_set_id, total_chunks, i as u8, frag))
            .collect();

        // Direct use of `decode_string` (one of the four allowed calls):
        // chunk 0's header round-trips at the BCH layer independent of the
        // full-card `mk_codec::decode` path exercised right below.
        let decoded0 = decode_string(&strings[0]).expect("chunk 0 must decode at the BCH layer");
        let (header0, _) = StringLayerHeader::from_5bit_symbols(decoded0.data()).unwrap();
        assert!(matches!(
            header0,
            StringLayerHeader::Chunked { chunk_index: 0, .. }
        ));

        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        let recovered =
            mk_codec::decode(&refs).expect("chunker output must decode as the real card");
        assert_eq!(recovered, card);
    }

    #[test]
    fn chunker_round_trips_a_small_real_card_n7() {
        // 7 stubs keeps the bytecode short enough that n stays small (well
        // under 11) while still exercising >1 chunk.
        real_card_round_trips_through_the_chunker(7);
    }

    #[test]
    fn chunker_round_trips_the_largest_real_card_n21() {
        // 255 stubs is mk-codec's own encoder-side cap (`u8` stub count);
        // r3 P-i measures this as n=21, the largest chunk count any real
        // `mk encode` invocation can produce.
        real_card_round_trips_through_the_chunker(255);
    }

    #[test]
    fn synth_string_is_deterministic() {
        let a = synth_string(0x12345, 32, 7, &synth_fragment(2, 7));
        let b = synth_string(0x12345, 32, 7, &synth_fragment(2, 7));
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_cards_yield_distinct_pieces_at_every_index() {
        // The property row 5 depends on: 5 synthetic cards, n=32, must give
        // 5 DISTINCT pieces at EVERY index (never a collapse).
        use std::collections::HashSet;
        for index in 0..32u8 {
            let pieces: HashSet<String> = (0..5u8)
                .map(|card| synth_string(0x22222, 32, index, &synth_fragment(card, index)))
                .collect();
            assert_eq!(
                pieces.len(),
                5,
                "index {index}: cards must not collapse to fewer than 5 distinct pieces"
            );
        }
    }

    #[test]
    fn synth_card_strings_declares_every_index_once() {
        let strings = synth_card_strings(0x33333, 32, 0);
        assert_eq!(strings.len(), 32);
        for (i, s) in strings.iter().enumerate() {
            let decoded = decode_string(s).unwrap();
            let (header, _) = StringLayerHeader::from_5bit_symbols(decoded.data()).unwrap();
            match header {
                StringLayerHeader::Chunked {
                    chunk_set_id,
                    total_chunks,
                    chunk_index,
                    ..
                } => {
                    assert_eq!(chunk_set_id, 0x33333);
                    assert_eq!(total_chunks, 32);
                    assert_eq!(chunk_index as usize, i);
                }
                StringLayerHeader::SingleString { .. } => panic!("must be Chunked"),
                _ => panic!("unrecognised header variant"),
            }
        }
    }
}
