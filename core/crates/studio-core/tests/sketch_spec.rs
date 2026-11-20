//! Spec for T19. The reading is hand-authored: examples/shop/sketch_reading.json.

use studio_core::command::{Command, Source};
use studio_core::evaluate::evaluate;
use studio_core::model::{Design, EdgeKind, Kind};
use studio_core::sketch::{reading_to_proposal, slug, SketchReading};

fn reading() -> SketchReading {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../examples/shop/sketch_reading.json"
    ));
    serde_json::from_str(text).expect("the example reading matches the SketchReading type")
}

#[test]
fn slug_examples() {
    assert_eq!(slug("Orders DB"), "orders_db");
    assert_eq!(slug("  Web!! "), "web");
    assert_eq!(slug("a--b"), "a_b");
}

#[test]
fn sketch_becomes_commands_with_regions() {
    let p = reading_to_proposal(&reading());
    assert_eq!(p.source, Source::Sketch);
    assert_eq!(p.regions, ["r1", "r2", "r3", "r4", "r1>r2", "r2>r3"]);
    assert_eq!(p.commands.len(), 6);
    assert!(matches!(&p.commands[0], Command::AddNode { id, kind: Kind::Component, .. } if id == "web"));
    assert!(matches!(&p.commands[2], Command::AddNode { id, kind: Kind::External, .. } if id == "stripe"));
    assert!(matches!(&p.commands[3], Command::AskQuestion { text } if text.contains("r4")));
    assert_eq!(
        p.commands[4],
        Command::AddEdge {
            from: "web".into(),
            to: "checkout".into(),
            kind: EdgeKind::Calls
        }
    );
    assert_eq!(
        p.commands[5],
        Command::AddEdge {
            from: "checkout".into(),
            to: "stripe".into(),
            kind: EdgeKind::Uses
        }
    );
}

#[test]
fn sketch_proposal_goes_through_the_same_gatekeeper() {
    let empty = Design::from_toml("[project]\nname = \"t\"\ntarget = \"typescript\"\n").unwrap();
    let v = evaluate(&empty, &reading_to_proposal(&reading()));
    assert!(v.ok, "{v:?}");
    assert_eq!(v.preview.unwrap().boxes.len(), 3);
    assert_eq!(v.questions.len(), 1);
}

#[test]
fn an_arrow_to_an_unknown_region_becomes_a_question() {
    let mut r = reading();
    r.arrows[0].to_region = "r99".into();
    let p = reading_to_proposal(&r);
    assert!(matches!(&p.commands[4], Command::AskQuestion { .. }));
    assert_eq!(p.commands.len(), p.regions.len());
}
