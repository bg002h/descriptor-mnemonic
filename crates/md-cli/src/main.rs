#![allow(missing_docs)]

mod bip388;
mod cmd;
#[cfg(feature = "cli-compiler")]
mod compile;
mod decompose;
mod error;
mod format;
mod output_advisory;
mod parse;
mod process_hardening;
mod seat;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use error::CliError;

/// CLI-facing network selector. Maps to `bitcoin::Network`.
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum CliNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl From<CliNetwork> for bitcoin::Network {
    fn from(n: CliNetwork) -> Self {
        match n {
            CliNetwork::Mainnet => bitcoin::Network::Bitcoin,
            CliNetwork::Testnet => bitcoin::Network::Testnet,
            CliNetwork::Signet => bitcoin::Network::Signet,
            CliNetwork::Regtest => bitcoin::Network::Regtest,
        }
    }
}

impl CliNetwork {
    /// Stable kebab-cased name for JSON output. Matches the clap
    /// `value_enum` rendering, NOT `bitcoin::Network::Display` (which
    /// emits "bitcoin" for mainnet — confusing for JSON consumers).
    fn as_str(self) -> &'static str {
        match self {
            CliNetwork::Mainnet => "mainnet",
            CliNetwork::Testnet => "testnet",
            CliNetwork::Signet => "signet",
            CliNetwork::Regtest => "regtest",
        }
    }
}

/// Parse `--separator`: WHITESPACE ONLY — the keyword `space` or the literal
/// `" "`. Returns the separator char. SPEC §6c. Rejection is a clap parse error
/// (exit 2), before command dispatch.
///
/// `hyphen` and `comma` were accepted before P3 and are now refused. The
/// reason is cross-tool rather than local: `md`'s own decoder strips display
/// separators of every kind, so a hyphen-grouped card round-trips here — but
/// `mt`'s decoder strips WHITESPACE and nothing else, so the same habit applied
/// to an `mt1` string produces a card `mt` refuses, after the plates are cut.
/// A rule that is safe per-tool and unsafe across tools is exactly the one an
/// operator carries between tools, so the constellation narrows to the
/// intersection. The cost is two cosmetic options; the cost of getting it wrong
/// is a plate.
///
/// The refusal names BOTH remedies (§6h — remedy text must be executable):
/// `--separator space` for a grouped card, `--group-size 0` for an unbroken
/// one. Someone reaching for `hyphen` wanted one or the other.
fn parse_separator(s: &str) -> Result<char, String> {
    match s {
        "space" | " " => Ok(' '),
        "hyphen" | "-" | "comma" | "," => Err(format!(
            "separator {s:?} is no longer accepted: --separator is whitespace-only across the \
             constellation (SPEC §6c), because `mt` strips whitespace and nothing else on decode, \
             so a hyphen- or comma-grouped card is one `mt` refuses AFTER the plates are cut. \
             Use `--separator space` (or the literal \" \") for a grouped card, or \
             `--group-size 0` for an unbroken one."
        )),
        other => Err(format!(
            "invalid separator {other:?}; --separator is whitespace-only: expected `space` (or \
             the literal \" \"). Use `--group-size 0` for an unbroken card."
        )),
    }
}

