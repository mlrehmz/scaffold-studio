//! Agent notes: `INDEX.md` plus one note per step, and their linter (M4).
//!
//! DONE: rendering, writing, link helpers, the checks `missing_index`, `broken_link`, `orphan_file`.
//! TODO: the five other checks (T01-T05). Each is one small function that follows the
//! pattern of `check_broken_link`. The tests at the bottom are the spec.

use crate::process::{Process, Step};
use crate::tokens::estimate_tokens;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::Path;

pub const ENTRY_TOKEN_LIMIT: usize = 400; // INDEX.md must stay under this (estimate)
pub const MAX_HOPS: usize = 3; // clicks from INDEX.md to any note
pub const SIMILAR_TRIGGER_OVERLAP: f32 = 0.6; // word-set overlap; stands in for embeddings

// ---------------------------------------------------------------- rendering (done)

/// Relative path of a step's note, e.g. `steps/02-checkout.md`. Numbering follows process order.
pub fn step_path(index: usize, step: &Step) -> String {
    format!("steps/{:02}-{}.md", index + 1, step.id)
}

/// All step note paths in order. Replay uses this as "the planned path" (T13).
pub fn step_paths(process: &Process) -> Vec<String> {
    process.steps.iter().enumerate().map(|(i, s)| step_path(i, s)).collect()
}

/// Pure: relative path -> file content. Deterministic (no timestamps, sorted by path).
pub fn render_notes(process: &Process, project: &str) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();

    let mut index = format!(
        "# {project}: agent entry\n\nWork through the steps in order. Open a step note only when you reach that step.\n\n"
    );
    for (i, s) in process.steps.iter().enumerate() {
        index.push_str(&format!("{}. [{}]({}): {}\n", i + 1, s.id, step_path(i, s), s.trigger));
    }
    files.insert("INDEX.md".to_string(), index);

    for (i, s) in process.steps.iter().enumerate() {
        files.insert(step_path(i, s), render_step(i, s));
    }
    files
}

fn render_step(i: usize, s: &Step) -> String {
    let mut out = format!(
        "# Step {:02}: {}\n\nBox: `{}`\nTrigger: {}\n\nFinish when:\n",
        i + 1,
        s.id,
        s.box_id,
        s.trigger
    );
    for item in &s.finish {
        out.push_str(&format!("- [ ] {item}\n"));
    }
    let tools = if s.tools.is_empty() {
        "none".to_string()
    } else {
        s.tools.join(", ")
    };
    out.push_str(&format!("\nTools: {tools}\n"));
    if let Some(h) = &s.helper_agent {
        out.push_str(&format!(
            "\nHelper agent:\n- Spawn when: {}\n- Do not spawn when: {}\n- Context files: {}\n- Return format: {}\n- Must not touch: {}\n",
            h.spawn_when,
            h.do_not_spawn_when,
            h.context_files.join(", "),
            h.return_format,
            h.must_not_touch.join(", ")
        ));
    }
    out
}

/// Writes `INDEX.md` and `steps/NN-<id>.md` inside `notes_dir` (created if needed).
pub fn write_notes(process: &Process, project: &str, notes_dir: &Path) -> Result<(), String> {
    for (rel, content) in render_notes(process, project) {
        let path = notes_dir.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(())
}

// ---------------------------------------------------------------- link helpers (done)

/// Targets of Markdown links `[text](target)`. Skips web links and pure `#anchors`; drops `#fragments`.
pub fn extract_links(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else { break };
        let target = rest[..end].split('#').next().unwrap_or("").trim();
        let is_web = target.contains("://") || target.starts_with("mailto:");
        if !target.is_empty() && !is_web {
            out.push(target.to_string());
        }
        rest = &rest[end..];
    }
    out
}

/// Resolve `target` relative to the folder of `from_file` and normalise `..` and `.`.
/// Both are paths relative to the notes dir, with `/` separators.
/// `resolve_link("steps/01-a.md", "../INDEX.md") == "INDEX.md"`.
pub fn resolve_link(from_file: &str, target: &str) -> String {
    let mut parts: Vec<&str> = from_file.split('/').collect();
    parts.pop(); // drop the file name, keep its folder
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

// ---------------------------------------------------------------- linter

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lint {
    /// Name of the check, e.g. `broken_link`. Tests look for this name.
    pub check: &'static str,
    pub severity: Severity,
    /// File (relative to the notes dir) or `step:<id>`; the UI paints the problem there.
    pub path: String,
    pub message: String,
}

/// Lint path for a problem that belongs to a step (not to a file): `step:<id>`. Use it in T03-T05.
pub fn step_ref(step: &Step) -> String {
    format!("step:{}", step.id)
}

/// Everything a check may look at. Checks are pure: Ctx in, Vec<Lint> out.
pub struct Ctx<'a> {
    /// Every `.md` file under the notes dir: relative path -> content.
    pub files: BTreeMap<String, String>,
    pub process: &'a Process,
}

