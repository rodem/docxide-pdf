//! Accessibility metrics, always relative to the Word reference: veraPDF
//! PDF/UA-1 rule parity plus a comparison of the structure trees as Poppler
//! reads them (`pdfinfo -struct-text`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pseudo element kind for a content item's text, kept in the element list so
/// text stays interleaved with inline children in reading order.
pub const TEXT: &str = "#text";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Elem {
    pub depth: usize,
    pub kind: String,
    pub inline: bool,
    pub alt: bool,
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Rule {
    pub failed: u64,
    pub passed: u64,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Analysis {
    /// Structure tree in pre-order; empty for an untagged PDF.
    pub elems: Vec<Elem>,
    /// PDF/UA-1 rule id ("7.1-3") → check counts.
    pub rules: BTreeMap<String, Rule>,
}

pub fn tools_available() -> bool {
    Command::new("pdfinfo").arg("-v").output().is_ok()
        && Command::new("verapdf").arg("--version").output().is_ok()
}

/// How a PDF fares against Word's reference: what tests/accessibility.rs
/// scores and the engine comparison shows.
pub struct Scores {
    /// PDF/UA-1 rules the PDF fails on its own.
    pub ua_fail: usize,
    /// The PDF claims PDF/UA-1 (rule 5-1 passes).
    pub claims_ua: bool,
    /// None for an untagged (macOS print-path) reference: no Word bar to measure against.
    pub vs_word: Option<VsWord>,
}

pub struct VsWord {
    pub deficit: Vec<String>,
    pub struct_score: f64,
    pub text_score: f64,
}

pub fn scores(reference: &Analysis, generated: &Analysis) -> Scores {
    Scores {
        ua_fail: generated.rules.values().filter(|r| r.failed > 0).count(),
        claims_ua: !generated.rules.get("5-1").is_some_and(|r| r.failed > 0),
        vs_word: (!reference.elems.is_empty()).then(|| VsWord {
            deficit: ua_deficit(&reference.rules, &generated.rules)
                .into_iter()
                .map(String::from)
                .collect(),
            struct_score: struct_score(&reference.elems, &generated.elems),
            text_score: text_score(&reference.elems, &generated.elems),
        }),
    }
}

/// The `analyze_cached` cache for `<dir>/<name>.pdf`, given `<dir>/<name>`.
pub fn cache_path(stem: &Path) -> PathBuf {
    stem.with_extension("a11y.json")
}

/// Analyse `pdf`, reusing `cache` while it is newer than the PDF.
// ponytail: cache ignores the veraPDF/Poppler version; delete tests/output/**/*.a11y.json after upgrading them
pub fn analyze_cached(pdf: &Path, cache: &Path) -> Result<Analysis, String> {
    let mtime = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    if matches!((mtime(cache), mtime(pdf)), (Some(c), Some(p)) if c >= p)
        && let Some(a) = fs::read_to_string(cache)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    {
        return Ok(a);
    }
    // veraPDF first, so a machine without it fails before the pdfinfo work.
    let rules = ua_rules(pdf)?;
    let analysis = Analysis {
        elems: struct_elems(pdf)?,
        rules,
    };
    if let Ok(json) = serde_json::to_string(&analysis) {
        fs::write(cache, json).ok();
    }
    Ok(analysis)
}

fn struct_elems(pdf: &Path) -> Result<Vec<Elem>, String> {
    let out = Command::new("pdfinfo")
        .arg("-struct-text")
        .arg(pdf)
        .output()
        .map_err(|e| format!("pdfinfo: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "pdfinfo: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(parse_struct_text(&String::from_utf8_lossy(&out.stdout)))
}

/// Parse `pdfinfo -struct-text`: two spaces of indent per level, element lines
/// like `Note <Note 1> (inline)` or `Figure ["alt"]:`, attribute lines starting
/// with `/`, annotation references `Object 11 0`, and quoted text lines.
pub fn parse_struct_text(dump: &str) -> Vec<Elem> {
    let mut elems = Vec::new();
    for line in dump.lines() {
        let body = line.trim_start();
        let depth = (line.len() - body.len()) / 2;
        if body.is_empty() || body.starts_with('/') || body.starts_with("Object ") {
            continue;
        }
        if let Some(quoted) = body.strip_prefix('"') {
            elems.push(Elem {
                depth,
                kind: TEXT.into(),
                inline: true,
                alt: false,
                text: quoted.strip_suffix('"').unwrap_or(quoted).to_string(),
            });
            continue;
        }
        let kind: String = body
            .chars()
            .take_while(|c| !matches!(c, ' ' | ':' | '[' | '<' | '('))
            .collect();
        let alt = match (body.find("[\""), body.rfind("\"]")) {
            (Some(a), Some(b)) if b > a + 1 => !body[a + 2..b].trim().is_empty(),
            _ => false,
        };
        elems.push(Elem {
            depth,
            kind,
            inline: body.contains("(inline)"),
            alt,
            text: String::new(),
        });
    }
    elems
}

fn ua_rules(pdf: &Path) -> Result<BTreeMap<String, Rule>, String> {
    // Forced to ua1: Word writes no pdfuaid, so auto-detection would validate
    // PDF/A-1b. --success lists passing rules too, with per-rule check counts.
    let out = Command::new("verapdf")
        .args([
            "-f",
            "ua1",
            "--format",
            "json",
            "--success",
            "--maxfailuresdisplayed",
            "0",
        ])
        .arg(pdf)
        .output()
        .map_err(|e| format!("verapdf: {e}"))?;
    // Exit code 1 only means "not compliant".
    if !matches!(out.status.code(), Some(0 | 1)) {
        return Err(format!("verapdf exited with {:?}", out.status.code()));
    }
    parse_verapdf_json(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_verapdf_json(json: &str) -> Result<BTreeMap<String, Rule>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("verapdf json: {e}"))?;
    let summaries = v["report"]["jobs"][0]["validationResult"][0]["details"]["ruleSummaries"]
        .as_array()
        .ok_or("verapdf json: no ruleSummaries")?;
    Ok(summaries
        .iter()
        .map(|r| {
            let id = format!(
                "{}-{}",
                r["clause"].as_str().unwrap_or("?"),
                r["testNumber"]
            );
            let rule = Rule {
                failed: r["failedChecks"].as_u64().unwrap_or(0),
                passed: r["passedChecks"].as_u64().unwrap_or(0),
                description: r["description"].as_str().unwrap_or("").to_string(),
            };
            (id, rule)
        })
        .collect())
}

/// Rules where we do worse than Word: failed where Word passes, or failing a
/// larger share of the rule's checks than Word does. The share test matters for
/// rules like 7.1-3 (untagged content) that Word fails on one stray item while
/// an untagged PDF fails them everywhere.
pub fn ua_deficit<'a>(
    reference: &BTreeMap<String, Rule>,
    generated: &'a BTreeMap<String, Rule>,
) -> Vec<&'a str> {
    let share = |r: &Rule| r.failed as f64 / (r.failed + r.passed).max(1) as f64;
    generated
        .iter()
        .filter(|(id, g)| {
            g.failed > 0
                && reference
                    .get(*id)
                    .is_none_or(|r| r.failed == 0 || share(g) > share(r) + 0.01)
        })
        .map(|(id, _)| id.as_str())
        .collect()
}

/// Element types in reading order, Span dropped (Word splits them per
/// formatting run) and a Figure's alt text presence folded into its token.
fn struct_tokens(elems: &[Elem]) -> Vec<&str> {
    elems
        .iter()
        .filter(|e| e.kind != TEXT && e.kind != "Span")
        .map(|e| {
            if e.kind == "Figure" && e.alt {
                "Figure+alt"
            } else {
                e.kind.as_str()
            }
        })
        .collect()
}

fn levenshtein<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for x in a {
        let mut cur = vec![prev[0] + 1];
        for (j, y) in b.iter().enumerate() {
            cur.push(
                (prev[j] + usize::from(x != y))
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

/// 1 − edit distance between the two element-type sequences, relative to the longer.
pub fn struct_score(reference: &[Elem], generated: &[Elem]) -> f64 {
    let (a, b) = (struct_tokens(reference), struct_tokens(generated));
    let n = a.len().max(b.len());
    if n == 0 {
        return 1.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / n as f64
}

/// Symbol glyphs (checkboxes, arrows, dingbats, symbol-font private-use codes)
/// all count as one character. Word extracts the same Wingdings checkbox as raw
/// U+F0A8 in one document and as Unicode in the next, and which symbol it is
/// isn't what this score measures. Math operators (U+2200–22FF) and • stay
/// distinct.
fn fold_symbol(c: char) -> char {
    match c as u32 {
        0xF000..=0xF0FF // symbol-font private use
        | 0x2190..=0x21FF // arrows
        | 0x2300..=0x23FF // misc technical
        | 0x25A0..=0x27FF // geometric shapes, misc symbols, dingbats, misc math symbols-A, arrows-A
        | 0x2900..=0x297F // arrows-B
        | 0x2B00..=0x2BFF // misc symbols and arrows
        | 0x1F300..=0x1FAFF => '\u{FFFC}', // pictographs, emoji, arrows-C
        _ => c,
    }
}

/// Whitespace- and symbol-normalised (`fold_symbol`) text per block element in
/// reading order; inline content (Span, Link, Note, text) folds into its
/// nearest block ancestor.
pub fn block_texts(elems: &[Elem]) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut open: Vec<(usize, usize)> = Vec::new(); // (depth, block index) of enclosing blocks
    for e in elems {
        while open.last().is_some_and(|&(d, _)| d >= e.depth) {
            open.pop();
        }
        if e.kind == TEXT {
            match open.last() {
                Some(&(_, i)) => blocks[i].push_str(&e.text),
                None => blocks.push(e.text.clone()),
            }
        } else if !e.inline {
            open.push((e.depth, blocks.len()));
            blocks.push(String::new());
        }
    }
    blocks
        .into_iter()
        .map(|b| b.split_whitespace().collect::<Vec<_>>().join(" "))
        .map(|b| b.chars().map(fold_symbol).collect::<String>())
        .filter(|b| !b.is_empty())
        .collect()
}

/// Characters in blocks that match exactly and in the same order (LCS over
/// blocks, weighted by length), relative to the longer text. A reordered,
/// missing, extra or word-merged paragraph costs its whole length.
pub fn text_score(reference: &[Elem], generated: &[Elem]) -> f64 {
    let (a, b) = (block_texts(reference), block_texts(generated));
    let total = |v: &[String]| v.iter().map(|s| s.chars().count()).sum::<usize>();
    let n = total(&a).max(total(&b));
    if n == 0 {
        return 1.0;
    }
    let mut prev = vec![0usize; b.len() + 1];
    for x in &a {
        let w = x.chars().count();
        let mut cur = vec![0usize; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            cur[j + 1] = if x == y {
                prev[j] + w
            } else {
                prev[j + 1].max(cur[j])
            };
        }
        prev = cur;
    }
    prev[b.len()] as f64 / n as f64
}
