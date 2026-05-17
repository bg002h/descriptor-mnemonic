use crate::compile::{ScriptContext, compile_policy_to_template};
use crate::error::CliError;

pub fn run(
    expr: &str,
    ctx_str: &str,
    unspendable_key: Option<&str>,
    json: bool,
) -> Result<u8, CliError> {
    let ctx: ScriptContext = ctx_str
        .parse()
        .map_err(|e: crate::compile::CompileError| CliError::Compile(e.to_string()))?;
    let template =
        compile_policy_to_template(expr, ctx, unspendable_key).map_err(CliError::from)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let v = serde_json::json!({ "schema": SCHEMA, "template": template, "context": ctx_str });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(0);
    }
    let _ = json;

    println!("{template}");
    Ok(0)
}
