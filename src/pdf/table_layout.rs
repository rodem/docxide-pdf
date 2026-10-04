use std::collections::HashMap;
use std::sync::LazyLock;

use crate::fonts::{FontEntry, font_key_buf};

static EMPTY_INLINE_IMAGE_MAP: LazyLock<HashMap<usize, String>> = LazyLock::new(HashMap::new);
static EMPTY_EFFECT_MAP: LazyLock<HashMap<usize, super::images::EffectXObjs>> =
    LazyLock::new(HashMap::new);
use crate::model::{
    Alignment, Block, CellMargins, HorizontalPosition, Table, TextDirection, VMerge,
    VerticalPosition, WrapType,
};

use super::RenderContext;
use super::header_footer::substitute_hf_runs;
use super::layout::{
    TextLine, build_paragraph_lines, build_tabbed_line, east_asian_leading, is_text_empty,
    run_line_metrics,
};
use super::resolve_line_h;

pub(super) fn cell_span_width(col_widths: &[f32], grid_col: usize, span: usize) -> f32 {
    // Clamp the start too: malformed tables (missing tblGrid, gridSpan
    // overrun) can push grid_col past the grid — yield 0 instead of panicking.
    let start = grid_col.min(col_widths.len());
    col_widths[start..col_widths.len().min(grid_col + span)]
        .iter()
        .sum()
}

/// Scale column widths so the table occupies its `w:tblW type="pct"` share
/// of the available content width. tcW pct values are mis-read as twips at
/// parse time, but their proportions survive — only the total needs fixing.
/// Only applies to inferred grids: when a real tblGrid exists, Word renders
/// its widths as-is even when the pct preferred width disagrees (observed
/// with pct values of 100.4–115% alongside grids matching the content width).
pub(super) fn apply_pct_width(table: &Table, widths: &mut [f32], available_w: f32) {
    if !table.grid_inferred {
        return;
    }
    let Some(pct) = table.width_pct else { return };
    // Word caps a table's width at the text column width.
    let target = available_w * pct.min(1.0);
    let total: f32 = widths.iter().sum();
    if total > 0.0 && target > 0.0 {
        let scale = target / total;
        for w in widths {
            *w *= scale;
        }
    }
}

pub(super) fn cell_x_offset(col_widths: &[f32], table_left: f32, grid_col: usize) -> f32 {
    table_left
        + col_widths[..grid_col.min(col_widths.len())]
            .iter()
            .sum::<f32>()
}

/// Height of a paragraph's text content, matching the layout computation in
/// `compute_row_layouts`. Empty paragraphs (no lines) still occupy one line
/// height unless they carry an explicit `content_height` (e.g. from an image).
pub(super) fn para_block_height(p: &CellParagraphLayout) -> f32 {
    if p.lines.is_empty() {
        if p.paragraph_mark_vanish {
            0.0
        } else if p.content_height > 0.0 {
            p.content_height
        } else {
            p.line_h
        }
    } else {
        cell_lines_h(p, 0..p.lines.len())
    }
}

/// Height of a cell paragraph's lines `range`, each at its own pitch
/// (`size_lines_by_own_runs`).
pub(super) fn cell_lines_h(p: &CellParagraphLayout, range: std::ops::Range<usize>) -> f32 {
    p.lines[range]
        .iter()
        .map(|l| l.pitch.unwrap_or(p.line_h))
        .sum()
}

/// Auto-fit column widths so that the longest non-breakable word in each column
/// fits within the cell (including padding). Columns that need more space grow;
/// other columns shrink proportionally. Total width is preserved.
/// When `available_width` is provided and the table exceeds it, all columns
/// are scaled down proportionally to fit (matching Word's behavior).
/// For nested auto-fit tables (`available_width` is Some and not fixed layout),
/// Word shrinks columns to content-based minimum widths rather than using the
/// gridCol preferred widths.
/// Per-column natural (unwrapped, single-line) content width including cell
/// horizontal padding. This is the "max" width input to Word's AutoFit.
fn natural_widths(
    table: &Table,
    fonts: &HashMap<String, FontEntry>,
    cm: &crate::model::CellMargins,
) -> Vec<f32> {
    let ncols = table.col_widths.len();
    let mut natural = vec![0.0f32; ncols];
    for row in &table.rows {
        for (grid_col, span, cell) in row.grid_cells() {
            if grid_col >= ncols || span > 1 {
                continue;
            }
            let ecm = cell.cell_margins.as_ref().unwrap_or(cm);
            let h_pad = ecm.left + ecm.right;
            let mut key_buf = String::new();
            for para in cell.all_paragraphs() {
                let mut para_w = 0.0f32;
                for run in &para.runs {
                    let key = font_key_buf(run, &mut key_buf);
                    let Some(entry) = fonts.get(key) else {
                        continue;
                    };
                    let fs = run.font_size;
                    let text = if run.caps {
                        std::borrow::Cow::Owned(run.text.to_uppercase())
                    } else {
                        std::borrow::Cow::Borrowed(&run.text)
                    };
                    if run.small_caps {
                        para_w += super::layout::smallcaps_segments(&text, fs)
                            .iter()
                            .map(|(seg, seg_fs, _)| {
                                let kern = run.kerns_at(*seg_fs);
                                entry.word_width(seg, *seg_fs, kern)
                            })
                            .sum::<f32>();
                    } else {
                        let kern = run.kerns_at(fs);
                        para_w += entry.word_width(&text, fs, kern);
                    }
                }
                natural[grid_col] = natural[grid_col].max(para_w + h_pad);
            }
        }
    }
    natural
}

/// Raise each column's natural width to fit any directly nested table at its
/// own content-fitted width (plus the host cell's h-padding). The flattened
/// paragraph widths from `natural_widths` can't see the nested grid: three
/// side-by-side nested columns need their *sum*, not the widest paragraph.
fn raise_natural_for_nested_tables(
    table: &Table,
    fonts: &HashMap<String, FontEntry>,
    cm: &CellMargins,
    natural: &mut [f32],
) {
    let ncols = natural.len();
    for row in &table.rows {
        for (grid_col, span, cell) in row.grid_cells() {
            if grid_col >= ncols || span > 1 {
                continue;
            }
            let ecm = cell.cell_margins.as_ref().unwrap_or(cm);
            let h_pad = ecm.left + ecm.right;
            for block in &cell.content {
                if let Block::Table(nt) = block {
                    let ncm = &nt.cell_margins;
                    let mut nat = natural_widths(nt, fonts, ncm);
                    raise_natural_for_nested_tables(nt, fonts, ncm, &mut nat);
                    let min_cell = ncm.left + ncm.right;
                    let w: f32 = nat.iter().map(|&x| x.max(min_cell)).sum();
                    natural[grid_col] = natural[grid_col].max(w + h_pad);
                }
            }
        }
    }
}

