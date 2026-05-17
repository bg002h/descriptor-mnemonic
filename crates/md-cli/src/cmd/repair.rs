//! `md repair` — BCH error-correction for md1 strings (multi-chunk).
//!
//! Realizes plan §2.B.3 (v0.22.x follow-ups Tranche B.6). Wraps
//! `md_codec::decode_with_correction` (which performs full BCH correction
//! up to t=4 per chunk) and renders a per-chunk repair report.
//!
//! Multi-chunk atomic semantics (plan §1 D28):
//!   - If ANY chunk fails BCH capacity (> 4 errors), the WHOLE call fails
//!     with exit 2 + the failing chunk index named on stderr.
//!   - NO partial corrected chunks are emitted on stdout in the atomic-fail
//!     case (md_codec::decode_with_correction is itself atomic; this CLI
//!     does not emit until the call returns successfully).
//!
//! Exit codes (D26 cross-CLI parity with `mk repair` / `ms repair` /
//! `mnemonic repair`):
//!   - 0 — every input chunk was already valid (no corrections applied)
//!   - 5 — at least one chunk had corrections applied (REPAIR_APPLIED)
//!   - 2 — atomic-fail: BCH-uncorrectable / HRP-mismatch / parse-reject;
//!         caller's named-chunk error surfaces on stderr
//!
//! Text output mirrors `mnemonic repair`'s text-form report shape (see
//! `mnemonic-toolkit/src/cmd/repair.rs::emit_repair_text`). JSON output
//! byte-matches the toolkit's standalone `RepairJson` schema (D27 — fields
//! `schema_version`, `kind`, `corrected_chunks`, `repairs`) so cross-CLI
//! parsers reuse the same struct.

use clap::Args;

use crate::error::CliError;

/// Codex32 alphabet — mirrors `md_codec::chunk::CODEX32_ALPHABET` (which
/// is module-private). Needed for the `(was, now)` char rendering in the
/// repair report. Stable per BIP 173.
const CODEX32_ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// `md repair` arguments.
#[derive(Args, Debug)]
pub struct RepairArgs {
    /// One or more md1 strings to attempt to repair. Use `-` to read
    /// one string per line from stdin. Multi-chunk semantics are
    /// atomic per plan §1 D28 — ANY failing chunk aborts the call.
    #[arg(required = true, num_args = 1..)]
    pub md1_strings: Vec<String>,

    /// Emit a single JSON envelope on stdout instead of the text-form
    /// report. Schema byte-matches `mnemonic repair --json`'s
    /// `RepairJson` shape (cross-CLI parser reuse).
    #[arg(long)]
    pub json: bool,
}

/// Per-chunk repair report. Mirrors toolkit's `RepairDetail` shape so
/// JSON output is byte-identical to `mnemonic repair --json`.
#[derive(Debug, Clone)]
struct RepairDetail {
    chunk_index: usize,
    /// Only consumed by `emit_json` (cfg = json) — the text-form report
    /// reconstructs the original chunk on-the-fly from chunk_index +
    /// corrected_positions. `#[allow(dead_code)]` keeps no-default-features
    /// builds warning-free.
    #[allow(dead_code)]
    original_chunk: String,
    corrected_chunk: String,
    /// `(position, was, now)` — `position` is 0-indexed into the data-part
    /// (chars after the `md1` HRP).
    corrected_positions: Vec<(usize, char, char)>,
}

