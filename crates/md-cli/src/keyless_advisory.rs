//! The `--experimental` no-key warning (bughunt-1119 `bh-md-I8`,
//! `design/FIX_bh1119_md_output_advisories.md` section 3): the one emitter `md encode` and
//! `md decode` both call. It names what opens a spend path that needs no key: a timelock alone
//! (no secret at all), or a preimage (bearer access), or nothing.

use miniscript::DescriptorPublicKey;
use miniscript::Threshold;
use miniscript::policy::Liftable;
use miniscript::policy::semantic::Policy;
use std::sync::Arc;

/// Which verb prints the line (the text differs, the analysis does not).
#[derive(Clone, Copy)]
pub enum Verb {
    /// `md encode --experimental`.
    Encode,
    /// `md decode`, text and `--json`.
    Decode,
}

type Pol = Policy<DescriptorPublicKey>;

const S_T: &str = " A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it.";
const S_H: &str = " Whoever learns a preimage named there can spend that path without a key once any timelock beside it is met. If such a preimage is engraved, THE PLATE IS BEARER ACCESS.";
const TAIL_E: &str =
    " Malleability, resource limits, repeated keys and timelock mixing were still checked.";
const TAIL_D: &str = " It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.";

fn is_hash(p: &Pol) -> bool {
    matches!(
        p,
        Policy::Sha256(_) | Policy::Hash256(_) | Policy::Ripemd160(_) | Policy::Hash160(_)
    )
}

/// `p` with every `Key` leaf (and, when `hashes`, every hash leaf) replaced by `Unsatisfiable`.
fn strip(p: &Pol, hashes: bool) -> Pol {
    match p {
        Policy::Key(_) => Policy::Unsatisfiable,
        other if hashes && is_hash(other) => Policy::Unsatisfiable,
        Policy::Thresh(t) => {
            let kids: Vec<Arc<Pol>> = t.iter().map(|c| Arc::new(strip(c, hashes))).collect();
            Policy::Thresh(Threshold::new(t.k(), kids).expect("same k, same n"))
        }
        other => other.clone(),
    }
}

fn has_hash(p: &Pol) -> bool {
    match p {
        Policy::Thresh(t) => t.iter().any(|c| has_hash(c)),
        other => is_hash(other),
    }
}

/// The lock leaves of `p`, in Display order, each once.
fn locks<'a>(p: &'a Pol, out: &mut Vec<&'a Pol>) {
    match p {
        Policy::After(_) | Policy::Older(_) => {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        Policy::Thresh(t) => t.iter().for_each(|c| locks(c, out)),
        _ => {}
    }
}

fn lock_piece(p: &Pol) -> String {
    match p {
        Policy::After(a) => {
            let n = a.to_consensus_u32();
            if n < 500_000_000 {
                format!(
                    " after({n}) is block height {n}, not {n} blocks from now; md cannot see the chain, so if the chain is already past height {n}, that path is open today."
                )
            } else {
                format!(
                    " after({n}) is Unix time {n}; once the chain's median time passes it, that path is open."
                )
            }
        }
        Policy::Older(o) => {
            let n = o.to_consensus_u32();
            if n & (1 << 22) == 0 {
                format!(" older({n}) is {n} blocks after each coin confirms.")
            } else {
                format!(
                    " older({n}) is {} seconds after each coin confirms.",
                    (n & 0xffff) * 512
                )
            }
        }
        _ => String::new(),
    }
}

fn fallback(verb: Verb) -> String {
    match verb {
        Verb::Encode => format!(
            "warning: --experimental relaxed the signature rule. rust-miniscript could not prove that every spend path here needs a signature, and md could not name a spend path without one. Check every spend path of this descriptor before funding it.{TAIL_E}"
        ),
        Verb::Decode => format!(
            "warning: this card's policy fails the rule that every spend path needs a signature, and md could not name a spend path without one. Check every spend path before funding it.{TAIL_D}"
        ),
    }
}

/// The warning for the (relaxed) template text; one line, no trailing newline.
pub fn render_keyless_warning(template: &str, verb: Verb) -> String {
    let lifted =
        crate::parse::template::relaxed_ms_descriptor(template).and_then(|d| d.lift().ok());
    render_from(lifted.as_ref(), verb)
}

