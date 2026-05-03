use crate::error::CliError;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::encode::encode_payload;

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let (bytes, bit_len) = encode_payload(&descriptor)?;
    println!("payload-bits: {bit_len}");
    println!("payload-bytes: {}", bytes.len());
    print!("hex: ");
    for b in &bytes { print!("{b:02x}"); }
    println!();
    Ok(())
}
