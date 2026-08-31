use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::path::apply_path_override;
use crate::parse::template::{ctx_for_template, parse_template_ext};
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::encode::encode_payload;

pub struct VerifyArgs<'a> {
    pub strings: &'a [String],
    /// P3 §6b — read the md1 strings from this file instead of argv.
    pub in_file: Option<&'a std::path::Path>,
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    /// Shared origin-path override, mirroring `md encode --path`.
    pub path: Option<&'a str>,
    pub network: bitcoin::Network,
    /// Mirrors `md encode --experimental`.
    ///
    /// Verify must accept every template encode accepts, or a card authored
    /// with the flag becomes unverifiable — which is worse than not authoring
    /// it, because the operator has a plate and no way to check it.
    pub experimental: bool,
}

pub fn run(args: VerifyArgs<'_>) -> Result<u8, CliError> {
    // P3 §6b: argv, `--in FILE` or `-`; separators stripped on intake (§3.2).
    let strings = crate::cmd::read_md1_inputs(args.strings, args.in_file)?;
    let decoded = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let ctx = ctx_for_template(args.template);
    let parsed_keys = args
        .keys
        .iter()
        .map(|k| parse_key(k, ctx, args.network))
        .collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args
        .fingerprints
        .iter()
        .map(|s| parse_fingerprint(s))
        .collect::<Result<Vec<_>, _>>()?;
    // N1's WARN disposition, not REFUSE. `verify` READS: an operator holding a
    // legacy plate that carries a shape this cycle newly refuses must still be
    // able to check it, or the refusal has taken away the only tool that could
    // tell them what they have (SPEC_mdcli_mini.md N1 "Verb dispositions", and
    // Acceptance 5 -- such plates exist, `tests/fixtures/n1/`).
    let mut expected = parse_template_ext(
        args.template,
        &parsed_keys,
        &parsed_fps,
        args.experimental,
        crate::parse::reuse::Disposition::Warn,
    )?;
    // Mirrors `md encode --path`; see cmd/address.rs for why a verify without it
    // cannot reach a non-canonical wrapper at all.
    apply_path_override(&mut expected, args.path)?;
    let (decoded_bytes, decoded_bits) = encode_payload(&decoded)?;
    let (expected_bytes, expected_bits) = encode_payload(&expected)?;
    if decoded_bytes != expected_bytes || decoded_bits != expected_bits {
        return Err(CliError::Mismatch(format!(
            "expected {expected_bits}-bit payload, got {decoded_bits}-bit ({} vs {} bytes)",
            expected_bytes.len(),
            decoded_bytes.len()
        )));
    }
    println!("OK");
    Ok(0)
}
