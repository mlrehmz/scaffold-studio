//! Sketch import: contract only (M10).
//!
//! The reading JSON is hand-authored for now (`examples/shop/sketch_reading.json`). Later,
//! OCR + OpenCV fill it and a small vision model handles only ambiguous parts. Whatever produces
//! it, the result becomes a normal Proposal (source = sketch) that goes through the same gatekeeper.

use crate::command::{Command, Proposal, Source};
use crate::model::{EdgeKind, Kind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub bbox: [f32; 4], // x, y, w, h in photo pixels
    pub text: String,
    pub confidence: f32,
    pub kind: String, // "box" | "double_box" | "cloud" | "rounded" ...
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SketchArrow {
    pub from_region: String,
    pub to_region: String,
    pub dashed: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SketchReading {
    pub regions: Vec<Region>,
    pub arrows: Vec<SketchArrow>,
}

/// Box id from a label: lowercase, every run of non-alphanumerics becomes one `_`, trimmed.
/// ASCII only, so the result is always a valid id (see `validate::is_valid_id`). `"Orders DB"` -> `"orders_db"`. (Same rule as `slug` in app/server/src/proposer.ts.)
pub fn slug(label: &str) -> String {
    let mut out = String::new();
    for c in label.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

pub fn reading_to_proposal(reading: &SketchReading) -> Proposal {
    let mut commands = Vec::new();
    let mut regions = Vec::new();
    for region in &reading.regions {
        let command = if region.text.trim().is_empty() {
            Command::AskQuestion {
                text: format!("Region {} is unreadable: what is it?", region.id),
            }
        } else {
            Command::AddNode {
                id: slug(&region.text),
                kind: if region.kind == "cloud" {
                    Kind::External
                } else {
                    Kind::Component
                },
                parent: None,
                layer: None,
                responsibility: String::new(),
            }
        };
        commands.push(command);
        regions.push(region.id.clone());
    }

    for arrow in &reading.arrows {
        let from = reading.regions.iter().find(|region| region.id == arrow.from_region);
        let to = reading.regions.iter().find(|region| region.id == arrow.to_region);
        let command = match (from, to) {
            (Some(from), Some(to)) if !from.text.trim().is_empty() && !to.text.trim().is_empty() => Command::AddEdge {
                from: slug(&from.text),
                to: slug(&to.text),
                kind: if arrow.dashed { EdgeKind::Uses } else { EdgeKind::Calls },
            },
            _ => Command::AskQuestion {
                text: format!(
                    "Arrow {}>{} points at something I could not read.",
                    arrow.from_region, arrow.to_region
                ),
            },
        };
        commands.push(command);
        regions.push(format!("{}>{}", arrow.from_region, arrow.to_region));
    }
    Proposal {
        id: "sketch-1".into(),
        source: Source::Sketch,
        commands,
        rationale: "Draft read from a sketch.".into(),
        regions,
    }
}
