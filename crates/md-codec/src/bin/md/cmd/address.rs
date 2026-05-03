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

    // Collect (chain, index, address) tuples first; then emit text or JSON.
    let mut rows: Vec<(u32, u32, String)> = Vec::with_capacity(args.count as usize);
    for k in 0..args.count {
        let i = args.index.checked_add(k).ok_or_else(|| CliError::BadArg(
            format!("--index + --count overflows u32: {} + {}", args.index, args.count)
        ))?;
        let addr = descriptor.derive_address(args.chain, i, args.network)?.assume_checked();
        rows.push((args.chain, i, addr.to_string()));
    }

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let addresses: Vec<serde_json::Value> = rows.iter().map(|(c, i, a)| {
            serde_json::json!({ "chain": c, "index": i, "address": a })
        }).collect();
        let v = serde_json::json!({
            "schema": SCHEMA,
            "network": args.network_str,
            "addresses": addresses,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = args.json;

    for (_, _, addr) in &rows {
        println!("{addr}");
    }
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
        return parse_template(template, &parsed_keys, &parsed_fps);
    }
    // Phrase path
    if args.phrases.len() == 1 {
        Ok(decode_md1_string(&args.phrases[0])?)
    } else {
        let refs: Vec<&str> = args.phrases.iter().map(String::as_str).collect();
        Ok(reassemble(&refs)?)
    }
}
