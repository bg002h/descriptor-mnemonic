//! Test vector generator binary.
//!
//! # Usage
//!
//! ```text
//! gen_vectors --output <path>   # generate vectors, write JSON to path (atomic)
//! gen_vectors --verify <path>   # verify a committed JSON file matches regenerated vectors
//! ```
//!
//! # Tasks covered
//!
//! - 8.2: clap setup + `--output`/`--verify` modes
//! - 8.3: `--output` mode
//! - 8.4: `--verify` mode

use std::path::PathBuf;
use std::process;

use clap::Parser;

/// WDM test vector generator.
///
/// Generate a deterministic JSON file of positive and negative test vectors
/// from the canonical corpus, or verify a committed file matches a
/// freshly-generated set.
#[derive(Debug, Parser)]
#[command(name = "gen_vectors", version, about, long_about = None)]
struct Args {
    /// Write generated test vectors to this path (atomic: write to `<path>.tmp` then rename).
    #[arg(long, conflicts_with = "verify")]
    output: Option<PathBuf>,

    /// Verify the JSON file at this path matches freshly-generated vectors.
    #[arg(long, conflicts_with = "output")]
    verify: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let result = match (args.output, args.verify) {
        (Some(path), None) => cmd_output(&path),
        (None, Some(path)) => cmd_verify(&path),
        (None, None) => {
            eprintln!("gen_vectors: one of --output or --verify is required");
            eprintln!("  gen_vectors --output <path>   write vectors to path");
            eprintln!("  gen_vectors --verify <path>   verify committed file");
            process::exit(2);
        }
        (Some(_), Some(_)) => unreachable!("clap conflicts_with prevents both"),
    };

    if let Err(e) = result {
        eprintln!("gen_vectors: error: {e}");
        process::exit(1);
    }
}

/// Generate vectors and write to the given path atomically.
fn cmd_output(path: &PathBuf) -> Result<(), anyhow::Error> {
    let vectors = wdm_codec::vectors::build_test_vectors();
    let mut json = serde_json::to_string_pretty(&vectors)
        .map_err(|e| anyhow::anyhow!("JSON serialization failed: {e}"))?;
    // Determinism: always end with a trailing newline (consistent with editors
    // and `diff`). `serde_json::to_string_pretty` does not add one.
    json.push('\n');

    // Atomic write: write to <path>.tmp then rename.
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("failed to write temp file {}: {e}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        anyhow::anyhow!(
            "failed to rename {} → {}: {e}",
            tmp_path.display(),
            path.display()
        )
    })?;

    eprintln!(
        "gen_vectors: wrote {} vectors + {} negative vectors to {}",
        vectors.vectors.len(),
        vectors.negative_vectors.len(),
        path.display()
    );
    Ok(())
}

/// Verify the committed JSON file matches freshly-generated vectors.
fn cmd_verify(path: &PathBuf) -> Result<(), anyhow::Error> {
    // 1. Read the committed file.
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;

    // 2. Deserialize the committed file.
    let committed: wdm_codec::TestVectorFile = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("failed to parse JSON from {}: {e}", path.display()))?;

    // 3. Regenerate in-memory.
    let regenerated = wdm_codec::vectors::build_test_vectors();

    // 4. Typed comparison with structured diagnostics.
    let mut mismatches: Vec<String> = Vec::new();

    if committed.schema_version != regenerated.schema_version {
        mismatches.push(format!(
            "schema_version: committed={}, regenerated={}",
            committed.schema_version, regenerated.schema_version
        ));
    }
    // Note: generator field intentionally not compared — it contains the
    // package version which may differ between runs (e.g., during dev).

    if committed.vectors.len() != regenerated.vectors.len() {
        mismatches.push(format!(
            "positive vector count: committed={}, regenerated={}",
            committed.vectors.len(),
            regenerated.vectors.len()
        ));
    } else {
        for (i, (c, r)) in committed
            .vectors
            .iter()
            .zip(regenerated.vectors.iter())
            .enumerate()
        {
            if c != r {
                mismatches.push(format!(
                    "positive vector[{}] id={:?} differs\n  committed:    {:?}\n  regenerated:  {:?}",
                    i, c.id, c, r
                ));
            }
        }
    }

    if committed.negative_vectors.len() != regenerated.negative_vectors.len() {
        mismatches.push(format!(
            "negative vector count: committed={}, regenerated={}",
            committed.negative_vectors.len(),
            regenerated.negative_vectors.len()
        ));
    } else {
        for (i, (c, r)) in committed
            .negative_vectors
            .iter()
            .zip(regenerated.negative_vectors.iter())
            .enumerate()
        {
            if c != r {
                mismatches.push(format!(
                    "negative vector[{}] id={:?} differs\n  committed:    {:?}\n  regenerated:  {:?}",
                    i, c.id, c, r
                ));
            }
        }
    }

    if mismatches.is_empty() {
        eprintln!(
            "gen_vectors: PASS — committed file matches regenerated vectors ({} positive, {} negative)",
            committed.vectors.len(),
            committed.negative_vectors.len()
        );
        Ok(())
    } else {
        eprintln!("gen_vectors: FAIL — {} mismatch(es):", mismatches.len());
        for m in &mismatches {
            eprintln!("  - {m}");
        }
        Err(anyhow::anyhow!(
            "committed vectors at {} do not match regenerated output",
            path.display()
        ))
    }
}
