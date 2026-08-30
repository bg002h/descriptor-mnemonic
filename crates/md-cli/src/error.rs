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
