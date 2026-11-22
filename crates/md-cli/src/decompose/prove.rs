//! The template check (2.0) and the mint-route proofs (2.2, 2.3) of
//! `design/FIX_bh1119_md_decompose_routes.md`.
//!
//! `md decompose --emit commands` prints a mint route only after running that
//! route's own encode in process, re-deriving the descriptor from what it
//! minted and comparing it with the input descriptor under normalisation N
//! (every extended key's parent fingerprint set to `00000000`; a card does not
//! carry it and md renders it as zero).

use super::Decomposition;
use super::walk::Occurrence;
use crate::cmd::encode::{descriptor_for_mint, mint_md1_cards};
use crate::error::CliError;
use crate::parse::template::{PlaceholderOccurrence, lex_placeholders};
use bitcoin::Network;
use bitcoin::bip32::{DerivationPath, Fingerprint};
use miniscript::Translator;
use miniscript::descriptor::{Descriptor as MsDescriptor, DescriptorPublicKey};
use std::convert::Infallible;
use std::str::FromStr;

// ─── 2.0 the template check ─────────────────────────────────────────────────

/// The origin path an occurrence states (none = empty).
fn occurrence_origin(o: &Occurrence) -> DerivationPath {
    let t = o.origin_path_text.trim_start_matches('/');
    if t.is_empty() {
        DerivationPath::master()
    } else {
        DerivationPath::from_str(t).unwrap_or_else(|_| DerivationPath::master())
    }
}

/// The read-back use-site as text: `/<a;b;…>` when it has multipath
/// alternatives, then `/*` or `/*h`. The lexer cannot tell a missing wildcard
/// from `/*`, so a read-back always renders a wildcard.
fn readback_use_site(p: &PlaceholderOccurrence) -> String {
    let mut s = String::new();
    if !p.multipath_alts.is_empty() {
        let alts: Vec<String> = p.multipath_alts.iter().map(u32::to_string).collect();
        s.push_str(&format!("/<{}>", alts.join(";")));
    }
    s.push_str(if p.wildcard_hardened { "/*h" } else { "/*" });
    s
}

fn refusal(i: usize, o: &Occurrence) -> String {
    let has = if o.use_site.is_empty() {
        "has no use-site".to_string()
    } else {
        format!("has use-site `{}`", o.use_site)
    };
    format!(
        "this descriptor has no md1 template: key @{i} {has}, which an md1 card cannot hold \
         (after the origin it holds only a wildcard, optionally after one multipath step such \
         as `/<0;1>/*`), so a template, and any card minted from it, would describe a \
         different wallet. If this is the receive (`/0/*`) or change (`/1/*`) half of a \
         wallet, give the descriptor with `/<0;1>/*` in its place. The emissions that need no \
         template still work: `--emit keys`, `--emit fingerprints`, `--emit descriptor`."
    )
}

/// 2.0: read the template back with md's own lexer and require every `@i`'s
/// origin path and use-site to equal occurrence i's. `Some(message)` is the
/// refusal text; `None` means the template says what the input says.
pub fn template_refusal(template: &str, occ: &[Occurrence]) -> Option<String> {
    if occ.is_empty() {
        return None;
    }
    match lex_placeholders(template) {
        Ok(lexed) => {
            for (i, o) in occ.iter().enumerate() {
                let Some(p) = lexed.iter().find(|p| usize::from(p.i) == i) else {
                    return Some(refusal(i, o));
                };
                let read_origin = p.origin_path.clone().unwrap_or_else(DerivationPath::master);
                if read_origin != occurrence_origin(o)
                    || readback_use_site(p) != o.use_site.replace('\'', "h")
                {
                    return Some(refusal(i, o));
                }
            }
            None
        }
        Err(_) => {
            for (i, o) in occ.iter().enumerate() {
                let alone = format!("@{i}{}{}", o.origin_path_text, o.use_site);
                if lex_placeholders(&alone).is_err() {
                    return Some(refusal(i, o));
                }
            }
            Some(refusal(0, &occ[0]))
        }
    }
}

// ─── normalisation N and the comparison ─────────────────────────────────────

