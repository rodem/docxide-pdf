use std::collections::{BTreeMap, HashMap, HashSet};

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Name, Pdf, Rect, Ref, Str};
use ttf_parser::Face;
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};

use super::FontMetrics;
use super::encoding::winansi_to_char;

pub(super) fn embed_truetype(
    pdf: &mut Pdf,
    font_ref: Ref,
    descriptor_ref: Ref,
    data_ref: Ref,
    font_name: &str,
    font_data: &[u8],
    face_index: u32,
    used_chars: &HashSet<char>,
    alloc: &mut impl FnMut() -> Ref,
) -> Option<FontMetrics> {
    let face = Face::parse(font_data, face_index).ok()?;
    let units = face.units_per_em() as f32;
    let to_1000 = |v: f32| v / units * 1000.0;

    let ascent = to_1000(face.ascender() as f32);
    let descent = to_1000(face.descender() as f32);
    let cap_height = face
        .capital_height()
        .map(|h| to_1000(h as f32))
        .unwrap_or(700.0);

    let bb = face.global_bounding_box();
    let bbox = Rect::new(
        to_1000(bb.x_min as f32),
        to_1000(bb.y_min as f32),
        to_1000(bb.x_max as f32),
        to_1000(bb.y_max as f32),
    );

    let advance_1000 = |gid: ttf_parser::GlyphId| -> f32 {
        face.glyph_hor_advance(gid)
            .map(|adv| to_1000(adv as f32))
            .unwrap_or(0.0)
    };

    let widths_1000 = (32u8..=255u8)
        .map(|byte| {
            face.glyph_index(winansi_to_char(byte))
                .map(&advance_1000)
                .unwrap_or(0.0)
        })
        .collect();

    let mut remapper = subsetter::GlyphRemapper::new();
    let mut char_to_gid = HashMap::with_capacity(used_chars.len());
    let mut char_widths_1000 = HashMap::with_capacity(used_chars.len());

    // HashSet order differs per process, and the remapper numbers glyphs in visit order:
    // sorting keeps the subset, and so the PDF bytes, the same from run to run.
    let mut chars: Vec<char> = used_chars.iter().copied().collect();
    chars.sort_unstable();
    for ch in chars {
        let gid = resolve_glyph(&face, ch);
        if let Some(gid) = gid {
            let new_gid = remapper.remap(gid.0);
            char_to_gid.insert(ch, new_gid);
            char_widths_1000.insert(ch, advance_1000(gid));
        }
    }

    let mut kern_pairs = HashMap::new();
    let char_gids: Vec<(ttf_parser::GlyphId, u16)> = char_to_gid
        .iter()
        .filter_map(|(&ch, &new_gid)| face.glyph_index(ch).map(|orig| (orig, new_gid)))
        .collect();

    extract_kern_pairs(&face, &char_gids, units, &mut kern_pairs);
    extract_gpos_pairs(&face, &char_gids, units, &mut kern_pairs);

    if !kern_pairs.is_empty() {
        log::info!(
            "Kerning for {font_name}: {} pairs from {} chars",
            kern_pairs.len(),
            char_gids.len(),
        );
    }

    let subset_data = subsetter::subset(font_data, face_index, &remapper).unwrap_or_else(|e| {
        log::warn!("Font subsetting failed for {font_name}: {e} — embedding full font");
        font_data.to_vec()
    });

    let data_len = i32::try_from(subset_data.len()).ok()?;
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&subset_data, 6);
    {
        let mut stream = pdf.stream(data_ref, &compressed);
        stream.filter(Filter::FlateDecode);
        stream.pair(Name(b"Length1"), data_len);
    }

    let ps_name = font_name.replace(' ', "");
    let ps_name_ref = Name(ps_name.as_bytes());
    let system_info = SystemInfo {
        registry: Str(b"Adobe"),
        ordering: Str(b"Identity"),
        supplement: 0,
    };

    pdf.font_descriptor(descriptor_ref)
        .name(ps_name_ref)
        .flags(FontFlags::NON_SYMBOLIC)
        .bbox(bbox)
        .italic_angle(0.0)
        .ascent(ascent)
        .descent(descent)
        .cap_height(cap_height)
        .stem_v(80.0)
        .font_file2(data_ref);

    let cid_font_ref = alloc();
    {
        let mut cid = pdf.cid_font(cid_font_ref);
        cid.subtype(CidFontType::Type2);
        cid.base_font(ps_name_ref);
        cid.system_info(system_info);
        cid.font_descriptor(descriptor_ref);
        cid.default_width(0.0);
        cid.cid_to_gid_map_predefined(Name(b"Identity"));

        let mut gid_widths: Vec<(u16, f32)> = char_to_gid
            .iter()
            .filter_map(|(&ch, &new_gid)| {
                // Same lookup as the glyph itself: a symbol font's character
                // missing here got /DW 0 while drawn at its real width (7.21.5).
                resolve_glyph(&face, ch).map(|gid| (new_gid, advance_1000(gid)))
            })
            .collect();
        gid_widths.sort_by_key(|&(gid, _)| gid);
        if !gid_widths.is_empty() {
            let mut w = cid.widths();
            for &(gid, width) in &gid_widths {
                w.consecutive(gid, [width]);
            }
        }
    }

    let tounicode_ref = alloc();
    let cmap_name = format!("{}-UTF16", ps_name);
    let mut cmap = UnicodeCmap::new(Name(cmap_name.as_bytes()), system_info);
    let pua_unicode: fn(char) -> Option<char> = if font_name.eq_ignore_ascii_case("symbol") {
        symbol_font_unicode
    } else if font_name.eq_ignore_ascii_case("wingdings") {
        wingdings_unicode
    } else {
        |_| None
    };
    // Several characters can share a glyph (hyphen variants, no-break space)
    // but a CID maps to one character: keep the lowest code point, the plain
    // form, instead of whichever the HashMap happened to yield last.
    let mut cid_unicode: BTreeMap<u16, char> = BTreeMap::new();
    for (&ch, &new_gid) in &char_to_gid {
        let uni = pua_unicode(ch).unwrap_or(ch);
        cid_unicode
            .entry(new_gid)
            .and_modify(|c| *c = (*c).min(uni))
            .or_insert(uni);
    }
    for (cid, uni) in cid_unicode {
        cmap.pair(cid, uni);
    }
    pdf.stream(tounicode_ref, cmap.finish().as_slice());

    pdf.type0_font(font_ref)
        .base_font(ps_name_ref)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid_font_ref)
        .to_unicode(tounicode_ref);

    let lm = compute_line_metrics(&face, units);

    Some(FontMetrics {
        widths_1000,
        line_h_ratio: lm.line_h_ratio,
        ascender_ratio: lm.ascender_ratio,
        grid_line_ratio: lm.grid_line_ratio,
        plain_line_h_ratio: lm.plain_line_h_ratio,
        grid_baseline_shift: lm.grid_baseline_shift,
        east_asian: lm.east_asian,
        plain_ascender_ratio: lm.plain_ascender_ratio,
        char_to_gid,
        char_widths_1000,
        kern_pairs,
    })
}