/// Distribute `avail` across columns proportionally to their natural ("max")
/// width, floored at each column's minimum width. Columns that would fall
/// below their minimum are pinned and the remaining width is re-shared among
/// the rest (Word's AutoFit-to-Window behavior). If the minimums alone exceed
/// `avail`, everything is scaled down to fit.
fn distribute_autofit(minw: &[f32], maxw: &[f32], avail: f32) -> Vec<f32> {
    let n = minw.len();
    let mut widths = vec![0.0f32; n];
    let mut pinned = vec![false; n];
    loop {
        let rem_avail: f32 = avail
            - (0..n)
                .filter(|&i| pinned[i])
                .map(|i| widths[i])
                .sum::<f32>();
        let active_max: f32 = (0..n).filter(|&i| !pinned[i]).map(|i| maxw[i]).sum();
        if active_max <= 0.0 {
            break;
        }
        let mut newly_pinned = false;
        for i in 0..n {
            if pinned[i] {
                continue;
            }
            let w = rem_avail * maxw[i] / active_max;
            if w < minw[i] {
                widths[i] = minw[i];
                pinned[i] = true;
                newly_pinned = true;
            }
        }
        if !newly_pinned {
            for i in 0..n {
                if !pinned[i] {
                    widths[i] = rem_avail * maxw[i] / active_max;
                }
            }
            break;
        }
    }
    let total: f32 = widths.iter().sum();
    if total > avail && total > 0.0 {
        let scale = avail / total;
        for w in &mut widths {
            *w *= scale;
        }
    }
    widths
}

/// Word squeezes an autofit table wider than its newspaper column by taking
/// from each grid column in proportion to its room above its longest word
/// (case80: two 234pt cells in a 252pt column come out 130 and 121.5).
pub(super) fn squeeze_to_width(widths: &mut [f32], min: &[f32], avail: f32) {
    let total: f32 = widths.iter().sum();
    if total <= avail {
        return;
    }
    let excess = total - avail;
    let slack: f32 = widths.iter().zip(min).map(|(w, m)| (w - m).max(0.0)).sum();
    if slack >= excess {
        for (w, m) in widths.iter_mut().zip(min) {
            *w -= (*w - m).max(0.0) * excess / slack;
        }
    } else {
        for w in widths {
            *w *= avail / total;
        }
    }
}

/// Each grid column's longest unbreakable word plus its cell padding.
pub(super) fn min_content_widths(table: &Table, fonts: &HashMap<String, FontEntry>) -> Vec<f32> {
    let ncols = table.col_widths.len();
    let cm = &table.cell_margins;
    let mut min_widths = vec![0.0f32; ncols];
    for row in &table.rows {
        for (grid_col, span, cell) in row.grid_cells() {
            if grid_col >= ncols || span > 1 {
                continue;
            }
            if cell.text_direction != TextDirection::LrTb {
                continue;
            }
            let ecm = cell.cell_margins.as_ref().unwrap_or(cm);
            let h_pad = ecm.left + ecm.right;
            let mut key_buf = String::new();
            for para in cell.all_paragraphs() {
                for run in &para.runs {
                    let key = font_key_buf(run, &mut key_buf);
                    let Some(entry) = fonts.get(key) else {
                        continue;
                    };
                    let text = if run.caps {
                        std::borrow::Cow::Owned(run.text.to_uppercase())
                    } else {
                        std::borrow::Cow::Borrowed(&run.text)
                    };
                    let fs = run.font_size;
                    // Same break opportunities as line layout: a CJK sentence holds
                    // no spaces but may wrap after any ideograph.
                    let words = super::layout::split_preserving_spaces(&text)
                        .into_iter()
                        .map(|(_, w)| w);
                    for word in words {
                        let ww = if run.small_caps {
                            super::layout::smallcaps_segments(word, fs)
                                .iter()
                                .map(|(seg, seg_fs, _)| {
                                    let kern = run.kerns_at(*seg_fs);
                                    entry.word_width(seg, *seg_fs, kern)
                                })
                                .sum::<f32>()
                                + h_pad
                        } else {
                            let kern = run.kerns_at(fs);
                            entry.word_width(word, fs, kern) + h_pad
                        };
                        min_widths[grid_col] = min_widths[grid_col].max(ww);
                    }
                }
            }
        }
    }
    min_widths
}

