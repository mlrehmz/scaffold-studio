//! Specs for the TypeScript plugin (T09-T14). The golden directory is the answer key:
//! `tests/golden/shop_ts/`. Run one at a time: `cargo test -p studio-core --test scaffold_golden plan -- --ignored`

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use studio_core::model::{Design, DesignBox, Kind};
use studio_core::scaffold::{LanguagePlugin, PlannedFile, ScaffoldPlan, TypeScriptPlugin};

const SHOP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../examples/shop/design.toml"
));

fn shop() -> Design {
    Design::from_toml(SHOP).unwrap()
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/shop_ts")
}

/// relative path -> content, for every file under `dir`.
fn read_tree(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            read_tree(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.insert(rel, fs::read_to_string(&path).unwrap());
        }
    }
}

fn golden() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    read_tree(&golden_dir(), &golden_dir(), &mut out);
    out
}

#[test]
fn plan_has_one_sorted_file_per_box() {
    let plan = TypeScriptPlugin.plan(&shop()).unwrap();
    let paths: Vec<&str> = plan.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["src/checkout.ts", "src/orders_db.ts", "src/web.ts"]);
    let web = &plan.files[2];
    assert_eq!(
        (web.box_id.as_str(), web.imports.clone()),
        ("web", vec!["checkout".to_string()])
    );
    let checkout = &plan.files[0];
    assert_eq!(checkout.imports, ["orders_db"]);
    assert_eq!(checkout.members, ["placeOrder(cart): Order"]);
    assert_eq!(checkout.responsibility, "Turns a cart into an order");
}

#[test]
fn plan_skips_external_and_system_boxes() {
    let mut d = shop();
    for (id, kind) in [("stripe", Kind::External), ("everything", Kind::System)] {
        d.boxes.push(DesignBox {
            id: id.into(),
            kind,
            parent: None,
            layer: None,
            responsibility: String::new(),
            members: vec![],
            locked: false,
        });
    }
    assert_eq!(TypeScriptPlugin.plan(&d).unwrap().files.len(), 3);
}

#[test]
#[ignore]
fn materialized_shop_matches_golden() {
    let plugin = TypeScriptPlugin;
    let files = plugin.materialize(&plugin.plan(&shop()).unwrap()).unwrap();
    let got: BTreeMap<String, String> = files.into_iter().map(|f| (f.path, f.content)).collect();
    assert_eq!(got, golden());
}

#[test]
#[ignore]
fn materialize_is_deterministic() {
    let plugin = TypeScriptPlugin;
    let plan = plugin.plan(&shop()).unwrap();
    assert_eq!(plugin.materialize(&plan).unwrap(), plugin.materialize(&plan).unwrap());
}

#[test]
#[ignore]
fn generated_shop_compiles() {
    let result = TypeScriptPlugin.check(&golden_dir()).unwrap();
    assert!(result.ok, "tsc said: {}", result.output);
}

#[test]
#[ignore]
fn type_errors_are_reported() {
    let dir = std::env::temp_dir().join(format!("studio-bad-ts-{}", std::process::id()));
    let mut files = BTreeMap::new();
    read_tree(&golden_dir(), &golden_dir(), &mut files);
    files.insert("src/web.ts".into(), "const x: number = \"nope\";\nexport {};\n".into());
    for (rel, content) in files {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    assert!(!TypeScriptPlugin.check(&dir).unwrap().ok);
}

fn file(path: &str, box_id: &str) -> PlannedFile {
    PlannedFile {
        path: path.into(),
        box_id: box_id.into(),
        responsibility: String::new(),
        imports: vec![],
        members: vec![],
    }
}

#[test]
#[ignore]
fn plan_validator_accepts_a_good_plan_and_catches_missing_and_invented_files() {
    use studio_core::scaffold::validate_plan;
    let d = shop();
    let good = TypeScriptPlugin.plan(&d).unwrap();
    assert!(validate_plan(&d, &good).is_empty());

    let mut missing = good.clone();
    missing.files.retain(|f| f.box_id != "web");
    assert!(validate_plan(&d, &missing)
        .iter()
        .any(|p| p.contains("web") && p.contains("no file")));

    let mut invented = good.clone();
    invented.files.push(file("src/ghost.ts", "ghost"));
    assert!(validate_plan(&d, &invented).iter().any(|p| p.contains("ghost")));

    let dup = ScaffoldPlan {
        files: vec![file("src/a.ts", "web"), file("src/a.ts", "checkout")],
    };
    assert!(validate_plan(&d, &dup)
        .iter()
        .any(|p| p.contains("same path") || p.contains("duplicate")));
}

#[test]
#[ignore]
fn read_back_finds_boxes_and_imports() {
    let graph = TypeScriptPlugin.read_back(&golden_dir()).unwrap();
    assert_eq!(graph.boxes, ["checkout", "orders_db", "web"]);
    assert_eq!(
        graph.imports,
        [
            ("checkout".to_string(), "orders_db".to_string()),
            ("web".to_string(), "checkout".to_string())
        ]
    );
}
