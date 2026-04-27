//! WDM CLI binary — encode, decode, verify, inspect, and bytecode subcommands.
//!
//! # Tasks covered
//!
//! - 7.1: binary skeleton (clap derive, subcommand dispatch)
//! - 7.2: `wdm encode`
//! - 7.3: `wdm decode`
//! - 7.4: `wdm verify`
//! - 7.5: `wdm inspect`
//! - 7.6: `wdm bytecode`
//! - 7.7: `wdm vectors` — implemented in Phase 8 (shares `build_test_vectors` with `gen_vectors`)
//! - 7.8: `--path` argument parser for `wdm encode`

use std::process;
use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use clap::{Parser, Subcommand};
use wdm_codec::{
    BchCode, ChunkHeader, DecodeOptions, EncodeOptions, WalletPolicy, decode, decode_string,
    encode, five_bit_to_bytes, wallet_id::WalletIdSeed,
};

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

/// Wallet Descriptor Mnemonic (WDM) — engravable BIP 388 wallet policy backups.
#[derive(Debug, Parser)]
#[command(name = "wdm", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into WDM backup strings.
    Encode {
        /// The wallet policy (BIP 388 template string).
        policy: String,

        /// Override the inferred shared derivation path.
        ///
        /// Accepts: a name (e.g. bip48, bip84), a hex indicator (e.g. 0x05),
        /// or a literal derivation path (e.g. "m/48'/0'/0'/2'").
        #[arg(long, value_name = "PATH")]
        path: Option<String>,

        /// Force chunked encoding even for short policies.
        #[arg(long)]
        force_chunked: bool,

        /// Force the long BCH code even when the regular code would suffice.
        #[arg(long)]
        force_long_code: bool,

        /// Override the chunk-header wallet ID seed (4-byte hex, e.g. 0xdeadbeef).
        #[arg(long, value_name = "0xHEX")]
        seed: Option<String>,

        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },

    /// Decode one or more WDM backup strings into a wallet policy.
    Decode {
        /// One or more WDM backup strings (all chunks of one backup, any order).
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,

        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },

    /// Decode and verify backup strings match an expected policy.
    Verify {
        /// One or more WDM backup strings.
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,

        /// The expected wallet policy.
        #[arg(long, required = true)]
        policy: String,
    },

    /// Inspect a single WDM backup string (chunk header only, no full decode).
    Inspect {
        /// A single WDM backup string.
        string: String,
    },

    /// Hex-dump the canonical WDM bytecode for a wallet policy.
    Bytecode {
        /// The wallet policy (BIP 388 template string).
        policy: String,
    },

    /// Generate test vectors as JSON and print to stdout.
    ///
    /// Outputs the same content as `gen_vectors --output -` (write mode prints to stderr).
    /// Use `wdm vectors > vectors.json` to capture the output.
    Vectors,
}

// ---------------------------------------------------------------------------
// Path argument parser (Task 7.8)
// ---------------------------------------------------------------------------

/// Maps friendly name → dictionary indicator byte.
const NAME_TABLE: &[(&str, u8)] = &[
    // Mainnet
    ("bip44", 0x01),
    ("bip49", 0x02),
    ("bip84", 0x03),
    ("bip86", 0x04),
    ("bip48", 0x05),
    ("bip87", 0x07),
    // Testnet variants
    ("bip44t", 0x11),
    ("bip49t", 0x12),
    ("bip84t", 0x13),
    ("bip86t", 0x14),
    ("bip48t", 0x15),
    ("bip87t", 0x17),
];