/// `fill_width`, when `Some`, is the content width a top-level AutoFit-to-Window
/// table should fill. It is kept separate from `available_width` (which drives
/// the nested-table shrink path) so passing a fill target does not accidentally
/// push a top-level table onto the shrink path.
pub(super) fn auto_fit_columns(
    table: &Table,
    fonts: &HashMap<String, FontEntry>,
    available_width: Option<f32>,
    fill_width: Option<f32>,
) -> Vec<f32> {
    let ncols = table.col_widths.len();
    if ncols == 0 {
        return table.col_widths.clone();
    }

    let cm = &table.cell_margins;
    let min_widths = min_content_widths(table, fonts);

    // Word's AutoFit (tblLayout=autofit, the default) ignores the stored
    // gridCol widths for a `tblW type="auto"` table and re-derives column
    // widths from cell *content*, filling the available width (AutoFit to
    // Window). OOXML §17.18.87: "uses the contents of each cell to determine
    // final column widths."
    //
    // We only apply this to a uniform-grid `type="auto"` table whose cells
    // directly hold a nested table. A Word-saved autofit table stores
    // content-derived (unequal) gridCol widths, so honoring those reproduces
    // Word — recomputing from our own font metrics would only drift. The narrow
    // case where the stored grid is provably meaningless is an equal-column grid
    // wrapping a nested table (case51's 4680/4680 outer tables): the nested
    // table establishes a hard content width that the equal split ignores, so
    // Word sizes purely to content. Gating on a directly-nested table keeps
    // ordinary text tables (which legitimately keep ~equal columns) on the
    // gridCol path and avoids the corpus-wide redistribution regressions.
    let grid_uniform = ncols >= 2 && table.col_widths.iter().all(|&w| w > 0.0) && {
        let first = table.col_widths[0];
        table
            .col_widths
            .iter()
            .all(|&w| (w - first).abs() <= first * 0.02 + 0.5)
    };
    let has_nested_table = table.rows.iter().any(|r| {
        r.cells
            .iter()
            .any(|c| c.content.iter().any(|b| matches!(b, Block::Table(_))))
    });
    if table.auto_width
        && !table.fixed_layout
        && grid_uniform
        && has_nested_table
        && let Some(avail) = fill_width.filter(|a| *a > 0.0)
    {
        let mut natural = natural_widths(table, fonts, cm);
        raise_natural_for_nested_tables(table, fonts, cm, &mut natural);
        let min_cell = cm.left + cm.right;
        let maxw: Vec<f32> = (0..ncols)
            .map(|i| natural[i].max(min_widths[i]).max(min_cell))
            .collect();
        // AutoFit to Contents: when every column fits at max-content
        // width, Word leaves the table narrower than the window rather
        // than stretching it to fill.
        if maxw.iter().sum::<f32>() <= avail {
            return maxw;
        }
        let minw: Vec<f32> = (0..ncols).map(|i| min_widths[i].max(min_cell)).collect();
        return distribute_autofit(&minw, &maxw, avail);
    }

    // For nested auto-fit tables, Word shrinks columns to content-based widths
    // rather than preserving the gridCol total. Each column is sized based on
    // a blend of the minimum width (longest word) and the natural width
    // (longest single-line paragraph), capped by the gridCol preferred width.
    if available_width.is_some() && !table.fixed_layout {
        let natural_widths = natural_widths(table, fonts, cm);
        // Word's auto-fit for nested tables produces column widths slightly
        // below the full natural paragraph width. Scale down by 0.9 to
        // approximate Word's sizing, ensuring text wraps where Word wraps it.
        let min_cell = cm.left + cm.right;
        let avail = available_width.unwrap_or(0.0);
        let preferred_total: f32 = table.col_widths.iter().sum();
        // When the nested table has an explicit tblInd and the gridCol
        // preferred widths fit inside the parent cell, Word uses those
        // preferred widths rather than shrinking to content. An explicit
        // tblInd signals the author deliberately sized and positioned the
        // nested table, so its column hints should be honored.
        let mut widths: Vec<f32> =
            if table.table_indent_explicit && preferred_total > 0.0 && preferred_total <= avail {
                (0..ncols)
                    .map(|i| {
                        let pref = table.col_widths.get(i).copied().unwrap_or(0.0);
                        let mw = min_widths[i].max(min_cell);
                        pref.max(mw)
                    })
                    .collect()
            } else {
                // At full natural width the table fits the parent cell → Word
                // keeps the content-fitted widths (AutoFit to Contents), ignoring
                // the stored gridCol hints (§17.18.87 derives purely from cell
                // content). Only when it overflows does Word squeeze below
                // natural width; the 0.9 factor approximates that squeeze.
                let full: Vec<f32> = (0..ncols)
                    .map(|i| natural_widths[i].max(min_widths[i].max(min_cell)))
                    .collect();
                if full.iter().sum::<f32>() <= avail {
                    full
                } else {
                    (0..ncols)
                        .map(|i| {
                            let mw = min_widths[i].max(min_cell);
                            let nw = natural_widths[i].max(mw);
                            let fitted = (nw * 0.9).max(mw);
                            fitted.min(table.col_widths.get(i).copied().unwrap_or(f32::MAX))
                        })
                        .collect()
                }
            };
        let total: f32 = widths.iter().sum();
        if total > avail && avail > 0.0 {
            let scale = avail / total;
            for w in &mut widths {
                *w *= scale;
            }
        }
        return widths;
    }

    // Per OOXML §17.18.87: the fixed-width base (used by auto-fit too)
    // sets each grid column to the maximum preferred width (tcW) from
    // all cells at that column, then scales proportionally if the total
    // exceeds the table width.
    let total: f32 = table.col_widths.iter().sum();
    let mut preferred = table.col_widths.clone();
    for row in &table.rows {
        for (grid_col, span, cell) in row.grid_cells() {
            if grid_col >= ncols {
                break;
            }
            if span == 1 {
                preferred[grid_col] = preferred[grid_col].max(cell.width);
            } else {
                // Distribute multi-span cell width proportionally across
                // the spanned grid columns.
                let grid_sum: f32 = table.col_widths[grid_col..ncols.min(grid_col + span)]
                    .iter()
                    .sum();
                if grid_sum > 0.0 && cell.width > grid_sum {
                    for g in grid_col..ncols.min(grid_col + span) {
                        let share = cell.width * (table.col_widths[g] / grid_sum);
                        preferred[g] = preferred[g].max(share);
                    }
                }
            }
        }
    }
    let pref_total: f32 = preferred.iter().sum();
    let mut widths = if pref_total > total && total > 0.0 {
        let scale = total / pref_total;
        preferred.iter().map(|&w| w * scale).collect::<Vec<_>>()
    } else {
        preferred
    };

    // Apply content minimums: ensure each column fits its longest word.
    // If any column needed boosting, shrink others proportionally.
    let mut extra_needed: f32 = 0.0;
    let mut shrinkable: f32 = 0.0;
    for i in 0..ncols {
        if min_widths[i] > widths[i] {
            extra_needed += min_widths[i] - widths[i];
            widths[i] = min_widths[i];
        } else {
            shrinkable += widths[i] - min_widths[i];
        }
    }
    if extra_needed > 0.0 && shrinkable > 0.0 {
        let factor = extra_needed.min(shrinkable) / shrinkable;
        for i in 0..ncols {
            if widths[i] > min_widths[i] {
                let available = widths[i] - min_widths[i];
                widths[i] -= available * factor;
            }
        }
        let new_total: f32 = widths.iter().sum();
        if (new_total - total).abs() > 0.01 {
            let scale = total / new_total;
            for w in &mut widths {
                *w *= scale;
            }
        }
    }

    if let Some(avail) = available_width {
        let final_total: f32 = widths.iter().sum();
        if final_total > avail && avail > 0.0 {
            let scale = avail / final_total;
            for w in &mut widths {
                *w *= scale;
            }
        }
    }

    widths
}

pub(super) struct CellFloatingImageLayout {
    pub(super) pdf_name: String,
    pub(super) display_width: f32,
    pub(super) display_height: f32,
    pub(super) h_offset: f32,
    pub(super) v_offset: f32,
    /// In-plane rotation in degrees (OOXML clockwise). Cell-anchored floats must
    /// carry this just like body floats, else e.g. a 90°-rotated vertical label
    /// renders horizontally.
    pub(super) rotation_deg: f32,
    /// wp:anchor relativeHeight, so a cell picture can paint over a
    /// connector/textbox anchored in the same paragraph (annotation #241).
    pub(super) z_index: u32,
    /// The picture's `docPr@descr` and decorative flag, for tagging.
    pub(super) alt: Option<String>,
    pub(super) decorative: bool,
}

impl CellFloatingImageLayout {
    /// (alt, decorative), for `cell_figure`.
    pub(super) fn tagging(&self) -> (Option<&str>, bool) {
        (self.alt.as_deref(), self.decorative)
    }
}

#[derive(Default)]
pub(super) struct CellParagraphLayout {
    pub(super) lines: Vec<TextLine>,
    pub(super) line_h: f32,
    pub(super) font_size: f32,
    pub(super) ascender_ratio: f32,
    pub(super) descender_ratio: f32,
    pub(super) font_substituted: bool,
    pub(super) alignment: Alignment,
    pub(super) space_before: f32,
    pub(super) indent_left: f32,
    pub(super) indent_right: f32,
    pub(super) indent_hanging: f32,
    pub(super) indent_first_line: f32,
    /// Extra left indent from wrapSquare/Tight floating images in this paragraph.
    /// Text lines are laid out narrower and rendered further right to avoid the image.
    pub(super) float_indent_left: f32,
    pub(super) list_label: String,
    pub(super) list_label_font: Option<String>,
    /// For L/LI tagging inside the cell.
    pub(super) list_item: Option<crate::model::ListItem>,
    pub(super) label_color: Option<[u8; 3]>,
    pub(super) first_run_font_key: String,
    pub(super) image_name: Option<String>,
    /// The picture's `docPr@descr` and decorative flag, for tagging.
    pub(super) image_alt: Option<String>,
    pub(super) image_decorative: bool,
    pub(super) image_width: f32,
    pub(super) image_height: f32,
    pub(super) image_stroke_color: Option<[u8; 3]>,
    pub(super) image_stroke_width: f32,
    pub(super) image_shadow: Option<crate::model::ImageShadow>,
    pub(super) image_shadow_xobj: Option<String>,
    pub(super) image_glow: Option<crate::model::ImageGlow>,
    pub(super) image_glow_xobj: Option<String>,
    pub(super) image_clip: Option<crate::model::ShapeGeometry>,
    pub(super) content_height: f32,
    pub(super) paragraph_mark_vanish: bool,
    pub(super) floating_images: Vec<CellFloatingImageLayout>,
    pub(super) space_after: f32,
    pub(super) has_textboxes: bool,
    pub(super) has_connectors: bool,
}

