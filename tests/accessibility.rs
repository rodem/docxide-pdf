mod common;

use common::a11y::{self, Analysis};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

struct Scored {
    name: String,
    deficit: Vec<String>,
    struct_score: f64,
    text_score: f64,
}

enum Outcome {
    Scored(Scored),
    UntaggedReference,
    Error(String, String),
}

/// Which PDF in tests/output/<group>/<case>/ to score; e.g. `libreoffice.pdf`
/// scores LibreOffice's export with the same yardstick.
fn gen_pdf_name() -> String {
    std::env::var("DOCXSIDE_A11Y_GEN").unwrap_or_else(|_| "generated.pdf".into())
}

fn write_detail(path: &Path, reference: &Analysis, generated: &Analysis, s: &Scored) {
    let rules: Vec<_> = s
        .deficit
        .iter()
        .map(|id| {
            let counts = |a: &Analysis| a.rules.get(id).map(|r| [r.failed, r.passed]);
            serde_json::json!({
                "rule": id,
                "description": generated.rules[id].description,
                "reference_failed_passed": counts(reference),
                "generated_failed_passed": counts(generated),
            })
        })
        .collect();
    let detail = serde_json::json!({
        "ua_deficit": rules,
        "a11y_struct": s.struct_score,
        "a11y_text": s.text_score,
    });
    fs::write(path, serde_json::to_string_pretty(&detail).unwrap()).ok();
}

fn analyze_fixture(fixture: &Path, gen_name: &str) -> Option<Outcome> {
    let reference = fixture.join("reference.pdf");
    if !fixture.join("input.docx").exists() || !reference.exists() {
        return None;
    }
    let name = common::display_name(fixture);
    let out = common::output_dir(fixture);
    fs::create_dir_all(&out).ok();
    let fail = |e: String| Some(Outcome::Error(name.clone(), e));

    let ref_a = match a11y::analyze_cached(&reference, &out.join("reference.a11y.json")) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };
    // Untagged (macOS print-path) references give no bar to measure against.
    if ref_a.elems.is_empty() {
        return Some(Outcome::UntaggedReference);
    }

    let gen_pdf = if gen_name == "generated.pdf" {
        match common::ensure_generated_pdf(fixture) {
            Ok(p) => p,
            Err(e) => return fail(e),
        }
    } else {
        out.join(gen_name)
    };
    if !gen_pdf.exists() {
        return None;
    }
    let stem = gen_pdf.file_stem().unwrap().to_string_lossy().to_string();
    let gen_a = match a11y::analyze_cached(&gen_pdf, &out.join(format!("{stem}.a11y.json"))) {
        Ok(a) => a,
        Err(e) => return fail(e),
    };

    let scored = Scored {
        name,
        deficit: a11y::ua_deficit(&ref_a.rules, &gen_a.rules)
            .into_iter()
            .map(String::from)
            .collect(),
        struct_score: a11y::struct_score(&ref_a.elems, &gen_a.elems),
        text_score: a11y::text_score(&ref_a.elems, &gen_a.elems),
    };
    write_detail(&out.join(format!("{stem}.deficit.json")), &ref_a, &gen_a, &scored);
    Some(Outcome::Scored(scored))
}

#[test]
fn accessibility_vs_reference() {
    if !a11y::tools_available() {
        println!("SKIP accessibility: needs verapdf and pdfinfo (brew install verapdf poppler)");
        return;
    }
    let gen_name = gen_pdf_name();
    let fixtures = common::discover_fixtures().expect("Failed to read tests/fixtures");
    let outcomes: Vec<Outcome> = fixtures
        .par_iter()
        .filter_map(|f| analyze_fixture(f, &gen_name))
        .collect();

    let mut results = Vec::new();
    let mut errors = Vec::new();
    let mut untagged = 0;
    for o in outcomes {
        match o {
            Outcome::Scored(s) => results.push(s),
            Outcome::UntaggedReference => untagged += 1,
            Outcome::Error(name, e) => errors.push(format!("{name}: {e}")),
        }
    }
    results.sort_by(|a, b| a.name.cmp(&b.name));

    let name_w = results.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    println!("\n  {:<name_w$}  UaDef  Struct    Text  Deficit rules ({gen_name})", "Case");
    for r in &results {
        let shown: Vec<&str> = r.deficit.iter().take(6).map(String::as_str).collect();
        let more = r.deficit.len().saturating_sub(shown.len());
        println!(
            "  {:<name_w$}  {:>5}  {:>5.1}%  {:>5.1}%  {}{}",
            r.name,
            r.deficit.len(),
            r.struct_score * 100.0,
            r.text_score * 100.0,
            shown.join(" "),
            if more > 0 { format!(" +{more}") } else { String::new() }
        );
    }
    let n = results.len().max(1) as f64;
    println!(
        "\n  a11y ≥ Word (no UA-1 deficit): {}/{} · struct {:.1}% · text {:.1}% (means) · N/A untagged reference: {untagged}",
        results.iter().filter(|r| r.deficit.is_empty()).count(),
        results.len(),
        results.iter().map(|r| r.struct_score).sum::<f64>() / n * 100.0,
        results.iter().map(|r| r.text_score).sum::<f64>() / n * 100.0,
    );
    for e in &errors {
        println!("  ERROR {e}");
    }
    assert!(errors.is_empty(), "{} fixtures could not be analysed", errors.len());

    // Other PDFs are scored for comparison only; baselines track our own output.
    if gen_name != "generated.pdf" {
        return;
    }
    let updates: HashMap<String, common::Baselines> = results
        .iter()
        .map(|r| {
            let b = common::Baselines {
                ua_deficit: Some(r.deficit.len()),
                a11y_struct: Some(r.struct_score),
                a11y_text: Some(r.text_score),
                ..Default::default()
            };
            (r.name.clone(), b)
        })
        .collect();
    common::write_latest_scores(&updates);

    let baselines = common::read_baselines();
    let regressions: Vec<&str> = results
        .iter()
        .filter(|r| {
            baselines.get(&r.name).is_some_and(|b| {
                b.ua_deficit.is_some_and(|d| r.deficit.len() > d)
                    || b.a11y_struct.is_some_and(|v| r.struct_score < v - common::REGRESSION_SLACK)
                    || b.a11y_text.is_some_and(|v| r.text_score < v - common::REGRESSION_SLACK)
            })
        })
        .map(|r| r.name.as_str())
        .collect();
    assert!(regressions.is_empty(), "a11y regression in: {}", regressions.join(", "));
}

