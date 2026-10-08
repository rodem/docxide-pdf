use std::collections::HashMap;

use pdf_writer::{Content, Name, Str};

use crate::fonts::{FontEntry, font_key};
use crate::model::{Paragraph, Run};

use super::color::fill_rgb;

pub(super) fn label_font_key(para: &Paragraph) -> Option<String> {
    if let Some(ref bf) = para.list_label_font {
        let mut k = bf.clone();
        if para.list_label_bold {
            k.push_str("/B");
        }
        Some(k)
    } else {
        let run = para.runs.first()?;
        let key_run = Run {
            bold: para.list_label_bold || run.bold,
            ..run.clone()
        };
        Some(font_key(&key_run))
    }
}

/// Symbol-font PUA bullet codepoints map to Unicode equivalents that any
/// general-purpose font can render. When the labeled font is missing or
/// can't encode the PUA char, fall back to these.
pub(super) fn symbol_pua_to_unicode(c: char) -> Option<char> {
    match c {
        '\u{F0B7}' => Some('\u{2022}'), // •  Symbol bullet
        '\u{F0A7}' => Some('\u{25AA}'), // ▪  Symbol filled square
        '\u{F0D8}' => Some('\u{2192}'), // →  Symbol right arrow
        '\u{F0FC}' => Some('\u{2713}'), // ✓  Symbol checkmark
        _ => None,
    }
}

fn map_symbol_pua(text: &str) -> Option<String> {
    if !text.chars().any(|c| symbol_pua_to_unicode(c).is_some()) {
        return None;
    }
    Some(
        text.chars()
            .map(|c| symbol_pua_to_unicode(c).unwrap_or(c))
            .collect(),
    )
}

/// The label's glyphs and a space, so text extraction and screen readers
/// don't run the label into the paragraph text ("1.01SECTION"); invisible,
/// as nothing follows it in its text object. Fonts without a space get none.
/// None when the font draws none of the label (all `.notdef`).
pub(super) fn encode_label(entry: &FontEntry, label: &str) -> Option<Vec<u8>> {
    let mut bytes = entry.encode(label);
    if is_all_notdef(&bytes) {
        return None;
    }
    if entry.has_char(' ') {
        bytes.extend(entry.encode(" "));
    }
    Some(bytes)
}

fn label_for_paragraph<'a>(
    para: &Paragraph,
    seen_fonts: &'a HashMap<String, FontEntry>,
) -> (&'a str, Vec<u8>) {
    let key = label_font_key(para);
    let entry = key.as_deref().and_then(|k| seen_fonts.get(k));

    if let Some(entry) = entry
        && let Some(bytes) = encode_label(entry, &para.list_label)
    {
        return (entry.pdf_name.as_str(), bytes);
    }

    // Either the labeled font is missing, or it produced only .notdef
    // glyphs (typical for Symbol-PUA chars when the font's cmap can't
    // round-trip them). Fall back to the surrounding body-run font with
    // Symbol-PUA chars mapped to their Unicode equivalents.
    if let Some(mapped) = map_symbol_pua(&para.list_label)
        && let Some(run) = para.runs.first()
        && let Some(body_entry) = seen_fonts.get(&font_key(run))
    {
        let bytes = encode_label(body_entry, &mapped).unwrap_or_else(|| body_entry.encode(&mapped));
        return (body_entry.pdf_name.as_str(), bytes);
    }

    ("", vec![])
}

/// `encode_as_gids` writes the .notdef glyph (gid 0) for any char missing
/// from the map. A run of pure-zero output means the font couldn't render
/// anything — treat that as a miss and fall through to substitution.
fn is_all_notdef(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.chunks_exact(2).all(|c| c == [0, 0])
}

pub(super) fn render_list_label(
    content: &mut Content,
    para: &Paragraph,
    fonts: &HashMap<String, FontEntry>,
    label_x: f32,
    baseline_y: f32,
    fallback_font_size: f32,
) {
    if para.list_label.is_empty() {
        return;
    }
    let (label_font_name, label_bytes) = label_for_paragraph(para, fonts);
    let label_color = para
        .list_label_color
        .or_else(|| para.runs.first().and_then(|r| r.color));
    if let Some(c) = label_color {
        fill_rgb(content, c);
    }
    let label_fs = para.list_label_font_size.unwrap_or(fallback_font_size);
    content
        .begin_text()
        .set_font(Name(label_font_name.as_bytes()), label_fs)
        .next_line(label_x, baseline_y)
        .show(Str(&label_bytes))
        .end_text();
    if label_color.is_some() {
        content.set_fill_gray(0.0);
    }
}

/// Text starts after the label at the next available tab position. The
/// first-line indent also moves the label; tab gaps are not label width.
pub(super) fn text_hanging(para: &Paragraph, default_tab_stop: f32,
    fonts: &HashMap<String, FontEntry>) -> f32 {
    if para.list_label.is_empty() {
        return if para.indent_hanging > 0.0 { para.indent_hanging }
            else { -para.indent_first_line };
    }
    let label_x = para.indent_left - para.indent_hanging + para.indent_first_line;
    let entry = label_font_key(para).and_then(|k| fonts.get(&k));
    let size = para.list_label_font_size.unwrap_or_else(||
        para.runs.first().map_or(11.0, |r| r.font_size));
    let width = entry.map_or(0.0, |f| f.word_width(&para.list_label, size, false));
    let end = label_x + width;
    let next = para.tab_stops.iter().map(|t| t.position)
        .chain(para.num_level_tab_stop)
        .chain((para.indent_hanging > 0.0).then_some(para.indent_left))
        .filter(|p| *p > end + 0.01).min_by(f32::total_cmp)
        .unwrap_or_else(|| if default_tab_stop > 0.0 {
            ((end / default_tab_stop).floor() + 1.0) * default_tab_stop
        } else { end });
    para.indent_left - next
}