pub const INDEX: &str = "INDEX.md";

/// A check is a pure function: Ctx in, problems out.
type Check = fn(&Ctx) -> Vec<Lint>;

/// To add a check: write one `fn check_x(ctx: &Ctx) -> Vec<Lint>` and add it to this list. Nothing else changes.
const CHECKS: &[Check] = &[
    check_broken_link,
    check_orphan_file,
    check_too_many_hops,
    check_entry_too_big,
    check_trigger_and_finish,
    check_helper_agent,
    check_similar_triggers,
];

/// Runs all checks. Output is sorted, so it is deterministic.
pub fn lint_notes(notes_dir: &Path, process: &Process) -> Result<Vec<Lint>, String> {
    let mut files = BTreeMap::new();
    collect_md(notes_dir, notes_dir, &mut files)?;
    let ctx = Ctx { files, process };

    if !ctx.files.contains_key(INDEX) {
        return Ok(vec![Lint {
            check: "missing_index",
            severity: Severity::Error,
            path: INDEX.into(),
            message: "the entry page INDEX.md does not exist".into(),
        }]);
    }

    let mut all: Vec<Lint> = CHECKS.iter().flat_map(|check| check(&ctx)).collect();
    all.sort_by(|a, b| (&a.path, a.check, &a.message).cmp(&(&b.path, b.check, &b.message)));
    Ok(all)
}

fn collect_md(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_md(root, &path, out)?;
        } else if path.extension().map_or(false, |x| x == "md") {
            let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.insert(rel, fs::read_to_string(&path).map_err(|e| e.to_string())?);
        }
    }
    Ok(())
}

/// Distance (in links) from INDEX.md to every file that can be reached. INDEX.md itself is 0.
/// Links to files that do not exist are ignored here (`check_broken_link` reports them).
pub fn reading_depths(files: &BTreeMap<String, String>) -> BTreeMap<String, usize> {
    let mut depth = BTreeMap::new();
    let mut queue = VecDeque::new();
    depth.insert(INDEX.to_string(), 0);
    queue.push_back(INDEX.to_string());
    while let Some(file) = queue.pop_front() {
        let d = depth[&file];
        for target in extract_links(files.get(&file).map(String::as_str).unwrap_or("")) {
            let next = resolve_link(&file, &target);
            if files.contains_key(&next) && !depth.contains_key(&next) {
                depth.insert(next.clone(), d + 1);
                queue.push_back(next);
            }
        }
    }
    depth
}

// ---- done: copy this style for the TODO checks below

fn check_broken_link(ctx: &Ctx) -> Vec<Lint> {
    let mut out = Vec::new();
    for (file, content) in &ctx.files {
        for target in extract_links(content) {
            if !ctx.files.contains_key(&resolve_link(file, &target)) {
                out.push(Lint {
                    check: "broken_link",
                    severity: Severity::Error,
                    path: file.clone(),
                    message: format!("link to `{target}` does not point to an existing note"),
                });
            }
        }
    }
    out
}

fn check_orphan_file(ctx: &Ctx) -> Vec<Lint> {
    let reachable = reading_depths(&ctx.files);
    ctx.files
        .keys()
        .filter(|f| !reachable.contains_key(*f))
        .map(|f| Lint {
            check: "orphan_file",
            severity: Severity::Error,
            path: f.clone(),
            message: "not reachable from INDEX.md by following links".into(),
        })
        .collect()
}

// ---- TODO: five small checks

fn check_too_many_hops(ctx: &Ctx) -> Vec<Lint> {
    reading_depths(&ctx.files)
        .into_iter()
        .filter(|(_, depth)| *depth > MAX_HOPS)
        .map(|(path, depth)| Lint {
            check: "too_many_hops",
            severity: Severity::Error,
            path,
            message: format!("needs {depth} hops from INDEX.md (max {MAX_HOPS})"),
        })
        .collect()
}

fn check_entry_too_big(ctx: &Ctx) -> Vec<Lint> {
    let Some(index) = ctx.files.get(INDEX) else {
        return Vec::new();
    };
    let tokens = estimate_tokens(index);
    if tokens <= ENTRY_TOKEN_LIMIT {
        return Vec::new();
    }
    vec![Lint {
        check: "entry_too_big",
        severity: Severity::Error,
        path: INDEX.to_string(),
        message: format!("INDEX.md needs {tokens} tokens (max {ENTRY_TOKEN_LIMIT})"),
    }]
}

