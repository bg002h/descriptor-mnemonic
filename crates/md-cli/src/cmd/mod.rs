use crate::error::CliError;
use md_codec::encode::strip_display_separators;

/// Strip mstring display separators (SPEC §3.2) from each md1 input string so a
/// grouped or unbroken card both re-ingest. Applied at every md1-intake site
/// that has no stdin channel — `md address` and `md descriptor`, whose
/// positionals are `phrases` rather than the artifact positional §6b binds.
pub fn strip_md1_inputs(strings: &[String]) -> Vec<String> {
    strings
        .iter()
        .map(|s| strip_display_separators(s))
        .collect()
}

/// Read a list of md1 strings: positional `args` minus a leading `"-"`,
/// which means "read one string per line from stdin." `"-"` may appear
/// as any positional value but is processed once across the list.
/// Mirrors mk-cli's `read_mk1_strings` helper (cross-CLI parity).
///
/// **P3 §6b — hoisted here from `cmd::repair` and made `pub`.** It had one
/// caller and `md repair` was the only verb that could read stdin; `decode`,
/// `verify`, `inspect` and `bytecode` each took `-` as a LITERAL positional and
/// failed with `string does not start with HRP md1`. This is the shape `mk`
/// already has at `mk-cli`'s `cmd/mod.rs::read_mk1_strings`, which this
/// function's doc comment already claimed parity with while sitting in a
/// different module with a fifth of the reach.
///
/// It strips display separators **per line** before deciding a line is empty,
/// which is what lets a card copied off the stderr engraving card be pasted
/// straight back in. The shared crate's `records::split_record_stream` filters
/// on `trim().is_empty()` and returns the line unchanged, so it does not do
/// this — which is why P3 declines it and extends this instead.
/// P3 §6b — the md1 intake for the reading verbs, across all three channels.
///
/// `--in FILE` reads the tool's OWN input material: md1 strings, one per line.
/// It goes through the SAME per-line strip as `-` does, so a card copied off
/// the stderr engraving card re-ingests from a file exactly as it does from a
/// pipe.
///
/// `--in` and the positional are declared mutually exclusive in clap, so this
/// function never has to invent a precedence rule.
pub fn read_md1_inputs(
    args: &[String],
    in_file: Option<&std::path::Path>,
) -> Result<Vec<String>, CliError> {
    let Some(path) = in_file else {
        return read_md1_strings(args);
    };
    // The path is NAMED in the refusal. A bare "No such file or directory" out
    // of a pipeline that built the path itself is the one message an operator
    // cannot act on.
    let buf = std::fs::read_to_string(path)
        .map_err(|e| CliError::BadArg(format!("--in {}: {e}", path.display())))?;
    let mut out = Vec::new();
    for line in buf.lines() {
        let s = strip_display_separators(line);
        if !s.is_empty() {
            out.push(s);
        }
    }
    if out.is_empty() {
        return Err(CliError::BadArg(format!(
            "--in {}: no md1 strings in this file. An EMPTY file is what a FAILED \
             upstream command leaves behind -- check the command that wrote it.",
            path.display()
        )));
    }
    Ok(out)
}

/// P3 §6b — read a BIP-388 template from `--in FILE` on `md encode`.
///
/// "Own input material" is not a formality (§6b): `encode`'s input is a
/// TEMPLATE, and the reading verbs' input is md1 strings. A single `--in` that
/// meant both would be a filename implying a different artifact, which is what
/// made an earlier acceptance criterion unsatisfiable.
pub fn read_template_file(path: &std::path::Path) -> Result<String, CliError> {
    let buf = std::fs::read_to_string(path)
        .map_err(|e| CliError::BadArg(format!("--in {}: {e}", path.display())))?;
    let t = buf.trim();
    if t.is_empty() {
        return Err(CliError::BadArg(format!(
            "--in {}: no BIP-388 template in this file (it is empty or blank)",
            path.display()
        )));
    }
    Ok(t.to_string())
}

/// P3 §6b — write the artifact to `--out FILE`, created **0600**.
///
/// Through the shared crate's `write_private`, never `std::fs::write`: F-244.
/// `write` has no root re-export in `mnemonic-io-lib`, so the path is
/// module-qualified; the unqualified form is an `E0425`.
///
/// `--out` OVERWRITES (operator ruling 2026-08-26), and `write_private` sets
/// the mode a SECOND time on the open file because `OpenOptions::mode()` binds
/// on create only -- an existing 0644 target would otherwise stay 0644, which
/// is the case a re-run actually hits.
pub fn write_artifact(path: &std::path::Path, body: &str) -> Result<(), CliError> {
    mnemonic_io_lib::write::write_private(path, body.as_bytes())
        .map_err(|e| CliError::BadArg(format!("--out {}: {e}", path.display())))
}

pub fn read_md1_strings(args: &[String]) -> Result<Vec<String>, CliError> {
    let mut out = Vec::with_capacity(args.len());
    let mut consumed_stdin = false;
    for a in args {
        if a == "-" && !consumed_stdin {
            consumed_stdin = true;
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .map_err(|e| CliError::BadArg(format!("stdin read: {e}")))?;
            for line in buf.lines() {
                // mstring display-grouping (SPEC §3.2): strip separators so a
                // grouped or unbroken card both re-ingest. (repair OUTPUT stays
                // unbroken — no grouping flags on `md repair`.)
                let s = strip_display_separators(line);
                if !s.is_empty() {
                    out.push(s);
                }
            }
        } else if a == "-" {
            // Already consumed stdin; ignore additional `-` markers.
        } else {
            out.push(strip_display_separators(a));
        }
    }
    if out.is_empty() {
        return Err(CliError::BadArg(
            "expected at least one md1 string (positional or via stdin with '-')".into(),
        ));
    }
    Ok(out)
}

pub mod address;
pub mod build;
pub mod bytecode;
#[cfg(feature = "cli-compiler")]
pub mod compile;
pub mod decode;
pub mod descriptor;
pub mod encode;
pub mod gen_man;
#[cfg(feature = "json")]
pub mod gui_schema;
pub mod inspect;
pub mod partial;
pub mod repair;
pub mod vectors;
pub mod verify;
