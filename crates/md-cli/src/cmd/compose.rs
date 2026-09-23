//! `md compose` -- the FIXED lowering surface (SPEC_wallet_policy_composer.md
//! §10 item 1). The opposite contract to `md compile`: no search, no cost
//! model, the same answer from every implementation, forever.
//!
//! Not to be confused with `crate::seat::compose`, which SEATS keys into an
//! existing keyless card; this module builds the card's policy from a path list.

use crate::error::CliError;
use md_codec::compose::{
    Experimental, HashKind, HashLock, KeySet, Lock, PathList, SpendPath, Wrapper, compose, presets,
    template_with_origins,
};
use md_codec::render::descriptor_to_template;

pub fn parse_wrapper(s: &str) -> Result<Wrapper, CliError> {
    match s {
        "tr" => Ok(Wrapper::Tr),
        "wsh" => Ok(Wrapper::Wsh),
        "sh-wsh" => Ok(Wrapper::ShWsh),
        "sh" => Ok(Wrapper::Sh),
        other => Err(CliError::Compose(format!(
            "--wrapper {other}: expected tr, wsh, sh-wsh or sh"
        ))),
    }
}

fn parse_u32(s: &str, what: &str) -> Result<u32, CliError> {
    s.parse::<u32>()
        .map_err(|_| CliError::Compose(format!("{what}: `{s}` is not a number in 0..=4294967295")))
}

fn parse_u16(s: &str, what: &str) -> Result<u16, CliError> {
    s.parse::<u16>()
        .map_err(|_| CliError::Compose(format!("{what}: `{s}` is not a number in 0..=65535")))
}

/// One `--path` value: `<k>of<n>[,opt]*` or `keyless[,opt]*`.
pub fn parse_path(s: &str) -> Result<SpendPath, CliError> {
    let mut parts = s.split(',');
    let head = parts.next().unwrap_or("");
    let keys = if head == "keyless" {
        None
    } else {
        let (k, n) = head.split_once("of").ok_or_else(|| {
            CliError::Compose(format!("path `{s}`: expected <k>of<n> or keyless"))
        })?;
        let k = k
            .parse::<u8>()
            .map_err(|_| CliError::Compose(format!("path `{s}`: k `{k}` is not a small number")))?;
        let n = n
            .parse::<u8>()
            .map_err(|_| CliError::Compose(format!("path `{s}`: n `{n}` is not a small number")))?;
        Some(KeySet { k, n, sorted: true })
    };
    let mut path = SpendPath {
        keys,
        hash: None,
        lock: None,
    };
    for opt in parts {
        if opt == "unsorted" {
            match path.keys.as_mut() {
                Some(ks) => ks.sorted = false,
                None => {
                    return Err(CliError::Compose(format!(
                        "path `{s}`: `unsorted` needs keys"
                    )));
                }
            }
            continue;
        }
        let (name, value) = opt.split_once('=').ok_or_else(|| {
            CliError::Compose(format!("path `{s}`: option `{opt}` needs a value"))
        })?;
        match name {
            "older" if path.lock.is_none() => {
                path.lock = Some(if let Some(units) = value.strip_suffix('u') {
                    Lock::OlderUnits(parse_u16(units, "older units")?)
                } else {
                    // A number above 65535 is refused by the codec with the
                    // §4c wording; parse as u32 so the message names the band.
                    let v = parse_u32(value, "older blocks")?;
                    match u16::try_from(v) {
                        Ok(b) => Lock::OlderBlocks(b),
                        Err(_) => {
                            return Err(CliError::Compose(format!(
                                "path `{s}`: older in blocks needs 1..=65535, got {v}"
                            )));
                        }
                    }
                });
            }
            "after" if path.lock.is_none() => {
                path.lock = Some(if let Some(t) = value.strip_suffix('t') {
                    Lock::AfterTime(parse_u32(t, "after time")?)
                } else {
                    let h = parse_u32(value, "after height")?;
                    if h >= md_codec::compose::LOCKTIME_THRESHOLD {
                        // The band refusal alone never names the remedy; the
                        // operator who typed a Unix time needs the suffix.
                        return Err(CliError::Compose(format!(
                            "path `{s}`: after={h} reads as a block height and is above the height band (1..=499999999); for a Unix time write after={h}t"
                        )));
                    }
                    Lock::AfterHeight(h)
                });
            }
            "older" | "after" => {
                return Err(CliError::Compose(format!(
                    "path `{s}`: at most one lock per path"
                )));
            }
            // The four hash fragments are SIBLING options, not a rename of
            // one (spec §9): `sha256=` is kind-specific, so `hash256=`,
            // `ripemd160=` and `hash160=` sit beside it. This is the operand
            // `ms hashlock --kind <k>` prints on its card.
            "sha256" | "hash256" | "ripemd160" | "hash160" => {
                if path.hash.is_some() {
                    return Err(CliError::Compose(format!(
                        "path `{s}`: at most one hash per path"
                    )));
                }
                let kind = kind_for_option(name).expect("the match arm lists exactly these four");
                path.hash = Some(parse_hash_hex(kind, value, &format!("path `{s}`"))?);
            }
            other => {
                // ENUMERATE THE HASH KINDS WHEN THE TYPO LOOKS LIKE ONE (F-551).
                //
                // `ms hashlock`'s card tells the operator: "If it answers
                // `unknown option ripemd160`, that support has not shipped in
                // your md yet." So a CURRENT md printing that bare string for a
                // CASE error sent them hunting for a newer release of a tool
                // that was already correct.
                let looks_like_a_kind = matches!(
                    other.to_ascii_lowercase().as_str(),
                    "sha256" | "hash256" | "ripemd160" | "hash160"
                );
                let hint = if looks_like_a_kind {
                    " -- the four hash kinds are `sha256`, `hash256`, `ripemd160` and \
                     `hash160`, LOWERCASE; case is rejected, never folded. This md \
                     supports all four"
                } else {
                    ""
                };
                return Err(CliError::Compose(format!(
                    "path `{s}`: unknown option `{other}`{hint}"
                )));
            }
        }
    }
    Ok(path)
}

