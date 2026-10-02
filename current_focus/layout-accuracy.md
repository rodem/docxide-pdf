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
  in Source Sans Pro by Word, in Corbel by us. Fix pending (§7).
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
1. fontTable altName order: requested name first, altName as the fallback,
   keeping altName-first only for non-ASCII localized names; check the Korean
   fixtures and the full suite.
2. Route cell and footnote paragraphs through `build_paragraph` (§6).
3. Per-line heights in table cells and headers.
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
7. altName fix — recorded, not yet approved to implement.
8. Optional: `git gc --prune=now` once other sessions are idle (local only).
