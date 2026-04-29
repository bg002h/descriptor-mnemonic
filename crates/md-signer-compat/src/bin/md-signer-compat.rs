//! `md-signer-compat` CLI — validate an MD-encoded BIP 388 wallet
//! policy (or its bytecode) against a named hardware-signer subset.
//!
//! Architectural note: this CLI ships in the `md-signer-compat` crate
//! rather than as a `md validate --signer` subcommand on the main `md`
//! binary because md-signer-compat already depends on md-codec, and
//! adding the reverse dep for the CLI would create a workspace
//! dependency cycle.
//!
//! Usage:
//!
//! ```text
//! md-signer-compat validate --signer <coldcard|ledger> --bytecode-hex <HEX>
//! md-signer-compat validate --signer <coldcard|ledger> --string <md1...>...
//! md-signer-compat list-signers
//! ```

use std::process;

use clap::{Parser, Subcommand};
use md_codec::{DecodeOptions, WalletPolicy, decode};
use md_signer_compat::{COLDCARD_TAP, LEDGER_TAP, SignerSubset, validate_tap_tree};
use miniscript::descriptor::Descriptor;

#[derive(Debug, Parser)]
#[command(name = "md-signer-compat", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate an MD wallet policy (or its bytecode hex) against a named
    /// hardware-signer subset. Exits 0 on pass, non-zero on subset violation.
    Validate {
        /// Named signer subset to validate against.
        #[arg(long, value_name = "coldcard|ledger")]
        signer: String,

        /// Raw bytecode hex (e.g. `0034035...`). Mutually exclusive with `--string`.
        #[arg(long, value_name = "HEX", conflicts_with = "string")]
        bytecode_hex: Option<String>,

        /// One or more MD backup strings. Mutually exclusive with `--bytecode-hex`.
        #[arg(long = "string", value_name = "MD-STRING", num_args = 1..)]
        string: Vec<String>,
    },

    /// List the named subsets recognised by this binary.
    ListSigners,
}

fn resolve_signer(name: &str) -> Result<&'static SignerSubset, anyhow::Error> {
    match name.to_ascii_lowercase().as_str() {
        "coldcard" => Ok(&COLDCARD_TAP),
        "ledger" => Ok(&LEDGER_TAP),
        other => anyhow::bail!(
            "--signer must be one of: coldcard, ledger; got {other:?}. \
             (run `md-signer-compat list-signers` for the canonical list.)"
        ),
    }
}

fn main() {
    let cli = Cli::parse();

    let result: Result<(), anyhow::Error> = match cli.command {
        Command::Validate {
            signer,
            bytecode_hex,
            string,
        } => cmd_validate(&signer, bytecode_hex.as_deref(), &string),
        Command::ListSigners => cmd_list_signers(),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn cmd_validate(
    signer_name: &str,
    bytecode_hex: Option<&str>,
    strings: &[String],
) -> Result<(), anyhow::Error> {
    let subset = resolve_signer(signer_name)?;

    let policy: WalletPolicy = match (bytecode_hex, strings) {
        (Some(_), s) if !s.is_empty() => {
            anyhow::bail!("--bytecode-hex and --string are mutually exclusive");
        }
        (Some(hex_str), _) => {
            let bytes = hex::decode(hex_str.trim())
                .map_err(|e| anyhow::anyhow!("--bytecode-hex parse failed: {e}"))?;
            WalletPolicy::from_bytecode(&bytes)
                .map_err(|e| anyhow::anyhow!("WalletPolicy::from_bytecode failed: {e}"))?
        }
        (None, s) if !s.is_empty() => {
            let refs: Vec<&str> = s.iter().map(String::as_str).collect();
            let decoded = decode(&refs, &DecodeOptions::new())
                .map_err(|e| anyhow::anyhow!("decode failed: {e}"))?;
            decoded.policy
        }
        (None, _) => {
            anyhow::bail!("must supply either --bytecode-hex or --string");
        }
    };

    let descriptor = policy
        .inner()
        .clone()
        .into_descriptor()
        .map_err(|e| anyhow::anyhow!("WalletPolicy::into_descriptor failed: {e}"))?;

    match descriptor {
        Descriptor::Tr(tr) => match tr.tap_tree() {
            Some(tap_tree) => match validate_tap_tree(subset, tap_tree) {
                Ok(()) => {
                    println!("PASS — every tap leaf is in subset {:?}", subset.name);
                    Ok(())
                }
                Err(md_codec::Error::SubsetViolation {
                    operator,
                    leaf_index,
                    ..
                }) => {
                    eprintln!(
                        "FAIL — out-of-subset operator {operator:?} at leaf_index={leaf_index:?} \
                         (subset: {:?})",
                        subset.name
                    );
                    process::exit(2);
                }
                Err(other) => Err(anyhow::anyhow!(
                    "validate_tap_tree failed with unexpected error: {other:?}"
                )),
            },
            None => {
                println!(
                    "PASS — key-path-only tr() has no script leaves to validate (subset: {:?})",
                    subset.name
                );
                Ok(())
            }
        },
        // Non-tr top-level descriptors have no tap-leaves. Print a noop pass.
        other => {
            println!(
                "PASS — top-level descriptor is {} (no tap leaves to validate)",
                describe_top_level(&other)
            );
            Ok(())
        }
    }
}

fn cmd_list_signers() -> Result<(), anyhow::Error> {
    for s in [&COLDCARD_TAP, &LEDGER_TAP] {
        println!("{} ({} operators):", s.name, s.allowed_operators.len());
        for op in s.allowed_operators {
            println!("  - {op}");
        }
        println!();
    }
    Ok(())
}

fn describe_top_level(d: &Descriptor<miniscript::DescriptorPublicKey>) -> &'static str {
    match d {
        Descriptor::Wpkh(_) => "wpkh()",
        Descriptor::Wsh(_) => "wsh()",
        Descriptor::Sh(_) => "sh()",
        Descriptor::Pkh(_) => "pkh()",
        Descriptor::Tr(_) => "tr()",
        Descriptor::Bare(_) => "bare",
    }
}