fn check_trigger_and_finish(ctx: &Ctx) -> Vec<Lint> {
    let mut lints = Vec::new();
    for step in &ctx.process.steps {
        let path = step_ref(step);
        if step.trigger.trim().is_empty() {
            lints.push(Lint {
                check: "missing_trigger",
                severity: Severity::Error,
                path: path.clone(),
                message: "trigger is empty".into(),
            });
        }
        if step.finish.is_empty() {
            lints.push(Lint {
                check: "missing_finish",
                severity: Severity::Error,
                path,
                message: "finish list is empty".into(),
            });
        }
    }
    lints
}

fn check_helper_agent(ctx: &Ctx) -> Vec<Lint> {
    let mut lints = Vec::new();
    for step in &ctx.process.steps {
        let Some(helper) = &step.helper_agent else {
            continue;
        };
        let mut missing = Vec::new();
        if helper.spawn_when.trim().is_empty() {
            missing.push("spawn_when");
        }
        if helper.do_not_spawn_when.trim().is_empty() {
            missing.push("do_not_spawn_when");
        }
        if helper.context_files.is_empty() {
            missing.push("context_files");
        }
        if helper.return_format.trim().is_empty() || !helper.return_format.chars().any(|c| c.is_ascii_digit()) {
            missing.push("return_format");
        }
        if helper.must_not_touch.is_empty() {
            missing.push("must_not_touch");
        }
        for field in missing {
            lints.push(Lint {
                check: "helper_agent_incomplete",
                severity: Severity::Error,
                path: step_ref(step),
                message: format!("helper agent is missing {field}"),
            });
        }
    }
    lints
}

fn check_similar_triggers(ctx: &Ctx) -> Vec<Lint> {
    let steps = &ctx.process.steps;
    let mut lints = Vec::new();
    for i in 0..steps.len() {
        let first = trigger_words(&steps[i].trigger);
        if first.is_empty() {
            continue;
        }
        for second_step in &steps[i + 1..] {
            let second = trigger_words(&second_step.trigger);
            if second.is_empty() {
                continue;
            }
            let intersection = first.intersection(&second).count();
            let union = first.union(&second).count();
            let overlap = intersection as f32 / union as f32;
            if overlap >= SIMILAR_TRIGGER_OVERLAP {
                lints.push(Lint {
                    check: "similar_triggers",
                    severity: Severity::Warning,
                    path: step_ref(&steps[i]),
                    message: format!("trigger is similar to {} ({overlap:.2})", second_step.id),
                });
            }
        }
    }
    lints
}

