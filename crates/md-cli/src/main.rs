#![allow(missing_docs)]

mod cmd;
#[cfg(feature = "cli-compiler")]
mod compile;
mod error;
mod format;
mod parse;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use error::CliError;

/// CLI-facing network selector. Maps to `bitcoin::Network`.
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum CliNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl From<CliNetwork> for bitcoin::Network {
    fn from(n: CliNetwork) -> Self {
        match n {
            CliNetwork::Mainnet => bitcoin::Network::Bitcoin,
            CliNetwork::Testnet => bitcoin::Network::Testnet,
            CliNetwork::Signet => bitcoin::Network::Signet,
            CliNetwork::Regtest => bitcoin::Network::Regtest,
        }
    }
}

impl CliNetwork {
    /// Stable kebab-cased name for JSON output. Matches the clap
    /// `value_enum` rendering, NOT `bitcoin::Network::Display` (which
    /// emits "bitcoin" for mainnet — confusing for JSON consumers).
    fn as_str(self) -> &'static str {
        match self {
            CliNetwork::Mainnet => "mainnet",
            CliNetwork::Testnet => "testnet",
            CliNetwork::Signet => "signet",
            CliNetwork::Regtest => "regtest",
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "md", version, about = "Mnemonic Descriptor (MD) — engravable BIP 388 wallet policy backups", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup string(s).
    #[command(
        after_long_help = "EXAMPLES:\n  $ md encode wpkh(@0/<0;1>/*)\n  md1yqpqqxqq8xtwhw4xwn4qh"
    )]
    Encode {
        /// BIP 388 template, e.g. `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))`.
        template: Option<String>,
        /// Compile a sub-Miniscript-Policy expression into a template (cli-compiler).
        #[arg(long = "from-policy", value_name = "EXPR", conflicts_with = "template")]
        from_policy: Option<String>,
        /// Script context for `--from-policy`.
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"])]
        context: Option<String>,
        /// Tap-context only: fallback unspendable internal key passed to
        /// miniscript's `compile_tr`. Defaults to BIP-341 NUMS H-point when
        /// omitted (auto-NUMS); supplying a value is rare and used to force
        /// a specific NUMS-equivalent key. Rejected when --context segwitv0.
        #[arg(long, value_name = "KEY")]
        unspendable_key: Option<String>,
        /// Override the inferred origin path with a single shared path
        /// (flattens Divergent mode to Shared). Accepts named (bip44|48|49|84|86),
        /// hex (0xNN), or literal (m/...) forms.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        /// Concrete xpub for placeholder `@i`. Repeatable.
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder `@i`. Repeatable.
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
        /// Network for xpub validation (and JSON output labeling).
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Force chunked encoding even for short policies.
        #[arg(long)]
        force_chunked: bool,
        /// Force the long BCH code even when the regular code suffices.
        #[arg(long)]
        force_long_code: bool,
        /// Print the freshly-computed PolicyId fingerprint after the phrase.
        #[arg(long)]
        policy_id_fingerprint: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Decode one or more MD backup strings into a wallet policy template.
    #[command(
        after_long_help = "EXAMPLES:\n  $ md decode md1yqpqqxqq8xtwhw4xwn4qh\n  wpkh(@0/<0;1>/*)"
    )]
    Decode {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Verify backup strings re-encode to a given template.
    Verify {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long, required = true)]
        template: String,
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
        /// Network for xpub validation.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
    },
    /// Decode + pretty-print everything the codec sees.
    Inspect {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Dump the raw payload bits in an annotated layout.
    Bytecode {
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Regenerate the project's test-vector corpus (maintainer tool).
    Vectors {
        #[arg(long, value_name = "DIR")]
        out: Option<String>,
    },
    /// Compile a sub-Miniscript-Policy expression into a BIP 388 template.
    Compile {
        expr: String,
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"], required = true)]
        context: String,
        /// Tap-context only: fallback unspendable internal key passed to
        /// miniscript's `compile_tr`. Defaults to BIP-341 NUMS H-point when
        /// omitted (auto-NUMS); supplying a value is rare. Rejected when
        /// --context segwitv0.
        #[arg(long, value_name = "KEY")]
        unspendable_key: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Derive bitcoin addresses from a wallet-policy-mode descriptor.
    #[command(after_long_help = "EXAMPLES:\n  $ md address md1qq...\n  bc1q...",
              group = clap::ArgGroup::new("address_input").required(true).args(["phrases", "template"]))]
    Address {
        /// One or more md1 phrases. Mutually exclusive with --template.
        #[arg(num_args = 0..)]
        phrases: Vec<String>,
        /// BIP 388 template. Requires at least one --key. Mutually exclusive with phrases.
        #[arg(long, value_name = "TEMPLATE", conflicts_with = "phrases")]
        template: Option<String>,
        /// Concrete xpub for placeholder @i. Repeatable. Requires --template.
        #[arg(long = "key", value_name = "@i=XPUB", requires = "template")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder @i. Repeatable. Requires --template.
        #[arg(long = "fingerprint", value_name = "@i=HEX", requires = "template")]
        fingerprints: Vec<String>,
        /// Network for xpub validation and address rendering.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Multipath alternative selector (0 = receive, 1 = change for canonical <0;1>/*).
        #[arg(long, default_value_t = 0)]
        chain: u32,
        /// Sugar for --chain 1.
        #[arg(long, conflicts_with = "chain")]
        change: bool,
        /// Starting index along the wildcard.
        #[arg(long, default_value_t = 0)]
        index: u32,
        /// Number of consecutive addresses to derive starting at --index.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=1000))]
        count: u32,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli.command) {
        Ok(()) => ExitCode::from(0),
        Err(CliError::BadArg(m)) => {
            eprintln!("md: {m}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("md: {e}");
            ExitCode::from(1)
        }
    }
}

/// v0.18 Item G — reject `--unspendable-key` values that aren't the BIP-341
/// NUMS H-point literal hex. Empty-string and segwitv0-incompat checks fire
/// upstream of this guard; what reaches here is `Some(<non-empty-tap-value>)`.
#[cfg(feature = "cli-compiler")]
fn validate_unspendable_key_nums_only(uk: Option<&str>) -> Result<(), CliError> {
    if let Some(v) = uk {
        if v != parse::template::NUMS_H_POINT_X_ONLY_HEX {
            return Err(CliError::BadArg(
                "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex \
                 (50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0) or omitted \
                 (auto-NUMS default). Other forms (xpub-style descriptor keys, arbitrary x-only \
                 hex) are not supported in this release; track v0.19+ for caller-supplied \
                 internal-key support."
                    .into(),
            ));
        }
    }
    Ok(())
}

fn dispatch(c: Command) -> Result<(), CliError> {
    match c {
        Command::Encode {
            template,
            from_policy,
            context,
            unspendable_key,
            path,
            keys,
            fingerprints,
            network,
            force_chunked,
            force_long_code,
            policy_id_fingerprint,
            json,
        } => {
            let template_str: String = if let Some(expr) = from_policy {
                #[cfg(feature = "cli-compiler")]
                {
                    if unspendable_key.as_deref() == Some("") {
                        return Err(CliError::BadArg(
                            "--unspendable-key must not be empty (omit the flag for auto-NUMS default)".into()));
                    }
                    let ctx: compile::ScriptContext = context
                        .ok_or_else(|| {
                            CliError::BadArg("--from-policy requires --context tap|segwitv0".into())
                        })?
                        .parse()
                        .map_err(|e: compile::CompileError| CliError::Compile(e.to_string()))?;
                    if matches!(ctx, compile::ScriptContext::SegwitV0) && unspendable_key.is_some()
                    {
                        return Err(CliError::BadArg(
                            "--unspendable-key is only valid for --context tap (segwitv0 has no internal key)".into()));
                    }
                    validate_unspendable_key_nums_only(unspendable_key.as_deref())?;
                    compile::compile_policy_to_template(&expr, ctx, unspendable_key.as_deref())
                        .map_err(CliError::from)?
                }
                #[cfg(not(feature = "cli-compiler"))]
                {
                    let _ = (expr, context, unspendable_key);
                    return Err(CliError::BadArg(
                        "--from-policy requires the cli-compiler feature".into(),
                    ));
                }
            } else {
                if unspendable_key.is_some() {
                    return Err(CliError::BadArg(
                        "--unspendable-key is only meaningful with --from-policy".into(),
                    ));
                }
                template.ok_or_else(|| {
                    CliError::BadArg(
                        "encode: TEMPLATE required (or use --from-policy with cli-compiler)".into(),
                    )
                })?
            };
            cmd::encode::run(cmd::encode::EncodeArgs {
                template: &template_str,
                keys: &keys,
                fingerprints: &fingerprints,
                path: path.as_deref(),
                network: network.into(),
                network_str: network.as_str(),
                force_chunked,
                force_long_code,
                policy_id_fingerprint,
                json,
            })
        }
        Command::Decode { strings, json } => cmd::decode::run(&strings, json),
        Command::Verify {
            strings,
            template,
            keys,
            fingerprints,
            network,
        } => cmd::verify::run(cmd::verify::VerifyArgs {
            strings: &strings,
            template: &template,
            keys: &keys,
            fingerprints: &fingerprints,
            network: network.into(),
        }),
        Command::Inspect { strings, json } => cmd::inspect::run(&strings, json),
        Command::Bytecode { strings, json } => cmd::bytecode::run(&strings, json),
        Command::Vectors { out } => cmd::vectors::run(out),
        Command::Compile {
            expr,
            context,
            unspendable_key,
            json,
        } => {
            #[cfg(feature = "cli-compiler")]
            {
                if unspendable_key.as_deref() == Some("") {
                    return Err(CliError::BadArg(
                        "--unspendable-key must not be empty (omit the flag for auto-NUMS default)"
                            .into(),
                    ));
                }
                if context == "segwitv0" && unspendable_key.is_some() {
                    return Err(CliError::BadArg(
                        "--unspendable-key is only valid for --context tap (segwitv0 has no internal key)".into()));
                }
                validate_unspendable_key_nums_only(unspendable_key.as_deref())?;
                cmd::compile::run(&expr, &context, unspendable_key.as_deref(), json)
            }
            #[cfg(not(feature = "cli-compiler"))]
            {
                let _ = (expr, context, unspendable_key, json);
                Err(CliError::BadArg(
                "compile requires the cli-compiler feature; rebuild with --features cli-compiler".into()))
            }
        }
        Command::Address {
            phrases,
            template,
            keys,
            fingerprints,
            network,
            chain,
            change,
            index,
            count,
            json,
        } => {
            let chain = if change { 1 } else { chain };
            cmd::address::run(cmd::address::AddressArgs {
                phrases: &phrases,
                template: template.as_deref(),
                keys: &keys,
                fingerprints: &fingerprints,
                network: network.into(),
                network_str: network.as_str(),
                chain,
                index,
                count,
                json,
            })
        }
    }
}
