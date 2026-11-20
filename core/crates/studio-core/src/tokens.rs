//! Token estimates. Honest label: this is chars / 4, not a real tokenizer.

pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() + 3) / 4
}

use serde::{Deserialize, Serialize};

/// A tool profile is DATA (`profiles/*.toml`): where tasks live, which MCP to use, which commands.
/// Jira, Linear and GitHub are examples, never hard-coded integrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolProfile {
    pub name: String,
    pub tasks: String,
    pub mcp: String,
    pub commands: Vec<String>,
    /// Estimated tokens this tool's descriptions add to the agent's context.
    pub description_tokens: usize,
}

pub fn load_profile(text: &str) -> Result<ToolProfile, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

pub fn step_tokens(note_text: &str, tools: &[String], profiles: &[ToolProfile]) -> Result<usize, String> {
    let mut tokens = estimate_tokens(note_text);
    for name in tools {
        let profile = profiles
            .iter()
            .find(|profile| profile.name == *name)
            .ok_or_else(|| format!("unknown tool profile `{name}`"))?;
        tokens += profile.description_tokens;
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    fn github(tokens: usize) -> ToolProfile {
        ToolProfile {
            name: "github".into(),
            tasks: "GitHub issues".into(),
            mcp: "github".into(),
            commands: vec!["gh issue list".into()],
            description_tokens: tokens,
        }
    }

    #[test]
    fn profile_parses() {
        let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../profiles/github.toml"));
        let p = load_profile(text).unwrap();
        assert_eq!(p.name, "github");
        assert!(load_profile("name = \"x\"").is_err(), "missing fields are an error");
    }

    #[test]
    fn step_tokens_adds_tool_cost() {
        // "abcd" = 1 token, plus 100 for the tool.
        assert_eq!(
            step_tokens("abcd", &["github".to_string()], &[github(100)]).unwrap(),
            101
        );
        assert_eq!(
            step_tokens("abcd", &[], &[github(100)]).unwrap(),
            1,
            "no tools attached, no cost"
        );
    }

    #[test]
    fn unknown_tool_is_an_error() {
        let err = step_tokens("abcd", &["jira".to_string()], &[github(100)]).unwrap_err();
        assert!(err.contains("jira"));
    }
}
