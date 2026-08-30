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
    pub json: bool,
}

pub fn run(args: DescriptorArgs<'_>) -> Result<u8, CliError> {
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
