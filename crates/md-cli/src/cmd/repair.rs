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
//!     caller's named-chunk error surfaces on stderr
//!
//! DIVERGENCE, not parity (F-449 stage 2 Task 2c, SPEC §8.9): `md repair`
//! exits 5 on a corrected SINGLE-STRING card whose wire version this build
//! does not support (a multi-string set at such a version still exits 2,
//! ruling 7), while `mnemonic repair` exits 2 on it until the toolkit adopts
//! `md_codec::correct_chunks` (F-642, owned by the toolkit's post-stage-2
//! pin bump). The code -> meaning mapping is unchanged.
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
    #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
    pub md1_strings: Vec<String>,

    /// P3 §6b — read md1 strings from FILE, one per line. Display separators
    /// are stripped per line, so a card copied off an engraving card
    /// re-ingests.
    #[arg(long = "in", value_name = "FILE")]
    pub in_file: Option<std::path::PathBuf>,

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

/// Run `md repair`.
///
/// Returns an `Ok(u8)` exit code per D26. On atomic-fail (any md_codec
/// error from `decode_with_correction`), prints the error message to
/// stderr and returns `Ok(2)` — bypassing the `CliError::Codec → 1`
/// default route so the repair exit-code contract is honored.
pub fn run(args: RepairArgs) -> Result<u8, CliError> {
    let strings = crate::cmd::read_md1_inputs(&args.md1_strings, args.in_file.as_deref(), "--in")?;

    // Atomic per D28: decode_with_correction either succeeds for ALL
    // chunks or returns Err naming the first failing chunk. We do NOT
    // emit any partial output on stdout in the Err branch.
    let str_refs: Vec<&str> = strings.iter().map(String::as_str).collect();
    let (descriptor, details) = match md_codec::decode_with_correction(&str_refs) {
        Ok(t) => t,
        Err(md_codec::Error::WireVersionMismatch { got }) => {
            return corrected_but_unsupported(&strings, &str_refs, got, args.json);
        }
        Err(e) => {
            // Surface the codec error on stderr (with the chunk_index
            // named when present). NO stdout output per D28.
            eprintln!("md: repair: {e}");
            return Ok(2);
        }
    };

    let reports = build_reports(&strings, &details);
    let any_correction = reports.iter().any(|r| !r.corrected_positions.is_empty());
    let corrected_chunks: Vec<String> = reports.iter().map(|r| r.corrected_chunk.clone()).collect();

    if args.json {
        emit_json(&corrected_chunks, &reports)?;
    } else {
        emit_text(&corrected_chunks, &reports);
    }

    // L4 (cycle-9): branch the output class on whether the decoded md1 carries
    // watch-only key material. A wallet-policy md1 (non-empty Pubkeys TLV) is
    // WatchOnly; a keyless md1 is a Template. Previously labeled Template
    // unconditionally, mislabeling watch-only cards.
    let class = if descriptor.is_wallet_policy() {
        crate::output_advisory::OutputClass::WatchOnly
    } else {
        crate::output_advisory::OutputClass::Template
    };
    crate::output_advisory::emit_output_class_advisory(class, &mut std::io::stderr());
    Ok(if any_correction { 5 } else { 0 })
}

/// Reconstruct per-chunk corrected output + the (was, now) char view.
/// `details` is aggregated across all chunks; group by chunk_index.
fn build_reports(strings: &[String], details: &[md_codec::CorrectionDetail]) -> Vec<RepairDetail> {
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
    reports
}

/// F-449 stage 2 Task 2c (SPEC §8.9's stage-2 row): the decode refused the
/// card's WIRE VERSION, but BCH correction is version-agnostic and may have
/// repaired it. Keep that correction rather than discard it.
///
/// - Corrections found → emit the ordinary text/JSON report (D27 shape; it
///   needs no descriptor) and exit **5**: a correction WAS applied, 5 is
///   already distinct from 2, and no code is minted. D28 holds: this is not
///   the atomic-fail case, and stdout carries complete, BCH-valid strings.
///   **No output-class advisory** -- there is no `Descriptor` to classify,
///   and guessing is the L4 mislabel.
/// - No corrections, or correction itself fails → today's behaviour exactly:
///   the codec error on stderr, empty stdout, exit **2**. Never the success
///   path's `any_correction ? 5 : 0` (R3 M-6): a CLEAN card at an
///   unsupported version is not "already valid".
fn corrected_but_unsupported(
    strings: &[String],
    str_refs: &[&str],
    got: u8,
    json: bool,
) -> Result<u8, CliError> {
    // RULING 7 (F-449 stage 2 re-review NEW-1): SINGLE-STRING input only.
    // A build cannot know the chunk-header layout of a version it does not
    // support, so nothing it checks across a multi-string set can establish
    // that the mismatched version is one card's own: a single-string card in
    // the set reads as a bit-shifted "version" (I-1), and two genuine chunks
    // from UNRELATED wallets pass any per-string check (NEW-1). Any
    // multi-string call that fails on the version exits 2 with empty
    // stdout, exactly as 0.18.0 did.
    if strings.len() != 1 {
        eprintln!(
            "md: repair: {}",
            md_codec::Error::WireVersionMismatch { got }
        );
        return Ok(2);
    }
    let details = match md_codec::correct_chunks(str_refs) {
        Ok((_, details)) if !details.is_empty() => details,
        _ => {
            eprintln!(
                "md: repair: {}",
                md_codec::Error::WireVersionMismatch { got }
            );
            return Ok(2);
        }
    };
    let reports = build_reports(strings, &details);
    let corrected_chunks: Vec<String> = reports.iter().map(|r| r.corrected_chunk.clone()).collect();
    if json {
        emit_json(&corrected_chunks, &reports)?;
    } else {
        emit_text(&corrected_chunks, &reports);
    }
    let accepted: Vec<String> = (0..=u8::MAX)
        .filter(|v| md_codec::header::Header::is_supported_version(*v))
        .map(|v| v.to_string())
        .collect();
    eprintln!(
        "md: repair: corrected, but this build cannot read wire version {got} (accepted: {})",
        accepted.join(", ")
    );
    // Worded on `got` (R4 M-1): pre-v0.30 cards share the HRP and
    // `MD_REGULAR_CONST`, so they pass BCH and land here too, and no newer md
    // reads version 0 or 2. Wire versions have been even since v0.30.
    let newest = md_codec::header::Header::WF_UNSPENDABLE_VERSION;
    if got % 2 == 0 && got > newest {
        eprintln!(
            "md: repair: take the corrected card to a newer md, which may read wire version {got}"
        );
    } else {
        eprintln!(
            "md: repair: this looks like a pre-v0.30 or misread card; no md release reads wire \
             version {got}"
        );
    }
    Ok(5)
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
            // Whole-branch review M-9: write the correction in the INPUT's
            // case. md1 is single-case (BIP-173); an all-uppercase card (the
            // QR alphanumeric form) given a lowercase char becomes a
            // mixed-case string md refuses to read.
            chars[abs_idx] = if chars[abs_idx].is_ascii_uppercase() {
                now.to_ascii_uppercase()
            } else {
                now
            };
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
