//! Drift: code that diverges from the design (M6).

use crate::model::Design;
use crate::scaffold::CodeGraph;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Drift {
    /// The design has the box, the code has no file for it.
    BoxWithoutCode { id: String },
    /// The code has a `@box` file the design does not know.
    CodeWithoutBox { id: String },
    /// Code imports from one box to another, but no `calls`/`uses` edge allows it.
    UndeclaredConnection { from: String, to: String },
}

// Compare design and code. This function is pure. Its output order is deterministic:
//   1. BoxWithoutCode for design boxes that SHOULD have code (`kind.has_code()`)
//      and are missing from `code.boxes`, in design order.
//   2. CodeWithoutBox for ids in `code.boxes` that are not in the design, sorted by id.
//   3. UndeclaredConnection for every (from, to) in `code.imports` where both boxes exist in the
//      design, from != to, and the design has NO edge from -> to with `kind.is_dependency()`. Sorted.
// Imports that mention an unknown box are ignored here (rule 2 already reports the box).
pub fn check_drift(design: &Design, code: &CodeGraph) -> Vec<Drift> {
    let mut drift = Vec::new();
    for design_box in &design.boxes {
        if design_box.kind.has_code() && !code.boxes.contains(&design_box.id) {
            drift.push(Drift::BoxWithoutCode {
                id: design_box.id.clone(),
            });
        }
    }

    let mut code_without_box: Vec<String> = code
        .boxes
        .iter()
        .filter(|id| design.find(id).is_none())
        .cloned()
        .collect();
    code_without_box.sort();
    code_without_box.dedup();
    for id in code_without_box {
        drift.push(Drift::CodeWithoutBox { id });
    }

    let mut undeclared_connections = Vec::new();
    for (from, to) in &code.imports {
        if from == to || design.find(from).is_none() || design.find(to).is_none() {
            continue;
        }
        let is_declared = design
            .edges
            .iter()
            .any(|edge| edge.from == from.as_str() && edge.to == to.as_str() && edge.kind.is_dependency());
        if !is_declared {
            undeclared_connections.push((from.clone(), to.clone()));
        }
    }
    undeclared_connections.sort();
    undeclared_connections.dedup();
    for (from, to) in undeclared_connections {
        drift.push(Drift::UndeclaredConnection { from, to });
    }
    drift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop() -> Design {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../examples/shop/design.toml"
        ));
        Design::from_toml(text).unwrap()
    }

    fn code(boxes: &[&str], imports: &[(&str, &str)]) -> CodeGraph {
        CodeGraph {
            boxes: boxes.iter().map(|s| s.to_string()).collect(),
            imports: imports.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        }
    }

    const GOOD_IMPORTS: [(&str, &str); 2] = [("web", "checkout"), ("checkout", "orders_db")];

    #[test]
    fn drift_clean_code_has_none() {
        assert_eq!(
            check_drift(&shop(), &code(&["web", "checkout", "orders_db"], &GOOD_IMPORTS)),
            vec![]
        );
        // ...and an empty codebase is NOT clean (guards against a stub that always returns nothing).
        assert_eq!(check_drift(&shop(), &code(&[], &[])).len(), 3);
    }

    #[test]
    fn drift_box_without_code() {
        let got = check_drift(&shop(), &code(&["web", "checkout"], &[("web", "checkout")]));
        assert_eq!(got, [Drift::BoxWithoutCode { id: "orders_db".into() }]);
    }

    #[test]
    fn drift_code_without_box() {
        let got = check_drift(
            &shop(),
            &code(&["web", "checkout", "orders_db", "rogue"], &GOOD_IMPORTS),
        );
        assert_eq!(got, [Drift::CodeWithoutBox { id: "rogue".into() }]);
    }

    #[test]
    fn drift_undeclared_connection_ui_to_db() {
        let got = check_drift(
            &shop(),
            &code(
                &["web", "checkout", "orders_db"],
                &[("web", "checkout"), ("checkout", "orders_db"), ("web", "orders_db")],
            ),
        );
        assert_eq!(
            got,
            [Drift::UndeclaredConnection {
                from: "web".into(),
                to: "orders_db".into()
            }]
        );
    }
}
