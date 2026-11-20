//! `studio`: thin CLI around studio-core. Reads files, prints JSON.
//! Bun, the UI, and any AI all go through this one binary.
//!
//! Exit codes: 0 ok, 1 checked and not ok, 2 usage or I/O error.

use serde::Serialize;
use std::{env, fs, path::Path, process::exit};
use studio_core::{
    drift::check_drift,
    evaluate::evaluate_json,
    model::Design,
    notes::{lint_notes, step_paths, write_notes, Severity},
    process::{derive_process, merge_process, Process},
    rules::check_rules,
    scaffold::{validate_plan, LanguagePlugin, TypeScriptPlugin},
    tokens::estimate_tokens,
    validate::Issue,
};

const USAGE: &str = "usage:
  studio check    <design.toml>                 validate structure
  studio show     <design.toml>                 design + current rule violations (JSON)
  studio evaluate <design.toml> <proposal.json> verdict, writes nothing
  studio apply    <design.toml> <proposal.json> evaluate, and write design.toml only if ok
  studio lock     <design.toml> <id> [--unlock] lock or unlock a box (humans only)
  studio process  <design.toml> <process.toml>  derive steps, ADD missing ones to process.toml, print JSON
  studio notes    <design.toml> <process.toml> <notes_dir>   write INDEX.md + steps/*.md
  studio lint     <process.toml> <notes_dir>    lint the notes (exit 1 if any error)
  studio scaffold <design.toml> <out_dir>       generate a TypeScript project
  studio drift    <design.toml> <project_dir>   report design/code drift
later (not implemented): replay, score, schema, context";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => exit(code),
        Err(msg) => {
            eprintln!("{msg}");
            exit(2);
        }
    }
}

fn run(args: &[String]) -> Result<i32, String> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["check", path] => {
            let issues = load(path).err().unwrap_or_default();
            print_json(&Checked {
                ok: issues.is_empty(),
                issues: &issues,
            });
            Ok(if issues.is_empty() { 0 } else { 1 })
        }
        ["show", path] => match load(path) {
            Ok(design) => {
                print_json(&Shown {
                    violations: check_rules(&design),
                    design,
                });
                Ok(0)
            }
            Err(issues) => {
                print_json(&Checked {
                    ok: false,
                    issues: &issues,
                });
                Ok(1)
            }
        },
        ["evaluate", path, proposal] => {
            let verdict = evaluate_json(&load_or_exit(path)?, &read(proposal)?);
            print_json(&verdict);
            Ok(if verdict.ok { 0 } else { 1 })
        }
        ["apply", path, proposal] => {
            let verdict = evaluate_json(&load_or_exit(path)?, &read(proposal)?);
            if let (true, Some(next)) = (verdict.ok, &verdict.preview) {
                fs::write(path, next.to_toml()).map_err(|e| format!("cannot write {path}: {e}"))?;
            }
            print_json(&verdict);
            Ok(if verdict.ok { 0 } else { 1 })
        }
        ["lock", path, id, rest @ ..] => {
            let locked = !rest.contains(&"--unlock");
            let mut design = load_or_exit(path)?;
            let b = design
                .boxes
                .iter_mut()
                .find(|b| b.id == *id)
                .ok_or(format!("unknown box `{id}`"))?;
            b.locked = locked;
            fs::write(path, design.to_toml()).map_err(|e| format!("cannot write {path}: {e}"))?;
            Ok(0)
        }
        ["process", design, process_path] => {
            let derived = derive_process(&load_or_exit(design)?)?;
            // Missing file = start from nothing. A broken file is an error, never silently replaced.
            let existing = match fs::read_to_string(process_path) {
                Ok(text) => Process::from_toml(&text).map_err(|e| format!("{process_path}: {e}"))?,
                Err(_) => Process::default(),
            };
            let merged = merge_process(existing, derived);
            fs::write(process_path, merged.to_toml()).map_err(|e| format!("cannot write {process_path}: {e}"))?;
            print_json(&merged);
            Ok(0)
        }
        ["notes", design, process_path, notes_dir] => {
            let project = load_or_exit(design)?.project.name;
            let process = Process::from_toml(&read(process_path)?)?;
            write_notes(&process, &project, std::path::Path::new(notes_dir))?;
            Ok(0)
        }
        ["lint", process_path, notes_dir] => {
            let process = Process::from_toml(&read(process_path)?)?;
            let lints = lint_notes(std::path::Path::new(notes_dir), &process)?;
            let mut note_tokens = Vec::new();
            for path in step_paths(&process) {
                let text = read(&format!("{notes_dir}/{path}"))?;
                note_tokens.push((path, estimate_tokens(&text)));
            }
            let errors = lints.iter().filter(|l| l.severity == Severity::Error).count();
            print_json(&Linted {
                ok: errors == 0,
                lints,
                note_tokens,
            });
            Ok(if errors == 0 { 0 } else { 1 })
        }
        ["scaffold", design_path, out_dir] => {
            let design = load_or_exit(design_path)?;
            let plugin = TypeScriptPlugin;
            let plan = plugin.plan(&design)?;
            let problems = validate_plan(&design, &plan);
            if !problems.is_empty() {
                print_json(&problems);
                return Ok(1);
            }
            let generated = plugin.materialize(&plan)?;
            for file in &generated {
                let path = Path::new(out_dir).join(&file.path);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
                }
                fs::write(&path, &file.content).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            }
            let paths: Vec<String> = generated.iter().map(|file| file.path.clone()).collect();
            print_json(&paths);
            Ok(0)
        }
        ["drift", design_path, project_dir] => {
            let design = load_or_exit(design_path)?;
            let code = TypeScriptPlugin.read_back(Path::new(project_dir))?;
            let drift = check_drift(&design, &code);
            print_json(&drift);
            Ok(if drift.is_empty() { 0 } else { 1 })
        }
        _ => Err(USAGE.to_string()),
    }
}

#[derive(Serialize)]
struct Checked<'a> {
    ok: bool,
    issues: &'a [Issue],
}

#[derive(Serialize)]
struct Linted {
    ok: bool,
    lints: Vec<studio_core::notes::Lint>,
    note_tokens: Vec<(String, usize)>,
}

#[derive(Serialize)]
struct Shown {
    design: Design,
    violations: Vec<studio_core::rules::Violation>,
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn load(path: &str) -> Result<Design, Vec<Issue>> {
    let text = fs::read_to_string(path).map_err(|e| vec![Issue::new(path, e.to_string())])?;
    Design::from_toml(&text)
}

/// For commands that need a valid design: print the issues and stop.
fn load_or_exit(path: &str) -> Result<Design, String> {
    load(path).map_err(|issues| {
        issues
            .iter()
            .map(|i| format!("{}: {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string(value).expect("output is always serializable")
    );
}
