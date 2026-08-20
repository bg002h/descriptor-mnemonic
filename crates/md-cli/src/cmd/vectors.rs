use crate::error::CliError;
use crate::parse::keys::{ParsedFingerprint, ParsedKey, parse_key};
use crate::parse::path::apply_path_override;
use crate::parse::template::{ctx_for_template, parse_template};
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
        // `Vector::keys` IS BOUND HERE. It was documented as "(@N, xpub) pairs
        // binding each @N placeholder" and read by NOTHING: this call passed
        // `&[]` unconditionally, so every vector encoded template-only no matter
        // what the manifest said. A populated `keys` changed no byte, which is a
        // worse failure than an empty one -- it would have taught the next
        // maintainer that keys do not affect the wire.
        let parsed_keys: Vec<ParsedKey> = v
            .keys
            .iter()
            .map(|(i, xpub)| {
                parse_key(
                    &format!("@{i}={xpub}"),
                    ctx_for_template(v.template),
                    bitcoin::Network::Bitcoin,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut descriptor = parse_template(v.template, &parsed_keys, &fps)?;
        // The explicit shared origin for path-carrying (non-canonical) vectors.
        // Without it the emitted card is a decode-rejecting "dead card" for
        // shapes whose `canonical_origin` is `None` (tr()+tree, NUMS-taproot).
        // Routed through the SAME helper `encode`/`address`/`verify` use, rather
        // than a fourth copy of the rule.
        apply_path_override(&mut descriptor, v.path)?;
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

            // The CONFORMANCE export (R3). Emitted only for KEYED vectors: a
            // keyless template has no addresses and no WalletPolicyId, so a
            // conformance file for one would be mostly nulls pretending to be
            // a contract.
            if !v.keys.is_empty() {
                let json = conformance_json(v, &descriptor)?;
                write_lf(&out_dir.join(format!("{}.conformance.json", v.name)), &json)?;
            }
        }
    }
    Ok(0)
}

/// Build the per-vector conformance record the Go port checks itself against.
///
/// WHY THIS EXISTS (R3). Every entry in the manifest was keyless, so the Go
/// side had nothing keyed to conform to: it could agree with Rust about
/// template bytes and still derive a different address, and nothing would say
/// so. These records are the contract.
///
/// EVERY FIELD IS DERIVED, NEVER TRANSCRIBED. The descriptor string, the ids,
/// the chunks and the addresses all come out of the same `Descriptor` that
/// produced the wire bytes beside them, so a record cannot disagree with its
/// own card.
#[cfg(feature = "json")]
fn conformance_json(v: &Vector, d: &md_codec::encode::Descriptor) -> Result<String, CliError> {
    use md_codec::identity::{
        compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id,
    };
    use serde_json::{Map, Value, json};

    let hex16 = |b: &[u8; 16]| -> String {
        let mut s = String::with_capacity(32);
        for x in b {
            use std::fmt::Write as _;
            write!(s, "{x:02x}").unwrap();
        }
        s
    };
    let err = |e: md_codec::Error| CliError::BadArg(format!("{}: {e}", v.name));

    // Both ids, always, and NAMED. They differ for the same wallet -- the
    // template id is key-stable, the policy id is not -- so a consumer that
    // compared the wrong one against a coordinator would see a false mismatch.
    let mut root = Map::new();
    root.insert("name".into(), json!(v.name));
    root.insert("template".into(), json!(v.template));
    root.insert("path".into(), json!(v.path));
    root.insert(
        "keys".into(),
        Value::Array(
            v.keys
                .iter()
                .map(|(i, x)| json!({ "index": i, "xpub": x }))
                .collect(),
        ),
    );
    root.insert(
        "fingerprints".into(),
        Value::Array(
            v.fingerprints
                .iter()
                .map(|(i, fp)| {
                    let mut h = String::new();
                    for b in fp {
                        use std::fmt::Write as _;
                        write!(h, "{b:02x}").unwrap();
                    }
                    json!({ "index": i, "fingerprint": h })
                })
                .collect(),
        ),
    );
    root.insert(
        "md1_encoding_id".into(),
        json!(hex16(compute_md1_encoding_id(d).map_err(err)?.as_bytes())),
    );
    root.insert(
        "wallet_descriptor_template_id".into(),
        json!(hex16(
            compute_wallet_descriptor_template_id(d)
                .map_err(err)?
                .as_bytes()
        )),
    );
    root.insert(
        "wallet_policy_id".into(),
        json!(hex16(compute_wallet_policy_id(d).map_err(err)?.as_bytes())),
    );

    // Per chain: the canonical descriptor string, and the first addresses.
    // `<0;1>` multipath means chain 0 is receive and chain 1 is change; D6's
    // reason for carrying BOTH is that change is where a policy mismatch
    // silently loses funds.
    const ADDRS_PER_CHAIN: u32 = 3;
    let mut chains = Map::new();
    for chain in 0u32..2 {
        let desc = match md_codec::to_miniscript::to_miniscript_descriptor(d, chain) {
            Ok(x) => x,
            Err(_) => continue, // single-path vectors have no chain 1
        };
        let mut addrs = Vec::new();
        for index in 0..ADDRS_PER_CHAIN {
            match d.derive_address(chain, index, bitcoin::Network::Bitcoin) {
                Ok(a) => addrs.push(json!(a.assume_checked().to_string())),
                Err(_) => break,
            }
        }
        chains.insert(
            chain.to_string(),
            json!({ "descriptor": desc.to_string(), "addresses": addrs }),
        );
    }
    root.insert("chains".into(), Value::Object(chains));

    Ok(serde_json::to_string_pretty(&Value::Object(root)).unwrap())
}

fn write_lf(path: &std::path::Path, contents: &str) -> Result<(), CliError> {
    let mut s = contents.replace("\r\n", "\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    fs::write(path, s.as_bytes()).map_err(|e| CliError::BadArg(format!("write {path:?}: {e}")))
}