struct ZeroParent;

impl Translator<DescriptorPublicKey> for ZeroParent {
    type TargetPk = DescriptorPublicKey;
    type Error = Infallible;

    fn pk(&mut self, pk: &DescriptorPublicKey) -> Result<DescriptorPublicKey, Infallible> {
        let mut pk = pk.clone();
        match &mut pk {
            DescriptorPublicKey::XPub(x) => x.xkey.parent_fingerprint = Fingerprint::default(),
            DescriptorPublicKey::MultiXPub(m) => {
                m.xkey.parent_fingerprint = Fingerprint::default();
            }
            DescriptorPublicKey::Single(_) => {}
        }
        Ok(pk)
    }
    fn sha256(
        &mut self,
        h: &bitcoin::hashes::sha256::Hash,
    ) -> Result<bitcoin::hashes::sha256::Hash, Infallible> {
        Ok(*h)
    }
    fn hash256(
        &mut self,
        h: &miniscript::hash256::Hash,
    ) -> Result<miniscript::hash256::Hash, Infallible> {
        Ok(*h)
    }
    fn ripemd160(
        &mut self,
        h: &bitcoin::hashes::ripemd160::Hash,
    ) -> Result<bitcoin::hashes::ripemd160::Hash, Infallible> {
        Ok(*h)
    }
    fn hash160(
        &mut self,
        h: &bitcoin::hashes::hash160::Hash,
    ) -> Result<bitcoin::hashes::hash160::Hash, Infallible> {
        Ok(*h)
    }
}

/// The Display of a descriptor with every key's parent fingerprint zeroed.
fn normalised(d: &MsDescriptor<DescriptorPublicKey>) -> Result<String, String> {
    d.translate_pk(&mut ZeroParent)
        .map(|n| n.to_string())
        .map_err(|_| "internal error: could not normalise the descriptor".to_string())
}

const DIFFERS: &str = "the wallet it mints is not this descriptor";

/// Compare the rendered wallet with the decompose input under N.
fn same_wallet(rendered: &MsDescriptor<DescriptorPublicKey>, input: &str) -> Result<(), String> {
    let input = MsDescriptor::<DescriptorPublicKey>::from_str(input)
        .map_err(|e| format!("internal error: the input descriptor did not re-parse: {e}"))?;
    if normalised(rendered)? == normalised(&input)? {
        Ok(())
    } else {
        Err(DIFFERS.to_string())
    }
}

/// Decode minted md1 strings (one string, or a chunk set).
fn decode_minted(cards: &[String]) -> Result<md_codec::encode::Descriptor, CliError> {
    if cards.len() == 1 {
        Ok(md_codec::decode::decode_md1_string(&cards[0])?)
    } else {
        let refs: Vec<&str> = cards.iter().map(String::as_str).collect();
        Ok(md_codec::chunk::reassemble(&refs)?)
    }
}

fn md_encode_refuses(e: impl std::fmt::Display) -> String {
    format!("md encode refuses it: {e}")
}

// ─── 2.2 route 1 ────────────────────────────────────────────────────────────

/// Prove route 1: with exactly the printed arguments, run the calls `md encode`
/// text mode makes, decode what was minted, render and compare.
pub fn prove_route1(
    d: &Decomposition,
    keys: &[String],
    fingerprints: &[String],
    experimental: bool,
    network: Network,
) -> Result<(), String> {
    let (descriptor, _, _) =
        descriptor_for_mint(&d.template, keys, fingerprints, experimental, network)
            .map_err(md_encode_refuses)?;
    md_codec::validate::validate_relative_timelocks(&descriptor.tree)
        .map_err(|e| md_encode_refuses(CliError::from(e)))?;
    let (cards, _) = mint_md1_cards(&descriptor, false).map_err(md_encode_refuses)?;
    let decoded = decode_minted(&cards).map_err(md_encode_refuses)?;
    let rendered = md_codec::to_miniscript_descriptor_multipath_with_network(&decoded, network)
        .map_err(|e| md_encode_refuses(CliError::from(e)))?;
    same_wallet(&rendered, &d.descriptor)
}