/// The option name -> kind map, and the ONLY place that mapping is written.
/// Case is rejected, never folded, so an uppercase spelling falls through to
/// the caller's `unknown option` refusal rather than being quietly accepted.
fn kind_for_option(name: &str) -> Option<HashKind> {
    match name {
        "sha256" => Some(HashKind::Sha256),
        "hash256" => Some(HashKind::Hash256),
        "ripemd160" => Some(HashKind::Ripemd160),
        "hash160" => Some(HashKind::Hash160),
        _ => None,
    }
}

/// `value` as `kind.digest_len()` lowercase-hex bytes, or a
/// `{ctx}: <kind> needs N hex characters` refusal.
/// Shared by `--path ...,<kind>=HEX` and `--preset hashlock-gated,<kind>=HEX`.
///
/// **THE WIDTH COMES FROM `digest_len()`, never from a literal** (spec §5:
/// "the only place a length is written"). A 40-hex `sha256` and a 64-hex
/// `ripemd160` are both refusals here, and the message names the kind's OWN
/// width -- an operator who pasted the wrong digest needs to be told which
/// one they should have.
fn parse_hash_hex(kind: HashKind, value: &str, ctx: &str) -> Result<HashLock, CliError> {
    let want = kind.digest_len() * 2;
    // NAME THE CLAUSE THAT WAS VIOLATED, not the whole rule (F-551). This said
    // "needs 40 hex characters, lowercase" to an operator whose input WAS 40
    // characters and differed only in case -- a message naming a rule the input
    // already satisfies, which reads as the tool being wrong about the length.
    // `me sysw pack` and `ms --kind` both get this right; md was the outlier.
    if value.len() != want {
        return Err(CliError::Compose(format!(
            "{ctx}: {} needs exactly {want} hex characters, got {}",
            kind.token(),
            value.len()
        )));
    }
    if let Some(bad) = value.bytes().find(|b| !b.is_ascii_hexdigit()) {
        return Err(CliError::Compose(format!(
            "{ctx}: {} takes hex only; `{}` is not a hex digit",
            kind.token(),
            bad as char
        )));
    }
    if value.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(CliError::Compose(format!(
            "{ctx}: {} is {want} LOWERCASE hex; case is rejected, never folded \
             (SPEC_hashlock_kinds §6 -- the wire spelling is what gets hashed)",
            kind.token()
        )));
    }
    // The loop fills exactly `digest_len()` bytes; the tail of the array stays
    // zero and IS the alloc-gate padding (spec §5), which `HashLock::digest`
    // never hands back.
    let mut h = [0u8; 32];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).expect("checked") as u8;
        let lo = (chunk[1] as char).to_digit(16).expect("checked") as u8;
        h[i] = (hi << 4) | lo;
    }
    Ok(HashLock::new(kind, h))
}

