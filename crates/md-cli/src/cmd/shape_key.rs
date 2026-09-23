//! `md shape-key` -- print a policy's `SkeletonKey` (coordinator-compat plan
//! 1b; design §2, "the shape key has exactly one implementation").
//!
//! A thin CLI over md-codec: an md1 card goes through `reassemble` and
//! `skeleton`, and `--descriptor` goes through
//! `md_codec::descriptor_route::skeleton_key_of_text` -- the same function
//! `cargo xtask verdicts` keys the evidence with. Nothing here computes a key.
//! `tests/cli_shape_key.rs` asserts the CLI and the library agree on every
//! vendored evidence row.

use crate::error::CliError;
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::skeleton::{skeleton, skeleton_key};

pub fn run(phrases: &[String], descriptor: Option<&str>) -> Result<u8, CliError> {
    let key = match descriptor {
        Some(text) => md_codec::descriptor_route::skeleton_key_of_text(text)
            .map_err(|e| CliError::BadArg(format!("shape-key: {e}")))?,
        None => {
            let strings = crate::cmd::strip_md1_inputs(phrases);
            let d = if strings.len() == 1 {
                decode_md1_string(&strings[0])?
            } else {
                let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
                reassemble(&refs)?
            };
            // Design §1A (a3): a card whose keys will not expand has no key.
            let s = skeleton(&d).map_err(|e| CliError::BadArg(format!("shape-key: {e}")))?;
            skeleton_key(&s)
        }
    };
    // The key's own spelling, U+001F separators included (design §1A): this
    // is the string the evidence table is keyed by, byte for byte.
    println!("{}", key.as_str());
    Ok(0)
}