// ─── 2.3 route 2 ────────────────────────────────────────────────────────────

/// Prove route 2 (the split set), in process. The mirror of mk-cli's record
/// admission sits here next to `check_depth_consistency` (which mirrors mk's
/// depth rule): rule (a) is mk-cli `keyfile.rs` `parse_key_record` (a key card
/// must declare an origin path) and rule (b) is the provable-fingerprint loop
/// in mk-cli `cmd/encode.rs` (at depth 1 the xpub's parent is the master key),
/// both @434d789. Drift fails closed; it is tracked by FOLLOWUPS
/// `md-decompose-mk-admission-mirror` / `mk-record-admission-mirrored-in-md`.
pub fn prove_route2(
    d: &Decomposition,
    fingerprints: &[String],
    experimental: bool,
    network: Network,
) -> Result<(), String> {
    // 1. the md half: no keys, no --network (mainnet default; keyless).
    let (md_half, _, _) = descriptor_for_mint(
        &d.template,
        &[],
        fingerprints,
        experimental,
        Network::Bitcoin,
    )
    .map_err(md_encode_refuses)?;
    md_codec::validate::validate_relative_timelocks(&md_half.tree)
        .map_err(|e| md_encode_refuses(CliError::from(e)))?;
    let (md_cards, _) = mint_md1_cards(&md_half, false).map_err(md_encode_refuses)?;
    let md_decoded = decode_minted(&md_cards).map_err(md_encode_refuses)?;

    // 2. mk-cli's record admission, rule (a) for every record, then (b).
    for (n, o) in d.occurrences.iter().enumerate() {
        if let Some((fp, path)) = &o.origin {
            if path.is_empty() {
                return Err(format!(
                    "mk encode refuses it: keys.txt:{}: origin \"{fp}\" has no derivation path; \
                     a key card must declare one (e.g. `[{fp}/48'/0'/0'/2']`)",
                    n + 1
                ));
            }
        }
    }
    for o in &d.occurrences {
        if let Some((fp, path)) = &o.origin {
            if path.len() == 1 && o.xpub.parent_fingerprint != *fp {
                return Err(format!(
                    "mk encode refuses it: origin fingerprint {fp} does not match the xpub: at \
                     depth 1 the xpub's parent is the master key, so the fingerprint must be \
                     {}. The record pairs a key with an origin that is not its own.",
                    o.xpub.parent_fingerprint
                ));
            }
        }
    }

    // 3. one card per record, stubbed like `mk encode --from-md1-set`.
    let id = md_codec::compute_wallet_descriptor_template_id(&md_decoded)
        .map_err(|e| md_encode_refuses(CliError::from(e)))?;
    let b = id.as_bytes();
    let stub = [b[0], b[1], b[2], b[3]];
    let mut mk_strings: Vec<String> = Vec::new();
    for o in &d.occurrences {
        let Some((fp, path)) = &o.origin else {
            // Unreachable: origin-less keys refuse before any proof.
            return Err("mk encode refuses it: a record states no origin".to_string());
        };
        let card = mk_codec::KeyCard::new(vec![stub], Some(*fp), path.clone(), o.xpub);
        let strings = mk_codec::encode(&card).map_err(|e| format!("mk encode refuses it: {e}"))?;
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        mk_codec::decode(&refs).map_err(|e| format!("mk encode refuses it: {e}"))?;
        mk_strings.extend(strings);
    }

    // 4. seat them the way `md descriptor <policy.md1> --from-mk1 …` does.
    let md_refuses = |e: CliError| format!("md descriptor refuses it: {e}");
    let seating = crate::seat::run(&crate::seat::SeatingRequest {
        phrases: &md_cards,
        from_mk1: &mk_strings,
        seats: &[],
        network,
        cmd: "descriptor",
    })
    .map_err(md_refuses)?;
    let rendered =
        md_codec::to_miniscript_descriptor_multipath_with_network(&seating.descriptor, network)
            .map_err(|e| md_refuses(CliError::from(e)))?;
    same_wallet(&rendered, &d.descriptor)
}
