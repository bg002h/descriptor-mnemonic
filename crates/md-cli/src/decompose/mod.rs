//! `md decompose` — the D row: a concrete descriptor becomes an ENTRANCE.
//!
//! SPEC `design/SPEC_wallet_form_converter.md`, "P3 — the concrete descriptor
//! becomes an entrance"; plan §3 C3. Parsing is UNGATED: only `compile` needs
//! `miniscript/compiler`, and `miniscript` itself is an unconditional
//! dependency (SPEC P3, r1 I4).
//!
//! What comes out:
//!
//! * the KEYLESS BIP-388 template, in md's admitted surface and `'` spelling;
//! * one origin-notated key line per slot — a valid `mk encode --keys` file;
//! * the per-slot `--fingerprint` flags;
//! * the canonicalised concrete descriptor, checksum recomputed;
//! * `--emit commands`, the two mint routes as runnable command lines.
//!
//! Every refusal below is named in SPEC P3 and pinned by a roster row
//! (`tests/cmd_decompose.rs`, `tests/cmd_decompose_roundtrip.rs`). The
//! diagnostics say "forbidden by BIP 388" and "UNSUPPORTED", NEVER "invalid":
//! a repeated-key descriptor is well-formed script and this is a POLICY
//! refusal of a BIP-forbidden shape (operator ruling 2026-08-30).

pub mod walk;

use crate::error::CliError;
use bitcoin::Network;
use bitcoin::bip32::ChildNumber;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey};
use std::collections::BTreeMap;
use std::str::FromStr;
use walk::Occurrence;

/// Everything a decomposition can emit, computed once.
#[derive(Debug)]
pub struct Decomposition {
    /// The keyless BIP-388 template. No checksum — see `walk::build_template`.
    pub template: String,
    /// The canonicalised concrete descriptor with a RECOMPUTED checksum
    /// (SPEC "Canonicalisation").
    pub descriptor: String,
    /// Slots in template order; `occurrences[i]` is `@i`.
    pub occurrences: Vec<Occurrence>,
    /// Advisories for stderr. stdout stays the machine contract.
    pub notes: Vec<String>,
}

impl Decomposition {
    /// The slots with no origin in the input — excluded from the mk-mintable
    /// set (SPEC P3 "Origin-less keys").
    pub fn origin_less(&self) -> Vec<usize> {
        self.occurrences
            .iter()
            .enumerate()
            .filter(|(_, o)| o.origin.is_none())
            .map(|(i, _)| i)
            .collect()
    }

    /// `--fingerprint @i=hex` for every slot that states one.
    pub fn fingerprint_flags(&self) -> Vec<String> {
        self.occurrences
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.fingerprint().map(|f| format!("--fingerprint @{i}={f}")))
            .collect()
    }
}

// ─── input boundary (SPEC P3 "Input boundary", r1 I6) ───────────────────────

/// Does this input look like JSON rather than a descriptor?
///
/// A descriptor's first character is always a fragment or wrapper name — never
/// `{` or `[`. (A KEY expression may begin with `[`, but a whole descriptor
/// never is one: `Descriptor::Bare` still wraps a miniscript.) So the test is
/// exact rather than heuristic, which is what keeps this refusal off the
/// checksum path.
fn looks_like_json(s: &str) -> bool {
    matches!(s.trim().chars().next(), Some('{') | Some('['))
}

fn json_refusal() -> CliError {
    CliError::Decompose(
        "this input is JSON, not a descriptor. If it came from Bitcoin Core, \
         `listdescriptors` wraps each descriptor in an object — extract the string in \
         the \"desc\" field of the entry you want and pass THAT:\n    \
         md decompose 'wsh(sortedmulti(2,[…]xpub…/<0;1>/*,…))'\n\
         Core also emits receive and change as separate entries; combine them into one \
         multipath descriptor (`/<0;1>/*` in place of `/0/*` and `/1/*`) first. \
         Extraction is deliberately left to you or a front-end — md does not parse \
         wallet-manager JSON."
            .into(),
    )
}

fn pair_refusal(n: usize) -> CliError {
    CliError::Decompose(format!(
        "decompose takes ONE descriptor and {n} were supplied. Bitcoin Core and several \
         coordinators export a wallet as SEPARATE receive and change descriptors; if that \
         is what these are, combine them into ONE multipath descriptor before decomposing \
         — write `/<0;1>/*` where the receive descriptor has `/0/*` and the change \
         descriptor has `/1/*` — and pass that single string. md will not guess which two \
         descriptors belong to one wallet, because guessing wrong seats a change key on a \
         receive slot."
    ))
}

