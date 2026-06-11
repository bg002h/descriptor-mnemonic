//! Output-class stderr advisory (Phase 2 sibling sweep).
//!
//! Byte-for-byte duplicate of mnemonic-toolkit's
//! `secret_advisory::emit_output_class_advisory`. md-cli is upstream of the
//! toolkit and cannot depend on it, so the helper is duplicated; cross-repo
//! byte parity is enforced by `tests/cli_output_class.rs::byte_parity_advisory_lines`.

use std::io::Write;

/// Security class of what a command wrote to stdout. Byte-identical variant set
/// to mnemonic-toolkit's `secret_advisory::OutputClass`.
///
/// `#[allow(dead_code)]`: md-cli is a bin-only crate; `PrivateKeyMaterial` is
/// never constructed (md emits only `Template`/`WatchOnly`) but is kept for
/// advisory-text parity (guarded by the byte-parity test).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputClass {
    PrivateKeyMaterial,
    WatchOnly,
    Template,
}

/// Emit the one-line stderr class advisory. Byte-identical to mnemonic-toolkit's
/// `secret_advisory::emit_output_class_advisory` (cross-repo parity — see the
/// byte-parity test). Inert outputs do NOT call this.
pub fn emit_output_class_advisory<W: Write>(class: OutputClass, stderr: &mut W) {
    let line = match class {
        OutputClass::PrivateKeyMaterial => {
            "warning: stdout carries private key material (can spend) \u{2014} redirect or encrypt (e.g. '> file.txt' or '| age -e ...')"
        }
        OutputClass::WatchOnly => {
            "note: stdout is watch-only \u{2014} public keys only, cannot spend"
        }
        OutputClass::Template => "note: stdout is a keyless descriptor template (no keys)",
    };
    let _ = writeln!(stderr, "{line}");
}
