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
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    /// `None` = multipath (`<0;1>`); `Some(n)` = collapse to that chain.
    pub chain: Option<u32>,
    pub json: bool,
}

pub fn run(args: DescriptorArgs<'_>) -> Result<u8, CliError> {
    let descriptor = build_descriptor(&DescriptorInput {
        phrases: args.phrases,
        template: args.template,
        keys: args.keys,
        fingerprints: args.fingerprints,
        path: args.path,
        network: args.network,
        cmd: "descriptor",
    })?;

    // A TEMPLATE HAS NO CONCRETE FORM, and saying so is the whole point of the
    // check. Without keys there is nothing to substitute, and rendering
    // something anyway would hand a coordinator a string that looks like a
    // wallet and is not one. `md decode` is the command for that card.
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "descriptor requires wallet-policy mode (Pubkeys TLV): this card is a keyless \
             TEMPLATE, which has no concrete form. Supply --key @i=XPUB, or use `md decode` \
             to see the template."
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
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::WatchOnly,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }
    let _ = args.json;
    let _ = args.network_str;

    println!("{rendered}");
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::WatchOnly,
        &mut std::io::stderr(),
    );
    Ok(0)
}
