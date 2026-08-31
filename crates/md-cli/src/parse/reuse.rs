//! N1 — the placeholder/key-reuse admission taxonomy, in ONE place.
//!
//! `design/SPEC_mdcli_mini.md` §N1 classifies two families on the mint/compose
//! surface:
//!
//! * **Family 1** — one placeholder at more than one use site. The key is the
//!   triple the lexer already records per occurrence: (inline origin path,
//!   multipath set, wildcard hardening). Five outcomes, on three different
//!   authorities — see [`Finding`].
//! * **Family 2** — one key at more than one placeholder. Only the
//!   DISJOINT-use-site **delta** belongs here; the same-use-site case has been
//!   refused at the codec floor since F-218
//!   (`md_codec::validate::validate_no_duplicate_key_slots`, called inside
//!   `encode_payload`) and keeps its own wording and its own position. This
//!   module must never absorb it — that would be a second implementation of a
//!   shipped predicate, which the spec's single-source rule forbids, and it
//!   would replace a correct message with one written for a different case.
//!
//! # Two entrances, one classifier
//!
//! [`check`] takes the two inputs directly and is what the TEMPLATE path
//! calls, from `parse_template_ext`. [`check_descriptor`] reconstructs the
//! same two inputs from a DECODED card and is what the CARD path calls —
//! `cmd/build.rs`'s phrase branch (`md descriptor`/`md address`), the three
//! reading verbs, and the seating engine's door check. There is one
//! predicate underneath both, so one wallet draws one message however it
//! arrived.
//!
//! # Why the input is an occurrence list plus key bindings
//!
//! The spec states the single-source rule as a statement about the
//! classifier's INPUT rather than about a code location, because no single
//! existing location sees both halves: Family 1 needs only the per-occurrence
//! triples, Family 2's delta needs the resolved per-`@i` key material, and
//! `resolve_placeholders` (which collapses the occurrences) carries no key
//! material at all. Both halves ARE in hand at `parse_template_ext` time on
//! the template path, and both are reconstructible from a decoded card on the
//! card path — so this module takes them as arguments and is invoked from
//! wherever they exist.
//!
//! # Why the disposition is a parameter
//!
//! Each predicate has exactly ONE implementation here. The verbs differ only
//! in what they DO with a finding: `encode`, `descriptor` and `address` refuse
//! (they mint or compose), while the reading verbs warn and proceed, so an
//! operator can still check a legacy plate carrying a shape this cycle newly
//! refuses (SPEC N1 "Verb dispositions", and Acceptance 5 — such plates exist:
//! `crates/md-cli/tests/fixtures/n1/`). A per-verb second copy of a predicate
//! is what `cmd/build.rs`'s own rule already forbids, and it is how the two
//! sides drift into disagreeing about the same wallet.
//!
//! # Placement constraint (NORMATIVE, SPEC §"Placement constraint")
//!
//! Nothing here may be added to `encode_payload`'s validator set: `md inspect`
//! and `md verify` re-enter `encode_payload` on a DECODED card, so a check
//! there would make already-engraved plates of newly-refused shapes
//! uninspectable and unverifiable.

use crate::error::CliError;
use crate::parse::keys::ParsedKey;
use crate::parse::template::PlaceholderOccurrence;
use md_codec::encode::Descriptor;
use md_codec::tree::{Body, Node};

/// What a verb does with a finding. NEVER a second implementation of the
/// predicate — only what happens after it fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Mint/compose surface: `encode`, `descriptor`, `address`.
    Refuse,
    /// Read surface: the finding is reported and the verb proceeds, so a plate
    /// already carrying the shape stays readable (Acceptance 5).
    Warn,
}

