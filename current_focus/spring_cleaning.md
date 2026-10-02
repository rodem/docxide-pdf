# Spring cleaning (started 2026-10-02)

Branch `ccr-a9bd9fc4-3ejxi3`, 27 commits on top of `59a26ef`. Goal: less
code, better structure, same output. Every commit was gated by
`cargo build --all-features`, `cargo clippy --all-targets --all-features`,
`cargo fmt`, `cargo test --lib` and `cargo test --no-run`. The visual suite
was **not** run (no Word fonts in the cloud container): run
`tools/score_snapshot.sh` on a Mac against `main` before merging.

## Numbers

| | Rust lines (src, tests, tools/src) |
|---|---|
| base `59a26ef` | 62,487 |
| after `rustfmt` commit | 65,430 (+2,943, pure reflow) |
| after clippy fixes | 65,381 |
| now | 64,061 (**−1,320** of real code) |

clippy warnings 157 → 49; `#[allow(dead_code)]` 25 → 1 (the justified
crate-level one in `tests/common`).

## What changed

**Formatting / lint.** One-off `cargo fmt` of the whole tree (isolated
commit, droppable). `cargo clippy --fix`: collapsed ifs → let-chains,
needless borrows, `then_some`, `map_or`, `div_ceil`, redundant closures.

