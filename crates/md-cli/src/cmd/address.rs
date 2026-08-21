use crate::error::CliError;

pub struct AddressArgs<'a> {
    pub phrases: &'a [String],
    pub template: Option<&'a str>,
    pub keys: &'a [String],
    pub fingerprints: &'a [String],
    /// Shared origin-path override, mirroring `md encode --path`.
    pub path: Option<&'a str>,
    pub network: bitcoin::Network,
    pub network_str: &'static str,
    pub chain: u32,
    pub index: u32,
    pub count: u32,
    pub json: bool,
}

pub fn run(args: AddressArgs<'_>) -> Result<u8, CliError> {
    let descriptor = crate::cmd::build::build_descriptor(&crate::cmd::build::DescriptorInput {
        phrases: args.phrases,
        template: args.template,
        keys: args.keys,
        fingerprints: args.fingerprints,
        path: args.path,
        network: args.network,
        cmd: "address",
    })?;
    if !descriptor.is_wallet_policy() {
        return Err(CliError::BadArg(
            "address requires wallet-policy mode (Pubkeys TLV); supply --key @i=XPUB or use a wallet-policy-mode phrase".into(),
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
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::WatchOnly,
        &mut std::io::stderr(),
    );
    Ok(0)
}
