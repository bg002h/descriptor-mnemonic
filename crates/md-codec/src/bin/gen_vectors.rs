//! Test vector generator binary.
//!
//! # Usage
//!
//! ```text
//! gen_vectors --output <path>             # generate schema-2 vectors → JSON (atomic)
//! gen_vectors --output <path> --schema 1  # generate schema-1 vectors (v0.1.json regen)
//! gen_vectors --verify <path>             # verify a committed JSON file
//! ```
//!
//! `--verify` reads the file's `schema_version` field first and dispatches
//! to the matching builder; callers do not pass `--schema`.
//!
//! # Tasks covered
//!
//! - 8.2: clap setup + `--output`/`--verify` modes
//! - 8.3: `--output` mode
//! - 8.4: `--verify` mode
//! - Phase F (F-9): `--schema` arg, schema-aware verify, dual-builder
//!   dispatch.

use std::path::PathBuf;
use std::process;

use clap::Parser;

/// MD test vector generator.
///
/// Generate a deterministic JSON file of positive and negative test vectors
/// from the canonical corpus, or verify a committed file matches a
/// freshly-generated set. Two schema versions are supported:
///
/// - `--schema 1` regenerates `tests/vectors/v0.1.json` byte-identical
///   (the v0.1.0 lock).
/// - `--schema 2` (default) writes `tests/vectors/v0.2.json` (the v0.2.0
///   lock; superset of schema 1 with taproot + fingerprints corpus
///   additions and per-variant generated negative vectors).
#[derive(Debug, Parser)]
#[command(name = "gen_vectors", version, about, long_about = None)]
struct Args {
    /// Write generated test vectors to this path (atomic: write to `<path>.tmp` then rename).
    #[arg(long, conflicts_with = "verify")]
    output: Option<PathBuf>,

    /// Verify the JSON file at this path matches freshly-generated vectors.
    /// The schema version is inferred from the file's `schema_version` field.
    #[arg(long, conflicts_with = "output")]
    verify: Option<PathBuf>,

    /// Schema version to emit when `--output` is set: `1` for the v0.1.0
    /// lock, `2` for the v0.2.0 lock. Defaults to `2`. Ignored with
    /// `--verify`.
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=2))]
    schema: u32,
}

fn main() {
    let args = Args::parse();

    let result = match (args.output, args.verify) {
        (Some(path), None) => cmd_output(&path, args.schema),
        (None, Some(path)) => cmd_verify(&path),
        (None, None) => {
            eprintln!("gen_vectors: one of --output or --verify is required");
            eprintln!("  gen_vectors --output <path>             write schema-2 vectors to path");
            eprintln!(
                "  gen_vectors --output <path> --schema 1  write schema-1 vectors (v0.1 regen)"
            );
            eprintln!(
                "  gen_vectors --verify <path>             verify committed file (schema inferred)"
            );
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
fn cmd_output(path: &PathBuf, schema: u32) -> Result<(), anyhow::Error> {
    let vectors = build_for_schema(schema)?;

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
        "gen_vectors: wrote schema-{} ({} vectors + {} negative vectors) to {}",
        vectors.schema_version,
        vectors.vectors.len(),
        vectors.negative_vectors.len(),
        path.display()
    );
    // The JSON's `generator` field embeds only the family version
    // (`"md-codec 0.X"`) so the file SHA stays stable across patch bumps.
    // The full crate version is logged here for traceability — useful when
    // a contributor regenerates a file and wants to know which exact build
    // produced it, without touching the on-disk SHA.
    eprintln!(
        "gen_vectors: family generator = {:?}; full crate version = {:?}",
        vectors.generator,
        env!("CARGO_PKG_VERSION"),
    );
    Ok(())
}

/// Verify the committed JSON file matches freshly-generated vectors.
///
/// Schema version is inferred from the file's `schema_version` field; the
/// matching builder is invoked.
fn cmd_verify(path: &PathBuf) -> Result<(), anyhow::Error> {
    // 1. Read the committed file.
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;

    // 2. Deserialize the committed file.
    let committed: md_codec::TestVectorFile = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("failed to parse JSON from {}: {e}", path.display()))?;

    // 3. Regenerate in-memory using the file's declared schema version.
    let regenerated = build_for_schema(committed.schema_version)?;

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
            "gen_vectors: PASS — committed file matches regenerated schema-{} vectors ({} positive, {} negative)",
            committed.schema_version,
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

/// Dispatch to the matching schema builder.
fn build_for_schema(schema: u32) -> Result<md_codec::TestVectorFile, anyhow::Error> {
    match schema {
        1 => Ok(md_codec::vectors::build_test_vectors_v1()),
        2 => Ok(md_codec::vectors::build_test_vectors_v2()),
        other => Err(anyhow::anyhow!(
            "unsupported schema version {other}; supported: 1, 2"
        )),
    }
}
