use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Codec(md_codec::Error),
    TemplateParse(String),
    BadXpub { i: u8, why: String },
    BadFingerprint { i: u8, why: String },
    Compile(String),
    Mismatch(String),
    BadArg(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Codec(e) => write!(f, "codec error: {e}"),
            CliError::TemplateParse(m) => write!(f, "template parse error: {m}"),
            CliError::BadXpub { i, why } => write!(f, "--key @{i}: {why}"),
            CliError::BadFingerprint { i, why } => write!(f, "--fingerprint @{i}: {why}"),
            CliError::Compile(m) => write!(f, "compile error: {m}"),
            CliError::Mismatch(m) => write!(f, "MISMATCH: {m}"),
            CliError::BadArg(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<md_codec::Error> for CliError {
    fn from(e: md_codec::Error) -> Self { CliError::Codec(e) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_bad_xpub() {
        let e = CliError::BadXpub { i: 2, why: "checksum failed".into() };
        assert_eq!(format!("{e}"), "--key @2: checksum failed");
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
