use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Codec(md_codec::Error),
    /// A `@N`-template render failure from `md_codec`'s canonical renderer.
    /// Distinct from [`CliError::Codec`] (which wraps `md_codec::Error`, the
    /// wire/decode taxonomy): `md_codec::RenderError` is a separate type for
    /// the renderer's fail-closed structural guards.
    Render(md_codec::RenderError),
    TemplateParse(String),
    BadXpub {
        i: u8,
        why: String,
    },
    BadFingerprint {
        i: u8,
        why: String,
    },
    /// Malformed BIP-380 origin notation on a `--key @i=[fp/path]xpub` value
    /// (P1 — `design/SPEC_wallet_form_converter.md` "The three pieces").
    /// Distinct from [`CliError::BadXpub`]'s bare "base58check decode"
    /// error: this variant fires while parsing the `[fp/path]` bracket
    /// itself, before the xpub half is ever decoded, so the message can
    /// name the origin-notation grammar the value failed rather than
    /// blaming the base58 payload (the motivation refusal 3 fix).
    ///
    /// C0 landed `parse_key_with_origin` (the only constructor) as a
    /// standalone parser; C1 wires it into `--key` flag handling on
    /// `descriptor`/`address` (`cmd/build.rs::build_descriptor`).
    BadOrigin {
        i: u8,
        why: String,
    },
    /// Reserved for `cli-compiler` feature paths (`compile` subcommand,
    /// `encode --from-policy`). Constructed only when that feature is on;
    /// `#[allow(dead_code)]` keeps default-features clippy clean.
    #[allow(dead_code)]
    Compile(String),
    Mismatch(String),
    BadArg(String),
    /// `md compose` refusals: structural (§4e), lock band (§4c), DSL, or an
    /// EXPERIMENTAL shape without `--experimental`. The message is complete in
    /// itself; `main`'s generic arm prefixes `md: ` and exits 1 (`BadArg`,
    /// above, is the one variant with its own arm, exiting 2).
    Compose(String),
    /// A refusal from the seating engine (SPEC "NORMATIVE — the seating
    /// engine": A1-A5, B1-B2), including its input pipeline.
    ///
    /// ONE variant for the whole engine, deliberately: a seating refusal is
    /// a statement about the CARD SET and the POLICY together, and the
    /// message — which names cards by full chunk-set id, slots by index and
    /// declared origin, and the remedies — is the contract the vector rows
    /// pin. Splitting the taxonomy across variants would move it into the
    /// type, where the rows would stop checking it.
    ///
    /// Exit code 1 (a content refusal), not 2 (a usage error): the flags
    /// were spelled correctly; it is the material the engine declines.
    Seat(String),
    /// BIP 388's pairwise-distinctness rule refused on the COMPOSE side of the template row:
    /// `--template` + `--key` handed one extended key to two distinct `@N`
    /// slots at the same use-site (SPEC A3's shape (1)).
    ///
    /// Its own variant rather than [`CliError::Codec`] because the message is
    /// SPEC A3's diagnostic contract — cite BIP 388, say UNSUPPORTED, never
    /// call the input invalid — which `md_codec::Error::DuplicateKeySlots`
    /// (the same defect, worded for the wire layer) does not carry, and
    /// because it must not be [`CliError::BadArg`]: the flags were spelled
    /// correctly, so this is a content refusal at exit 1 like its `Seat` and
    /// `Decompose` siblings, not a usage error at exit 2.
    KeyReuse(String),
    /// N1's placeholder/key-reuse admission taxonomy
    /// (`design/SPEC_mdcli_mini.md` §N1, [`crate::parse::reuse`]).
    ///
    /// ONE variant for the whole taxonomy, for the reason [`CliError::Seat`]
    /// and [`CliError::Decompose`] are one each: the contract the vector rows
    /// pin is the RENDERED LINE, and a taxonomy split across variants would
    /// move it into the type, where the rows stop checking it.
    ///
    /// **It exists because [`CliError::TemplateParse`] could not carry these
    /// messages.** R-N1c refuses a wallet BIP 388 explicitly permits — md1
    /// simply cannot write it (F-417) — and "md: template parse error:" blames
    /// an input that is not at fault. The spec makes the prefix normative for
    /// exactly that reason: it must read as a statement about md's capability,
    /// not about the operator's template. `unsupported` is also the
    /// Principle's word: a BIP-forbidden or wire-inexpressible shape is
    /// UNSUPPORTED, never "invalid".
    ///
    /// Exit code 1, like its `Seat`/`Decompose`/`KeyReuse` siblings: the flags
    /// were spelled correctly; it is the material md declines.
    Unsupported(String),
    /// A refusal from `md decompose` (SPEC "P3 — the concrete descriptor
    /// becomes an entrance"), including its input boundary.
    ///
    /// ONE variant for the whole verb, for the same reason [`CliError::Seat`]
    /// is one for the engine: a P3 refusal is a statement about the SUPPLIED
    /// DESCRIPTOR, and its text — which names the BIP 388 rule, the positions,
    /// the remedy, and never calls the input "invalid" — is the contract the
    /// V-D-* rows pin. A taxonomy in the type would move that contract
    /// somewhere the rows stop checking it.
    ///
    /// Exit code 1 (a content refusal), not 2: the flags were spelled
    /// correctly; it is the material decompose declines.
    Decompose(String),
    /// `--verify-against`'s argument did not decode, NAMING which branch of
    /// `resolve_verify_against`'s existence check ran (whole-diff review r1
    /// N4). `Path::new(arg).is_file()` decides file-vs-literal before either
    /// branch can fail, so by the time this variant is built the branch is
    /// already known; without naming it, an operator whose cwd happens to
    /// hold a file matching their pasted md1 string sees md reject the
    /// string they typed, and an operator with a mistyped path sees md
    /// reject it as a string rather than say no such file exists.
    ///
    /// Exit code 1, same as the decode error it wraps: the message is
    /// enriched, the disposition (a decode failure, never a spend-equality
    /// verdict) is unchanged.
    VerifyAgainstUnreadable(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Codec(e) => write!(f, "codec error: {e}"),
            CliError::Render(e) => write!(f, "template render error: {e}"),
            CliError::TemplateParse(m) => write!(f, "template parse error: {m}"),
            CliError::BadXpub { i, why } => write!(f, "--key @{i}: {why}"),
            CliError::BadFingerprint { i, why } => write!(f, "--fingerprint @{i}: {why}"),
            CliError::BadOrigin { i, why } => write!(f, "--key @{i}: origin notation {why}"),
            CliError::Compile(m) => write!(f, "compile error: {m}"),
            CliError::Mismatch(m) => write!(f, "MISMATCH: {m}"),
            CliError::BadArg(m) => write!(f, "{m}"),
            CliError::Compose(m) => write!(f, "{m}"),
            CliError::Seat(m) => write!(f, "seating refused: {m}"),
            CliError::KeyReuse(m) => write!(f, "key reuse refused: {m}"),
            CliError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CliError::Decompose(m) => write!(f, "decompose: {m}"),
            CliError::VerifyAgainstUnreadable(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<md_codec::Error> for CliError {
    fn from(e: md_codec::Error) -> Self {
        CliError::Codec(e)
    }
}

impl From<md_codec::RenderError> for CliError {
    fn from(e: md_codec::RenderError) -> Self {
        CliError::Render(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_bad_xpub() {
        let e = CliError::BadXpub {
            i: 2,
            why: "checksum failed".into(),
        };
        assert_eq!(format!("{e}"), "--key @2: checksum failed");
    }

    #[test]
    fn display_bad_origin() {
        let e = CliError::BadOrigin {
            i: 0,
            why: "fingerprint must be 8 hex chars, got `zzzz`".into(),
        };
        assert_eq!(
            format!("{e}"),
            "--key @0: origin notation fingerprint must be 8 hex chars, got `zzzz`"
        );
    }

    #[test]
    fn display_mismatch() {
        let e = CliError::Mismatch("policy id differs".into());
        assert_eq!(format!("{e}"), "MISMATCH: policy id differs");
    }

    #[test]
    fn from_codec_wraps() {
        let codec_err = md_codec::Error::ChunkSetIdOutOfRange { id: 0xFFFFFF };
        let cli_err: CliError = codec_err.into();
        assert!(matches!(cli_err, CliError::Codec(_)));
    }
}
