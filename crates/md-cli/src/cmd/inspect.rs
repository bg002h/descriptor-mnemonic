use crate::cmd::partial::{ORIGIN_UNSPECIFIED_MARKER, emit_partial_stderr_note};
use crate::error::CliError;
use crate::format::text;
use md_codec::chunk::reassemble_with_opts;
use md_codec::decode::{DecodeOpts, decode_md1_string_with_opts};
use md_codec::identity::{
    WalletPolicyId, compute_md1_encoding_id, compute_wallet_descriptor_template_id,
    compute_wallet_policy_id,
};

pub fn run(strings: &[String], json: bool) -> Result<u8, CliError> {
    // mstring display-grouping (SPEC §3.2): strip separators on intake.
    let strings = crate::cmd::strip_md1_inputs(strings);
    // P1.1: decode via the partial-allowing entry (see `cmd::decode` for the
    // full contract).
    let opts = DecodeOpts::partial();
    let descriptor = if strings.len() == 1 {
        decode_md1_string_with_opts(&strings[0], opts)?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble_with_opts(&refs, opts)?
    };
    let unres = descriptor.unresolved_origin_indices();
    let partial = !unres.is_empty();

    let md1 = compute_md1_encoding_id(&descriptor)?;
    let tpl = compute_wallet_descriptor_template_id(&descriptor)?;
    // M-2 (funds-relevant): gate the COMPUTATION, not just the output.
    // `compute_wallet_policy_id` calls `expand_per_at_n` internally, which
    // stays strict/fail-closed and would raise `MissingExplicitOrigin` on a
    // partial descriptor — branch BEFORE calling it, never call it under
    // partial.
    let pid: Option<WalletPolicyId> = if partial {
        None
    } else {
        Some(compute_wallet_policy_id(&descriptor)?)
    };

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, JsonHash, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert(
            "descriptor".into(),
            serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap(),
        );
        obj.insert(
            "md1_encoding_id".into(),
            serde_json::to_value(JsonHash::from(&md1)).unwrap(),
        );
        obj.insert(
            "wallet_descriptor_template_id".into(),
            serde_json::to_value(JsonHash::from(&tpl)).unwrap(),
        );
        if let Some(pid) = &pid {
            obj.insert(
                "wallet_policy_id".into(),
                serde_json::to_value(JsonHash::from(pid)).unwrap(),
            );
        }
        if partial {
            obj.insert(
                "partial".into(),
                serde_json::json!({
                    "reason": "missing_explicit_origin",
                    "unresolved_indices": unres,
                }),
            );
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::Template,
            &mut std::io::stderr(),
        );
        if partial {
            emit_partial_stderr_note(&unres, &mut std::io::stderr());
        }
        return Ok(if partial { 4 } else { 0 });
    }
    let _ = json;

    println!("template: {}", text::descriptor_to_template(&descriptor)?);
    if partial {
        println!("{ORIGIN_UNSPECIFIED_MARKER}");
    }
    println!("n: {}", descriptor.n);
    println!("wallet-policy-mode: {}", descriptor.is_wallet_policy());
    println!("md1-encoding-id: {}", text::fmt_md1_id(&md1));
    println!(
        "wallet-descriptor-template-id: {}",
        text::fmt_template_id(&tpl)
    );
    if let Some(pid) = &pid {
        println!("wallet-policy-id: {}", text::fmt_policy_id(pid));
        println!(
            "wallet-policy-id-fingerprint: {}",
            text::fmt_policy_id_fingerprint(pid)
        );
    }
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::Template,
        &mut std::io::stderr(),
    );
    if partial {
        emit_partial_stderr_note(&unres, &mut std::io::stderr());
    }
    Ok(if partial { 4 } else { 0 })
}
