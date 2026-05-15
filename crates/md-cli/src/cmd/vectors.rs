use crate::error::CliError;
use crate::parse::keys::ParsedFingerprint;
use crate::parse::template::parse_template;
use std::fs;
use std::path::PathBuf;

// v0.5.1 crates.io-publish fix: inline the MANIFEST constant that was
// previously `include!`d from `../md-codec/tests/vectors/manifest.rs`.
// `cargo publish` refuses out-of-package source includes (the published
// .crate file is self-contained). Maintenance note: this manifest is
// also used by `crates/md-codec/tests/template_roundtrip.rs`; the two
// copies must stay in sync until a future cycle factors this into a
// shared module (FOLLOWUP candidate: move to
// `md-codec/src/test_vectors.rs` as a `#[cfg(test)]`-or-feature-gated
// public API, OR feature-gate this whole `vectors` subcommand off by
// default in published md-cli builds).
mod manifest {
    #[allow(dead_code)]
    pub struct Vector {
        pub name: &'static str,
        pub template: &'static str,
        pub keys: &'static [(u8, &'static str)],
        pub fingerprints: &'static [(u8, [u8; 4])],
        pub force_chunked: bool,
    }

    pub const MANIFEST: &[Vector] = &[
        Vector { name: "wpkh_basic",         template: "wpkh(@0/<0;1>/*)",                                   keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "pkh_basic",          template: "pkh(@0/<0;1>/*)",                                    keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "wsh_multi_2of2",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",                keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "wsh_multi_2of3",     template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "wsh_sortedmulti",    template: "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))", keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "tr_keyonly",         template: "tr(@0/<0;1>/*)",                                     keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "sh_wsh_multi",       template: "sh(wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*)))",            keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "wsh_divergent_paths", template: "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))",               keys: &[], fingerprints: &[], force_chunked: false },
        Vector { name: "wsh_with_fingerprints", template: "wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))",
            keys: &[],
            fingerprints: &[(0, [0xDE,0xAD,0xBE,0xEF]), (1, [0xCA,0xFE,0xBA,0xBE])],
            force_chunked: false },
        Vector { name: "wsh_multi_chunked",  template: "wsh(multi(3,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))",     keys: &[], fingerprints: &[], force_chunked: true },
    ];
}
use manifest::MANIFEST;

pub fn run(out: Option<String>) -> Result<(), CliError> {
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

    let mut entries: Vec<&manifest::Vector> = MANIFEST.iter().collect();
    entries.sort_by_key(|v| v.name);

    for v in entries {
        let fps: Vec<ParsedFingerprint> = v
            .fingerprints
            .iter()
            .map(|(i, fp)| ParsedFingerprint { i: *i, fp: *fp })
            .collect();
        let descriptor = parse_template(v.template, &[], &fps)?;
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
    Ok(())
}

fn write_lf(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    let mut s = contents.replace("\r\n", "\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    fs::write(path, s.as_bytes()).map_err(|e| CliError::BadArg(format!("write {path:?}: {e}")))
}