/// One classified violation. Each variant carries the values its message
/// quotes, so the rendering below can echo the operator's OWN template rather
/// than a canned example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// **R-N1a** — one placeholder, several use sites, IDENTICAL triples.
    /// BIP 388 forbids it by name.
    SamePathExpression { i: u8, sites: usize },
    /// **R-N1b** — the triples differ only in their multipath sets, and the
    /// sets OVERLAP. BIP 388's disjointness rule.
    MultipathOverlap {
        i: u8,
        a: String,
        b: String,
        shared: String,
    },
    /// **R-N1c** — the triples differ only in their multipath sets, and the
    /// sets are DISJOINT. The wallet is BIP-388-LEGAL; md1 cannot express it
    /// (F-417). The one refusal in this taxonomy that is not about a defect.
    MultipathDisjoint { i: u8, a: String, b: String },
    /// **R-N1-origin** — the triples differ in inline ORIGIN. md's own
    /// representability limit; cites no BIP rule.
    OriginDiffers { i: u8, a: String, b: String },
    /// **R-N1-hardening** — the triples differ in wildcard HARDENING. md's own
    /// derivability limit; cites no BIP rule.
    HardeningDiffers { i: u8, a: String, b: String },
    /// **R-N1d** — one key at two placeholders whose use sites DIFFER. The
    /// same-use-site case is NOT this variant; it belongs to the codec floor.
    KeyAtDisjointUseSites {
        a: u8,
        b: u8,
        sa: String,
        sb: String,
    },
}

/// The multipath set as the operator wrote it: `<0;1>`, or a plain-language
/// stand-in when the placeholder carries no multipath group at all.
fn multipath_display(alts: &[u32]) -> String {
    if alts.is_empty() {
        return "no multipath group".to_owned();
    }
    let body: Vec<String> = alts.iter().map(u32::to_string).collect();
    format!("<{}>", body.join(";"))
}

/// The use-site half of the triple — multipath set plus the wildcard's
/// hardening, which is exactly what the wire's `UseSitePath` records.
fn use_site_display(occ: &PlaceholderOccurrence) -> String {
    let star = if occ.wildcard_hardened { "*'" } else { "*" };
    if occ.multipath_alts.is_empty() {
        star.to_owned()
    } else {
        format!("{}/{star}", multipath_display(&occ.multipath_alts))
    }
}

/// The inline origin as written, or a plain-language stand-in for its absence.
fn origin_display(occ: &PlaceholderOccurrence) -> String {
    match &occ.origin_path {
        None => "no inline origin".to_owned(),
        Some(p) => format!("/{p}"),
    }
}

/// True iff two occurrences agree on all three components of the key triple.
fn same_triple(a: &PlaceholderOccurrence, b: &PlaceholderOccurrence) -> bool {
    a.origin_path == b.origin_path
        && a.multipath_alts == b.multipath_alts
        && a.wildcard_hardened == b.wildcard_hardened
}

/// Classify the resolved template. Returns the FIRST finding in a
/// deterministic order, or `None` when nothing is wrong.
///
/// ORDER, and it is deliberate rather than incidental:
///
/// 1. **Family 1 before Family 2.** Family 1 is a property of the template
///    alone and fires on the keyless spelling too, so reporting it first means
///    the same template draws the same diagnostic whether or not keys were
///    supplied.
/// 2. **Lowest `@i` first**, then first divergence in TEMPLATE order, so the
///    message names the earliest place an operator can look.
/// 3. Within one placeholder: **origin, then hardening, then multipath.** The
///    spec's two multipath rows are qualified "differ ONLY in multipath sets",
///    so they are reachable only once the other two axes agree; between origin
///    and hardening the order is this module's choice, and origin goes first
///    because it is the axis an operator is likelier to have written on
///    purpose.
pub fn classify(occs: &[PlaceholderOccurrence], keys: &[ParsedKey]) -> Option<Finding> {
    // ── Family 1 ───────────────────────────────────────────────────────────
    let mut indices: Vec<u8> = occs.iter().map(|o| o.i).collect();
    indices.sort_unstable();
    indices.dedup();

    for i in &indices {
        let group: Vec<&PlaceholderOccurrence> = occs.iter().filter(|o| o.i == *i).collect();
        if group.len() < 2 {
            continue;
        }
        let head = group[0];
        let Some(other) = group[1..].iter().find(|o| !same_triple(head, o)) else {
            return Some(Finding::SamePathExpression {
                i: *i,
                sites: group.len(),
            });
        };
        if head.origin_path != other.origin_path {
            return Some(Finding::OriginDiffers {
                i: *i,
                a: origin_display(head),
                b: origin_display(other),
            });
        }
        if head.wildcard_hardened != other.wildcard_hardened {
            return Some(Finding::HardeningDiffers {
                i: *i,
                a: use_site_display(head),
                b: use_site_display(other),
            });
        }
        // Only the multipath sets are left, or `same_triple` would have held.
        let shared: Vec<String> = head
            .multipath_alts
            .iter()
            .filter(|v| other.multipath_alts.contains(v))
            .map(|v| v.to_string())
            .collect();
        let (a, b) = (
            multipath_display(&head.multipath_alts),
            multipath_display(&other.multipath_alts),
        );
        return Some(if shared.is_empty() {
            Finding::MultipathDisjoint { i: *i, a, b }
        } else {
            Finding::MultipathOverlap {
                i: *i,
                a,
                b,
                shared: shared.join(", "),
            }
        });
    }

    // ── Family 2, the DISJOINT-use-site delta only ─────────────────────────
    //
    // Identical key MATERIAL — the 65-byte `chain code ‖ compressed pubkey`,
    // the same comparison the codec floor makes and for the same reason: the
    // fingerprint identifies a MASTER (so it would refuse the legitimate
    // cosigner contributing two accounts) and the base58 string carries
    // depth/parent metadata that differs between two sources of one key.
    //
    // The SAME-use-site case is deliberately NOT reported here. It is the
    // codec floor's, and falls through to it.
    for (n, a) in keys.iter().enumerate() {
        for b in &keys[n + 1..] {
            if a.i == b.i || a.payload != b.payload {
                continue;
            }
            let (Some(oa), Some(ob)) = (
                occs.iter().find(|o| o.i == a.i),
                occs.iter().find(|o| o.i == b.i),
            ) else {
                continue;
            };
            if oa.multipath_alts == ob.multipath_alts
                && oa.wildcard_hardened == ob.wildcard_hardened
            {
                continue; // same use site — the codec floor's case, not ours
            }
            let (lo, hi) = if a.i < b.i { (a, b) } else { (b, a) };
            let (olo, ohi) = if a.i < b.i { (oa, ob) } else { (ob, oa) };
            return Some(Finding::KeyAtDisjointUseSites {
                a: lo.i,
                b: hi.i,
                sa: use_site_display(olo),
                sb: use_site_display(ohi),
            });
        }
    }
    None
}

