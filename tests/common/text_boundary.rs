//! Text-boundary metric: do lines start and end on the same words as the reference?
//! Shared by `tests/text_boundary.rs` and `tools/src/bin/page_metrics.rs` so the
//! engine-comparison viewer reports exactly the harness number.
use std::path::Path;
use std::process::Command;

/// Words per page from one `mutool draw -F text` run over the whole document. mutool ends every
/// page with a form feed, and the whole-document output is byte-identical to the per-page outputs
/// concatenated, so this is the same tokenisation as one run per page at a fraction of the cost:
/// mutool parses the PDF once instead of once per page (on a 200-page document the per-page
/// spawns were two thirds of the metric's time).
pub fn extract_all_pages(pdf: &Path) -> Vec<Vec<String>> {
    let output = Command::new("mutool")
        .args(["draw", "-F", "text", pdf.to_str().unwrap()])
        .output()
        .expect("Failed to run mutool draw");
    let text = String::from_utf8_lossy(&output.stdout);
    let mut pages: Vec<Vec<String>> = text
        .split('\x0c')
        .map(|page| page.split_whitespace().map(String::from).collect())
        .collect();
    if pages.last().is_some_and(|p| p.is_empty()) {
        pages.pop(); // after the last page's form feed
    }
    pages
}

/// Text lines per page from one `mutool draw -F stext` run over the whole document, each page
/// parsed by [`stext_lines`].
pub fn extract_all_page_lines(pdf: &Path) -> Vec<Vec<String>> {
    let output = Command::new("mutool")
        .args(["draw", "-F", "stext", pdf.to_str().unwrap()])
        .output()
        .expect("Failed to run mutool draw -F stext");
    let xml = String::from_utf8_lossy(&output.stdout);
    let mut pages = Vec::new();
    let mut page: Option<Vec<&str>> = None;
    for xml_line in xml.lines() {
        let trimmed = xml_line.trim();
        if trimmed.starts_with("<page ") {
            page = Some(Vec::new());
        } else if trimmed == "</page>" {
            if let Some(p) = page.take() {
                pages.push(stext_lines(p));
            }
        } else if let Some(p) = page.as_mut() {
            p.push(trimmed);
        }
    }
    pages
}

/// The text lines of one stext page, in reading order: lines sorted by y, clustered within an
/// 8pt y-tolerance, each cluster sorted by x so super/subscript fragments recombine
/// left-to-right (e.g. "xi" + "2 + yj" + "3 = zk").
fn stext_lines(xml_lines: Vec<&str>) -> Vec<String> {
    let mut lines: Vec<(f64, f64, String)> = Vec::new(); // (x_left, y_top, text)
    for trimmed in xml_lines {
        if let Some(rest) = trimmed.strip_prefix("<line ") {
            let bbox_vals: Option<(f64, f64)> = rest.strip_prefix("bbox=\"").and_then(|b| {
                let mut parts = b.split_whitespace();
                let x = parts.next()?.parse::<f64>().ok()?;
                let y = parts.next()?.parse::<f64>().ok()?;
                Some((x, y))
            });
            let (x_left, y_top) = bbox_vals.unwrap_or((0.0, 0.0));
            if let Some(start) = rest.find("text=\"") {
                let after_quote = &rest[start + 6..];
                if let Some(end) = after_quote.find('"') {
                    let text = &after_quote[..end];
                    let text = text.trim();
                    if !text.is_empty() {
                        lines.push((x_left, y_top, text.to_string()));
                    }
                }
            }
        }
    }
    lines.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut clusters: Vec<Vec<(f64, f64, String)>> = Vec::new();
    for item in lines {
        if let Some(last) = clusters.last_mut()
            && (item.1 - last[0].1).abs() < 8.0
        {
            last.push(item);
            continue;
        }
        clusters.push(vec![item]);
    }
    clusters
        .into_iter()
        .map(|mut cluster| {
            cluster.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            cluster
                .into_iter()
                .map(|(_, _, t)| t)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

pub fn break_positions(pages: &[Vec<String>]) -> Vec<usize> {
    let mut pos = Vec::with_capacity(pages.len());
    let mut cumulative = 0;
    for page in pages {
        cumulative += page.len();
        pos.push(cumulative);
    }
    pos
}

/// Replace tab-leader dots (runs of 3+) with a space so comparison focuses on text content.
pub fn normalize_leaders(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dots = 0usize;
    for c in s.chars() {
        if c == '.' {
            dots += 1;
        } else {
            if dots >= 3 {
                out.push(' ');
            } else {
                for _ in 0..dots {
                    out.push('.');
                }
            }
            dots = 0;
            out.push(c);
        }
    }
    if dots > 0 && dots < 3 {
        for _ in 0..dots {
            out.push('.');
        }
    }
    out
}

pub fn first_word(s: &str) -> String {
    let n = normalize_leaders(s);
    n.split_whitespace().next().unwrap_or_default().to_string()
}

pub fn last_word(s: &str) -> String {
    let n = normalize_leaders(s);
    n.split_whitespace().last().unwrap_or_default().to_string()
}

pub struct TextBoundary {
    pub ref_pages: usize,
    pub gen_pages: usize,
    pub max_break_drift: i64,
    pub total_words: usize,
    pub total_lines: usize,
    pub matching_lines: usize,
}

impl TextBoundary {
    pub fn line_match_pct(&self) -> f64 {
        if self.total_lines > 0 {
            self.matching_lines as f64 / self.total_lines as f64
        } else {
            0.0
        }
    }
}

/// Compare `generated_pdf` against `reference_pdf` page by page.
pub fn analyze(reference_pdf: &Path, generated_pdf: &Path) -> TextBoundary {
    let ref_word_pages = extract_all_pages(reference_pdf);
    let gen_word_pages = extract_all_pages(generated_pdf);
    let common_pages = ref_word_pages.len().min(gen_word_pages.len());

    let ref_breaks = break_positions(&ref_word_pages);
    let gen_breaks = break_positions(&gen_word_pages);
    let total_words = ref_breaks.last().copied().unwrap_or(0);
    let break_count = (ref_breaks.len().saturating_sub(1)).min(gen_breaks.len().saturating_sub(1));
    let max_break_drift = (0..break_count)
        .map(|i| gen_breaks[i] as i64 - ref_breaks[i] as i64)
        .max_by_key(|d| d.unsigned_abs())
        .unwrap_or(0);

    let ref_line_pages = extract_all_page_lines(reference_pdf);
    let gen_line_pages = extract_all_page_lines(generated_pdf);
    let mut total_lines = 0;
    let mut matching_lines = 0;
    for p in 0..common_pages {
        let (Some(ref_lines), Some(gen_lines)) = (ref_line_pages.get(p), gen_line_pages.get(p))
        else {
            break;
        };

        let max_count = ref_lines.len().max(gen_lines.len());
        let min_count = ref_lines.len().min(gen_lines.len());
        if max_count > 0 && (max_count - min_count) as f64 / max_count as f64 > 0.15 {
            continue;
        }

        for l in 0..min_count {
            total_lines += 1;
            if first_word(&ref_lines[l]) == first_word(&gen_lines[l])
                && last_word(&ref_lines[l]) == last_word(&gen_lines[l])
            {
                matching_lines += 1;
            }
        }
    }

    TextBoundary {
        ref_pages: ref_word_pages.len(),
        gen_pages: gen_word_pages.len(),
        max_break_drift,
        total_words,
        total_lines,
        matching_lines,
    }
}