/// Adobe Symbol encoding for the Symbol font's private-use codes (U+F0xx) that
/// documents use as bullets and signs. Word's export maps them the same way, so
/// extracted text reads "•" rather than U+F0B7.
// ponytail: common bullets/signs only; the full Symbol encoding when Greek/math text needs it
fn symbol_font_unicode(ch: char) -> Option<char> {
    Some(match ch as u32 {
        0xF0B7 => '•',
        0xF02D => '−',
        0xF0B0 => '°',
        0xF0B1 => '±',
        0xF0B4 => '×',
        0xF0B8 => '÷',
        0xF0A3 => '≤',
        0xF0B3 => '≥',
        0xF0B9 => '≠',
        0xF0AB => '↔',
        0xF0AC => '←',
        0xF0AD => '↑',
        0xF0AE => '→',
        0xF0AF => '↓',
        0xF0A7 => '♣',
        0xF0A8 => '♦',
        0xF0A9 => '♥',
        0xF0AA => '♠',
        0xF0D7 => '⋅',
        0xF0D8 => '¬',
        0xF0E0 => '◊',
        _ => return None,
    })
}

/// Unicode for the Wingdings codes documents use as bullets and signs, from
/// the font's own glyph names (0xA7 `square4` → ▪). Word's export keeps the
/// private-use code in some documents (irish_school's U+F0A8 checkboxes),
/// which a screen reader skips, and maps to Unicode in others (samtale's ☺).
// ponytail: the corpus's codes only (no Wingdings 2/3, Webdings); add the
// full table when other symbols show up
fn wingdings_unicode(ch: char) -> Option<char> {
    // Runs can hold the low byte itself; the font maps both to one glyph.
    let code = match ch as u32 {
        c @ 0x20..=0xFF => c | 0xF000,
        c => c,
    };
    Some(match code {
        0xF021 => '✏',
        0xF026 => '📖',
        0xF04A => '☺',
        0xF06C => '●',
        0xF06E => '■',
        0xF06F => '□',
        0xF071 => '❑',
        0xF076 => '❖',
        0xF09E => '·',
        0xF09F => '•',
        0xF0A7 => '▪',
        0xF0A8 => '◻',
        0xF0B2 => '⟡',
        0xF0D8 => '➢',
        0xF0E0 | 0xF0E8 => '→',
        0xF0E4 => '↗',
        0xF0FC => '✔',
        0xF0FE => '☑',
        _ => return None,
    })
}

