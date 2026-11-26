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
    /// A comparison came out no: `md verify`'s MISMATCH (exit 1, NO).
    Mismatch(String),
    /// The command line alone is wrong, judged without reading any input:
    /// a missing argument, flags that cannot go together, a bad setting.
    /// Exit 64 (USAGE). A malformed INPUT value on argv is [`CliError::Input`].
    Usage(String),
    /// An input is wrong or insufficient for this verb: empty input, a
    /// malformed input value, a file that is not text. Exit 2 (INVALID).
    Input(String),
    /// Something needed from the system failed: an input file that cannot be
    /// read, an output that cannot be written, a feature this build lacks.
    /// Exit 32 (IO).
    Io(String),
    /// A refusal a flag or a different destination waives (`--experimental`,
    /// `--md-only`, an output that is one of the inputs). Exit 8 (REFUSED).
    Refused(String),
    /// A key or card on another network than `--network` says, or a set that
    /// holds both kinds. Exit 8 (REFUSED; the network rule's bit).
    Network(String),
    /// A bug: a failure no class covers. Exit 101 (INTERNAL).
    Internal(String),
    /// `md compose` refusals: structural (§4e), lock band (§4c), DSL, or an
    /// DSL. The message is complete in itself; `main` prefixes `md: `. Exit 2:
    /// the wrapper, paths and preset are the wallet's shape, an input. (An
    /// EXPERIMENTAL shape without `--experimental`, and `--md-only`'s refusal,
    /// are [`CliError::Refused`], 8: a flag waives them.)
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
    /// Exit code 2 (an input the engine declines), not 64 (a usage error):
    /// the flags were spelled correctly; it is the material the engine
    /// declines. Two exceptions are built as other variants: a card of the
    /// wrong network ([`CliError::Network`], 8) and a `--from-mk1-file` that
    /// cannot be read ([`CliError::Io`], 32).
    Seat(String),
    /// BIP 388's pairwise-distinctness rule refused on the COMPOSE side of the template row:
    /// `--template` + `--key` handed one extended key to two distinct `@N`
    /// slots at the same use-site (SPEC A3's shape (1)).
    ///
    /// Its own variant rather than [`CliError::Codec`] because the message is
    /// SPEC A3's diagnostic contract — cite BIP 388, say UNSUPPORTED, never
    /// call the input invalid — which `md_codec::Error::DuplicateKeySlots`
    /// (the same defect, worded for the wire layer) does not carry, and
    /// because it must not be [`CliError::Usage`]: the flags were spelled
    /// correctly, so this is an input refusal at exit 2 like its `Seat` and
    /// `Decompose` siblings, not a usage error at exit 64.
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
    /// Exit code 2, like its `Seat`/`Decompose`/`KeyReuse` siblings: the flags
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
    /// Exit code 2 (an input refusal), not 64: the flags were spelled
    /// correctly; it is the material decompose declines. A key of the wrong
    /// network is [`CliError::Network`] (8).
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
    /// Exit code 2, same as the decode error it wraps: the message is
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
            CliError::Usage(m)
            | CliError::Input(m)
            | CliError::Io(m)
            | CliError::Refused(m)
            | CliError::Network(m)
            | CliError::Internal(m) => write!(f, "{m}"),
            CliError::Compose(m) => write!(f, "{m}"),
            CliError::Seat(m) => write!(f, "seating refused: {m}"),
            CliError::KeyReuse(m) => write!(f, "key reuse refused: {m}"),
            CliError::Unsupported(m) => write!(f, "unsupported: {m}"),
            CliError::Decompose(m) => write!(f, "decompose: {m}"),
            CliError::VerifyAgainstUnreadable(m) => write!(f, "{m}"),
        }
    }
}

