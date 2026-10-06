# Layout accuracy — current focus

Working document for the layout-accuracy round: where it stands, how to
measure, every rule implemented with its evidence, findings, open items and
decisions. `roadmap.md` → "Layout accuracy round (2026-10-01)" is the short
public summary; this file has the detail.

## 1. Status

- Branch `layout-accuracy` (worktree `.worktrees/layout-accuracy`), on top of
  `7319c3fe`. **Not merged — merge into `main` only when the user says so.**
  `main` has moved on since; expect conflicts in `roadmap.md` and possibly
  `src/pdf/*.rs`.
- 43 layout rules, one commit each, each verified by a full suite run and an
  external corpus run; plus a cleanup pass (`9cfb33a1`).
- Baselines accepted: scores for the 138 fixtures whose Jaccard improved
  (`10693fa8`, the 12 that dropped keep their old baseline), the new `case79`
  (`93642611`), and the visual hashes for all 154 changed fixtures
  (`1e66222c`).

| Fixtures (Jaccard) | start of round | now |
|---|---|---|
| cases (75, + case79) | 69.9 | **74.3** |
| scraped (53) | 39.9 | **57.7** |
| new (74) | 38.5 | **55.9** |
| samples (5) | 35.4 | **55.9** |
| hyphenation (8) | 61.2 | 63.2 |
| fonts (6) | 69.0 | 69.0 |

External corpus (258 Mac Word documents with Word's own PDF exports, kept
outside the repo): **45.5 → 60.3** Jaccard.

## 2. How to measure

All tools are in `tools/` (listed in `CLAUDE.md` → "Accuracy work").

```bash
tools/score_snapshot.sh fixN fixN-1          # full suite → tests/output/snapshots/fixN.json + comparison
python3 tools/compare_scores.py a.json b.json  # group means + every moved case
cp target/release/docxide-pdf <scratch>/docxide-before   # keep the old build, then after the change:
VDIFF=<vdiff> python3 tools/ab_view.py before <scratch>/docxide-before scraped/some_case
VDIFF=<vdiff> python3 tools/ab_view.py after  target/release/docxide-pdf scraped/some_case
python3 tools/convert_scan.py                # find hangs/crashes before a suite run

# external corpus: <dir>/docx/<name>.docx + <dir>/pdf/<name>.pdf
cd tools && cargo build --release --bin page-metrics && cd ..
python3 tools/corpus_score.py <dir> <cli> <label>   # → tests/output/corpus/<label>.json
python3 tools/corpus_compare.py <before> <after>
VDIFF=<vdiff> python3 tools/corpus_view.py <dir> <label> <cli> <name-prefix> -v --score
```

Diagnostics: `pdf_lines.py` (border rules + baselines), `line_diff.py` (where
reflow starts), `word_x_diff.py` (horizontal drift), `ink_diff.py` (diff
image), `docx_edit.py` (what-if edits), `experiments/squeeze_rule.py`,
`experiments/url_line_ends.py`.

**vdiff** (per-line baseline drift, the most useful tool here) lives only on
branch `systemic_work` (`tools/src/bin/vdiff.rs`); a built binary is at
`tools/target/debug/vdiff` in the main checkout. Pass it via `VDIFF=...`.

Corpus data, census scripts and per-fix snapshots from the round are local
(`accuracy_push_local/`, untracked).

## 3. Working rules and gotchas