fn resolve_glyph(face: &Face, ch: char) -> Option<ttf_parser::GlyphId> {
    face.glyph_index(ch)
        .or_else(|| {
            // Symbol fonts use Private Use Area (0xF000-0xF0FF); try the low byte
            let cp = ch as u32;
            if (0xF000..=0xF0FF).contains(&cp) {
                face.glyph_index(char::from_u32(cp - 0xF000)?)
            } else {
                None
            }
        })
        .or_else(|| {
            // Fallback: direct cmap subtable lookup for symbol fonts
            face.tables()
                .cmap?
                .subtables
                .into_iter()
                .find_map(|st| st.glyph_index(ch as u32))
        })
        // A Unicode space the font lacks (U+202F in macOS Arial 5.01 and Aptos
        // Italic) takes the font's own space instead of drawing .notdef.
        // ponytail: spaces only, at U+0020's width; Word rescues any missing
        // glyph from another font at its real width (Arial Italic in
        // learning_cultures): widen the CJK rescue in `register_font` to
        // non-CJK characters when other glyphs go missing
        .or_else(|| {
            matches!(ch, '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}')
                .then(|| face.glyph_index(' '))
                .flatten()
        })
}

fn extract_kern_pairs(
    face: &Face,
    char_gids: &[(ttf_parser::GlyphId, u16)],
    units: f32,
    kern_pairs: &mut HashMap<(u16, u16), f32>,
) {
    let Some(kern) = face.tables().kern else {
        return;
    };
    let subtables: Vec<_> = kern
        .subtables
        .into_iter()
        .filter(|st| st.horizontal && !st.variable)
        .collect();
    for &(l_orig, l_new) in char_gids {
        for &(r_orig, r_new) in char_gids {
            let total: i16 = subtables
                .iter()
                .filter_map(|st| st.glyphs_kerning(l_orig, r_orig))
                .sum();
            if total != 0 {
                kern_pairs.insert((l_new, r_new), total as f32 / units * 1000.0);
            }
        }
    }
}

