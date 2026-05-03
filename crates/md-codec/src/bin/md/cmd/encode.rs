use crate::error::CliError;
use crate::format::text;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::template::{ctx_for_template, parse_template};

use md_codec::encode::encode_md1_string;
use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id};

pub struct EncodeArgs<'a> {
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub force_chunked: bool,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<(), CliError> {
    let ctx = ctx_for_template(args.template);
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let descriptor = parse_template(args.template, &parsed_keys, &parsed_fps)?;

    if args.force_chunked {
        // v0.14 `split` takes only &Descriptor (chunk size is determined
        // internally) and returns Vec<String>, no per-chunk struct.
        let chunks = split(&descriptor)?;
        let md1_id = compute_md1_encoding_id(&descriptor)?;
        let csid = derive_chunk_set_id(&md1_id);
        println!("chunk-set-id: 0x{csid:05x}");
        for s in &chunks {
            println!("{s}");
        }
    } else {
        let phrase = encode_md1_string(&descriptor)?;
        println!("{phrase}");
    }

    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!("policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&id));
    }

    let _ = args.force_long_code;  // long-code dropped in v0.12; arg accepted for forward-compat
    Ok(())
}
