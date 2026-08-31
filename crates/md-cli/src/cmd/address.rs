use crate::error::CliError;

pub struct AddressArgs<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    /// Shared origin-path override, mirroring `md encode --path`.
    pub path: Option<&'a str>,
    /// mk1 key-card strings (P2). Non-empty routes to the seating engine.
    pub from_mk1: &'a [String],
    /// Raw `--seat '@i=<chunk-set-id>'` values (A5).
    pub seats: &'a [String],
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub chain: u32,
    pub index: u32,
    pub count: u32,
    pub json: bool,
}

pub fn run(args: AddressArgs<'_>) -> Result<u8, CliError> {
    // R9 — checked before either branch below decides what to build; see
    // its own doc comment in cmd::build for why it must run first.
    crate::cmd::build::check_from_mk1_arity(args.phrases, args.from_mk1, args.template, "address")?;

    // P2 — the S row, exactly as on `md descriptor`: the two commands must
    // seat identically or one of them derives a different wallet.
    let mut seating_notes: Vec<String> = Vec::new();
    let descriptor = if args.from_mk1.is_empty() {
        if !args.seats.is_empty() {
            return Err(CliError::Seat(
                "--seat asserts which mk1 key card fills a slot, so it needs \
                 --from-mk1/--from-mk1-file cards to choose among."
                    .into(),
            ));
        }
        crate::cmd::build::build_descriptor(&crate::cmd::build::DescriptorInput {
            phrases: args.phrases,
            template: args.template,
            keys: args.keys,
            fingerprints: args.fingerprints,
            path: args.path,
            network: args.network,
            cmd: "address",
        })?
    } else {
        let seating = crate::seat::run(&crate::seat::SeatingRequest {
            phrases: args.phrases,
            from_mk1: args.from_mk1,
            seats: args.seats,
            network: args.network,
            cmd: "address",
        })?;
        seating_notes = seating.notes;
        seating.descriptor
    };
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "address requires wallet-policy mode (Pubkeys TLV): this card is a keyless \
             TEMPLATE. Supply the matching mk1 key cards with --from-mk1 <STRING> \
             (repeatable) or --from-mk1-file <FILE>, or rebuild the policy with \
             --template <T> --key @i=XPUB."
                .into(),
        ));
    }

    // Collect (chain, index, address) tuples first; then emit text or JSON.
    let mut rows: Vec<(u32, u32, String)> = Vec::with_capacity(args.count as usize);
    for k in 0..args.count {
        let i = args.index.checked_add(k).ok_or_else(|| {
            CliError::BadArg(format!(
                "--index + --count overflows u32: {} + {}",
                args.index, args.count
            ))
        })?;
        let addr = descriptor
            .derive_address(args.chain, i, args.network)?
            .assume_checked();
        rows.push((args.chain, i, addr.to_string()));
    }

    #[cfg(feature = "json")]
    if args.json {
        use crate::format::json::SCHEMA;
        let addresses: Vec<serde_json::Value> = rows
            .iter()
            .map(|(c, i, a)| serde_json::json!({ "chain": c, "index": i, "address": a }))
            .collect();
        let v = serde_json::json!({
            "schema": SCHEMA,
            "network": args.network_str,
            "addresses": addresses,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        crate::cmd::descriptor::emit_seating_notes(&seating_notes);
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::WatchOnly,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }
    let _ = args.json;
    let _ = args.network_str;

    for (_, _, addr) in &rows {
        println!("{addr}");
    }
    crate::cmd::descriptor::emit_seating_notes(&seating_notes);
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::WatchOnly,
        &mut std::io::stderr(),
    );
    Ok(0)
}
