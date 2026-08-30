//! `md decompose` — CLI surface for the D row (SPEC P3; plan §3 C3).
//!
//! stdout is the machine contract; notes and advisories go to stderr, the
//! convention every other verb here follows.

use crate::decompose::{Decomposition, decompose};
use crate::error::CliError;

/// What to print. The default prints the three artifacts SPEC P3 names —
/// template, key lines, fingerprint flags — under `#` headers; the single-
/// section forms exist so each can be redirected straight into the file the
/// next command wants (`--emit keys > keys.txt` is a `mk encode --keys` file).
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Emit {
    /// Template, key lines and fingerprint flags, under `#` headers.
    All,
    /// The keyless BIP-388 template alone, one line, no checksum.
    Template,
    /// The origin-notated key records alone — an `mk encode --keys` file.
    Keys,
    /// The `--fingerprint @i=HEX` flags alone.
    Fingerprints,
    /// The canonicalised concrete descriptor with a recomputed checksum.
    Descriptor,
    /// Runnable `md encode` / `mk encode` command lines for both mint routes.
    Commands,
}

pub struct DecomposeArgs<'a> {
    pub descriptors: &'a [String],
    pub in_file: Option<&'a std::path::Path>,
    pub emit: Emit,
    pub network: bitcoin::Network,
}

pub fn run(a: DecomposeArgs<'_>) -> Result<u8, CliError> {
    let raw = match a.in_file {
        Some(p) => crate::decompose::read_descriptor_file(p)?,
        None => a.descriptors.to_vec(),
    };
    let d = decompose(&raw, a.network)?;

    match a.emit {
        Emit::Template => println!("{}", d.template),
        Emit::Descriptor => println!("{}", d.descriptor),
        Emit::Keys => {
            for line in key_block(&d) {
                println!("{line}");
            }
        }
        Emit::Fingerprints => {
            for f in d.fingerprint_flags() {
                println!("{f}");
            }
        }
        Emit::All => {
            println!("# template — `md encode <TEMPLATE>` / `md descriptor --template`");
            println!("{}", d.template);
            println!("# keys — BIP-380 origin notation, one record per line (`mk encode --keys`)");
            for line in key_block(&d) {
                println!("{line}");
            }
            println!("# fingerprints — per-slot flags for `md encode` / `md descriptor`");
            for f in d.fingerprint_flags() {
                println!("{f}");
            }
        }
        Emit::Commands => {
            for line in commands(&d)? {
                println!("{line}");
            }
        }
    }

    for n in &d.notes {
        eprintln!("note: {n}");
    }
    Ok(0)
}

/// The key records, with an origin-less one preceded by a `#` comment saying
/// it cannot be minted. mk's own reader skips `#` lines and blank lines, so
/// the comment costs the file nothing — and mk refuses the bare record itself
/// ("expected BIP-380 origin notation `[fingerprint/path]xpub`", measured),
/// which is exactly the truth the comment states in advance.
fn key_block(d: &Decomposition) -> Vec<String> {
    let mut out = Vec::new();
    for (i, o) in d.occurrences.iter().enumerate() {
        if o.origin.is_none() {
            out.push(format!(
                "# @{i} states no origin — NOT mk-mintable; `mk encode --keys` refuses this record."
            ));
        }
        out.push(o.record.clone());
    }
    out
}

/// POSIX single-quote a value so the emitted line is genuinely RUNNABLE.
///
/// Not cosmetic: every md template carries `'` as its hardened marker (SPEC
/// "Canonicalisation" — 44 apostrophes in the composed pathological
/// descriptor), and a naive `'{template}'` closes the quote at the first
/// hardened step. The `'\''` idiom — close, escaped quote, reopen — is the
/// portable way out, and `emit_commands_route1_line_actually_runs` executes
/// the result rather than eyeballing it.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// **SPEC P3 "Origin-less keys":** `--emit commands` refuses when any key
/// lacks an origin, naming the keys and the reason. The template, keys and
/// descriptor emissions still work — only the MINT instructions are withheld,
/// because they are the ones that would not run.
fn commands(d: &Decomposition) -> Result<Vec<String>, CliError> {
    let missing = d.origin_less();
    if !missing.is_empty() {
        let named = missing
            .iter()
            .map(|i| {
                let r = &d.occurrences[*i].record;
                format!("@{i} ({}…)", &r[..16.min(r.len())])
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Err(CliError::Decompose(format!(
            "--emit commands cannot be produced: {} key(s) state no origin in this descriptor \
             — {named}. An mk1 key card binds a key to the origin it was derived at BY DESIGN, \
             so a card cannot be minted for an origin the input never stated, and \
             `mk encode --keys` refuses such a record (\"expected BIP-380 origin notation \
             `[fingerprint/path]xpub`\"). The emissions that mint nothing still work: \
             `--emit template`, `--emit keys`, `--emit descriptor`. To mint, add the origin \
             to the descriptor — the wallet that produced it knows the derivation path.",
            missing.len()
        )));
    }

    let mut out = Vec::new();
    let fps: Vec<String> = d
        .occurrences
        .iter()
        .enumerate()
        .filter_map(|(i, o)| {
            o.fingerprint()
                .map(|f| format!("--fingerprint {}", sh_quote(&format!("@{i}={f}"))))
        })
        .collect();

    out.push("# ── route 1: the KEYED card — one md1 artifact carrying template + keys ──".into());
    let keys: Vec<String> = d
        .occurrences
        .iter()
        .enumerate()
        .map(|(i, o)| {
            // The key AS PARSED — SPEC P3 "Key emission is round-trip-grade".
            format!("--key {}", sh_quote(&format!("@{i}={}", o.xpub)))
        })
        .collect();
    out.push(format!(
        "md encode {} \\\n  {} \\\n  {}",
        sh_quote(&d.template),
        keys.join(" \\\n  "),
        fps.join(" \\\n  ")
    ));
    out.push(String::new());
    out.push(
        "# ── route 2: the SPLIT set — a keyless policy card + one mk1 card per key ──".into(),
    );
    out.push(format!(
        "md encode {} \\\n  {} \\\n  --out policy.md1",
        sh_quote(&d.template),
        fps.join(" \\\n  ")
    ));
    out.push("cat > keys.txt <<'MDKEYS'".into());
    out.extend(d.occurrences.iter().map(|o| o.record.clone()));
    out.push("MDKEYS".into());
    out.push("mk encode --keys keys.txt --from-md1-set policy.md1".into());
    Ok(out)
}