impl Finding {
    /// The diagnostic BODY — everything after the rendered prefix.
    ///
    /// It is the SAME text under either disposition for every finding except
    /// [`Finding::SamePathExpression`] (R-N1a), whose final clause is
    /// disposition-aware (review r1 M4): a REFUSE tail says md declines to
    /// mint or compose, which is only true when something was actually
    /// refused; a WARN tail says the card still reads and names the shape's
    /// actual consequence instead. That is still one classifier and one
    /// body up to the tail — "one implementation, the disposition is a
    /// parameter" is about the predicate and the shared prefix, not a claim
    /// that no finding may ever read the disposition it was given.
    ///
    /// The word "invalid" appears in none of these. A BIP-forbidden or
    /// wire-inexpressible shape is UNSUPPORTED, never invalid — a repeated-key
    /// descriptor is legal script, and calling the operator's wallet invalid
    /// is both false and the wrong instruction (operator ruling 2026-08-30).
    pub fn message(&self, disposition: Disposition) -> String {
        match self {
            Finding::SamePathExpression { i, sites } => {
                // BIP 388's rule (1) (pairwise distinctness of the key
                // information VECTOR) is satisfied here — the vector holds
                // only ONE element for one placeholder repeated verbatim.
                // What this shape breaks is rule (2), the disjointness rule:
                // the SAME multipath set compared with itself is never
                // disjoint from it (review r1 I1; the prior citation named
                // rule (1), which this shape does not violate).
                let tail = match disposition {
                    Disposition::Refuse => {
                        "md declines to mint or compose this shape: give \
                         each distinct key its own placeholder."
                    }
                    Disposition::Warn => {
                        "This shape can no longer be minted or composed; the \
                         card remains readable."
                    }
                };
                format!(
                    "@{i} appears at {sites} use sites in this template with the same path \
                     expression, so ONE key would fill every one of them. That is forbidden by \
                     BIP 388's disjointness rule (\"{rule}\"), whose forbidden-example list \
                     names sh(multi(1,@0/**,@0/**)) — \"Repeated keys with the same path \
                     expression\". {tail}",
                    rule = crate::bip388::DISJOINTNESS_RULE,
                )
            }
            Finding::MultipathOverlap { i, a, b, shared } => format!(
                "@{i} appears at use sites whose multipath sets OVERLAP — {a} and {b} share \
                 {shared}. BIP 388 requires two key expressions on one placeholder to have \
                 DISJOINT multipath sets, so this shape is forbidden; md1 could not carry two \
                 use sites for one key slot in any case (one path per key slot, F-417), but \
                 the disjointness rule is the primary ground. md declines to mint or compose \
                 this shape: give each use site its own placeholder."
            ),
            Finding::MultipathDisjoint { i, a, b } => format!(
                "@{i} appears at use sites with DISJOINT multipath sets — {a} and {b}. The \
                 WALLET is legal under BIP 388, which permits exactly this on one \
                 placeholder; md1 deliberately cannot express it, because an md1 card carries \
                 ONE path per key slot and that narrowness is a design decision the wire \
                 format will not widen (F-417). md declines to mint or compose this shape. \
                 Keep this wallet as a descriptor instead: {ESCAPE}"
            ),
            Finding::OriginDiffers { i, a, b } => format!(
                "@{i} appears at use sites declaring DIFFERENT key origins — {a} and {b}. One \
                 placeholder is one key slot and an md1 card records ONE origin per key slot, \
                 so it cannot carry both. Inline origins are md's own normal template \
                 spelling, so this is md's representability limit and not a statement about \
                 your wallet. md declines to mint or compose this shape: give each origin its \
                 own placeholder."
            ),
            Finding::HardeningDiffers { i, a, b } => format!(
                "@{i} appears at use sites whose wildcards differ in HARDENING — {a} and {b}. \
                 One placeholder is one key slot and an md1 card records ONE use-site \
                 wildcard per slot, so it cannot carry both; an xpub derives no hardened \
                 child either, so one key could not serve both use sites even if the card \
                 could carry them. This is md's own derivability limit, not a statement about \
                 your wallet. md declines to mint or compose this shape: give each use site \
                 its own placeholder."
            ),
            Finding::KeyAtDisjointUseSites { a, b, sa, sb } => format!(
                "@{a} and @{b} were given the SAME extended public key at DIFFERENT use sites \
                 — {sa} and {sb}. Spelled with two placeholders, this policy lists that key \
                 TWICE in BIP 388's key information vector, and rule (1) requires \"{rule}\" \
                 — so what BIP 388 forbids is THIS SPELLING's key vector, not the wallet it \
                 describes. The wallet — one key at two disjoint path sets — is a legal \
                 descriptor, and BIP 388 writes it with ONE placeholder carrying both sets; \
                 md1 cannot write that spelling either, because an md1 card carries one path \
                 per key slot (F-417). md declines to mint or compose this shape. Keep this \
                 wallet as a descriptor: {ESCAPE}",
                rule = crate::bip388::PAIRWISE_DISTINCT_RULE,
            ),
        }
    }
}

