use crate::error::CliError;
use crate::format::text;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::path::parse_path;
use crate::parse::template::{ctx_for_template, parse_template, to_origin_path};

use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::encode::{encode_md1_string, render_grouped};
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id};
use md_codec::origin_path::PathDeclPaths;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};

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
    /// mstring display-grouping (SPEC §3): insert `separator` every `group_size`
    /// chars in the text emit (0 = unbroken). Display only — `--json` stays unbroken.
    pub group_size: usize,
    pub separator: char,
    pub force_long_code: bool,
    pub policy_id_fingerprint: bool,
    pub json: bool,
}

pub fn run(args: EncodeArgs<'_>) -> Result<u8, CliError> {
    // F-A3: the long BCH code was removed in v0.12.0; md1 is regular-code-only
    // (payloads that don't fit a single string are chunked). The flag is kept
    // in the clap surface (no flag-NAME removal) but referencing it is now a
    // hard error rather than a silent no-op — a flag pointing at a nonexistent
    // mode must not exit 0.
    if args.force_long_code {
        return Err(CliError::BadArg(
            "the long BCH code was removed in v0.12.0; md1 is regular-code-only (payloads >400 bits are chunked)".into(),
        ));
    }
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
        // F-A4: legacy-P2SH-multisig footgun advisory (stderr, warn-only).
        // Must fire on the --json branch too (parity with the text branch).
        emit_legacy_p2sh_advisory(&descriptor.tree, &mut std::io::stderr());
        // L19 (cycle-9): a keyed (wallet-policy) md1 is watch-only, not a
        // keyless template — branch the advisory on the Pubkeys TLV.
        let class = if descriptor.is_wallet_policy() {
            crate::output_advisory::OutputClass::WatchOnly
        } else {
            crate::output_advisory::OutputClass::Template
        };
        crate::output_advisory::emit_output_class_advisory(class, &mut std::io::stderr());
        return Ok(0);
    }

    if args.force_chunked {
        let chunks = split(&descriptor)?;
        let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
        println!("chunk-set-id: 0x{csid:05x}");
        for s in &chunks {
            println!("{}", render_grouped(s, args.group_size, args.separator));
        }
    } else {
        println!(
            "{}",
            render_grouped(
                &encode_md1_string(&descriptor)?,
                args.group_size,
                args.separator
            )
        );
    }
    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!(
            "policy-id-fingerprint: {}",
            text::fmt_policy_id_fingerprint(&id)
        );
    }

    // F-A4: legacy-P2SH-multisig footgun advisory (stderr, warn-only).
    emit_legacy_p2sh_advisory(&descriptor.tree, &mut std::io::stderr());
    // L19 (cycle-9): a keyed (wallet-policy) md1 is watch-only, not a keyless
    // template — branch the advisory on the Pubkeys TLV.
    let class = if descriptor.is_wallet_policy() {
        crate::output_advisory::OutputClass::WatchOnly
    } else {
        crate::output_advisory::OutputClass::Template
    };
    crate::output_advisory::emit_output_class_advisory(class, &mut std::io::stderr());
    Ok(0)
}

/// F-A4: is `tree` a top-level bare legacy P2SH multisig — `sh(multi(...))`
/// or `sh(sortedmulti(...))` (the multi body directly under `sh`, NOT nested
/// in `wsh`)? These carry known footguns and are superseded by segwit forms.
fn is_legacy_p2sh_multisig(tree: &Node) -> bool {
    tree.tag == Tag::Sh
        && matches!(
            &tree.body,
            Body::Children(children)
                if children.len() == 1
                    && matches!(children[0].tag, Tag::Multi | Tag::SortedMulti)
        )
}

/// F-A4: emit the legacy-P2SH-multisig footgun advisory to `stderr` when
/// `tree` is a top-level bare `sh(multi)` / `sh(sortedmulti)`. Warn-only —
/// the card is still emitted on stdout. Modern forms (`wsh(multi)`, `wpkh`,
/// `tr`), `sh(wsh(...))`, and the canonical BIP44 `pkh` default are SILENT.
fn emit_legacy_p2sh_advisory<W: std::io::Write>(tree: &Node, stderr: &mut W) {
    if is_legacy_p2sh_multisig(tree) {
        let _ = writeln!(
            stderr,
            "warning: sh(multi)/sh(sortedmulti) is legacy P2SH multisig \u{2014} \
             susceptible to third-party txid malleability, the 520-byte redeemScript \
             limit caps you near ~15 keys, and it gets no segwit witness discount; \
             prefer wsh(...) or sh(wsh(...))"
        );
    }
}
