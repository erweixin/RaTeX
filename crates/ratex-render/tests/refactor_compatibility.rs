//! Same-machine compatibility capture for structural refactors. Artifacts are
//! deliberately opt-in and never replace the committed golden references.
use std::fs;
use std::path::{Path, PathBuf};

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::{Color, MathStyle};
use serde_json::{json, Value};

fn capture_suite(root: &Path, output: &Path, name: &str, corpus: &str, font_size: f32, dpr: f32) {
    let directory = output.join(name);
    fs::create_dir(&directory).expect("create new suite directory");
    let source = fs::read_to_string(root.join("tests/golden").join(corpus)).unwrap();
    let options = RenderOptions {
        font_dir: root.join("fonts").to_string_lossy().into_owned(),
        font_size,
        device_pixel_ratio: dpr,
        ..RenderOptions::default()
    };
    let mut records = Vec::new();
    let mut errors = String::new();
    for (index, formula) in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(['#', '%']))
        .enumerate()
    {
        let index = index + 1;
        let mut record = json!({"index": index, "formula": formula});
        match parse(formula) {
            Err(error) => {
                record["status"] = json!("parse_error");
                record["error"] = json!(error.to_string());
                errors.push_str(&format!("ERR {index:4} {formula} — parse error: {error}\n"));
            }
            Ok(ast) => {
                record["ast"] = serde_json::to_value(&ast).unwrap();
                let box_ = layout(&ast, &LayoutOptions::default());
                let display = to_display_list(&box_);
                record["box"] = json!([box_.width, box_.height, box_.depth]);
                record["display"] = serde_json::to_value(&display).unwrap();
                // Exercise style/color/size propagation as well as the default
                // path, without multiplying the number of PNG artifacts.
                let inline_options = LayoutOptions::default()
                    .with_style(MathStyle::Text)
                    .with_color(Color::new(0.2, 0.4, 0.6, 0.75))
                    .with_inter_glyph_kern(0.02);
                record["inline_display"] =
                    serde_json::to_value(to_display_list(&layout(&ast, &inline_options))).unwrap();
                match render_to_png(&display, &options) {
                    Ok(png) => {
                        fs::write(directory.join(format!("{index:04}.png")), png).unwrap();
                        record["status"] = json!("rendered");
                    }
                    Err(error) => {
                        record["status"] = json!("render_error");
                        record["error"] = json!(error);
                        errors.push_str(&format!("ERR {index:4} {formula} — {error}\n"));
                    }
                }
            }
        }
        records.push(record);
    }
    fs::write(directory.join("errors.log"), errors).unwrap();
    fs::write(
        directory.join("records.json"),
        serde_json::to_vec(&records).unwrap(),
    )
    .unwrap();
    println!("captured {}: {} formulas", name, records.len());
}

fn compare_suite(baseline: &Path, output: &Path, name: &str) {
    let before = baseline.join(name);
    let after = output.join(name);
    let old: Vec<Value> =
        serde_json::from_slice(&fs::read(before.join("records.json")).unwrap()).unwrap();
    let new: Vec<Value> =
        serde_json::from_slice(&fs::read(after.join("records.json")).unwrap()).unwrap();
    assert_eq!(old.len(), new.len(), "{name}: corpus size changed");
    for (old, new) in old.iter().zip(&new) {
        let index = new["index"].as_u64().unwrap();
        assert_eq!(
            old, new,
            "{name}/{index:04}: AST, geometry, display items or status changed"
        );
        if new["status"] == "rendered" {
            let png = format!("{index:04}.png");
            // The same encoder/options make byte equality a stricter check
            // than decoded dimensions and pixel equality.
            assert_eq!(
                fs::read(before.join(&png)).unwrap(),
                fs::read(after.join(&png)).unwrap(),
                "{name}/{png}: PNG changed"
            );
        }
    }
}

#[test]
#[ignore = "set RATEX_COMPAT_OUTPUT; optionally RATEX_COMPAT_BASELINE to compare two captures"]
fn capture_and_compare_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let output =
        PathBuf::from(std::env::var_os("RATEX_COMPAT_OUTPUT").expect("set RATEX_COMPAT_OUTPUT"));
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::create_dir(&output).expect("output must be a new directory; never overwrite a baseline");
    for (name, corpus, size, dpr) in [
        ("main", "test_cases.txt", 40.0, 1.0),
        ("mhchem", "test_case_ce.txt", 40.0, 2.0),
        ("prooftree", "test_cases_prooftree.txt", 36.0, 1.0),
    ] {
        capture_suite(&root, &output, name, corpus, size, dpr);
        if let Some(baseline) = std::env::var_os("RATEX_COMPAT_BASELINE") {
            compare_suite(Path::new(&baseline), &output, name);
        }
    }
}