/// Resolve the raw input channel to exactly one descriptor string.
///
/// The JSON test runs FIRST so a multi-line JSON blob is not reported as a
/// descriptor PAIR, and neither is reported as a checksum problem — the
/// F-420 class SPEC P3 names.
pub fn resolve_input(raw: &[String]) -> Result<String, CliError> {
    if raw.iter().any(|s| looks_like_json(s)) {
        return Err(json_refusal());
    }
    match raw.len() {
        0 => Err(CliError::BadArg(
            "decompose: DESCRIPTOR required (on argv or via --in FILE)".into(),
        )),
        1 => Ok(raw[0].trim().to_string()),
        n => Err(pair_refusal(n)),
    }
}

/// Shared line-processing for decompose's descriptor input: blank lines and
/// `#` comments are skipped; every remaining line is a descriptor. Used by
/// BOTH `--in FILE` and `-` (R7 — stdin), so a file holding a receive/change
/// PAIR and a PIPED receive/change PAIR draw the SAME pair guidance.
fn parse_descriptor_lines(buf: &str) -> Vec<String> {
    buf.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Read `--in FILE`: decompose's OWN input material is a descriptor (P3 §6b).
/// Blank lines and `#` comments are skipped; every remaining line is a
/// descriptor, so a file holding a receive/change PAIR reaches `resolve_input`
/// as two entries and draws the pair guidance.
pub fn read_descriptor_file(path: &std::path::Path) -> Result<Vec<String>, CliError> {
    let buf = std::fs::read_to_string(path)
        .map_err(|e| CliError::BadArg(format!("--in {}: {e}", path.display())))?;
    if looks_like_json(&buf) {
        return Err(json_refusal());
    }
    let out = parse_descriptor_lines(&buf);
    if out.is_empty() {
        return Err(CliError::BadArg(format!(
            "--in {}: no descriptor in this file (it is empty, blank, or all comments). \
             An EMPTY file is what a FAILED upstream command leaves behind — check the \
             command that wrote it.",
            path.display()
        )));
    }
    Ok(out)
}

/// **R7 — `-` on the positional, `≡ --in /dev/stdin`.** Same per-line
/// convention as `--in FILE` (blank/`#` skipped, receive/change PAIR draws
/// the same guidance) — sourced from stdin DIRECTLY rather than by opening a
/// literal `/dev/stdin` path, which the windows-latest CI leg (`ci.yml`'s
/// three-OS test matrix) does not have. Closes
/// `md-decompose-does-not-read-stdin`.
pub fn read_descriptor_stdin() -> Result<Vec<String>, CliError> {
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
        .map_err(|e| CliError::BadArg(format!("stdin read: {e}")))?;
    if looks_like_json(&buf) {
        return Err(json_refusal());
    }
    let out = parse_descriptor_lines(&buf);
    if out.is_empty() {
        return Err(CliError::BadArg(
            "-: no descriptor on stdin (it is empty, blank, or all comments). An EMPTY \
             stream is what a FAILED upstream command leaves behind — check the command \
             that wrote it."
                .into(),
        ));
    }
    Ok(out)
}

// ─── parsing (SPEC P3: rust-miniscript, UNGATED) ────────────────────────────

/// Parse the descriptor, turning a checksum mismatch into a NAMED refusal.
///
/// SPEC P3 requires "a bare descriptor accepts with or without checksum" and
/// that a wrong one draws something better than the bare parser error. The
/// expected value is COMPUTED here with `miniscript`'s public checksum engine
/// rather than scraped out of the error text, so the message cannot drift with
/// upstream's wording.
///
/// **R6 — the checksum is verified against `s` AS WRITTEN, before anything
/// touches the text.** The `/**` desugar below changes the byte content a
/// BIP-380 checksum covers (`/**` and `/<0;1>/*` checksum differently — the
/// pathological wallet's two metadata forms already measure this), so
/// desugaring must run strictly AFTER the check above and the checksum-free
/// `body` is what gets desugared and parsed — never handing rust-miniscript
/// a checksum that was verified against text it will not see.
fn parse_descriptor(s: &str) -> Result<Descriptor<DescriptorPublicKey>, CliError> {
    let body: &str = if let Some((body, supplied)) = s.rsplit_once('#') {
        let mut eng = miniscript::descriptor::checksum::Engine::new();
        let expected = match eng.input(body) {
            Ok(()) => eng.checksum(),
            // A character outside BIP-380's INPUT_CHARSET: let the parser
            // below name it, since it is not a checksum problem.
            Err(_) => String::new(),
        };
        if !expected.is_empty() && supplied != expected {
            return Err(CliError::Decompose(format!(
                "the descriptor's BIP-380 checksum does not match its text: the string ends \
                 `#{supplied}`, but this descriptor's checksum is `{expected}`. Something \
                 altered one or the other in transit — re-copy the descriptor from the wallet \
                 that produced it, or drop the `#…` suffix entirely and pass the bare \
                 descriptor: md accepts it and recomputes the checksum itself. (md is NOT \
                 telling you the descriptor is malformed; only that the two disagree.)"
            )));
        }
        body
    } else {
        s
    };
    // R6 — BIP-388's `/**` shorthand, desugared to `/<0;1>/*` so decompose
    // reads it identically to the explicit spelling
    // (`design/SPEC_mdcli_mini.md` R6; closes
    // `md-decompose-rejects-double-wildcard-input`).
    let desugared = crate::parse::template::desugar_double_wildcard_descriptor(body);
    Descriptor::<DescriptorPublicKey>::from_str(&desugared).map_err(|e| {
        CliError::Decompose(format!(
            "this is not a descriptor md can parse: {e}. decompose takes ONE concrete output \
             descriptor — real xpubs, with or without a `#checksum`, multipath (`<0;1>`) or \
             fixed-path — on the positional, via --in FILE, or piped in with `-`. A BIP-388 \
             TEMPLATE (with `@0`, `@1`, …) goes to `md encode`, not here."
        ))
    })
}

// ─── refusals over the collected keys ───────────────────────────────────────

/// **Key reuse — SPEC A3's two BIP-388 shapes, on the DECOMPOSE side.**
///
/// BIP 388's "Additional rules": the pairwise-distinctness rule (the public
/// keys obtained by deserializing elements of the key information vector
/// must be pairwise distinct) and the disjointness rule (two KEY expressions
/// on the SAME placeholder must have DISJOINT multipath sets). BIP 388 does
/// not number these paragraphs itself, so neither does this comment (review
/// r1 N1).
///
/// In a CONCRETE descriptor those separate cleanly. Two occurrences of one
/// extended key under DIFFERENT key expressions would become two entries of
/// the key information vector deserializing to the same key — the
/// pairwise-distinctness rule. Two occurrences of the SAME key expression
/// are one placeholder used twice — the disjointness rule, which is
/// satisfied only when the multipath sets are disjoint.
///
/// The third case is real and has no BIP violation in it: the same expression
/// twice with DISJOINT sets, which BIP 388 permits and md's own template
/// surface does not — `md descriptor`/`md encode` refuse `@0` at two use
/// sites outright, R-N1c
/// (`crate::parse::reuse::Finding::MultipathDisjoint`, review r1 I5
/// corrected the citation here after the classifier moved upstream of
/// `resolve_placeholders`, where the message this comment used to quote
/// lived). Emitting that template would hand the operator something md
/// itself refuses to ingest, so decompose refuses it too — naming md as the
/// limit, NOT the BIP, because there is no BIP violation to name.
///
/// Grouping is by the whole extended key serialisation. `sanity_check` does
/// NOT cover the pairwise-distinctness rule here: measured 2026-08-30, the same xpub under two
/// different origins returns `Ok(())` because the two `DescriptorPublicKey`
/// values differ. The check is md's own.
fn check_no_repeated_key(occ: &[Occurrence]) -> Result<(), CliError> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (n, o) in occ.iter().enumerate() {
        groups.entry(o.xpub.to_string()).or_default().push(n);
    }
    for (xpub, idxs) in &groups {
        if idxs.len() < 2 {
            continue;
        }
        let short = format!("{}…{}", &xpub[..12], &xpub[xpub.len() - 8..]);
        let records: Vec<&str> = idxs.iter().map(|i| occ[*i].record.as_str()).collect();
        let distinct: std::collections::BTreeSet<&str> = records.iter().copied().collect();

        if distinct.len() > 1 {
            // The pairwise-distinctness rule — the key information vector's
            // elements must be pairwise distinct.
            let where_ = idxs
                .iter()
                .map(|i| format!("  {} at {}", occ[*i].label(), occ[*i].use_site))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(CliError::Decompose(format!(
                "the same extended key is used at {} positions — {short}:\n{where_}\n\
                 BIP 388 requires that \"the public keys obtained by deserializing elements of \
                 the key information vector must be pairwise distinct\", and adds: \"Reusing \
                 pubkeys could be insecure in the context of wallet policies containing \
                 miniscript. Avoiding repeated public keys altogether avoids the problem at \
                 the source.\" This wallet is well-formed script and it is forbidden by BIP \
                 388 — UNSUPPORTED here, never invalid. Re-derive one distinct key per \
                 position and rebuild the descriptor.",
                idxs.len()
            )));
        }

        // One key expression, several occurrences: the disjointness rule.
        for (a, i) in idxs.iter().enumerate() {
            for j in idxs.iter().skip(a + 1) {
                let overlap: Vec<String> = occ[*i]
                    .paths
                    .intersection(&occ[*j].paths)
                    .cloned()
                    .collect();
                if !overlap.is_empty() {
                    return Err(CliError::Decompose(format!(
                        "the key expression `{}` appears at {} positions whose multipath sets are \
                         NOT DISJOINT — `{}` and `{}` share the derivation path(s) {}. BIP 388 \
                         requires two KEY expressions on the same placeholder to have disjoint \
                         multipath sets, and lists `sh(multi(1,@0/**,@0/**))` (\"Repeated keys \
                         with the same path expression\") among its invalid examples. Forbidden \
                         by BIP 388 — UNSUPPORTED here, never invalid.",
                        occ[*i].record,
                        idxs.len(),
                        occ[*i].use_site,
                        occ[*j].use_site,
                        overlap.join(", ")
                    )));
                }
            }
        }

        // Disjoint — BIP-legal, md-unsupported.
        let sets = idxs
            .iter()
            .map(|i| occ[*i].use_site.clone())
            .collect::<Vec<_>>()
            .join(" and ");
        return Err(CliError::Decompose(format!(
            "the key expression `{}` appears at {} positions with DISJOINT multipath sets \
             ({sets}). BIP 388 permits that shape — this is not a BIP violation — but md's \
             template surface is narrower: it refuses one placeholder at two use sites \
             outright (R-N1c), so decompose would be handing you a template md itself \
             cannot ingest. UNSUPPORTED here. Keep this wallet as a descriptor instead: \
             {escape}",
            occ[idxs[0]].record,
            idxs.len(),
            escape = crate::parse::reuse::ESCAPE,
        )));
    }
    Ok(())
}

