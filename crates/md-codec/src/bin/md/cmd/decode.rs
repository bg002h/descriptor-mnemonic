use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let template = text::descriptor_to_template(&descriptor)?;
    println!("{template}");
    Ok(())
}