pub(super) enum CellContentItem {
    Paragraph(CellParagraphLayout),
    /// A table inside the cell, laid out (and drawn) at `col_widths`. A row
    /// split breaks between its rows (the cursor's `line` counts nested rows)
    /// or inside one that can split (`CellCursor::nested`).
    NestedTable {
        col_widths: Vec<f32>,
        rows: Vec<RowLayout>,
    },
}

pub(super) struct CellLayout {
    pub(super) items: Vec<CellContentItem>,
    /// The cell's margins (its own or the table's), border bands included.
    pub(super) cm: CellMargins,
    pub(super) total_height: f32,
    /// The space after the last paragraph that `total_height` includes; the
    /// chunk of a split row that finishes the cell charges it too.
    pub(super) trailing_space_after: f32,
    pub(super) text_direction: TextDirection,
}

pub(super) struct RowLayout {
    pub(super) height: f32,
    pub(super) cells: Vec<CellLayout>,
    /// Word breaks a row across pages unless it is cantSplit or has an
    /// explicit trHeight.
    pub(super) can_split: bool,
}

/// When provided, field codes in header/footer table runs are substituted with
/// their resolved values before layout.
pub(super) struct HfSubstitution<'a> {
    pub(super) page_num: usize,
    pub(super) total_pages: usize,
    pub(super) styleref_values: &'a HashMap<String, String>,
    pub(super) page_num_format: Option<&'a str>,
}