impl CliError {
    /// The exit status (`SPEC_constellation_exit_codes.md` §3), by the
    /// variant alone. Exhaustive on purpose (spec R13): a new variant must be
    /// given its class here, and `every_variant_maps_to_its_class` pins it.
    pub fn exit_code(&self) -> u8 {
        use crate::exit;
        match self {
            CliError::Mismatch(_) => exit::NO,
            CliError::Codec(_)
            | CliError::Render(_)
            | CliError::TemplateParse(_)
            | CliError::BadXpub { .. }
            | CliError::BadFingerprint { .. }
            | CliError::BadOrigin { .. }
            | CliError::Compile(_)
            | CliError::Compose(_)
            | CliError::Seat(_)
            | CliError::KeyReuse(_)
            | CliError::Unsupported(_)
            | CliError::Decompose(_)
            | CliError::VerifyAgainstUnreadable(_)
            | CliError::Input(_) => exit::INVALID,
            CliError::Refused(_) | CliError::Network(_) => exit::REFUSED,
            CliError::Io(_) => exit::IO,
            CliError::Usage(_) => exit::USAGE,
            CliError::Internal(_) => exit::INTERNAL,
        }
    }

    /// An input file that could not be read: a file that is not UTF-8 text is
    /// an invalid input (2); any other failure (missing, unreadable, a
    /// directory) is the system's (32). `what` names the flag and path.
    pub fn read_failure(what: std::fmt::Arguments<'_>, e: &std::io::Error) -> CliError {
        if e.kind() == std::io::ErrorKind::InvalidData {
            CliError::Input(format!("{what}: {e}"))
        } else {
            CliError::Io(format!("{what}: {e}"))
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

    /// Spec R13: every variant maps to its §3 class. The `match` has no
    /// wildcard, so a new variant fails to compile until it is placed here.
    #[test]
    fn every_variant_maps_to_its_class() {
        let s = || String::from("x");
        let all = [
            CliError::Codec(md_codec::Error::ChunkSetEmpty),
            CliError::Render(md_codec::RenderError::MalformedTree(s())),
            CliError::TemplateParse(s()),
            CliError::BadXpub { i: 0, why: s() },
            CliError::BadFingerprint { i: 0, why: s() },
            CliError::BadOrigin { i: 0, why: s() },
            CliError::Compile(s()),
            CliError::Mismatch(s()),
            CliError::Usage(s()),
            CliError::Input(s()),
            CliError::Io(s()),
            CliError::Refused(s()),
            CliError::Network(s()),
            CliError::Internal(s()),
            CliError::Compose(s()),
            CliError::Seat(s()),
            CliError::KeyReuse(s()),
            CliError::Unsupported(s()),
            CliError::Decompose(s()),
            CliError::VerifyAgainstUnreadable(s()),
        ];
        for e in &all {
            let want = match e {
                CliError::Mismatch(_) => 1,
                CliError::Codec(_)
                | CliError::Render(_)
                | CliError::TemplateParse(_)
                | CliError::BadXpub { .. }
                | CliError::BadFingerprint { .. }
                | CliError::BadOrigin { .. }
                | CliError::Compile(_)
                | CliError::Input(_)
                | CliError::Compose(_)
                | CliError::Seat(_)
                | CliError::KeyReuse(_)
                | CliError::Unsupported(_)
                | CliError::Decompose(_)
                | CliError::VerifyAgainstUnreadable(_) => 2,
                CliError::Refused(_) | CliError::Network(_) => 8,
                CliError::Io(_) => 32,
                CliError::Usage(_) => 64,
                CliError::Internal(_) => 101,
            };
            assert_eq!(e.exit_code(), want, "{e:?}");
        }
        assert_eq!(all.len(), 20, "a variant is missing from the table");
    }

    #[test]
    fn a_file_that_is_not_text_is_invalid_and_a_missing_one_is_io() {
        let bad = std::io::Error::from(std::io::ErrorKind::InvalidData);
        let gone = std::io::Error::from(std::io::ErrorKind::NotFound);
        assert_eq!(
            CliError::read_failure(format_args!("--in f"), &bad).exit_code(),
            2
        );
        assert_eq!(
            CliError::read_failure(format_args!("--in f"), &gone).exit_code(),
            32
        );
    }

    #[test]
    fn from_codec_wraps() {
        let codec_err = md_codec::Error::ChunkSetIdOutOfRange { id: 0xFFFFFF };
        let cli_err: CliError = codec_err.into();
        assert!(matches!(cli_err, CliError::Codec(_)));
    }
}
