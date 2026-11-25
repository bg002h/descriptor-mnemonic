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

/// Bug hunt 2026-11-25 md#6: the BIP's MUSTs on fingerprints
/// (`bip/bip-mnemonic-descriptor.mediawiki`, the `Fingerprints` TLV paragraph): "Recovery tools MUST
/// warn the user before including fingerprints" and "Decoders that surface
/// parsed fingerprints to the user MUST flag the data as privacy-sensitive".
/// Silent when the card carries no fingerprint. stderr only.
pub fn emit_fingerprint_privacy_note<W: Write>(
    d: &md_codec::encode::Descriptor,
    minting: bool,
    stderr: &mut W,
) {
    if d.tlv.fingerprints.as_deref().unwrap_or_default().is_empty() {
        return;
    }
    let line = if minting {
        "warning: this card will carry master-key fingerprints. A fingerprint identifies \
         the seed behind a key and links the card to that seed's other wallets: anyone who \
         reads the card learns it. Leave the fingerprints out if you do not need them to \
         tell slots apart."
    } else {
        "note: the fingerprints shown are privacy-sensitive: a fingerprint identifies the \
         seed behind a key and links this card to that seed's other wallets."
    };
    let _ = writeln!(stderr, "{line}");
}

/// Bug hunt 2026-11-25 md#1 (Brian's ruling): the slots of a KEYED card that
/// carry no origin path. Empty for a keyless template, where an empty origin
/// on a canonical wrapper is an elision the BIP resolves to the canonical one.
pub fn keyed_slots_without_origin(d: &md_codec::encode::Descriptor) -> Vec<u8> {
    if !d.is_wallet_policy() {
        return Vec::new();
    }
    let mut idx = d.unresolved_origin_indices();
    if let Ok(expanded) = md_codec::canonicalize::expand_per_at_n(d) {
        idx.extend(
            expanded
                .iter()
                .filter(|e| e.origin_path.components.is_empty())
                .map(|e| e.idx),
        );
    }
    idx.sort_unstable();
    idx.dedup();
    idx
}

/// Bug hunt 2026-11-25 md#1: the loud warning a reader without the seed
/// prints for a keyed card that carries no origin path. md never guesses the
/// path (not the master node, not the shape's standard path). stderr only.
pub fn emit_missing_origin_warning<W: Write>(d: &md_codec::encode::Descriptor, stderr: &mut W) {
    let idx = keyed_slots_without_origin(d);
    if idx.is_empty() {
        return;
    }
    let slots = idx
        .iter()
        .map(|i| format!("@{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        stderr,
        "warning: the origin path of {slots} is NOT on this card, and md does not guess it \
         (it is neither the master key nor assumed to be the standard path). A signer will not \
         recognise the key until the path it was derived at is supplied. Addresses do not \
         depend on the path."
    );
}
