//! The seating engine (P2/A1-A5, B1-B2).
//!
//! C2 lands the engine under this doc comment; C0 landed the comment alone,
//! embedding plan section 1's
//! goal-and-gaps matrix per the operator's MATRIX-TRAVELS directive
//! (2026-08-30): the table below is byte-identical to
//! `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` section 1, and
//! to `design/SPEC_wallet_form_converter.md`'s copy.
//!
//! ## Where each normative rule lives
//!
//! | SPEC rule | module |
//! | --- | --- |
//! | P2 input pipeline (A3(a): dedupe → group → reassemble) | [`input`] |
//! | A2 satisfaction, the two door checks, the two card-set checks | [`satisfy`] |
//! | A3 matchings + compose-canonicalise-compare + cap + tie-break | [`matching`] |
//! | A4 completeness (unfilled slots, leftover cards) | [`complete`] |
//! | A5 `--seat '@i=<chunk-set-id>'` | [`directive`] |
//! | B1 stub disposition, B2 oracles | [`disposition`] |
//! | composition, the comparison form, the spend-equality checker | [`compose`] |
//!
//! ## 1. The surface: one matrix
//!
//! **THE MATRIX TRAVELS (operator directive, 2026-08-30): this table is
//! the cycle's goal-and-gaps statement and is embedded, cells kept
//! current, in EVERY artifact — brainstorm, spec, this plan, and the
//! seating engine's module doc comment in the code. A document or module
//! missing it is incomplete.**
//!
//! Input forms (any COMPLETE wallet expression):
//!
//! - **D** — concrete descriptor (miniscript or plain), keys + origins inline
//! - **T** — BIP-388 template + per-slot keys/origins as flags
//! - **S** — the split card set: keyless md1 phrases + mk1 strings
//! - **K** — keyed md1 phrases (Pubkeys TLV)
//!
//! Output forms: concrete descriptor · addresses · keyed card (via the
//! existing `md encode --key` bridge) · template + origin-notated key lines.
//!
//! | in \ out | concrete descriptor | addresses | keyed card | keyless + mk1 cards |
//! | --- | --- | --- | --- | --- |
//! | **D** concrete descriptor | — | ✗ P3 | ✗ P3+bridge | ✗ P3 (the decomposer) |
//! | **T** template + key flags | ⚠ P1 (flag-form gap; inline template origins already work — r1 I8) | ⚠ P1 | ✓ `md encode --key` (Divergent) | ✓ |
//! | **S** keyless card + mk1 strings | ✗ P2 (the seating engine) | ✗ P2 | ✗ P2+bridge | — |
//! | **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |
//!
//! ✓ measured working; ⚠/✗ the gaps, tagged with the piece that closes
//! them. On C4 close, the ⚠/✗ cells this cycle owns flip to ✓ in every
//! embedded copy in the same commit as the acceptance walk that proves
//! them.

pub mod complete;
pub mod compose;
pub mod directive;
pub mod disposition;
pub mod input;
pub mod matching;
pub mod satisfy;

use crate::error::CliError;
use md_codec::encode::Descriptor;

/// Everything `md descriptor` / `md address` hand the engine.
pub struct SeatingRequest<'a> {
    /// The keyless policy card's md1 strings (the command's positionals).
    pub phrases: &'a [String],
    /// mk1 key-card strings, from `--from-mk1` and `--from-mk1-file`
    /// together.
    pub from_mk1: &'a [String],
    /// Raw `--seat '@i=<chunk-set-id>'` values, unparsed.
    pub seats: &'a [String],
    /// Network, for B2's address-0 note.
    pub network: bitcoin::Network,
    /// Names the calling subcommand so a refusal reads in the operator's
    /// terms.
    pub cmd: &'static str,
}

/// A successful seating: the composed descriptor, plus the PHASE B notes
/// that belong on stderr.
pub struct Seating {
    /// The composed wallet policy — stdout's machine contract.
    pub descriptor: Descriptor,
    /// B1 dispositions then B2's address-0 instruction, in order.
    pub notes: Vec<String>,
}

