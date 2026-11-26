//! `md repair` — BCH error-correction for md1 strings (multi-chunk).
//!
//! Realizes plan §2.B.3 (v0.22.x follow-ups Tranche B.6). Wraps
//! `md_codec::decode_with_correction` (which performs full BCH correction
//! up to t=4 per chunk) and renders a per-chunk repair report.
//!
//! Exit codes (`SPEC_constellation_exit_codes.md` §3, decisions 6 and 7;
//! D26's meanings with the new numbers):
//!   - 0  -- nothing to correct (a complete card, or an incomplete chunk set
//!     whose chunks are all clean).
//!   - 16 -- REPAIRED: a correction was applied and is verified by
//!     reassembly (a chunked set's content id), or will be when the rest of
//!     an incomplete set is reassembled (a lone chunk counts as one).
//!   - 20 -- REPAIRED + UNVERIFIED: a correction to a string that is not
//!     chunked, which has no second check (it can alias to another card).
//!   - 22 -- REPAIRED + UNVERIFIED + INVALID: a single corrected string that
//!     then does not decode (decision 6), for example a wire version this
//!     build cannot read. The corrected string is still printed.
//!   - 2  -- INVALID, nothing printed: an uncorrectable string, a wrong
//!     HRP, a clean string that does not decode, a complete set that does
//!     not reassemble, several strings at an unsupported version (ruling 7).
//!
//! An incomplete chunk set (D28 kept its atomicity for UNCORRECTABLE chunks:
//! any one of them fails the whole call) is corrected chunk by chunk and
//! printed, so a partly damaged set can be repaired before every plate is
//! found (decision 7).
//!
//! Text output mirrors `mnemonic repair`'s text-form report shape (see
//! `mnemonic-toolkit/src/cmd/repair.rs::emit_repair_text`). JSON output
//! follows the toolkit's standalone `RepairJson` schema (D27 — fields
//! `schema_version`, `kind`, `corrected_chunks`, `repairs`) so cross-CLI
//! parsers reuse the same struct. Two fields are added only when they
//! apply: `not_a_card` at 22 and `incomplete` for decision 7's set.

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
    /// are stripped per line, so a card copied in grouped form
    /// re-ingests.
    #[arg(long = "in", value_name = "FILE")]
    pub in_file: Option<std::path::PathBuf>,

    /// Emit a single JSON envelope on stdout instead of the text-form
    /// report: `mnemonic repair --json`'s `RepairJson` shape, plus
    /// `not_a_card` at exit 22 and `incomplete: true` for an incomplete set.
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

