use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::path::apply_path_override;
use crate::parse::template::{ctx_for_template, parse_template_ext};
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::encode::encode_payload_unadmitted;

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
    let strings = crate::cmd::read_md1_inputs(args.strings, args.in_file, "--in")?;
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
    // F-639 (F-449 stage 2): verify COMPARES two serialisations and mints
    // nothing, so neither side goes through mint-time admission policy --
    // the comment above says the same of the parse. A mint rule must never
    // decide whether an engraved card can be checked (the 2026-09-19 class,
    // `md_codec::encode::encode_payload_for_identity`). Structural errors
    // still surface: they come from the writers, not the policy.
    let (decoded_bytes, decoded_bits) = encode_payload_unadmitted(&decoded)?;
    let (expected_bytes, expected_bits) = encode_payload_unadmitted(&expected)?;
    if decoded_bytes != expected_bytes || decoded_bits != expected_bits {
        // F-582: the message named two numbers and no cause, and the most
        // common cause is a flag the operator simply did not repeat.
        // Fingerprints are PART OF THE PAYLOAD, so verifying a plate minted
        // with `--fingerprint` against a template without it mismatches on
        // size -- on a plate set that is perfectly correct.
        //
        // The failure mode is the reaction, not the message: the operator who
        // makes verification pass by re-minting WITHOUT --fingerprint gets a
        // set two gates bless and a third had already condemned as unseatable.
        // So the hint points at the template side, never at re-minting.
        let hint = if parsed_fps.is_empty() && decoded_bytes.len() > expected_bytes.len() {
            "\n      The card is LARGER than the template. Fingerprints are part of the \
             payload, and none were given here: if this plate was minted with \
             --fingerprint, pass the SAME values to verify. Do not re-mint without \
             them to make this pass -- that is a different wallet, and its keys may \
             not be seatable."
        } else if decoded_bytes.len() == expected_bytes.len() && decoded_bits == expected_bits {
            // F-606: when only the CONTENT differs the message read
            // "expected 1447-bit payload, got 1447-bit (181 vs 181 bytes)" --
            // it offered two identical numbers as its evidence, at exactly the
            // moment the operator is standing over a plate and a descriptor
            // wanting to know WHICH field drifted. The verdict was right; the
            // evidence was vacuous.
            //
            // The first differing byte is the cheapest true thing available
            // here, and it localises the drift without decoding either side
            // again.
            let at = decoded_bytes
                .iter()
                .zip(expected_bytes.iter())
                .position(|(a, b)| a != b);
            match at {
                Some(i) => &format!(
                    "\n      Same SIZE, different CONTENT -- the byte counts above are not the \
                     difference. First byte that differs: offset {i} (template {:#04x}, card \
                     {:#04x}). A changed digest, key or path shows up exactly like this.",
                    expected_bytes[i], decoded_bytes[i]
                ),
                // Equal length, equal bytes, unequal bits: the tail bit-count
                // differs. Naming it beats repeating the byte counts.
                None => {
                    "\n      Same bytes and same size: the payloads differ only in their \
                         trailing BIT count."
                }
            }
        } else {
            ""
        };
        return Err(CliError::Mismatch(format!(
            "expected {expected_bits}-bit payload, got {decoded_bits}-bit ({} vs {} bytes){hint}",
            expected_bytes.len(),
            decoded_bytes.len()
        )));
    }
    println!("OK");
    Ok(0)
}
