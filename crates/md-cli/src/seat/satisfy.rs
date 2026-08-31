//! **A2 — seat by declared origin: the DECLARATION is the CONSTRAINT**, plus
//! the two door checks that run before any card is looked at and the two
//! card-set checks that run before any matching is enumerated.
//!
//! A2's relation is deliberately NOT an equivalence (SPEC A2, r5 I1): a card
//! can satisfy two unequal declarations — `[fp/path]` and a fingerprint-free
//! `path` — which is why "origin-equivalence classes" are gone from the spec
//! and from this module. What lives here is a bipartite SATISFACTION
//! relation and nothing that pretends to partition either side.
//!
//! Comparison is over DECODED `(fingerprint, path)` values, never strings.
//! `h`/`'` hardening, an `m/` prefix and fingerprint case are all measured to
//! vary across the constellation's own outputs, so a string comparison here
//! would refuse legitimate sets for spelling.
//!
//! ## The four cells of A2
//!
//! | declaration | card | satisfied? |
//! | --- | --- | --- |
//! | `Some(f)` | `Some(g)` | paths equal AND `f == g` |
//! | `Some(f)` | `None`    | never — a declared fingerprint is a requirement the card cannot meet blind |
//! | `None`    | `Some(g)` | paths equal — the card's extra fingerprint is information, not a mismatch |
//! | `None`    | `None`    | paths equal |
//!
//! The third cell is the **named residue** (SPEC A2, r5 M2): a
//! fingerprint-free declaration accepts a FOREIGN card at the right path.
//! That is the policy AUTHOR's accepted risk, taken at mint time where
//! `md encode` already warns that fingerprint-free slots cannot be told
//! apart; the converter inherits the choice rather than overriding it, and
//! V-CE1 pins the consequence in both halves.

use crate::error::CliError;
use crate::seat::input::DecodedCard;
use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint};
use md_codec::canonicalize::expand_per_at_n;
use md_codec::encode::Descriptor;
use md_codec::origin_path::OriginPath;

/// One slot's declaration, as the policy card states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotDecl {
    /// Placeholder index `@i`.
    pub i: u8,
    /// `Some` iff the policy carries a `Fingerprints` TLV entry for `@i`.
    pub fingerprint: Option<[u8; 4]>,
    /// The declared origin path, decoded.
    pub path: DerivationPath,
}

impl SlotDecl {
    /// `@i [fp/path]` / `@i [path]` — how every refusal names a slot.
    pub fn label(&self) -> String {
        match self.fingerprint {
            Some(fp) => format!(
                "@{} [{}/{}]",
                self.i,
                Fingerprint::from(fp),
                self.path.to_string().trim_start_matches("m/")
            ),
            None => format!(
                "@{} [{}] (no fingerprint declared)",
                self.i,
                self.path.to_string().trim_start_matches("m/")
            ),
        }
    }
}

/// Convert md-codec's wire `OriginPath` to a `DerivationPath`, matching
/// `md_codec::to_miniscript`'s private converter exactly (checked against
/// `crates/md-codec/src/to_miniscript.rs::origin_path_to_derivation`) so a
/// path this module compares can never disagree with the path the renderer
/// emits for the same slot.
fn origin_path_to_derivation(p: &OriginPath) -> DerivationPath {
    let children: Vec<ChildNumber> = p
        .components
        .iter()
        .map(|c| {
            if c.hardened {
                ChildNumber::from_hardened_idx(c.value)
                    .unwrap_or(ChildNumber::Hardened { index: c.value })
            } else {
                ChildNumber::from_normal_idx(c.value)
                    .unwrap_or(ChildNumber::Normal { index: c.value })
            }
        })
        .collect();
    DerivationPath::from(children)
}

