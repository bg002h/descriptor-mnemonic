use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id};

pub fn run(strings: &[String], json: bool) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };
    let md1 = compute_md1_encoding_id(&descriptor)?;
    let tpl = compute_wallet_descriptor_template_id(&descriptor)?;
    let pid = compute_wallet_policy_id(&descriptor)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::{JsonDescriptor, JsonHash, SCHEMA};
        let mut obj = serde_json::Map::new();
        obj.insert("schema".into(), SCHEMA.into());
        obj.insert("descriptor".into(), serde_json::to_value(JsonDescriptor::from(&descriptor)).unwrap());
        obj.insert("md1_encoding_id".into(), serde_json::to_value(JsonHash::from(&md1)).unwrap());
        obj.insert("wallet_descriptor_template_id".into(), serde_json::to_value(JsonHash::from(&tpl)).unwrap());
        obj.insert("wallet_policy_id".into(), serde_json::to_value(JsonHash::from(&pid)).unwrap());
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("template: {}", text::descriptor_to_template(&descriptor)?);
    println!("n: {}", descriptor.n);
    println!("wallet-policy-mode: {}", descriptor.is_wallet_policy());
    println!("md1-encoding-id: {}", text::fmt_md1_id(&md1));
    println!("wallet-descriptor-template-id: {}", text::fmt_template_id(&tpl));
    println!("wallet-policy-id: {}", text::fmt_policy_id(&pid));
    println!("wallet-policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&pid));
    Ok(())
}
