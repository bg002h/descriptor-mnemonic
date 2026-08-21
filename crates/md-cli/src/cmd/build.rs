//! Building a [`Descriptor`] from either input shape, in ONE place.
//!
//! Two commands now need it — `md address` and `md descriptor` — and they must
//! agree exactly. The `--path` override in particular is load-bearing: a
//! non-canonical wrapper refuses with "non-canonical wrapper requires explicit
//! origin for @N", so a command that forgot to apply it would refuse the very
//! shapes the flag exists for (taproot with a script tree, and the miniscript
//! wrappers generally). Two copies is how one of them ends up without it.

use crate::error::CliError;
use crate::parse::keys::{ParsedFingerprint, parse_fingerprint, parse_key};
use crate::parse::path::apply_path_override;
use crate::parse::template::{ctx_for_template, parse_template};
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::encode::Descriptor;

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
        let parsed_keys = args
            .keys
            .iter()
            .map(|k| parse_key(k, ctx, args.network))
            .collect::<Result<Vec<_>, _>>()?;
        let parsed_fps: Vec<ParsedFingerprint> = args
            .fingerprints
            .iter()
            .map(|s| parse_fingerprint(s))
            .collect::<Result<Vec<_>, _>>()?;
        let mut descriptor = parse_template(template, &parsed_keys, &parsed_fps)?;
        apply_path_override(&mut descriptor, args.path)?;
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
