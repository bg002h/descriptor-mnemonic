mod cmd;
#[cfg(feature = "cli-compiler")]
mod compile;
mod error;
mod format;
mod parse;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use error::CliError;

#[derive(Debug, Parser)]
#[command(name = "md", version, about = "Mnemonic Descriptor (MD) — engravable BIP 388 wallet policy backups", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup string(s).
    #[command(after_long_help = "EXAMPLES:\n  $ md encode wpkh(@0/<0;1>/*)\n  md1qqpqqxqxkceprx7rap4t")]
    Encode {
        /// BIP 388 template, e.g. `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))`.
        template: Option<String>,
        /// Compile a sub-Miniscript-Policy expression into a template (cli-compiler).
        #[arg(long = "from-policy", value_name = "EXPR", conflicts_with = "template")]
        from_policy: Option<String>,
        /// Script context for `--from-policy`.
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"])]
        context: Option<String>,
        /// Override the inferred shared derivation path.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        /// Concrete xpub for placeholder `@i`. Repeatable.
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder `@i`. Repeatable.
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
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
    #[command(after_long_help = "EXAMPLES:\n  $ md decode md1qqpqqxqxkceprx7rap4t\n  wpkh(@0/<0;1>/*)")]
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

fn dispatch(c: Command) -> Result<(), CliError> {
    match c {
        Command::Encode {
            template, from_policy, context, path: _,
            keys, fingerprints, force_chunked, force_long_code,
            policy_id_fingerprint, json,
        } => {
            let template_str: String = if let Some(expr) = from_policy {
                #[cfg(feature = "cli-compiler")]
                {
                    let ctx: compile::ScriptContext = context
                        .ok_or_else(|| CliError::BadArg("--from-policy requires --context tap|segwitv0".into()))?
                        .parse().map_err(|e: compile::CompileError| CliError::Compile(e.to_string()))?;
                    compile::compile_policy_to_template(&expr, ctx).map_err(CliError::from)?
                }
                #[cfg(not(feature = "cli-compiler"))]
                { let _ = (expr, context); return Err(CliError::BadArg(
                    "--from-policy requires the cli-compiler feature".into())); }
            } else {
                template.ok_or_else(|| CliError::BadArg(
                    "encode: TEMPLATE required (or use --from-policy with cli-compiler)".into()))?
            };
            cmd::encode::run(cmd::encode::EncodeArgs {
                template: &template_str, keys: &keys, fingerprints: &fingerprints,
                force_chunked, force_long_code, policy_id_fingerprint, json,
            })
        }
        Command::Decode { strings, json } => cmd::decode::run(&strings, json),
        Command::Verify { strings, template, keys, fingerprints } => cmd::verify::run(cmd::verify::VerifyArgs {
            strings: &strings,
            template: &template,
            keys: &keys,
            fingerprints: &fingerprints,
        }),
        Command::Inspect { strings, json } => cmd::inspect::run(&strings, json),
        Command::Bytecode { strings, json } => cmd::bytecode::run(&strings, json),
        Command::Vectors { out } => cmd::vectors::run(out),
        Command::Compile { expr, context, json } => {
            #[cfg(feature = "cli-compiler")]
            { cmd::compile::run(&expr, &context, json) }
            #[cfg(not(feature = "cli-compiler"))]
            { let _ = (expr, context, json); Err(CliError::BadArg(
                "compile requires the cli-compiler feature; rebuild with --features cli-compiler".into())) }
        },
    }
}