/// Parse a `--path` argument in one of three forms (tried in order):
///
/// 1. **Name**: `bip44`, `bip49`, `bip84`, `bip86`, `bip48`, `bip87`, plus
///    testnet `*t` variants.
/// 2. **Hex indicator**: `0x05` → look up in the path dictionary.
/// 3. **Literal path**: `m/48'/0'/0'/2'` → parse via `DerivationPath::from_str`.
fn parse_path_arg(s: &str) -> Result<DerivationPath, anyhow::Error> {
    // 1. Name lookup (case-insensitive).
    let lower = s.to_ascii_lowercase();
    for &(name, indicator) in NAME_TABLE {
        if lower == name {
            return indicator_to_derivation_path(indicator).ok_or_else(|| {
                anyhow::anyhow!("internal: name {name:?} maps to indicator 0x{indicator:02x} which is not in the path dictionary")
            });
        }
    }

    // 2. Hex indicator: `0x??`.
    if let Some(hex_part) = lower.strip_prefix("0x") {
        let indicator = u8::from_str_radix(hex_part, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex indicator {s:?}: expected 0x00–0xFF"))?;

        if indicator == 0xFE {
            return Err(anyhow::anyhow!(
                "indicator 0xFE selects the explicit-path encoding; supply the literal derivation path instead (e.g. \"m/48'/0'/0'/2'\")"
            ));
        }

        return indicator_to_derivation_path(indicator).ok_or_else(|| {
            anyhow::anyhow!(
                "indicator 0x{indicator:02x} is not in the WDM path dictionary; \
                 known indicators: 0x01-0x07, 0x11-0x15, 0x17. \
                 Use a literal path like \"m/48'/0'/0'/2'\" instead."
            )
        });
    }

    // 3. Literal derivation path.
    DerivationPath::from_str(s).map_err(|e| anyhow::anyhow!("invalid derivation path {s:?}: {e}"))
}

/// Look up the `DerivationPath` for a known indicator byte using the path
/// dictionary in `wdm_codec::bytecode::path`.
fn indicator_to_derivation_path(indicator: u8) -> Option<DerivationPath> {
    wdm_codec::bytecode::path::indicator_to_path(indicator).cloned()
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

fn cmd_encode(
    policy_str: &str,
    path_arg: Option<&str>,
    force_chunked: bool,
    force_long_code: bool,
    seed_arg: Option<&str>,
    json: bool,
) -> Result<(), anyhow::Error> {
    // Parse the policy.
    let mut policy: WalletPolicy = policy_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid policy: {e}"))?;

    // Apply --path override if present.
    // NOTE: In v0.1 the `--path` override is parsed and validated, but the
    // EncodeOptions struct does not yet have a `shared_path` field — that is
    // deferred to v0.2 (FOLLOWUPS.md: 7-encode-path-override). The option is
    // accepted so the CLI surface is complete; a warning is printed if the
    // user supplies it. The path is used to re-parse the policy with an
    // explicit origin when the override is provided.
    if let Some(path_str) = path_arg {
        let _path = parse_path_arg(path_str).map_err(|e| anyhow::anyhow!("--path: {e}"))?;
        // Re-parse is not straightforward without a full descriptor + xpub.
        // We validate the path arg and warn that it has no effect in v0.1.
        eprintln!(
            "warning: --path is parsed but the shared-path override is not yet applied to the bytecode encoder (deferred to v0.2; see FOLLOWUPS.md 7-encode-path-override)"
        );
        let _ = policy; // suppress move-without-use lint
        policy = policy_str.parse().map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Parse --seed.
    let wallet_id_seed: Option<WalletIdSeed> = match seed_arg {
        None => None,
        Some(s) => {
            let hex_part = s.to_ascii_lowercase();
            let hex_part = hex_part.strip_prefix("0x").unwrap_or(&hex_part);
            let val = u32::from_str_radix(hex_part, 16).map_err(|_| {
                anyhow::anyhow!("--seed: expected 4-byte hex like 0xdeadbeef, got {s:?}")
            })?;
            Some(WalletIdSeed::from(val))
        }
    };

    let mut opts = EncodeOptions::default();
    opts = opts.with_force_chunking(force_chunked);
    opts.force_long_code = force_long_code;
    opts.wallet_id_seed = wallet_id_seed;

    let backup = encode(&policy, &opts).map_err(|e| anyhow::anyhow!("encode failed: {e}"))?;

    if json {
        // Manual JSON construction (option b per spec: avoids serde derives on
        // library types that contain non-Serialize miniscript internals).
        let chunks_json: Vec<serde_json::Value> = backup
            .chunks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "raw": c.raw,
                    "chunk_index": c.chunk_index,
                    "total_chunks": c.total_chunks,
                    "code": match c.code { BchCode::Regular => "regular", BchCode::Long => "long" },
                })
            })
            .collect();
        let out = serde_json::json!({
            "chunks": chunks_json,
            "wallet_id_words": backup.wallet_id_words.to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        // Human-readable: one chunk string per line, wallet ID at the end.
        for chunk in &backup.chunks {
            println!("{}", chunk.raw);
        }
        println!();
        println!("Wallet ID: {}", backup.wallet_id_words);
    }

    Ok(())
}

fn cmd_decode(strings: &[String], json: bool) -> Result<(), anyhow::Error> {
    let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
    let result =
        decode(&refs, &DecodeOptions::new()).map_err(|e| anyhow::anyhow!("decode failed: {e}"))?;

    if json {
        let corr_json: Vec<serde_json::Value> = result
            .report
            .corrections
            .iter()
            .map(|c| {
                serde_json::json!({
                    "chunk_index": c.chunk_index,
                    "char_position": c.char_position,
                    "original": c.original.to_string(),
                    "corrected": c.corrected.to_string(),
                })
            })
            .collect();
        let v = result.report.verifications;
        let out = serde_json::json!({
            "policy": result.policy.to_canonical_string(),
            "report": {
                "outcome": format!("{:?}", result.report.outcome),
                "confidence": format!("{:?}", result.report.confidence),
                "corrections": corr_json,
                "verifications": {
                    "cross_chunk_hash_ok": v.cross_chunk_hash_ok,
                    "wallet_id_consistent": v.wallet_id_consistent,
                    "total_chunks_consistent": v.total_chunks_consistent,
                    "bytecode_well_formed": v.bytecode_well_formed,
                    "version_supported": v.version_supported,
                },
            },
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("{}", result.policy.to_canonical_string());
        println!();
        println!("Outcome:     {:?}", result.report.outcome);
        println!("Confidence:  {:?}", result.report.confidence);
        println!("Corrections: {}", result.report.corrections.len());
        let v = result.report.verifications;
        println!("Verifications:");
        println!("  cross_chunk_hash_ok:    {}", v.cross_chunk_hash_ok);
        println!("  wallet_id_consistent:   {}", v.wallet_id_consistent);
        println!("  total_chunks_consistent:{}", v.total_chunks_consistent);
        println!("  bytecode_well_formed:   {}", v.bytecode_well_formed);
        println!("  version_supported:      {}", v.version_supported);
    }

    Ok(())
}

fn cmd_verify(strings: &[String], expected_policy_str: &str) -> Result<(), anyhow::Error> {
    let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
    let result =
        decode(&refs, &DecodeOptions::new()).map_err(|e| anyhow::anyhow!("decode failed: {e}"))?;

    let expected: WalletPolicy = expected_policy_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid --policy: {e}"))?;

    let decoded_canonical = result.policy.to_canonical_string();
    let expected_canonical = expected.to_canonical_string();

    if decoded_canonical == expected_canonical {
        println!("OK — decoded policy matches expected policy.");
        println!("Policy: {decoded_canonical}");
        Ok(())
    } else {
        eprintln!("MISMATCH");
        eprintln!("  Decoded:  {decoded_canonical}");
        eprintln!("  Expected: {expected_canonical}");
        process::exit(1);
    }
}

fn cmd_inspect(string: &str) -> Result<(), anyhow::Error> {
    // Parse at the codex32 layer only.
    let decoded = decode_string(string).map_err(|e| anyhow::anyhow!("string parse failed: {e}"))?;

    // Convert 5-bit data to bytes.
    let bytes = five_bit_to_bytes(&decoded.data)
        .ok_or_else(|| anyhow::anyhow!("invalid 5-bit data in string"))?;

    // Parse the chunk header (do NOT proceed to bytecode decode).
    let (header, header_len) = ChunkHeader::from_bytes(&bytes)
        .map_err(|e| anyhow::anyhow!("chunk header parse failed: {e}"))?;

    let fragment_len = bytes.len() - header_len;

    // BCH code from the data-part length.
    let code_str = match decoded.code {
        BchCode::Regular => "Regular (13-char checksum)",
        BchCode::Long => "Long (15-char checksum)",
    };

    println!("BCH code:        {code_str}");
    println!("BCH corrections: {}", decoded.corrections_applied);
    println!("Fragment length: {fragment_len} bytes");

    match header {
        ChunkHeader::SingleString { version } => {
            println!("Type:            SingleString");
            println!("Version:         {version}");
        }
        ChunkHeader::Chunked {
            version,
            wallet_id,
            count,
            index,
        } => {
            println!("Type:            Chunked");
            println!("Version:         {version}");
            println!("Wallet ID:       0x{:05x}", wallet_id.as_u32());
            println!("Total chunks:    {count}");
            println!("Chunk index:     {index}");
        }
        // ChunkHeader is #[non_exhaustive]; this arm satisfies the compiler
        // for any future variants added by later format versions.
        _ => {
            println!("Type:            (unknown future variant)");
        }
    }

    Ok(())
}

fn cmd_vectors() -> Result<(), anyhow::Error> {
    let vectors = wdm_codec::vectors::build_test_vectors();
    let json = serde_json::to_string_pretty(&vectors)?;
    println!("{json}");
    Ok(())
}

fn cmd_bytecode(policy_str: &str) -> Result<(), anyhow::Error> {
    let policy: WalletPolicy = policy_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid policy: {e}"))?;

    let bytecode = policy
        .to_bytecode()
        .map_err(|e| anyhow::anyhow!("bytecode encode failed: {e}"))?;

    // One continuous lowercase hex string (easy to pipe).
    let hex: String =
        bytecode
            .iter()
            .fold(String::with_capacity(bytecode.len() * 2), |mut acc, b| {
                use std::fmt::Write;
                write!(acc, "{b:02x}").unwrap();
                acc
            });
    println!("{hex}");

    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Encode {
            policy,
            path,
            force_chunked,
            force_long_code,
            seed,
            json,
        } => cmd_encode(
            &policy,
            path.as_deref(),
            force_chunked,
            force_long_code,
            seed.as_deref(),
            json,
        ),

        Command::Decode { strings, json } => cmd_decode(&strings, json),

        Command::Verify { strings, policy } => cmd_verify(&strings, &policy),

        Command::Inspect { string } => cmd_inspect(&string),

        Command::Bytecode { policy } => cmd_bytecode(&policy),

        Command::Vectors => cmd_vectors(),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
