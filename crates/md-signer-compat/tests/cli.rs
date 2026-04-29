//! Integration tests for the `md-signer-compat` CLI binary.
//!
//! Gated to `cli` feature builds (the binary itself requires it).

#![cfg(feature = "cli")]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn list_signers_prints_both_named_subsets() {
    Command::cargo_bin("md-signer-compat")
        .expect("binary built")
        .args(["list-signers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Coldcard tap-leaf"))
        .stdout(predicate::str::contains("Ledger tap-leaf"));
}

#[test]
fn validate_unknown_signer_name_errors() {
    Command::cargo_bin("md-signer-compat")
        .expect("binary built")
        .args([
            "validate",
            "--signer",
            "trezor", // not a recognised name
            "--bytecode-hex",
            "00", // payload doesn't matter — fails on signer parse first
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--signer must be one of"));
}

/// `wsh(pk(@0/**))` has no tap leaves; running `validate` against it should
/// pass trivially with the "no tap leaves to validate" message.
#[test]
fn validate_non_tr_passes_trivially() {
    use md_codec::{EncodeOptions, WalletPolicy};

    let policy: WalletPolicy = "wsh(pk(@0/**))".parse().unwrap();
    let bytecode = policy.to_bytecode(&EncodeOptions::default()).unwrap();
    let hex_str: String =
        bytecode
            .iter()
            .fold(String::with_capacity(bytecode.len() * 2), |mut acc, b| {
                use std::fmt::Write;
                write!(acc, "{b:02x}").unwrap();
                acc
            });

    Command::cargo_bin("md-signer-compat")
        .expect("binary built")
        .args([
            "validate",
            "--signer",
            "coldcard",
            "--bytecode-hex",
            &hex_str,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}
