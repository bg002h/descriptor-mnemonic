//! `md compose` -- the FIXED lowering surface (SPEC_wallet_policy_composer.md
//! §10 item 1). The opposite contract to `md compile`: no search, no cost
//! model, the same answer from every implementation, forever.
//!
//! Not to be confused with `crate::seat::compose`, which SEATS keys into an
//! existing keyless card; this module builds the card's policy from a path list.

use crate::error::CliError;
use md_codec::compose::{
    Experimental, KeySet, Lock, PathList, SpendPath, Wrapper, compose, template_with_origins,
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
                if value.len() != 64
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    return Err(CliError::Compose(format!(
                        "path `{s}`: sha256 needs 64 hex characters, lowercase"
                    )));
                }
                let mut h = [0u8; 32];
                for (i, chunk) in value.as_bytes().chunks(2).enumerate() {
                    let hi = (chunk[0] as char).to_digit(16).expect("checked") as u8;
                    let lo = (chunk[1] as char).to_digit(16).expect("checked") as u8;
                    h[i] = (hi << 4) | lo;
                }
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

pub fn run(
    wrapper: &str,
    paths: &[String],
    experimental: bool,
    json: bool,
) -> Result<u8, CliError> {
    let wrapper = parse_wrapper(wrapper)?;
    let paths: Vec<SpendPath> = paths
        .iter()
        .map(|p| parse_path(p))
        .collect::<Result<_, _>>()?;
    let list = PathList { wrapper, paths };
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
    // rather than accept a typed request silently.
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
        let v = serde_json::json!({
            "schema": SCHEMA,
            "template": template,
            "template_with_origins": with_origins,
            "wrapper": wrapper_name(wrapper),
            "slots": slots,
            "internal_key_path": composed.internal_key_path,
            "experimental": exp,
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
