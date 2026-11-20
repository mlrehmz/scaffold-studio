//! The edit grammar: the whole vocabulary any AI may speak (M2).
//!
//! `apply` is a pure function: (design, command) -> new design or an error.
//! Purity makes undo trivial and lets us try a proposal on a copy.
//! An unknown `op` fails at deserialization, so a wrong format is impossible.

use crate::model::{Design, DesignBox, Edge, EdgeKind, Kind};
use crate::validate::is_valid_id;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    AddNode {
        id: String,
        kind: Kind,
        #[serde(default)]
        parent: Option<String>,
        #[serde(default)]
        layer: Option<String>,
        #[serde(default)]
        responsibility: String,
    },
    AddMember {
        node: String,
        signature: String,
    },
    AddEdge {
        from: String,
        to: String,
        kind: EdgeKind,
    },
    // Expanded by `patterns::expand_pattern` (T18).
    ApplyPattern {
        pattern: String,
        target: String,
    },
    // HUMAN DECISION H1: SetTrigger changes process.toml, but `apply` only sees a Design.
    SetTrigger {
        step: String,
        trigger: String,
    },
    /// Changes nothing. Carries a question for the human.
    AskQuestion {
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    User,
    Local,
    Frontier,
    Sketch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub id: String,
    pub source: Source,
    pub commands: Vec<Command>,
    #[serde(default)]
    pub rationale: String,
    /// Sketch proposals only: `regions[i]` names the photo region command `i` was read from
    /// (an arrow is `"<from region>><to region>"`). Empty for every other source.
    #[serde(default)]
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Empty(&'static str),
    InvalidId(String),
    DuplicateId(String),
    DuplicateEdge(String),
    Pattern(String),
    UnknownBox(String),
    Locked(String),
    Unsupported(&'static str),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Empty(field) => write!(f, "`{field}` must not be empty"),
            CommandError::InvalidId(id) => write!(f, "`{id}` is not a valid id (letters, digits, `_`, `-`)"),
            CommandError::DuplicateId(id) => write!(f, "box `{id}` already exists"),
            CommandError::DuplicateEdge(e) => write!(f, "edge {e} already exists"),
            CommandError::Pattern(message) => write!(f, "pattern failed: {message}"),
            CommandError::UnknownBox(id) => write!(f, "unknown box `{id}`"),
            CommandError::Locked(id) => write!(f, "box `{id}` is locked"),
            CommandError::Unsupported(what) => write!(f, "{what} is not implemented yet"),
        }
    }
}

/// Look up a box that exists and is not locked.
fn editable<'a>(d: &'a mut Design, id: &str) -> Result<&'a mut DesignBox, CommandError> {
    let b = d
        .boxes
        .iter_mut()
        .find(|b| b.id == id)
        .ok_or_else(|| CommandError::UnknownBox(id.to_string()))?;
    if b.locked {
        return Err(CommandError::Locked(id.to_string()));
    }
    Ok(b)
}

pub fn apply(design: &Design, cmd: &Command) -> Result<Design, CommandError> {
    let mut d = design.clone();
    match cmd {
        Command::AddNode {
            id,
            kind,
            parent,
            layer,
            responsibility,
        } => {
            if !is_valid_id(id) {
                return Err(CommandError::InvalidId(id.clone()));
            }
            if d.find(id).is_some() {
                return Err(CommandError::DuplicateId(id.clone()));
            }
            if let Some(p) = parent {
                editable(&mut d, p)?; // parent must exist and not be locked
            }
            d.boxes.push(DesignBox {
                id: id.clone(),
                kind: *kind,
                parent: parent.clone(),
                layer: layer.clone(),
                responsibility: responsibility.clone(),
                members: vec![],
                locked: false,
            });
        }
        Command::AddMember { node, signature } => {
            if signature.trim().is_empty() {
                return Err(CommandError::Empty("signature"));
            }
            editable(&mut d, node)?.members.push(signature.clone());
        }
        Command::AddEdge { from, to, kind } => {
            editable(&mut d, from)?;
            editable(&mut d, to)?;
            let edge = Edge {
                from: from.clone(),
                to: to.clone(),
                kind: *kind,
            };
            if d.edges.contains(&edge) {
                return Err(CommandError::DuplicateEdge(format!("{from} -> {to}")));
            }
            d.edges.push(edge);
        }
        Command::ApplyPattern { pattern, target } => {
            let commands = crate::patterns::expand_pattern(&d, pattern, target).map_err(CommandError::Pattern)?;
            for command in commands {
                d = apply(&d, &command)?;
            }
        }
        Command::SetTrigger { .. } => return Err(CommandError::Unsupported("set_trigger")),
        Command::AskQuestion { .. } => {}
    }
    Ok(d)
}