/// Run `md repair`. Returns the exit status (module docs).
pub fn run(args: RepairArgs) -> Result<u8, CliError> {
    use crate::exit::{Bits, INVALID, OK, REPAIRED, UNVERIFIED};
    // FIX_bh1119_md_out_names_input S2: the guard is the first statement.
    crate::cmd::refuse_output_names_input(
        true,
        None,
        false,
        &crate::cmd::InputName::in_or_stdin(args.in_file.as_deref(), &args.md1_strings),
    )?;
    let strings = crate::cmd::read_md1_inputs(&args.md1_strings, args.in_file.as_deref(), "--in")?;

    // BCH correction, chunk by chunk. An uncorrectable chunk fails the whole
    // call (D28): the codec error on stderr, nothing on stdout.
    let str_refs: Vec<&str> = strings.iter().map(String::as_str).collect();
    let (corrected, details) = match md_codec::correct_chunks(&str_refs) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("md: repair: {e}");
            return Ok(INVALID);
        }
    };
    let any_correction = !details.is_empty();
    let reports = build_reports(&strings, &details);
    let corrected_chunks: Vec<String> = reports.iter().map(|r| r.corrected_chunk.clone()).collect();
    let corrected_refs: Vec<&str> = corrected.iter().map(String::as_str).collect();

    // The decode check (decision 6). A single string that is not chunked
    // decodes on its own; anything chunked reassembles.
    let single_unchunked = corrected.len() == 1 && !is_chunked(&corrected[0]);
    let decoded = if single_unchunked {
        md_codec::decode::decode_md1_string(&corrected[0])
    } else {
        md_codec::chunk::reassemble(&corrected_refs)
    };
    match decoded {
        Ok(descriptor) => {
            emit(
                &corrected_chunks,
                &reports,
                args.json,
                &JsonExtra::default(),
            )?;
            // L4 (cycle-9): a wallet-policy md1 (non-empty Pubkeys TLV) is
            // WatchOnly; a keyless md1 is a Template.
            let class = if descriptor.is_wallet_policy() {
                crate::output_advisory::OutputClass::WatchOnly
            } else {
                crate::output_advisory::OutputClass::Template
            };
            crate::output_advisory::emit_output_class_advisory(class, &mut std::io::stderr());
            if !any_correction {
                Ok(OK)
            } else if single_unchunked {
                eprintln!(
                    "md: repair: UNVERIFIED: this card is a single string, so nothing checks \
                     the correction a second time, and it can alias to a different valid card. \
                     Verify the corrected card (for example `md verify <card> --template \
                     <TEMPLATE>`) before trusting it."
                );
                Ok((Bits::of(REPAIRED) | UNVERIFIED).to_exit_code())
            } else {
                Ok(REPAIRED)
            }
        }
        // Decision 7: an incomplete set (or a lone chunk of a larger one) is
        // printed chunk by chunk. Every chunk header parsed, so each string
        // decodes as a chunk of its format; the content id checks the
        // corrections when the rest of the set is reassembled.
        // Only a set short of chunks, with distinct indices inside the set,
        // is incomplete (decision 6: each string decodes as a chunk of its
        // set); a set with too many, repeated or stray chunks is 2.
        Err(md_codec::Error::ChunkSetIncomplete { got, expected }) => {
            if let Err(why) = short_set_of_distinct_chunks(&corrected, got, expected) {
                eprintln!("md: repair: {why}");
                return Ok(INVALID);
            }
            emit(
                &corrected_chunks,
                &reports,
                args.json,
                &JsonExtra::incomplete(),
            )?;
            if any_correction {
                eprintln!(
                    "md: repair: the chunk set is incomplete ({got} of {expected} chunks): each \
                     correction passed its chunk's checksum, and is verified when the rest of \
                     the set is reassembled (`md decode` with every chunk)."
                );
                Ok(REPAIRED)
            } else {
                eprintln!(
                    "md: repair: nothing to correct; the chunk set is incomplete ({got} of \
                     {expected} chunks)."
                );
                Ok(OK)
            }
        }
        // F-449 stage 2 Task 2c, ruling 7: a single string this build cannot
        // read keeps its correction (BCH is version-agnostic); several
        // strings at such a version fail whole, as no check across them can
        // establish that the version is one card's own.
        Err(md_codec::Error::WireVersionMismatch { got }) if corrected.len() == 1 => {
            if !any_correction {
                eprintln!(
                    "md: repair: {}",
                    md_codec::Error::WireVersionMismatch { got }
                );
                return Ok(INVALID);
            }
            emit(
                &corrected_chunks,
                &reports,
                args.json,
                &JsonExtra::not_a_card(&md_codec::Error::WireVersionMismatch { got }),
            )?;
            unsupported_version_notes(got);
            Ok((Bits::of(REPAIRED) | UNVERIFIED | INVALID).to_exit_code())
        }
        // Decision 6: a corrected single string that is not a card. It is
        // printed with its warning, since a person repairing by hand needs
        // to see it.
        Err(e) if single_unchunked && any_correction => {
            emit(
                &corrected_chunks,
                &reports,
                args.json,
                &JsonExtra::not_a_card(&e),
            )?;
            eprintln!(
                "md: repair: the corrected string passed its checksum but is not a card md \
                 can decode: {e}. Do not trust it."
            );
            Ok((Bits::of(REPAIRED) | UNVERIFIED | INVALID).to_exit_code())
        }
        // A clean string that does not decode, or a complete set that does
        // not reassemble: nothing printed (D28).
        Err(e) => {
            eprintln!("md: repair: {e}");
            Ok(INVALID)
        }
    }
}

/// `Ok` when `chunks` are fewer than the `expected` count of their set, and
/// their indices are distinct and below that count; otherwise what is wrong.
fn short_set_of_distinct_chunks(
    chunks: &[String],
    got: usize,
    expected: usize,
) -> Result<(), String> {
    if got > expected {
        return Err(format!(
            "{got} chunks were given, but the set holds {expected}; the set does not reassemble"
        ));
    }
    let mut seen = vec![false; expected];
    for (n, chunk) in chunks.iter().enumerate() {
        let (bytes, bits) =
            md_codec::codex32::unwrap_string(chunk).map_err(|e| format!("chunk {}: {e}", n + 1))?;
        let mut r = md_codec::bitstream::BitReader::with_bit_limit(&bytes, bits);
        let h = md_codec::chunk::ChunkHeader::read(&mut r)
            .map_err(|e| format!("chunk {}: {e}", n + 1))?;
        let index = usize::from(h.index);
        match seen.get_mut(index) {
            None => {
                return Err(format!(
                    "chunk {} claims index {index}, outside its set of {expected}",
                    n + 1
                ));
            }
            Some(true) => {
                return Err(format!(
                    "chunk {} repeats index {index}; each chunk of the set is given once",
                    n + 1
                ));
            }
            Some(slot) => *slot = true,
        }
    }
    Ok(())
}

/// The chunked flag: bit 0 of the first data symbol (after the 4-bit
/// version). Only read on a string `correct_chunks` returned, which parsed.
fn is_chunked(s: &str) -> bool {
    const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    s.get(3..4)
        .and_then(|c| {
            let c = c.as_bytes()[0].to_ascii_lowercase();
            ALPHABET.iter().position(|&b| b == c)
        })
        .is_none_or(|v| v & 1 == 1)
}

