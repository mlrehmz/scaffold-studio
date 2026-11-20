//! Replay: compare the planned path with the path an agent actually took (M7).
//!
//! We define our own neutral `trace.jsonl` and adapt real agent transcripts later
//! (check each agent's CURRENT transcript docs first). One JSON object per line:
//!   {"type":"read","path":"steps/02-checkout.md"}    {"type":"spawn_agent"}

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceEvent {
    Read { path: String },
    SpawnAgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReplayReport {
    pub followed: Vec<String>,  // green: planned and read
    pub skipped: Vec<String>,   // grey: planned, never read
    pub unplanned: Vec<String>, // orange: read, not planned
    pub unplanned_helpers: usize,
    pub tokens_loaded: usize,
    /// followed / planned (1.0 when nothing was planned).
    pub adherence: f32,
}

pub fn parse_trace(jsonl: &str) -> Result<Vec<TraceEvent>, String> {
    let mut events = Vec::new();
    for (index, line) in jsonl.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event = serde_json::from_str(line).map_err(|error| format!("trace line {}: {error}", index + 1))?;
        events.push(event);
    }
    Ok(events)
}

pub fn replay(planned: &[String], trace: &[TraceEvent], tokens: &BTreeMap<String, usize>) -> ReplayReport {
    let mut seen = BTreeSet::new();
    let mut reads = Vec::new();
    let mut unplanned_helpers = 0;
    for event in trace {
        match event {
            TraceEvent::Read { path } if seen.insert(path.clone()) => reads.push(path.clone()),
            TraceEvent::Read { .. } => {}
            TraceEvent::SpawnAgent => unplanned_helpers += 1,
        }
    }

    let mut followed = Vec::new();
    let mut skipped = Vec::new();
    for path in planned {
        if seen.contains(path) {
            followed.push(path.clone());
        } else {
            skipped.push(path.clone());
        }
    }
    let unplanned = reads
        .iter()
        .filter(|path| path.as_str() != "INDEX.md" && !planned.contains(path))
        .cloned()
        .collect();
    let tokens_loaded = reads.iter().map(|path| tokens.get(path).copied().unwrap_or(0)).sum();
    let adherence = if planned.is_empty() {
        1.0
    } else {
        followed.len() as f32 / planned.len() as f32
    };
    ReplayReport {
        followed,
        skipped,
        unplanned,
        unplanned_helpers,
        tokens_loaded,
        adherence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(p: &str) -> TraceEvent {
        TraceEvent::Read { path: p.into() }
    }

    #[test]
    fn parse_trace_reads_lines_and_reports_the_bad_line() {
        let ok = "{\"type\":\"read\",\"path\":\"INDEX.md\"}\n\n{\"type\":\"spawn_agent\"}\n";
        assert_eq!(parse_trace(ok).unwrap(), [read("INDEX.md"), TraceEvent::SpawnAgent]);
        let err = parse_trace("{\"type\":\"read\",\"path\":\"a\"}\nnot json\n").unwrap_err();
        assert!(err.starts_with("trace line 2"), "{err}");
    }

    #[test]
    fn replay_paints_all_three_colors() {
        let planned: Vec<String> = ["steps/01-orders_db.md", "steps/02-checkout.md", "steps/03-web.md"]
            .map(String::from)
            .to_vec();
        let trace = vec![
            read("INDEX.md"),
            read("steps/01-orders_db.md"),
            read("steps/03-web.md"),
            read("steps/99-extra.md"),
            read("steps/01-orders_db.md"), // read twice: counts once
            TraceEvent::SpawnAgent,
        ];
        let tokens: BTreeMap<String, usize> = [
            ("INDEX.md", 50),
            ("steps/01-orders_db.md", 100),
            ("steps/03-web.md", 70),
        ]
        .map(|(k, v)| (k.to_string(), v))
        .into();
        let r = replay(&planned, &trace, &tokens);
        assert_eq!(r.followed, ["steps/01-orders_db.md", "steps/03-web.md"]);
        assert_eq!(r.skipped, ["steps/02-checkout.md"]);
        assert_eq!(r.unplanned, ["steps/99-extra.md"]);
        assert_eq!(r.unplanned_helpers, 1);
        assert_eq!(r.tokens_loaded, 220); // 50 + 100 + 70, the unknown extra note counts 0
        assert!((r.adherence - 2.0 / 3.0).abs() < 1e-6);
    }
}