/// The RUNNABLE escape, quoted identically by R-N1c and R-N1d.
///
/// `me sysw pack` is the real surface (`me-cli/src/main.rs:723`), not an
/// invented one: a refusal that names no way forward leaves an operator
/// holding a legal wallet and no tool, which is the outcome these two rows
/// exist to avoid.
///
/// `pub(crate)` (review r1 I5) so `decompose`'s D-row disjoint-multipath
/// refusal — the SAME wallet R-N1c refuses, reached from a concrete
/// descriptor instead of a template — can name the identical escape rather
/// than its own vaguer wording drifting from this one.
pub(crate) const ESCAPE: &str = "me sysw pack --as descriptor --in <your export file>";

/// Classify, then apply the caller's disposition.
///
/// `Warn` writes to stderr and returns `Ok(())`; the caller proceeds. `Refuse`
/// returns [`CliError::Unsupported`], which renders as
/// `md: unsupported: <body>` at exit 1.
pub fn check(
    occs: &[PlaceholderOccurrence],
    keys: &[ParsedKey],
    disposition: Disposition,
) -> Result<(), CliError> {
    let Some(finding) = classify(occs, keys) else {
        return Ok(());
    };
    match disposition {
        Disposition::Refuse => Err(CliError::Unsupported(finding.message(disposition))),
        Disposition::Warn => {
            eprintln!("md: warning: {}", finding.message(disposition));
            Ok(())
        }
    }
}