/// Read every slot's declaration off the policy descriptor.
///
/// Delegates the `path_decl` Shared/Divergent fold and the sparse
/// `Fingerprints` lookup to `md_codec::canonicalize::expand_per_at_n` — the
/// same function `compute_wallet_policy_id` uses — rather than re-deriving
/// them here, so a slot's declaration cannot drift from the one the identity
/// computation sees.
pub fn slot_declarations(policy: &Descriptor) -> Result<Vec<SlotDecl>, CliError> {
    let expanded = expand_per_at_n(policy)?;
    Ok(expanded
        .into_iter()
        .map(|e| SlotDecl {
            i: e.idx,
            fingerprint: e.fingerprint,
            path: origin_path_to_derivation(&e.origin_path),
        })
        .collect())
}

/// A2, exactly as the table in this module's doc comment states it.
pub fn satisfies(decl: &SlotDecl, card: &DecodedCard) -> bool {
    if card.card.origin_path != decl.path {
        return false;
    }
    match (decl.fingerprint, card.card.origin_fingerprint) {
        (Some(want), Some(have)) => Fingerprint::from(want) == have,
        (Some(_), None) => false,
        (None, _) => true,
    }
}

/// **Door check 1 — a placeholder used at more than one position.**
///
/// AN INVOCATION OF THE SHARED N1 CLASSIFIER, IN PLACE (plan P3 step 3b).
/// `md descriptor`'s card input is on the REFUSE mint/compose surface, so a
/// separate card-input refusal would have made this function a SECOND
/// implementation of a predicate `crate::parse::reuse` already owns — which
/// the spec's single-source rule forbids, and which is how two sides drift
/// into disagreeing about one wallet. The wording below is therefore the
/// taxonomy's, and the message an operator sees is the same whether the
/// shape arrives as a template, as a keyed card, or as a policy card here.
///
/// **Its POSITION is order-normative and does not move** (see
/// [`crate::seat::run`]: "each of those refusals is accurate only where it
/// sits"). It stays the first door check, ahead of A3, and needs nothing
/// from the card set to be right.
///
/// **Its reachable domain is exactly R-N1a**, which is why replacing the
/// wording loses nothing:
///
/// * The policy here is KEYLESS by construction — [`crate::seat::run`]
///   refuses a wallet-policy card before this point — so Family 2 has no key
///   material to fire on, in principle rather than in practice.
/// * The wire carries ONE origin, ONE multipath set and ONE hardening per
///   key slot, so two occurrences of `@i` cannot differ and the four
///   axis-divergence rows are unreachable from a card.
///
/// The diagnostic says "forbidden by BIP 388" and "unsupported", NEVER
/// "invalid": a repeated-key descriptor is technically valid script, and
/// this is a POLICY refusal of a BIP-forbidden shape (operator ruling
/// 2026-08-30, verbatim: "Bad ideas can be valid, but we don't want to
/// support BIP forbidden wallets").
///
/// This is r5-M1's construction, REGROUNDED twice: it shipped as a measured
/// over-strictness note, became a REFUSE row under the key-reuse ruling, and
/// is now the seating entrance to one taxonomy.
pub fn check_no_repeated_placeholder(policy: &Descriptor) -> Result<(), CliError> {
    crate::parse::reuse::check_descriptor(policy, crate::parse::reuse::Disposition::Refuse)
}