pub(super) fn compute_row_layouts(
    table: &Table,
    col_widths: &[f32],
    ctx: &RenderContext,
    hf_sub: Option<&HfSubstitution>,
) -> Vec<RowLayout> {
    let cm = &table.cell_margins;
    let mut layouts: Vec<RowLayout> = table
        .rows
        .iter()
        .map(|row| {
            let mut max_h: f32 = 0.0;
            let cells: Vec<CellLayout> = row
                .grid_cells()
                .map(|(grid_col, span, cell)| {
                    let span_w = cell_span_width(col_widths, grid_col, span);
                    // For auto-fit tables the resolved grid width is what the
                    // renderer draws borders and content at, so the layout must
                    // use the same width — a larger tcW preference otherwise wraps
                    // text past the drawn cell border (#115). Fixed-layout tables
                    // keep honoring the cell's preferred width (their gridCol can
                    // be narrower than Word's effective column).
                    let col_w = if table.fixed_layout {
                        span_w.max(cell.width)
                    } else {
                        span_w
                    };
                    if cell.v_merge == VMerge::Continue {
                        return CellLayout {
                            items: vec![],
                            cm: *cm,
                            total_height: 14.4,
                            trailing_space_after: 0.0,
                            text_direction: TextDirection::LrTb,
                        };
                    }

                    let ecm = cell.cell_margins.as_ref().unwrap_or(cm);
                    let is_rotated = cell.text_direction != TextDirection::LrTb;
                    let cell_text_w = if is_rotated {
                        10000.0
                    } else {
                        (col_w - ecm.left - ecm.right).max(0.0)
                    };
                    let mut total_h: f32 = ecm.top + ecm.bottom;
                    let mut max_rotated_line_w: f32 = 0.0;
                    let mut items: Vec<CellContentItem> = Vec::new();
                    let mut prev_space_after = 0.0f32;
                    let mut para_idx = 0usize;
                    let mut prev_was_nested_table = false;

                    let block_count = cell.content.len();
                    for (block_idx, block) in cell.content.iter().enumerate() {
                        match block {
                            Block::Paragraph(para) => {
                                let substituted;
                                let runs = if let Some(sub) = hf_sub {
                                    substituted = substitute_hf_runs(
                                        &para.runs,
                                        sub.page_num,
                                        sub.total_pages,
                                        sub.styleref_values,
                                        sub.page_num_format,
                                    );
                                    &substituted
                                } else if let Some(marked) = ctx.with_note_marks(&para.runs) {
                                    // A note reference mark's run is empty until it
                                    // shows its note's number, as in body paragraphs.
                                    substituted = marked;
                                    &substituted
                                } else {
                                    &para.runs
                                };
                                // Size the cell line from the tallest run, not the
                                // first — a small leading run (e.g. padding spaces)
                                // must not pull the baseline up. Math runs are
                                // clamped inside tallest_run_metrics so a header
                                // cell leading with math doesn't balloon the row.
                                // Cell metrics come from the first run carrying
                                // real text — a leading padding-spaces run must
                                // not set the baseline (Word sizes the line from
                                // the content run; observed in header cells like
                                // "                g" where g is larger).
                                let metric_run = runs
                                    .iter()
                                    .find(|r| {
                                        r.is_tab
                                            || r.text.is_empty()
                                            || !r.text.trim().is_empty()
                                    })
                                    .or(runs.first());
                                let font_size = metric_run.map_or(12.0, |r| r.font_size);
                                // A math run uses a math font (e.g. Cambria Math)
                                // whose tall ascent/descent must not set the cell
                                // line height (mirrors the is_math clamp in
                                // tallest_run_metrics) — otherwise a header cell
                                // whose metric run is math balloons the row.
                                let mut kb0 = String::new();
                                let metric_font = metric_run
                                    .filter(|r| !r.is_math)
                                    .map(|r| font_key_buf(r, &mut kb0).to_owned())
                                    .and_then(|k| ctx.fonts.get(&k));
                                let (tallest_lhr, tallest_ar) = metric_run
                                    .zip(metric_font)
                                    .map_or((None, None), |(r, e)| run_line_metrics(e, &r.text));
                                let effective_ls =
                                    para.line_spacing.unwrap_or(ctx.doc_line_spacing);
                                let line_h =
                                    resolve_line_h(effective_ls, font_size, tallest_lhr);
                                // Auto-spaced cell lines snap to the section's line grid
                                // like body lines: japanese_medical's ten empty cell
                                // paragraphs step 18pt (the grid), not their 15pt
                                // natural height; chinese_student's at-least-0 cell
                                // lines keep their own height.
                                let grid_pitch = ctx.cell_grid_pitch.get();
                                let grid_snapped = para.snap_to_grid
                                    && grid_pitch > 0.0
                                    && matches!(effective_ls, crate::model::LineSpacing::Auto(_));
                                let line_h = if grid_snapped {
                                    super::layout::grid_snapped_line_h(
                                        runs,
                                        ctx.fonts,
                                        effective_ls,
                                        line_h,
                                        grid_pitch,
                                    )
                                } else {
                                    line_h
                                };

                                // A numbering label taller than the text raises the
                                // first line (see `label_boosted_line_h`); CV's 9pt
                                // Symbol bullets on 9pt Georgia add 0.8pt per item.
                                // ponytail: folded into space_before, which equals the
                                // baseline drop at single spacing; split them if a
                                // multiple-spaced list cell ever drifts.
                                let label_extra = super::label_boosted_line_h(
                                    para,
                                    ctx.fonts,
                                    line_h,
                                    effective_ls,
                                    font_size,
                                    tallest_lhr,
                                    tallest_ar,
                                ) - line_h;
                                // A cell drops HTML auto spacing at its edges.
                                let space_before = if para_idx > 0 {
                                    f32::max(prev_space_after, para.space_before)
                                } else if para.space_before_auto {
                                    0.0
                                } else {
                                    para.space_before
                                } + label_extra;
                                total_h += space_before;

                                // The first baseline sits this far (per em) below the
                                // cell top: the ascent, line gap included, as in body
                                // text; one em for an East Asian font, whose 1.3×
                                // leading Word keeps out of the cell's top
                                // (japanese_interlibrary: 11pt MS Mincho 11.0).
                                let east_asian = metric_run
                                    .zip(metric_font)
                                    .is_some_and(|(r, e)| east_asian_leading(e, &r.text));
                                let ascender_ratio =
                                    if east_asian { 1.0 } else { tallest_ar.unwrap_or(0.75) };
                                // Win-path metrics identity: line_h_ratio −
                                // ascender_ratio = usWinDescent/units. Fallback
                                // 0.2 pairs with the 1.2 default line ratio so
                                // standard fonts get zero trailing leading.
                                let descender_ratio = tallest_lhr
                                    .zip(tallest_ar)
                                    .map(|(lh, ar)| (lh - ar).max(0.0))
                                    .unwrap_or(0.2);
                                let font_substituted =
                                    metric_font.is_some_and(|e| e.is_substituted);

                                // Compute extra left indent from left-aligned
                                // wrapSquare/Tight floating images so text wraps
                                // to the right of the image within the cell.
                                let float_indent_left: f32 = para
                                    .floating_images
                                    .iter()
                                    .filter(|fi| {
                                        matches!(
                                            fi.wrap_type,
                                            WrapType::Square | WrapType::Tight | WrapType::Through
                                        ) && matches!(
                                            fi.h_position,
                                            HorizontalPosition::AlignLeft
                                                | HorizontalPosition::Offset(_)
                                        )
                                    })
                                    .map(|fi| {
                                        let left_edge = match fi.h_position {
                                            HorizontalPosition::Offset(o) => o,
                                            _ => 0.0,
                                        };
                                        left_edge + fi.image.display_width + fi.dist_right
                                    })
                                    .fold(0.0f32, f32::max);

                                // An inline-image paragraph whose only other run
                                // is a trailing line break (Word's logo-in-cell
                                // idiom) is NOT text-empty, but its row height must
                                // come from the image's content_height, not a stray
                                // text line — otherwise the cell collapses and a
                                // tall header logo fails to push the body down.
                                let image_only = para.content_height > 0.0
                                    && runs.iter().all(|r| r.text.is_empty() && !r.is_tab);
                                let lines = if !is_text_empty(runs) && !image_only {
                                    let para_text_w = (cell_text_w
                                        - para.indent_left
                                        - para.indent_right
                                        - float_indent_left)
                                        .max(0.0);
                                    // Match the rendering's first_line_hanging: when a
                                    // list label is present, the label is drawn separately
                                    // and the text starts at indent_left, so the first
                                    // line has no extra hanging width.
                                    let hanging = if !para.list_label.is_empty() {
                                        if para.indent_first_line > 0.0
                                            && para.indent_hanging == 0.0
                                        {
                                            -para.indent_first_line
                                        } else {
                                            0.0
                                        }
                                    } else {
                                        para.indent_hanging
                                    };
                                    let has_tabs = runs.iter().any(|r| r.is_tab);
                                    let mut lines = if has_tabs {
                                        build_tabbed_line(
                                            runs,
                                            ctx.fonts,
                                            &para.tab_stops,
                                            para.indent_left,
                                            para_text_w,
                                            para.indent_right,
                                            hanging,
                                            &EMPTY_INLINE_IMAGE_MAP,
                                            &EMPTY_EFFECT_MAP,
                                            ctx.default_tab_stop,
                                            &[],
                                        )
                                    } else {
                                        build_paragraph_lines(
                                            runs,
                                            ctx.fonts,
                                            para_text_w,
                                            hanging,
                                            &EMPTY_INLINE_IMAGE_MAP,
                                            &EMPTY_EFFECT_MAP,
                                            None,
                                            None,
                                            None,
                                            ctx.cjk(para.auto_space_de || para.auto_space_dn, para.alignment),
                                        )
                                    };
                                    if is_rotated {
                                        for line in &lines {
                                            max_rotated_line_w =
                                                max_rotated_line_w.max(line.total_width);
                                        }
                                    }
                                    // Each line is as tall as its own runs, as in body
                                    // text: nabl's "(Mark √ in the" header line, √ a
                                    // w:sym Symbol run, steps 12.24 where Arial gives 11.50.
                                    if !east_asian && !grid_snapped && !matches!(effective_ls, crate::model::LineSpacing::Exact(_)) {
                                        super::layout::size_lines_by_own_runs(
                                            &mut lines,
                                            ctx.fonts,
                                            effective_ls,
                                            line_h,
                                            font_size * ascender_ratio,
                                        );
                                    }
                                    total_h += lines.iter().map(|l| l.pitch.unwrap_or(line_h)).sum::<f32>();
                                    lines
                                } else {
                                    if para.paragraph_mark_vanish {
                                        // vanished paragraph mark: zero height
                                    } else if cell.hide_mark
                                        && block_idx == block_count - 1
                                    {
                                        // hideMark: last empty paragraph in cell
                                        // contributes no height
                                    } else if prev_was_nested_table
                                        && block_idx == block_count - 1
                                        && para.content_height == 0.0
                                    {
                                        // End-of-cell mark directly after a nested
                                        // table: Word hides it; space_after is
                                        // suppressed in the trailing-space block
                                        // below.
                                    } else if para.content_height > 0.0 {
                                        // Image paragraph: the image is line 1; each
                                        // trailing w:br adds a further blank line
                                        // (matches the non-table header height path).
                                        let br_count = runs
                                            .iter()
                                            .filter(|r| r.is_line_break)
                                            .count();
                                        total_h += para.content_height
                                            + br_count as f32 * line_h;
                                    } else {
                                        total_h += line_h;
                                    }
                                    vec![]
                                };

                                let first_run_font_key = runs
                                    .first()
                                    .map(|r| {
                                        let mut kb2 = String::new();
                                        font_key_buf(r, &mut kb2).to_owned()
                                    })
                                    .unwrap_or_default();

                                let image_name = para.image.as_ref().and_then(|img| {
                                    let key = std::sync::Arc::as_ptr(&img.data) as usize;
                                    ctx.table_cell_image_names.get(&key).cloned()
                                });
                                let (image_width, image_height, img_stroke_color, img_stroke_width, img_shadow) = para
                                    .image
                                    .as_ref()
                                    .map(|img| (img.display_width, img.display_height, img.stroke_color, img.stroke_width, img.shadow.clone()))
                                    .unwrap_or((0.0, 0.0, None, 0.0, None));
                                let table_fx = para.image.as_ref().and_then(|img| {
                                    let key = std::sync::Arc::as_ptr(&img.data) as usize;
                                    ctx.effect_table_names.get(&key)
                                });
                                let img_shadow_xobj = table_fx.and_then(|fx| fx.shadow.clone());
                                let img_glow = para.image.as_ref().and_then(|img| img.glow.clone());
                                let img_glow_xobj = table_fx.and_then(|fx| fx.glow.clone());

                                let cell_floats: Vec<CellFloatingImageLayout> = para
                                    .floating_images
                                    .iter()
                                    .filter_map(|fi| {
                                        let key =
                                            std::sync::Arc::as_ptr(&fi.image.data) as usize;
                                        let pdf_name =
                                            ctx.table_cell_image_names.get(&key)?.clone();
                                        let h_offset = match fi.h_position {
                                            HorizontalPosition::Offset(o) => o,
                                            HorizontalPosition::AlignCenter => {
                                                (col_w - fi.image.display_width) / 2.0
                                            }
                                            HorizontalPosition::AlignRight => {
                                                col_w - fi.image.display_width
                                            }
                                            HorizontalPosition::AlignLeft => 0.0,
                                        };
                                        let v_offset = match fi.v_position {
                                            VerticalPosition::Offset(o) => o,
                                            _ => 0.0,
                                        };
                                        Some(CellFloatingImageLayout {
                                            pdf_name,
                                            display_width: fi.image.display_width,
                                            display_height: fi.image.display_height,
                                            h_offset,
                                            v_offset,
                                            rotation_deg: fi.image.rotation_deg,
                                            z_index: fi.z_index,
                                            alt: fi.image.alt.clone(),
                                            decorative: fi.image.decorative,
                                        })
                                    })
                                    .collect();

                                items.push(CellContentItem::Paragraph(CellParagraphLayout {
                                    lines,
                                    line_h,
                                    font_size,
                                    ascender_ratio,
                                    descender_ratio,
                                    font_substituted,
                                    alignment: para.alignment,
                                    space_before,
                                    indent_left: para.indent_left,
                                    indent_right: para.indent_right,
                                    indent_hanging: para.indent_hanging,
                                    indent_first_line: para.indent_first_line,
                                    float_indent_left,
                                    list_label: para.list_label.clone(),
                                    list_label_font: para.list_label_font.clone(),
                                    list_item: para.list_item,
                                    label_color: para.runs.first().and_then(|r| r.color),
                                    first_run_font_key,
                                    image_name,
                                    image_alt: para.image.as_ref().and_then(|i| i.alt.clone()),
                                    image_decorative: para
                                        .image
                                        .as_ref()
                                        .is_some_and(|i| i.decorative),
                                    image_width,
                                    image_height,
                                    image_stroke_color: img_stroke_color,
                                    image_stroke_width: img_stroke_width,
                                    image_shadow: img_shadow,
                                    image_shadow_xobj: img_shadow_xobj,
                                    image_glow: img_glow,
                                    image_glow_xobj: img_glow_xobj,
                                    image_clip: para.image.as_ref().and_then(|img| img.clip_geometry.clone()),
                                    content_height: para.content_height,
                                    paragraph_mark_vanish: para.paragraph_mark_vanish,
                                    floating_images: cell_floats,
                                    space_after: para.space_after,
                                    has_textboxes: !para.textboxes.is_empty(),
                                    has_connectors: !para.connectors.is_empty(),
                                }));

                                prev_space_after = if para.space_after_auto && block_idx == block_count - 1 {
                                    0.0
                                } else {
                                    para.space_after
                                };
                                prev_was_nested_table = false;
                                para_idx += 1;
                            }
                            Block::Table(nested_table) => {
                                // The widths the nested table is drawn at, so a split's
                                // line cursors index the lines it draws.
                                let mut nested_cw = auto_fit_columns(nested_table, ctx.fonts, Some(cell_text_w), None);
                                apply_pct_width(nested_table, &mut nested_cw, cell_text_w);
                                let nested_layouts =
                                    compute_row_layouts(nested_table, &nested_cw, ctx, hf_sub);
                                total_h += nested_layouts.iter().map(|rl| rl.height).sum::<f32>();
                                items.push(CellContentItem::NestedTable {
                                    col_widths: nested_cw,
                                    rows: nested_layouts,
                                });
                                prev_space_after = 0.0;
                                prev_was_nested_table = true;
                                para_idx += 1;
                            }
                        }
                    }

                    // When a cell ends with a nested table plus the mandatory
                    // end-of-cell paragraph mark (empty, no text), Word does
                    // not count the trailing paragraph's space_after toward
                    // the row height — the mark glyph height and line_h are
                    // already suppressed above via prev_was_nested_table.
                    let trailing_mark_after_table = items.len() >= 2
                        && matches!(items.get(items.len() - 2), Some(CellContentItem::NestedTable { .. }))
                        && matches!(items.last(), Some(CellContentItem::Paragraph(p)) if p.lines.is_empty() && p.image_name.is_none() && p.floating_images.is_empty());
                    let trailing_space_after =
                        if trailing_mark_after_table { 0.0 } else { prev_space_after };
                    total_h += trailing_space_after;
                    if is_rotated {
                        total_h = ecm.top + ecm.bottom + max_rotated_line_w;
                    }
                    if cell.v_merge != VMerge::Restart {
                        max_h = max_h.max(total_h);
                    }
                    CellLayout {
                        items,
                        cm: *ecm,
                        total_height: total_h,
                        trailing_space_after,
                        text_direction: cell.text_direction,
                    }
                })
                .collect();

            // Border bands are already in the cell insets (docx::tables); the
            // 0.5pt once added here was Table Grid's border width.
            let content_h = max_h;
            // An at-least trHeight bounds the row between its border bands,
            // which sit on top of it: 0.5pt Table Grid rows step trHeight + 0.5
            // (belgian_youth 19.85 → 20.5, italian_evaluation 12.0 → 12.48,
            // japanese_interlibrary 21.25 → 21.77). An exact height is the
            // border-to-border pitch (case15: 36.0).
            let bands = row
                .cells
                .iter()
                .map(|c| (c.borders.top.band() + c.borders.bottom.band()) / 2.0)
                .fold(0.0f32, f32::max);
            let height = match (row.height, row.height_exact) {
                (Some(h), true) => h,
                (Some(h), false) => content_h.max(h + bands),
                _ => content_h,
            };


            RowLayout {
                height,
                cells,
                can_split: !row.cant_split && row.height.is_none(),
            }
        })
        .collect();

    // A merged cell taller than the rows it spans grows the last of them:
    // indonesian_school_admission_checklist's "NO." header (49.7pt of content)
    // makes the second row 29.28pt where its at-least trHeight asks 24.15.
    for (ri, row) in table.rows.iter().enumerate() {
        for (ci, (grid_col, _, cell)) in row.grid_cells().enumerate() {
            if cell.v_merge != VMerge::Restart {
                continue;
            }
            let mut last = ri;
            while table.rows.get(last + 1).is_some_and(|next| {
                next.grid_cells()
                    .any(|(c, _, n)| c == grid_col && n.v_merge == VMerge::Continue)
            }) {
                last += 1;
            }
            let spanned: f32 = layouts[ri..=last].iter().map(|l| l.height).sum();
            let overflow = layouts[ri].cells[ci].total_height - spanned;
            if overflow > 0.0
                && let Some(r) = (ri..=last).rev().find(|&r| !table.rows[r].height_exact)
            {
                layouts[r].height += overflow;
            }
        }
    }
    layouts
}