fn extract_gpos_pairs(
    face: &Face,
    char_gids: &[(ttf_parser::GlyphId, u16)],
    units: f32,
    kern_pairs: &mut HashMap<(u16, u16), f32>,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for lookup_idx in 0..gpos.lookups.len() {
        let Some(lookup) = gpos.lookups.get(lookup_idx) else {
            continue;
        };
        for st_idx in 0..lookup.subtables.len() {
            let Some(PositioningSubtable::Pair(pair)) =
                lookup.subtables.get::<PositioningSubtable>(st_idx)
            else {
                continue;
            };
            match pair {
                PairAdjustment::Format1 { coverage, sets } => {
                    for &(l_orig, l_new) in char_gids {
                        let Some(cov_idx) = coverage.get(l_orig) else {
                            continue;
                        };
                        let Some(pair_set) = sets.get(cov_idx) else {
                            continue;
                        };
                        for &(r_orig, r_new) in char_gids {
                            if let Some((val1, _)) = pair_set.get(r_orig)
                                && val1.x_advance != 0
                            {
                                kern_pairs
                                    .entry((l_new, r_new))
                                    .or_insert(val1.x_advance as f32 / units * 1000.0);
                            }
                        }
                    }
                }
                PairAdjustment::Format2 {
                    coverage,
                    classes,
                    matrix,
                } => {
                    for &(l_orig, l_new) in char_gids {
                        if coverage.get(l_orig).is_none() {
                            continue;
                        }
                        let c1 = classes.0.get(l_orig);
                        for &(r_orig, r_new) in char_gids {
                            let c2 = classes.1.get(r_orig);
                            if let Some((val1, _)) = matrix.get((c1, c2))
                                && val1.x_advance != 0
                            {
                                kern_pairs
                                    .entry((l_new, r_new))
                                    .or_insert(val1.x_advance as f32 / units * 1000.0);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) struct LineMetrics {
    pub(super) line_h_ratio: f32,
    pub(super) ascender_ratio: f32,
    /// What Word counts docGrid cells with: sTypo metrics for Latin fonts (win +
    /// hhea lineGap would put Yu Mincho's 18pt-grid lines into two cells), the
    /// 1.3× line height for East Asian fonts.
    pub(super) grid_line_ratio: Option<f32>,
    /// The Latin-rule values, for whitespace-only runs and empty paragraph marks
    /// in an East Asian font (`pdf::layout::run_line_metrics`).
    pub(super) plain_line_h_ratio: f32,
    pub(super) plain_ascender_ratio: f32,
    /// How far below a docGrid cell's centre Word puts the baseline, per em.
    pub(super) grid_baseline_shift: f32,
    /// Laid out by Word's East Asian rules (1.3× leading, see above).
    pub(super) east_asian: bool,
}

/// Has glyphs for CJK ideographs, Hangul or kana — what Word treats as an East Asian font.
fn is_east_asian_font(face: &Face) -> bool {
    ['一', '가', 'あ']
        .iter()
        .any(|&c| face.glyph_index(c).is_some())
}

/// Word lays out an East Asian font 1.3× taller than its Windows metrics
/// (10.5pt SimSun → the classic 15.6pt line), no hhea lineGap, the extra leading
/// above the glyphs so an exact-height box still bottom-aligns at winDescent.
/// Whitespace-only runs keep the plain values. Measurements: roadmap, "CJK
/// Rendering Polish" item 3.
fn compute_line_metrics(face: &Face, units: f32) -> LineMetrics {
    let (plain_line_h_ratio, plain_ascender_ratio, typo_ratio) = plain_line_metrics(face, units);
    let east_asian = match face.tables().os2 {
        Some(os2) if is_east_asian_font(face) => {
            let win_desc = -(os2.windows_descender() as f32) / units;
            let win_h = (os2.windows_ascender() - os2.windows_descender()) as f32 / units;
            let line_h = win_h * 1.3;
            Some((line_h, line_h - win_desc))
        }
        _ => None,
    };
    // Word centres a grid-snapped line's glyph box (ascent + descent) in its
    // cell. A Latin font keeps its line gap above the ascent, as in its normal
    // lines; an East Asian font has none (its 1.3× leading is dropped too).
    let grid_baseline_shift = match face.tables().os2 {
        Some(os2) => {
            let typo = os2.use_typographic_metrics() && east_asian.is_none();
            let (asc, desc) = if typo {
                (os2.typographic_ascender(), os2.typographic_descender())
            } else {
                (os2.windows_ascender(), os2.windows_descender())
            };
            let gap = if typo || east_asian.is_some() {
                0
            } else {
                face.line_gap()
            };
            ((asc + desc) as f32 / 2.0 + gap as f32) / units
        }
        None => (face.ascender() + face.descender()) as f32 / 2.0 / units,
    };
    LineMetrics {
        line_h_ratio: east_asian.map_or(plain_line_h_ratio, |(h, _)| h),
        ascender_ratio: east_asian.map_or(plain_ascender_ratio, |(_, a)| a),
        grid_line_ratio: east_asian.map(|(h, _)| h).or(typo_ratio),
        plain_line_h_ratio,
        plain_ascender_ratio,
        grid_baseline_shift,
        east_asian: east_asian.is_some(),
    }
}

/// Returns (line_h_ratio, ascender_ratio, typo_line_ratio) by the Latin rules.
fn plain_line_metrics(face: &Face, units: f32) -> (f32, f32, Option<f32>) {
    if let Some(os2) = face.tables().os2 {
        let t_asc = os2.typographic_ascender() as f32;
        let t_desc = os2.typographic_descender() as f32;
        let t_gap = os2.typographic_line_gap() as f32;
        let typo_ratio = Some((t_asc - t_desc + t_gap) / units);
        if os2.use_typographic_metrics() {
            return ((t_asc - t_desc + t_gap) / units, t_asc / units, typo_ratio);
        }
        let win_asc = os2.windows_ascender() as f32;
        let win_desc = os2.windows_descender() as f32;
        // usWinAscent/Descent define glyph clipping bounds; hhea lineGap
        // provides external leading that Word includes in both line spacing
        // and baseline positioning (ascender offset from slot top)
        let gap = face.line_gap() as f32;
        return (
            (win_asc - win_desc + gap) / units,
            (win_asc + gap) / units,
            typo_ratio,
        );
    }

    let line_gap = face.line_gap() as f32;
    (
        (face.ascender() as f32 - face.descender() as f32 + line_gap) / units,
        face.ascender() as f32 / units,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wingdings_bullets_extract_as_unicode() {
        assert_eq!(wingdings_unicode('\u{F0A7}'), Some('▪'));
        assert_eq!(wingdings_unicode('\u{F0FC}'), Some('✔'));
        assert_eq!(wingdings_unicode('§'), Some('▪'));
        assert_eq!(wingdings_unicode(' '), None);
        assert_eq!(wingdings_unicode('\u{F0FA}'), None);
    }
}