/// The four kinds, in the order the CLI and the JSON present them.
const KINDS: [HashKind; 4] = [
    HashKind::Sha256,
    HashKind::Hash256,
    HashKind::Ripemd160,
    HashKind::Hash160,
];

/// A hashlock's digest as hex AT ITS KIND'S WIDTH -- 40 characters or 64,
/// never the alloc-gate padding.
fn hex_digest(h: &HashLock) -> String {
    h.digest().iter().fold(
        String::with_capacity(h.kind().digest_len() * 2),
        |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        },
    )
}

/// The resolved parameters of one `--preset` invocation, named for `--json`'s
/// `preset` field (SPEC §4d, C2). One variant per `md_codec::compose::presets`
/// constructor, same field names as its arguments.
#[derive(Debug, Clone, Copy)]
pub enum PresetParams {
    PlainMultisig {
        k: u8,
        n: u8,
    },
    SimpleTimelockedInheritance {
        older_blocks: u32,
    },
    KofnRecovery {
        k: u8,
        n: u8,
        older_blocks: u32,
    },
    TieredRecovery {
        k1: u8,
        n1: u8,
        k2: u8,
        n2: u8,
        older_blocks: u32,
    },
    HashlockGated {
        /// Which hash, and the digest at that kind's width.
        hash: HashLock,
        older_blocks: u32,
    },
    DecayingMultisig {
        k1: u8,
        n1: u8,
        k2: u8,
        n2: u8,
        older1: u32,
        older2: u32,
        after_height: u32,
    },
}

/// The six archetype names, kebab-case, in the order `--preset --help` and
/// every "expected one of" refusal lists them.
pub const PRESET_NAMES: [&str; 6] = [
    "plain-multisig",
    "simple-timelocked-inheritance",
    "kofn-recovery",
    "tiered-recovery",
    "hashlock-gated",
    "decaying-multisig",
];

fn parse_kofn(tok: &str, ctx: &str) -> Result<(u8, u8), CliError> {
    let (k, n) = tok
        .split_once("of")
        .ok_or_else(|| CliError::Compose(format!("{ctx}: `{tok}` is not <k>of<n>")))?;
    let k = k
        .parse::<u8>()
        .map_err(|_| CliError::Compose(format!("{ctx}: k `{k}` is not a small number")))?;
    let n = n
        .parse::<u8>()
        .map_err(|_| CliError::Compose(format!("{ctx}: n `{n}` is not a small number")))?;
    Ok((k, n))
}

