use md_codec::encode::strip_display_separators;

/// Strip mstring display separators (SPEC §3.2) from each md1 input string so a
/// grouped or unbroken card both re-ingest. Applied at every md1-intake site.
pub fn strip_md1_inputs(strings: &[String]) -> Vec<String> {
    strings
        .iter()
        .map(|s| strip_display_separators(s))
        .collect()
}

pub mod address;
pub mod bytecode;
#[cfg(feature = "cli-compiler")]
pub mod compile;
pub mod decode;
pub mod encode;
#[cfg(feature = "json")]
pub mod gui_schema;
pub mod inspect;
pub mod repair;
pub mod vectors;
pub mod verify;