/// Run the whole engine, in PHASE order.
///
/// PHASE A runs before any assignment is chosen; PHASE B after a total
/// assignment exists. Within PHASE A the order is itself normative — the
/// input pipeline first (A3(a)), then the checks that can be answered from
/// the DECLARATION alone, then the ones that need the card set, then A2's
/// graph, then A5's pins, then A3's decision. Each of those refusals is
/// accurate only where it sits: deferred past A3, V-IMPOSS would surface as
/// a leftover-card message about the wrong thing.
///
/// A1's triage is not a separate pass. It "records, never refuses alone"
/// (SPEC A1) and its only consumer is B1's shape tier, which recomputes the
/// template id from the policy — so carrying a triage result across the
/// phases would be a second copy of a value B1 already has.
pub fn run(req: &SeatingRequest<'_>) -> Result<Seating, CliError> {
    // Step 1 of the pipeline applies to the md1 side too: a policy card
    // scanned twice is one card.
    let phrases = input::dedupe_strings(&crate::cmd::strip_md1_inputs(req.phrases));
    let refs: Vec<&str> = phrases.iter().map(String::as_str).collect();
    let policy = if refs.len() == 1 {
        md_codec::decode_md1_string(refs[0])?
    } else {
        md_codec::reassemble(&refs)?
    };
    if policy.is_wallet_policy() {
        return Err(CliError::Seat(format!(
            "{} --from-mk1 seats key cards into a KEYLESS policy card, but these md1 \
             phrases already carry their keys (Pubkeys TLV). Drop --from-mk1 to render \
             this card directly.",
            req.cmd
        )));
    }

    let cards = input::decode_cards(req.from_mk1)?;

    // Declaration-only door checks.
    satisfy::check_no_repeated_placeholder(&policy)?;
    let decls = satisfy::slot_declarations(&policy)?;
    satisfy::check_no_identical_fp_bearing_declarations(&decls)?;

    // Card-set checks.
    satisfy::check_no_impossible_card_pair(&cards)?;
    satisfy::check_no_repeated_xpub(&cards)?;

    // A2, A5, A3.
    let per_slot = matching::candidates(&decls, &cards);
    let directives: Vec<directive::SeatDirective> = req
        .seats
        .iter()
        .map(|s| directive::parse(s))
        .collect::<Result<_, _>>()?;
    let per_slot = directive::apply(&directives, &decls, &cards, &per_slot)?;
    let assignment = match matching::decide(&policy, &decls, &cards, &per_slot)? {
        matching::Outcome::Seated(a) => a,
        matching::Outcome::NoPerfectMatching => {
            return Err(complete::refusal(&decls, &cards, &per_slot));
        }
    };

    // PHASE B.
    let descriptor = compose::compose(&policy, &cards, &assignment)?;
    let mut notes = disposition::notes(&policy, &descriptor, &cards)?;
    notes.push(disposition::address_zero_note(&descriptor, req.network)?);
    Ok(Seating { descriptor, notes })
}

/// Read one mk1 string per line from a file, for `--from-mk1-file`.
///
/// Blank lines and `#` comments are skipped, following `mk`'s own
/// `--from-md1-set` reader, so a card file carrying provenance works. Any
/// OTHER non-mk1 line refuses by name rather than being silently dropped —
/// a typo'd or truncated line is exactly the input a restore must not
/// quietly ignore.
pub fn read_mk1_file(path: &std::path::Path) -> Result<Vec<String>, CliError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CliError::Seat(format!("--from-mk1-file {}: {e}", path.display())))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let stripped: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if !stripped.starts_with("mk1") {
            return Err(CliError::Seat(format!(
                "--from-mk1-file {}: line {} is not an mk1 string (it starts `{}`). \
                 Blank lines and `#` comments are skipped; anything else is refused \
                 rather than ignored.",
                path.display(),
                n + 1,
                &trimmed.chars().take(12).collect::<String>()
            )));
        }
        out.push(stripped);
    }
    if out.is_empty() {
        return Err(CliError::Seat(format!(
            "--from-mk1-file {}: no mk1 strings in this file.",
            path.display()
        )));
    }
    Ok(out)
}
