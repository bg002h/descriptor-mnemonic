use crate::error::CliError;
use crate::compile::{compile_policy_to_template, ScriptContext};

pub fn run(expr: &str, ctx_str: &str, json: bool) -> Result<(), CliError> {
    let ctx: ScriptContext = ctx_str.parse().map_err(|e: crate::compile::CompileError| {
        CliError::Compile(e.to_string())
    })?;
    let template = compile_policy_to_template(expr, ctx).map_err(CliError::from)?;

    #[cfg(feature = "json")]
    if json {
        use crate::format::json::SCHEMA;
        let v = serde_json::json!({ "schema": SCHEMA, "template": template, "context": ctx_str });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(());
    }
    let _ = json;

    println!("{template}");
    Ok(())
}
