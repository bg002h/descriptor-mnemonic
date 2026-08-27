use crate::error::CliError;
use crate::format::text;
use crate::parse::keys::{parse_fingerprint, parse_key};
use crate::parse::path::apply_path_override;
use crate::parse::template::{ctx_for_template, parse_template_ext};

use md_codec::chunk::{derive_chunk_set_id, split};
use md_codec::encode::{Descriptor, encode_md1_string, render_grouped};
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_policy_id};
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};

pub struct EncodeArgs<'a> {
    pub template: &'a str,
    /// P3 §6b — write the artifact to this file (created 0600) instead of
    /// stdout. Affects the ARTIFACT only: the engraving card, the chunk-set-id
    /// and every advisory still go to stderr.
    pub out_file: Option<&'a std::path::Path>,
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
    /// Relax rust-miniscript's SIGNATURE sanity rule, admitting a spend path
    /// that requires no key (e.g. a hashlock + timelock recovery tier).
    ///
    /// Only `top_unsafe` is relaxed; malleability, resource limits, repeated
    /// keys and timelock mixing still apply. See `parse_template_ext`.
    pub experimental: bool,
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
    let mut descriptor =
        parse_template_ext(args.template, &parsed_keys, &parsed_fps, args.experimental)?;
    if args.experimental {
        // LOUD, on stderr, every time. This authors a card for a wallet whose
        // guarantees rust-miniscript declines to vouch for, and the card itself
        // carries no record that a flag was used to create it — the operator's
        // memory and this line are the only trace.
        eprintln!(
            "warning: --experimental relaxed the signature rule. This descriptor has at least \
             one spend path that needs NO key, so whoever learns its preimage can spend it \
             alone. If that preimage is engraved, THE PLATE IS BEARER ACCESS. Malleability, \
             resource limits, repeated keys and timelock mixing were still checked."
        );
    }
    apply_path_override(&mut descriptor, args.path)?;

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
        // P1.2 (pathless/dead-card partial-decode): loud advisory when the
        // FINAL descriptor (post `--path`) still has an unresolvable origin.
        // Must fire on the --json branch too (parity with the text branch).
        emit_pathless_advisory(&descriptor, &mut std::io::stderr());
        // F-227: unseatable keyless template (stderr, warn-only).
        emit_unseatable_template_advisory(&descriptor, &mut std::io::stderr());
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

    // AUTHORING GATE: refuse an `older()` that consensus will not enforce.
    //
    // BIP-68 reads bit 31, bit 22 and bits 0-15 of a relative locktime and
    // discards the rest, so `older(210000)` locks for 13392 blocks and
    // `older(65536)` locks for nothing at all -- while rust-miniscript accepts
    // both and this codec round-trips them faithfully. That leniency is
    // deliberate and pinned (`proptest_to_miniscript`'s
    // `self_test_older_0x10000_miniscript_leniency`): a codec must represent
    // whatever the descriptor layer accepts.
    //
    // But `md encode` AUTHORS an artifact that gets engraved in metal and read
    // for years, and a plate asserting a four-year lock the chain releases in
    // three months is a funds-safety defect. `mnemonic-toolkit` already split
    // it exactly this way -- a blocking gate on its authoring surface
    // (SPEC_older_timelock_mask_gate.md) and a non-blocking advisory on intake
    // (SPEC_older_timelock_advisory.md, scoped "toolkit-only") -- and this is
    // the same gate on md's authoring surface.
    md_codec::validate::validate_relative_timelocks(&descriptor.tree)?;

    // Single string when it fits, chunked when it does not -- AUTOMATICALLY.
    //
    // A payload over the codex32 regular code's 80-data-symbol cap used to be a
    // hard error telling the operator to retry with `--force-chunked`. Every
    // keyed wallet policy is over that cap (a 2-of-2 is 246 data symbols), so
    // the first encounter with a real multisig read as "this policy is
    // unsupported". Two places already described the dispatch as automatic --
    // this flag's own help ("even for SHORT policies", implying long ones are
    // automatic) and `--force-long-code`'s ("payloads over 400 bits are
    // chunked"). The documentation was right about the intent; the encoder was
    // what disagreed. F-136.
    //
    // `--force-chunked` keeps its documented meaning: chunk even when a single
    // string would fit.
    //
    // The fallback matches ONLY the overflow error. Any other encode failure
    // still propagates -- silently chunking around an unrelated fault would
    // turn a diagnosable error into a surprising output shape.
    let single = if args.force_chunked {
        None
    } else {
        match encode_md1_string(&descriptor) {
            Ok(s) => Some(s),
            Err(md_codec::error::Error::PayloadTooLongForSingleString { .. }) => None,
            Err(e) => return Err(e.into()),
        }
    };

    // P3 §6a + D4. stdout is the canonical artifact, UNBROKEN, and nothing
    // else; the grouped form -- the thing a human actually transcribes onto a
    // plate -- moves to the stderr engraving card below. Before this, a
    // pipeline had to strip a header line AND pass `--group-size 0`, and
    // `me sysw pack` classified a grouped chunk as "not a form this container
    // can place" at exit 4.
    let cards: Vec<String> = match single {
        Some(s) => vec![s],
        None => {
            let chunks = split(&descriptor)?;
            let csid = derive_chunk_set_id(&compute_md1_encoding_id(&descriptor)?);
            // The chunk-set-id is not deleted -- it is the only thing that
            // tells an operator which chunks belong to one card. It heads the
            // engraving card on stderr.
            //
            // The fixture-file writer in `cmd/vectors.rs` still emits this same
            // text into a FILE and is deliberately untouched: §6a is a rule
            // about stdout, and a grep for the string finds two sites of which
            // only this one moves.
            eprintln!("chunk-set-id: 0x{csid:05x}");
            chunks
        }
    };
    // P3 §6b: the artifact goes to `--out FILE` when given, stdout otherwise --
    // "stdout is used when `--out` is not given", and the input channel has no
    // bearing on it. The file is CREATED 0600 through the shared crate's
    // `write_private`; a shell redirect cannot do that, which is why the flag
    // exists at all (F-244).
    let mut body = String::new();
    for s in &cards {
        body.push_str(s);
        body.push('\n');
    }
    match args.out_file {
        Some(p) => crate::cmd::write_artifact(p, &body)?,
        None => print!("{body}"),
    }
    emit_engraving_card(
        &cards,
        args.group_size,
        args.separator,
        &mut std::io::stderr(),
    );
    if args.policy_id_fingerprint {
        let id = compute_wallet_policy_id(&descriptor)?;
        println!(
            "policy-id-fingerprint: {}",
            text::fmt_policy_id_fingerprint(&id)
        );
    }

    // F-A4: legacy-P2SH-multisig footgun advisory (stderr, warn-only).
    emit_legacy_p2sh_advisory(&descriptor.tree, &mut std::io::stderr());
    // P1.2 (pathless/dead-card partial-decode): loud advisory when the FINAL
    // descriptor (post `--path`) still has an unresolvable origin.
    emit_pathless_advisory(&descriptor, &mut std::io::stderr());
    // F-227: unseatable keyless template (stderr, warn-only).
    emit_unseatable_template_advisory(&descriptor, &mut std::io::stderr());
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