/// **Depth consistency IN THE INPUT — SPEC P3 "Key emission is
/// round-trip-grade" (r1 C3).**
///
/// `mk`'s compact-73 key encoding drops `depth` and `child_number` from the
/// wire and RECONSTRUCTS both from the origin path, so an inconsistent record
/// would decode to a different-metadata xpub. `mk encode` therefore refuses it
/// at mint — measured 2026-08-30:
///
/// ```text
/// error: --keys record 1 ([73c5da0a/48'/0'/0']): xpub origin-path mismatch:
///   xpub depth 4 / child 2' vs origin_path depth 3 / last Some(Hardened { index: 0 })
/// ```
///
/// decompose refuses first, with mk's constraint named, rather than emitting a
/// key file that cannot be minted.
fn check_depth_consistency(occ: &[Occurrence]) -> Result<(), CliError> {
    for (i, o) in occ.iter().enumerate() {
        let Some((fp, path)) = &o.origin else {
            // No origin stated: nothing to be inconsistent WITH. Such a key is
            // excluded from the mintable set instead (SPEC P3 "Origin-less
            // keys"), which `--emit commands` refuses over.
            continue;
        };
        let comps: &[ChildNumber] = path.as_ref();
        let path_depth = comps.len();
        let xpub_depth = usize::from(o.xpub.depth);
        if xpub_depth != path_depth {
            return Err(CliError::Decompose(format!(
                "key @{i} is depth-inconsistent IN THE INPUT: the extended key states depth \
                 {xpub_depth}, but its origin path `[{fp}/{path}]` has {path_depth} component(s) \
                 — depth {path_depth}. An mk1 key card drops depth and child number from the \
                 wire and reconstructs BOTH from the origin path, so `mk encode --keys` refuses \
                 such a record outright (\"xpub origin-path mismatch: xpub depth {xpub_depth} \
                 … vs origin_path depth {path_depth} …\"). decompose will not emit a key line \
                 that could not be minted: correct the origin in the descriptor so it states \
                 the path this key was actually derived at."
            )));
        }
        if let Some(last) = comps.last() {
            if o.xpub.child_number != *last {
                return Err(CliError::Decompose(format!(
                    "key @{i} is depth-inconsistent IN THE INPUT: the extended key's child \
                     number is {}, but its origin path `[{fp}/{path}]` ends at {last}. An mk1 \
                     key card reconstructs the child number from the origin path, so \
                     `mk encode --keys` refuses such a record (\"xpub origin-path mismatch: \
                     xpub depth {} / child {} vs origin_path depth {path_depth} / last {last}\"). \
                     decompose will not emit a key line that could not be minted: correct the \
                     origin in the descriptor so it states the path this key was actually \
                     derived at.",
                    o.xpub.child_number, o.xpub.depth, o.xpub.child_number
                )));
            }
        }
    }
    Ok(())
}