- **Derive rules, never fit one document.** Measure Word's PDFs (`mutool draw
  -F trace/stext`), check a candidate rule on all fixture groups *and* the
  corpus, commit only net-positive changes and explain every regression. When
  a rule breaks another document, look for the more general rule (the
  keep-with-next story in §5). Document names in comments are examples, never
  conditions.
- One fix per commit, each verified by a full suite run; snapshot scores
  before and after (`score_snapshot.sh`).
- Query the OOXML spec RAG before implementing
  (`mcp__local-rag__query_documents`; it was disconnected for parts of the
  round).
- Never accept baselines without the user's approval (scores and hashes are
  separate approvals). Word conversions are the user's job — give them the
  command.
- Worktrees need `fonts -> ../../fonts` (gitignored) or every score collapses.
- Always `DOCXSIDE_NO_FONT_CACHE=1` in a worktree (the font index cache is
  global in `~/Library/Caches`).
- The CLI never overwrites: delete the output PDF first.
- Don't edit `src/` while the suite is compiling (wait until the
  `visual_comparison-*` test binary runs).
- Other agents share the machine; a ~6 min suite can take >30. Run long jobs in
  the background with a long timeout.
- In this setup cargo, the suite and writes inside the worktree need the
  sandbox off. System `python3` is 3.9 (no `X | None` without
  `from __future__ import annotations`).
- Jaccard at 150 DPI punishes 1–2pt vertical offsets hard and is
  non-monotonic: a document 13pt off can score higher than the same document
  6pt off. Read vdiff before calling something a regression.
- This branch's `accept-baselines` has no `--scores-only`/`--hashes-only` and
  writes both files unless `--dry-run` (an unknown flag like `--help` does not
  stop it).

## 4. Rules implemented (commit order)

Short hashes are on `layout-accuracy`. Evidence is Jaccard before → after;
"corpus" means a document of the external corpus.

| # | Commit | Rule | Evidence |
|---|---|---|---|
| 1 | ecaa906e | Parse `compatibilityMode`; compat-15 tables sit at margin + tblInd (no cell-margin outdent) | |
| 2 | 9fe64c00 | Header float with negative paragraph-relative offset counts toward header extent | |
| 3 | abf6664c | Table-cell list lines get the marker-ascent boost | |
| 4 | 0820f568 | **Border bands**: cell content between horizontal bands (old +0.5pt/row fudge was Table Grid's border width) | 50 fixtures up |
| 5 | 435e9c1e | Justification spreads slack over word spaces only | |
| 6 | a66bb28b | **Justified squeeze (compat 15)**: spaces shrink to ≥75%, word kept if its midpoint is inside the margin | 94.6% of 14,122 measured decisions |
| 7 | a449d6bf | **Per-line heights** from each line's own runs (run-border pads, math excluded) | |
| 8 | 01ef7fb5 | Compat-15 table left border band starts at the indent | |
| 9 | 5dcebc26 | `w:kern="0"` = kerning off | |
| 10 | a19ae40a | Paragraph top border band inside the paragraph | |
| 11 | 271baa31 | Line = max ascent + max descent across runs; sub/superscript offsets don't grow it | |
| 12 | 968b1a13 | docDefaults without pPrDefault → Word defaults (8pt after, 278) | corpus 7 → 82 |
| 13 | ad903bda | HTML auto spacing = 14pt; not at doc start, not within one list, not at cell edges | |
| 14 | 42719453 | At-least trHeight bounds content between border bands | |
| 15 | 3af666ab | Negative pgMar top/bottom = fixed absolute margin (§17.6.11) | |
| 16 | 9535d152 | Picture paragraph gets its own mark's line-spacing leading | |
| 17 | c3b1ff5c | Space-only/break-only line sized by its break run, else the paragraph mark (mark font inherits style → defaults, theme-resolved) | eco_int 24 → 52 |
| 18 | f2cf1267 | Leading spaces indent an inline picture | russian_chess +7.6, family_kinship +15.4 |
| 19 | 6f70858f | `w:shd` solid/pctNN paint colour over fill | corpus 24 → 99 |
| 20 | 694a8ba1 | Empty paragraph's synthetic run takes the mark's font even without w:sz | corpus 8 → 94 |
| 21 | 586725ec | A tab never raises its line | mandated_reporter 46 → 79 |
| 22 | 8ad39222 | No beforeAutospacing at the start of a header/footer | |
| 23 | 96915560 | `w:cr` is a line break | |
| 24 | 59510dbc | **`w:linkStyles` without attachedTemplate → stock Normal.dotm** (docDefaults 12pt/8pt/278, empty Normal) | corpus 8 → 59, 10 → 83 |
| 25 | 7d43581a | `doNotExpandShiftReturn` (spec setting) | no fixture change |
| 26 | f953f7e9 | GIF/TIFF transcoded to PNG | corpus 6 → 85 |
| 27 | 3878e433 | A word wider than the line breaks at the margin | corpus 14 → 67 |
| 28 | 6ef7eb74 | URLs wrap only after `-` or at the margin (old `/ ? # & = ;` breaks removed) | 102 measured URL ends, 2 after `/` |
| 29 | 76fc62c0 | Float reach tested against the first line top (slot − gap) | corpus 12 → 68 |
| 30 | d4bb3001 | Style's own ind beats the numbering it carries when the ind is on/below the numPr style (§17.7.2) | corpus 28 → 70 |
| 31 | cc82aa21 | Empty paragraph's synthetic run marked inheriting (table style sizes apply) | corpus 25 → 83 |
| 32 | 5f75a262 | `w:br` directly under `w:p` breaks the line | |
| 33 | b5a8f2d2 | **Keep-with-next** reserves what widow control keeps together: ≤3-line paragraph whole, else 2, no widow control 1 | lithuanian 33 → 74, western_australia 38 → 50 |
| 34 | 5761c52f | **contextualSpacing** only beside a same-style paragraph (§17.3.1.9) | corpus 6 → 66, +35/+39 on others |
| 35 | cb93ffc6 | Table-style bold/italic never overrides direct `w:b`/`w:i` (§17.7.3) | physical_therapy 12.6 → 19.1 |
| 36 | 8dbc1134 | **docGrid lines**: centre the win glyph box in the snapped cells; Latin fonts add their hhea lineGap above, East Asian fonts don't (`grid_baseline_shift`) | taiwanese 16.3 → 49.4, physical_therapy 19.1 → 46.1, tokyo 24.3 → 34.8, interlibrary 59.1 → 63.0 |
| 37 | 4a4833f7 | A run boundary breaks iff the two characters would break inside one run; a run-spanning word that overflows moves whole (after the squeeze test) | slovak_pedagogical 34.4 → 48.3, 11 others up |
| 38 | cee2d5a3 | An over-wide word breaks at the margin of the line it wraps to | polish_tender 39.8 → 73.3; corpus +10.2/+7.8 |
| 39 | 19fbb65f | Small caps at 80% (the old −2pt was never measured) | italian_project 11.2 → 28.7 (one doc) |
| 40 | 216e051f | Footnote contextualSpacing same-style (like 34) | no change — see §6, footnotes never carry the flag |
| 41 | eada8244 | **Cell first baseline = ascent (+gap) as in body text**; East Asian fonts keep 1.0 em | 31 fixtures up (who_prescribing +21.8, italian_project +18.1, physical_therapy +16.5, CV +12.2); corpus +0.9 |
| 42 | c014389e | docGrid multiples: cells(glyph height) × multiple, not cells(height × multiple) | case79 68.4 → 72.0, all 36 lines within 0.4pt |
| 43 | b7226387 | macOS faces with only Mac Roman family names indexed in a lower tier (vendored Microsoft Symbol still wins); "Times"/"Courier" → Times New Roman / Courier New first | fixtures unchanged; corpus 59.41 → 59.60 (one doc +40.7) |

Regressions and what explained them:
- 36: a Chinese corpus doc −2.6 — our empty paragraph takes 4 cells where
  Word takes 3; the centring rule itself fits that doc.
