//! One smell, one pattern (M8). Catalog is data in `patterns/catalog.toml`.
//!
//! Smell: a caller has `calls` edges to 2+ concrete implementers of the same contract.
//! Pattern: Strategy, expanded deterministically into commands (they show as ghost boxes).
//! Every pattern MUST have a non-empty `reason`.

use crate::command::Command;
use crate::model::{Design, EdgeKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Smell {
    pub caller: String,
    pub contract: String,
    /// Sorted ids of the implementers the caller calls directly.
    pub implementers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternDef {
    pub id: String,
    pub smell: String,
    pub reason: String,
    pub expand: String,
}

#[derive(Deserialize)]
struct Catalog {
    #[serde(default, rename = "pattern")]
    patterns: Vec<PatternDef>,
}

pub fn load_catalog(text: &str) -> Result<Vec<PatternDef>, String> {
    let catalog = toml::from_str::<Catalog>(text).map_err(|error| error.to_string())?;
    for pattern in &catalog.patterns {
        if pattern.reason.trim().is_empty() {
            return Err(format!("pattern `{}` has no reason", pattern.id));
        }
    }
    Ok(catalog.patterns)
}

pub fn find_smells(design: &Design) -> Vec<Smell> {
    let mut callers: Vec<_> = design.boxes.iter().map(|item| item.id.as_str()).collect();
    callers.sort_unstable();
    let contracts: BTreeSet<&str> = design
        .edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::Implements)
        .map(|edge| edge.to.as_str())
        .collect();
    let mut smells = Vec::new();

    for caller in callers {
        let targets: BTreeSet<&str> = design
            .edges
            .iter()
            .filter(|edge| edge.from == caller && edge.kind == EdgeKind::Calls)
            .map(|edge| edge.to.as_str())
            .collect();
        for contract in &contracts {
            let implementers: Vec<String> = design
                .edges
                .iter()
                .filter(|edge| {
                    edge.kind == EdgeKind::Implements && edge.to == *contract && targets.contains(edge.from.as_str())
                })
                .map(|edge| edge.from.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            if implementers.len() >= 2 {
                smells.push(Smell {
                    caller: caller.to_string(),
                    contract: contract.to_string(),
                    implementers,
                });
            }
        }
    }
    smells
}

pub fn expand_pattern(design: &Design, pattern: &str, target: &str) -> Result<Vec<Command>, String> {
    if pattern != "strategy" {
        return Err(format!("unknown pattern `{pattern}`"));
    }
    let smell = find_smells(design)
        .into_iter()
        .find(|smell| smell.caller == target)
        .ok_or_else(|| format!("no smell found for `{target}`"))?;
    let text = format!(
        "Strategy: `{}` now calls `{}`. Delete its direct edges to {} yourself: there is no remove_edge command yet.",
        smell.caller,
        smell.contract,
        smell.implementers.join(", ")
    );
    Ok(vec![
        Command::AddEdge {
            from: smell.caller,
            to: smell.contract,
            kind: EdgeKind::Calls,
        },
        Command::AskQuestion { text },
    ])
}