/// Pre-compute how much extra height each vMerge Restart cell spans beyond its own row.
/// Returns a map from (row_idx, grid_col) to the sum of Continue row heights below;
/// a Continue cell maps to the height of the span rows still below it, absent
/// on the last, so a row can tell whether the merged cell ends there.
pub(super) fn compute_merge_spans(
    table: &Table,
    row_layouts: &[RowLayout],
) -> HashMap<(usize, usize), f32> {
    // Build a grid index: vmerge_grid[row][grid_col] = VMerge value
    let max_cols = table
        .rows
        .iter()
        .map(|r| {
            r.grid_before
                + r.cells
                    .iter()
                    .map(|c| c.grid_span.max(1) as usize)
                    .sum::<usize>()
        })
        .max()
        .unwrap_or(0);
    let mut vmerge_grid: Vec<Vec<VMerge>> = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        let mut row_vmerge = vec![VMerge::None; max_cols];
        for (col, _, cell) in row.grid_cells() {
            if col < max_cols {
                row_vmerge[col] = cell.v_merge;
            }
        }
        vmerge_grid.push(row_vmerge);
    }

    let mut spans = HashMap::new();
    for (ri, row) in table.rows.iter().enumerate() {
        for (grid_col, _, cell) in row.grid_cells() {
            if cell.v_merge == VMerge::Restart {
                let end = (ri + 1..table.rows.len())
                    .find(|&next_ri| {
                        grid_col >= max_cols || vmerge_grid[next_ri][grid_col] != VMerge::Continue
                    })
                    .unwrap_or(table.rows.len());
                let mut below = 0.0f32;
                for next_ri in (ri + 1..end).rev() {
                    if below > 0.0 {
                        spans.insert((next_ri, grid_col), below);
                    }
                    below += row_layouts[next_ri].height;
                }
                if below > 0.0 {
                    spans.insert((ri, grid_col), below);
                }
            }
        }
    }
    spans
}

