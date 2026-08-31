//! `md descriptor` — emit the CONCRETE output descriptor for a wallet policy.
//!
//! Everything else this CLI prints for a wallet policy is either a *template*
//! (`@0`, `@1`, …) or an address. Neither is what a coordinator asks for when
//! it says "paste your descriptor": that is the concrete string, with real
//! xpubs, key origins, and its BIP-380 checksum.
//!
//! THE RENDERER ALREADY EXISTED. `md_codec::to_miniscript` has built these
//! since before this command — it is what every derived address goes through,
//! and what the conformance corpus stores per chain. What was missing was a way
//! for anyone but the maintainer vector exporter to see one. Worth stating
//! plainly, because the staged plan recorded the opposite ("concrete-key
//! rendering does not exist in md-codec at all") and ordered a whole stage
//! around writing it.
//!
//! MULTIPATH IS THE DEFAULT, and it is the form a coordinator wants: one
//! descriptor carrying `<0;1>` rather than two that a human has to keep in
//! step. `--chain N` collapses to a single path for the cases that need it.
//!
//! The output is rust-miniscript's `Display` — the same renderer Sparrow and
//! Nunchuk sit on — so a byte comparison against a coordinator is meaningful
//! rather than approximate.

use crate::cmd::build::{DescriptorInput, build_descriptor};
use crate::error::CliError;
use md_codec::encode::Descriptor;

/// What `--emit` puts on stdout instead of the concrete descriptor.
///
/// **N2, the S → K cell** (`design/SPEC_mdcli_mini.md`): the only value is
/// `md1`, the KEYED card minted from the seating result. It is not a second
/// route through `md encode --key` -- that bridge admits an account-level
/// xpub at depth 3 or 4, and a card composes depth-0 keys, which is what left
/// this cell ✗ for the whole converter cycle. Minting from the seating result
/// loses nothing: a keyed card's `Pubkeys` TLV is 65 bytes (chain code ‖
/// compressed point) with no depth field, so the depth the bridge wanted is
/// not part of what gets engraved.
///
/// **The flag-name reuse with `md decompose --emit` is deliberate** (SPEC N2):
/// per-verb value vocabularies, each documented in its own `--help`. This one
/// spells an OUTPUT FORM for one wallet; decompose's spells which of several
/// artifacts of one descriptor to print.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Emit {
    /// The keyed md1 card, minted from the seating result.
    Md1,
}

pub struct DescriptorArgs<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    pub path: Option<&'a str>,
    /// mk1 key-card strings (P2). Non-empty routes to the seating engine.
    pub from_mk1: &'a [String],
    /// Raw `--seat '@i=<chunk-set-id>'` values (A5).
    pub seats: &'a [String],
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    /// `None` = multipath (`<0;1>`); `Some(n)` = collapse to that chain.
    pub chain: Option<u32>,
    /// N2 — `Some(Emit::Md1)` replaces stdout's descriptor with the keyed
    /// card minted from the seating result. `None` is today's behaviour.
    pub emit: Option<Emit>,
    pub json: bool,
}

pub fn run(args: DescriptorArgs<'_>) -> Result<u8, CliError> {
    // R9 — checked before either branch below decides what to build; see
    // its own doc comment in cmd::build for why it must run first.
    crate::cmd::build::check_from_mk1_arity(
        args.phrases,
        args.from_mk1,
        args.template,
        "descriptor",
    )?;

    // N2 — `--emit md1` is admissible on the S row and nowhere else, and the
    // check runs here, before either branch below decides what to build, so
    // the refusal names the input mode rather than whatever the wrong mode
    // happened to fail at later.
    if args.emit == Some(Emit::Md1) {
        check_emit_md1_input_mode(args.template, args.from_mk1)?;
    }

    // P2 — the S row. `--from-mk1` composes a keyless policy card with mk1
    // key cards through the seating engine. stdout stays the machine
    // contract (the descriptor and nothing else); every PHASE B note and
    // the B2 address go to stderr.
    let mut seating_notes: Vec<String> = Vec::new();
    let descriptor = if args.from_mk1.is_empty() {
        if !args.seats.is_empty() {
            return Err(CliError::Seat(
                "--seat asserts which mk1 key card fills a slot, so it needs \
                 --from-mk1/--from-mk1-file cards to choose among."
                    .into(),
            ));
        }
        build_descriptor(&DescriptorInput {
            phrases: args.phrases,
            template: args.template,
            keys: args.keys,
            fingerprints: args.fingerprints,
            path: args.path,
            network: args.network,
            cmd: "descriptor",
        })?
    } else {
        let seating = crate::seat::run(&crate::seat::SeatingRequest {
            phrases: args.phrases,
            from_mk1: args.from_mk1,
            seats: args.seats,
            network: args.network,
            cmd: "descriptor",
        })?;
        seating_notes = seating.notes;
        seating.descriptor
    };

    // A TEMPLATE HAS NO CONCRETE FORM, and saying so is the whole point of the
    // check. Without keys there is nothing to substitute, and rendering
    // something anyway would hand a coordinator a string that looks like a
    // wallet and is not one. `md decode` is the command for that card.
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "descriptor requires wallet-policy mode (Pubkeys TLV): this card is a keyless \
             TEMPLATE, which has no concrete form. Supply the matching mk1 key cards with \
             --from-mk1 <STRING> (repeatable) or --from-mk1-file <FILE> and they will be \
             seated into it; or rebuild the policy from a template with --template <T> \
             --key @i=XPUB; or use `md decode` to see the template as it stands."
                .into(),
        ));
    }

    // N2 — the S → K cell. `--emit` changes ONLY the output form: every A2/A3/
    // A4 seating rule above ran exactly as it does without the flag, and a
    // refusal there never reaches this line.
    if args.emit == Some(Emit::Md1) {
        return emit_md1_card(&descriptor, &seating_notes);
    }

    let rendered = match args.chain {
        Some(chain) => md_codec::to_miniscript_descriptor(&descriptor, chain)?.to_string(),
        None => md_codec::to_miniscript_descriptor_multipath(&descriptor)?.to_string(),
    };

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let v = serde_json::json!({
            "schema": SCHEMA,
            "network": args.network_str,
            "chain": args.chain,
            "descriptor": rendered,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        emit_seating_notes(&seating_notes);
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::WatchOnly,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }
    let _ = args.json;
    let _ = args.network_str;

    println!("{rendered}");
    emit_seating_notes(&seating_notes);
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::WatchOnly,
        &mut std::io::stderr(),
    );
    Ok(0)
}

