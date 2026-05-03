use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key, ParsedFingerprint};
use crate::parse::template::{ctx_for_template, parse_template};
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::Descriptor;

pub struct AddressArgs<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub chain: u32,
    pub index: u32,
    pub count: u32,
    pub json: bool,
}

pub fn run(args: AddressArgs<'_>) -> Result<(), CliError> {
    let descriptor = build_descriptor(&args)?;
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "address requires wallet-policy mode (Pubkeys TLV); supply --key @i=XPUB or use a wallet-policy-mode phrase".into(),
        ));
    }

    let _ = args.json;             // Phase 5 wires --json
    let _ = args.network_str;      // Phase 5 uses for JSON
    let _ = args.count;            // Phase 4 wires the loop

    // Phase 3 baseline: derive exactly one address at (chain, index).
    let addr = descriptor.derive_address(args.chain, args.index, args.network)?
        .assume_checked();
    println!("{addr}");
    Ok(())
}

fn build_descriptor(args: &AddressArgs<'_>) -> Result<Descriptor, CliError> {
    if let Some(template) = args.template {
        if args.keys.is_empty() {
            return Err(CliError::BadArg(
                "--key @i=<XPUB> required when --template is supplied".into()
            ));
        }
        let ctx = ctx_for_template(template);
        let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, args.network)).collect::<Result<Vec<_>, _>>()?;
        let parsed_fps: Vec<ParsedFingerprint> = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
        return Ok(parse_template(template, &parsed_keys, &parsed_fps)?);
    }
    // Phrase path
    if args.phrases.len() == 1 {
        Ok(decode_md1_string(&args.phrases[0])?)
    } else {
        let refs: Vec<&str> = args.phrases.iter().map(String::as_str).collect();
        Ok(reassemble(&refs)?)
    }
}