/// The `K`/`T`/text work of `render_keyless_warning`, on an already-lifted policy.
pub fn render_from(lifted: Option<&Pol>, verb: Verb) -> String {
    let Some(p) = lifted else {
        return fallback(verb);
    };
    let k = strip(p, false).normalized();
    if k.is_unsatisfiable() {
        return fallback(verb);
    }
    let (head, tail) = match verb {
        Verb::Encode => (
            "warning: --experimental relaxed the signature rule. This descriptor",
            TAIL_E,
        ),
        Verb::Decode => ("warning: this card", TAIL_D),
    };
    if k.is_trivial() {
        return format!(
            "{head} can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now.{tail}"
        );
    }
    let t = strip(p, true).normalized();
    let mut s = format!("{head} can be spent with NO key when: {k}.");
    if !t.is_unsatisfiable() {
        s.push_str(S_T);
    }
    if has_hash(&k) {
        s.push_str(S_H);
    }
    let mut ls = Vec::new();
    locks(&k, &mut ls);
    for l in ls {
        s.push_str(&lock_piece(l));
    }
    s.push_str(tail);
    s
}

/// The F-610 two-parse predicate: the rendered template is refused without `--experimental`
/// and admitted with it, i.e. the signature rule is what rejects it (a spend path needs no key).
///
/// REFUSE, NOT WARN: `Disposition::Warn` is the READ-side disposition and returns Ok, so the
/// check would silently never fire. The question is "would a MINTING verb refuse this?", so it
/// is asked in the minting disposition.
///
/// It skips the dead-leaf rule (md review r5 I-1): that rule is about another defect, and with it
/// in, a card with a dead leaf and a no-key leaf failed both parses and lost this warning.
pub(crate) fn needs_no_key(template: &str) -> bool {
    crate::parse::template::parse_template_signature_probe(template, false).is_err()
        && crate::parse::template::parse_template_signature_probe(template, true).is_ok()
}

/// stderr of `md decode --json` and `md bytecode` (text and `--json`), bh-md-M5 and bh-md-I8:
/// the no-key warning when the card has a no-key spend path (nothing is printed if the template
/// cannot be rebuilt), then the class note. A card with a no-key path and a key gets the warning
/// INSTEAD of the watch-only note (it would say "cannot spend" of a card anyone can spend); a
/// keyless card keeps the template note; otherwise watch-only when the card carries keys.
pub(crate) fn emit_card_advisories(
    descriptor: &md_codec::encode::Descriptor,
    w: &mut impl std::io::Write,
) {
    use crate::output_advisory::{OutputClass, emit_output_class_advisory};
    let wallet = descriptor.is_wallet_policy();
    let no_key = crate::format::text::descriptor_to_template(descriptor)
        .map(|t| {
            let hit = needs_no_key(&t);
            if hit {
                let _ = writeln!(w, "{}", render_keyless_warning(&t, Verb::Decode));
            }
            hit
        })
        .unwrap_or(false);
    if no_key && wallet {
        return;
    }
    let class = if wallet && !no_key {
        OutputClass::WatchOnly
    } else {
        OutputClass::Template
    };
    emit_output_class_advisory(class, w);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const KEY: &str = "tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*";

    fn want(verb: Verb) -> String {
        fallback(verb)
    }

    #[test]
    fn i8_u1_fallback_when_policy_cannot_be_lifted() {
        assert_eq!(
            render_from(None, Verb::Encode),
            "warning: --experimental relaxed the signature rule. rust-miniscript could not prove that every spend path here needs a signature, and md could not name a spend path without one. Check every spend path of this descriptor before funding it. Malleability, resource limits, repeated keys and timelock mixing were still checked."
        );
        assert_eq!(
            render_from(None, Verb::Decode),
            "warning: this card's policy fails the rule that every spend path needs a signature, and md could not name a spend path without one. Check every spend path before funding it. It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself."
        );
    }

    #[test]
    fn i8_u2_fallback_when_every_path_needs_a_key() {
        let p: Pol = Policy::Key(DescriptorPublicKey::from_str(KEY).unwrap());
        for v in [Verb::Encode, Verb::Decode] {
            assert_eq!(render_from(Some(&p), v), want(v));
        }
        assert_eq!(
            render_from(Some(&p), Verb::Decode),
            render_from(None, Verb::Decode)
        );
    }
}
