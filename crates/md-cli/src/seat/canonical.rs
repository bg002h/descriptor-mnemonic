//! SPEC `design/SPEC_seat_auto_partition.md` §1 — the canonical-piece key.
//!
//! > two strings are the SAME piece iff their `(chunk_set_id, total_chunks,
//! > chunk_index, 5-bit payload symbol tail)` are equal — the symbol tail,
//! > not re-derived bytes, so no new failure route; a string failing
//! > `decode_string` is refused exactly as today.
//!
//! Shipped as PRODUCTION code from P0 (plan `IMPLEMENTATION_PLAN_seat_auto_partition.md`
//! P0 item 2); wired into `seat::run` at P1 step 1, where
//! [`super::input::canonicalize_group`] calls this same function right
//! after [`super::input::dedupe_strings`] — P0's shape tests, and P1's
//! engine (`super::partition`), all call THIS function directly, never a
//! second implementation of the same rule (plan-r1 M3).
//!
//! Built only on mk-codec's public API ([`decode_string`],
//! [`StringLayerHeader::from_5bit_symbols`]) — no local mirroring of BCH or
//! header-parsing internals (mirrors the discipline `input.rs::group_key_of`
//! already follows for the coarser `GroupId` key).

use crate::error::CliError;
use mk_codec::string_layer::{StringLayerHeader, decode_string};

/// SPEC §1's four-tuple identity for one mk1 CHUNKED string's piece.
///
/// The fourth field is the 5-bit SYMBOL tail exactly as `decode_string`
/// returns it (post-BCH-correction, pre-`five_bit_to_bytes`) — SPEC §1 is
/// explicit that identity is decided on the symbol tail, not re-derived
/// bytes, so a payload that fails `five_bit_to_bytes`'s padding check (a
/// route this function never calls) is still comparable here.
///
/// `Hash`/`Eq` are structural (derived), so two pieces compare equal iff
/// all four fields match — exactly SPEC §1's rule, with no separate
/// comparison function to drift from it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CanonicalPieceKey {
    /// The declared 20-bit chunk-set id.
    pub(crate) chunk_set_id: u32,
    /// The declared total chunk count for this piece's set.
    pub(crate) total_chunks: u8,
    /// This piece's zero-based index within its declared set.
    pub(crate) chunk_index: u8,
    /// The 5-bit payload symbols following the chunked header, verbatim.
    pub(crate) symbol_tail: Vec<u8>,
}