- 37: uk_commercial needed the squeeze test for glued words (footnote marks).
- 38: italian_project −15.6, fixed by 39 (small caps width); a vMerge test doc
  −4.2 (our column ~8pt vs Word's ~31pt).
- 41: slovak_eu (rows ~0.03pt short each), a corpus doc with a doubled top
  border, case61 (cells now match its own body offset) — the old em offset
  was cancelling other table errors. CJK cells first regressed until East
  Asian fonts kept the em.
- 42: a corpus doc −0.8 whose title wraps in ours (lines below already 60pt
  off).
- 43: a Helvetica Neue doc −14.3 with a pre-existing 48pt top offset; its
  fonts are now right.

## 5. Findings

- **Overfitting story (33):** a first version ("an empty next paragraph needs
  one line") fixed lithuanian but broke western_australia; the general
  widow/orphan rule fixed both. Line counts must use the body measure: a
  hanging label tabs its first line to the indent.
- **34 was found from a regression:** 33 exposed a corpus doc's page 1 running
  33pt high; the cause was the wrong contextualSpacing rule.
- **docGrid placement (36, 42), confirmed by `case79`** (25 fonts and sizes on
  18pt and 15.6pt grids, converted with Word): 34 single-spaced lines within
  0.4pt (Latin mean +0.00; East Asian lines sit 0.17pt lower in Word on
  average — unexplained). Earlier samples: TNR 12 on 18 → 13.63; MS Gothic 16
  on 36 → 23.67; YaHei 16 on 36 → 24.53; Yu Mincho 10.5 on 18 → 12.74 (its
  0.5 em gap decided "no gap for East Asian fonts"). Multiples below 1 and
  at-least spacing on a grid are untested.
- **case3** has docGrid without `w:type`, so no grid. Its gap is heading
  kerning (Word kerns T-e in Aptos Display by −1.54pt; GPOS class pair) and
  list line heights; kerning is the known TJ dead end.
- **East Asian leading** looks split half above/half below in headers
  (japanese_medical: 11pt Yu Mincho baseline 13.23 below the header top ≈
  0.15·W + winAsc) and cells use 1.0 em; the body's "all leading above" may
  only have held for grid documents. Untested for body text.
- **Mac Word fonts:** Mac Word draws its own Microsoft Symbol, not Apple's;
  for Times New Roman it uses the installed macOS face (hhea lineGap 87 →
  13.8pt at 12pt). It never draws Apple's Times (two Mac docs → Times New
  Roman; two Windows docs name a non-embedded "Times", Windows' substitute).
  Helvetica needs no special line height (body lines step as in Word once
  indexed). Helvetica Neue used to resolve to its Thin face, the only one
  with a Unicode name.
- **Office cloud fonts:** Jokerman, Source Sans Pro, Segoe UI Black and
  Script MT Bold are in Word's cloud catalog; Word downloads them into
  `~/Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts/` when a
  document names them. Copied into `fonts/CloudFonts/` and the private
  assets repo (`59143d9`, 2026-10-02): corpus docs 8.8 → 92.4, 17.1 → 79.8.
- **fontTable altName order:** we try `w:altName` before the requested name
  (since fcb84c7e, for a Korean localized name "바탕"). Word uses altName only
  when the font is missing: a Source Sans Pro doc with altName Corbel is drawn
  in Source Sans Pro by Word, in Corbel by us. Fixed in `f4ef46b8` (§7).
- **romanian_quality header row:** Word makes it 67.5pt = trHeight 60.2 + both
  3.6pt cell margins, though the content is shorter (we give it 60.2). One
  sample; the usual reading is that an at-least height includes the margins.
  Its page 2 −12pt is a knock-on (−8.3 on page 1).
- **Tracked changes:** Word's export shows insertions underlined and deletions
  struck through in the author's colour, deleted text still laid out, change
  bars in the margin. We render the final text; on 100 tracked-changes
  documents that scores ≈ 25 Jaccard, page count wrong on 36. On the roadmap
  ("Tracked-Changes (Redline) Rendering").
- Space-only lines: "spaces size nothing, breaks size the line they end, else
  the paragraph mark" (17). Whether the mark counts on a line with text is
  untested.
- `overrideTableStyleFontSizeAndJustification`: our "Normal's size beats the
  table style's" matches compat 15; the legacy rule (<15 without the flag:
  table style size wins unless 10pt) is not implemented.

## 6. Cleanup review (`/simplify`, `9cfb33a1`)

Reviewed everything since the previous simplify (`2c0706d8`, 63 commits) from
four angles: reuse, simplification, efficiency, altitude. Fixture scores
unchanged.

**Fixed**
- One `drops_contextual_spacing` helper for the same-style rule (8 sites).
- `FloatZone::narrow_paragraph` replaces three copies of the float-zone
  narrowing block (and the unused `narrow_paragraph_geometry`); it narrows the
  caller's box in place, so the second copy's behaviour on an already-narrowed
  box is kept.
- `breaks_between` tests run-boundary breaks without allocating (it was a
  `format!` + split per glued word); a test checks it against the in-run
  splitter on 289 pairs.
- Keep-with-next look-ahead only off the page top, and the next paragraph is
  laid out only when widow control needs its line count; the grid baseline
  offset only for grid paragraphs.
- Explicit `FontEntry::east_asian` (`east_asian_leading`) instead of comparing
  derived ascender ratios; one `sizes_line` predicate.
- Textbox images no longer embed shadow/glow XObjects that nothing draws.
- Shared `anchored_frame_top` for header frames; `theme_minor_font` replaces
  `chart_font_name` (it now also drives font substitution); `parse_on_off` for
  default styles; one width closure for the over-wide word cut;
  `compute_text_hanging`'s doc comment back in place.

**Not fixed** (behaviour change or well outside the diff):
- Cell and footnote paragraphs are built by hand (`docx/tables.rs`,
  `headers_footers::parse_notes_simple`), not by `paragraph::build_paragraph`:
  cells never get `space_before_auto`/`space_after_auto` (13's cell-edge drop
  never fires), footnotes never get `style_id`/`contextual_spacing` (40 is a
  no-op there), cells also miss `widow_control`, `ind_over_numbering`, the
  mark font, paragraph shading and borders. Correctness fix; needs its own
  suite run. (On the roadmap.)
- One per-line height model for body, cells and headers (per-line `pitch` is
  a sparse override that four places compensate for; cells sum
  `lines × line_h` and fold the label boost into `space_before`).
- One widow/orphan split helper for the body split, keep-with-next and cell
  splits (cells assume widow control on and ignore keepLines).
- Run-boundary breaks from UAX #14 over the paragraph's joined text (a URL
  split across runs can still break after `/` at the boundary).
- One `table_left_x` for body, header and nested tables (nested tables miss
  the compat-15 rule).
- Border bands split between a parse-time margin edit and one render site
  (row splits, header table height and nested tables miss part of them).
- Two shading blends (`shd_color` vs `approx_pattern_shade` in cells) give
  different answers for the same `w:shd`.
- `numId` resolved twice (`parse_zip` vs `parse_list_info`), disagreeing when
  a numPr has no numId; story-edge auto spacing implemented in three places.
- Table-style bold/italic precedence patched after the run cascade
  (`bold_is_direct`) instead of feeding the table style into it.
- Cell float z-order is a two-level approximation; the page already sorts
  deferred shapes by relativeHeight.
- `CjkLayout` now carries non-CJK switches (squeeze, shift-return): rename to
  `LineBreakOptions`, built once per paragraph; `alignment` is threaded
  through helpers only to build it.
- The pdf_name → FontEntry map is rebuilt per paragraph in four places;
  build it once per render.
- `autospacing()` runs twice per paragraph; GIF/TIFF are re-encoded to PNG
  and decoded again; the test `FontEntry` literal in `pdf/mod.rs` restates
  `stub_font_entry()`.
- Python tools: stext parsing in five scripts, `raster()`/page-metrics in two,
  `ab_view.py` ≈ `corpus_view.py`, `corpus_compare.py` ≈
  `compare_scores.py`, `ink_diff.py` re-implements the suite's diff image; a
  shared `tools/_common.py` would cover them. Each corpus document also pays
  a full font scan (`DOCXSIDE_NO_FONT_CACHE=1`).

## 7. Next steps

**Fixtures with the largest remaining gaps** (our J, known cause):

