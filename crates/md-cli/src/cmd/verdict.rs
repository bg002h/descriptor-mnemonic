//! The coordinator verdict notice `md compose` and `md descriptor` print on
//! stderr (coordinator-compat plan 1b; design §4 "`md` on the host").
//!
//! One copy: every line comes from `md_codec::coordinator::describe` over the
//! generated registry. md-cli holds no coordinator knowledge of its own -- the
//! Liana warnings `liana_refuse_or_warn` used to hand-write are gone.

use md_codec::coordinator::{CoordinatorVerdict, Form, describe, verdicts};
use md_codec::encode::Descriptor;
use md_codec::skeleton::skeleton;

/// Print the notice for `d` and return the verdicts, or `None` when the card
/// has no key at all (design §1A (a3): keys that will not expand, or a walk
/// the classifier does not understand), in which case one line says so and
/// no coordinator row is printed.
pub fn notice(d: &Descriptor, form: Option<Form>, what: &str) -> Option<Vec<CoordinatorVerdict>> {
    let s = match skeleton(d) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("note: coordinators: no verdict ({e})");
            return None;
        }
    };
    let vs = verdicts(&s, form);
    eprintln!("note: coordinators for {what} (verified versions only; newer: unmeasured):");
    for v in &vs {
        eprintln!("  {}", describe(v));
    }
    Some(vs)
}
