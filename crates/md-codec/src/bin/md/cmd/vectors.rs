use crate::error::CliError;
use crate::parse::keys::ParsedFingerprint;
use crate::parse::template::parse_template;
use std::path::PathBuf;
use std::fs;

#[path = "../../../../tests/vectors/manifest.rs"]
mod manifest;
use manifest::MANIFEST;

pub fn run(out: Option<String>) -> Result<(), CliError> {
    let out_dir = PathBuf::from(out.unwrap_or_else(|| "crates/md-codec/tests/vectors".into()));
    fs::create_dir_all(&out_dir).map_err(|e| CliError::BadArg(format!("mkdir {out_dir:?}: {e}")))?;

    let mut entries: Vec<&manifest::Vector> = MANIFEST.iter().collect();
    entries.sort_by_key(|v| v.name);

    for v in entries {
        let fps: Vec<ParsedFingerprint> = v.fingerprints.iter().map(|(i, fp)| ParsedFingerprint { i: *i, fp: *fp }).collect();
        let descriptor = parse_template(v.template, &[], &fps)?;
        let (bytes, _bits) = md_codec::encode::encode_payload(&descriptor)?;

        write_lf(&out_dir.join(format!("{}.template", v.name)), v.template)?;
        write_lf(&out_dir.join(format!("{}.bytes.hex", v.name)),
            &bytes.iter().map(|b| format!("{b:02x}")).collect::<String>())?;

        let phrase_text = if v.force_chunked {
            use md_codec::chunk::{derive_chunk_set_id, split};
            use md_codec::identity::compute_md1_encoding_id;
            let chunks = split(&descriptor)?;
            let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
            let mut s = format!("chunk-set-id: 0x{csid:05x}\n");
            for c in &chunks { s.push_str(c); s.push('\n'); }
            s.trim_end_matches('\n').to_string()
        } else {
            md_codec::encode::encode_md1_string(&descriptor)?
        };
        write_lf(&out_dir.join(format!("{}.phrase.txt", v.name)), &phrase_text)?;

        #[cfg(feature = "json")]
        {
            use crate::format::json::JsonDescriptor;
            let json = serde_json::to_string_pretty(&JsonDescriptor::from(&descriptor)).unwrap();
            write_lf(&out_dir.join(format!("{}.descriptor.json", v.name)), &json)?;
        }
    }
    Ok(())
}

fn write_lf(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    let mut s = contents.replace("\r\n", "\n");
    if !s.ends_with('\n') { s.push('\n'); }
    fs::write(path, s.as_bytes()).map_err(|e| CliError::BadArg(format!("write {path:?}: {e}")))
}
