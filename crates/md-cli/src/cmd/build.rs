//! Building a [`Descriptor`] from either input shape, in ONE place.
//!
//! Two commands now need it — `md address` and `md descriptor` — and they must
//! agree exactly. The `--path` override in particular is load-bearing: a
//! non-canonical wrapper refuses with "non-canonical wrapper requires explicit
//! origin for @N", so a command that forgot to apply it would refuse the very
//! shapes the flag exists for (taproot with a script tree, and the miniscript
//! wrappers generally). Two copies is how one of them ends up without it.

use crate::error::CliError;
use crate::parse::keys::{
    ParsedFingerprint, ParsedKey, ScriptCtx, parse_fingerprint, parse_key_with_origin,
};
use crate::parse::path::apply_path_override_per_slot;
use crate::parse::template::{ctx_for_template, lex_placeholders, parse_template};
use bitcoin::bip32::DerivationPath;
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::encode::Descriptor;
use std::collections::{BTreeMap, BTreeSet};

/// The two ways a wallet policy reaches a command: as md1 phrases, or as a
/// template plus concrete keys.
pub struct DescriptorInput<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    /// Shared origin-path override, mirroring `md encode --path`.
    pub path: Option<&'a str>,
    pub network: bitcoin::Network,
    /// Names the calling subcommand, so a refusal reads in the operator's terms
    /// rather than in the shared helper's.
    pub cmd: &'static str,
}

pub fn build_descriptor(args: &DescriptorInput<'_>) -> Result<Descriptor, CliError> {
    // Defense in depth — clap's ArgGroup::required(true) is the primary guard;
    // this catches the case where it ever fails (clap regression, a custom
    // invocation bypassing the parser) and routes to a clean exit-2 BadArg
    // instead of a confusing exit-1 from reassemble(&[]).
    if args.phrases.is_empty() && args.template.is_none() {
        return Err(CliError::BadArg(format!(
            "{} requires either positional <STRING>... or --template <T> --key @i=<XPUB>; \
             clap should have caught this — please report a bug",
            args.cmd
        )));
    }
    if let Some(template) = args.template {
        if args.keys.is_empty() {
            return Err(CliError::BadArg(
                "--key @i=<XPUB> required when --template is supplied".into(),
            ));
        }
        let ctx = ctx_for_template(template);
        let (parsed_keys, parsed_fps, inline_declared) = resolve_keys_fingerprints_and_precedence(
            template,
            args.keys,
            args.fingerprints,
            ctx,
            args.network,
        )?;
        let mut descriptor = parse_template(template, &parsed_keys, &parsed_fps)?;
        apply_path_override_per_slot(&mut descriptor, args.path, &inline_declared)?;
        refuse_key_reuse_across_slots(&descriptor, args.cmd)?;
        return Ok(descriptor);
    }
    // Phrase path. mstring display-grouping (SPEC §3.2): strip separators so a
    // grouped or unbroken card both re-ingest.
    let phrases = crate::cmd::strip_md1_inputs(args.phrases);
    if phrases.len() == 1 {
        Ok(decode_md1_string(&phrases[0])?)
    } else {
        let refs: Vec<&str> = phrases.iter().map(String::as_str).collect();
        Ok(reassemble(&refs)?)
    }
}

/// SPEC P1's per-datum precedence (`design/SPEC_wallet_form_converter.md`
/// "The three pieces"), resolved BEFORE `parse_template` runs. Accepts the
/// origin-notated `--key '@i=[fp/path]xpub'` form (C0's `parse_key_with_origin`)
/// alongside the bare `@i=XPUB` form it already accepted (both go through the
/// same parser — a bare value just carries no bracket).
///
/// "the sources never overlap on the same datum, so precedence is
/// per-DATUM, not per-source":
///
/// - PATHS come from the inline template origin where present, else the
///   shared `--path` (applied afterward, per slot, by
///   `apply_path_override_per_slot`), else today's non-canonical-wrapper
///   refusal stands. An origin-notated `--key`'s bracket PATH is NEVER a
///   source of descriptor path data — it exists only to be checked against
///   the slot's inline template path when BOTH are present (V-PATHAGREE):
///   agreement is not an override, and disagreement refuses naming the slot
///   and both paths.
/// - FINGERPRINTS come from `--fingerprint @i=` or an origin-notated
///   `--key`'s bracket fingerprint; when BOTH name slot i they must AGREE
///   (V-FPAGREE) or refuse — never a silent override in either direction.
///
/// Returns `(parsed_keys, parsed_fingerprints, inline_declared)` — the last
/// is the per-slot set `apply_path_override_per_slot` needs to know which
/// slots the shared `--path` may still fill in.
type ResolvedKeysAndFingerprints = (Vec<ParsedKey>, Vec<ParsedFingerprint>, BTreeSet<u8>);