/// Position in a cell's content for row splitting: items before `item` are
/// emitted; when `line > 0`, paragraph `item` is emitted up to that line.
#[derive(Clone, Default, PartialEq, Debug)]
pub(super) struct CellCursor {
    pub(super) item: usize,
    pub(super) line: usize,
    /// Nested table `item` broken inside its row `line`: where each cell of
    /// that row stands (empty when the break is between rows).
    pub(super) nested: Vec<CellCursor>,
}

/// A cell's beginning, for borrowing where a cursor is missing.
pub(super) static CELL_START: CellCursor = CellCursor {
    item: 0,
    line: 0,
    nested: Vec::new(),
};

impl CellCursor {
    pub(super) fn at(item: usize, line: usize) -> Self {
        CellCursor {
            item,
            line,
            nested: Vec::new(),
        }
    }
}

impl CellContentItem {
    /// The item's full height (a paragraph's block, a nested table's rows).
    pub(super) fn height(&self) -> f32 {
        match self {
            CellContentItem::Paragraph(p) => para_block_height(p),
            CellContentItem::NestedTable { rows, .. } => rows.iter().map(|r| r.height).sum(),
        }
    }
}

/// One piece of a cell between two cursors: item `item`, its lines (or nested
/// rows) `l0..l1` (`l1` None = to the end), and for a nested table the row
/// cursors `from` continuing row `l0` partway and `to` ending row `l1` partway.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Chunk<'a> {
    pub(super) item: usize,
    pub(super) l0: usize,
    pub(super) l1: Option<usize>,
    pub(super) from: &'a [CellCursor],
    pub(super) to: &'a [CellCursor],
}

/// The pieces of a cell between two cursors.
pub(super) fn cursor_chunks<'a>(
    items: &'a [CellContentItem],
    start: &'a CellCursor,
    end: &'a CellCursor,
) -> impl Iterator<Item = Chunk<'a>> + 'a {
    let partial_end = end.line > 0 || !end.nested.is_empty();
    let last = if partial_end { end.item + 1 } else { end.item };
    (start.item..last.min(items.len())).map(move |pi| {
        let first = pi == start.item;
        let at_end = pi == end.item && partial_end;
        Chunk {
            item: pi,
            l0: if first { start.line } else { 0 },
            l1: at_end.then_some(end.line),
            from: if first { &start.nested } else { &[] },
            to: if at_end { &end.nested } else { &[] },
        }
    })
}

/// How a nested table's chunk draws: whole rows, or one row from `starts` to
/// `ends` (missing cursors: the cell's start, or its end).
pub(super) enum RowPiece<'a> {
    Rows(std::ops::Range<usize>),
    Partial {
        row: usize,
        starts: &'a [CellCursor],
        ends: &'a [CellCursor],
    },
}

/// A nested table chunk as row pieces: the rest of row `l0` when it continues
/// partway, the whole rows, then row `l1` up to `to` when it ends partway.
pub(super) fn row_pieces<'a>(n_rows: usize, c: &Chunk<'a>) -> Vec<RowPiece<'a>> {
    let end = c.l1.unwrap_or(n_rows).min(n_rows);
    let mut pieces = Vec::new();
    let mut r = c.l0;
    if !c.from.is_empty() && r < n_rows {
        let ends = if c.l1 == Some(r) { c.to } else { &[] };
        pieces.push(RowPiece::Partial {
            row: r,
            starts: c.from,
            ends,
        });
        r += 1;
    }
    if r < end {
        pieces.push(RowPiece::Rows(r..end));
    }
    if !c.to.is_empty() && end < n_rows && end >= r {
        pieces.push(RowPiece::Partial {
            row: end,
            starts: &[],
            ends: c.to,
        });
    }
    pieces
}

/// Height of a chunk; items without lines use their block height.
pub(super) fn item_chunk_height(item: &CellContentItem, c: &Chunk) -> f32 {
    match item {
        CellContentItem::Paragraph(p) if !p.lines.is_empty() => {
            cell_lines_h(p, c.l0..c.l1.unwrap_or(p.lines.len()))
        }
        CellContentItem::Paragraph(p) => para_block_height(p),
        CellContentItem::NestedTable { rows, .. } => row_pieces(rows.len(), c)
            .into_iter()
            .map(|piece| match piece {
                RowPiece::Rows(range) => rows[range].iter().map(|rl| rl.height).sum(),
                RowPiece::Partial { row, starts, ends } => {
                    partial_row_height(&rows[row], starts, ends)
                }
            })
            .sum(),
    }
}

/// Height of a row's piece from `starts` to `ends` (one cursor per cell; a
/// missing start is the cell's beginning, a missing end its finish): its
/// tallest cell's content.
pub(super) fn partial_row_height(
    layout: &RowLayout,
    starts: &[CellCursor],
    ends: &[CellCursor],
) -> f32 {
    let mut max_h: f32 = 0.0;
    for (ci, cell_layout) in layout.cells.iter().enumerate() {
        let cm = &cell_layout.cm;
        let start = starts.get(ci).unwrap_or(&CELL_START);
        let done = CellCursor::at(cell_layout.items.len(), 0);
        let end = ends.get(ci).unwrap_or(&done);
        let mut h = cm.top + cm.bottom;
        for c in cursor_chunks(&cell_layout.items, start, end) {
            let item = &cell_layout.items[c.item];
            h += chunk_space_before(item, c.item, start) + item_chunk_height(item, &c);
        }
        // The chunk that finishes the cell keeps its last paragraph's space
        // after, as an unsplit row does.
        if end.item >= cell_layout.items.len() {
            h += cell_layout.trailing_space_after;
        }
        max_h = max_h.max(h);
    }
    max_h
}

/// The paragraph's space_before as charged inside a chunk starting at
/// `start`: a paragraph continuing mid-way never repeats it, but one starting
/// a chunk whole keeps it like an unsplit row (croatian_grant's floating
/// "Važno!" box starts 6pt below its top border in Word; nabl's carried-over
/// "Remarks" paragraph keeps its 4pt).
pub(super) fn chunk_space_before(item: &CellContentItem, pi: usize, start: &CellCursor) -> f32 {
    match item {
        CellContentItem::Paragraph(p) if pi != start.item || start.line == 0 => p.space_before,
        _ => 0.0,
    }
}

