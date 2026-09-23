//! The FRESH placeholder walker — SPEC P3 "New walker, not `compile`'s (r1 M5)".
//!
//! ## Why this is not `parse::template`'s machinery run backwards
//!
//! `parse::template::substitute_synthetic` walks the OTHER direction: it turns
//! `@i/…` into a synthetic depth-3/4 xpub so rust-miniscript will parse a
//! template, then `walk_root` reads the result back into an `md_codec` tree.
//! Two properties make it useless here, both measured:
//!
//! 1. It strips placeholders to **bare synthetic xpubs** — origin and key
//!    material are exactly what P3 must PRESERVE, and the synthetic key is
//!    manufactured, not read.
//! 2. Its drift guard forbids `MultiXPub` — and every multipath key in a real
//!    concrete descriptor parses AS `MultiXPub`, so the very inputs P3 exists
//!    for are the ones it rejects.
//!
//! So this module walks a `Descriptor<DescriptorPublicKey>` in its own right.
//!
//! ## How the template is built
//!
//! `miniscript::Descriptor::translate_pk` rebuilds the descriptor over a
//! different key type; translating to `String` and rendering with the ALTERNATE
//! formatter (`{:#}`) yields the structure verbatim with each key replaced by
//! its placeholder text and NO BIP-380 checksum. That matters twice over:
//!
//! * The substitution is STRUCTURAL, not textual — no string surgery on a
//!   descriptor, so a key expression can never be half-replaced.
//! * A BIP-388 template carries no checksum. `md`'s own parser computes the
//!   checksum over the SYNTHETIC-substituted form (measured: a template
//!   bearing its own checksum draws "invalid checksum …; expected …"), so a
//!   suffix here would be one md refuses. `{:#}` is the alternate form the
//!   upstream `write_descriptor!` macro checks (`write_checksum_if_not_alt`).
//!
//! Placeholder NUMBERING is by first appearance in the canonical rendering —
//! textual order, which SPEC "Canonicalisation" requires ("preserve input key
//! order"). It is deliberately NOT `for_each_key` order: measured 2026-08-30,
//! `for_each_key` on `tr(K,pk(L))` yields `[L, K]`, the leaf before the
//! internal key, so numbering from it would relabel every taproot wallet.

use crate::error::CliError;
use bitcoin::Network;
use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint, Xpub};
use miniscript::descriptor::{Descriptor, DescriptorPublicKey, Wildcard};
use miniscript::{ForEachKey, Translator};
use std::collections::{BTreeMap, BTreeSet};

/// One KEY expression occurrence, as parsed and as rendered.
///
/// Every text field is taken from the key's OWN canonical rendering rather
/// than rebuilt from its parts, so what decompose emits is byte-identical to
/// what the descriptor it emits alongside contains. That is also what makes
/// the hardened spelling right for free: `bip32::ChildNumber`'s `Display`
/// emits `'`, so an `h`-spelled input is normalised on the way out (SPEC
/// "Canonicalisation": md emits `'`, and the spelling changes the checksum).
#[derive(Debug, Clone)]
pub struct Occurrence {
    /// The whole key expression, e.g. `[fp/48'/0'/0'/2']xpub…/<0;1>/*`.
    pub display: String,
    /// The `mk encode --keys` record: `[fp/path]xpub`, no use-site suffix.
    /// For an origin-less key this is the bare xpub.
    pub record: String,
    /// `/48'/0'/0'/2'` (leading slash) or `""` when the key states no origin
    /// path. This is what follows `@i` in the emitted template.
    pub origin_path_text: String,
    /// `/<0;1>/*` — everything after the xpub.
    pub use_site: String,
    /// The extended key itself, AS PARSED: true depth, child number and
    /// parent fingerprint (SPEC P3 "Key emission is round-trip-grade").
    pub xpub: Xpub,
    /// Origin as decoded, `None` when the expression carries no `[...]`.
    pub origin: Option<(Fingerprint, DerivationPath)>,
    /// The BIP-389 multipath set: every derivation path this occurrence
    /// derives at. A single-path key contributes one element.
    pub paths: BTreeSet<String>,
}

impl Occurrence {
    /// `73c5da0a` or `None`.
    pub fn fingerprint(&self) -> Option<Fingerprint> {
        self.origin.as_ref().map(|(f, _)| *f)
    }

    /// A short label for diagnostics: the origin if there is one, else the
    /// truncated key.
    pub fn label(&self) -> String {
        match &self.origin {
            Some((f, p)) if p.as_ref().is_empty() => format!("[{f}]"),
            Some((f, p)) => format!("[{f}/{p}]"),
            None => format!("(no origin) {}…", &self.record[..16.min(self.record.len())]),
        }
    }
}