/// Read a list of md1 strings: positional `args` minus a leading `"-"`,
/// which means "read one string per line from stdin." `"-"` may appear
/// as any positional value but is processed once across the list.
/// Mirrors mk-cli's `read_mk1_strings` helper (cross-CLI parity).
fn read_md1_strings(args: &[String]) -> Result<Vec<String>, CliError> {
    let mut out = Vec::with_capacity(args.len());
    let mut consumed_stdin = false;
    for a in args {
        if a == "-" && !consumed_stdin {
            consumed_stdin = true;
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .map_err(|e| CliError::BadArg(format!("stdin read: {e}")))?;
            for line in buf.lines() {
                let s = line.trim();
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
        } else if a == "-" {
            // Already consumed stdin; ignore additional `-` markers.
        } else {
            out.push(a.clone());
        }
    }
    if out.is_empty() {
        return Err(CliError::BadArg(
            "expected at least one md1 string (positional or via stdin with '-')".into(),
        ));
    }
    Ok(out)
}

/// Run `md repair`.
///
/// Returns an `Ok(u8)` exit code per D26. On atomic-fail (any md_codec
/// error from `decode_with_correction`), prints the error message to
/// stderr and returns `Ok(2)` — bypassing the `CliError::Codec → 1`
/// default route so the repair exit-code contract is honored.
pub fn run(args: RepairArgs) -> Result<u8, CliError> {
    let strings = read_md1_strings(&args.md1_strings)?;

    // Atomic per D28: decode_with_correction either succeeds for ALL
    // chunks or returns Err naming the first failing chunk. We do NOT
    // emit any partial output on stdout in the Err branch.
    let str_refs: Vec<&str> = strings.iter().map(String::as_str).collect();
    let (_descriptor, details) = match md_codec::decode_with_correction(&str_refs) {
        Ok(t) => t,
        Err(e) => {
            // Surface the codec error on stderr (with the chunk_index
            // named when present). NO stdout output per D28.
            eprintln!("md: repair: {e}");
            return Ok(2);
        }
    };

    // Reconstruct per-chunk corrected output + the (was, now) char view.
    // `details` is aggregated across all chunks; group by chunk_index.
    let mut reports: Vec<RepairDetail> = Vec::with_capacity(strings.len());
    for (idx, original) in strings.iter().enumerate() {
        let mut positions: Vec<(usize, char, char)> = details
            .iter()
            .filter(|d| d.chunk_index == idx)
            .map(|d| (d.position, d.was, d.now))
            .collect();
        positions.sort_by_key(|(p, _, _)| *p);
        let corrected = apply_corrections(original, &positions);
        reports.push(RepairDetail {
            chunk_index: idx,
            original_chunk: original.clone(),
            corrected_chunk: corrected,
            corrected_positions: positions,
        });
    }

    let any_correction = reports.iter().any(|r| !r.corrected_positions.is_empty());
    let corrected_chunks: Vec<String> =
        reports.iter().map(|r| r.corrected_chunk.clone()).collect();

    if args.json {
        emit_json(&corrected_chunks, &reports)?;
    } else {
        emit_text(&corrected_chunks, &reports);
    }

    Ok(if any_correction { 5 } else { 0 })
}

/// Apply the (position, was, now) corrections to an md1 chunk string,
/// returning the corrected string. Position is 0-indexed into the
/// data-part (post-`md1` HRP). Out-of-range positions (defensive only —
/// md_codec's CorrectionDetail.position is bounded by data-part length)
/// are silently skipped.
fn apply_corrections(original: &str, positions: &[(usize, char, char)]) -> String {
    let hrp_len = 3; // "md1"
    let mut chars: Vec<char> = original.chars().collect();
    for &(pos, _was, now) in positions {
        let abs_idx = hrp_len + pos;
        if abs_idx < chars.len() {
            chars[abs_idx] = now;
        }
    }
    chars.iter().collect()
}

/// Text-form report: `# Repair report` header (only if any chunk had
/// corrections), per-chunk correction lines, then corrected chunks one
/// per line. Mirrors toolkit's `cmd::repair::emit_repair_text` shape
/// byte-exact (modulo the `md1`-only `kind_str`).
fn emit_text(corrected_chunks: &[String], reports: &[RepairDetail]) {
    let any_correction = reports.iter().any(|r| !r.corrected_positions.is_empty());
    if any_correction {
        println!("# Repair report");
        for r in reports {
            if r.corrected_positions.is_empty() {
                continue;
            }
            let n = r.corrected_positions.len();
            let plural = if n == 1 { "correction" } else { "corrections" };
            let mut line = format!("#   md1 chunk {}: {} {} at ", r.chunk_index, n, plural);
            for (i, (pos, was, now)) in r.corrected_positions.iter().enumerate() {
                if i > 0 {
                    line.push_str(", ");
                }
                line.push_str(&format!("position {pos}: '{was}' -> '{now}'"));
            }
            println!("{line}");
        }
    }
    for chunk in corrected_chunks {
        println!("{chunk}");
    }
    // Suppress unused-const warning when feature = "json" is off.
    let _ = CODEX32_ALPHABET;
}

// JSON envelope — schema MUST byte-match toolkit's standalone `RepairJson`
// at `mnemonic-toolkit/src/cmd/repair.rs:162-183` (D27 cross-CLI parser
// reuse). Field order is part of the schema (serde preserves struct field
// order in the default JSON serializer).
#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct RepairJson<'a> {
    schema_version: &'static str,
    kind: &'static str,
    corrected_chunks: &'a [String],
    repairs: Vec<RepairJsonDetail<'a>>,
}

#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct RepairJsonDetail<'a> {
    chunk_index: usize,
    original_chunk: &'a str,
    corrected_chunk: &'a str,
    corrected_positions: Vec<RepairJsonPosition>,
}

#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct RepairJsonPosition {
    position: usize,
    was: String,
    now: String,
}

#[cfg(feature = "json")]
fn emit_json(corrected_chunks: &[String], reports: &[RepairDetail]) -> Result<(), CliError> {
    let envelope = RepairJson {
        schema_version: "1",
        kind: "md1",
        corrected_chunks,
        repairs: reports
            .iter()
            // Mirror toolkit: only include entries for chunks that
            // actually had corrections applied.
            .filter(|r| !r.corrected_positions.is_empty())
            .map(|r| RepairJsonDetail {
                chunk_index: r.chunk_index,
                original_chunk: &r.original_chunk,
                corrected_chunk: &r.corrected_chunk,
                corrected_positions: r
                    .corrected_positions
                    .iter()
                    .map(|(p, w, n)| RepairJsonPosition {
                        position: *p,
                        was: w.to_string(),
                        now: n.to_string(),
                    })
                    .collect(),
            })
            .collect(),
    };
    let body = serde_json::to_string(&envelope)
        .map_err(|e| CliError::BadArg(format!("repair JSON serialize: {e}")))?;
    println!("{body}");
    Ok(())
}

#[cfg(not(feature = "json"))]
fn emit_json(_corrected_chunks: &[String], _reports: &[RepairDetail]) -> Result<(), CliError> {
    Err(CliError::BadArg(
        "--json requires the `json` feature (rebuild with --features json)".into(),
    ))
}
