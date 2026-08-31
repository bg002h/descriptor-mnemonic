use crate::error::CliError;
use md_codec::chunk::reassemble;
use md_codec::decode::decode_md1_string;
use md_codec::encode::encode_payload;

pub fn run(
    strings: &[String],
    in_file: Option<&std::path::Path>,
    json: bool,
) -> Result<u8, CliError> {
    // P3 §6b: argv, `--in FILE` or `-`; separators stripped on intake (§3.2).
    let strings = crate::cmd::read_md1_inputs(strings, in_file, "--in")?;
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    // N1's WARN disposition on the CARD (plan P3 step 2, Acceptance 5). A
    // plate already carrying a shape this cycle newly refuses must still
    // READ, or the refusal has taken away the only tool that could tell its
    // holder what they have. Same predicate and same body as the refusal --
    // only the disposition differs (`crate::parse::reuse`).
    crate::parse::reuse::check_descriptor(&descriptor, crate::parse::reuse::Disposition::Warn)?;
    let (bytes, bit_len) = encode_payload(&descriptor)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let mut hex = String::with_capacity(bytes.len() * 2);
        for b in &bytes {
            use std::fmt::Write as _;
            write!(hex, "{b:02x}").unwrap();
        }
        let v = serde_json::json!({
            "schema": SCHEMA,
            "payload_bits": bit_len,
            "payload_bytes": bytes.len(),
            "hex": hex,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        crate::output_advisory::emit_output_class_advisory(
            crate::output_advisory::OutputClass::Template,
            &mut std::io::stderr(),
        );
        return Ok(0);
    }
    let _ = json;

    println!("payload-bits: {bit_len}");
    println!("payload-bytes: {}", bytes.len());
    print!("hex: ");
    for b in &bytes {
        print!("{b:02x}");
    }
    println!();
    crate::output_advisory::emit_output_class_advisory(
        crate::output_advisory::OutputClass::Template,
        &mut std::io::stderr(),
    );
    Ok(0)
}