#[derive(Debug, Parser)]
#[command(name = "md", version, about = "Mnemonic Descriptor (MD) — engravable BIP 388 wallet policy backups", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a wallet policy into MD backup string(s).
    #[command(
        after_long_help = "EXAMPLES:\n  $ md encode wpkh(@0/<0;1>/*) --group-size 0\n  md1yqpqqxqq8xtwhw4xwn4qh"
    )]
    Encode {
        /// BIP 388 template, e.g. `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))`.
        #[arg(conflicts_with = "in_file")]
        template: Option<String>,
        /// Read the BIP 388 template from FILE instead of argv (SPEC §6b).
        /// The file holds one template; surrounding whitespace is trimmed.
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        /// Write the md1 artifact to FILE instead of stdout, CREATED 0600
        /// (SPEC §6b, F-244) — which a shell redirect cannot do. OVERWRITES an
        /// existing file, and tightens its mode. The stderr engraving card,
        /// the chunk-set-id and the advisories are unaffected.
        #[arg(long = "out", value_name = "FILE")]
        out_file: Option<std::path::PathBuf>,
        /// Compile a sub-Miniscript-Policy expression into a template (cli-compiler).
        #[arg(
            long = "from-policy",
            value_name = "EXPR",
            conflicts_with = "template",
            // P3 §6b: --in supplies a TEMPLATE, so it is the same channel the
            // positional occupies and the same one --from-policy replaces.
            // Without this, `--from-policy X --in f` would silently ignore the
            // file -- a precedence rule nobody wrote down.
            conflicts_with = "in_file"
        )]
        from_policy: Option<String>,
        /// Script context for `--from-policy`.
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"])]
        context: Option<String>,
        /// Tap-context only: fallback unspendable internal key passed to
        /// miniscript's `compile_tr`. Defaults to BIP-341 NUMS H-point when
        /// omitted (auto-NUMS); supplying a value is rare and used to force
        /// a specific NUMS-equivalent key. Rejected when --context segwitv0.
        #[arg(long, value_name = "KEY")]
        unspendable_key: Option<String>,
        /// Override the inferred origin path with a single shared path
        /// (flattens Divergent mode to Shared). Accepts named (bip44|48|49|84|86),
        /// hex (0xNN), or literal (m/...) forms.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        /// Concrete xpub for placeholder `@i`. Repeatable.
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder `@i`. Repeatable.
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
        /// Network for xpub validation (and JSON output labeling).
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Force chunked encoding even for short policies.
        #[arg(long)]
        force_chunked: bool,
        /// Insert a separator every N characters in the ENGRAVING CARD on
        /// stderr (0 = unbroken card). SPEC §6c. stdout is always the
        /// unbroken md1 string; --json stays unbroken too.
        #[arg(long, default_value_t = 5)]
        group_size: u16,
        /// Separator for the engraving card: `space` (keyword) or the literal
        /// " ". Whitespace only — `hyphen` and `comma` are no longer accepted.
        /// SPEC §6c.
        #[arg(long, default_value = "space", value_parser = parse_separator)]
        separator: char,
        /// Removed in v0.12.0; now a hard error. md1 is regular-code-only
        /// (payloads over 400 bits are chunked).
        #[arg(long)]
        force_long_code: bool,
        /// Print the freshly-computed PolicyId fingerprint after the phrase.
        #[arg(long)]
        policy_id_fingerprint: bool,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
        /// Admit a spend path that requires NO signature (e.g. a hashlock +
        /// timelock recovery tier).
        ///
        /// rust-miniscript refuses these by default with "All spend paths must
        /// require a signature" — a safety policy, not a language rule; the
        /// script is well-formed and valid. This relaxes ONLY that rule:
        /// malleability, resource limits, repeated keys and timelock mixing are
        /// still enforced. Whoever learns the preimage of a keyless path can
        /// spend it alone, so if that preimage is engraved, the plate is bearer
        /// access. Prints a warning on every use.
        #[arg(long)]
        experimental: bool,
    },
    /// Decode one or more MD backup strings into a wallet policy template.
    #[command(
        after_long_help = "EXAMPLES:\n  $ md decode md1yqpqqxqq8xtwhw4xwn4qh\n  wpkh(@0/<0;1>/*)"
    )]
    Decode {
        #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
        strings: Vec<String>,
        /// Read md1 strings from FILE, one per line (SPEC §6b). Display
        /// separators are stripped per line, so a card copied off the
        /// engraving card re-ingests.
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        /// Emit a structured JSON object on stdout instead of the
        /// plain BIP-388 wallet-policy template string.
        #[arg(long)]
        json: bool,
    },
    /// Verify backup strings re-encode to a given template.
    Verify {
        #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
        strings: Vec<String>,
        /// Read md1 strings from FILE, one per line (SPEC §6b).
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        #[arg(long, required = true)]
        template: String,
        #[arg(long = "key", value_name = "@i=XPUB")]
        keys: Vec<String>,
        #[arg(long = "fingerprint", value_name = "@i=HEX")]
        fingerprints: Vec<String>,
        /// Override the inferred origin path with a single shared path
        /// (flattens Divergent mode to Shared). Accepts named (bip44|48|49|84|86),
        /// hex (0xNN), or literal (m/...) forms.
        ///
        /// Mirrors `md encode --path`. Without it the non-canonical wrappers this
        /// flag exists for are unreachable here: they refuse with "non-canonical
        /// wrapper requires explicit origin for @N", and there was no way to
        /// supply one.
        #[arg(long, value_name = "PATH")]
        path: Option<String>,
        /// Network for xpub validation.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Accept a template with a spend path that requires no signature,
        /// mirroring `md encode --experimental`.
        ///
        /// Without this, a card authored with `--experimental` cannot be
        /// verified at all — the operator would hold a plate and have no way to
        /// check it, which is worse than not authoring it.
        #[arg(long)]
        experimental: bool,
    },
    /// Decode + pretty-print everything the codec sees.
    Inspect {
        #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
        strings: Vec<String>,
        /// Read md1 strings from FILE, one per line (SPEC §6b).
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        /// Emit a structured JSON object on stdout instead of the
        /// pretty-printed multi-line text form.
        #[arg(long)]
        json: bool,
    },
    /// Dump the raw payload bits in an annotated layout.
    Bytecode {
        #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
        strings: Vec<String>,
        /// Read md1 strings from FILE, one per line (SPEC §6b).
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Regenerate the project's test-vector corpus (maintainer tool).
    Vectors {
        #[arg(long, value_name = "DIR")]
        out: Option<String>,
    },
    /// Compile a sub-Miniscript-Policy expression into a BIP 388 template.
    Compile {
        expr: String,
        #[arg(long, value_name = "CTX", value_parser = ["tap", "segwitv0"], required = true)]
        context: String,
        /// Tap-context only: fallback unspendable internal key passed to
        /// miniscript's `compile_tr`. Defaults to BIP-341 NUMS H-point when
        /// omitted (auto-NUMS); supplying a value is rare. Rejected when
        /// --context segwitv0.
        #[arg(long, value_name = "KEY")]
        unspendable_key: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Lower an ORDERED list of spend paths to a BIP-388 template by FIXED rules
    /// (SPEC_wallet_policy_composer.md §5). The opposite of `compile`: no search,
    /// no cost model, the same text from every implementation.
    Compose {
        /// tr | wsh | sh-wsh | sh
        #[arg(long, value_name = "WRAPPER", required = true)]
        wrapper: String,
        /// One spend path in listed order: `<k>of<n>[,older=N|older=Nu|after=H|after=Tt][,sha256=HEX][,unsorted]`
        /// or `keyless,sha256=HEX[,older=..|after=..]`. Repeatable.
        #[arg(long = "path", value_name = "PATH", required = true, action = clap::ArgAction::Append)]
        paths: Vec<String>,
        /// Admit key-less paths and unsorted-where-sorted-was-legal, with a warning.
        #[arg(long)]
        experimental: bool,
        /// Emit JSON: the origin-less template, the inline-origin template, the
        /// slot map, the taproot internal-key path and the EXPERIMENTAL marks.
        #[arg(long)]
        json: bool,
    },
    /// Emit the CONCRETE output descriptor -- real xpubs, key origins and the
    /// BIP-380 checksum -- for pasting into a coordinator.
    ///
    /// Everything else this CLI prints for a wallet policy is a TEMPLATE (@0,
    /// @1, ...) or an address. Neither is what "paste your descriptor" means.
    /// Multipath (<0;1>) by default, which is the form a coordinator wants;
    /// --chain collapses it.
    // R9 (`design/SPEC_mdcli_mini.md`): `from_mk1` joins this group, with
    // `.multiple(true)`, so `--from-mk1`'s mere presence satisfies
    // `required(true)` even when a swallowed positional leaves `phrases`
    // empty (see `cmd::build::check_from_mk1_arity`'s doc comment) --
    // WITHOUT making `phrases` and `from_mk1` mutually exclusive, since
    // `.multiple(true)` lifts the group's default at-most-one constraint and
    // the two are meant to be supplied TOGETHER on the S row. `template`
    // stays mutually exclusive with both via its own `conflicts_with`
    // attributes, unaffected by this group's `multiple` setting.
    #[command(after_long_help = "EXAMPLES:\n  $ md descriptor md1qq...\n  wsh(...)#checksum",
              group = clap::ArgGroup::new("descriptor_input").required(true).multiple(true).args(["phrases", "template", "from_mk1"]))]
    Descriptor {
        /// One or more md1 phrases. Mutually exclusive with --template.
        #[arg(num_args = 0..)]
        phrases: Vec<String>,
        /// BIP 388 template. Requires at least one --key. Mutually exclusive with phrases.
        #[arg(long, value_name = "TEMPLATE", conflicts_with = "phrases")]
        template: Option<String>,
        /// Concrete xpub for placeholder @i, or the origin-notated
        /// @i=[fingerprint/path]xpub form (BIP-380). Repeatable. Requires
        /// --template. An origin-notated key's path must AGREE with the
        /// slot's inline template origin when both are present (never an
        /// override); its fingerprint must AGREE with --fingerprint when
        /// both name the same slot.
        #[arg(
            long = "key",
            value_name = "@i=XPUB|@i=[fp/path]XPUB",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder @i. Repeatable. Requires
        /// --template. Must AGREE with an origin-notated --key's own
        /// fingerprint when both name the same slot -- never a silent
        /// override.
        #[arg(
            long = "fingerprint",
            value_name = "@i=HEX",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        fingerprints: Vec<String>,
        /// Shared origin path, applied PER SLOT to whichever @i the
        /// template gave no inline origin -- a slot's inline template
        /// origin always wins. Same VALUE grammar as `md encode --path`
        /// (named, hex or literal) but NOT the same rule: `md encode
        /// --path` replaces the declaration wholesale, this fills only the
        /// slots that declared nothing. A slot with neither an inline
        /// origin nor this flag hits today's non-canonical-wrapper
        /// refusal.
        #[arg(
            long,
            value_name = "PATH",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        path: Option<String>,
        /// mk1 key-card string. Repeatable, and a single occurrence also
        /// takes several values (`--from-mk1 mk1a mk1b mk1c`) so a natural
        /// paste of a scanned card set works without repeating the flag.
        /// Supplied TOGETHER WITH the KEYLESS md1 phrases of a policy card:
        /// the seating engine matches each card to the slot whose declared
        /// origin it satisfies, then composes the concrete descriptor.
        /// Mutually exclusive with --template.
        #[arg(
            long = "from-mk1",
            value_name = "STRING",
            num_args = 1..,
            conflicts_with = "template"
        )]
        from_mk1: Vec<String>,
        /// Read mk1 key-card strings from FILE, one per line. Blank lines
        /// and `#` comments are skipped; any other line is refused rather
        /// than ignored. Combines with --from-mk1.
        #[arg(
            long = "from-mk1-file",
            value_name = "FILE",
            conflicts_with = "template"
        )]
        from_mk1_file: Option<std::path::PathBuf>,
        // rustdoc reads `<chunk-set-id>` as an unclosed HTML tag and
        // `-D warnings` (CI's `doc` job) turns that into an error. The text is
        // a CLAP help string, not documentation prose — rewording it would
        // change `md descriptor --help`, the man page and the gui-schema — so
        // the lint is silenced here rather than the help text edited.
        #[allow(rustdoc::invalid_html_tags)]
        /// Assert the seating of one slot: --seat '@i=<chunk-set-id>',
        /// repeatable. The id is the FULL five-hex-digit label a seating
        /// refusal prints beside each card, never a prefix. If that id
        /// auto-partitioned into several collided cards, add '#<k>' (the
        /// 1-based ordinal from its own '<id>#<k>' label) to bind one
        /// specific carrier -- the bare id alone refuses as ambiguous
        /// among them. The named card must still satisfy the slot's
        /// declared origin, so --seat can only choose among seatings the
        /// engine already permits -- it never places a card the engine
        /// would not, never silences a stub warning, and never fills a
        /// missing-card gap.
        #[arg(
            long = "seat",
            value_name = "@i=CHUNK-SET-ID[#K]",
            conflicts_with = "template"
        )]
        seats: Vec<String>,
        /// Network for xpub validation.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Collapse the multipath group to ONE chain (0 = receive, 1 = change).
        /// Omit for the multipath <0;1> form.
        #[arg(long)]
        chain: Option<u32>,
        /// Sugar for --chain 1.
        #[arg(long, conflicts_with = "chain")]
        change: bool,
        /// Put the KEYED md1 card on stdout instead of the concrete
        /// descriptor: `--emit md1`, minted from the seating result. Needs
        /// --from-mk1/--from-mk1-file input -- a template's minting tool is
        /// `md encode`, and a card on the positional is that artifact
        /// already. Changes the OUTPUT FORM only; every seating rule is
        /// unaffected.
        //
        // N2 (`design/SPEC_mdcli_mini.md`). `conflicts_with = "json"` because
        // `--json`'s envelope carries a `descriptor` field this form does not
        // produce; the cycle ships no new JSON envelope (SPEC "Non-goals"),
        // and a silently discarded flag on this verb is precisely
        // REVIEW-converter-whole-diff-r1 I4's finding.
        #[arg(
            long = "emit",
            value_name = "FORM",
            value_enum,
            conflicts_with = "json"
        )]
        emit: Option<cmd::descriptor::Emit>,
        /// Write the KEYED md1 artifact to FILE instead of stdout, CREATED
        /// 0600 -- mirrors `md encode --out` (F-244) byte for byte, which a
        /// shell redirect cannot do. OVERWRITES an existing file, and
        /// tightens its mode. The stderr engraving card, the chunk-set-id
        /// and the advisories are unaffected. Only meaningful with `--emit
        /// md1` -- `requires = "emit"` refuses it structurally rather than
        /// silently discarding it on the plain descriptor row (the I4
        /// defect class this cycle keeps naming on this verb).
        #[arg(long = "out", value_name = "FILE", requires = "emit")]
        out_file: Option<std::path::PathBuf>,
        /// Insert a separator every N characters in the ENGRAVING CARD on
        /// stderr (0 = unbroken card) -- mirrors `md encode --group-size`
        /// exactly, same default. stdout is always the unbroken md1 string.
        /// Only meaningful with `--emit md1`; see `--out`'s note on
        /// `requires`.
        #[arg(long, default_value_t = 5, requires = "emit")]
        group_size: u16,
        /// Separator for the engraving card: `space` (keyword) or the
        /// literal " " -- mirrors `md encode --separator` exactly, same
        /// default. Whitespace only. Only meaningful with `--emit md1`; see
        /// `--out`'s note on `requires`.
        #[arg(
            long,
            default_value = "space",
            value_parser = parse_separator,
            requires = "emit"
        )]
        separator: char,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
        /// SPEND-EQUAL comparison target — an md1 string or a FILE holding
        /// one or more, one per line. Wires `seat::compose::spend_equal`:
        /// states SPEND-EQUAL or NOT, names the failing half (structure,
        /// values, use-sites), and states that origin metadata is excluded
        /// and why.
        ///
        /// Exit codes: 0 = spend-equal; 5 = NOT spend-equal (`md repair`'s
        /// reserved-5 precedent for a non-error, non-default answer); 1/2 =
        /// errors, unchanged — so a mistyped input can never read as "not
        /// equal", the false signal that invites re-cutting a good plate.
        ///
        /// Admissible on every input mode that composes a descriptor here
        /// (the positional card, --from-mk1/--from-mk1-file seating,
        /// --template) — deliberately NOT the T-row `requires = "template"`
        /// plus `conflicts_with_all` pattern, which would make it unusable
        /// on exactly the two modes the FOLLOWUP exists for.
        #[arg(long = "verify-against", value_name = "md1|FILE")]
        verify_against: Option<String>,
    },

    /// Derive bitcoin addresses from a wallet-policy-mode descriptor.
    // R9 — see the identical comment on `descriptor_input` above.
    #[command(after_long_help = "EXAMPLES:\n  $ md address md1qq...\n  bc1q...",
              group = clap::ArgGroup::new("address_input").required(true).multiple(true).args(["phrases", "template", "from_mk1"]))]
    Address {
        /// One or more md1 phrases. Mutually exclusive with --template.
        #[arg(num_args = 0..)]
        phrases: Vec<String>,
        /// BIP 388 template. Requires at least one --key. Mutually exclusive with phrases.
        #[arg(long, value_name = "TEMPLATE", conflicts_with = "phrases")]
        template: Option<String>,
        /// Concrete xpub for placeholder @i, or the origin-notated
        /// @i=[fingerprint/path]xpub form (BIP-380). Repeatable. Requires
        /// --template. An origin-notated key's path must AGREE with the
        /// slot's inline template origin when both are present (never an
        /// override); its fingerprint must AGREE with --fingerprint when
        /// both name the same slot.
        #[arg(
            long = "key",
            value_name = "@i=XPUB|@i=[fp/path]XPUB",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        keys: Vec<String>,
        /// Master-key fingerprint for placeholder @i. Repeatable. Requires
        /// --template. Must AGREE with an origin-notated --key's own
        /// fingerprint when both name the same slot -- never a silent
        /// override.
        #[arg(
            long = "fingerprint",
            value_name = "@i=HEX",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        fingerprints: Vec<String>,
        /// Shared origin path, applied PER SLOT to whichever @i the template
        /// gave no inline origin -- a slot's inline template origin always
        /// wins. Accepts named (bip44|48|49|84|86), hex (0xNN), or literal
        /// (m/...) forms.
        ///
        /// Same VALUE grammar as `md encode --path` but NOT the same rule:
        /// `md encode --path` replaces the declaration wholesale, this fills
        /// only the slots that declared nothing. A slot with neither an
        /// inline origin nor this flag still hits "non-canonical wrapper
        /// requires explicit origin for @N".
        #[arg(
            long,
            value_name = "PATH",
            requires = "template",
            // REVIEW-converter-whole-diff-r1 I4 — the T-row flags have no
            // meaning on the S row. `requires = "template"` alone did NOT
            // refuse them: it fires only when the whole
            // `<PHRASES|--template>` group is absent, so with a policy card
            // supplied it was inert, and `--key`/`--fingerprint`/`--path`
            // were accepted and silently discarded on a SUCCESSFUL
            // composition (measured 2026-08-30: same descriptor, same
            // checksum `#9uzthz8n`, exit 0). Declared here so the conflict is
            // structural on every entrance rather than a runtime check one
            // route could forget.
            //
            // `phrases` is in the list for the same reason and it is the
            // PRE-EXISTING half: `md descriptor <keyed md1 card> --key @0=X`
            // also composed at exit 0 with the key discarded (measured
            // 2026-08-30 on the v-d-rt keyed card: byte-identical output with
            // and without the flag). Nothing legitimate is lost -- these three
            // flags already `requires = "template"`, and `--template` already
            // `conflicts_with = "phrases"`, so no valid invocation pairs them
            // with phrases; the declaration only makes the existing rule
            // actually fire.
            conflicts_with_all = ["phrases", "from_mk1", "from_mk1_file", "seats"],
        )]
        path: Option<String>,
        /// mk1 key-card string. Repeatable, and a single occurrence also
        /// takes several values (`--from-mk1 mk1a mk1b mk1c`) so a natural
        /// paste of a scanned card set works without repeating the flag.
        /// Supplied TOGETHER WITH the KEYLESS md1 phrases of a policy card:
        /// the seating engine matches each card to the slot whose declared
        /// origin it satisfies, then composes the concrete descriptor.
        /// Mutually exclusive with --template.
        #[arg(
            long = "from-mk1",
            value_name = "STRING",
            num_args = 1..,
            conflicts_with = "template"
        )]
        from_mk1: Vec<String>,
        /// Read mk1 key-card strings from FILE, one per line. Blank lines
        /// and `#` comments are skipped; any other line is refused rather
        /// than ignored. Combines with --from-mk1.
        #[arg(
            long = "from-mk1-file",
            value_name = "FILE",
            conflicts_with = "template"
        )]
        from_mk1_file: Option<std::path::PathBuf>,
        // rustdoc reads `<chunk-set-id>` as an unclosed HTML tag and
        // `-D warnings` (CI's `doc` job) turns that into an error. The text is
        // a CLAP help string, not documentation prose — rewording it would
        // change `md descriptor --help`, the man page and the gui-schema — so
        // the lint is silenced here rather than the help text edited.
        #[allow(rustdoc::invalid_html_tags)]
        /// Assert the seating of one slot: --seat '@i=<chunk-set-id>',
        /// repeatable. The id is the FULL five-hex-digit label a seating
        /// refusal prints beside each card, never a prefix. If that id
        /// auto-partitioned into several collided cards, add '#<k>' (the
        /// 1-based ordinal from its own '<id>#<k>' label) to bind one
        /// specific carrier -- the bare id alone refuses as ambiguous
        /// among them. The named card must still satisfy the slot's
        /// declared origin, so --seat can only choose among seatings the
        /// engine already permits -- it never places a card the engine
        /// would not, never silences a stub warning, and never fills a
        /// missing-card gap.
        #[arg(
            long = "seat",
            value_name = "@i=CHUNK-SET-ID[#K]",
            conflicts_with = "template"
        )]
        seats: Vec<String>,
        /// Network for xpub validation and address rendering.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
        /// Multipath alternative selector (0 = receive, 1 = change for canonical <0;1>/*).
        #[arg(long, default_value_t = 0)]
        chain: u32,
        /// Sugar for --chain 1.
        #[arg(long, conflicts_with = "chain")]
        change: bool,
        /// Starting index along the wildcard.
        #[arg(long, default_value_t = 0)]
        index: u32,
        /// Number of consecutive addresses to derive starting at --index.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=1000))]
        count: u32,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
    },
    /// Turn a CONCRETE output descriptor back into the pieces md engraves:
    /// the keyless BIP-388 template, one origin-notated key line per slot, and
    /// the per-slot --fingerprint flags.
    ///
    /// The inverse of `md descriptor`. Takes ONE descriptor — real xpubs, with
    /// or without a `#checksum`, multipath (`<0;1>`) or fixed-path (BIP-389's
    /// `/**` shorthand for `/<0;1>/*` is also accepted, on either spelling).
    /// Bitcoin Core's `listdescriptors` JSON and separate receive/change
    /// descriptor PAIRS are refused with guidance rather than parsed.
    #[command(
        after_long_help = "EXAMPLES:\n  $ md decompose wpkh([73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf/<0;1>/*) --emit template\n  wpkh(@0/48'/0'/0'/2'/<0;1>/*)"
    )]
    Decompose {
        /// The concrete output descriptor. Exactly one; two are refused with
        /// the receive/change-pair guidance. Use `-` to read it from stdin.
        #[arg(required_unless_present = "in_file", num_args = 1.., conflicts_with = "in_file")]
        descriptors: Vec<String>,
        /// Read the descriptor from FILE instead of argv — decompose's own
        /// input material (SPEC §6b). Blank lines and `#` comments are
        /// skipped; a file holding a receive/change PAIR draws the pair
        /// guidance, not a parse error.
        #[arg(long = "in", value_name = "FILE")]
        in_file: Option<std::path::PathBuf>,
        /// Which artifact to print. `all` (the default) prints template, key
        /// lines and fingerprint flags under `#` headers; the single-section
        /// forms redirect straight into the file the next command wants.
        /// `commands` prints runnable `md encode` / `mk encode` lines and
        /// REFUSES when any key states no origin (an mk1 card binds key to
        /// origin by design).
        #[arg(long, value_enum, default_value_t = cmd::decompose::Emit::All)]
        emit: cmd::decompose::Emit,
        /// Network the descriptor's extended keys must belong to.
        #[arg(long, value_enum, default_value_t = CliNetwork::Mainnet)]
        network: CliNetwork,
    },
    /// Emit a machine-readable JSON description of this CLI's flag surface
    /// (SPEC §7 of the mnemonic-gui v0.2 plan). Consumed by the mnemonic-gui
    /// overlay to bootstrap and drift-check per-subcommand widget schemas.
    #[cfg(feature = "json")]
    #[command(name = "gui-schema")]
    GuiSchema,
    /// BCH error-correction for md1 strings. Wraps `md_codec::decode_with_correction`
    /// and renders a per-chunk repair report.
    ///
    /// Exit codes (D26 cross-CLI parity with `ms repair` / `mk repair` /
    /// `mnemonic repair`):
    ///   0 — every input was already valid (no corrections applied)
    ///   5 — at least one chunk had corrections applied (REPAIR_APPLIED)
    ///   2 — atomic-fail per plan §1 D28: ANY chunk failing BCH capacity
    ///       fails the whole call; the failing chunk's index is named in
    ///       the stderr message and NO partial corrected output is emitted
    ///       on stdout.
    #[command(
        after_long_help = "ATOMIC SEMANTICS (multi-chunk):\n  When more than one md1 chunk is supplied, the call is atomic per plan\n  §1 D28: if ANY chunk fails BCH error-correction capacity (> 4 errors),\n  the WHOLE call exits 2 with the failing chunk index named on stderr.\n  NO partial corrected chunks are emitted on stdout.\n\nINPUT FORMAT:\n  Accepts BOTH chunked-form md1 strings (those bearing a chunk header, as\n  emitted by `md encode --force-chunked` or by automatic chunking when the\n  payload exceeds 320 bits) AND non-chunked single-string md1 (those\n  emitted by plain `md encode` for small payloads). Since md-codec v0.35.0,\n  single-string md1 are repaired directly — no need to fall back to\n  `md decode`.\n\nEXAMPLES:\n  $ md repair md1qq...\n  $ md repair md1qq... md1qq... md1qq...\n  $ md repair --json md1qq..."
    )]
    Repair(cmd::repair::RepairArgs),
    /// Generate roff man pages for the whole `md` CLI tree into a directory.
    ///
    /// Writes one `<name>.1` page per (sub)command via `clap_mangen`, rendered
    /// directly from the compiled clap `Command` tree (binary-faithful by
    /// construction). `scripts/install.sh` invokes this post-install to drop
    /// pages into the user manpath (no sudo, no system files).
    #[command(name = "gen-man")]
    GenMan {
        /// Directory to write `*.1` man pages into (created if absent).
        #[arg(long, value_name = "DIR")]
        out: std::path::PathBuf,
    },
}

