//! studio-core: the single gatekeeper.
//!
//! Every proposal (user, local model, frontier model, sketch) is checked here
//! and nowhere else. Done: model, validate, command, rules, evaluate, tokens.
//! Stubs for later milestones: process, notes, scaffold, drift, replay,

pub mod adapter;
pub mod command;
pub mod drift;
pub mod evaluate;
pub mod model;
pub mod notes;
pub mod patterns;
pub mod process;
pub mod replay;
pub mod rules;
pub mod scaffold;
pub mod sketch;
pub mod tokens;
pub mod validate;

/// Message returned by every stub, so a missing feature is loud, never silent.
pub fn not_implemented(what: &str) -> String {
    format!("not implemented yet: {what}")
}