/// Split a rendered key expression into `([origin]`, `xpub`, `use-site)`.
///
/// The xpub is base58 and so contains no `/`, and the origin — which may — is
/// delimited by `]`. That makes the split unambiguous without re-parsing.
fn split_rendered(display: &str) -> (String, String, String) {
    let (origin_part, rest) = match display.strip_prefix('[') {
        Some(after) => match after.find(']') {
            Some(close) => (display[..=close + 1].to_string(), &after[close + 1..]),
            // Unreachable: the value parsed, so a `[` had its `]`.
            None => (String::new(), display),
        },
        None => (String::new(), display),
    };
    let cut = rest.find('/').unwrap_or(rest.len());
    (
        origin_part,
        rest[..cut].to_string(),
        rest[cut..].to_string(),
    )
}

/// The `/path` half of an origin bracket, leading slash included, or `""`.
fn origin_path_text(origin_part: &str) -> String {
    if origin_part.len() < 2 {
        return String::new();
    }
    // `[` + 8 hex fingerprint chars + optional `/path` + `]`
    let inner = &origin_part[1..origin_part.len() - 1];
    match inner.find('/') {
        Some(slash) => inner[slash..].to_string(),
        None => String::new(),
    }
}

fn occurrence_of(key: &DescriptorPublicKey) -> Result<Occurrence, CliError> {
    let display = key.to_string();
    let (origin_part, _xpub_text, use_site) = split_rendered(&display);
    let origin_path_text = origin_path_text(&origin_part);
    let (origin, xpub, paths) = match key {
        DescriptorPublicKey::XPub(x) => {
            let mut set = BTreeSet::new();
            set.insert(x.derivation_path.to_string());
            (x.origin.clone(), x.xkey, set)
        }
        DescriptorPublicKey::MultiXPub(m) => {
            let set = m
                .derivation_paths
                .paths()
                .iter()
                .map(|p| p.to_string())
                .collect();
            (m.origin.clone(), m.xkey, set)
        }
        DescriptorPublicKey::Single(_) => {
            return Err(CliError::Decompose(format!(
                "this descriptor contains a RAW public key, not an extended key — `{display}`. \
                 An md wallet policy seats one xpub per placeholder and an mk1 card carries an \
                 xpub, so there is nothing to decompose into a slot here. UNSUPPORTED: replace \
                 the raw key with the extended key it came from, or engrave this wallet by some \
                 other route."
            )));
        }
    };
    Ok(Occurrence {
        record: format!("{origin_part}{_xpub_text}"),
        display,
        origin_path_text,
        use_site,
        xpub,
        origin,
        paths,
    })
}