/// Where the cell content from `start` must break to fit `available_h`.
/// Word breaks a row's paragraph between lines, keeping two lines on each
/// side (widow control; annotation #237: a 10-line cell paragraph moved whole
/// to the next page, leaving 90pt of the row empty). A paragraph that cannot
/// split that way moves whole. Always progresses by at least one item.
/// ponytail: widowControl is assumed on, not read from the paragraph.
pub(super) fn find_cell_split(
    cell: &CellLayout,
    start: &CellCursor,
    available_h: f32,
) -> CellCursor {
    let cm = &cell.cm;
    let done = CellCursor::at(cell.items.len(), 0);
    if start.item >= cell.items.len() {
        return done;
    }
    let mut h = cm.top + cm.bottom;
    for pi in start.item..cell.items.len() {
        let first = pi == start.item;
        let l0 = if first { start.line } else { 0 };
        let item = &cell.items[pi];
        let sb = chunk_space_before(item, pi, start);
        let from: &[CellCursor] = if first { &start.nested } else { &[] };
        let rest = Chunk {
            item: pi,
            l0,
            l1: None,
            from,
            to: &[],
        };
        let item_h = sb + item_chunk_height(item, &rest);
        // A paragraph fits only with its space after: nabl's "Remarks" row
        // moves its last (4pt after) paragraph to the next page in Word.
        let sa = match item {
            CellContentItem::Paragraph(p) => p.space_after,
            CellContentItem::NestedTable { .. } => 0.0,
        };
        if h + item_h + sa <= available_h {
            h += item_h;
            continue;
        }
        if let CellContentItem::Paragraph(p) = item {
            let remaining = p.lines.len().saturating_sub(l0);
            let mut used = h + sb;
            let room = p.lines[l0..]
                .iter()
                .take_while(|l| {
                    used += l.pitch.unwrap_or(p.line_h);
                    used <= available_h
                })
                .count();
            let fit = room.min(remaining.saturating_sub(2));
            if fit >= 2 {
                return CellCursor::at(pi, l0 + fit);
            }
        }
        // Word breaks a nested table between its rows (radiographer's
        // "Internal / External to the Trust" table starts on page 1), and
        // inside the first row that does not fit when that row may split, each
        // of its cells by these same rules (its "Administrative teams within
        // Radiology" closes page 1).
        if let CellContentItem::NestedTable { rows, .. } = item {
            let mut used = h + sb;
            let mut r = l0;
            // Only the first row can continue partway; the rest are whole.
            let mut row_start = from;
            while r < rows.len() {
                let rest = if row_start.is_empty() {
                    rows[r].height
                } else {
                    partial_row_height(&rows[r], row_start, &[])
                };
                if used + rest > available_h {
                    break;
                }
                used += rest;
                r += 1;
                row_start = &[];
            }
            if r < rows.len() && rows[r].can_split {
                let starts: Vec<CellCursor> = (0..rows[r].cells.len())
                    .map(|ci| row_start.get(ci).unwrap_or(&CELL_START).clone())
                    .collect();
                let ends: Vec<CellCursor> = rows[r]
                    .cells
                    .iter()
                    .zip(&starts)
                    .map(|(c, s)| find_cell_split(c, s, available_h - used))
                    .collect();
                if ends != starts
                    && used + partial_row_height(&rows[r], &starts, &ends) <= available_h
                {
                    return CellCursor {
                        item: pi,
                        line: r,
                        nested: ends,
                    };
                }
            }
            if r > l0 && r < rows.len() {
                return CellCursor::at(pi, r);
            }
        }
        if !first {
            return CellCursor::at(pi, 0);
        }
        // The first item is force-included so the split makes progress.
        h += item_h;
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    /// stem_partnerships p4 (annotation #237): a 10-line cell paragraph after a
    /// heading must break between lines, two lines minimum on either side.
    #[test]
    fn row_split_breaks_inside_a_long_paragraph() {
        let para = |n: usize| {
            CellContentItem::Paragraph(CellParagraphLayout {
                lines: (0..n).map(|_| TextLine::default()).collect(),
                line_h: 10.0,
                space_before: 5.0,
                ..Default::default()
            })
        };
        let cell = CellLayout {
            items: vec![para(1), para(10)],
            cm: CellMargins {
                top: 0.0,
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
            },
            total_height: 0.0,
            trailing_space_after: 0.0,
            text_direction: TextDirection::default(),
        };
        let split = |start, avail| find_cell_split(&cell, &start, avail);
        let at = CellCursor::at;

        // the heading's own space_before (5, the cell opens with it) + heading
        // (10) + space_before (5) + four of the ten lines
        assert_eq!(split(at(0, 0), 60.0), at(1, 4));
        // the remaining six lines fit, no space_before on a continuation
        assert_eq!(split(at(1, 4), 60.0), at(2, 0));
        // room for one line only: the paragraph moves whole
        assert_eq!(split(at(0, 0), 26.0), at(1, 0));
        // nine lines would fit but two must stay for the next page
        assert_eq!(split(at(0, 0), 110.0), at(1, 8));
        let chunks: Vec<_> = cursor_chunks(&cell.items, &at(0, 0), &at(1, 4))
            .map(|c| (c.item, c.l0, c.l1))
            .collect();
        assert_eq!(chunks, vec![(0, 0, None), (1, 0, Some(4))]);
    }

    #[test]
    fn test_cell_span_width_single() {
        let widths = vec![100.0, 200.0, 300.0];
        assert_eq!(cell_span_width(&widths, 0, 1), 100.0);
        assert_eq!(cell_span_width(&widths, 1, 1), 200.0);
        assert_eq!(cell_span_width(&widths, 2, 1), 300.0);
    }

    #[test]
    fn test_cell_span_width_multi() {
        let widths = vec![100.0, 200.0, 300.0];
        assert_eq!(cell_span_width(&widths, 0, 2), 300.0);
        assert_eq!(cell_span_width(&widths, 0, 3), 600.0);
        assert_eq!(cell_span_width(&widths, 1, 2), 500.0);
    }

    #[test]
    fn test_cell_span_width_clamps_to_len() {
        let widths = vec![100.0, 200.0];
        // span=5 but only 2 columns from index 0
        assert_eq!(cell_span_width(&widths, 0, 5), 300.0);
    }

    #[test]
    fn test_cell_span_width_start_past_end() {
        // Missing tblGrid (empty widths) or gridSpan overrun must not panic.
        assert_eq!(cell_span_width(&[], 1, 1), 0.0);
        let widths = vec![100.0, 200.0];
        assert_eq!(cell_span_width(&widths, 5, 2), 0.0);
    }

    #[test]
    fn test_cell_x_offset() {
        let widths = vec![100.0, 200.0, 300.0];
        assert_eq!(cell_x_offset(&widths, 50.0, 0), 50.0);
        assert_eq!(cell_x_offset(&widths, 50.0, 1), 150.0);
        assert_eq!(cell_x_offset(&widths, 50.0, 2), 350.0);
        assert_eq!(cell_x_offset(&widths, 50.0, 3), 650.0);
    }
}
