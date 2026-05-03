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
    pub json: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<(), CliError> {
    let ctx = ctx_for_template(args.template);
    let parsed_keys = args.keys.iter().map(|k| parse_key(k, ctx, bitcoin::Network::Bitcoin)).collect::<Result<Vec<_>, _>>()?;
    let parsed_fps = args.fingerprints.iter().map(|s| parse_fingerprint(s)).collect::<Result<Vec<_>, _>>()?;
    let descriptor = parse_template(args.template, &parsed_keys, &parsed_fps)?;

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        if args.force_chunked {
            let chunks = split(&descriptor)?;
            let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
            obj.insert("chunk_set_id".into(), format!("0x{csid:05x}").into());
            obj.insert("chunks".into(), serde_json::to_value(&chunks).unwrap());
        } else {
            obj.insert("phrase".into(), encode_md1_string(&descriptor)?.into());
        }
        if args.policy_id_fingerprint {
            let id = compute_wallet_policy_id(&descriptor)?;
            obj.insert("policy_id_fingerprint".into(), text::fmt_policy_id_fingerprint(&id).into());
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }

    if args.force_chunked {
        let chunks = split(&descriptor)?;
        let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
        println!("chunk-set-id: 0x{csid:05x}");
        for s in &chunks { println!("{s}"); }
    } else {
        println!("{}", encode_md1_string(&descriptor)?);
    }
    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!("policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&id));
    }

    let _ = args.force_long_code;
    Ok(())
}