/// What `--json` adds to the D27 envelope so stdout says what the exit code
/// says (final review M-3). Both fields are absent from the 0, 16 and 20
/// envelopes of a complete card, which therefore keep their old bytes.
#[derive(Default)]
struct JsonExtra {
    /// Decision 6's 22: why the corrected string is not a card.
    not_a_card: Option<(String, String)>,
    /// Decision 7: an incomplete set or a lone chunk, verified only when the
    /// rest of the set is reassembled.
    incomplete: bool,
}

impl JsonExtra {
    fn incomplete() -> Self {
        JsonExtra {
            incomplete: true,
            ..JsonExtra::default()
        }
    }

    /// `kind` is the codec error's variant name, as ms names its own.
    fn not_a_card(e: &md_codec::Error) -> Self {
        let debug = format!("{e:?}");
        let kind = debug
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .unwrap_or_default()
            .to_string();
        JsonExtra {
            not_a_card: Some((kind, e.to_string())),
            incomplete: false,
        }
    }
}

fn emit(
    corrected_chunks: &[String],
    reports: &[RepairDetail],
    json: bool,
    extra: &JsonExtra,
) -> Result<(), CliError> {
    if json {
        emit_json(corrected_chunks, reports, extra)
    } else {
        emit_text(corrected_chunks, reports);
        Ok(())
    }
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

/// F-449 stage 2 Task 2c (SPEC §8.9's stage-2 row): a corrected card whose
/// wire version this build cannot read. The notes say where to take it.
fn unsupported_version_notes(got: u8) {
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
}

/// Apply the (position, was, now) corrections to an md1 chunk string,
/// returning the corrected string. Position is 0-indexed into the
/// data-part (post-`md1` HRP). Out-of-range positions (defensive only —
/// md_codec's CorrectionDetail.position is bounded by data-part length)
/// are silently skipped.
fn apply_corrections(original: &str, positions: &[(usize, char, char)]) -> String {
    let hrp_len = 3; // "md1"
    let mut chars: Vec<char> = original.chars().collect();
    // Whole-branch review M-9, widened by bh-md-I3: write each correction in
    // the INPUT STRING's case, not the damaged character's. md1 is
    // single-case (BIP-173); an all-uppercase card (the QR alphanumeric form)
    // given a lowercase char becomes a mixed-case string md refuses to read.
    // The case comes from the whole string because the damaged character may
    // be a digit, which has no case. Every input string starts with `md`/`MD`,
    // so its case is always determined.
    let upper = chars.iter().any(|c| c.is_ascii_uppercase());
    for &(pos, _was, now) in positions {
        let abs_idx = hrp_len + pos;
        if abs_idx < chars.len() {
            chars[abs_idx] = if upper { now.to_ascii_uppercase() } else { now };
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

// JSON envelope — toolkit's standalone `RepairJson` at
// `mnemonic-toolkit/src/cmd/repair.rs:162-183` (D27 cross-CLI parser reuse),
// byte for byte when neither optional field applies. Field order is part of
// the schema (serde preserves struct field order in the default JSON
// serializer).
#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct RepairJson<'a> {
    schema_version: &'static str,
    kind: &'static str,
    /// Present only at exit 22 (decision 6): why the corrected string is not
    /// a card. Named and shaped as ms's field; md has no `verdict`, so it
    /// follows `kind`.
    #[serde(skip_serializing_if = "Option::is_none")]
    not_a_card: Option<NotACardJson<'a>>,
    /// `true` only for an incomplete set or a lone chunk (decision 7).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    incomplete: bool,
    corrected_chunks: &'a [String],
    repairs: Vec<RepairJsonDetail<'a>>,
}

#[cfg(feature = "json")]
#[derive(serde::Serialize)]
struct NotACardJson<'a> {
    kind: &'a str,
    message: &'a str,
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
fn emit_json(
    corrected_chunks: &[String],
    reports: &[RepairDetail],
    extra: &JsonExtra,
) -> Result<(), CliError> {
    let envelope = RepairJson {
        schema_version: "1",
        kind: "md1",
        not_a_card: extra
            .not_a_card
            .as_ref()
            .map(|(kind, message)| NotACardJson { kind, message }),
        incomplete: extra.incomplete,
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
        .map_err(|e| CliError::Internal(format!("repair JSON serialize: {e}")))?;
    println!("{body}");
    Ok(())
}

#[cfg(not(feature = "json"))]
fn emit_json(
    _corrected_chunks: &[String],
    _reports: &[RepairDetail],
    _extra: &JsonExtra,
) -> Result<(), CliError> {
    Err(CliError::Io(
        "--json requires the `json` feature (rebuild with --features json)".into(),
    ))
}