/// Merge `--from-mk1` values with `--from-mk1-file`'s lines. Both channels
/// feed ONE list; the input pipeline dedupes it, so overlapping channels are
/// harmless by construction rather than by a rule here (SPEC A3(a) step 1).
fn collect_mk1(inline: &[String], file: Option<&std::path::Path>) -> Result<Vec<String>, CliError> {
    let mut out = inline.to_vec();
    if let Some(p) = file {
        out.extend(seat::read_mk1_file(p)?);
    }
    Ok(out)
}

fn main() -> ExitCode {
    // v0.6.1: deny other-UID /proc/$PID/cmdline reads + core dumps.
    process_hardening::set_non_dumpable();
    let cli = Cli::parse();
    match dispatch(cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(CliError::BadArg(m)) => {
            eprintln!("md: {m}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("md: {e}");
            ExitCode::from(1)
        }
    }
}

/// v0.18 Item G — reject `--unspendable-key` values that aren't the BIP-341
/// NUMS H-point literal hex. Empty-string and segwitv0-incompat checks fire
/// upstream of this guard; what reaches here is `Some(<non-empty-tap-value>)`.
#[cfg(feature = "cli-compiler")]
fn validate_unspendable_key_nums_only(uk: Option<&str>) -> Result<(), CliError> {
    if let Some(v) = uk {
        if v != parse::template::NUMS_H_POINT_X_ONLY_HEX {
            return Err(CliError::BadArg(
                "--unspendable-key currently only accepts the BIP-341 NUMS H-point literal hex \
                 (50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0) or omitted \
                 (auto-NUMS default). Other forms (xpub-style descriptor keys, arbitrary x-only \
                 hex) are deferred to a future version."
                    .into(),
            ));
        }
    }
    Ok(())
}

fn dispatch(c: Command) -> Result<u8, CliError> {
    match c {
        Command::Encode {
            template,
            in_file,
            out_file,
            from_policy,
            context,
            unspendable_key,
            path,
            keys,
            fingerprints,
            network,
            force_chunked,
            group_size,
            separator,
            force_long_code,
            policy_id_fingerprint,
            json,
            experimental,
        } => {
            let template_str: String = if let Some(expr) = from_policy {
                #[cfg(feature = "cli-compiler")]
                {
                    if unspendable_key.as_deref() == Some("") {
                        return Err(CliError::BadArg(
                            "--unspendable-key must not be empty (omit the flag for auto-NUMS default)".into()));
                    }
                    let ctx: compile::ScriptContext = context
                        .ok_or_else(|| {
                            CliError::BadArg("--from-policy requires --context tap|segwitv0".into())
                        })?
                        .parse()
                        .map_err(|e: compile::CompileError| CliError::Compile(e.to_string()))?;
                    if matches!(ctx, compile::ScriptContext::SegwitV0) && unspendable_key.is_some()
                    {
                        return Err(CliError::BadArg(
                            "--unspendable-key is only valid for --context tap (segwitv0 has no internal key)".into()));
                    }
                    validate_unspendable_key_nums_only(unspendable_key.as_deref())?;
                    compile::compile_policy_to_template(&expr, ctx, unspendable_key.as_deref())
                        .map_err(CliError::from)?
                }
                #[cfg(not(feature = "cli-compiler"))]
                {
                    let _ = (expr, context, unspendable_key);
                    return Err(CliError::BadArg(
                        "--from-policy requires the cli-compiler feature".into(),
                    ));
                }
            } else {
                if unspendable_key.is_some() {
                    return Err(CliError::BadArg(
                        "--unspendable-key is only meaningful with --from-policy".into(),
                    ));
                }
                // P3 §6b: the template arrives on argv or from `--in FILE`.
                // clap declares the two mutually exclusive, so there is no
                // precedence rule to invent here.
                match (template, in_file.as_deref()) {
                    (Some(t), _) => t,
                    (None, Some(p)) => cmd::read_template_file(p)?,
                    (None, None) => {
                        return Err(CliError::BadArg(
                            "encode: TEMPLATE required (on argv, via --in FILE, or use \
                             --from-policy with cli-compiler)"
                                .into(),
                        ));
                    }
                }
            };
            cmd::encode::run(cmd::encode::EncodeArgs {
                template: &template_str,
                out_file: out_file.as_deref(),
                keys: &keys,
                fingerprints: &fingerprints,
                path: path.as_deref(),
                network: network.into(),
                network_str: network.as_str(),
                force_chunked,
                group_size: group_size as usize,
                separator,
                force_long_code,
                policy_id_fingerprint,
                json,
                experimental,
            })
        }
        Command::Decode {
            strings,
            in_file,
            json,
        } => cmd::decode::run(&strings, in_file.as_deref(), json),
        Command::Verify {
            strings,
            in_file,
            template,
            keys,
            fingerprints,
            path,
            network,
            experimental,
        } => cmd::verify::run(cmd::verify::VerifyArgs {
            strings: &strings,
            in_file: in_file.as_deref(),
            template: &template,
            keys: &keys,
            fingerprints: &fingerprints,
            path: path.as_deref(),
            network: network.into(),
            experimental,
        }),
        Command::Inspect {
            strings,
            in_file,
            json,
        } => cmd::inspect::run(&strings, in_file.as_deref(), json),
        Command::Bytecode {
            strings,
            in_file,
            json,
        } => cmd::bytecode::run(&strings, in_file.as_deref(), json),
        Command::Vectors { out } => cmd::vectors::run(out),
        Command::Compile {
            expr,
            context,
            unspendable_key,
            json,
        } => {
            #[cfg(feature = "cli-compiler")]
            {
                if unspendable_key.as_deref() == Some("") {
                    return Err(CliError::BadArg(
                        "--unspendable-key must not be empty (omit the flag for auto-NUMS default)"
                            .into(),
                    ));
                }
                if context == "segwitv0" && unspendable_key.is_some() {
                    return Err(CliError::BadArg(
                        "--unspendable-key is only valid for --context tap (segwitv0 has no internal key)".into()));
                }
                validate_unspendable_key_nums_only(unspendable_key.as_deref())?;
                cmd::compile::run(&expr, &context, unspendable_key.as_deref(), json)
            }
            #[cfg(not(feature = "cli-compiler"))]
            {
                let _ = (expr, context, unspendable_key, json);
                Err(CliError::BadArg(
                "compile requires the cli-compiler feature; rebuild with --features cli-compiler".into()))
            }
        }
        Command::Compose {
            wrapper,
            paths,
            experimental,
            json,
        } => cmd::compose::run(&wrapper, &paths, experimental, json),
        Command::Descriptor {
            phrases,
            template,
            keys,
            fingerprints,
            path,
            from_mk1,
            from_mk1_file,
            seats,
            network,
            chain,
            change,
            emit,
            out_file,
            group_size,
            separator,
            json,
            verify_against,
        } => {
            let chain = if change { Some(1) } else { chain };
            let from_mk1 = collect_mk1(&from_mk1, from_mk1_file.as_deref())?;
            cmd::descriptor::run(cmd::descriptor::DescriptorArgs {
                phrases: &phrases,
                template: template.as_deref(),
                keys: &keys,
                fingerprints: &fingerprints,
                path: path.as_deref(),
                from_mk1: &from_mk1,
                seats: &seats,
                network: network.into(),
                network_str: network.as_str(),
                chain,
                emit,
                out_file: out_file.as_deref(),
                group_size: group_size as usize,
                separator,
                json,
                verify_against: verify_against.as_deref(),
            })
        }
        Command::Address {
            phrases,
            template,
            keys,
            fingerprints,
            path,
            from_mk1,
            from_mk1_file,
            seats,
            network,
            chain,
            change,
            index,
            count,
            json,
        } => {
            let chain = if change { 1 } else { chain };
            let from_mk1 = collect_mk1(&from_mk1, from_mk1_file.as_deref())?;
            cmd::address::run(cmd::address::AddressArgs {
                phrases: &phrases,
                template: template.as_deref(),
                keys: &keys,
                path: path.as_deref(),
                fingerprints: &fingerprints,
                from_mk1: &from_mk1,
                seats: &seats,
                network: network.into(),
                network_str: network.as_str(),
                chain,
                index,
                count,
                json,
            })
        }
        Command::Decompose {
            descriptors,
            in_file,
            emit,
            network,
        } => cmd::decompose::run(cmd::decompose::DecomposeArgs {
            descriptors: &descriptors,
            in_file: in_file.as_deref(),
            emit,
            network: network.into(),
        }),
        #[cfg(feature = "json")]
        Command::GuiSchema => cmd::gui_schema::run(),
        Command::Repair(a) => cmd::repair::run(a),
        Command::GenMan { out } => cmd::gen_man::run(out),
    }
}
