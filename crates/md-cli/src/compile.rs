use crate::error::CliError;
use std::str::FromStr;

#[derive(Debug)]
pub enum CompileError {
    Parse(String),
    Compile(String),
    BadContext(String),
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "parse: {m}"),
            CompileError::Compile(m) => write!(f, "compile: {m}"),
            CompileError::BadContext(m) => write!(f, "bad-context: {m}"),
        }
    }
}
impl From<CompileError> for CliError {
    fn from(e: CompileError) -> Self { CliError::Compile(e.to_string()) }
}

#[derive(Debug, Clone, Copy)]
pub enum ScriptContext { Tap, SegwitV0 }
impl FromStr for ScriptContext {
    type Err = CompileError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s { "tap" => Ok(Self::Tap), "segwitv0" => Ok(Self::SegwitV0),
                  other => Err(CompileError::BadContext(other.into())) }
    }
}

pub fn compile_policy_to_template(expr: &str, ctx: ScriptContext) -> Result<String, CompileError> {
    use miniscript::policy::concrete::Policy;
    let policy: Policy<String> = expr.parse().map_err(|e| CompileError::Parse(format!("{e}")))?;
    match ctx {
        ScriptContext::SegwitV0 => {
            let ms = policy.compile::<miniscript::Segwitv0>().map_err(|e| CompileError::Compile(format!("{e}")))?;
            Ok(format!("wsh({ms})"))
        }
        ScriptContext::Tap => {
            // Tap requires either a key-path (`tr(KEY)`) or key-path + script-tree
            // (`tr(KEY, TREE)`). v0.15 only supports the key-path form for single-key
            // policies (e.g. `pk(@0)` → `tr(@0)`); multi-leaf trees would need a NUMS
            // internal key, which is out of scope.
            let ms = policy.compile::<miniscript::Tap>().map_err(|e| CompileError::Compile(format!("{e}")))?;
            let rendered = ms.to_string();
            if let Some(inner) = rendered.strip_prefix("pk(").and_then(|s| s.strip_suffix(')')) {
                Ok(format!("tr({inner})"))
            } else {
                Err(CompileError::Compile(format!(
                    "v0.15 cli-compiler only supports single-key tap policies (got `{rendered}`); \
                     multi-leaf policies need a NUMS internal key, which is not yet supported"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_segwitv0_pk() {
        let s = compile_policy_to_template("pk(@0)", ScriptContext::SegwitV0).unwrap();
        assert!(s.starts_with("wsh("));
        assert!(s.contains("@0"));
    }

    #[test]
    fn bad_context() {
        assert!("xpub".parse::<ScriptContext>().is_err());
    }
}