/// Classify a DECODED CARD, then apply the caller's disposition.
///
/// The card path's entrance. It reconstructs the classifier's two inputs
/// from the wire and hands them to [`check`], so the CARD and the TEMPLATE
/// are judged by one predicate and answered with one message.
///
/// # What the wire can and cannot carry, and why that shrinks the taxonomy
///
/// A card records ONE origin, ONE multipath set and ONE wildcard hardening
/// per KEY SLOT — that is F-417's narrowness, the wire fact the whole N1
/// cycle is built around. So every occurrence of `@i` on a card necessarily
/// carries the IDENTICAL triple, and Family 1 has exactly one reachable
/// outcome here: [`Finding::SamePathExpression`]. The four axis-divergence
/// rows (origin, hardening, overlapping and disjoint multipath sets) are
/// template-only by construction, not by omission.
///
/// Family 2 IS reachable: the `Pubkeys` TLV can bind one key to two slots
/// whose use sites differ, and `tests/fixtures/n1/r-n1d-delta.txt` is a
/// minted card that does exactly that.
pub fn check_descriptor(d: &Descriptor, disposition: Disposition) -> Result<(), CliError> {
    check(&card_occurrences(d), &card_key_bindings(d), disposition)
}

/// Rebuild the per-occurrence triples from a decoded card.
///
/// The triple is built ONCE PER SLOT and then repeated for each position the
/// slot occupies in the tree, which is the structural form of the paragraph
/// above: two occurrences of one slot cannot differ, because there is only
/// one declaration to differ from.
///
/// `origin_path` is left `None` for every occurrence rather than decoded out
/// of `path_decl`. Its ONLY role in the classifier is `same_triple`, which
/// compares occurrences of the SAME slot — where a constant is exactly
/// right — and no message on this path quotes it. Decoding it would mean
/// running the strict per-`@N` expansion, which fails closed on a partial
/// card; the reading verbs must still read one (Acceptance 5), so the
/// classifier must not be the thing that stops them.
///
/// A slot with ZERO tree positions still contributes ONE occurrence: the
/// card DECLARES its use site whether or not the tree references it, and
/// Family 2's lookup needs a triple for every slot it holds a key for.
/// A floor of one can never manufacture a Family-1 finding, which needs two.
fn card_occurrences(d: &Descriptor) -> Vec<PlaceholderOccurrence> {
    let n = d.n as usize;
    let mut counts = vec![0u32; n];
    count_occurrences(&d.tree, &mut counts);
    let overrides = d.tlv.use_site_path_overrides.as_deref().unwrap_or_default();

    let mut out = Vec::new();
    for (i, count) in counts.iter().enumerate() {
        let usp = overrides
            .iter()
            .find(|(j, _)| usize::from(*j) == i)
            .map_or(&d.use_site_path, |(_, p)| p);
        // `Alternative` carries a hardened flag the template lexer can never
        // produce (a hardened multipath alt is un-derivable on a watch-only
        // card and is refused at lex), so flattening to the value here can
        // only ever make two use sites look MORE alike -- i.e. it can only
        // under-report Family 2, never invent it.
        let occ = PlaceholderOccurrence {
            i: i as u8,
            origin_path: None,
            multipath_alts: usp
                .multipath
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|a| a.value)
                .collect(),
            wildcard_hardened: usp.wildcard_hardened,
        };
        for _ in 0..(*count).max(1) {
            out.push(occ.clone());
        }
    }
    out
}

/// Rebuild the per-`@i` key bindings from the card's `Pubkeys` TLV.
///
/// `depth` is `0` because the wire carries no depth field — a keyed card's
/// `Pubkeys` entry is 65 bytes, chain code ‖ compressed point, and nothing
/// else. That is safe here and only here: `classify` reads `i` and
/// `payload`, these values never leave this module, and the advisories that
/// DO read `depth` (F-411's `emit_unhardened_origin_note`) are on the
/// template path and build their own `ParsedKey`s from a real xpub.
fn card_key_bindings(d: &Descriptor) -> Vec<ParsedKey> {
    d.tlv
        .pubkeys
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|(i, payload)| ParsedKey {
            i: *i,
            depth: 0,
            payload: *payload,
        })
        .collect()
}

