use crate::error::CliError;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::template::{ctx_for_template, parse_template};
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub struct VerifyArgs<'a> {
    pub strings: &'a [String],
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub network: bitcoin::Network,
}

pub fn run(args: VerifyArgs<'_>) -> Result<(), CliError> {
    let decoded = if args.strings.len() == 1 {
        decode_md1_string(&args.strings[0])?
    } else {
        let refs: Vec<&str> = args.strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let ctx = ctx_for_template(args.template);
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, args.network)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let expected = parse_template(args.template, &parsed_keys, &parsed_fps)?;
    let (decoded_bytes, decoded_bits) = encode_payload(&decoded)?;
    let (expected_bytes, expected_bits) = encode_payload(&expected)?;
    if decoded_bytes != expected_bytes || decoded_bits != expected_bits {
        return Err(CliError::Mismatch(format!(
            "expected {expected_bits}-bit payload, got {decoded_bits}-bit ({} vs {} bytes)",
            expected_bytes.len(), decoded_bytes.len()
        )));
    }
    println!("OK");
    Ok(())
}