fn resolve_keys_fingerprints_and_precedence(
    template: &str,
    keys: &[String],
    fingerprints: &[String],
    ctx: ScriptCtx,
    network: bitcoin::Network,
) -> Result<ResolvedKeysAndFingerprints, CliError> {
    let origin_keys = keys
        .iter()
        .map(|k| parse_key_with_origin(k, ctx, network))
        .collect::<Result<Vec<_>, _>>()?;
    let explicit_fps: Vec<ParsedFingerprint> = fingerprints
        .iter()
        .map(|s| parse_fingerprint(s))
        .collect::<Result<Vec<_>, _>>()?;

    // Inline per-slot template origins, read BEFORE parse_template folds them
    // into path_decl's Shared/Divergent representation — which cannot answer
    // "did slot i declare one at all" once folded (see
    // apply_path_override_per_slot's doc comment for why).
    let occs = lex_placeholders(template)?;
    let mut inline_paths: BTreeMap<u8, DerivationPath> = BTreeMap::new();
    for occ in &occs {
        if let Some(p) = &occ.origin_path {
            inline_paths.entry(occ.i).or_insert_with(|| p.clone());
        }
    }
    let inline_declared: BTreeSet<u8> = inline_paths.keys().copied().collect();

    // V-PATHAGREE: an origin-notated --key path is checked against the
    // slot's inline template path ONLY when BOTH exist. A key bracket with
    // no path (a bare @i=XPUB, or [fp]xpub with no path steps) has nothing
    // to agree or disagree about: parse_key_with_origin represents "no path
    // given" as an empty DerivationPath. There is no template or bracket
    // syntax for an explicitly-empty-but-present path (C0's parser refuses a
    // bare trailing slash as BadOrigin before it ever reaches here), so this
    // is not a real ambiguity.
    for ok in &origin_keys {
        if ok.path.as_ref().is_empty() {
            continue;
        }
        if let Some(inline) = inline_paths.get(&ok.i) {
            if &ok.path != inline {
                return Err(CliError::Mismatch(format!(
                    "@{}: origin-notated --key path `{}` disagrees with the \
                     template's inline origin path `{}` for this slot (agreement \
                     is required; the --key path never overrides the template)",
                    ok.i, ok.path, inline
                )));
            }
        }
    }

    // V-FPAGREE: merge --fingerprint and origin-notated --key fingerprints,
    // agreeing or refusing per slot — never a silent override.
    let mut fp_map: BTreeMap<u8, [u8; 4]> = BTreeMap::new();
    for f in &explicit_fps {
        fp_map.insert(f.i, f.fp);
    }
    for ok in &origin_keys {
        let Some(fp) = ok.fingerprint else {
            continue;
        };
        match fp_map.get(&ok.i) {
            Some(existing) if *existing != fp => {
                return Err(CliError::Mismatch(format!(
                    "@{}: --fingerprint {} disagrees with the origin-notated --key \
                     fingerprint {} for this slot (agreement is required; neither \
                     side silently overrides the other)",
                    ok.i,
                    fp_hex(*existing),
                    fp_hex(fp)
                )));
            }
            Some(_) => {} // agree — no-op, never an override
            None => {
                fp_map.insert(ok.i, fp);
            }
        }
    }
    let parsed_fps: Vec<ParsedFingerprint> = fp_map
        .into_iter()
        .map(|(i, fp)| ParsedFingerprint { i, fp })
        .collect();
    let parsed_keys: Vec<ParsedKey> = origin_keys.into_iter().map(|ok| ok.key).collect();

    Ok((parsed_keys, parsed_fps, inline_declared))
}

