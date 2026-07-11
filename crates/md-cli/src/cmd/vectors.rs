use crate::error::CliError;
use crate::parse::keys::ParsedFingerprint;
use crate::parse::path::parse_path;
use crate::parse::template::{parse_template, to_origin_path};
use md_codec::origin_path::PathDeclPaths;
use std::fs;
use std::path::PathBuf;

// v0.5.1 ships the canonical corpus via `md_codec::test_vectors` — the
// single source of truth shared by md-codec's own integration tests, by
// this subcommand, and by md-cli's `tests/json_snapshots.rs` /
// `tests/template_roundtrip.rs`. Previously inlined here as a workaround
// for `cargo publish`'s out-of-package-include refusal; replaced in
// 0.5.1 by md-codec 0.33's public API.
use md_codec::test_vectors::{MANIFEST, Vector};

pub fn run(out: Option<String>) -> Result<u8, CliError> {
    let out_dir = match out {
        Some(p) => PathBuf::from(p),
        // v0.5.1 publish-fix: was `concat!(MANIFEST_DIR, "/../md-codec/tests/vectors")`
        // which only worked from inside the original workspace checkout. End users
        // installing via `cargo install md-cli` need a path that actually exists;
        // default to `./vectors` (current dir).
        None => PathBuf::from("./vectors"),
    };
    fs::create_dir_all(&out_dir)
        .map_err(|e| CliError::BadArg(format!("mkdir {out_dir:?}: {e}")))?;

    let mut entries: Vec<&Vector> = MANIFEST.iter().collect();
    entries.sort_by_key(|v| v.name);

    for v in entries {
        let fps: Vec<ParsedFingerprint> = v
            .fingerprints
            .iter()
            .map(|(i, fp)| ParsedFingerprint { i: *i, fp: *fp })
            .collect();
        let mut descriptor = parse_template(v.template, &[], &fps)?;
        // Apply the explicit shared origin for path-carrying (non-canonical)
        // vectors, mirroring `cmd/encode.rs`'s `--path` override. Without this
        // the emitted card would be a decode-rejecting "dead card" for shapes
        // whose `canonical_origin` is `None` (tr()+tree, NUMS-taproot), and
        // the `.descriptor.json`'s `path_decl` would not reflect the origin.
        if let Some(p_arg) = v.path {
            let dp = parse_path(p_arg)?;
            descriptor.path_decl.paths = PathDeclPaths::Shared(to_origin_path(Some(&dp)));
        }
        let (bytes, _bits) = md_codec::encode::encode_payload(&descriptor)?;

        write_lf(&out_dir.join(format!("{}.template", v.name)), v.template)?;
        let mut hex_str = String::with_capacity(bytes.len() * 2);
        for b in &bytes {
            use std::fmt::Write as _;
            write!(hex_str, "{b:02x}").unwrap();
        }
        write_lf(&out_dir.join(format!("{}.bytes.hex", v.name)), &hex_str)?;

        let phrase_text = if v.force_chunked {
            use md_codec::chunk::{derive_chunk_set_id, split};
            use md_codec::identity::compute_md1_encoding_id;
            let chunks = split(&descriptor)?;
            let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
            let mut s = format!("chunk-set-id: 0x{csid:05x}\n");
            for c in &chunks {
                s.push_str(c);
                s.push('\n');
            }
            s.trim_end_matches('\n').to_string()
        } else {
            md_codec::encode::encode_md1_string(&descriptor)?
        };
        write_lf(
            &out_dir.join(format!("{}.phrase.txt", v.name)),
            &phrase_text,
        )?;

        #[cfg(feature = "json")]
        {
            use crate::format::json::JsonDescriptor;
            let json = serde_json::to_string_pretty(&JsonDescriptor::from(&descriptor)).unwrap();
            write_lf(&out_dir.join(format!("{}.descriptor.json", v.name)), &json)?;
        }
    }
    Ok(0)
}

fn write_lf(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    let mut s = contents.replace("\r\n", "\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    fs::write(path, s.as_bytes()).map_err(|e| CliError::BadArg(format!("write {path:?}: {e}")))
}
