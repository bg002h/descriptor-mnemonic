mod error;

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

fn dispatch(cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Encode { .. } => unimplemented!("encode"),
        Command::Decode { .. } => unimplemented!("decode"),
        Command::Verify { .. } => unimplemented!("verify"),
        Command::Inspect { .. } => unimplemented!("inspect"),
        Command::Bytecode { .. } => unimplemented!("bytecode"),
        Command::Vectors { .. } => unimplemented!("vectors"),
        Command::Compile { .. } => unimplemented!("compile"),
    }
}