/// SPEC §4a / §2 (`design/SPEC_liana_unspendable_internal_key.md`) — decompose's
/// OWN recogniser for a real Liana-unspendable taproot internal key.
///
/// This is the one surface of the three (`md decompose` / the template
/// grammar / `md encode`) that CAN recompute §2's recipe: it still holds the
/// real leaf keys, unlike `md encode`'s template pipeline, whose leaves are
/// already synthetic placeholders by the time an internal key is compared
/// (`parse::template::substitute_synthetic`'s own doc explains why a match
/// there can never succeed).
///
/// Deliberately walks `desc` directly — at the `Tr` node — rather than going
/// through `for_each_key`/`collect_occurrences`, which flattens the internal
/// key and every leaf key into one undifferentiated multiset with NO
/// structural position (`Placeholders::pk`'s whole contract is per-key, not
/// per-position). Recomputing the recipe needs exactly the distinction that
/// flattening destroys: "internal key" vs. "leaf key, in tap-tree order".
///
/// Returns the internal key's own rendered text (`DescriptorPublicKey::
/// to_string()`, byte-identical to what `occurrence_of` would compute as
/// `Occurrence::display`) on a byte match, so the caller can drop that ONE
/// occurrence out of the slot-numbered set before `collect_occurrences` ever
/// runs, and substitute the marker in its place. `None` on ANY mismatch —
/// including a non-`Tr` descriptor, a `Single` (non-extended) internal key,
/// or a `Single` leaf key that cannot contribute a 33-byte compressed pubkey
/// — which is deliberately indistinguishable from "recipe did not match":
/// every such shape falls through to today's annotated-slot path unchanged
/// (G-8), where the existing walker's own refusals (e.g. "RAW public key")
/// name the real problem if there is one.
///
/// **C-1 fix (whole-branch review, Critical).** `UNSPENDABLE(liana)` DENOTES
/// the whole key expression `<derived xpub>/<0;1>/*`, with NO origin — not
/// just the xpub bytes. Liana derives the internal key at exactly `0/i` and
/// `1/i` in every port (SPEC §2 step 6, "in every port, independent of the
/// wallet's use-site path"), so `UNSPENDABLE(liana)` on a template can only
/// ever mean that one BIP-389 spelling. Comparing only `actual_xpub ==
/// recomputed` (the pre-fix code) accepted ANY use-site on a byte-matching
/// xpub — a different multipath order, a fixed step, a hardened wildcard, a
/// prefixed origin — and replaced it with the marker regardless, silently
/// turning a real wallet whose internal key derives at, say, `2/i` into a
/// card that reads `0/i`. Below, every property BIP-389's `<0;1>` multipath
/// spelling carries is checked explicitly:
/// - `origin` is `None` (no `[fingerprint/path]` prefix) — REPRODUCED base
///   `37367c1f`'s own refusal of a prefixed origin here.
/// - the key is `MultiXPub`, not `XPub` — a SINGLE fixed path (`/0/*`, `/0`)
///   or no wildcard step at all (`/*h` with no `<;>`) can never spell
///   `<0;1>`, since that spelling requires BIP-389 multipath syntax in the
///   first place; this one variant check rules out `/0/*`, `/0` and `/*h`
///   together, with no separate per-case logic.
/// - `wildcard` is exactly `Unhardened` (`*`, never `*h`).
/// - `derivation_paths` is exactly `[[0], [1]]`, IN THAT ORDER: rejects
///   `<1;0>` (order), `<0;1;2>` (arity — three alternatives, not two), and
///   `<0;1>/5/*` (a trailing fixed step folds into EACH alternative's own
///   path — rust-miniscript reconstructs `/<0;1>/5/*` as
///   `derivation_paths == [[0,5], [1,5]]`, two components per path, not
///   one — measured directly against the pinned miniscript checkout before
///   writing this comment, not assumed from the grammar).
pub fn liana_internal_key_match(
    desc: &Descriptor<DescriptorPublicKey>,
    network: Network,
) -> Option<String> {
    let Descriptor::Tr(tr) = desc else {
        return None;
    };
    let internal = tr.internal_key();
    let m = match internal {
        DescriptorPublicKey::MultiXPub(m) => m,
        // A single-path xkey (`XPub`) or a raw key (`Single`) can never BE
        // `<0;1>/*` — that spelling requires BIP-389 multipath syntax, which
        // only `MultiXPub` represents. Covers `/0/*`, `/0` and `/*h` (none
        // carry a `<;>` marker at all) without a separate check for each.
        DescriptorPublicKey::XPub(_) | DescriptorPublicKey::Single(_) => return None,
    };
    if m.origin.is_some() {
        return None;
    }
    if m.wildcard != Wildcard::Unhardened {
        return None;
    }
    let paths = m.derivation_paths.paths();
    let standard = |i: u32| DerivationPath::from(vec![ChildNumber::Normal { index: i }]);
    if paths.len() != 2 || paths[0] != standard(0) || paths[1] != standard(1) {
        return None;
    }
    let actual_xpub = m.xkey;

    // SPEC §2 step 1: the ordered sequence of the tap-tree's leaf keys' own
    // 33-byte compressed public keys, one per OCCURRENCE (not deduplicated),
    // in tap-tree left-to-right order. `TapTree::leaves()` is rust-miniscript's
    // own depth-first-preorder walk (the same order `walk_tap_tree`'s md1 AST
    // reconstruction relies on being stable); each leaf's `iter_pk()` walks
    // its OWN key expressions in the same left-first order.
    let mut leaf_pubkeys: Vec<[u8; 33]> = Vec::new();
    if let Some(tap_tree) = tr.tap_tree() {
        for leaf in tap_tree.leaves() {
            for pk in leaf.miniscript().iter_pk() {
                let bytes = match &pk {
                    DescriptorPublicKey::XPub(x) => x.xkey.public_key.serialize(),
                    DescriptorPublicKey::MultiXPub(x) => x.xkey.public_key.serialize(),
                    // A raw single pubkey leaf: no recipe input to extract.
                    // Not this recogniser's refusal to make — fall through to
                    // the normal walk, which names it plainly ("RAW public
                    // key") when `collect_occurrences` reaches it.
                    DescriptorPublicKey::Single(_) => return None,
                };
                leaf_pubkeys.push(bytes);
            }
        }
    }

    let recomputed = md_codec::nums::liana_unspendable_xpub(&leaf_pubkeys, network);
    if actual_xpub == recomputed {
        Some(internal.to_string())
    } else {
        None
    }
}

