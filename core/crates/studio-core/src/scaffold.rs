//! Language plug-in boundary + scaffold generation (M5). Types are final; the TypeScript plugin is TODO (T09-T14).
//!
//! `plan` and `materialize` are split on purpose: today `plan` is a deterministic
//! template; later an LLM proposes the plan and Rust only validates it
//! (every box present, no invented files) before writing files.

use crate::model::Design;
use crate::not_implemented;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One file to generate. The plan says WHAT; `materialize` decides HOW it looks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedFile {
    /// Relative to the project root, `/` separators. TypeScript: `src/<box id>.ts`.
    pub path: String,
    pub box_id: String,
    /// The box's responsibility text, copied verbatim (written as a comment in the file).
    pub responsibility: String,
    /// Ids of boxes this file imports (edges of kind `calls` or `uses`), sorted, no duplicates.
    pub imports: Vec<String>,
    /// The box's members, copied verbatim, e.g. `placeOrder(cart): Order`.
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScaffoldPlan {
    pub files: Vec<PlannedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    pub ok: bool,
    pub output: String,
}

/// Boxes and imports found in real code (used by drift).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CodeGraph {
    pub boxes: Vec<String>,
    pub imports: Vec<(String, String)>, // (from box, to box)
}

pub trait LanguagePlugin {
    fn plan(&self, design: &Design) -> Result<ScaffoldPlan, String>;
    fn materialize(&self, plan: &ScaffoldPlan) -> Result<Vec<GeneratedFile>, String>;
    /// e.g. run `tsc --noEmit` in `dir`.
    fn check(&self, dir: &Path) -> Result<CheckResult, String>;
    /// Scan `// @box <id>` markers and imports.
    fn read_back(&self, dir: &Path) -> Result<CodeGraph, String>;
}

pub struct TypeScriptPlugin;

/// Look at `tests/golden/shop_ts/` first: it is the exact output expected for `examples/shop`.
/// Tests: `core/crates/studio-core/tests/scaffold_golden.rs`.
impl LanguagePlugin for TypeScriptPlugin {
    fn plan(&self, d: &Design) -> Result<ScaffoldPlan, String> {
        let mut files = Vec::new();
        for design_box in d.boxes.iter().filter(|design_box| design_box.kind.has_code()) {
            let mut imports: Vec<String> = d
                .edges
                .iter()
                .filter(|edge| edge.from == design_box.id && edge.kind.is_dependency())
                .map(|edge| edge.to.clone())
                .collect();
            imports.sort();
            imports.dedup();

            files.push(PlannedFile {
                path: format!("src/{}.ts", design_box.id),
                box_id: design_box.id.clone(),
                responsibility: design_box.responsibility.clone(),
                imports,
                members: design_box.members.clone(),
            });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(ScaffoldPlan { files })
    }

    fn materialize(&self, p: &ScaffoldPlan) -> Result<Vec<GeneratedFile>, String> {
        let mut files = vec![GeneratedFile {
            path: "tsconfig.json".into(),
            content: include_str!("../tests/golden/shop_ts/tsconfig.json").into(),
        }];
        for planned_file in &p.files {
            let mut content = format!("// @box {}\n// {}\n", planned_file.box_id, planned_file.responsibility);
            for import in &planned_file.imports {
                content.push_str(&format!("import \"./{import}\";\n"));
            }
            content.push_str("\nexport {};\n");

            for member in &planned_file.members {
                content.push_str(&format!("\n// signature: {member}\n"));
                let mut chars = member.chars();
                match chars.next() {
                    Some(first) if first.is_ascii_alphabetic() || first == '_' || first == '$' => {
                        let name = member
                            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
                            .next()
                            .unwrap_or_default();
                        content.push_str(&format!(
                            "export function {name}(..._args: unknown[]): never {{\n  throw new Error(\"TODO(step: {}): {member}\");\n}}\n",
                            planned_file.box_id
                        ));
                    }
                    _ => content.push_str(&format!("// TODO: unparseable member: {member}\n")),
                }
            }

            files.push(GeneratedFile {
                path: planned_file.path.clone(),
                content,
            });
        }
        Ok(files)
    }

    fn check(&self, _dir: &Path) -> Result<CheckResult, String> {
        Err(not_implemented("TypeScriptPlugin::check ()"))
    }

    fn read_back(&self, _dir: &Path) -> Result<CodeGraph, String> {
        Err(not_implemented("TypeScriptPlugin::read_back ()"))
    }
}


pub fn validate_plan(_design: &Design, _plan: &ScaffoldPlan) -> Vec<String> {
    Vec::new()
}