const LIST_DUMP: &str = r#"Document
  H1 (block)
    Span (inline)
      "Test"
      " "
  L (block):
     /ListNumbering /Disc
    LI (block)
      Lbl (block)
        "•"
      LBody (block)
        Span (inline)
          " "
          "This"
          " "
  P (block)
    Link (inline)
      Object 11 0
      Span (inline)
        "1"
      Note <Note 1> (inline)
        P (block)
          "A note."
    "after"
  Figure ["A close up of a sign??Description automatically generated "]:
     /Placement /Block
    ""
  Figure [""]:
    ""
"#;

#[test]
fn parses_struct_dump() {
    let elems = a11y::parse_struct_text(LIST_DUMP);
    let kinds: Vec<&str> = elems.iter().filter(|e| e.kind != a11y::TEXT).map(|e| e.kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["Document", "H1", "Span", "L", "LI", "Lbl", "LBody", "Span", "P", "Link", "Span", "Note", "P", "Figure", "Figure"]
    );
    let figures: Vec<bool> = elems.iter().filter(|e| e.kind == "Figure").map(|e| e.alt).collect();
    assert_eq!(figures, [true, false]);
    assert_eq!(
        a11y::block_texts(&elems),
        ["Test", "•", "This", "1after", "A note."]
    );
}

#[test]
fn identical_trees_score_perfectly() {
    let elems = a11y::parse_struct_text(LIST_DUMP);
    assert_eq!(a11y::struct_score(&elems, &elems), 1.0);
    assert_eq!(a11y::text_score(&elems, &elems), 1.0);
    assert_eq!(a11y::struct_score(&elems, &[]), 0.0);
    assert_eq!(a11y::text_score(&elems, &[]), 0.0);
}

#[test]
fn text_score_penalises_merged_words_and_reordering() {
    let doc = |texts: &[&str]| {
        let mut dump = String::from("Document\n");
        for t in texts {
            dump += &format!("  P (block)\n    \"{t}\"\n");
        }
        a11y::parse_struct_text(&dump)
    };
    let reference = doc(&["Hello world", "Second"]);
    assert_eq!(a11y::text_score(&reference, &doc(&["Helloworld", "Second"])), 6.0 / 17.0);
    assert_eq!(a11y::text_score(&reference, &doc(&["Second", "Hello world"])), 11.0 / 17.0);
}

#[test]
fn deficit_counts_only_rules_worse_than_word() {
    let json = |rules: &[(&str, u32, u64, u64)]| {
        let summaries: Vec<_> = rules
            .iter()
            .map(|(clause, test, failed, passed)| {
                serde_json::json!({"clause": clause, "testNumber": test, "failedChecks": failed, "passedChecks": passed, "description": "d"})
            })
            .collect();
        let report = serde_json::json!({"report": {"jobs": [{"validationResult": [{"details": {"ruleSummaries": summaries}}]}]}});
        a11y::parse_verapdf_json(&report.to_string()).unwrap()
    };
    // Word fails 5-1 (no pdfuaid) and one stray 7.1-3 check.
    let word = json(&[("5", 1, 1, 0), ("7.1", 3, 1, 999), ("6.2", 1, 0, 1)]);
    assert!(a11y::ua_deficit(&word, &word).is_empty());
    let ours = json(&[("5", 1, 1, 0), ("7.1", 3, 50, 50), ("6.2", 1, 1, 0)]);
    assert_eq!(a11y::ua_deficit(&word, &ours), ["6.2-1", "7.1-3"]);
}