/// Count each placeholder's OCCURRENCES in the tree.
///
/// `Body::MultiKeys` holds raw indices rather than child `Node`s, and
/// `Body::Tr` holds the internal key as a bare index, so a walker that only
/// recursed through `Body::Children` would miss every position that matters
/// here.
///
/// Moved here from `seat::satisfy` by plan P3 step 3b, when the seating
/// door check became an invocation of this module: it is now one walker
/// serving one predicate rather than a second copy beside it.
fn count_occurrences(node: &Node, counts: &mut [u32]) {
    match &node.body {
        Body::KeyArg { index } => {
            if (*index as usize) < counts.len() {
                counts[*index as usize] += 1;
            }
        }
        Body::MultiKeys { indices, .. } => {
            for idx in indices {
                if (*idx as usize) < counts.len() {
                    counts[*idx as usize] += 1;
                }
            }
        }
        Body::Tr {
            is_nums,
            key_index,
            tree,
        } => {
            if !*is_nums && (*key_index as usize) < counts.len() {
                counts[*key_index as usize] += 1;
            }
            if let Some(t) = tree {
                count_occurrences(t, counts);
            }
        }
        Body::Children(children) => {
            for c in children {
                count_occurrences(c, counts);
            }
        }
        Body::Variable { children, .. } => {
            for c in children {
                count_occurrences(c, counts);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::template::lex_placeholders;

    fn key(i: u8, seed: u8) -> ParsedKey {
        ParsedKey {
            i,
            depth: 4,
            payload: [seed; 65],
        }
    }

    fn find(template: &str, keys: &[ParsedKey]) -> Option<Finding> {
        classify(&lex_placeholders(template).expect("template lexes"), keys)
    }

    #[test]
    fn identical_triples_are_r_n1a() {
        assert!(matches!(
            find("wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))", &[]),
            Some(Finding::SamePathExpression { i: 0, sites: 2 })
        ));
    }

    #[test]
    fn the_site_count_is_the_real_count_not_two() {
        assert!(matches!(
            find(
                "wsh(thresh(2,pk(@0/<0;1>/*),s:pk(@0/<0;1>/*),s:pk(@0/<0;1>/*)))",
                &[]
            ),
            Some(Finding::SamePathExpression { i: 0, sites: 3 })
        ));
    }

    #[test]
    fn overlapping_and_disjoint_multipath_sets_are_different_findings() {
        assert!(matches!(
            find("wsh(multi(2,@0/<0;1>/*,@0/<1;2>/*))", &[]),
            Some(Finding::MultipathOverlap { .. })
        ));
        assert!(matches!(
            find("wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))", &[]),
            Some(Finding::MultipathDisjoint { .. })
        ));
    }

    #[test]
    fn the_origin_axis_wins_over_the_multipath_axis() {
        // Both differ. The origin row is not qualified "only", the multipath
        // rows are, so origin must be what is reported.
        assert!(matches!(
            find(
                "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@0/48'/0'/1'/2'/<2;3>/*))",
                &[]
            ),
            Some(Finding::OriginDiffers { .. })
        ));
    }

    #[test]
    fn the_hardening_axis_is_its_own_finding() {
        assert!(matches!(
            find("wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*'))", &[]),
            Some(Finding::HardeningDiffers { .. })
        ));
    }

    #[test]
    fn family_1_fires_before_family_2() {
        // A template carrying BOTH defects must report the key-blind one, so
        // the keyed and keyless spellings of one template agree.
        let f = find(
            "wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*,@1/<2;3>/*))",
            &[key(0, 7), key(1, 7)],
        );
        assert!(matches!(f, Some(Finding::SamePathExpression { .. })));
    }

    #[test]
    fn one_key_at_two_disjoint_use_sites_is_r_n1d() {
        assert!(matches!(
            find(
                "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",
                &[key(0, 7), key(1, 7)]
            ),
            Some(Finding::KeyAtDisjointUseSites { a: 0, b: 1, .. })
        ));
    }

    /// THE BOUNDARY WITH THE SHIPPED FLOOR. Same key, SAME use site is F-218's
    /// case and belongs to `validate_no_duplicate_key_slots`; if this module
    /// claimed it, the operator would get a message written for a different
    /// wallet and the floor's own rows would go red for the wrong reason.
    #[test]
    fn one_key_at_the_same_use_site_is_not_this_modules_case() {
        assert_eq!(
            find(
                "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
                &[key(0, 7), key(1, 7)]
            ),
            None
        );
    }

    #[test]
    fn distinct_keys_and_distinct_placeholders_are_clean() {
        assert_eq!(
            find(
                "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",
                &[key(0, 7), key(1, 9)]
            ),
            None
        );
        assert_eq!(
            find(
                "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))",
                &[key(0, 7), key(1, 9)]
            ),
            None
        );
    }

    /// No diagnostic in this taxonomy may call the operator's wallet invalid,
    /// and the two that hand out an escape must actually name a runnable one.
    #[test]
    fn every_message_obeys_the_principle() {
        let all = [
            Finding::SamePathExpression { i: 0, sites: 2 },
            Finding::MultipathOverlap {
                i: 0,
                a: "<0;1>".into(),
                b: "<1;2>".into(),
                shared: "1".into(),
            },
            Finding::MultipathDisjoint {
                i: 0,
                a: "<0;1>".into(),
                b: "<2;3>".into(),
            },
            Finding::OriginDiffers {
                i: 0,
                a: "/48'/0'/0'/2'".into(),
                b: "/48'/0'/1'/2'".into(),
            },
            Finding::HardeningDiffers {
                i: 0,
                a: "<0;1>/*".into(),
                b: "<0;1>/*'".into(),
            },
            Finding::KeyAtDisjointUseSites {
                a: 0,
                b: 1,
                sa: "<0;1>/*".into(),
                sb: "<2;3>/*".into(),
            },
        ];
        // Checked under BOTH dispositions: M4 made one finding's tail vary by
        // disposition, and the principle binds the WARN rendering too, not
        // only the one this suite happens to construct with Refuse.
        for f in &all {
            for disp in [Disposition::Refuse, Disposition::Warn] {
                let m = f.message(disp);
                assert!(
                    !m.to_lowercase().contains("invalid"),
                    "{f:?} ({disp:?}) calls the wallet invalid: {m}"
                );
                assert!(
                    !m.contains('\n'),
                    "{f:?} ({disp:?}) renders on more than one line"
                );
            }
        }
        // The two md-side axes cite no BIP rule at all — an inline origin and
        // a hardened wildcard are md's own limits, and a citation there would
        // be a false record about a normative document.
        for f in [&all[3], &all[4]] {
            let m = f.message(Disposition::Refuse);
            assert!(!m.contains("BIP"), "{f:?} cites a BIP rule: {m}");
        }
        for f in [&all[2], &all[5]] {
            assert!(
                f.message(Disposition::Refuse).contains(ESCAPE),
                "{f:?} names no escape"
            );
        }
    }

    #[test]
    fn the_disposition_changes_the_outcome_not_the_text_for_most_findings() {
        // R-N1a (`SamePathExpression`) is the one exception, covered by
        // `r_n1a_warn_tail_differs_from_refuse_tail_per_m4` below — this row
        // uses R-N1b instead, where the "same text either way" invariant
        // still holds.
        let occs = lex_placeholders("wsh(multi(2,@0/<0;1>/*,@0/<1;2>/*))").unwrap();
        let err = check(&occs, &[], Disposition::Refuse).unwrap_err();
        assert!(matches!(err, CliError::Unsupported(_)));
        let body = classify(&occs, &[]).unwrap().message(Disposition::Refuse);
        assert_eq!(err.to_string(), format!("unsupported: {body}"));
        // Warn proceeds. (Its stderr line is pinned end-to-end by
        // `tests/n1_admission_taxonomy.rs`; here the contract is that it does
        // not stop the caller.)
        assert!(check(&occs, &[], Disposition::Warn).is_ok());
    }

    /// M4 — the R-N1a tail is disposition-aware, and this is the row that
    /// pins it structurally rather than by reading the source.
    #[test]
    fn r_n1a_warn_tail_differs_from_refuse_tail_per_m4() {
        let occs = lex_placeholders("wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))").unwrap();
        let finding = classify(&occs, &[]).unwrap();
        let refuse = finding.message(Disposition::Refuse);
        let warn = finding.message(Disposition::Warn);
        assert_ne!(
            refuse, warn,
            "the warn tail must differ from the refuse tail"
        );
        assert!(
            refuse.ends_with(
                "md declines to mint or compose this shape: give each distinct key its own \
                 placeholder."
            ),
            "{refuse}"
        );
        assert!(
            warn.ends_with(
                "This shape can no longer be minted or composed; the card remains readable."
            ),
            "{warn}"
        );
        assert!(!warn.contains("md declines to mint or compose"), "{warn}");
    }
}