**Dead code deleted, not hidden.** Write-only model fields and the parsing
that fed them: `Document.auto_hyphenation`, `DocumentSettings.mirror_margins`
and `.auto_hyphenation`, `Paragraph/ParagraphStyle.suppress_auto_hyphens`,
`FontTableEntry.pitch_fixed`, `ChartAxis.delete`, `Textbox.is_wordart` /
`.dist_top`, `SmartArtDiagram.display_width/height`,
`ImageReflection.blur_radius`, the payload of `AutoFit::Normal`. In pdf:
`count_script_boundaries`, `_primary_gids`, `HfPageContext.effect_floating_names`,
the discarded `effect_hf_inline_names`, `CellFloatingImageLayout.behind_doc`,
`CellParagraphLayout.indent_right`, `CellLayout.total_height`,
`GlyphPath.advance_width`, a no-op connector match+clone. In geometry:
`PathFill::is_filled`, `EvaluatedPath.fill`, a stale allow on `text_rect`.
In tests: the no-op `ssim_comparison` test (and README's mention of it),
the commented-out timing block and its `*_ms` fields.

**docx / model.**
- `n.has_tag_name((NS, "x"))` replaces ~80 hand-written
  `name()==… && namespace()==…` checks; `is_wml`, `is_wpd_drawing`,
  `is_ns`, `has_dml`, `dml_children`/`chart_ns_children` wrappers gone.
- All namespace constants live in `docx/mod.rs` (`VML_NS_LOCAL` was defined
  four times in images.rs; `PIC_NS`, `OFFICE_NS` twice; `CHART_URI`==`CHART_NS`).
- `twips_attr`/`emu_attr_opt`/`frac_attr`/`angle_attr` for the ~50 hand-rolled
  attribute parses; `part_path`, `read_zip_bytes`; one anchor-attribute helper
  and `HorizontalPosition/VerticalPosition::offset_or_zero`.
- `ListCounters` replaces the `(counters, last_seen_level, applied_overrides)`
  `&mut` triple (`parse_list_info` 9 → 7 args).
- Table-style conditional formatting accumulates in one struct instead of a
  12-parameter closure called ten times.
- `Textbox`/`FloatingImage` built from `Default` + `From<WspResult>` instead
  of 28-field literals (8 sites).
- One `RunProps` parser for `w:rPr` (docDefaults, paragraph style, character
  style, inline run were four copies); `CharacterStyle` *was* that struct.
  `Run` is its own format template (`..self.clone()`), the 33-field
  `RunFormat` mirror and its copy-back are gone. `half_points` shared by
  `parse_font_size`/`parse_kern`.

**pdf.**
- `layout::build_lines` is the one tabbed-vs-plain dispatch (was 7 copies);
  empty placeholder maps are `LazyLock` statics.
- `color::stroke_segment`, `align_offset`, `HorizontalPosition::place`,
  `SectionProperties::text_width()`, `WrapType::wraps_beside()`,
  `EmbeddedImage::key()` replace 8–13 copies each.
- Charts: four bar-chart arms (col/bar × clustered/stacked/percent) → one with
  an orientation flag; pie + doughnut → one fn. Verified byte-identical on
  case29/30/31/52.
- `footnotes.rs` contextual-spacing rule → `helpers.rs`.
- Duplicate `split_row_across_pages` branches folded; `Decoration` type alias.

**fonts / geometry / tests / config.**
- One generic path-command type and one resolver for preset and custom
  geometry (`Cmd`, both `as_cmd`, duplicate `resolve_*`/`evaluate_*` gone).
- `register_font`: object refs and lookup context bundled (10 → fewer args).
- Visibility narrowed in fonts/, geometry/, cache.rs, tests/common.
- Test harness: `diff_pixel`, `ensure_screenshots`, `mutool_info`,
  `name_width`, regression filter, `mtime` shared via `tests/common`.
- `Cargo.toml`: `include` list instead of stale `exclude`; `image` dev-dep
  no longer pulls every decoder into test builds; duplicate `zip`/`roxmltree`
  dev-deps removed.
- CLAUDE.md: module tree, test layout (79 cases, groups), dependency
  versions, env vars corrected.

## Findings not acted on (behaviour divergences between copies)

Check these on a Mac with the visual suite before unifying anything near them:

1. `header_footer.rs` textbox renderer passes the **ascender ratio** to
   `resolve_line_h` where `textbox_render.rs` passes the **line-height
   ratio** in the same position.
2. In `textbox_render.rs` the anchor-offset pre-pass and the render pass use
   **different hanging-indent rules** — text is measured with one and drawn
   with the other. In all there are ~5 hanging-indent formulas
   (`mod.rs`, `textbox_render.rs` ×3, `header_footer.rs` ×2,
   `table_layout.rs`+`table.rs`).
3. Effect XObjects for header/footer inline and floating images are
   **embedded but never drawn** — shadows/glows on those images are missing.
4. `tools/src/bin/jaccard.rs` and `case_diff.rs` use float luma (`< 200.0`),
   `tests/common` integer luma: results can differ at the boundary.
5. `!(x >= 1.0)` in `mod.rs` (two wrap-side checks): clippy wants `<`, which
   differs only for NaN; left as is.
6. `Error::Pdf(String)` is never constructed; `pdf::render` is infallible.
   Public API, so a semver-breaking change — defer.
7. `tests/text_boundary.rs::text_boundaries_match` has no `#[test]` and
   never runs (SCORING.md already notes this).

## Not done (next rounds)

- **Parameter bundling** (the biggest structure win): `render_paragraph_block`
  20 args, `render_paragraph_lines` 18, `assemble_pdf_pages` 20,
  `render_textbox_paragraphs` 15, positioning.rs fns 12–13, table.rs cell/row
  renderers 9–13. A worker produced a clean 5-commit series doing exactly this
  (`RenderContext` gains per-document values; `Sink {content, gradient_specs,
  links}`; `LineInputs`/`RunStyle`; `Anchor {sp, col_x, col_w, text_width,
  slot_top}`; `LineGeometry` + `LineHooks` → `render_paragraph_lines` 18 → 5)
  but it was cut from `main`, not this branch, and conflicts with the
  formatted tree. It lives on local branch
  `worktree-agent-a69249f7b055f3538` (base `0783e5e`, HEAD `fef5a43`);
  rebase it onto this branch after merging `main`.
- Textbox paragraph layout written 4× (`textbox_height`, the anchor pre-pass,
  `render_textbox_paragraphs`, the ~240-line copy in `header_footer.rs`) —
  merge only the identical parts; see divergences 1–2.
- Picture pre/post effects (shadow, glow, clip, stroke, inner shadow,
  reflection) drawn the same way at 5 sites → `draw_picture_pre/post`.
- `table.rs` row renderers triplicated (`CellTagger` construction, border
  pass, vAlign maths); `render_cell_content` vs `render_partial_cell_content`.
- `PageBuilder`'s 11 parallel `all_*` Vecs → `Vec<FinishedPage>`, which also
  shrinks `assemble_pdf_pages`.
- Image-name maps re-indexed after the fact (`HfMaps` 5-tuple, per-paragraph
  re-filtering) → emit nested maps from `embed_all_images`.
- `resolve_header_for_page`/`resolve_footer_for_page` twins; six header/footer
  variants listed out 5× → `SectionProperties::headers_footers()`.
- `FontEntry` repeats every `FontMetrics` field as `Option` → `metrics:
  Option<FontMetrics>` (38 uses in pdf/).
- `geometry/definitions.rs` (15.4k lines) could be ~half the size if
  `generate_shapes.rs` emitted macros for the 270 `PathDef` literals and the
  2,915 `PathCommandDef::` prefixes.
- Three paragraph builders (`build_paragraph`, footnote `parse_para`, the
  cell builder inside the ~740-line `parse_table_node`) share indent merge,
  style lookup and tab-stop merge; `parse_table_node` wants splitting.
- VML `style=` parsing done 5×; `parse_notes_simple`/`_rich` share a skeleton;
  `handle_drawing_result!` macro's Group arm repeats the six outer arms.

## Housekeeping

`main` has moved ~50 commits past this branch's base; merge it first. The
rustfmt commit will make that merge noisier — dropping it and running
`cargo fmt` once after the merge is the cheaper order. Leftover local
worktrees `.claude/worktrees/agent-*` and one `git stash` entry can be
deleted.
