use crate::cmd::partial::{ORIGIN_UNSPECIFIED_MARKER, emit_partial_stderr_note};
use crate::error::CliError;
use crate::format::text;
use md_codec::chunk::reassemble_with_opts;
use md_codec::decode::{DecodeOpts, decode_md1_string_with_opts};

pub fn run(
    strings: &[String],
    in_file: Option<&std::path::Path>,
    json: bool,
) -> Result<u8, CliError> {
    // P3 §6b: argv, `--in FILE` or `-` for stdin. The reader strips mstring
    // display separators (SPEC §3.2) per line, so a grouped or unbroken card
    // both re-ingest through every one of the three.
    let strings = crate::cmd::read_md1_inputs(strings, in_file, "--in")?;
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
    // N1's WARN disposition on the CARD (plan P3 step 2, Acceptance 5). A
    // plate already carrying a shape this cycle newly refuses must still
    // READ, or the refusal has taken away the only tool that could tell its
    // holder what they have. Same predicate and same body as the refusal --
    // only the disposition differs (`crate::parse::reuse`).
    crate::parse::reuse::check_descriptor(&descriptor, crate::parse::reuse::Disposition::Warn)?;
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

    // F-610: `md encode --experimental` shouts "THE PLATE IS BEARER ACCESS" at
    // the person who MINTS. `md decode` is the verb the RESTORER runs --
    // possibly a different person, years later -- and it said nothing about a
    // spend path that needs no signature. encode.rs:76 names the gap itself:
    // "the card itself carries no record that a flag was used to create it —
    // the operator's memory and this line are the only trace."
    //
    // The key-less path IS visible in the template above, to a reader who can
    // read miniscript. The decode side is where the audience least able to do
    // that is standing.
    //
    // SAME PREDICATE AS F-599, not a second implementation: re-parse the
    // rendered template WITHOUT `--experimental`. Failing there means the
    // signature rule is what rejects it -- that flag relaxes exactly one rule.
    // A template that parses cleanly says nothing, so this cannot become noise.
    if crate::parse::template::parse_template_ext(
        &template,
        &[],
        &[],
        false,
        // REFUSE, NOT WARN, AND THAT IS THE WHOLE TRICK. `Disposition::Warn` is
        // the READ-side disposition -- deliberately lenient so a plate already
        // carrying a newly-refused shape can still be read -- so it returns Ok
        // here and the check silently never fires. Measured: the first version
        // of this used Warn, compiled, installed, and printed nothing on a
        // bearer card. The question being asked is "would a MINTING verb refuse
        // this?", so it must be asked in the minting disposition.
        crate::parse::reuse::Disposition::Refuse,
    )
    .is_err()
        && crate::parse::template::parse_template_ext(
            &template,
            &[],
            &[],
            true,
            crate::parse::reuse::Disposition::Refuse,
        )
        .is_ok()
    {
        eprintln!(
            "warning: this card has at least one spend path that needs NO KEY. Whoever \
             learns its preimage can spend that path alone — if the preimage is engraved, \
             THE PLATE IS BEARER ACCESS. It was minted with --experimental; the card carries \
             no record of that, so this is derived from the policy itself."
        );
    }
    // THE ORIGINS, ON STDERR (F-219).
    //
    // The rendered template writes `@0/<0;1>/*` — the per-key origin lives in
    // the payload and does not appear in that text, so an operator restoring a
    // plate reaches for the command named for the job and silently loses the
    // field a SIGNER uses to find its key. `md verify` proves the card carries
    // them; `md inspect` now prints them; and this is the command people
    // actually run.
    //
    // STDERR, not stdout, and deliberately: stdout is the template and is piped
    // into `md verify`, `md encode` and diffs. Adding lines there would break
    // every such pipeline to fix a display problem. This CLI already annotates
    // on stderr — the output-class advisory below is the precedent.
    //
    // NOT a fix for the deeper half: decode's stdout still does not round-trip,
    // because the rendered template omits the origins and re-encodes to a
    // different card. Rendering them inline means threading the resolved
    // origins through the whole normative renderer and rewriting every vector's
    // `.template`, which is a design change and is still filed under F-219.
    emit_origins_stderr(&descriptor, &mut std::io::stderr());
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::Template,
        &mut std::io::stderr(),
    );
    if partial {
        emit_partial_stderr_note(&unres, &mut std::io::stderr());
    }
    Ok(if partial { 4 } else { 0 })
}

/// Write the per-`@N` key origins the card carries, one per line, to `w`.
///
/// Silent when there is nothing to say — a single-key card whose origin is the
/// canonical default, or a descriptor whose keys cannot be expanded, gets no
/// note. A line per invocation that never varies is a line people stop reading.
fn emit_origins_stderr(d: &md_codec::encode::Descriptor, w: &mut impl std::io::Write) {
    use std::fmt::Write as _;
    let Ok(expanded) = md_codec::canonicalize::expand_per_at_n(d) else {
        return;
    };
    if expanded.is_empty() {
        return;
    }
    let mut out =
        String::from("note: key origins carried by this card (not shown in the template):\n");
    for e in &expanded {
        let mut path = String::from("m");
        for c in &e.origin_path.components {
            let _ = write!(path, "/{}{}", c.value, if c.hardened { "'" } else { "" });
        }
        match e.fingerprint {
            Some(fp) => {
                let mut hex = String::with_capacity(8);
                for b in &fp {
                    let _ = write!(hex, "{b:02x}");
                }
                let _ = writeln!(
                    out,
                    "  @{}: [{}/{}]",
                    e.idx,
                    hex,
                    path.trim_start_matches("m/")
                );
            }
            None => {
                let _ = writeln!(out, "  @{}: {path}", e.idx);
            }
        }
    }
    // F-595: the origin is on STDERR and the template on STDOUT, so the
    // pasteable half is missing exactly what the next verb needs --
    // `md address --template "$(md decode …)"` fails with "non-canonical
    // wrapper requires explicit origin for @0". The restoring operator has
    // plates, not the original policy file; that is the point of the backup.
    //
    // Naming the flag costs one line and was VERIFIED before being printed:
    // `md address --template <this stdout> --key @0=<xpub> --path m/84'/0'/0'`
    // returned bc1qdukhd56x6vy40sf65mrltzj0p7ydx3h0h48tlhv9kge8n84dueksklvjez.
    //
    // Only when every @N shares ONE origin: `--path` takes a single PATH, so
    // naming it for a card with divergent per-@N origins would be prescribing a
    // remedy that cannot carry them -- the F-581 defect.
    let shared: Vec<&_> = expanded.iter().collect();
    let one_origin = shared
        .first()
        .map(|h| shared.iter().all(|e| e.origin_path == h.origin_path))
        .unwrap_or(false);
    if one_origin {
        let mut path = String::from("m");
        for c in &expanded[0].origin_path.components {
            let _ = write!(path, "/{}{}", c.value, if c.hardened { "'" } else { "" });
        }
        let _ = writeln!(
            out,
            "note: the template above does NOT carry the origin. Verbs that resolve a \
             non-canonical wrapper need it:\n      md address --template <the line above> \
             --key @i=<xpub> --path {path}"
        );
    }
    let _ = w.write_all(out.as_bytes());
}
