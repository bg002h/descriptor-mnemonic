//! MD CLI binary — encode, decode, verify, inspect, and bytecode subcommands.
//!
//! # Tasks covered
//!
//! - 7.1: binary skeleton (clap derive, subcommand dispatch)
//! - 7.2: `md encode`
//! - 7.3: `md decode`
//! - 7.4: `md verify`
//! - 7.5: `md inspect`
//! - 7.6: `md bytecode`
//! - 7.7: `md vectors` — implemented in Phase 8 (shares `build_test_vectors` with `gen_vectors`)
//! - 7.8: `--path` argument parser for `md encode`

use std::process;
use std::str::FromStr;

use bitcoin::bip32::{DerivationPath, Fingerprint};
use clap::{Parser, Subcommand};
use md_codec::{
    BchCode, ChunkHeader, DecodeOptions, EncodeOptions, WalletPolicy, decode, decode_string,
    encode, five_bit_to_bytes, wallet_id::WalletIdSeed,
};

mod json;
use json::{DecodeJson, EncodeJson};

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

/// Mnemonic Descriptor (MD) — engravable BIP 388 wallet policy backups.
#[derive(Debug, Parser)]
#[command(name = "md", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup strings.
    ///
    /// WARNING: This tool encodes any BIP 388 wallet policy. It does NOT
    /// check whether the policy is signable on any particular hardware
    /// wallet — that is your responsibility. See the project README for
    /// the responsibility-chain framing.
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

        /// Embed a master-key fingerprint for placeholder `@INDEX`, repeatable
        /// for each placeholder. Format: `@0=deadbeef` (8 hex chars; 0x prefix
        /// optional). All placeholders must be supplied (BIP §"Fingerprints
        /// block" MUST clause). Indices must cover 0..N-1 with no gaps.
        ///
        /// Privacy: fingerprints leak which seeds match which @i placeholders.
        /// The CLI prints a stderr warning when this flag is used.
        #[arg(long = "fingerprint", value_name = "@INDEX=HEX")]
        fingerprints: Vec<String>,

        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },

    /// Decode one or more MD backup strings into a wallet policy.
    Decode {
        /// One or more MD backup strings (all chunks of one backup, any order).
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,

        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },

    /// Decode and verify backup strings match an expected policy.
    Verify {
        /// One or more MD backup strings.
        #[arg(required = true, num_args = 1..)]
        strings: Vec<String>,

        /// The expected wallet policy.
        #[arg(long, required = true)]
        policy: String,
    },

    /// Inspect a single MD backup string (chunk header only, no full decode).
    Inspect {
        /// A single MD backup string.
        string: String,
    },

    /// Hex-dump the canonical MD bytecode for a wallet policy.
    Bytecode {
        /// The wallet policy (BIP 388 template string).
        policy: String,
    },

    /// Generate test vectors as JSON and print to stdout.
    ///
    /// Outputs the same content as `gen_vectors --output -` (write mode prints to stderr).
    /// Use `md vectors > vectors.json` to capture the output.
    Vectors,

    /// Compile a Concrete-Policy expression and emit the resulting MD bytecode hex.
    ///
    /// Available only when the `cli-compiler` feature is enabled. The
    /// input policy uses rust-miniscript's high-level Concrete-Policy
    /// syntax with fully-qualified `DescriptorPublicKey` strings — NOT
    /// BIP 388 `@N/**` placeholders. The compiler picks an optimal
    /// miniscript shape; the wrapper projects to a wallet policy and
    /// runs the standard encode pipeline.
    #[cfg(feature = "compiler")]
    FromPolicy {
        /// The Concrete-Policy expression (e.g. `or(pk(<xpub1>),pk(<xpub2>))`).
        policy: String,

        /// Script context: `tap` (tapscript) or `segwitv0` (`wsh`).
        #[arg(long, value_name = "tap|segwitv0", default_value = "segwitv0")]
        context: String,

        /// Tap-context internal key. If omitted, rust-miniscript synthesises
        /// an unspendable NUMS key for script-path-only spends. Ignored for
        /// `segwitv0`.
        #[arg(long, value_name = "DESCRIPTOR_PUBLIC_KEY")]
        internal_key: Option<String>,
    },
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
    ("bip48-nested", 0x06),
    ("bip87", 0x07),
    // Testnet variants
    ("bip44t", 0x11),
    ("bip49t", 0x12),
    ("bip84t", 0x13),
    ("bip86t", 0x14),
    ("bip48t", 0x15),
    ("bip48-nestedt", 0x16),
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
                "indicator 0x{indicator:02x} is not in the MD path dictionary; \
                 known indicators: 0x01-0x07, 0x11-0x15, 0x17. \
                 Use a literal path like \"m/48'/0'/0'/2'\" instead."
            )
        });
    }

    // 3. Literal derivation path.
    DerivationPath::from_str(s).map_err(|e| anyhow::anyhow!("invalid derivation path {s:?}: {e}"))
}