/// **Door check 2 — two fingerprint-BEARING slots with the identical
/// `(fingerprint, path)`.**
///
/// SPEC A3 consequence (c): its only possible fill binds one xpub to two
/// slots, because one master at one path yields exactly one key — so the
/// policy is refused AT THE DOOR with that explanation, before any card is
/// read.
///
/// The legitimate same-path family is untouched by design: fingerprint-FREE
/// declarations across DIFFERENT masters (privacy-preserving multisig) are
/// pairwise-distinct keys, not reuse, and V-BOUND-SEAT pins that they seat.
pub fn check_no_identical_fp_bearing_declarations(decls: &[SlotDecl]) -> Result<(), CliError> {
    for (a, first) in decls.iter().enumerate() {
        let Some(fp) = first.fingerprint else {
            continue;
        };
        for second in decls.iter().skip(a + 1) {
            if second.fingerprint == Some(fp) && second.path == first.path {
                return Err(CliError::Seat(format!(
                    "slots {} and {} declare the IDENTICAL origin. One master at one path \
                     yields exactly one key, so the only possible fill binds that one xpub \
                     to both slots — forbidden by BIP 388's pairwise-distinct rule, and \
                     UNSUPPORTED here. Refused at the door, before any card is read. \
                     (Fingerprint-FREE declarations at one path are a DIFFERENT case and \
                     seat normally: different masters, pairwise-distinct keys.)",
                    first.label(),
                    second.label()
                )));
            }
        }
    }
    Ok(())
}

/// **Card-set check 1 — two cards claiming the identical
/// `(fingerprint, path)` with DIFFERENT xpubs.**
///
/// Impossible from one master (SPEC A3, r1): BIP 32 derivation is a
/// function, so one seed at one path has one answer. At least one of the two
/// cards is mis-declared, and seating either would reconstruct a different
/// wallet.
///
/// Runs BEFORE matching, so the refusal is the accurate one. Deferred to
/// A4 it would surface as a leftover-card message on the policy that happens
/// to declare only one such slot, which names a symptom rather than the
/// defect.
pub fn check_no_impossible_card_pair(cards: &[DecodedCard]) -> Result<(), CliError> {
    for (a, first) in cards.iter().enumerate() {
        let Some(fp) = first.card.origin_fingerprint else {
            continue;
        };
        for second in cards.iter().skip(a + 1) {
            if second.card.origin_fingerprint == Some(fp)
                && second.card.origin_path == first.card.origin_path
                && second.card.xpub != first.card.xpub
            {
                return Err(CliError::Seat(format!(
                    "cards {} and {} both declare origin [{}/{}] yet carry DIFFERENT xpubs. \
                     One master at one derivation path yields exactly one key, so this pair \
                     cannot both be genuine — at least one card's declared origin is wrong. \
                     Seating either would reconstruct a different wallet.",
                    first.label(),
                    second.label(),
                    fp,
                    first.card.origin_path.to_string().trim_start_matches("m/")
                )));
            }
        }
    }
    Ok(())
}

