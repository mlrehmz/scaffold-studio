use crate::{
    notes::{render_notes, INDEX},
    process::Process,
};
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAdapter {
    Codex,
    ClaudeCode,
}

impl AgentAdapter {
    pub fn write_entry(&self, process: &Process, project: &str, output_dir: &Path) -> Result<(), String> {
        let file_name = match self {
            Self::Codex => "AGENTS.md",
            Self::ClaudeCode => "CLAUDE.md",
        };
        let mut notes = render_notes(process, project);
        let content = notes
            .remove(INDEX)
            .ok_or_else(|| "agent entry page is missing".to_string())?;
        if !output_dir.as_os_str().is_empty() {
            fs::create_dir_all(output_dir).map_err(|e| format!("cannot create {}: {e}", output_dir.display()))?;
        }
        let path = output_dir.join(file_name);
        fs::write(&path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::AgentAdapter;
    use crate::{
        notes::{render_notes, INDEX},
        process::{Process, Step},
    };
    use std::fs;

    fn process() -> Process {
        Process {
            steps: vec![Step {
                id: "checkout".into(),
                box_id: "checkout".into(),
                trigger: "A customer checks out".into(),
                finish: vec!["The order is saved".into()],
                tools: vec![],
                draft: false,
                helper_agent: None,
            }],
        }
    }

    #[test]
    fn writes_the_rendered_entry_under_each_tool_filename() {
        let process = process();
        let expected = render_notes(&process, "shop")[INDEX].clone();
        let temp = tempfile::tempdir().unwrap();
        let output_dir = temp.path().join("agent");

        AgentAdapter::Codex.write_entry(&process, "shop", &output_dir).unwrap();
        AgentAdapter::ClaudeCode
            .write_entry(&process, "shop", &output_dir)
            .unwrap();

        assert_eq!(fs::read_to_string(output_dir.join("AGENTS.md")).unwrap(), expected);
        assert_eq!(fs::read_to_string(output_dir.join("CLAUDE.md")).unwrap(), expected);
        assert!(!output_dir.join("INDEX.md").exists());
    }

    #[test]
    fn reports_when_the_output_directory_is_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let output_dir = temp.path().join("file");
        fs::write(&output_dir, "content").unwrap();

        let error = AgentAdapter::Codex
            .write_entry(&Process::default(), "shop", &output_dir)
            .unwrap_err();

        assert!(error.contains("cannot create"));
    }
}