/// `--preset <name>[,<k>of<n>]*[,<param>=<value>]*` (SPEC §4d, C2; the CLI
/// grammar this task defines). The `<k>of<n>` tokens are consumed IN LISTED
/// ORDER to fill the archetype's key-set parameters (tier 1 before tier 2,
/// where an archetype has two); `<param>=<value>` tokens are matched BY NAME,
/// in any order, against exactly the constructor's remaining arguments.
/// `unsorted` is never a preset parameter: every `presets::*` key set is
/// `sorted: true` by construction (`presets::ks`), so there is nothing for it
/// to toggle. Every constructor call runs through `checked` (`validate`), so
/// a legacy-wrapper shape or an out-of-band lock surfaces as the SAME
/// `ComposeError` a hand-built `--path` list with the same shape would give.
pub fn parse_preset(wrapper: Wrapper, s: &str) -> Result<(PresetParams, PathList), CliError> {
    let mut parts = s.split(',');
    let name = parts.next().unwrap_or("");
    if !PRESET_NAMES.contains(&name) {
        return Err(CliError::Compose(format!(
            "--preset {name}: expected one of {}",
            PRESET_NAMES.join(", ")
        )));
    }
    let ctx = format!("preset {name}");
    let mut ofs: Vec<(u8, u8)> = Vec::new();
    let mut named: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for tok in parts {
        match tok.split_once('=') {
            Some((k, v)) => {
                if named.insert(k, v).is_some() {
                    return Err(CliError::Compose(format!("{ctx}: `{k}=` given twice")));
                }
            }
            None => ofs.push(parse_kofn(tok, &ctx)?),
        }
    }
    let need_ofs = |want: usize| -> Result<(), CliError> {
        if ofs.len() != want {
            return Err(CliError::Compose(format!(
                "{ctx} needs exactly {want} <k>of<n> parameter{}, got {}",
                if want == 1 { "" } else { "s" },
                ofs.len()
            )));
        }
        Ok(())
    };
    let named_only = |allowed: &[&str]| -> Result<(), CliError> {
        for k in named.keys() {
            if !allowed.contains(k) {
                return Err(CliError::Compose(format!("{ctx} admits no {k}= parameter")));
            }
        }
        Ok(())
    };
    let need_u32 = |k: &str| -> Result<u32, CliError> {
        let v: &str = named
            .get(k)
            .map(|v| &**v)
            .ok_or_else(|| CliError::Compose(format!("{ctx} needs {k}=<n>")))?;
        // `--path` spells a 512-second-unit relative lock `older=<n>u` and a
        // Unix-time absolute lock `after=<t>t`; the preset grammar has neither
        // kind, so an operator carrying either suffix over from `--path` is
        // told what it means and where it works, in the shape
        // `need_after_height` uses for the band, rather than a bare "is not a
        // number" (S0b whole-diff review M-3).
        let suffixed = if k.starts_with("older") {
            v.strip_suffix('u')
                .map(|rest| (rest, "`u` (older in 512-second units)"))
        } else if k == "after" {
            v.strip_suffix('t')
                .map(|rest| (rest, "`t` (after as a Unix time)"))
        } else {
            None
        };
        if let Some((rest, meaning)) = suffixed {
            if rest.parse::<u32>().is_ok() {
                return Err(CliError::Compose(format!(
                    "{ctx} {k}: `{v}` is --path's {meaning} spelling, which presets cannot express -- use --path with `{k}={v}` instead"
                )));
            }
        }
        parse_u32(v, &format!("{ctx} {k}"))
    };
    // `presets::decaying_multisig`'s `after_height` argument always builds
    // `Lock::AfterHeight` (never `AfterTime`), and the preset grammar has no
    // `t`-suffix to ask for a time lock at all -- unlike `--path`'s
    // `after=<H>|after=<T>t`. A value at or above the Unix-time band therefore
    // cannot be satisfied by retyping it; the only remedy is `--path`, which
    // this names, mirroring `--path`'s own "reads as a block height" wording
    // (`parse_path`'s `after` arm, above) rather than propagating the bare
    // `ComposeError::LockOutOfRange` text with no remedy.
    let need_after_height = |k: &str| -> Result<u32, CliError> {
        let v = need_u32(k)?;
        if v >= md_codec::compose::LOCKTIME_THRESHOLD {
            return Err(CliError::Compose(format!(
                "{ctx}: {k}={v} reads as a block height and is above the height band (1..=499999999); presets cannot express a Unix time -- use --path with `after={v}t` instead"
            )));
        }
        Ok(v)
    };
    let map_ce = |e: md_codec::compose::ComposeError| CliError::Compose(e.to_string());
    match name {
        "plain-multisig" => {
            named_only(&[])?;
            need_ofs(1)?;
            let (k, n) = ofs[0];
            let list = presets::plain_multisig(wrapper, k, n).map_err(map_ce)?;
            Ok((PresetParams::PlainMultisig { k, n }, list))
        }
        "simple-timelocked-inheritance" => {
            named_only(&["older"])?;
            need_ofs(0)?;
            let older_blocks = need_u32("older")?;
            let list =
                presets::simple_timelocked_inheritance(wrapper, older_blocks).map_err(map_ce)?;
            Ok((
                PresetParams::SimpleTimelockedInheritance { older_blocks },
                list,
            ))
        }
        "kofn-recovery" => {
            named_only(&["older"])?;
            need_ofs(1)?;
            let (k, n) = ofs[0];
            let older_blocks = need_u32("older")?;
            let list = presets::kofn_recovery(wrapper, k, n, older_blocks).map_err(map_ce)?;
            Ok((PresetParams::KofnRecovery { k, n, older_blocks }, list))
        }
        "tiered-recovery" => {
            named_only(&["older"])?;
            need_ofs(2)?;
            let (k1, n1) = ofs[0];
            let (k2, n2) = ofs[1];
            let older_blocks = need_u32("older")?;
            let list =
                presets::tiered_recovery(wrapper, k1, n1, k2, n2, older_blocks).map_err(map_ce)?;
            Ok((
                PresetParams::TieredRecovery {
                    k1,
                    n1,
                    k2,
                    n2,
                    older_blocks,
                },
                list,
            ))
        }
        "hashlock-gated" => {
            named_only(&["sha256", "hash256", "ripemd160", "hash160", "older"])?;
            need_ofs(0)?;
            // EXACTLY ONE of the four, not "sha256 or else". Two would compose
            // a wallet whose hashlock branch is not the one the operator cut a
            // plate for, so it is a refusal rather than a precedence rule.
            let mut found = KINDS
                .iter()
                .filter_map(|k| named.get(k.token()).map(|v| (*k, v)));
            let (kind, hex) = found.next().ok_or_else(|| {
                CliError::Compose(format!(
                    "{ctx} needs one of sha256=<64 hex>, hash256=<64 hex>, \
                     ripemd160=<40 hex> or hash160=<40 hex>"
                ))
            })?;
            if let Some((other, _)) = found.next() {
                return Err(CliError::Compose(format!(
                    "{ctx}: at most one hash per path, got {} and {}",
                    kind.token(),
                    other.token()
                )));
            }
            let hash = parse_hash_hex(kind, hex, &ctx)?;
            let older_blocks = need_u32("older")?;
            let list = presets::hashlock_gated(wrapper, hash, older_blocks).map_err(map_ce)?;
            Ok((PresetParams::HashlockGated { hash, older_blocks }, list))
        }
        "decaying-multisig" => {
            named_only(&["older1", "older2", "after"])?;
            need_ofs(2)?;
            let (k1, n1) = ofs[0];
            let (k2, n2) = ofs[1];
            let older1 = need_u32("older1")?;
            let older2 = need_u32("older2")?;
            let after_height = need_after_height("after")?;
            let list =
                presets::decaying_multisig(wrapper, k1, n1, k2, n2, older1, older2, after_height)
                    .map_err(map_ce)?;
            Ok((
                PresetParams::DecayingMultisig {
                    k1,
                    n1,
                    k2,
                    n2,
                    older1,
                    older2,
                    after_height,
                },
                list,
            ))
        }
        other => Err(CliError::Compose(format!(
            "preset {other}: internal error -- PRESET_NAMES advertises this name but no lowering rule exists for it (this is a bug in md, not a mistake in your command)"
        ))),
    }
}

