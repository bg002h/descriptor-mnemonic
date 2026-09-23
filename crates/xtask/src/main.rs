//! `cargo xtask verdicts [--check]` — build md-codec's coordinator verdict
//! table (`crates/md-codec/src/coordinator/table.rs`) from the vendored
//! evidence (`crates/md-codec/tests/fixtures/coordinator/evidence.jsonl`).
//!
//! This is design §5 step 1's table generator. It parses JSON and writes a
//! file; everything that decides a verdict — keying, the rules, D1-D5 — is
//! md-codec's own `coordinator::build`, so there is one implementation of the
//! key and one of the rules. `--check` writes nothing and exits 1 when the
//! committed table is not what the evidence builds -- a local convenience.
//! THE GATE is `table_is_fresh` below, which runs the same `generate` inside
//! `cargo test`; CI never calls `--check`, and no test exercises `main`.

use std::path::PathBuf;
use std::process::ExitCode;

use md_codec::coordinator::Form;
use md_codec::coordinator::build::{
    Disagreement, EvidenceRow, MeasuredOutcome, OwnedDescription, build, render_table,
};

const EVIDENCE: &str = "crates/md-codec/tests/fixtures/coordinator/evidence.jsonl";
const TABLE: &str = "crates/md-codec/src/coordinator/table.rs";

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn field<'a>(v: &'a serde_json::Value, k: &str, line: usize) -> Result<&'a str, String> {
    v[k].as_str()
        .ok_or_else(|| format!("{EVIDENCE}:{line}: missing string field {k:?}"))
}

fn description(v: &serde_json::Value) -> Option<OwnedDescription> {
    if v.is_null() {
        return None;
    }
    let threshold = v["threshold"].as_array().and_then(|a| {
        let k = u8::try_from(a.first()?.as_u64()?).ok()?;
        let n = u8::try_from(a.get(1)?.as_u64()?).ok()?;
        Some((k, n))
    });
    Some(OwnedDescription {
        wallet_kind: v["wallet_kind"].as_str().unwrap_or("").to_string(),
        threshold,
    })
}

/// Parse the vendored JSONL into rows.
fn parse(text: &str) -> Result<Vec<EvidenceRow>, String> {
    let mut rows = Vec::new();
    for (i, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let n = i + 1;
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("{EVIDENCE}:{n}: {e}"))?;
        let form = match field(&v, "form", n)? {
            "multipath" => Form::Multipath,
            "chain0" => Form::Chain0,
            "chain1" => Form::Chain1,
            other => return Err(format!("{EVIDENCE}:{n}: unknown form {other:?}")),
        };
        let o = &v["outcome"];
        let outcome = if let Some(m) = o["refused"].as_str() {
            MeasuredOutcome::Refused(m.to_string())
        } else if o.get("imported").is_some() {
            MeasuredOutcome::Imported(description(&o["imported"]))
        } else {
            return Err(format!(
                "{EVIDENCE}:{n}: outcome is neither imported nor refused"
            ));
        };
        rows.push(EvidenceRow {
            id: field(&v, "id", n)?.to_string(),
            coordinator: field(&v, "coordinator", n)?.to_string(),
            version: field(&v, "version", n)?.to_string(),
            library_rev: field(&v, "library_rev", n)?.to_string(),
            renderer_tool: field(&v, "renderer_tool", n)?.to_string(),
            renderer_version: field(&v, "renderer_version", n)?.to_string(),
            form,
            measured_at: field(&v, "measured_at", n)?.to_string(),
            descriptor: field(&v, "descriptor", n)?.to_string(),
            outcome,
            unkeyable: v["unkeyable"].as_str().map(str::to_string),
        });
    }
    Ok(rows)
}

/// The table the committed evidence builds, or why it cannot be built.
fn generate() -> Result<(String, String), String> {
    let text = std::fs::read_to_string(root().join(EVIDENCE))
        .map_err(|e| format!("read {EVIDENCE}: {e}"))?;
    let rows = parse(&text)?;
    match build(&rows) {
        Ok(built) => Ok((
            render_table(&built.cells, EVIDENCE),
            format!(
                "{} rows: {} cells stored, {} refusals the rules explain, {} unkeyable",
                rows.len(),
                built.cells.len(),
                built.agreed_refusals,
                built.unkeyable
            ),
        )),
        Err(bad) => {
            let mut msg = format!(
                "{} disagreement(s) between rules and evidence:\n",
                bad.len()
            );
            for d in &bad {
                msg.push_str(&format!("  {}\n", describe(d)));
            }
            Err(msg)
        }
    }
}

fn describe(d: &Disagreement) -> String {
    match d {
        Disagreement::D1FalseRefusal { row, class } => {
            format!("D1 false-refusal {row}: rule refuses ({class}), evidence imported")
        }
        Disagreement::D2MissedRefusal { row, message } => {
            format!("D2 missed-refusal {row}: rule admits, evidence refused: {message}")
        }
        Disagreement::D3ReasonDrift {
            row,
            rule,
            evidence,
        } => format!("D3 reason-drift {row}: rule says {rule}, message says {evidence}"),
        Disagreement::D4Orphan { row } => format!("D4 orphan {row}: no rule set spans it"),
        Disagreement::D5EvidenceConflict { rows } => {
            format!("D5 evidence-conflict {} vs {}", rows.0, rows.1)
        }
        Disagreement::Keying { row, detail } => format!("keying {row}: {detail}"),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["verdicts"] => false,
        ["verdicts", "--check"] => true,
        _ => {
            eprintln!("usage: cargo xtask verdicts [--check]");
            return ExitCode::from(2);
        }
    };
    let (table, summary) = match generate() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("xtask verdicts: {e}");
            return ExitCode::FAILURE;
        }
    };
    let path = root().join(TABLE);
    if check {
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        if committed != table {
            eprintln!("xtask verdicts: {TABLE} is stale; run `cargo xtask verdicts`");
            return ExitCode::FAILURE;
        }
        eprintln!("xtask verdicts: fresh ({summary})");
        return ExitCode::SUCCESS;
    }
    if let Err(e) = std::fs::write(&path, table) {
        eprintln!("xtask verdicts: write {TABLE}: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("xtask verdicts: wrote {TABLE} ({summary})");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed table is exactly what the committed evidence builds —
    /// so an evidence change without a regenerated table, a rule change that
    /// moves a cell, or a hand edit to `table.rs` is red in `cargo test`, not
    /// only in `--check`. Mutation: delete any one `Cell` line from
    /// `table.rs` -> red.
    #[test]
    fn table_is_fresh() {
        let (table, _) = generate().unwrap_or_else(|e| panic!("{e}"));
        let committed = std::fs::read_to_string(root().join(TABLE)).unwrap();
        assert!(
            committed == table,
            "{TABLE} is stale; run `cargo xtask verdicts`"
        );
    }
}