/// P3 §6c: the keyword for a separator char, for the card's `separator:` line.
///
/// The card names the flag VALUE a reader would pass to reproduce it, not the
/// raw char -- ` ` on a line of its own is invisible, which is the one value
/// that matters after §6c narrows this to whitespace.
fn separator_name(sep: char) -> String {
    match sep {
        ' ' => "space".into(),
        '-' => "hyphen".into(),
        ',' => "comma".into(),
        other => other.to_string(),
    }
}

/// P3 §6c / D4 — the engraving card, on stderr.
///
/// §6c rules that `md` and `mk` have no card to move the grouped form to, that
/// P3 owns the contents, and that the minimum it must carry is the grouped
/// string itself -- after D4 this is the only place that form exists.
///
/// SHAPE follows `ms`'s: plain `label: value` lines, no prefix character,
/// ending with the tool's existing output-class advisory (emitted by the
/// caller, after the warnings). `mnemonic bundle`'s `#`-prefixed card is the
/// other in-constellation precedent and is deliberately NOT followed -- its `#`
/// mirrors the comment headers on its own stdout, a surface `md` does not have.
///
/// The grouped string comes FIRST because it is the thing a human transcribes.
/// Every chunk of a chunk set is rendered, not just the first: a chunk missing
/// from the card is a chunk nobody engraves.
///
/// There is no `--no-engraving-card` on `md` -- §6c names that flag for `ms`
/// and `mnemonic` only, and warns that it is what makes "no grouped form
/// anywhere" possible.
fn emit_engraving_card<W: std::io::Write>(
    cards: &[String],
    group_size: usize,
    separator: char,
    stderr: &mut W,
) {
    for s in cards {
        let _ = writeln!(stderr, "{}", render_grouped(s, group_size, separator));
    }
    let _ = writeln!(stderr, "group size: {group_size}");
    let _ = writeln!(stderr, "separator: {}", separator_name(separator));
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

/// P1.2 (pathless/dead-card partial-decode): emit a loud stderr advisory
/// when the FINAL descriptor (`--path` already applied) still carries an
/// unresolvable origin — i.e. `descriptor.unresolved_origin_indices()` is
/// non-empty, the EXACT same P0 query `md decode`/`md inspect` use to decide
/// partial. This keys on actual resolvability, NOT a `canonical_origin ==
/// None` + `--path.is_none()` heuristic (I-1 whole-diff fix), so:
///   - an inline per-`@N` explicit origin (e.g.
///     `sh(sortedmulti(2,@0/48'/0'/0'/1'/<0;1>/*,…))`) with no `--path`
///     FULL-decodes (exit 0) and is therefore NOT falsely warned; and
///   - a `--path m` (zero-component) override on a dead shape, which still
///     partial-decodes at exit 4, IS warned (the footgun is not bypassed by
///     the presence of a `--path` flag alone).
///
/// A warned card still mints fine (exit 0, unchanged) — the advisory nudges
/// the encoder-side fix (a real `--path`) at mint time, mirroring the F-A4
/// footgun-advisory tone. Warn-only; never affects stdout or the exit code.
/// F-227: warn when a KEYLESS template's slots cannot be told apart.
///
/// A keyless template names its slots by origin and is restored by seating one
/// mk1 key card per slot. SeedHammer II's rule (`gui/key_card_seating.go`)
/// matches a card to a slot on the slot's declared **origin**, plus its
/// **fingerprint only when the template declares one**, and refuses every state
/// it cannot decide. So two slots sharing a declaration make the template
/// **unseatable**: every card matches both, the device refuses
/// (`errSeatSlotContested`), and the operator finds out on attempting a restore
/// — after the plate is cut.
///
/// THE PREDICATE IS THE DEVICE'S, not a heuristic. Slots collide iff
/// `(fingerprint-when-declared, origin path)` are equal. That is why declaring
/// a fingerprint on only SOME of a colliding group does not help: a slot with
/// no declaration still matches any card at that path.
///
/// WARN, DO NOT REFUSE. A bare template is legal, and an operator may
/// deliberately record slot order out of band. Exit stays 0 and the card is
/// still emitted, matching the pathless and legacy-P2SH advisories.
///
/// Keyed cards are exempt: they carry their own keys, so nothing is ever seated
/// onto them and the warning would be pure noise.
fn emit_unseatable_template_advisory<W: std::io::Write>(descriptor: &Descriptor, stderr: &mut W) {
    if descriptor.is_wallet_policy() {
        return;
    }
    let Ok(expanded) = md_codec::canonicalize::expand_per_at_n(descriptor) else {
        // An expansion failure is not this advisory's error to raise — the
        // pathless advisory above already speaks for a dead or pathless card,
        // and a second complaint about the same defect is noise.
        return;
    };
    if expanded.len() < 2 {
        return;
    }

    // AMBIGUITY IS NOT EQUALITY OF DECLARATIONS, and getting that wrong is the
    // whole subtlety here. Two slots are ambiguous iff ONE CARD CAN MATCH BOTH:
    //
    //     same origin path, AND NOT (both declare a fingerprint and differ)
    //
    // because `slotMatchesCard` checks the fingerprint only when the slot
    // declares one. So an undeclared slot is ambiguous with EVERY slot at its
    // path, whatever they declare — grouping by `(fingerprint, path)` would
    // put them in different buckets and report nothing.
    //
    // Note the relation is not transitive: @0=[X], @1=[-], @2=[Y] at one path
    // has @0~@1 and @1~@2 but not @0~@2. Reported per PATH rather than as
    // components, because the path is what the operator has to act on and the
    // remedy — declare the missing fingerprints — is the same either way.
    /// Slots sharing one origin path: `(slot index, declared fingerprint)`.
    type SlotsAtPath = Vec<(u8, Option<[u8; 4]>)>;
    let mut by_path: std::collections::BTreeMap<String, SlotsAtPath> =
        std::collections::BTreeMap::new();
    for e in &expanded {
        let mut path = String::from("m");
        for c in &e.origin_path.components {
            path.push('/');
            path.push_str(&c.value.to_string());
            if c.hardened {
                path.push('\'');
            }
        }
        by_path
            .entry(path)
            .or_default()
            .push((e.idx, e.fingerprint));
    }

    let mut collisions: Vec<(String, Vec<u8>)> = Vec::new();
    for (path, slots) in &by_path {
        if slots.len() < 2 {
            continue;
        }
        let any_undeclared = slots.iter().any(|(_, fp)| fp.is_none());
        let ambiguous: Vec<u8> = if any_undeclared {
            // One undeclared slot is ambiguous with every other slot here, so
            // the whole path is undecidable.
            slots.iter().map(|(i, _)| *i).collect()
        } else {
            // All declared: only slots sharing a fingerprint collide.
            let mut counts: std::collections::BTreeMap<[u8; 4], usize> =
                std::collections::BTreeMap::new();
            for (_, fp) in slots {
                *counts.entry(fp.expect("checked declared")).or_default() += 1;
            }
            slots
                .iter()
                .filter(|(_, fp)| counts[&fp.expect("checked declared")] > 1)
                .map(|(i, _)| *i)
                .collect()
        };
        if ambiguous.len() > 1 {
            let mut idxs = ambiguous;
            idxs.sort_unstable();
            collisions.push((path.clone(), idxs));
        }
    }
    if collisions.is_empty() {
        return;
    }
    collisions.sort_by_key(|(_, idxs)| idxs[0]);

    let detail = collisions
        .iter()
        .map(|(path, idxs)| {
            let slots = idxs
                .iter()
                .map(|i| format!("@{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{slots} all declare {path}")
        })
        .collect::<Vec<_>>()
        .join("; ");

    let _ = writeln!(
        stderr,
        "warning: this keyless template's slots cannot be told apart \u{2014} {detail}. \
         Restoring seats one key card per slot by matching the slot's declared origin \
         (and its fingerprint, when declared), so a card here matches several slots and \
         a device that will not guess must refuse the whole set. Pass one \
         --fingerprint @N=HEX per slot to make them distinguishable; it costs about \
         one extra md1 chunk and changes no path, no key and no policy."
    );
}

fn emit_pathless_advisory<W: std::io::Write>(descriptor: &Descriptor, stderr: &mut W) {
    if descriptor.unresolved_origin_indices().is_empty() {
        return;
    }
    let _ = writeln!(
        stderr,
        "warning: this template's top-level wrapper has no canonical default derivation path \
         \u{2014} without an explicit origin, `md decode`/`md inspect` will only PARTIAL-DECODE \
         this card (origin unspecified, exit 4) and it cannot be reliably restored on its own; \
         supply --path (e.g. --path bip48) for a fully-decodable backup"
    );
}