/// Collect every KEY expression occurrence in TEXTUAL order.
///
/// `for_each_key` supplies the multiset (repeats included — measured: a key at
/// two positions is yielded twice); the canonical rendering supplies the order.
/// Ordering by `str::find` is sound because the caller refuses every repeated
/// key BEFORE numbering (see `mod.rs`), so each rendered expression occurs
/// exactly once and each contains its own 111-character xpub — no expression
/// can be a substring of another.
pub fn collect_occurrences(
    desc: &Descriptor<DescriptorPublicKey>,
) -> Result<Vec<Occurrence>, CliError> {
    let mut keys: Vec<DescriptorPublicKey> = Vec::new();
    desc.for_each_key(|k| {
        keys.push(k.clone());
        true
    });
    keys.iter().map(occurrence_of).collect()
}

/// Order occurrences by first appearance in `rendering`. Only meaningful once
/// repeats are refused; see `collect_occurrences`.
pub fn order_by_appearance(occurrences: &mut [Occurrence], rendering: &str) {
    occurrences.sort_by_key(|o| rendering.find(&o.display).unwrap_or(usize::MAX));
}

/// Substitutes each key expression with its `@i…` placeholder text.
struct Placeholders {
    /// rendered key expression → template text
    map: BTreeMap<String, String>,
}

impl Translator<DescriptorPublicKey> for Placeholders {
    type TargetPk = String;
    type Error = CliError;

    fn pk(&mut self, pk: &DescriptorPublicKey) -> Result<String, CliError> {
        let rendered = pk.to_string();
        self.map.get(&rendered).cloned().ok_or_else(|| {
            // Fail closed: emitting a template with an unsubstituted key would
            // put key material in what is meant to be the KEYLESS half.
            CliError::Decompose(format!(
                "internal error: key `{rendered}` was not assigned a placeholder"
            ))
        })
    }

    fn sha256(&mut self, h: &bitcoin::hashes::sha256::Hash) -> Result<String, CliError> {
        Ok(h.to_string())
    }
    fn hash256(&mut self, h: &miniscript::hash256::Hash) -> Result<String, CliError> {
        Ok(h.to_string())
    }
    fn ripemd160(&mut self, h: &bitcoin::hashes::ripemd160::Hash) -> Result<String, CliError> {
        Ok(h.to_string())
    }
    fn hash160(&mut self, h: &bitcoin::hashes::hash160::Hash) -> Result<String, CliError> {
        Ok(h.to_string())
    }
}

