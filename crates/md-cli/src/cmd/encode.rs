use crate::error::CliError;
use crate::format::text;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::path::parse_path;
use crate::parse::template::{ctx_for_template, parse_template, to_origin_path};

use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::encode::encode_md1_string;
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id};
use md_codec::origin_path::PathDeclPaths;

pub struct EncodeArgs<'a> {
    pub template: &'a str,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    /// Override the inferred shared origin path. Accepts named (`bip44|48|49|84|86`),
    /// hex (`0xNN`), or literal (`m/...`) forms. When `Some`, replaces
    /// `descriptor.path_decl.paths` with `PathDeclPaths::Shared(parsed)`,
    /// preserving the placeholder count `n`.
    pub path: Option<&'a str>,
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub force_chunked: bool,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
    pub json: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<u8, CliError> {
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
    let mut descriptor = parse_template(args.template, &parsed_keys, &parsed_fps)?;
    if let Some(p_arg) = args.path {
        let dp = parse_path(p_arg)?;
        descriptor.path_decl.paths = PathDeclPaths::Shared(to_origin_path(Some(&dp)));
    }

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("network".into(), args.network_str.into());
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
            obj.insert(
                "policy_id_fingerprint".into(),
                text::fmt_policy_id_fingerprint(&id).into(),
            );
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::Template,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }

    if args.force_chunked {
        let chunks = split(&descriptor)?;
        let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
        println!("chunk-set-id: 0x{csid:05x}");
        for s in &chunks {
            println!("{s}");
        }
    } else {
        println!("{}", encode_md1_string(&descriptor)?);
    }
    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!(
            "policy-id-fingerprint: {}",
            text::fmt_policy_id_fingerprint(&id)
        );
    }

    // --force-long-code: long-code mode was dropped in v0.12.0; the flag is
    // accepted for forward-compat (so older scripts don't break) but has no
    // effect. Status: wont-fix at v0.15.2 (FOLLOWUPS v0.15.1-phase-2-low-1).
    // Revisit only if a real long-code mode is reintroduced.
    let _ = args.force_long_code;
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::Template,
        &mut std::io::stderr(),
    );
    Ok(0)
}