/// Compute one mk1 string's SPEC §1 canonical-piece key.
///
/// A string that fails `decode_string` (the BCH layer) or has a malformed
/// string-layer header is refused exactly as it is refused elsewhere in
/// this pipeline today — SPEC §1's "no new failure route" guarantee.
///
/// A `SingleString`-headered input carries no `chunk_set_id`/`total_chunks`/
/// `chunk_index` to canonicalise against, so it is out of §1's scope (mk1
/// cards always chunk — SPEC "Out of scope"; the 73-byte compact xpub alone
/// exceeds the single-string ceiling, so no `mk encode` invocation can ever
/// produce one — `input.rs`'s `GroupId::Single` doc comment makes the same
/// point). It is refused by name rather than silently coerced into a key
/// that would claim an identity the wire format does not carry.
pub(crate) fn canonical_piece_key(s: &str) -> Result<CanonicalPieceKey, CliError> {
    let decoded =
        decode_string(s).map_err(|e| CliError::Seat(format!("not a decodable mk1 string: {e}")))?;
    let data = decoded.data();
    let (header, consumed) = StringLayerHeader::from_5bit_symbols(data)
        .map_err(|e| CliError::Seat(format!("malformed mk1 string-layer header: {e}")))?;
    match header {
        StringLayerHeader::Chunked {
            chunk_set_id,
            total_chunks,
            chunk_index,
            ..
        } => Ok(CanonicalPieceKey {
            chunk_set_id,
            total_chunks,
            chunk_index,
            symbol_tail: data[consumed..].to_vec(),
        }),
        StringLayerHeader::SingleString { .. } => Err(CliError::Seat(
            "mk1 single-string headers carry no chunk-set id to canonicalise against (SPEC §1 \
             applies only to chunked mk1 cards)"
                .to_string(),
        )),
        // `StringLayerHeader` is `#[non_exhaustive]`: mk-codec may add a
        // third variant in a later minor version. Refuse by name rather
        // than silently fabricating a key for a header shape §1 was never
        // written against.
        _ => Err(CliError::Seat(
            "mk1 string-layer header variant not recognised by this build (SPEC §1 covers only \
             `SingleString`/`Chunked`)"
                .to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reuse the shipped R5-classifier fixtures (input.rs) as ready-made
    // chunked strings rather than minting fresh ones for these micro-tests —
    // they are already pinned, decodable literals.
    const CHUNK0_OF_11111: &str = "mk1qpzyg3pqqsq4kj90x9eutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5x94fvrdwgu6g0lq";
    const OTHER_CHUNK0_OF_11111: &str = "mk1qpzyg3pqqsq4kj90xxux3r03q5zg3vs7llvu2xd8x2rk7av9gmew82jq5zap9302ynhp37ggd6z5u4emag0zr8gh9upnj76samqgd9kc0zqu";
    const CHUNK1_OF_11111: &str =
        "mk1qpzyg3pp806lhaeh6reknylagmwyjycf8044xtt9flsdlkvt6f6cthyl99lqmcdjwej7x7ylmk0lq";

    #[test]
    fn same_string_yields_equal_keys() {
        let a = canonical_piece_key(CHUNK0_OF_11111).unwrap();
        let b = canonical_piece_key(CHUNK0_OF_11111).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.chunk_set_id, 0x11111);
        assert_eq!(a.chunk_index, 0);
        assert_eq!(a.total_chunks, 2);
    }

    #[test]
    fn same_index_different_payload_yields_different_keys() {
        // Both declare chunk-set 11111, chunk_index 0 of 2 (the r5-merged
        // fixture: two DIFFERENT cards pinned to one id) — same header
        // fields, different symbol tails.
        let a = canonical_piece_key(CHUNK0_OF_11111).unwrap();
        let b = canonical_piece_key(OTHER_CHUNK0_OF_11111).unwrap();
        assert_eq!(a.chunk_set_id, b.chunk_set_id);
        assert_eq!(a.chunk_index, b.chunk_index);
        assert_eq!(a.total_chunks, b.total_chunks);
        assert_ne!(
            a.symbol_tail, b.symbol_tail,
            "different payloads must not canonicalise to the same piece"
        );
        assert_ne!(a, b);
    }

    #[test]
    fn different_chunk_index_yields_different_keys_even_with_same_set_id() {
        let a = canonical_piece_key(CHUNK0_OF_11111).unwrap();
        let b = canonical_piece_key(CHUNK1_OF_11111).unwrap();
        assert_eq!(a.chunk_set_id, b.chunk_set_id);
        assert_ne!(a.chunk_index, b.chunk_index);
        assert_ne!(a, b);
    }

    #[test]
    fn undecodable_string_is_refused_not_panicked() {
        let err = canonical_piece_key("mk1notavalidstring").unwrap_err();
        assert!(err.to_string().contains("not a decodable mk1 string"));
    }

    #[test]
    fn key_is_hashable_for_distinct_piece_counting() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(canonical_piece_key(CHUNK0_OF_11111).unwrap());
        set.insert(canonical_piece_key(CHUNK0_OF_11111).unwrap());
        set.insert(canonical_piece_key(OTHER_CHUNK0_OF_11111).unwrap());
        assert_eq!(
            set.len(),
            2,
            "identical pieces collapse; distinct ones don't"
        );
    }
}