| fixture | J | known cause |
|---|---|---|
| scraped/german_mezzo_soprano_bio | 50.3 | CSS-style font names ("Archivo;sans-serif"); Word substitutes Tahoma-ish |
| cases/case3 | 62.1 | heading kerning (see §5) |
| scraped/go_math_grade4_guide | 44.1 | 24 vs 26 pages; table header row with a floating picture and a page break inside a cell is 6.6pt short |
| scraped/romanian_quality_evaluation_strategy | 53.9 | header row height (see §5) |
| scraped/slovak_pedagogical_practice_agreement | 48.3 | header line sized by the paragraph, not its own runs (headers don't use per-line heights); body 0.5pt low |
| cases/case50 | 54.4 | lines wrap differently |
| scraped/eco_int_agriculture_registration | 55.9 | Mac 0.25pt line grid + empty paragraph after a table |
| scraped/turkish_prostate_cancer_course | 63.4 | – |
| scraped/french_sexual_health_youth_strategy | 22.2 | – |
| scraped/dutch_government_budget_letter | 17.6 | – |
| scraped/polish_council_resolution | 49.4 | – |
| cases/case46 | 52.3 | – |

**Pending fixes**
1. **Done** (`f4ef46b8`, 2026-10-02). fontTable altName order: requested name first, altName as the fallback,
   keeping altName-first only for non-ASCII localized names; check the Korean
   fixtures and the full suite.
2. Route cell and footnote paragraphs through `build_paragraph` (§6).
3. Per-line heights in table cells and headers. Cells done (`53764bee`, §9);
   headers open.
4. Legacy table-style font size rule (<compat 15 without the override flag).
5. covid_insomnia two-column flow regressed with the squeeze (6).
6. polish_municipal_letter floating table double bottom border; japanese_land
   SSIM drop from 14.
7. A whitespace-only run between bold and italic runs (slovak_constitution)
   doesn't set the ascent in Word; exact rule open.

**Corpus gap categories:** header structures (a nested logo + title table
5–7pt low, first-page header frames +21pt, an empty header line pushing the
body 1.5pt); one 56-page document with many causes; several untriaged.

**Tracked changes:** roadmap plan, step 1 = inline markup for `w:ins` /
`w:del` with deleted runs kept in layout.

## 8. Decisions

1. Accept baselines — yes, done (scores for improved fixtures, case79, visual
   hashes; see §1).
2. Merge into `main` — only when the user says so.
3. Synthetic docGrid fixture — done (`case79`, reference converted by the
   user).
4. Tracked-changes rendering — on the roadmap; whether markup or final text
   is the default is open. Comparing two documents into a tracked-changes
   .docx is a separate tool, out of scope.
5. Mac-only system fonts — yes, done (43).
6. Cloud fonts — copied into `fonts/CloudFonts/` and committed to the private
   assets repo (`59143d9`, 2026-10-02).
7. altName fix — implemented in `f4ef46b8` (2026-10-02).
8. Optional: `git gc --prune=now` once other sessions are idle (local only).

## 9. New scraped cases round (2026-10-02)

Ten new scraped fixtures (`tests/fixtures/scraped/`: chinese_asset_disposal_appraisal,
czech_wastewater_discharge_permit, door_air_cooling_unit_spec,
greek_history_lecture_press_release, indonesian_school_admission_checklist,
italian_teacher_hiring_preferences, nabl_lab_preassessment_guidelines,
pasto_city_hall_press_bulletin, sao_paulo_procurement_contract,
welsh_palliative_care_abstract_form) plus two older ones that got their first
reference (strategi_pengembangan_information_center_resort,
ukrainian_municipal_heating_resolution). Fixtures committed in `e3612354`,
scores and visual hashes accepted in `429a9a91` (regressions included, so the
suite no longer flags them). Each low case was diagnosed by
a read-only agent with what-if docx edits; the queue below comes from those
reports. The last two reports are in `tests/output/pending/diagnosis-*.md`,
which is gitignored and local only.

**Scores** (Jaccard / SSIM, start → now):

| fixture | J | SSIM |
|---|---|---|
| door_air_cooling_unit_spec | 10.7 → 79.8 | 24.8 → 85.3 |
| greek_history_lecture_press_release | 38.7 → 79.1 | 50.4 → 89.7 |
| sao_paulo_procurement_contract | 42.5 → 78.6 | 55.5 → 97.0 |
| welsh_palliative_care_abstract_form | 14.1 → 74.9 | 19.8 → 91.5 |
| indonesian_school_admission_checklist | 13.4 → 73.5 | 36.7 → 90.1 |
| italian_teacher_hiring_preferences | 64.1 | 96.8 |
| pasto_city_hall_press_bulletin | 60.4 | 97.2 |
| chinese_asset_disposal_appraisal | 52.6 | 88.8 |
| czech_wastewater_discharge_permit | 23.9 → 50.8 | 41.8 → 74.3 |
| ukrainian_municipal_heating_resolution | 39.3 → 48.5 | 58.8 → 68.8 |
| nabl_lab_preassessment_guidelines | 35.0 → 47.1 | 60.4 → 66.6 |
| strategi_pengembangan_information_center_resort | 20.9 | 39.1 |

Mean of the 12: 34.6 → 60.9. The other 222 fixtures: 63.99 → 64.43 (against
`tests/baselines.json`).

**Rules implemented** (one commit each, each verified by a full suite run;
2453a6f5 and b3103351 share one run):

| Commit | Rule | Evidence |
|---|---|---|
| 6a917b1b | A missing name that is a face's full name ("Arial Bold") resolves to its family at the run's own weight, after the altName | welsh 14.1 → 21.3 |
| 62240f49 | Comment-pane zoom = page_w / (page_w − right margin + 279.7), tx 0.96, ty centred + 0.54; headers/footers zoomed too | door_air 10.7 → 76.1; case63/64 −0.5 (0.08pt from Word's transform) |
| 423f59fc | An empty paragraph's trailing leading may hang past the bottom margin | sao_paulo 42.5 → 78.2, feminist +7.1, chinese_student +3.9 |
| 26c7f39e | A non-ASCII lvlText takes the level's hAnsi font (§17.3.2.26) | welsh 21.3 → 74.9 |
| f47b2afb | Italic in a face without one is sheared 87/256 (absolute Tm); find_font_file reports the face's own style | greek 38.7 → 79.1 |
| 6f7be31e | A break opportunity after every breaking space, also before what UAX #14 LB13 glues (`.` `,` `/` `)`) | 20 up / 3 down, czech_wastewater +7.6, ukrainian +9.2 |
| b4dc1f65 | A vMerge restart cell taller than its spanned rows grows the last non-exact one | indonesian SSIM 36.7 → 85.8 |
| 80f9d345 | firstLine and hanging are one value: a direct (or closer style's) either replaces both | indonesian 17.4 → 22.5 |
| be3ed75a | Top/bottom wrap distance = distT/distB + effectExtent t/b | indonesian +5.6, indigenous_innovation +1.2 |
| 53764bee | Table-cell lines sized by their own runs; a line is sized at each run's own size (not small caps' 80% or sub/superscript's 0.58) | 12 up / 1 down, croatian_military +18.7, czech +10.6, russian_university +7.7 |
| dfeffba7 | The page top is where the body starts, also below a header taller than the top margin | nabl +6.2, federal_procurement +6.1 |
| f17df7b1 | The keepNext chain ends on an in-flow table's first row | czech 42.1 → 50.8 |
| 2453a6f5 | An empty paragraph's mark keeps its own w:b / w:i | (with the next) |
| b3103351 | Latin line = win box + max(0, hhea ascent + lineGap − usWinAscent); descents never count | indonesian 28.1 → 73.5, slovak_constitution 51.1 → 66.7, multi_font 22.0 → 34.9 |

Regressions and what explained them:
- 423f59fc: brazilian_logistics 64.8 → 62.1. On its page 10 the bullets sit
  36pt left of Word's and wrap short; an empty paragraph pushed to page 11
  used to hide that.
- b4dc1f65: covid_insomnia −0.5. The first row a merged cell spans is already
  1.4pt short; the merged cell now makes that up in the second row. The table
  total equals Word's.
- b3103351, first version: GDI's external leading, max(0, lineGap − (win sum −
  hhea sum)), counted the descents and sent slovak_constitution to 36.2 (Book
  Antiqua Bold, hhea descent 578, win 543). Only the ascent overshoot fits
  every measured font.

**Further work for the new cases** (gain = the agent's what-if, J):

- **strategi** (20.9; #1 + #2 together → 59.1, + #3/#4 → 62.5):
  1. **Done** (`7c4a1e02`, §10). Table-style pPr spacing is never parsed. Header table style `a5` (after=0,
     line=240, no borders) gets docDefaults instead (1.15×, 10pt after), so
     9pt header lines step 21.90 where Word steps 10.32. `tables.rs:304` uses
     "the style has borders" as a stand-in. Fix: add before/after/line to
     `TableStyleDef` (`styles.rs`, resolving basedOn, storing spacing-only
     styles) and cascade docDefaults → table style → paragraph style →
     direct. 38 table styles in the fixtures carry pPr spacing.
  2. **Reference artifact, fixed by replacing the reference** (§10): an empty footnote gets an extra blank line in the Normal style (page 1
     footnote area 51pt taller in Word). Seen in education_consultant_posting
     too; turkish_journal (footnotes with text) has none. Trigger understood,
     reason not. Same code: paragraphs without pStyle fall back to
     FootnoteText (Word: Normal); the separator gap is a fixed 12pt (Word: the
     separator paragraph's line, 13.43 here).
  3. **Done** (uncommitted, 2026-10-03). A word glued across runs that is wider
     than a fresh line is carried there anyway and this run's part is cut at the
     margin with `fitting_prefix_len` (`layout.rs`, continuation carry); the rest
     re-enters as the next word. Page 12's URLs now break like Word's.
  4. **Done** (uncommitted, 2026-10-03, 4 Word probes). A page shows the number
     of the section at its top; a restart in a section beginning further down
     counts that page as its start, so the next page shows start + 1 (probe:
     restart 50 mid-page 1 → 1, 51, 52). `page_numbers` in `pdf/mod.rs`.
     Against the clean export: J 59.5 → 63.2, SSIM 87.4 → 92.3. Elsewhere
     only bush_fires p105 (a blank filler page in Word, we draw its
     header/footer) and wa_child p56 (our surplus page) changed.
  - Minor: a star shape in a vAlign=center cell ignores the centring and the
    left cell margin.
- **ukrainian** (48.5; all five → 76.5):
  1. No justified space squeeze in paragraphs with `w:tab` (+9.0):
     `build_tabbed_line` wraps on plain overflow. Apply the compat-15 squeeze,
     counting only the inter-word gaps after the last tab. 24 fixtures have
     justified tab paragraphs.
  2. An empty paragraph whose line box overlaps a floating table is not
     pushed below it (+6.5). The float test checks only the line top
     (`mod.rs` float-zone test, `narrow_paragraph`). The same holds in
     russian_university_proceedings; re-check case32/45/46 (14 fixtures have
     floating tables).
  3. Tab gaps get justification stretch (+6.7). Word starts text at the tab
     stop and stretches only the spaces after the last tab: add a
     `justify_from` chunk index. Not yet checked on other references.
  4. Connector `relativeFrom` and `cmpd="thickThin"` are ignored (+0.3).
  5. `w:lvlJc` is not parsed (+0.2): right-aligned "1." markers in cells
     start at the indent instead of ending there.
  6. Underlines merge across x gaps (`push_decoration`) (+0.2);
     czech_crisis p4 looks like the same bug.
- **nabl** (47.1):
  - **Done** (`87f8d863`, guard 14pt). The row-split sliver guard `available_h > 50.0` (`table.rs`) (+8.9): Word
    splits two-paragraph rows with 22–30pt left. victorian_universal_design
    page 8 ends 6.1pt high, which the guard currently hides; fix that first.
  - The last section has no headerReference; Word inherits header1 for its
    body top (`effective_slot_top` reads `header_default` only), −11.5pt on
    page 15.
  - **Done** (`ddcf1e59`). vMerge continuation cells advance list counters (we print 15/16/…, Word
    9/10/…; 18 fixtures have `<w:vMerge/>`).
  - A merged first-column border runs past the table bottom on split pages.
  - A table at the page top sits 0.25pt high (Word starts the top border band
    at the slot top).
- **czech_wastewater** (50.8):
  - **Done** (`9bdab4af`). A page break in mid-paragraph is handled as a break after it
    (`runs.rs` has_page_break_after; spec-certain, −10.5 in its
    leave-one-out). Only this fixture.
  - Legacy FORMCHECKBOX fields are not drawn (−6.2): 10.08pt outer box at
    w:size 20, 0.72pt stroke, bottom 1.3pt below the baseline.
  - The tabbed-line breaker never wraps a word joined across runs (−1.6).
  - **Done** (`f40c709e`). Superscript size = run size × OS/2 ySuperscriptYSize/unitsPerEm, rounded
    to 0.5pt (0.65 Arial/TNR/Calibri/Verdana, 0.60 Aptos/Palatino; ~430
    measured). We use a flat 0.58.
- **welsh** (74.9): highlights fill the line box (we draw y − 0.2fs, 1.15fs);
  a highlight on the paragraph mark also covers the list label; small-caps
  spaces are drawn at 80%.
- **indonesian** (73.5): paragraph shading must not cover the float reserve
  (a behindDoc box is painted over); a paragraph-relative box sits 0.3pt low
  (offset measured before space-before?).
- **greek** (79.1, Mac reference): Mac Word's synthetic bold has a stroke of
  0.02·size + 0.12pt and a 1/1.02 vertical squash (6 Mac references). The
  bundled Comic Sans MS Bold has no Greek glyphs, so Word synthesizes the
  bold.
- **sao_paulo** (78.6): NBSP stretching in compat-14 justified lines (3
  samples, unresolved).
- **door_air** (79.8): deletions in headers and footers go into balloons in
  Word (tracked-changes roadmap step 6).
- **chinese_asset, italian_teacher, pasto**: not diagnosed (52–64 J, SSIM ≥ 88).

**Gotchas learned this round:**
- Run the CLI with `DOCXSIDE_FONTS=fonts` for what-ifs. Outside cargo the
  vendored fonts are missing (Symbol lost its √ glyph, which hid a cell
  line-height change).
- A `cargo package` leaves `target/package`, and the CLI binary then goes
  stale. Purge it (memory: cargo-stale-binary-gotcha).
- Fonts installed under `~/Library/Fonts` outrank the system fonts in our
  discovery. A 324-font folder there collapsed 76 fixtures for one run.
- Edit nothing in `src/` while a suite runs. A poll on rustc can miss the
  gaps between test binaries.

## 10. New-case accuracy round (2026-10-03, branch `accuracy-oct3`)

Ten more scraped fixtures (bulgarian_road_safety_program,
estonian_community_development_grant, radiographer_interventional_job_desc,
wa_child_services_regulations, massachusetts_community_sanitation,
chinese_costume_design_course, and four more in `cbf0773b`), references
exported unattended with `tools/word_export.py` (`ffe83298`). Rules came from
those references and from **Word probe documents**: python-docx files that
isolate one behaviour, exported by Word for Mac and compared with our render
(probe scripts in the job scratch dir; easy to rebuild). One commit per rule,
each verified by a full suite run; nothing below regressed a fixture unless
noted.

**Scores** (Jaccard, `main-pre-spring` snapshot → end of round): the 138
pre-existing scraped fixtures 58.92 → **60.51**, cases 74.30 → 74.37,
hyphenation 63.48 → 63.71. Biggest movers: croatian_thesis 38.2 → 88.2,
air_pollution 13.2 → 38.6, croatian_grant 14.2 → 28.2. New fixtures now:
estonian 34.3, bulgarian 34.1, radiographer 24.9, wa_child 20.4, chinese_costume
11.3, massachusetts 8.2. strategi 20.9 → 15.5 is a reference artifact (below).

**Rules implemented:**

| Commit | Rule | Evidence |
|---|---|---|
| ce713644 | `w:moveTo` text kept, `w:moveFrom` dropped | |
| 9bdab4af | A mid-paragraph page break splits the paragraph (continuation: no label, no first-line indent, no space before) | czech_wastewater |
| 9e02a22b | `w:position` raises/lowers a run and grows the line on that side only | |
| 97a240d6 | Cell picture paragraphs indent like text; a picture wider than the cell is clipped | |
| ddcf1e59 | vMerge continuation cells don't advance list counters | nabl |
| 60c87819 | A skipped empty sectPr paragraph's space after collapses with the next section's space before | |
| f40c709e | Super/subscript size = size × OS/2 script ratio (default 0.65), rounded to 0.5pt | |
| 87f8d863 | Row split guard 14pt (Word splits with one line of room) | nabl |
| 4bde195e | A split row's first part keeps the cell's opening space before | croatian_grant "Važno!" box |
| 5055470a | A hyperlink nested in a hyperlink keeps its text | |
| a15cefbc | A body line must fit whole above a footnote area | |
| 9a25fee1 | Cell lines snap to the docGrid only under `adjustLineHeightInTable` and auto spacing | chinese_student, physical_therapy unharmed |
| 2e91966e | compressPunctuation also squeezes opening brackets and `・`; no autoSpaceDE gap beside U+3000 | japanese_medical |
| 7c4a1e02 | Table-style `tblCellMar` and pPr spacing apply to cells, resolved along basedOn | estonian, strategi (vs a clean export) |
| af32080e | A line that wrapped tabs start keeps its (empty) line | |
| 34cb65c0 | contextualSpacing compares with a following table's first cell paragraph | |
| c44b853a | A cell's nested table splits between its rows across pages | radiographer +1.9 |
| 31c57c99 | **Odd/even section breaks** (15 probes): filler page iff the continuing number has the wrong parity and (no restart, or evenAndOddHeaders/mirrorMargins); a restart with wrong parity shows start+1; filler pages carry no header/footer; first/even header variants inherit only from the same variant, else none (§17.10.5) | croatian_thesis +50, wa_child +8.6 |
| 168f3916 | Autofit minimum width breaks CJK words after each ideograph | chinese_costume columns now match Word |
| 3340170a | Before compat 15, tables always outdent by the cell margin (also an explicit tblInd 0) | 6 fixtures +3–6 |
| 48b1e3d8 | **Grid + auto multiple** (40 probes): line = max(cells(glyphs), m × pitch); supersedes 42's cells × m | case79 unchanged |
| a26dfddd | PAGEREF prints its cached result | wa_child TOC |
| bda52988 | A text box's first paragraph drops auto space before | air_pollution +25.4 |
| c82e802e / bfffabed | Footnote list paragraphs (inline or style numbering) keep their numbering | croatian_grant bullets |
| ef29618f | A row taller than a page starts where each cell's first paragraph (two lines of a long one) fits | croatian_grant +10.6 |
| 86b5d3b6 | **No break after `/` before a letter or digit** (probe: 138 margin crossings, all wrapped whole) | 13 fixtures up to +4.2 |
| d029dd30 | Cell paragraphs carry their auto-spacing flags, so the cell-edge drop (13) runs | bulgarian J +0.1, SSIM −0.6 (later drift) |
| 4f5b1070 | Header/footer tables snap to their own section's grid pitch | no change |
| eb42536c | The row-split fit check counts the opening space before | no change |
| 86dd058d | Autofit measures words at line layout's break opportunities | no change |

Cleanup pass `a6a9605a` (`/simplify`, four reviewers): one first-chunk fit
check, a `successors` basedOn walk, `parse_line_spacing` reads `@line`,
`HfVariant` enum, `SectionProperties::line_grid_pitch`, no score change. It
closes part of §6's "cells built by hand" item (auto spacing).

**Findings:**
- **Reference artifacts.** Some fixtures trip Word's "unreadable content —
  recover?" prompt although the zip and XML are valid. The trigger is 4
  stray bytes (`\r\n\r\n`, a scraper artifact) after the end-of-central-
  directory record: stripping just them opens strategi cleanly, and its
  export matches a re-zipped one line for line. Those references show the
  *repaired* layout: strategi (15.5 vs ours, 59.5 vs a clean export, which
  also explains its "extra footnote line"), dutch_government (17.6 vs 20.4).
  Re-exporting the other 11 refs of batch `e3612354` (none with the bytes)
  reproduced them exactly. strategi's input and reference were replaced with
  the stripped file and its export (2026-10-03). 111 tracked fixtures end in
  the same bytes, and every one tested trips the repair, but the repair
  usually changes nothing. Of the 37 with local (Quartz) references, only
  strategi and education_consultant were damaged: for each, Word today
  through the repair reproduces the old reference exactly, and differs from
  the stripped export (Word lists "Footnotes" among its repairs). Both were
  replaced. Five others differ from today's export only by Word-version
  drift (repaired = stripped today): stem_partnerships (8 vs 7 pages),
  usep_handbook, alpharetta, east_asia, russian_university; refs kept. The
  74 online (tagged) references are untested: that needs Word's online
  export preset, set by hand.
  The bytes came from docxcorp.us (the manifest hashes include them; the
  site now serves the same files without them), so all 110 inputs were
  re-downloaded (2026-10-03); references unchanged, suite output identical.
- **All references online (2026-10-03).** Every reference except
  fonts/missing_font_substitution is now Word's online, tagged export
  (802b1d93, dc0fc19f, de03ae5d; `word_export.py --preset online`). That
  fixture is held back: it records missing-font substitution, and the
  online service picks from a different font set. Against the 38 scraped
  references that were local before, mean J fell 65.9 -> 56.0. Two causes:
  - Online Word places glyphs with rounded (hinted-looking) advances: Calibri
    'o'/'d' 6.003pt at 11.5pt where the font gives 6.06, 'e' exact; local
    Word and we use exact widths, so lines drift ~0.4pt (0.06pt vs local).
    Open: derive the rounding rule from many online refs (fonts, sizes).
  - Layout rules (fixed, probes in tests/output/probe_kn): a keep-with-next
    chain ends at a paragraph opening with a page break (a9746f5f, usep
    41.1 -> 79.5); a page break ending a section's last paragraph is dropped
    (8e7d5782, stem_partnerships 23.9 -> 66.6).
- References made before the staging fix print `<stem>_<hex>.docx` in FILENAME
  fields (massachusetts' footer); `word_export.py` now keeps the file name.
- Word for Mac never breaks after `/`; older references that do
  (education_consultant "Partners/", romanian "septembrie/") come from
  another Word build.
- Word floors auto-multiple grid lines to 0.24pt steps (19.44 where we give
  19.50); CJK fallback inside a Latin-font run raises a grid line in Word.
- `word_export.py`'s watchdog cannot answer dialogs without macOS
  Accessibility access; a repair prompt then blocks every later export until
  the tool recycles Word. It now rejects broken zips up front.

**Open:**
- ~~massachusetts: page-anchored body frames~~ done 2026-10-05 (roadmap,
  Annotation Fixes 2026-10-05 item 11): J 8.3 → 77.2.
- dutch_government: a page-anchored floating table moves the next body table
  2.4pt down in Word (not to the float's bottom); two 1pt paragraphs 0.7pt
  short.
- croatian_grant (71 vs 65 pages): page 28 starts with an extra line in Word.
- radiographer: Word also splits inside a nested row, between its lines.
- chinese_costume, wa_child: remaining drift not diagnosed.
- Not done from the cleanup review: cache a split nested table's layout
  across pages (relaid per page now), resolve table-style basedOn at parse
  time, drop the derivable `NestedTable.height`.

## 11. Synthetic-case gaps (2026-10-04, branch `synthetic-gaps`)

The lowest-scoring handcrafted `cases/` fixtures, re-scored first on the
current references with fresh conversions. Snapshots `base` → `fix6`
(`tests/output/snapshots/` in the worktrees); `cases` 74.08 → **75.81** J.

| Commit | Rule | Evidence |
|---|---|---|
| 46738736 | Character styles inherit along `basedOn` (closer style wins) | case50 54.4 → 79.2 |
| 78db4626 | Auto line spacing: text line × multiple, plus the marker's extra ascent **unscaled** (was (marker ascent + text descent) × multiple) | SymbolMT on Aptos 12 at 278: 17.76 vs Word 17.75 (was 17.89); on Calibri 11 at 1.15: 16.03 vs 16.00 (was 16.12). case3 +17.8, case33 +5.9, dialysis +8.9, romanian +3.2, german +2.5; scottish −2.3 (pre-existing −1.3pt page-top offset), polish_ministry −1.2 (0.1pt shifts, its marker steps moved closer to Word) |
| 58cb8d02 | Before compat 15 a floating table's `tblpX` places the first cell's text (border a cell margin left), as `tblInd` does inline | case46 R1 at the margin, border at 66.8; case46 +15.7, case40 +15.5, case45 +12.0 |
| 97249e22 | Tight/through wrap: the polygon's extent over the line's whole box (top to bottom), not a scanline at its top | case42 right wrap edges within 0.3pt of Word on 17 lines; +11.7 |
| 43803d1b | A word too wide for the left gap beside a both-sides float goes whole to the right region (Word leaves the gap empty) | case42 "ullamcorper." in a 56pt gap; +7.8 |
| f8985f80 | Glyphs inside a word drawn with the pair kerning its width already includes (TJ) | Aptos Display "Te" 20pt advances 8.0 in Word, 9.5 unkerned; case1's l-l, o-, and w-o land on Word's positions. case3 +13.5, russian_sports +15.5, czech_health +9.2, case18 +3.7, 6 more small gains, none lost; total PDF size +0.22% |

The kerning result overturns the March 2026 "TJ kerning is a dead end"
finding: that was measured against local (print-preset) references, which
carry plain hmtx advances; the online references do apply the font's pair
kerning. Word's extra per-size hinting adjustments (a few 1000ths of an em)
remain unmodelled.


## 12. Biggest scraped gaps (2026-10-04, branch `gap-fixes`)

The scraped fixtures with the largest Jaccard deficits, triaged by matching
text lines between Word's PDF and ours (vertical offset, page shifts,
horizontal drift). Snapshots `base` → `fix4b2`: mean J 63.85 → **64.19**,
scraped 58.02 → 58.58, SSIM 81.28 → 81.73; nothing down.

| Commit | Rule | Evidence |
|---|---|---|
| 66e68c8f | `IF` fields over nested fields (complex or `fldSimple`) are evaluated per page; STYLEREF `\n` is the paragraph's list number (0 when unnumbered); a top-level STYLEREF `fldSimple` is re-evaluated; style names match case-insensitively; a character style's value spans its consecutive runs; failing the page and before it, the search runs forward | Legislation running heads `IF {STYLEREF X \n} = 0 "{STYLEREF X}" "Part {STYLEREF X \n}"`: western_australia 41 of 58 heads wrong → 2, wa_child 19 → 1; J +1.5 (heads are little ink) |
| 01c46288 | Table-cell paragraphs resolve indents like body paragraphs (style and document defaults, not just a direct `w:ind`) | turkish_prostate cell text 7.2pt left of Word (style `ind left=144`) → exact; 63.4 → 82.9, turkish_ancient +3.5 |
| fa4db3fa | A floating table (`tblpPr`) in a header sits at its `tblpX`/`tblpY` and takes no room in the header flow | french_sexual's logo paragraph starts at the header top, logo 13.6pt above it; body back at the top margin; 22.2 → 44.4 |
| 00aa38ce | A column break in a one-column section breaks the page; mid-paragraph it splits the paragraph like a page break | bosch page 2 ends 7 lines short in Word (next paragraph opens with `w:br type="column"`); 27.4 → 62.4 |

**radiographer round** (snapshots `fix4b2` → `fix9b`: mean J 64.19 → **64.77**,
scraped 58.58 → 59.54; radiographer 24.1 → 75.8; nothing down):

| Commit | Rule | Evidence |
|---|---|---|
| c28b97ee | A split outer row breaks a nested table inside its first row that does not fit, each nested cell by the outer rules (recursive `CellCursor::nested`); nested tables draw from the cell layout's own widths/rows | radiographer page 1 ends at "Administrative teams within Radiology" as in Word |
| dd530994 | A section without its own header/footer lays its body out around the inherited one (shared `inherited_hf` walk with the renderer) | radiographer +26.2, wa_child +21.4, transition_to_work +13.3, go_math +13.1, western_australia +6.3, federal_procurement +4.3 |
| 319ce3b1 | A table at a page top adds no previous space after (the paragraphs' page-top rule with space before 0, shared `page_top_gap`); resetting the space after at the section break instead broke case25/26/28 | radiographer +13.7 |
| 42d12b9b | A row continued on a new page tops it with each cell's own top border, not the edge resolved with the row above (`CellBorders::own_top`) | radiographer's nested row (`top nil`) starts page 2 with no line; J −0.4 (the removed line overlapped Word's) |
| 8d33c8db | Split rows use each cell's own margins (border bands included), stored on `CellLayout` | slovak_eu +13.0, radiographer +7.9, education_consultant +4.1, nabl +3.2, isla +3.1 |

**Open (diagnosed, not fixed):**
- radiographer: a list label hanging left of its cell (numId 23, `left=65
  hanging=360`, x 40 vs the cell edge 49.5) is not drawn by Word; we draw it.
  Word may clip cell content to the cell box: probe before implementing.
- A continued row's top border sits wholly inside the row in Word (rule
  centre 62.53 under a 62.28 page top) and its text starts ~0.45pt lower than
  ours; one sample. Whether a whole row moved to a new page also takes its own
  top border is untested.
- estonian: no single break; lines drift down 0.15–0.4pt each through the
  opening paragraphs (first heading +0.4), then hold at +2pt.
- Table cells build paragraphs with their own parser (`tables.rs`) instead of
  `build_paragraph`; it has drifted (alignment default, tab stops incl. the
  implicit hanging-indent stop, multi-image paragraphs, borders, keep flags,
  paragraph-mark font). Switching moves many fixtures at once: own commit,
  full snapshot.
- The new-page sequence (flush, slot/column/page tops, bottom margin) is
  copied in `advance_column_or_page`, the page-break-before and -after
  blocks, and twice in `table.rs`.
- A floating header table neither wraps header text nor extends the header;
  no fixture shows Word doing either.
- A column break at the very top of a page (or with a page break before it)
  starts another page; whether Word leaves that page empty is untested (no
  fixture has one).
- `cargo fmt` reformats two spots in `src/pdf/layout.rs` (`push_decoration`
  and its call) that were committed unformatted; every commit of this round
  reverted them. Format them in a commit of their own.

## 13. Large-corpus triage (2026-10-06, branch `corpus-triage`)

A second external corpus: ~6,400 Word for Mac exports in four states (clean,
tracked changes, comments, both), kept outside the repo like the first. The
2,480 **clean** documents were scored with `tools/corpus_score.py` and
v0.18.3 (`a8986732`): **mean Jaccard 68.1** (median 79.4), SSIM 81.7,
0 conversion failures, page count wrong on 170 (6.9%). The three
tracked/commented states are dominated by missing redline markup (§5,
roadmap "Tracked-Changes (Redline) Rendering") and are scored separately.

**Where the clean score goes** (mean J of docs with / without):

| split | docs | J |
|---|---|---|
| page count right / wrong | 2,310 / 170 | 71.2 / 21–32 |
| 1 page / 2–3 / 4–10 / >10 (Word's count) | 1,298 / 631 / 421 / 130 | 77.4 / 61.2 / 55.5 / 49.5 |
| compat 12 / 14 / 15 | 528 / 390 / 1,149 | 83.7 / 60.8 / 62.0 |
| tables | 829 / 1,651 | 56.6 / 73.9 |
| VML shapes (`v:shape`) | 234 / 2,246 | 46.2 / 70.4 |
| text boxes | 177 / 2,303 | 48.6 / 69.6 |
| right-to-left text | 59 / 2,421 | 44.1 / 68.7 |
| East Asian text | 62 / 2,418 | 28.4 / 69.1 |

Feature splits overlap (documents with tables are also longer and carry
headers, pictures and lists); they rank, they don't add up.

**Triage of every clean document** (reference PDF vs ours, first matching
signal; script and data local):

| bucket | docs | mean J | corpus points lost |
|---|---|---|---|
| page-1 lines >3pt off vertically (median over lines matched by text) | 160 | 22–58 | 4.7 |
| page count drift (page 1 in place) | 106 | 17–52 | 3.1 |
| missing pictures (fewer images than Word) | 135 | 18–81 | 3.1 |
| a font Word drew is absent from ours (>20% of glyphs) | 91 | 16–79 | 2.9 |
| genuinely missing text (<70% of Word's words) | 19 | 0–73 | — |
| no single signal (inner layout / reflow) | 1,885 | 28–89 | 15.7 |

Measurement gotchas found on the way:
- Word's PDFs carry a whitespace-only text line for every empty paragraph
  mark; a "first line" taken from stext lands on those. Match lines by text.
- Text recall from extracted words under-reports for Arabic (Word emits
  presentation-form glyphs), CJK (different word splits) and ligatures
  ("ti" in Calibri splits "informa|tion"). Compare word *counts* to find
  real loss.
- Font names must be compared by visible glyphs per family: Word embeds
  faces that draw only spaces or bullets.

**Findings so far:**
- **Legacy VML pictures are never drawn.** `w:pict/v:shape/v:imagedata`
  (no text box) appears in 55 clean documents (278 shapes: 217 body, 59
  header, 2 footer; 221 inline, 57 absolutely positioned, mostly centred on
  the margin behind the text = header watermarks). It is in ~40 of the 135
  documents that lose pictures and in 2 of the 602 that keep them. Only
  `w:object` previews took the VML image path; `w:pict` went to the text box
  parser only.
- **Missing-font substitution by PANOSE (hypothesis).** A document in
  "TimesLT" (not installed; fontTable panose1 `02020603050405020304` =
  Times New Roman's, family roman, charset BA) is drawn in Times New Roman
  by Word, in Cambria by us (rule from `font-family-auto-substitution`:
  roman without a usable altName → Cambria). Needs a Word probe before
  changing the rule.
- Most-missed families where >20% of a document's glyphs are affected:
  Verdana, Helvetica, Hiragino Mincho ProN, MS Mincho, Roboto, Ubuntu,
  SimSun, Poppins (under diagnosis).
- Missing text: 7 of the 19 documents lose DrawingML text box content
  (`wps:txbx` inside `mc:AlternateContent`); one document renders 1 page
  where Word has 3 (under diagnosis).