fn trigger_words(trigger: &str) -> BTreeSet<String> {
    trigger
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Design;
    use crate::process::{derive_process, HelperAgent};
    use std::path::PathBuf;

    fn shop_process() -> Process {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../examples/shop/design.toml"
        ));
        derive_process(&Design::from_toml(text).unwrap()).unwrap()
    }

    /// Writes good notes for `process` into a fresh temp dir that is unique per test name.
    fn fixture(name: &str, process: &Process) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("studio-notes-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write_notes(process, "shop", &dir).unwrap();
        dir
    }

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn checks(lints: &[Lint]) -> Vec<&'static str> {
        lints.iter().map(|l| l.check).collect()
    }

    #[test]
    fn link_helpers() {
        assert_eq!(
            extract_links("a [x](steps/a.md#top) b [web](https://x.io) [y](#here) [z](../INDEX.md)"),
            ["steps/a.md", "../INDEX.md"]
        );
        assert_eq!(resolve_link("steps/01-a.md", "../INDEX.md"), "INDEX.md");
        assert_eq!(resolve_link("INDEX.md", "steps/01-a.md"), "steps/01-a.md");
    }

    #[test]
    fn notes_are_deterministic_and_linked_in_order() {
        let p = shop_process();
        assert_eq!(render_notes(&p, "shop"), render_notes(&p, "shop"));
        let index = &render_notes(&p, "shop")["INDEX.md"];
        assert!(index.contains("1. [orders_db](steps/01-orders_db.md)"));
        assert!(index.contains("3. [web](steps/03-web.md)"));
    }

    #[test]
    fn good_notes_have_no_lints() {
        let p = shop_process();
        assert_eq!(lint_notes(&fixture("good", &p), &p).unwrap(), vec![]);
    }

    #[test]
    fn missing_index_is_reported() {
        let p = shop_process();
        let dir = fixture("noindex", &p);
        fs::remove_file(dir.join("INDEX.md")).unwrap();
        assert_eq!(checks(&lint_notes(&dir, &p).unwrap()), ["missing_index"]);
    }

    #[test]
    fn broken_link_is_reported() {
        let p = shop_process();
        let dir = fixture("broken", &p);
        write(
            &dir,
            "INDEX.md",
            &(render_notes(&p, "shop")["INDEX.md"].clone() + "\n[gone](steps/99-gone.md)\n"),
        );
        assert!(checks(&lint_notes(&dir, &p).unwrap()).contains(&"broken_link"));
    }

    #[test]
    fn orphan_file_is_reported() {
        let p = shop_process();
        let dir = fixture("orphan", &p);
        write(&dir, "steps/99-lonely.md", "# nobody links here\n");
        let lints = lint_notes(&dir, &p).unwrap();
        assert_eq!(checks(&lints), ["orphan_file"]);
        assert_eq!(lints[0].path, "steps/99-lonely.md");
    }

    #[test]
    fn too_many_hops_is_reported() {
        let p = shop_process();
        let dir = fixture("hops", &p);
        // INDEX -> steps/a (1) -> steps/b (2) -> steps/c (3) -> steps/d (4: too far)
        let index = render_notes(&p, "shop")["INDEX.md"].clone();
        write(&dir, "INDEX.md", &(index + "\n[a](steps/a.md)\n"));
        write(&dir, "steps/a.md", "[b](b.md)");
        write(&dir, "steps/b.md", "[c](c.md)");
        write(&dir, "steps/c.md", "[d](d.md)");
        write(&dir, "steps/d.md", "end");
        let lints = lint_notes(&dir, &p).unwrap();
        assert_eq!(checks(&lints), ["too_many_hops"]);
        assert_eq!(lints[0].path, "steps/d.md");
    }

    #[test]
    fn entry_too_big_is_reported() {
        let p = shop_process();
        let dir = fixture("big", &p);
        let index = render_notes(&p, "shop")["INDEX.md"].clone();
        write(&dir, "INDEX.md", &(index + &"filler ".repeat(300))); // ~2100 chars = ~525 tokens
        assert_eq!(checks(&lint_notes(&dir, &p).unwrap()), ["entry_too_big"]);
    }

    #[test]
    fn missing_trigger_and_finish_are_reported() {
        let mut p = shop_process();
        p.steps[0].trigger = "   ".into();
        p.steps[1].finish.clear();
        let lints = lint_notes(&fixture("trigger", &p), &p).unwrap();
        assert_eq!(checks(&lints), ["missing_finish", "missing_trigger"]); // sorted by path: step:checkout, step:orders_db
        assert_eq!(lints[0].path, "step:checkout");
        assert_eq!(lints[1].path, "step:orders_db");
    }

    #[test]
    fn helper_agent_missing_return_format_is_reported() {
        let mut p = shop_process();
        p.steps[0].helper_agent = Some(HelperAgent {
            spawn_when: "the test suite takes over 5 minutes".into(),
            do_not_spawn_when: "the change fits in one file".into(),
            context_files: vec!["src/orders_db.ts".into()],
            return_format: String::new(), // <- the missing field
            must_not_touch: vec!["design.toml".into()],
        });
        let lints = lint_notes(&fixture("helper", &p), &p).unwrap();
        assert_eq!(checks(&lints), ["helper_agent_incomplete"]);
        assert!(lints[0].message.contains("return_format"));
        // A return format without a number has no length limit: also reported.
        p.steps[0].helper_agent.as_mut().unwrap().return_format = "a short summary".into();
        assert_eq!(
            checks(&lint_notes(&fixture("helper2", &p), &p).unwrap()),
            ["helper_agent_incomplete"]
        );
        // A complete helper is fine.
        p.steps[0].helper_agent.as_mut().unwrap().return_format = "bullet list, max 200 words".into();
        assert_eq!(lint_notes(&fixture("helper3", &p), &p).unwrap(), vec![]);
    }

    #[test]
    fn similar_triggers_are_a_warning() {
        let mut p = shop_process();
        p.steps[0].trigger = "implement the payment service api".into();
        p.steps[1].trigger = "implement the payment service api layer".into(); // overlap 5/6
        p.steps[2].trigger = "draw the checkout form".into();
        let lints = lint_notes(&fixture("similar", &p), &p).unwrap();
        assert_eq!(checks(&lints), ["similar_triggers"]);
        assert_eq!(lints[0].severity, Severity::Warning);
        assert_eq!(lints[0].path, "step:orders_db");
    }
}
