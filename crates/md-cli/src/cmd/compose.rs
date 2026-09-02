//! `md compose` -- the FIXED lowering surface (SPEC_wallet_policy_composer.md
//! §10 item 1). The opposite contract to `md compile`: no search, no cost
//! model, the same answer from every implementation, forever.
//!
//! Not to be confused with `crate::seat::compose`, which SEATS keys into an
//! existing keyless card; this module builds the card's policy from a path list.

use crate::error::CliError;
use md_codec::compose::{
    Experimental, KeySet, Lock, PathList, SpendPath, Wrapper, compose, presets,
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
            "sha256" => {
                if path.hash.is_some() {
                    return Err(CliError::Compose(format!(
                        "path `{s}`: at most one hash per path"
                    )));
                }
                let h = parse_sha256_hex(value, &format!("path `{s}`"))?;
                path.hash = Some(h);
            }
            other => {
                return Err(CliError::Compose(format!(
                    "path `{s}`: unknown option `{other}`"
                )));
            }
        }
    }
    Ok(path)
}

/// `value` as 32 lowercase-hex bytes, or a `{ctx}: sha256 needs ...` refusal.
/// Shared by `--path ...,sha256=HEX` and `--preset hashlock-gated,sha256=HEX`.
fn parse_sha256_hex(value: &str, ctx: &str) -> Result<[u8; 32], CliError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(CliError::Compose(format!(
            "{ctx}: sha256 needs 64 hex characters, lowercase"
        )));
    }
    let mut h = [0u8; 32];
    for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).expect("checked") as u8;
        let lo = (chunk[1] as char).to_digit(16).expect("checked") as u8;
        h[i] = (hi << 4) | lo;
    }
    Ok(h)
}

fn hex32(h: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for b in h {
        let _ = write!(out, "{b:02x}");
    }
    out
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
        sha256: [u8; 32],
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
        let v = named
            .get(k)
            .ok_or_else(|| CliError::Compose(format!("{ctx} needs {k}=<n>")))?;
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
            need_ofs(1)?;
            named_only(&[])?;
            let (k, n) = ofs[0];
            let list = presets::plain_multisig(wrapper, k, n).map_err(map_ce)?;
            Ok((PresetParams::PlainMultisig { k, n }, list))
        }
        "simple-timelocked-inheritance" => {
            need_ofs(0)?;
            named_only(&["older"])?;
            let older_blocks = need_u32("older")?;
            let list =
                presets::simple_timelocked_inheritance(wrapper, older_blocks).map_err(map_ce)?;
            Ok((
                PresetParams::SimpleTimelockedInheritance { older_blocks },
                list,
            ))
        }
        "kofn-recovery" => {
            need_ofs(1)?;
            named_only(&["older"])?;
            let (k, n) = ofs[0];
            let older_blocks = need_u32("older")?;
            let list = presets::kofn_recovery(wrapper, k, n, older_blocks).map_err(map_ce)?;
            Ok((PresetParams::KofnRecovery { k, n, older_blocks }, list))
        }
        "tiered-recovery" => {
            need_ofs(2)?;
            named_only(&["older"])?;
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
            need_ofs(0)?;
            named_only(&["sha256", "older"])?;
            let hex = named
                .get("sha256")
                .ok_or_else(|| CliError::Compose(format!("{ctx} needs sha256=<64 hex>")))?;
            let sha256 = parse_sha256_hex(hex, &ctx)?;
            let older_blocks = need_u32("older")?;
            let list = presets::hashlock_gated(wrapper, sha256, older_blocks).map_err(map_ce)?;
            Ok((
                PresetParams::HashlockGated {
                    sha256,
                    older_blocks,
                },
                list,
            ))
        }
        "decaying-multisig" => {
            need_ofs(2)?;
            named_only(&["older1", "older2", "after"])?;
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
        PresetParams::HashlockGated {
            sha256,
            older_blocks,
        } => (
            "hashlock-gated",
            serde_json::json!({ "sha256": hex32(&sha256), "older_blocks": older_blocks }),
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

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let slots: Vec<serde_json::Value> = composed
            .slots
            .iter()
            .map(|s| serde_json::json!({ "index": s.index, "path": s.path, "ordinal": s.ordinal }))
            .collect();
        let exp: Vec<String> = composed.experimental.iter().map(describe).collect();
        let preset_json = preset_params.as_ref().map(preset_params_json);
        let v = serde_json::json!({
            "schema": SCHEMA,
            "template": template,
            "template_with_origins": with_origins,
            "wrapper": wrapper_name(wrapper),
            "slots": slots,
            "internal_key_path": composed.internal_key_path,
            "experimental": exp,
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
