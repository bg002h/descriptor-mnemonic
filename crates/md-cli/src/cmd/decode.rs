use crate::cmd::partial::{ORIGIN_UNSPECIFIED_MARKER, emit_partial_stderr_note};
use crate::error::CliError;
use crate::format::text;
use md_codec::chunk::reassemble_with_opts;
use md_codec::decode::{DecodeOpts, decode_md1_string_with_opts};

pub fn run(strings: &[String], json: bool) -> Result<u8, CliError> {
    // mstring display-grouping (SPEC §3.2): strip separators so a grouped or
    // unbroken card both re-ingest.
    let strings = crate::cmd::strip_md1_inputs(strings);
    // P1.1: decode via the partial-allowing entry — a `canonical_origin ==
    // None` dead shape with no explicit origin now decodes (instead of
    // hard-rejecting `MissingExplicitOrigin`); `unresolved_origin_indices()`
    // below tells us whether that happened.
    let opts = DecodeOpts::partial();
    let descriptor = if strings.len() == 1 {
        decode_md1_string_with_opts(&strings[0], opts)?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble_with_opts(&refs, opts)?
    };
    let unres = descriptor.unresolved_origin_indices();
    let partial = !unres.is_empty();

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert(
            "descriptor".into(),
            serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap(),
        );
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

    let template = text::descriptor_to_template(&descriptor)?;
    println!("{template}");
    if partial {
        println!("{ORIGIN_UNSPECIFIED_MARKER}");
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