/// Look up the `DerivationPath` for a known indicator byte using the path
/// dictionary in `md_codec::bytecode::path`.
fn indicator_to_derivation_path(indicator: u8) -> Option<DerivationPath> {
    md_codec::bytecode::path::indicator_to_path(indicator).cloned()
}

// ---------------------------------------------------------------------------
// Fingerprint argument parser (v0.2.1 — `phase-e-cli-fingerprint-flag`)
// ---------------------------------------------------------------------------

/// Parse one `--fingerprint @INDEX=HEX` argument. Returns `(index, fp)`.
///
/// Accepted forms:
/// - `@0=deadbeef` (canonical)
/// - `0=deadbeef` (`@` optional)
/// - `@1=0xcafebabe` (0x prefix optional)
///
/// `INDEX` must be a non-negative integer. `HEX` must be exactly 8 lowercase
/// hex characters representing a 4-byte master-key fingerprint.
fn parse_fingerprint_arg(arg: &str) -> Result<(usize, Fingerprint), anyhow::Error> {
    let (idx_part, hex_part) = arg.split_once('=').ok_or_else(|| {
        anyhow::anyhow!("--fingerprint: expected '@INDEX=HEX' (e.g. '@0=deadbeef'), got {arg:?}")
    })?;
    let idx_str = idx_part.strip_prefix('@').unwrap_or(idx_part);
    let index: usize = idx_str.parse().map_err(|_| {
        anyhow::anyhow!(
            "--fingerprint: index {idx_str:?} must be a non-negative integer (e.g. '@0=...')"
        )
    })?;
    let hex_clean = hex_part.to_ascii_lowercase();
    let hex_clean = hex_clean.strip_prefix("0x").unwrap_or(&hex_clean);
    if hex_clean.len() != 8 {
        anyhow::bail!(
            "--fingerprint: hex {hex_part:?} must be exactly 8 hex chars (4 bytes); got {} chars",
            hex_clean.len()
        );
    }
    let mut bytes = [0u8; 4];
    for i in 0..4 {
        let byte_str = &hex_clean[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(byte_str, 16).map_err(|_| {
            anyhow::anyhow!("--fingerprint: invalid hex byte {byte_str:?} in {hex_part:?}")
        })?;
    }
    Ok((index, Fingerprint::from(bytes)))
}

/// Parse a `Vec<String>` of `--fingerprint` arguments into the
/// `Vec<Fingerprint>` shape `EncodeOptions::fingerprints` expects.
///
/// Validates that the supplied indices cover `0..N` with no gaps, no
/// duplicates, and starting at `0`. Returns the fingerprints in placeholder
/// index order.
fn parse_fingerprints_args(args: &[String]) -> Result<Vec<Fingerprint>, anyhow::Error> {
    let mut by_index: Vec<Option<Fingerprint>> = Vec::new();
    for arg in args {
        let (index, fp) = parse_fingerprint_arg(arg)?;
        if index >= by_index.len() {
            by_index.resize(index + 1, None);
        }
        if by_index[index].is_some() {
            anyhow::bail!("--fingerprint: duplicate entry for index {index}");
        }
        by_index[index] = Some(fp);
    }
    let mut out = Vec::with_capacity(by_index.len());
    for (i, slot) in by_index.into_iter().enumerate() {
        match slot {
            Some(fp) => out.push(fp),
            None => anyhow::bail!(
                "--fingerprint: missing entry for index {i} (indices must cover 0..N with no gaps)"
            ),
        }
    }
    Ok(out)
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
    fingerprint_args: &[String],
    json: bool,
) -> Result<(), anyhow::Error> {
    // Parse the policy.
    let policy: WalletPolicy = policy_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid policy: {e}"))?;

    // Parse --path override if present. Phase B (v0.2) wires this through
    // `EncodeOptions::shared_path`, which `WalletPolicy::to_bytecode` now
    // honors as the highest-precedence shared-path source. See
    // `IMPLEMENTATION_PLAN_v0.2.md` Phase B and the precedence rule in the
    // `to_bytecode` rustdoc.
    let shared_path_override = match path_arg {
        Some(path_str) => {
            Some(parse_path_arg(path_str).map_err(|e| anyhow::anyhow!("--path: {e}"))?)
        }
        None => None,
    };

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

    // Parse --fingerprint arguments. v0.2.1 (`phase-e-cli-fingerprint-flag`).
    // Empty Vec → no fingerprints block (header byte 0x00, v0.1 wire output);
    // non-empty → header bit 2 = 1, validated against the policy's placeholder
    // count by the encoder per BIP §"Fingerprints block".
    let fingerprints: Option<Vec<Fingerprint>> = if fingerprint_args.is_empty() {
        None
    } else {
        // BIP MUST clause: privacy-warn before encoding. The CLI is a
        // recovery tool per the BIP; the warning is mandatory.
        eprintln!(
            "warning: --fingerprint embeds master-key fingerprints into the backup. \
             This leaks which seeds match which @i placeholders. \
             Only use if recovery requires the disclosure."
        );
        Some(parse_fingerprints_args(fingerprint_args)?)
    };

    let mut opts = EncodeOptions::default();
    opts = opts.with_force_chunking(force_chunked);
    opts.force_long_code = force_long_code;
    opts.wallet_id_seed = wallet_id_seed;
    opts.shared_path = shared_path_override;
    opts.fingerprints = fingerprints;

    let backup = encode(&policy, &opts).map_err(|e| anyhow::anyhow!("encode failed: {e}"))?;

    if json {
        // v0.2: derive-based wrappers (see `json` module). Output is
        // byte-identical to v0.1.1's hand-built `serde_json::json!{}`
        // literal — wrapper field order is alphabetical so
        // `serde_json::to_string_pretty` reproduces the same key order
        // that BTreeMap-backed `json!{}` produced.
        let out = EncodeJson::from(&backup);
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
        // v0.2: derive-based wrappers (see `json` module). Output is
        // byte-identical to v0.1.1's hand-built `serde_json::json!{}`
        // literal.
        let out = DecodeJson::from(&result);
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
    let bytes = five_bit_to_bytes(decoded.data())
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

#[cfg(feature = "compiler")]
fn cmd_from_policy(
    policy_str: &str,
    context_str: &str,
    internal_key_str: Option<&str>,
) -> Result<(), anyhow::Error> {
    use md_codec::{ScriptContext, policy_to_bytecode};

    let context = match context_str.to_ascii_lowercase().as_str() {
        "segwitv0" | "wsh" => ScriptContext::Segwitv0,
        "tap" | "tr" => ScriptContext::Tap,
        other => {
            anyhow::bail!("--context must be one of: segwitv0, wsh, tap, tr; got {other:?}");
        }
    };

    let internal_key = match internal_key_str {
        Some(s) => Some(
            s.parse::<miniscript::DescriptorPublicKey>()
                .map_err(|e| anyhow::anyhow!("--internal-key parse failed: {e}"))?,
        ),
        None => None,
    };

    let bytecode = policy_to_bytecode(policy_str, &EncodeOptions::default(), context, internal_key)
        .map_err(|e| anyhow::anyhow!("policy compile/encode failed: {e}"))?;

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

fn cmd_vectors() -> Result<(), anyhow::Error> {
    let vectors = md_codec::vectors::build_test_vectors();
    let json = serde_json::to_string_pretty(&vectors)?;
    println!("{json}");
    Ok(())
}

fn cmd_bytecode(policy_str: &str) -> Result<(), anyhow::Error> {
    let policy: WalletPolicy = policy_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid policy: {e}"))?;

    // Intentionally hardcoded `EncodeOptions::default()` — `bytecode` is a
    // debug-aid subcommand that prints the canonical default-options encoding
    // for spec/BIP cross-reference. Unlike `cmd_encode`, it deliberately does
    // not expose `--path` / `--fingerprint` / etc. so the printed bytes always
    // reflect the BIP 84 mainnet fallback path with no fingerprints block.
    let bytecode = policy
        .to_bytecode(&EncodeOptions::default())
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
            fingerprints,
            json,
        } => cmd_encode(
            &policy,
            path.as_deref(),
            force_chunked,
            force_long_code,
            seed.as_deref(),
            &fingerprints,
            json,
        ),

        Command::Decode { strings, json } => cmd_decode(&strings, json),

        Command::Verify { strings, policy } => cmd_verify(&strings, &policy),

        Command::Inspect { string } => cmd_inspect(&string),

        Command::Bytecode { policy } => cmd_bytecode(&policy),

        Command::Vectors => cmd_vectors(),

        #[cfg(feature = "compiler")]
        Command::FromPolicy {
            policy,
            context,
            internal_key,
        } => cmd_from_policy(&policy, &context, internal_key.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
