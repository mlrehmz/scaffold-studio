//! Language plug-in boundary and TypeScript scaffold generation (M5).
//!
//! `plan` and `materialize` are split on purpose: today `plan` is a deterministic
//! template; later an LLM proposes the plan and Rust only validates it
//! (every box present, no invented files) before writing files.

use crate::model::Design;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

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

    fn check(&self, dir: &Path) -> Result<CheckResult, String> {
        let output = Command::new("bunx")
            .args(["tsc", "--noEmit", "-p", "."])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("cannot run bunx: {e}; install Bun (https://bun.sh) or TypeScript"))?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(CheckResult {
            ok: output.status.success(),
            output: text,
        })
    }

    // Read code back into a graph for drift. Walk `<dir>/src/*.ts` in sorted order.
    // A file is a box if its FIRST line is `// @box <id>`. For each line of the form
    // `import "./<name>";` (also accept `import ... from "./<name>";`) find the box whose FILE is
    // `<name>.ts` and record (this box, that box). Result: sorted `boxes`, sorted `imports`,
    // no duplicates. LAZY: `walkdir` and `regex` are already in Cargo.toml, use them (e.g.
    // `Regex::new(r#"^import (?:.* from )?"\./([\w-]+)";"#)`, compile it once outside the loop).
    // Regexes are crude on purpose (tree-sitter comes later). Test: `read_back_finds_boxes_and_imports`.
    fn read_back(&self, dir: &Path) -> Result<CodeGraph, String> {
        let src_dir = dir.join("src");
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(&src_dir) {
            let entry = entry.map_err(|e| format!("cannot read {}: {e}", src_dir.display()))?;
            if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "ts") {
                paths.push(entry.path().to_path_buf());
            }
        }
        paths.sort();

        let mut boxes_by_file = BTreeMap::new();
        let mut boxes = Vec::new();
        let mut sources = Vec::new();
        for path in paths {
            let source = fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let box_id = source
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("// @box "))
                .filter(|id| !id.is_empty())
                .map(str::to_string);
            if let Some(id) = &box_id {
                let file_name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| format!("cannot read TypeScript file name: {}", path.display()))?;
                boxes_by_file.insert(file_name.to_string(), id.clone());
                boxes.push(id.clone());
            }
            sources.push((box_id, source));
        }

        let import_pattern = Regex::new(r#"^import (?:.* from )?"\./([\w-]+)";"#)
            .map_err(|e| format!("cannot compile TypeScript import pattern: {e}"))?;
        let mut imports = Vec::new();
        for (box_id, source) in sources {
            if let Some(from) = box_id {
                for line in source.lines() {
                    if let Some(captures) = import_pattern.captures(line) {
                        if let Some(to) = captures.get(1).and_then(|name| boxes_by_file.get(name.as_str())) {
                            imports.push((from.clone(), to.clone()));
                        }
                    }
                }
            }
        }
        boxes.sort();
        boxes.dedup();
        imports.sort();
        imports.dedup();
        Ok(CodeGraph { boxes, imports })
    }
}


pub fn validate_plan(design: &Design, plan: &ScaffoldPlan) -> Vec<String> {
    let mut problems = Vec::new();
    for design_box in &design.boxes {
        if design_box.kind.has_code() && !plan.files.iter().any(|file| file.box_id == design_box.id) {
            problems.push(format!("box `{}` has no file", design_box.id));
        }
    }
    for file in &plan.files {
        if design.find(&file.box_id).is_none() {
            problems.push(format!("file `{}` has no box", file.path));
        }
    }
    let mut paths = BTreeSet::new();
    for file in &plan.files {
        if !paths.insert(file.path.as_str()) {
            problems.push(format!("duplicate path `{}`", file.path));
        }
    }
    problems
}