/// Every key must belong to the network the caller declared. Without this,
/// decompose would happily print `md encode --key` commands md then refuses
/// ("expected mainnet xpub version …"), with nothing naming the reason.
fn check_network(occ: &[Occurrence], network: Network) -> Result<(), CliError> {
    let want = bitcoin::NetworkKind::from(network);
    for (i, o) in occ.iter().enumerate() {
        if o.xpub.network != want {
            let (have, flag) = match o.xpub.network {
                bitcoin::NetworkKind::Main => ("mainnet", "--network mainnet"),
                bitcoin::NetworkKind::Test => ("testnet", "--network testnet"),
            };
            return Err(CliError::Decompose(format!(
                "key @{i} is a {have} extended key, but --network says {}. The emitted \
                 `md encode --key` commands would be refused by md's own version-byte check, \
                 so decompose stops here instead. Re-run with `{flag}`.",
                match want {
                    bitcoin::NetworkKind::Main => "mainnet",
                    bitcoin::NetworkKind::Test => "testnet",
                }
            )));
        }
    }
    Ok(())
}

// ─── the entry point ────────────────────────────────────────────────────────

/// Decompose one concrete descriptor. `raw` is the input channel's contents:
/// one entry per descriptor supplied, so a receive/change PAIR arrives as two.
pub fn decompose(raw: &[String], network: Network) -> Result<Decomposition, CliError> {
    let input = resolve_input(raw)?;
    let desc = parse_descriptor(&input)?;

    let mut occurrences = walk::collect_occurrences(&desc)?;
    // Refuse repeats BEFORE numbering: `order_by_appearance` locates each key
    // expression by its rendering, which is unambiguous only once every
    // expression is unique.
    check_no_repeated_key(&occurrences)?;

    let descriptor = desc.to_string();
    walk::order_by_appearance(&mut occurrences, &format!("{desc:#}"));

    check_network(&occurrences, network)?;
    check_depth_consistency(&occurrences)?;

    let template = walk::build_template(&desc, &occurrences)?;

    let mut notes = Vec::new();
    let origin_less = occurrences
        .iter()
        .enumerate()
        .filter(|(_, o)| o.origin.is_none())
        .map(|(i, o)| format!("@{i} ({}…)", &o.record[..16.min(o.record.len())]))
        .collect::<Vec<_>>();
    if !origin_less.is_empty() {
        notes.push(format!(
            "{} key(s) state NO origin in this descriptor — {}. They are emitted as bare key \
             lines and are EXCLUDED from the mk-mintable set: an mk1 card binds a key to the \
             origin it was derived at by design, and `mk encode --keys` refuses a record that \
             is not `[fingerprint/path]xpub`. The template and descriptor emissions are \
             unaffected; `--emit commands` refuses.",
            origin_less.len(),
            origin_less.join(", ")
        ));
    }
    // md's template surface is narrower than BIP 388's. Say so HERE, where the
    // operator can still act, rather than letting them discover it when
    // `md encode` refuses the template decompose just printed.
    if let Err(e) = crate::parse::template::parse_template(&template, &[], &[]) {
        notes.push(format!(
            "`md encode` may not accept this template as printed: {e}. The decomposition itself \
             is sound — md's own template surface is narrower than BIP 388's."
        ));
    }

    Ok(Decomposition {
        template,
        descriptor,
        occurrences,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const K0: &str = "[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
    const K1: &str = "[73c5da0a/48'/0'/1'/2']xpub6DzhyrnFFYQ1HimDiM388xHnDiRPNdZJFBmmxge3Y1WWcHLtMJLfRuhRHqnQCPbTj3fGKTuKFLHzzwpJkp5Dtc3UtLKZKaVZe1yqMBXd6Vk";
    /// Abandon-mnemonic tpub at m/84'/1'/0' — depth 3, child 0', so the origin
    /// below is depth-CONSISTENT and the network check is the only thing under
    /// test. Same constant as `parse::keys`'s `ABANDON_TPUB_DEPTH3_BIP84`.
    const TPUB: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M";

    fn d2() -> String {
        format!("wsh(sortedmulti(2,{K0}/<0;1>/*,{K1}/<0;1>/*))")
    }

    #[test]
    fn json_beats_pair_and_checksum() {
        // A multi-line JSON blob reaches `resolve_input` as several entries;
        // without the JSON test running first it would draw the PAIR message.
        let raw = vec!["{".to_string(), "\"desc\": \"wsh(...)\"".to_string()];
        let msg = resolve_input(&raw).unwrap_err().to_string();
        assert!(msg.contains("listdescriptors"), "{msg}");
        assert!(!msg.contains("checksum"), "{msg}");
    }

    #[test]
    fn a_key_expression_alone_is_not_mistaken_for_json() {
        // A KEY expression starts with `[`, but a DESCRIPTOR never is one.
        // This pins the reasoning in `looks_like_json`'s doc comment.
        assert!(looks_like_json("[{\"desc\":\"x\"}]"));
        assert!(!looks_like_json(&d2()));
        assert!(!looks_like_json("tr(xpub.../<0;1>/*)"));
    }

    #[test]
    fn network_mismatch_refuses_naming_the_flag() {
        let d = format!("wpkh([73c5da0a/84'/1'/0']{TPUB}/<0;1>/*)");
        let err = decompose(&[d], Network::Bitcoin).unwrap_err().to_string();
        assert!(err.contains("testnet"), "{err}");
        assert!(err.contains("--network testnet"), "{err}");
    }

    #[test]
    fn testnet_descriptor_decomposes_under_the_matching_network() {
        // The negative half of the check above.
        let d = format!("wpkh([73c5da0a/84'/1'/0']{TPUB}/<0;1>/*)");
        let out = decompose(&[d], Network::Testnet).unwrap();
        assert_eq!(out.template, "wpkh(@0/84'/1'/0'/<0;1>/*)");
    }

    #[test]
    fn fingerprint_flags_are_emitted_per_slot_that_states_one() {
        let out = decompose(&[d2()], Network::Bitcoin).unwrap();
        assert_eq!(
            out.fingerprint_flags(),
            vec![
                "--fingerprint @0=73c5da0a".to_string(),
                "--fingerprint @1=73c5da0a".to_string()
            ]
        );
        assert!(out.origin_less().is_empty());
    }

    #[test]
    fn origin_less_slots_are_listed_and_noted() {
        let bare = K0.split(']').nth(1).unwrap().to_string();
        let d = format!("wsh(sortedmulti(2,{bare}/<0;1>/*,{K1}/<0;1>/*))");
        let out = decompose(&[d], Network::Bitcoin).unwrap();
        assert_eq!(out.origin_less(), vec![0]);
        assert_eq!(out.fingerprint_flags(), vec!["--fingerprint @1=73c5da0a"]);
        assert!(
            out.notes.iter().any(|n| n.contains("EXCLUDED")),
            "{:?}",
            out.notes
        );
    }

    #[test]
    fn descriptor_emission_recomputes_the_checksum_from_an_h_spelled_input() {
        // SPEC "Canonicalisation": the spelling changes the checksum, so an
        // `h`-spelled input must NOT keep its own suffix.
        let h = d2().replace('\'', "h");
        let out = decompose(&[h], Network::Bitcoin).unwrap();
        assert_eq!(out.descriptor, format!("{}#{}", d2(), "tpdwkkds"));
    }
}