/// **BIP 388 rule (1) on the T row** (REVIEW-converter-whole-diff-r1 C1).
///
/// SPEC A3 promises the converter "refuses BOTH forbidden shapes in BOTH
/// directions", and names shape (1) — one xpub filling two slots — as "the
/// reachable case where the engine's refusal binds". The S row delivered it
/// (`seat::satisfy::check_no_repeated_xpub`) and the D row delivered it
/// (`decompose::check_no_repeated_key`). This route did not: `md descriptor`
/// and `md address` build a `Descriptor` and RENDER it without ever encoding,
/// so `md_codec::validate::validate_no_duplicate_key_slots` — whose only call
/// site is `encode.rs:120` — never ran. Measured 2026-08-30 at `9d0c30dc`:
/// `--key @0=X --key @1=X --key @2=Y` emitted a checksummed `sortedmulti(2,X,X,Y)`
/// at exit 0, a 2-of-3 that X alone can spend, and `md decompose` then refused
/// the exact string `md descriptor` had just printed.
///
/// **THE DETECTION IS THE ENGINE'S OWN CALL, NOT A SECOND COPY.** The rule that
/// makes this check correct rather than merely strict is the use-site: one xpub
/// at `<0;1>` and at `<2;3>` derives a different child at every index, which is
/// two wallets and not a duplicate — BIP 388 permits it and `md encode` mints
/// it (`tests/duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`,
/// and measured through this very route before the fix: the disjoint template
/// composed at exit 0). A payload-only comparison over the parsed `--key`
/// values would not have that boundary and would newly refuse a wallet
/// `md encode` accepts — a fourth answer from one binary, in place of the
/// third. Calling the codec validator on the BUILT descriptor gets the
/// boundary, the multipath handling and the `@N` expansion for free, and
/// cannot drift from what `md encode` decides.
///
/// **THE WORDING IS THIS SIDE'S OWN.** `md_codec::Error::DuplicateKeySlots`
/// is worded for the wire layer and cites no BIP; SPEC A3's diagnostic rule
/// for the converter is "cite BIP 388, say unsupported, never invalid", which
/// is what the S and D rows say. The citation itself comes from
/// [`crate::bip388`], shared byte-for-byte with the S row.
///
/// Runs AFTER `apply_path_override_per_slot` so it inspects the descriptor
/// exactly as it is about to be rendered — and so that a slot whose origin
/// only `--path` supplies is expandable when the check runs (`expand_per_at_n`
/// raises `MissingExplicitOrigin` otherwise, which makes the validator a
/// silent no-op).
fn refuse_key_reuse_across_slots(d: &Descriptor, cmd: &str) -> Result<(), CliError> {
    match md_codec::validate::validate_no_duplicate_key_slots(d) {
        Ok(()) => Ok(()),
        Err(md_codec::Error::DuplicateKeySlots { a, b, n }) => Err(CliError::KeyReuse(format!(
            "@{a} and @{b} were given the SAME extended public key at the same use-site, so \
             `md {cmd}` would emit a policy that names {n} cosigners and that ONE of them can \
             satisfy alone — forbidden by BIP 388 (\"{rule}\"; {note}). UNSUPPORTED here, not \
             a malformed input: supply one distinct key per slot. `md encode` and \
             `md decompose` already refuse this wallet, so a card minted from it could never \
             be read back.",
            rule = crate::bip388::PAIRWISE_DISTINCT_RULE,
            note = crate::bip388::REUSE_SECURITY_NOTE,
        ))),
        // Unreachable today — the validator returns only that one variant, and
        // returns Ok(()) when expansion fails — but propagating rather than
        // swallowing keeps it that way if it ever grows a second.
        Err(e) => Err(CliError::from(e)),
    }
}

fn fp_hex(fp: [u8; 4]) -> String {
    format!("{:02x}{:02x}{:02x}{:02x}", fp[0], fp[1], fp[2], fp[3])
}
