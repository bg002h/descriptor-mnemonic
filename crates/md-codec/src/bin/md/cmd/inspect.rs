use crate::error::CliError;
use crate::format::text;
use md_codec::decode::decode_md1_string;
use md_codec::chunk::reassemble;
use md_codec::identity::{compute_md1_encoding_id, compute_wallet_descriptor_template_id, compute_wallet_policy_id};

pub fn run(strings: &[String]) -> Result<(), CliError> {
    let descriptor = if strings.len() == 1 {
        decode_md1_string(&strings[0])?
    } else {
        let refs: Vec<&str> = strings.iter().map(String::as_str).collect();
        reassemble(&refs)?
    };

    println!("template: {}", text::descriptor_to_template(&descriptor)?);
    println!("n: {}", descriptor.n);
    println!("wallet-policy-mode: {}", descriptor.is_wallet_policy());

    let md1 = compute_md1_encoding_id(&descriptor)?;
    println!("md1-encoding-id: {}", text::fmt_md1_id(&md1));

    let tpl = compute_wallet_descriptor_template_id(&descriptor)?;
    println!("wallet-descriptor-template-id: {}", text::fmt_template_id(&tpl));

    let pid = compute_wallet_policy_id(&descriptor)?;
    println!("wallet-policy-id: {}", text::fmt_policy_id(&pid));
    println!("wallet-policy-id-fingerprint: {}", text::fmt_policy_id_fingerprint(&pid));

    Ok(())
}
