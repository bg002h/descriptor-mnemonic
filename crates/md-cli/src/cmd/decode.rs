use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("descriptor".into(), serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap());
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }
    let _ = json;

    let template = text::descriptor_to_template(&descriptor)?;
    println!("{template}");
    Ok(())
}
