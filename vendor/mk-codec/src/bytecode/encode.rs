//! Top-level bytecode encoder: `KeyCard` → canonical `Vec<u8>`.
//!
//! Per `design/SPEC_mk_v0_1.md` §3.2 payload field order (closure Q-6):
//!
//! ```text
//! [bytecode_header   : 1 B]
//! [stub_count        : 1 B; MUST be ≥ 1]
//! [policy_id_stubs   : 4 × N B]
//! [origin_fingerprint: 4 B]   ← present iff bytecode_header bit 2 set
//! [origin_path       : variable]
//! [xpub_compact      : 73 B]
//! ```

use bitcoin::bip32::ChildNumber;

use crate::bytecode::header::BytecodeHeader;
use crate::bytecode::path::encode_path;
use crate::bytecode::xpub_compact::{XpubCompact, encode_xpub_compact};
use crate::consts::MAX_PATH_COMPONENTS;
use crate::error::{Error, Result};
use crate::key_card::KeyCard;

/// Encode a `KeyCard` to its canonical bytecode form (pre-chunking).
pub fn encode_bytecode(card: &KeyCard) -> Result<Vec<u8>> {
    if card.policy_id_stubs.is_empty() {
        return Err(Error::InvalidPolicyIdStubCount);
    }
    if card.policy_id_stubs.len() > u8::MAX as usize {
        return Err(Error::InvalidPolicyIdStubCount);
    }

    // Encoder-side path cap. `decode_explicit_path` has always refused
    // `count > MAX_PATH_COMPONENTS` with `PathTooDeep`, but `encode_path` wrote
    // the count unchecked -- so the encoder could mint a well-formed card that
    // this crate's OWN decoder refuses. That is a write-only card: engraved in
    // metal and unrecoverable, produced at exit 0. Refusing here makes the
    // write side agree with the read side (R2/C2, 2026-08-21).
    //
    // Checked before the depth/child guard below so the error names the real
    // problem: an over-deep path also has an over-deep xpub, and reporting
    // XpubOriginPathMismatch would send the operator after the wrong thing.
    let component_count = card.origin_path.into_iter().count();
    if component_count > MAX_PATH_COMPONENTS as usize {
        // SATURATE, do not truncate. `Error::PathTooDeep` carries a `u8`, and
        // this check deliberately runs BEFORE the depth/child guard below --
        // so the guard cannot be relied on to bound the count, and `as u8`
        // wrapped: 256 components reported "0 components", 300 reported "44".
        //
        // An earlier comment here claimed the truncation was unreachable
        // BECAUSE of that guard. Moving this check above the guard is what
        // made that false, and the claim shipped anyway (R7/B1, 2026-08-21).
        // Diagnostics only -- the refusal was always correct -- but a refusal
        // that misreports its own reason is the class R2 already filed against
        // `XpubOriginPathMismatch`, reproduced in the fix for it.
        //
        // Saturation makes the number a floor rather than a lie: a path at or
        // beyond 255 components reports 255, and every such path is refused.
        let reported = u8::try_from(component_count).unwrap_or(u8::MAX);
        return Err(Error::PathTooDeep(reported));
    }

    // Encoder-side invariant (SPEC_mk_v0_1.md §4): compact-73 reconstructs depth/
    // child_number from origin_path on decode; reject any xpub whose depth/
    // child_number disagree, else the emitted card decodes to a different-
    // metadata xpub (the decoder cannot detect — no on-wire depth).
    // expected_child mirrors reconstruct_xpub exactly: the terminal component,
    // or Normal{0} for an empty path (depth-0 / no-path key, e.g. a WIF). A card
    // encodes iff it survives compact-drop + reconstruction unchanged.
    let path_depth = component_count;
    let path_child = card.origin_path.into_iter().last().copied();
    let expected_child = path_child.unwrap_or(ChildNumber::Normal { index: 0 });
    if card.xpub.depth as usize != path_depth || card.xpub.child_number != expected_child {
        return Err(Error::XpubOriginPathMismatch {
            xpub_depth: card.xpub.depth,
            path_depth: path_depth as u8,
            xpub_child: card.xpub.child_number,
            path_child,
        });
    }

    let header = BytecodeHeader {
        version: 0,
        fingerprint_flag: card.origin_fingerprint.is_some(),
    };

    let mut out: Vec<u8> = Vec::new();
    out.push(header.to_byte());
    out.push(card.policy_id_stubs.len() as u8);
    for stub in &card.policy_id_stubs {
        out.extend_from_slice(stub);
    }
    if let Some(fp) = &card.origin_fingerprint {
        out.extend_from_slice(fp.as_bytes());
    }
    out.extend_from_slice(&encode_path(&card.origin_path));
    let compact = XpubCompact::from_xpub(&card.xpub);
    encode_xpub_compact(&compact, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::test_helpers::synthetic_xpub;
    use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint};
    use std::str::FromStr;

    fn fixture_card_1stub_with_fp() -> KeyCard {
        let path = DerivationPath::from_str("m/48'/0'/0'/2'").unwrap();
        KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: Some(Fingerprint::from([0xD3, 0x4D, 0xB3, 0x3F])),
            xpub: synthetic_xpub(&path),
            origin_path: path,
        }
    }

    #[test]
    fn encodes_typical_1stub_card_to_84_bytes() {
        let card = fixture_card_1stub_with_fp();
        let wire = encode_bytecode(&card).unwrap();
        // header(1) + stub_count(1) + 1*stub(4) + fp(4) + std-table indicator(1) + xpub_compact(73) = 84
        assert_eq!(wire.len(), 84);
        assert_eq!(wire[0], 0x04, "fingerprint flag set");
        assert_eq!(wire[1], 1, "stub_count = 1");
        assert_eq!(&wire[2..6], &[0xAA; 4], "stub bytes");
        assert_eq!(&wire[6..10], &[0xD3, 0x4D, 0xB3, 0x3F], "fp bytes");
        assert_eq!(wire[10], 0x05, "std-table indicator for m/48'/0'/0'/2'");
    }

    #[test]
    fn encodes_card_without_fingerprint_to_80_bytes() {
        let mut card = fixture_card_1stub_with_fp();
        card.origin_fingerprint = None;
        let wire = encode_bytecode(&card).unwrap();
        // 84 - 4 (omitted fp) = 80
        assert_eq!(wire.len(), 80);
        assert_eq!(wire[0], 0x00, "fingerprint flag unset");
    }

    #[test]
    fn rejects_zero_stubs() {
        let mut card = fixture_card_1stub_with_fp();
        card.policy_id_stubs.clear();
        assert!(matches!(
            encode_bytecode(&card),
            Err(Error::InvalidPolicyIdStubCount),
        ));
    }

    #[test]
    fn deterministic_output() {
        let card = fixture_card_1stub_with_fp();
        let a = encode_bytecode(&card).unwrap();
        let b = encode_bytecode(&card).unwrap();
        assert_eq!(a, b, "encoder must be byte-deterministic");
    }

    // ── XpubOriginPathMismatch encoder-side guard (SPEC §5) ──────────────────

    // Cell 1: xpub.depth ≠ component_count(origin_path) → reject.
    #[test]
    fn rejects_xpub_depth_mismatch() {
        let mut card = fixture_card_1stub_with_fp(); // path m/48'/0'/0'/2' → depth 4
        card.xpub.depth = 3;
        assert!(matches!(
            encode_bytecode(&card),
            Err(Error::XpubOriginPathMismatch {
                xpub_depth: 3,
                path_depth: 4,
                ..
            }),
        ));
    }

    // Cell 2 + 6: same depth, wrong terminal child (the previously-silent case;
    // the fixture is a standard-table path, so this also covers the dictionary
    // child-mismatch). A depth-only check (as the toolkit's does) would MISS this.
    #[test]
    fn rejects_xpub_child_mismatch_same_depth() {
        let mut card = fixture_card_1stub_with_fp(); // terminal child = 2'
        card.xpub.child_number = ChildNumber::Hardened { index: 1 }; // → 1', depth still 4
        assert!(matches!(
            encode_bytecode(&card),
            Err(Error::XpubOriginPathMismatch { .. }),
        ));
    }

    // Cell 5 (v0.4.0): a consistent depth-0 / no-path card (empty path, depth 0,
    // child Normal{0} — the WIF shape) now ENCODES. Was rejected pre-0.4.0.
    #[test]
    fn accepts_consistent_depth0_card() {
        let path = DerivationPath::from_str("m").unwrap(); // empty path
        let card = KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: None,
            xpub: synthetic_xpub(&path), // depth 0, child Normal{0}
            origin_path: path,
        };
        assert!(
            encode_bytecode(&card).is_ok(),
            "consistent depth-0 no-path card must encode"
        );
    }

    // Cell 6: a depth-0 card with a non-canonical terminal child (Normal{5})
    // would NOT round-trip (reconstruct yields Normal{0}) → still rejected.
    #[test]
    fn rejects_depth0_noncanonical_child() {
        let path = DerivationPath::from_str("m").unwrap(); // empty path
        let mut card = KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: None,
            xpub: synthetic_xpub(&path), // depth 0, child Normal{0}
            origin_path: path,
        };
        card.xpub.child_number = ChildNumber::Normal { index: 5 };
        assert!(matches!(
            encode_bytecode(&card),
            Err(Error::XpubOriginPathMismatch {
                xpub_depth: 0,
                path_depth: 0,
                path_child: None,
                ..
            }),
        ));
    }

    // Cell 8: the WIF/no-path card survives the full bytecode round-trip
    // (encode_bytecode → decode_bytecode), proving end-to-end support.
    #[test]
    fn depth0_card_round_trips() {
        use crate::bytecode::decode::decode_bytecode;
        let path = DerivationPath::from_str("m").unwrap();
        let card = KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: None,
            xpub: synthetic_xpub(&path),
            origin_path: path.clone(),
        };
        let wire = encode_bytecode(&card).unwrap();
        let decoded = decode_bytecode(&wire).unwrap();
        assert_eq!(decoded.origin_path, path);
        assert_eq!(decoded.xpub.depth, 0);
        assert_eq!(decoded.xpub.child_number, ChildNumber::Normal { index: 0 });
        assert_eq!(decoded.xpub.public_key, card.xpub.public_key);
        assert_eq!(decoded.xpub.chain_code, card.xpub.chain_code);
    }

    // Cell 4: an aligned EXPLICIT-path card (not in the standard table) encodes
    // OK — guards against false-positives on explicit-mode paths. (The existing
    // `encodes_typical_1stub_card_to_84_bytes` covers the standard-table-aligned
    // case = SPEC cell 5; `xpub_compact.rs::round_trip_full_xpub_depth_4` covers
    // the reconstruct round-trip = SPEC cell 4 losslessness.)
    /// An origin path deeper than `MAX_PATH_COMPONENTS` must be refused AT
    /// ENCODE, not merely at decode.
    ///
    /// `decode_explicit_path` has always refused `count > MAX_PATH_COMPONENTS`
    /// with `PathTooDeep`, but `encode_path` wrote `components.len() as u8`
    /// unchecked. The asymmetry let the encoder mint a well-formed card that
    /// its own decoder refuses -- a WRITE-ONLY card. Engraved in metal it is
    /// unrecoverable, and the operator gets exit 0 and a full-looking bundle.
    /// Found by an adversarial review, 2026-08-21 (R2/C2).
    #[test]
    fn rejects_path_deeper_than_max_components() {
        // 256 and 300 are the cases the previous `as u8` wrapped on; the old
        // test stopped at 255, one short of the boundary that broke.
        for n in [MAX_PATH_COMPONENTS as usize + 1, 20, 255, 256, 300] {
            let comps: Vec<String> = (0..n).map(|i| format!("{i}'")).collect();
            let path = DerivationPath::from_str(&format!("m/{}", comps.join("/"))).unwrap();
            let card = KeyCard {
                policy_id_stubs: vec![[0xAA; 4]],
                origin_fingerprint: None,
                xpub: synthetic_xpub(&path), // depth-aligned, so the SPEC §5 guard passes
                origin_path: path,
            };
            match encode_bytecode(&card) {
                // Saturating: at or beyond 255 the reported count is a floor.
                Err(Error::PathTooDeep(got)) => {
                    let want = u8::try_from(n).unwrap_or(u8::MAX);
                    assert_eq!(
                        got, want,
                        "{n} components must report {want}, not a wrapped value"
                    );
                }
                other => panic!("{n} components must be refused at encode, got {other:?}"),
            }
        }
    }

    /// The boundary itself still encodes -- the cap refuses `> MAX`, not `>=`.
    #[test]
    fn accepts_path_at_exactly_max_components() {
        let comps: Vec<String> = (0..MAX_PATH_COMPONENTS as usize)
            .map(|i| format!("{i}'"))
            .collect();
        let path = DerivationPath::from_str(&format!("m/{}", comps.join("/"))).unwrap();
        let card = KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: None,
            xpub: synthetic_xpub(&path),
            origin_path: path,
        };
        assert!(
            encode_bytecode(&card).is_ok(),
            "exactly MAX_PATH_COMPONENTS must still encode"
        );
    }

    #[test]
    fn aligned_explicit_path_card_encodes() {
        let path = DerivationPath::from_str("m/44'/0'/0'/0/5").unwrap(); // 5 comps, explicit
        let card = KeyCard {
            policy_id_stubs: vec![[0xAA; 4]],
            origin_fingerprint: None,
            xpub: synthetic_xpub(&path), // depth 5, child Normal{5} — aligned
            origin_path: path,
        };
        assert!(
            encode_bytecode(&card).is_ok(),
            "aligned explicit-path card must encode"
        );
    }
}
