use crate::error::CliError;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let (bytes, bit_len) = encode_payload(&descriptor)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let mut hex = String::with_capacity(bytes.len() * 2);
        for b in &bytes { use std::fmt::Write as _; write!(hex, "{b:02x}").unwrap(); }
        let v = serde_json::json!({
            "schema": SCHEMA,
            "payload_bits": bit_len,
            "payload_bytes": bytes.len(),
            "hex": hex,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("payload-bits: {bit_len}");
    println!("payload-bytes: {}", bytes.len());
    print!("hex: ");
    for b in &bytes { print!("{b:02x}"); }
    println!();
    Ok(())
}