/// PHASE B's notes belong on stderr: stdout is the machine contract a
/// coordinator pastes, and a note in it would corrupt exactly the consumer
/// the descriptor exists for.
pub fn emit_seating_notes(notes: &[String]) {
    for n in notes {
        eprintln!("{n}");
    }
}

/// N2's input-mode rule: `--emit md1` mints a card from a SEATING RESULT, so
/// it needs a seating request — the keyless policy card plus its mk1 key
/// cards. The other two input modes each already have their own answer, and
/// naming which one is the entire content of these refusals.
///
/// **`BadArg` (exit 2), not a content refusal at exit 1.** Every flag here is
/// spelled correctly and every value parses; what is wrong is the
/// COMBINATION, which is the class clap reports at exit 2 through its own
/// `conflicts_with`. Nothing is being said about the material.
fn check_emit_md1_input_mode(template: Option<&str>, from_mk1: &[String]) -> Result<(), CliError> {
    const NEEDS: &str = "--emit md1 mints a keyed card from a SEATED card set, so it needs the \
                         keyless policy card together with its mk1 key cards (--from-mk1 \
                         <STRING>, repeatable, or --from-mk1-file <FILE>).";
    if template.is_some() {
        return Err(CliError::BadArg(format!(
            "{NEEDS} Minting a card from a template plus keys is what `md encode <TEMPLATE> \
             --key @i=XPUB` does -- use that."
        )));
    }
    if from_mk1.is_empty() {
        // Reached with a card on the positional and no key cards: a KEYED one
        // (the re-emit the spec names) or a keyless one, and the sentence is
        // true of both -- either way the argument already is the artifact.
        return Err(CliError::BadArg(format!(
            "{NEEDS} These md1 phrases are a card already, and re-emitting a card you are \
             holding would hand back what you pasted in. Drop --emit md1 to render this card \
             as a descriptor."
        )));
    }
    Ok(())
}

/// Mint the seated policy as a keyed md1 card: stdout carries the artifact
/// unbroken, one string per line, exactly as `md encode` writes it.
///
/// The cards come from [`crate::cmd::encode::mint_md1_cards`], the same
/// function `md encode` mints through — so N2's byte-identity oracle is a
/// question about the composed `Descriptor` and nothing else.
///
/// `--json` cannot arrive here: its envelope carries a `descriptor` field
/// this form does not produce, so the two flags are declared mutually
/// exclusive at the clap surface rather than one silently discarding the
/// other (REVIEW-converter-whole-diff-r1 I4's defect class, on this verb).
fn emit_md1_card(descriptor: &Descriptor, seating_notes: &[String]) -> Result<u8, CliError> {
    // THE AUTHORING GATE, for the same reason `md encode` runs it before it
    // mints (`cmd/encode.rs`): BIP-68 reads only bits 31, 22 and 0-15 of a
    // relative locktime, so a plate can assert a four-year lock the chain
    // releases in three months. This is a minting surface too, and which
    // command engraved the plate makes no difference to what it claims.
    md_codec::validate::validate_relative_timelocks(&descriptor.tree)?;

    let (cards, chunk_set_id) = crate::cmd::encode::mint_md1_cards(descriptor, false)?;
    if let Some(csid) = chunk_set_id {
        // The one thing that tells an operator which chunks belong to one
        // card, on stderr where `md encode` also puts it -- stdout stays the
        // artifact and nothing else.
        eprintln!("chunk-set-id: 0x{csid:05x}");
    }
    let mut body = String::new();
    for s in &cards {
        body.push_str(s);
        body.push('\n');
    }
    print!("{body}");
    emit_seating_notes(seating_notes);
    // A keyed card is watch-only material: public keys only, no spend.
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::WatchOnly,
        &mut std::io::stderr(),
    );
    Ok(0)
}