#[cfg(feature = "json")]
fn preset_params_json(p: &PresetParams) -> serde_json::Value {
    let (name, params) = match *p {
        PresetParams::PlainMultisig { k, n } => {
            ("plain-multisig", serde_json::json!({ "k": k, "n": n }))
        }
        PresetParams::SimpleTimelockedInheritance { older_blocks } => (
            "simple-timelocked-inheritance",
            serde_json::json!({ "older_blocks": older_blocks }),
        ),
        PresetParams::KofnRecovery { k, n, older_blocks } => (
            "kofn-recovery",
            serde_json::json!({ "k": k, "n": n, "older_blocks": older_blocks }),
        ),
        PresetParams::TieredRecovery {
            k1,
            n1,
            k2,
            n2,
            older_blocks,
        } => (
            "tiered-recovery",
            serde_json::json!({ "k1": k1, "n1": n1, "k2": k2, "n2": n2, "older_blocks": older_blocks }),
        ),
        PresetParams::HashlockGated { hash, older_blocks } => (
            "hashlock-gated",
            // `kind` + `digest`, not `sha256`: the old key was wrong for three
            // of the four kinds, and a consumer reading it could not tell which
            // fragment the wallet commits to. This is a breaking change to a
            // machine-readable contract and is recorded as one.
            serde_json::json!({
                "kind": hash.kind().token(),
                "digest": hex_digest(&hash),
                "older_blocks": older_blocks,
            }),
        ),
        PresetParams::DecayingMultisig {
            k1,
            n1,
            k2,
            n2,
            older1,
            older2,
            after_height,
        } => (
            "decaying-multisig",
            serde_json::json!({ "k1": k1, "n1": n1, "k2": k2, "n2": n2, "older1": older1, "older2": older2, "after_height": after_height }),
        ),
    };
    serde_json::json!({ "name": name, "params": params })
}