/// Build the keyless BIP-388 template: every key expression replaced by
/// `@i` + its origin path + its use-site path — EXCEPT the recognised Liana
/// internal key (SPEC §4a), which `liana_internal_key_match` has already
/// excluded from `occurrences` (no slot) and which is substituted here by
/// its own rendered text (from that same function) to the fixed marker
/// instead of a numbered placeholder.
pub fn build_template(
    desc: &Descriptor<DescriptorPublicKey>,
    occurrences: &[Occurrence],
    liana_internal_key: Option<&str>,
) -> Result<String, CliError> {
    let mut map = BTreeMap::new();
    for (i, o) in occurrences.iter().enumerate() {
        map.insert(
            o.display.clone(),
            format!("@{i}{}{}", o.origin_path_text, o.use_site),
        );
    }
    if let Some(key_display) = liana_internal_key {
        map.insert(
            key_display.to_string(),
            md_codec::nums::LIANA_UNSPENDABLE_MARKER.to_string(),
        );
    }
    let mut t = Placeholders { map };
    let translated = desc.translate_pk(&mut t).map_err(|e| match e {
        miniscript::TranslateErr::TranslatorErr(inner) => inner,
        miniscript::TranslateErr::OuterError(outer) => CliError::Decompose(format!(
            "internal error: the keyless template did not re-form: {outer}"
        )),
    })?;
    // `{:#}` — the alternate form, which suppresses the BIP-380 checksum.
    let template = format!("{translated:#}");

    // FAIL-CLOSED structural guard. The whole point of the keyless half is
    // that it carries no key material; a walker defect that left one key in
    // place would otherwise ship silently. Cheap, and it cannot false-pass:
    // every extended key serialisation in every network begins with one of
    // these four-character prefixes.
    for marker in ["xpub", "tpub", "ypub", "zpub", "vpub", "upub"] {
        if template.contains(marker) {
            return Err(CliError::Decompose(format!(
                "internal error: the emitted template still contains key material \
                 (`{marker}`); refusing to print it"
            )));
        }
    }
    let placeholders = template.matches('@').count();
    if placeholders != occurrences.len() {
        return Err(CliError::Decompose(format!(
            "internal error: emitted {placeholders} placeholder(s) for {} key(s)",
            occurrences.len()
        )));
    }
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const K0: &str = "[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
    const K1: &str = "[73c5da0a/48'/0'/1'/2']xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";

    fn parse(s: &str) -> Descriptor<DescriptorPublicKey> {
        Descriptor::<DescriptorPublicKey>::from_str(s).expect("fixture must parse")
    }

    fn walked(s: &str) -> (Vec<Occurrence>, String) {
        let d = parse(s);
        let mut occ = collect_occurrences(&d).unwrap();
        order_by_appearance(&mut occ, &format!("{d:#}"));
        let t = build_template(&d, &occ, None).unwrap();
        (occ, t)
    }

    #[test]
    fn splits_origin_key_and_use_site() {
        let (occ, _) = walked(&format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K1}/<0;1>/*))"));
        assert_eq!(occ[0].record, K0);
        assert_eq!(occ[0].origin_path_text, "/48'/0'/0'/2'");
        assert_eq!(occ[0].use_site, "/<0;1>/*");
        assert_eq!(occ[0].paths, ["0".to_string(), "1".to_string()].into());
    }

    #[test]
    fn template_is_keyless_and_checksum_free() {
        let (_, t) = walked(&format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K1}/<0;1>/*))"));
        assert_eq!(
            t,
            "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))"
        );
    }

    /// The taproot case `for_each_key` gets wrong. Measured 2026-08-30:
    /// `for_each_key` on `tr(K,pk(L))` yields `[L, K]`, so numbering from its
    /// order would label the INTERNAL key `@1`. This test fails if the
    /// `order_by_appearance` step is dropped.
    #[test]
    fn taproot_internal_key_is_slot_zero() {
        let (occ, t) = walked(&format!("tr({K0}/<0;1>/*,pk({K1}/<0;1>/*))"));
        assert_eq!(occ[0].record, K0, "the internal key is textually first");
        assert_eq!(t, "tr(@0/48'/0'/0'/2'/<0;1>/*,pk(@1/48'/0'/1'/2'/<0;1>/*))");
    }

    /// SPEC "Canonicalisation": `'`, never `h`. rust-miniscript re-renders the
    /// origin through `bip32::ChildNumber`, so an `h`-spelled input normalises
    /// on the way out — asserted rather than assumed.
    #[test]
    fn h_spelled_input_is_emitted_with_apostrophes() {
        let h_form = K0.replace('\'', "h");
        assert!(h_form.contains("48h/"));
        let (occ, t) = walked(&format!(
            "wsh(sortedmulti(2,{h_form}/<0;1>/*,{K1}/<0;1>/*))"
        ));
        assert_eq!(occ[0].record, K0);
        assert!(
            !t.contains("48h/") && !t.contains("0h/") && !t.contains("2h/"),
            "an `h` spelling survived: {t}"
        );
        assert!(t.contains("@0/48'/0'/0'/2'/<0;1>/*"), "{t}");
    }

    #[test]
    fn origin_less_key_gets_a_bare_placeholder_and_a_bare_record() {
        let bare = K0.split(']').nth(1).unwrap().to_string();
        let (occ, t) = walked(&format!("wsh(sortedmulti(2,{bare}/<0;1>/*,{K1}/<0;1>/*))"));
        assert_eq!(occ[0].record, bare);
        assert!(occ[0].origin.is_none());
        assert_eq!(occ[0].origin_path_text, "");
        assert!(t.starts_with("wsh(sortedmulti(2,@0/<0;1>/*,"), "{t}");
    }

    #[test]
    fn fingerprint_only_origin_contributes_no_path_segment() {
        let bare = K0.split(']').nth(1).unwrap().to_string();
        let (occ, t) = walked(&format!(
            "wsh(sortedmulti(2,[73c5da0a]{bare}/<0;1>/*,{K1}/<0;1>/*))"
        ));
        assert_eq!(occ[0].fingerprint().unwrap().to_string(), "73c5da0a");
        assert_eq!(occ[0].origin_path_text, "");
        assert!(t.starts_with("wsh(sortedmulti(2,@0/<0;1>/*,"), "{t}");
    }

    #[test]
    fn a_raw_public_key_is_refused_by_name() {
        // 33-byte compressed key, no extended key to seat.
        let raw = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let d = parse(&format!("wsh(multi(2,{raw},{K1}/<0;1>/*))"));
        let err = collect_occurrences(&d).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("RAW public key"), "{msg}");
        assert!(msg.contains("UNSUPPORTED"), "{msg}");
    }
}