/// **Card-set check 2 — the same xpub offered twice.**
///
/// Every perfect matching seats EVERY card (A4: completeness is total), so
/// two cards carrying one xpub necessarily put that xpub in two slots — the
/// reachable half of BIP 388's pairwise-distinct rule (SPEC A3: "shape (1)
/// … is the reachable case where the engine's refusal binds").
///
/// The wording is "forbidden by BIP 388" / "unsupported" and never
/// "invalid", per the operator ruling recorded in SPEC A3.
pub fn check_no_repeated_xpub(cards: &[DecodedCard]) -> Result<(), CliError> {
    for (a, first) in cards.iter().enumerate() {
        for second in cards.iter().skip(a + 1) {
            if first.card.xpub.public_key == second.card.xpub.public_key
                && first.card.xpub.chain_code == second.card.xpub.chain_code
            {
                return Err(CliError::Seat(format!(
                    "cards {} and {} carry the SAME extended public key. Every slot must be \
                     filled by a card and every card must be seated, so this pair would put \
                     one key in two slots — forbidden by BIP 388 (\"{rule}\"; {note}). \
                     UNSUPPORTED here, not a malformed input: supply one card per distinct \
                     key.",
                    first.label(),
                    second.label(),
                    rule = crate::bip388::PAIRWISE_DISTINCT_RULE,
                    note = crate::bip388::REUSE_SECURITY_NOTE,
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod fixture {
    //! Shared fixture reader for the seating rows.
    //!
    //! Every fixture file carries `#` provenance, the keyless policy card's
    //! md1 strings, and the mk1 key-card strings, in that order; the split
    //! is by HRP so a file can be read without a schema.
    use super::*;
    use crate::seat::input::{DecodedCard, decode_cards};

    pub fn md1_lines(text: &str) -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| l.starts_with("md1"))
            .map(str::to_string)
            .collect()
    }

    pub fn mk1_lines(text: &str) -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect()
    }

    /// Decode a fixture's md1 half into the keyless policy descriptor.
    pub fn policy(text: &str) -> Descriptor {
        let phrases = md1_lines(text);
        let refs: Vec<&str> = phrases.iter().map(String::as_str).collect();
        if refs.len() == 1 {
            md_codec::decode_md1_string(refs[0]).expect("fixture md1 decodes")
        } else {
            md_codec::reassemble(&refs).expect("fixture md1 reassembles")
        }
    }

    /// Decode a fixture's mk1 half into cards.
    pub fn cards(text: &str) -> Vec<DecodedCard> {
        decode_cards(&mk1_lines(text)).expect("fixture mk1 decodes")
    }

    pub const PATHOLOGICAL: &str =
        include_str!("../../tests/fixtures/pathological/backup-strings.txt");
    pub const V_IMPOSS: &str = include_str!("../../tests/fixtures/seating/v-imposs.txt");
    pub const V_DOOR: &str = include_str!("../../tests/fixtures/seating/v-door.txt");
    pub const V_FPFREE_CARD: &str = include_str!("../../tests/fixtures/seating/v-fpfree-card.txt");
    pub const V_R5M1: &str = include_str!("../../tests/fixtures/seating/v-r5m1.txt");
    pub const V_BOUND_REF: &str = include_str!("../../tests/fixtures/seating/v-bound-ref.txt");
    pub const V_BOUND_REF_PATHS: &str =
        include_str!("../../tests/fixtures/seating/v-bound-ref-paths.txt");
    pub const V_BOUND_SEAT: &str = include_str!("../../tests/fixtures/seating/v-bound-seat.txt");
    pub const V_USP: &str = include_str!("../../tests/fixtures/seating/v-usp.txt");
    pub const V_MIX: &str = include_str!("../../tests/fixtures/seating/v-mix.txt");
    pub const V_R2_ORD: &str = include_str!("../../tests/fixtures/seating/v-r2-ord.txt");
    pub const V_R4_IK: &str = include_str!("../../tests/fixtures/seating/v-r4-ik.txt");
    pub const V_GRP: &str = include_str!("../../tests/fixtures/seating/v-grp.txt");
    pub const V_CAP: &str = include_str!("../../tests/fixtures/seating/v-cap.txt");
    pub const V_LEFTOVER: &str = include_str!("../../tests/fixtures/seating/v-leftover.txt");
    pub const V_UNFILLED: &str = include_str!("../../tests/fixtures/seating/v-unfilled.txt");
    pub const V_B1_WALLET: &str = include_str!("../../tests/fixtures/seating/v-b1-wallet.txt");
    pub const V_B1_SHAPE: &str = include_str!("../../tests/fixtures/seating/v-b1-shape.txt");
    pub const V_B1_WARN: &str = include_str!("../../tests/fixtures/seating/v-b1-warn.txt");
    pub const V_B1_CROSS: &str = include_str!("../../tests/fixtures/seating/v-b1-cross.txt");
    pub const V_CE1: &str = include_str!("../../tests/fixtures/seating/v-ce1.txt");
    pub const V_CE1_FOREIGN: &str = include_str!("../../tests/fixtures/seating/v-ce1-foreign.txt");
    pub const V_SPENDEQ_KEYED: &str =
        include_str!("../../tests/fixtures/seating/v-spendeq-keyed.txt");
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;

    // ─── A2's four cells ────────────────────────────────────────────────

    #[test]
    fn a2_declared_fingerprint_must_match() {
        let policy = policy(PATHOLOGICAL);
        let decls = slot_declarations(&policy).unwrap();
        let cards = cards(PATHOLOGICAL);
        assert_eq!(decls.len(), 11);
        assert_eq!(cards.len(), 11);
        // The pathological policy declares 11 DISTINCT (fp, path) pairs and
        // the 11 cards match them one-for-one, so the relation is a
        // bijection: exactly one satisfying card per slot.
        for d in &decls {
            let hits = cards.iter().filter(|c| satisfies(d, c)).count();
            assert_eq!(hits, 1, "slot {} must have exactly one candidate", d.i);
        }
        for c in &cards {
            let hits = decls.iter().filter(|d| satisfies(d, c)).count();
            assert_eq!(hits, 1, "card {} must have exactly one slot", c.label());
        }
    }

    #[test]
    fn a2_compares_decoded_values_not_strings() {
        // The declaration's path arrives as md-codec `OriginPath` components
        // and the card's as a bip32 `DerivationPath` decoded from mk's
        // standard-path indicator. Neither ever renders to a string here;
        // this asserts the decoded values are what matched above, by
        // showing the two RENDERINGS differ in the `m/` prefix while the
        // match still held.
        let policy = policy(PATHOLOGICAL);
        let decls = slot_declarations(&policy).unwrap();
        let cards = cards(PATHOLOGICAL);
        let d = &decls[0];
        let matching = cards.iter().find(|c| satisfies(d, c)).unwrap();
        assert_eq!(d.path, matching.card.origin_path);

        // And the reason a decoded comparison is REQUIRED, constructed: the
        // two hardening spellings the constellation's own tools both emit
        // are one value and two strings.
        use std::str::FromStr;
        let apostrophe = DerivationPath::from_str("48'/0'/0'/2'").unwrap();
        let h_form = DerivationPath::from_str("48h/0h/0h/2h").unwrap();
        assert_eq!(apostrophe, h_form, "one decoded value");
        assert_ne!(
            apostrophe.to_string(),
            "48h/0h/0h/2h",
            "two strings -- a string comparison would refuse this pair"
        );
        assert_eq!(d.path, apostrophe);
    }

    // ─── V-FPFREE-CARD (A2's restrictive half) ──────────────────────────

    #[test]
    fn v_fpfree_card_cannot_satisfy_a_fingerprint_bearing_declaration() {
        let policy = policy(V_FPFREE_CARD);
        let decls = slot_declarations(&policy).unwrap();
        let cards = cards(V_FPFREE_CARD);
        let fp_free: Vec<&DecodedCard> = cards
            .iter()
            .filter(|c| c.card.origin_fingerprint.is_none())
            .collect();
        assert_eq!(
            fp_free.len(),
            1,
            "fixture carries one privacy-preserving card"
        );
        let card = fp_free[0];
        let target = decls
            .iter()
            .find(|d| d.path == card.card.origin_path)
            .expect("the fp-free card sits at a declared path");
        assert!(
            target.fingerprint.is_some(),
            "fixture: that declaration states a fingerprint"
        );
        assert!(
            !satisfies(target, card),
            "a fingerprint-free CARD cannot meet a declared fingerprint blind"
        );
        // ...and the same card WOULD satisfy the same path with no
        // fingerprint declared, so the refusal is about the declaration,
        // not about the path.
        let relaxed = SlotDecl {
            fingerprint: None,
            ..target.clone()
        };
        assert!(satisfies(&relaxed, card));
    }

    #[test]
    fn a2_fingerprint_free_declaration_accepts_a_fingerprint_bearing_card() {
        // A2's third cell — the named residue (r5 M2). Asserted directly so
        // a future "tighten the rule" edit fails here rather than silently
        // making a legitimate set unrestorable (the r4 I1 regression).
        let policy = policy(V_BOUND_SEAT);
        let decls = slot_declarations(&policy).unwrap();
        assert!(decls.iter().all(|d| d.fingerprint.is_none()));
        let cards = cards(V_BOUND_SEAT);
        assert!(cards.iter().all(|c| c.card.origin_fingerprint.is_some()));
        for d in &decls {
            assert_eq!(cards.iter().filter(|c| satisfies(d, c)).count(), 2);
        }
    }

    // ─── V-DOOR ─────────────────────────────────────────────────────────

    #[test]
    fn v_door_two_identical_fp_bearing_declarations_refuse_at_the_door() {
        let policy = policy(V_DOOR);
        let decls = slot_declarations(&policy).unwrap();
        let err = check_no_identical_fp_bearing_declarations(&decls).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("@0 ["), "names the first slot: {msg}");
        assert!(msg.contains("@1 ["), "names the second slot: {msg}");
        assert!(msg.contains("BIP 388"), "names the rule: {msg}");
        assert!(
            msg.contains("UNSUPPORTED"),
            "unsupported, not invalid: {msg}"
        );
        assert!(
            !msg.to_lowercase().contains("invalid"),
            "the word `invalid` is forbidden here (operator ruling): {msg}"
        );
    }

    #[test]
    fn v_door_lets_the_legitimate_same_path_family_through() {
        // The SEAT side of the same boundary: identical PATHS, no
        // fingerprints declared -> not the door's case at all.
        let policy = policy(V_BOUND_SEAT);
        let decls = slot_declarations(&policy).unwrap();
        assert_eq!(decls[0].path, decls[1].path);
        assert!(check_no_identical_fp_bearing_declarations(&decls).is_ok());
    }

    // ─── V-R5M1 ─────────────────────────────────────────────────────────

    /// The door check's wording IS the taxonomy's R-N1a message, which is
    /// the whole content of the P3.3b unification: this asserts the two are
    /// the same string rather than two strings that happen to agree today.
    /// The RENDERED line is pinned as a literal one layer out, in
    /// `tests/seating_vectors.rs::v_r5m1_reaches_the_command`.
    #[test]
    fn v_r5m1_repeated_placeholder_refuses_as_bip388_forbidden() {
        let policy = policy(V_R5M1);
        // The GROUND for the domain claim in the function's doc: the policy
        // at this door is keyless, so Family 2 has nothing to fire on.
        assert!(
            !policy.is_wallet_policy(),
            "a policy card carrying keys never reaches this door check"
        );
        let err = check_no_repeated_placeholder(&policy).unwrap_err();
        let msg = err.to_string();

        // md canonicalises placeholder indices by first appearance before
        // encoding (SPEC section 6.1), so the fixture's source `@0`/`@1`
        // arrive here renumbered: the internal key becomes @0 and the two
        // shared leaf placeholders become @1 and @2, each at two positions.
        // The classifier reports the FIRST finding in a deterministic order
        // (lowest `@i`), so @1 is what the message names.
        assert_eq!(
            msg,
            format!(
                "unsupported: {}",
                crate::parse::reuse::Finding::SamePathExpression { i: 1, sites: 2 }
                    .message(crate::parse::reuse::Disposition::Refuse)
            ),
            "the door check no longer speaks the taxonomy's R-N1a message"
        );
        assert!(msg.contains("forbidden by BIP 388"), "{msg}");
        assert!(
            !msg.to_lowercase().contains("invalid"),
            "the word `invalid` is forbidden here (operator ruling): {msg}"
        );
    }

    #[test]
    fn v_r5m1_control_a_reuse_free_policy_passes_the_same_check() {
        assert!(check_no_repeated_placeholder(&policy(PATHOLOGICAL)).is_ok());
        assert!(check_no_repeated_placeholder(&policy(V_BOUND_SEAT)).is_ok());
    }

    // ─── V-IMPOSS ───────────────────────────────────────────────────────

    #[test]
    fn v_imposs_same_fp_and_path_with_different_xpubs_refuses() {
        let cards = cards(V_IMPOSS);
        let err = check_no_impossible_card_pair(&cards).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("DIFFERENT xpubs"), "{msg}");
        assert!(msg.contains("One master at one derivation path"), "{msg}");
        assert!(
            msg.contains("stub "),
            "cards are named by set id + stub: {msg}"
        );
    }

    #[test]
    fn v_imposs_control_the_genuine_card_set_passes() {
        assert!(check_no_impossible_card_pair(&cards(PATHOLOGICAL)).is_ok());
    }

    // ─── V-BOUND-REF ────────────────────────────────────────────────────

    #[test]
    fn v_bound_ref_same_xpub_twice_refuses_as_bip388_forbidden() {
        let cards = cards(V_BOUND_REF);
        assert_eq!(cards.len(), 2, "fixture: two distinct cards, one xpub");
        let err = check_no_repeated_xpub(&cards).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("SAME extended public key"), "{msg}");
        assert!(msg.contains("BIP 388"), "{msg}");
        assert!(msg.contains("UNSUPPORTED"), "{msg}");
        assert!(
            !msg.to_lowercase().contains("invalid"),
            "the word `invalid` is forbidden here (operator ruling): {msg}"
        );
    }

    /// **The SIBLING**, and the reason `check_no_repeated_xpub` compares KEY
    /// MATERIAL and nothing else: the same xpub declared at two DIFFERENT
    /// paths is still one key in two slots, so it refuses too.
    ///
    /// The row also pins WHICH check answers. Because the two declared paths
    /// differ, neither door check nor the impossible-pair check has a case
    /// here — each is asserted OK first, so the refusal below cannot be some
    /// earlier check firing for another reason and the row cannot pass
    /// vacuously on a fixture that was wrong in a different way.
    ///
    /// Shipped behaviour, measured 2026-08-31 and identical on the plan's
    /// baseline binary; unpinned until plan P3 step 3.
    #[test]
    fn v_bound_ref_paths_same_xpub_at_different_paths_still_refuses() {
        let policy = policy(V_BOUND_REF_PATHS);
        let decls = slot_declarations(&policy).unwrap();
        assert_eq!(decls.len(), 2, "fixture: a two-slot policy");
        assert_ne!(
            decls[0].path, decls[1].path,
            "fixture: the two slots must declare DIFFERENT paths, or this is \
             V-BOUND-REF over again"
        );
        let cards = cards(V_BOUND_REF_PATHS);
        assert_eq!(cards.len(), 2, "fixture: two distinct cards, one xpub");
        assert_ne!(
            cards[0].card.origin_path, cards[1].card.origin_path,
            "fixture: the two cards must declare DIFFERENT paths"
        );

        // Everything upstream of the xpub check has no case here.
        assert!(check_no_repeated_placeholder(&policy).is_ok());
        assert!(check_no_identical_fp_bearing_declarations(&decls).is_ok());
        assert!(check_no_impossible_card_pair(&cards).is_ok());

        // So this is the check that answers.
        let err = check_no_repeated_xpub(&cards).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("SAME extended public key"), "{msg}");
        assert!(msg.contains("BIP 388"), "{msg}");
        assert!(msg.contains("UNSUPPORTED"), "{msg}");
        assert!(
            !msg.to_lowercase().contains("invalid"),
            "the word `invalid` is forbidden here (operator ruling): {msg}"
        );
    }

    #[test]
    fn v_bound_ref_control_different_masters_at_one_path_pass() {
        // V-BOUND-SEAT's cards: same declared path, DIFFERENT xpubs. The
        // reuse check must not fire — this is the anti-over-refusal duty
        // r5-M1 handed to the boundary's seat side.
        assert!(check_no_repeated_xpub(&cards(V_BOUND_SEAT)).is_ok());
        assert!(check_no_repeated_xpub(&cards(PATHOLOGICAL)).is_ok());
    }
}