/// F-603: the machine-readable twin of `describe`.
///
/// `describe` numbers paths from 1 because that is how a human reads "the
/// first path", and `--json`'s `experimental[]` carried those same sentences
/// while `slots[].path` next to them counted from 0. A consumer joining the two
/// got a FALSE statement with no parse error to warn it: the six-path wallet's
/// object said "path 4 has no key" while listing @6 and @7 as key slots on
/// path 4.
///
/// Additive rather than a type change: `experimental[]` keeps its exact prose,
/// so nothing that reads it breaks and the top-level `"schema"` string stays
/// honest at whatever value it currently is (docs/json-schema-v1.md: the
/// version bumps on BREAKING changes — this addition, on its own, was not
/// one; stage 1b task 6 later bumped it for an unrelated reason, SPEC §4a).
/// This is the field to join on.
fn experimental_json(e: &Experimental) -> serde_json::Value {
    let (kind, path) = match e {
        Experimental::KeylessPath(i) => ("keyless_path", *i),
        Experimental::UnsortedKeys(i) => ("unsorted_keys", *i),
    };
    serde_json::json!({ "kind": kind, "path": path })
}

fn describe(e: &Experimental) -> String {
    match e {
        Experimental::KeylessPath(i) => format!(
            "path {} has no key (bearer access to whoever holds the preimage)",
            i + 1
        ),
        Experimental::UnsortedKeys(i) => format!(
            "path {} uses unsorted keys where sorted was possible (key order is part of this wallet)",
            i + 1
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    wrapper: &str,
    paths: &[String],
    preset: Option<&str>,
    experimental: bool,
    json: bool,
) -> Result<u8, CliError> {
    let wrapper = parse_wrapper(wrapper)?;
    let (list, preset_params): (PathList, Option<PresetParams>) = match preset {
        Some(spec) => {
            let (params, list) = parse_preset(wrapper, spec)?;
            (list, Some(params))
        }
        None => {
            let paths: Vec<SpendPath> = paths
                .iter()
                .map(|p| parse_path(p))
                .collect::<Result<_, _>>()?;
            (PathList { wrapper, paths }, None)
        }
    };
    let composed = compose(&list).map_err(|e| CliError::Compose(e.to_string()))?;
    if !composed.experimental.is_empty() && !experimental {
        let mut msg = String::from("this policy needs --experimental:");
        for e in &composed.experimental {
            msg.push_str("\n  ");
            msg.push_str(&describe(e));
        }
        return Err(CliError::Compose(msg));
    }
    for e in &composed.experimental {
        eprintln!("warning: EXPERIMENTAL: {}", describe(e));
    }
    // EVERY PATH CARRIES A HASHLOCK, so the preimage is the ONLY way to spend
    // this wallet -- lose it and the coins are gone, with no keyed escape and no
    // maturation to wait out.
    //
    // THE ASYMMETRY IS THE POINT (phase 1 journey walk, F-A1). `md` already
    // warns on the `keyless` shape, where the hashlock is an extra way IN, and
    // said nothing about the shape where it is the only way in -- so the
    // direction that loses money was the unwarned one. `keyless` stays an
    // EXPERIMENTAL refusal; this is a warning, not a gate, because the shape is
    // legitimate (it is how a pure hashlock escrow is written) and because
    // sha256 wallets have composed this way since before the cycle.
    if !list.paths.is_empty() && list.paths.iter().all(|p| p.hash.is_some()) {
        let kinds: Vec<&str> = {
            let mut k: Vec<&str> = list
                .paths
                .iter()
                .filter_map(|p| p.hash.map(|h| h.kind().token()))
                .collect();
            k.sort_unstable();
            k.dedup();
            k
        };
        eprintln!(
            "warning: EVERY path of this wallet needs the hashlock preimage ({}): \
             there is no path that spends with keys alone, so losing the preimage \
             loses the coins -- no key can recover them and no timelock matures. \
             Add a keyed path if that is not what you meant.",
            kinds.join(", ")
        );
    }
    // `unsorted` where sorted was never available is dropped by the lowering
    // (spec §5a: the §8b confirm fires only where sorted was legal); say so
    // rather than accept a typed request silently. No preset ever sets
    // `sorted: false` (`presets::ks` always sorts), so this loop is inert for
    // every `--preset` list and unchanged from `--path`'s behaviour.
    for (i, p) in list.paths.iter().enumerate() {
        if matches!(p.keys, Some(KeySet { n, sorted: false, .. }) if n >= 2)
            && !composed
                .experimental
                .contains(&Experimental::UnsortedKeys(i))
        {
            eprintln!(
                "note: path {}: `unsorted` has no effect here; sorted keys are not available in this position, so it is multi either way",
                i + 1
            );
        }
    }
    let template = descriptor_to_template(&composed.descriptor).map_err(CliError::Render)?;
    let with_origins = template_with_origins(&composed).map_err(CliError::Render)?;

    // F-600: compose's contract is stated four lines from the end of this
    // function -- "The inline-origin form: what `md encode` reads back to the
    // same card". It was emitting, at EXIT 0, templates `md encode` refuses:
    //
    //   md compose --wrapper wsh --experimental \
    //     --path 2of3 --path keyless,sha256=<h> --path keyless,ripemd160=<h>
    //   -> exit 0, prints wsh(or_d(multi(2,…),or_i(sha256(…),ripemd160(…))))
    //   md encode --in that --experimental
    //   -> "Miniscript is malleable"
    //
    // Two key-less hash paths next to each other compile to an `or_i` of two
    // hash fragments, which is malleable -- and `--experimental` relaxes ONLY
    // the signature rule, so nothing downstream will ever take it. The operator
    // got a plausible template, a zero exit, and a wall at the next verb.
    //
    // So compose now reads back what it is about to emit, with the SAME parser
    // `md encode` uses. This is not a second implementation of the rules: it is
    // the same function, which is what keeps the two verbs from drifting.
    //
    // THAT SHAPE NO LONGER REACHES HERE. md-codec 0.45 states the rule instead
    // of discovering it: `compose::validate` refuses a path list with more than
    // one key-less path (`ComposeError::TooManyKeylessPaths`), because the
    // device's Go port has no miniscript library to read back with and was
    // cutting such a policy into steel (composer fable review r0, lens 1 C-1).
    // The read-back STAYS as defence in depth over every OTHER way `md encode`
    // could refuse compose's output -- resource limits, repeated keys, timelock
    // mixing -- but it no longer knows which rule was broken, so it names the
    // parser's reason and stops guessing. The old hint here guessed "give one
    // of them a key, a timelock, or fold them into one path", and a timelock is
    // exactly what does NOT work.
    if let Err(e) = crate::parse::template::parse_template_ext(
        &with_origins,
        &[],
        &[],
        experimental,
        crate::parse::reuse::Disposition::Refuse,
    ) {
        return Err(CliError::Compose(format!(
            "composed a template that `md encode` refuses, so nothing was emitted:\n  {e}\n\
             \n\
             This is a defect in the path list, not in the keys or the hashes: the reason \
             above is the rule that was broken. Change the path the reason names."
        )));
    }

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let slots: Vec<serde_json::Value> = composed
            .slots
            .iter()
            .map(|s| serde_json::json!({ "index": s.index, "path": s.path, "ordinal": s.ordinal }))
            .collect();
        let exp: Vec<String> = composed.experimental.iter().map(describe).collect();
        let exp_paths: Vec<serde_json::Value> = composed
            .experimental
            .iter()
            .map(experimental_json)
            .collect();
        let preset_json = preset_params.as_ref().map(preset_params_json);
        let v = serde_json::json!({
            "schema": SCHEMA,
            "template": template,
            "template_with_origins": with_origins,
            "wrapper": wrapper_name(wrapper),
            "slots": slots,
            "internal_key_path": composed.internal_key_path,
            "experimental": exp,
            "experimental_paths": exp_paths,
            "preset": preset_json,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::Template,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }
    let _ = json;

    // The inline-origin form: what `md encode` reads back to the same card.
    println!("{with_origins}");
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::Template,
        &mut std::io::stderr(),
    );
    Ok(0)
}

fn wrapper_name(w: Wrapper) -> &'static str {
    match w {
        Wrapper::Tr => "tr",
        Wrapper::Wsh => "wsh",
        Wrapper::ShWsh => "sh-wsh",
        Wrapper::Sh => "sh",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // R0 round-1 fold-verification (Important): the ORIGINAL version of this
    // test iterated a hand-typed `[(&str, &str); 6]` fixture and asserted its
    // `.len() == 6` -- a tautology that cannot fail under any edit to
    // `PRESET_NAMES` or the `match` in `parse_preset`. Confirmed live: adding
    // a 7th, unmatched name to `PRESET_NAMES` compiled, passed clippy, passed
    // all 31 CLI tests, and then PANICKED (`unreachable!()`, exit 101) on a
    // real `md compose --preset <name>,...` invocation. This version iterates
    // `PRESET_NAMES` ITSELF and calls `parse_preset` directly (only possible
    // from inside this crate -- `cli_compose_preset.rs` is a black-box
    // integration test with no access to either), so a name added to
    // `PRESET_NAMES` with no matching valid-parameter fixture or no matching
    // `match` arm fails HERE, not in production.
    #[test]
    fn every_preset_name_parses_with_some_valid_parameters() {
        fn valid_params(name: &str) -> &'static str {
            match name {
                "plain-multisig" => "2of3",
                "simple-timelocked-inheritance" => "older=26280",
                "kofn-recovery" => "2of3,older=26280",
                "tiered-recovery" => "2of2,1of2,older=26280",
                "hashlock-gated" => {
                    "sha256=a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8,older=26280"
                }
                "decaying-multisig" => "2of2,1of1,older1=13140,older2=26280,after=1000000",
                other => panic!(
                    "PRESET_NAMES gained `{other}` with no valid-parameter fixture in this test"
                ),
            }
        }
        for name in PRESET_NAMES {
            let spec = format!("{name},{}", valid_params(name));
            parse_preset(Wrapper::Wsh, &spec).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}
