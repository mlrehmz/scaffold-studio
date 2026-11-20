//! Specs for T17a, T17b, T18. Fixture: tests/fixtures/strategy_smell.toml.

use studio_core::command::Command;
use studio_core::evaluate::evaluate_json;
use studio_core::model::{Design, Edge, EdgeKind};
use studio_core::patterns::{expand_pattern, find_smells, load_catalog, Smell};

const SMELLY: &str = include_str!("fixtures/strategy_smell.toml");
const CATALOG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../patterns/catalog.toml"));

fn smelly() -> Design {
    Design::from_toml(SMELLY).unwrap()
}

#[test]
fn catalog_loads_and_demands_a_reason() {
    let defs = load_catalog(CATALOG).unwrap();
    assert_eq!(defs[0].id, "strategy");
    let no_reason = "[[pattern]]\nid=\"x\"\nsmell=\"s\"\nreason=\"  \"\nexpand=\"e\"\n";
    assert!(load_catalog(no_reason).unwrap_err().contains("`x`"));
}

#[test]
fn smell_is_found_for_two_implementers() {
    assert_eq!(
        find_smells(&smelly()),
        [Smell {
            caller: "checkout".into(),
            contract: "payment_gateway".into(),
            implementers: vec!["paypal_adapter".into(), "stripe_adapter".into()]
        }]
    );
}

#[test]
fn no_smell_when_the_caller_uses_the_contract() {
    assert_eq!(find_smells(&smelly()).len(), 1, "sanity: the fixture really is smelly");
    let mut d = smelly();
    d.edges.retain(|e| !(e.from == "checkout" && e.kind == EdgeKind::Calls));
    d.edges.push(Edge {
        from: "checkout".into(),
        to: "payment_gateway".into(),
        kind: EdgeKind::Calls,
    });
    assert!(find_smells(&d).is_empty());
}

#[test]
fn no_smell_for_a_single_implementer() {
    assert_eq!(find_smells(&smelly()).len(), 1, "sanity: the fixture really is smelly");
    let mut d = smelly();
    d.edges
        .retain(|e| e.to != "paypal_adapter" || e.kind == EdgeKind::Implements);
    assert!(find_smells(&d).is_empty());
}

#[test]
fn expand_gives_an_edge_and_a_question() {
    let cmds = expand_pattern(&smelly(), "strategy", "checkout").unwrap();
    assert_eq!(cmds.len(), 2);
    assert_eq!(
        cmds[0],
        Command::AddEdge {
            from: "checkout".into(),
            to: "payment_gateway".into(),
            kind: EdgeKind::Calls
        }
    );
    assert!(
        matches!(&cmds[1], Command::AskQuestion { text } if text.contains("paypal_adapter") && text.contains("stripe_adapter"))
    );
    assert!(expand_pattern(&smelly(), "visitor", "checkout")
        .unwrap_err()
        .contains("visitor"));
    assert!(expand_pattern(&smelly(), "strategy", "stripe_adapter").is_err());
}

#[test]
fn apply_pattern_adds_the_contract_edge() {
    let proposal =
        r#"{"id":"p","source":"user","commands":[{"op":"apply_pattern","pattern":"strategy","target":"checkout"}]}"#;
    let v = evaluate_json(&smelly(), proposal);
    assert!(v.ok, "{v:?}");
    assert!(v
        .preview
        .unwrap()
        .edges
        .iter()
        .any(|e| e.from == "checkout" && e.to == "payment_gateway"));
}
