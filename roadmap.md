# Roadmap

## Accessibility (IN PROGRESS — started 2026-10-01)

Goal: our PDFs are accessible on their own merits; Word's export is a floor,
not the target, and where we can do better than Word we do (line numbers as
artifacts, three-level tables kept). Failures that come from the DOCX lacking
something (no title → 7.1-9, picture without `descr` → 7.3-1, headings that
skip a level → 7.4.2-1) are expected: we never invent titles or alt text or
renumber headings; the baselines absorb them. Measured by
`./tools/run-tests.sh --test accessibility` (needs `brew install verapdf poppler`;
the test skips with a notice when they are missing). Every fixture gets
**`ua_fail`** (PDF/UA-1 rules our PDF fails on its own, any increase is a
regression; a PDF claiming PDF/UA must fail none), plus three
reference-relative metrics where Word's reference is tagged, all in
`baselines.json` like Jaccard/SSIM (explained in `SCORING.md`):

- **`ua_deficit`** — veraPDF PDF/UA-1 (`-f ua1`, forced because Word writes no
  pdfuaid) rules we fail where Word passes, or where we fail a larger share of
  the rule's checks than Word (so 7.1-3 "content not tagged" can't hide behind
  Word's one stray failure). Count, lower is better, **0 = at least as good as
  Word** on the machine checks. Any increase is a regression.
- **`a11y_struct`** — 1 − edit distance over the structure-tree element types
  in reading order (`pdfinfo -struct`, role maps resolved, Span dropped,
  `Figure+alt` distinct from `Figure`).
- **`a11y_text`** — block texts in structure order (`pdfinfo -struct-text`,
  whitespace-normalised): characters of blocks that match exactly and in
  order. Catches reading order, headers/footers leaking in as content, and
  words merged because no space glyph was emitted.

Per-case detail (deficit rules with descriptions and check counts for both
PDFs) lands in `tests/output/<group>/<case>/generated.deficit.json`; analyses
are cached in `*.a11y.json` next to it (delete them after upgrading
veraPDF/Poppler). `DOCXSIDE_A11Y_GEN=libreoffice.pdf` scores another engine's
PDF with the same yardstick (no baselines written).

**What Word's references look like** (surveyed 2026-10-01): 174/222 are tagged
exports — `Document` root, P/Span/H1–H6/L/LI/Lbl/LBody/Table/THead/TBody/TR/
TH/TD/Link/Figure/Footnote/Textbox/TOC/TOCI, Word's RoleMap (Footnote/
Endnote→Note, Textbox/Header/Footer/InlineShape/Artifact→Sect, Title→H1,
Diagram→Figure, CommentAnchor→Span), `/Tabs /S` on pages, headers/footers as
`/Artifact /Pagination`. Word's own bar is low: no pdfuaid (5-1), catalog
`/Lang` always `en` (real language on Span `/Lang`), DisplayDocTitle without
a Title in 157/174, TH without `/Scope`, Figure `/Alt` copied verbatim from
`wp:docPr/@descr` (41/140 figures).

**Deferred: 48 untagged references** are macOS Quartz print-path PDFs (not
the accessibility export README.md claims) and score N/A until re-exported:
cases/case17 case18 case19 case20 case36 case63 case64; new/alfies_arc_adult_safeguarding_policy
alpharetta_school_governance_council americas_counter_terrorism_agenda
arizona_physical_education_standards bosch_software_ai_announcement
czech_census_2021_instructions dutch_council_member_resignation
family_kinship_lesson_plan japanese_land_development_sign_form
lithuanian_railway_transport_code romanian_quality_evaluation_strategy
russian_chess_pawn_lesson russian_regional_spatial_development
turkish_chemistry_course_plan; samples/double-underline run-borders
sample500kB; scraped/classroom_weekly_newsletter croatian_regulations_altchunk
czech_expert_witness_law east_asia_conference_form education_consultant_posting
federal_procurement_terms feminist_voice_dissertation go_math_grade4_guide
indonesian_benchmarking_guide italian_evaluation_minutes italian_project_proposal
learning_cultures_dissertation lithuanian_ethics_law lithuanian_food_quality_order
mandated_reporter_child_abuse polish_archery_range_plan
russian_university_proceedings seminary_hill_board_meeting
slovak_misdemeanor_amendment stem_partnerships_guide transition_to_work_deed
usep_handbook vaccines_history_chapter waste_management_request.
Their print path may also lay out differently from the online converter, so
re-exporting can shift visual scores.

**Starting point (173 scored):** struct 0 / text 0 everywhere (untagged);
ua_deficit 6–11. Every fixture: 6.2-1 MarkInfo, 7.1-3 untagged content,
7.1-8 no XMP, 7.1-10 no DisplayDocTitle, 7.1-11 no StructTreeRoot, 7.2-34 no
language. Also 7.2-2 outline language (59), 7.18.3-1 no `/Tabs` (25),
7.18.5-1 untagged links (25), 7.21.5-1 font widths ≠ glyph widths (7),
7.21.4.1-1 non-embedded base-14 fallback (3), 7.18.5-2 link `/Contents` (3),
7.21.7-1 missing ToUnicode (1), 7.21.8-1 `.notdef` referenced (1).

**Done (2026-10-01, one commit each, every one with no visual change):**
catalog `/Lang`, XMP, DisplayDocTitle, `/Tabs` · boundary space glyphs
(appended to the word's own `Tj`) · structure tree with P/H1–H6, artifacts by
default (`pdf/tagging.rs`) · object streams (`pdf/objstm.rs`, −15% size) ·
L/LI/Lbl/LBody · deterministic ToUnicode (lowest code point per CID) · tables
(THead/TBody/TR/TH/TD, TH `/Scope`, `/ColSpan`) · word boundaries at tabs and
`w:br` · Figures (alt from `docPr@descr`, decorative → artifact) · Links +
OBJR + `/Contents` · TOC/TOCI inside TOC fields · PAGEREF `\h` links ·
outlineLvl 9 = body text · built-in "heading N" levels · footnote/endnote
Notes · cell links, notes and lists.

**Done, round 2 (2026-10-01, `dad4444`..`6e50ff06`, no visual change):**
`ua_fail` scored on all 221 fixtures · textboxes → `Sect > P` hoisted after
the anchor paragraph (`Tags::hoist`) · floating pictures → hoisted Figure,
inline pictures → Figure inside their P (effects stay artifacts) · line
numbers as artifacts · nested tables inside their TD/TH · footnote/endnote
marks → Link with a GoTo to the note, holding the Note ("Footnote 3"
/Contents) · symbol-font glyph widths in `/W` (7.21.5-1) · `pdfuaid:part=1`
claimed only when title, Figure alt, heading order, embedded fonts and no
.notdef all hold (13 fixtures, all clean). Harness: `run-tests.sh` runs every
suite even when one fails; compact report shows `UaFail`.

**Done, round 3 (2026-10-01, `7587e1d9`..`b9909a03`):** caps/small caps
keep their source letters as `/ActualText` on a Span structure element (not
nested marked content, which Poppler's structure reader truncates after) ·
text-shadowed words read once · note marks drawn and linked in table cells
(the only visual change: 6 fixtures) · per-run language: inherited
`w:lang`, catalog `/Lang` = the text's dominant language (`document_lang`),
`/Lang` Spans for passages in another language. Scores unchanged except the
cell marks (erasmus_plus text 68 → 81%).

**Done, round 4 (2026-10-01, `5010019f`..`4525f72d`):** paragraph styles
inherit `w:lang` through basedOn (the write-back skipped it, so styles based
on Normal fell to docDefaults: lithuanian_public_information_law's ~157
`/Lang en-US` Spans are gone; the docDefaults-policy idea was a misdiagnosis)
· hoisted Figures/Sects follow their anchors' XML order (`anchor_seq`,
german_mezzo struct 94.3 → 100%) · Wingdings ToUnicode → real Unicode from
the font's glyph names (▪ ✔ ☺ …; Word itself is inconsistent: samtale's
reference has ☺, irish_school's keeps U+F0A8) · theme slots
`minor/majorEastAsia` and `minor/majorBidi` for Latin text (`ThemeFonts::slot`;
cs falls back to the themeFontLang `@bidi` script font) — the 3 fixtures that
use them get Word's Malgun Gothic/Arial instead of Type1 Helvetica
(east_asia_conference_form J 12 → 21%, SSIM 33 → 63%) · a font that resolves
nowhere gets Arial/Liberation Sans/Arimo/Helvetica/DejaVu Sans before Type1
(case60, multi_font) · a Unicode space the font lacks draws the font's space,
not `.notdef` (U+202F in macOS Arial 5.01, Aptos Italic). One commit each,
plus a `/simplify` pass (one field list in `resolve_based_on`).

Progress over the 173 tagged references: struct 0 → 94.8%, text 0 → 95.2%,
ua_deficit 1165 → 0 (every fixture fails no PDF/UA-1 rule Word passes);
LibreOffice's own tagged export scores 76% / 84% on the same yardstick. Over
all 221: ua_fail 459, 13 PDFs claim PDF/UA-1 and pass all 106 rules; what
remains is 5-1 (no claim, 208), 7.1-9 (198), 7.3-1 (29) and 7.4.2-1 (24),
all source-limited; every font is embedded. Output size ~24.0 MB (round 4
+37 KB: real fonts embedded where Type1 Helvetica was).

**Baselines accepted (round 4):** 15 fixtures' scores, 7 visual hashes. The
one drop is multi_font SSIM 42.6 → 39.2%: Copperplate Gothic Light now falls
back to Arial with real widths and fits on one line, where the
approximate-width Type1 Helvetica wrapped it like Word's real (wide) face
does; our page 1 also runs ~14pt taller than Word's, pushing its last line
(Bodoni MT) to page 2. Vendoring CopperplateGothic-Light in the assets repo is
the faithful fix. (irish_school's text drop is gone: `a11y_text` now folds
symbol glyphs, see SCORING.md.)

**How Word tags things (learned the hard way):**
- Pictures, charts and SmartArt: the paragraph's own (empty) P, then a
  `Figure` hoisted to Document level. Chart and SmartArt Figures carry **no
  extractable text** — their labels are artifacts; SmartArt's `/Alt` is the
  node texts one per line plus "(Layout Name)".
- Tables: `tblLook` firstRow → THead/TH (on when `tblLook` is absent),
  firstColumn → TH; a header-only table gets an empty TBody; a vertically
  merged cell's continuation is an empty cell (no RowSpan); no ColSpan, no
  Scope (so Word fails 7.2-42 and 7.5-1). Word also flattens some bordered
  data tables to one P per cell (who_prescribing 16×7, bush_fires 56×4,
  covid_insomnia) — the rule isn't recoverable from the DOCX features; we tag
  them as tables, which costs a few pp struct on those fixtures.
- Nested lists: the sub-list's L sits inside the parent item's LBody.
- Tabs and line breaks extract as spaces.
- TOC/TOCI only inside a real TOC field; hand-styled "toc N" paragraphs stay
  P. Each TOCI's Link is the `PAGEREF \h` page number.
- Footnotes: `P > Link > (OBJR, Span mark, Footnote > P)` — the Note sits
  inside a Link on the reference mark.

**Backlog, ordered by gap data (`tag_gaps.py` / `text_gaps.py` in the session
scratchpad; rebuild them from `tests/common/a11y.rs` if needed):**
1. Textbox lists are tagged P (not L/LI); table-cell and header/footer
   textboxes and floats stay artifacts; WordArt / text on a path has no text.
2. slovak_eu_directive: we emit 9 table rows where Word has 14 (table model).
3. Links in headers/footers and footnote text are still dropped; link rects and
   outline destinations ignore `BODY_SCALE`/vAlign (`assembly.rs`).
4. `/ListNumbering` on L (not checked by UA-1; Word writes Disc/Decimal/…;
   needs numFmt + lvlText threaded to `Tags::list_item`, ~10 sites);
   Wingdings 2/3 and Webdings still extract as private-use code points;
   `w:lang/@bidi` (complex-script text) ignored. (`w:softHyphen` dropped and
   `w:noBreakHyphen` → U+002D both match Word's extraction.)
5. Missing glyphs other than spaces still draw `.notdef`: Word rescues them per
   character from another font at their real width; widen the CJK rescue
   (`missing_cjk_chars`, `__cjk_fallback`) to non-CJK characters. The space
   fallback also lays U+2002/2003/2009/202F out at U+0020's width.
6. `anchor_seq` counts per parse pass: textboxes from paragraph-level
   `mc:Choice` (`collect_textboxes_from_paragraph`) sort after every run-level
   anchor. The XML position of the anchor node would give true order.
7. Test-run time: with Microsoft Defender scanning `tests/output` and a
   concurrent worktree run, the full suite took >60 min (normally ~6–10).

**Layout side findings (round 4 `/simplify`):** basedOn inheritance skips
`keep_next`, `keep_lines`, `contextual_spacing`, `page_break_before` and
`borders` (plain bool / default in `ParagraphStyle`, so "unset" can't be told
from false): a custom style based on Heading 1 loses keepNext. Also
`eastAsiaTheme="minorHAnsi"` (997 runs in the corpus) is ignored by
`resolve_east_asia_font`, and Times New Roman / Calibri / Cambria have no
metric-clone fallback (Liberation Serif, Carlito, Caladea) on Linux.

**Harness side findings (2026-10-01):** `tests/text_boundary.rs` has had no
`#[test]` since fb9373b, so the TxtBnd baselines are stale;
`engine_compare.py` `pdf_creator()` truncates Quartz producers at the escaped
paren.

## Deterministic Output (DONE — 2026-10-01, `6f64723a`)

All 226 fixtures convert to identical bytes across runs (three renders + `cmp`).
Three hash-order sources: `embed_truetype` fed `used_chars` (a `HashSet`) into the
glyph remapper; `collect_and_register_fonts` registered fonts seen outside runs
(SmartArt etc.) in `HashMap` order, shuffling F-names and font objects; and the
alpha ExtGStates were allocated and listed in `HashSet` order. Mattered beyond
reproducibility: the harness keeps a byte-identical `generated.pdf` (and its
screenshots, diffs, veraPDF results), which non-deterministic output defeated —
a `src/` touch with no output change took 2m25s, now 42s (warm 32s, cold 2m27s).
Corpus 0.7% smaller (sorted glyphs compress better); scores and conversion time
unchanged. Any new `HashMap`/`HashSet` iteration that reaches the PDF must sort.

## Note Marks in Table Cells (DONE — 2026-10-01, `a950737d`)

Footnote and endnote reference marks inside table cells were drawn empty
(erasmus_plus_staff_mobility_agreement "Seniority" vs Word's "Seniority²"):
cell layout never replaced the empty mark run with the note's number.
`RenderContext::with_note_marks` now does it for cells and body alike, and
the marks get their Link to the note. Visual output changed in the 6
fixtures with marks in cells (baselines accepted in `7879bc7f`).
Column auto-fit still measures cells without the marks (a mark's width).

## Layout accuracy round (2026-10-01)

Rules derived from Word reference PDFs (borders, text positions measured with
`mutool trace`/`stext`), one commit each. Fixture Jaccard over the round:
cases 69.9 → 74.1, scraped 39.9 → 55.7, new 38.5 → 52.5, samples 35.4 → 52.8,
hyphenation 61.2 → 63.2.

1. **compatibilityMode** is parsed (`docx::settings`). Compat 15 tables sit at
   margin + tblInd (no cell-margin outdent).
2. Header floats: a negative paragraph-relative offset counts toward the
   header's extent.
3. Table-cell list lines get the marker-ascent boost body lines already had.
4. **Border bands**: cell content sits between horizontal border bands (row =
   content + half of each band; table flow includes the outer halves). The old
   flat +0.5pt per row was Table Grid's border width.
5. Justification spreads slack over word spaces only (space gaps and text-less
   space chunks), not run joins inside a word.
6. **Justified squeeze (compat 15)**: a word stays on the line if its midpoint
   is inside the measure and the spaces can shrink to ≥ 75% (`SPACE_SQUEEZE`;
   94.6% of 14,122 measured Word line-end decisions; either condition alone
   ~90%). Older compat modes never squeeze.
7. **Per-line heights**: each body line is sized by its own runs (max top + max
   bottom, run-border pads included, math excluded); grid/exact paragraphs
   keep one box.
8. Compat 15 tables: the left border band starts at the indent (shift by half
   a band).
9. `w:kern w:val="0"` means kerning off (it overrides docDefaults).
10. Paragraph borders: the top band lies inside the paragraph like the bottom.
11. Lines span max ascent + max descent across their runs (case8: a pixel font
    with no descent beside Arial); sub/superscript offsets don't grow the box.
12. docDefaults without pPrDefault → Word's built-in 8pt after, line 278;
    an empty pPrDefault stays OOXML single/0.
13. HTML auto spacing (before/afterAutospacing) = 14pt, not at document start,
    not between items of one list (same numId), not at a cell's edges.
14. At-least trHeight bounds the row between its border bands (rows step
    trHeight + band); exact trHeight is the border-to-border pitch.
15. Negative pgMar top/bottom = absolute value, header/footer never push (§17.6.11).
16. A picture-only paragraph taller than its line gets its own mark's
    line-spacing leading (replaces the next paragraph's `after_image_boost`).
17. A body line of only spaces/breaks takes its height from a break run (it
    sizes the line it ends) or else the paragraph mark, whose font now
    inherits style → docDefaults and resolves theme fonts.
18. Leading spaces indent an inline picture as they indent a word.
19. `w:shd` `solid`/`pctNN` paint `w:color` over `w:fill` (auto black over
    auto white) for runs, paragraphs, styles and cells.
20. An empty paragraph's synthetic run takes the mark's font even when the
    mark sets no size (a Calibri mark in a Times style).
21. A tab never raises its line (12pt tabs among 11pt footer text leave the
    footer at the 11pt line); a line of only tabs still takes their size.
22. beforeAutospacing is dropped on a header/footer's first paragraph too.
23. `w:cr` is a text-wrapping break (§17.3.3.4).
24. **`w:linkStyles` without `w:attachedTemplate`**: Word reloads the styles
    from the stock Normal.dotm on open — its docDefaults (12pt, 8pt after,
    278 auto) and a Normal with no formatting of its own.
25. `doNotExpandShiftReturn`: justified lines ending in a manual break keep
    their natural width (no fixture changes; spec setting).
26. GIF and TIFF pictures are transcoded to PNG on load.

Remaining gaps are mostly fonts we lack and small cumulative vertical drift
(≈1–2px) that Jaccard punishes.

Open findings (not done):
- docGrid type="lines" with Latin text: Word places a 12pt TNR baseline 13.63pt
  into an 18pt cell (physical_therapy); neither centring nor leading-above fits;
  too few Latin grid samples to derive the rule.
- Empty table-cell paragraph height (turkish_journal: Word 11.84 for 10pt TNR,
  we 11.5) — exposed by the border-band fix.
- Table cells don't use per-line heights yet (`table_layout` sums
  `lines × line_h`).
- A whitespace-only run between bold and italic runs (slovak_constitution)
  does not set the line's ascent in Word; whether it counts for anything is
  open (counting it as text made the line too low).
- covid_insomnia two-column flow regressed with the squeeze.
- Tracked changes: Word's PDF export can show revision markup (balloons);
  we always render the final view. Not started.
- Below compat 15 without `overrideTableStyleFontSizeAndJustification`, a
  table style's font size beats Normal's (unless it is 10pt); we always let
  Normal win.
- A nested header table (logo beside a title table) can sit 5–7pt low,
  pushing the body down.

## Annotation Fixes 2026-09-18 (5 fixes, one commit each)

Baseline for the round: HEAD 2c0706d, 170 tests passing. Every fix verified by
a full-suite run against the previous fix's run (`touch src/lib.rs` before each
run — a stash pop mid-run bumps mtimes and the harness silently reuses PDFs).

1. **#241 cell float z-order** (japanese_land_development): a picture anchored
   in a table-cell paragraph was always drawn before the paragraph's
   connectors/textboxes; `CellFloatingImageLayout` now carries `z_index` and
   pictures above the shapes draw after them (`draw_cell_float`). No score
   change (small region), hash change only there. Also confirmed #220 already
   fixed by the row-split work (Q3 on p6 in both).
2. **#66 list marker line height** (case33): Word sizes a list line as *marker
   ascent + text descent* (× spacing), never the marker's descent, and drops
   the first baseline by the marker's extra ascent. 11pt Symbol on 11pt Calibri
   → 16.115 (Word 16.00) instead of our 15.50. Courier New "o" sub-bullets
   (streamnet p5) and Symbol on Arial (dialysis) confirmed the marker descent
   is ignored. 27 fixtures improve (samtale SSIM +36pp, case3 J +35pp,
   usep_handbook J +31pp, case33 +21pp, romanian_quality +21pp, czech_crisis
   +9pp); streamnet SSIM −3.8pp while its Jaccard gains 13pp — an agenda
   *table* on p1 runs +0.43/+0.68pt per row (row-height drift, was cancelled
   by the wrong list pitch). Residual: Word's 0.25pt line grid (16.115 vs 16.0).
3. **#158 #195 bosch**: (a) Word substitutes the *theme body font* for a
   missing `w:family="auto"` font — bosch (theme Calibri) → Calibri,
   german_mezzo "Archivo" (theme Arial) → Arial, three more fixtures agree;
   a plain "always Calibri" broke german_mezzo (1 → 2 pages). `register_font`
   takes `theme_body_font`; `try_candidate` writes the font, so it is called
   once. (b) A header `framePr` with `w:wrap="notBeside"` and `w:h` pushes each
   in-flow header *line* that would overlap its band below the frame (per
   line: bosch's first two 14.75pt lines fit above the 33–139pt band, the
   third lands at 139 → body at 153.75 like Word). bosch SSIM +21pp,
   croatian_thesis SSIM +9pp.
4. **#240 case41 p6**: the #152 look-ahead only fired for image-only anchor
   paragraphs; it now accepts text-carrying ones, the anchor paragraph's own
   zone peeks the handed-forward anchor top, and the look-ahead only fires
   when the float leaves ≥48pt beside it for text (case41 p3 wraps with
   64.8pt free; brazilian p9's figure with 37.5pt beside it went 20 → 21
   pages without that gate — Word leaves its caption alone).
5. **#237 stem_partnerships p4**: table rows split between *lines* of a cell
   paragraph (2 lines kept on each side), not only between paragraphs:
   `find_cell_split` returns a `CellCursor { item, line }`; `cursor_chunks` /
   `item_chunk_height` / `chunk_space_before` share the chunk arithmetic with
   `render_partial_row` / `render_partial_cell_content`; the split gate also
   accepts a single paragraph of ≥4 lines. stem_partnerships 9 → 7 pages (= Word).

Findings left for later:
- **#93 justified space compression**: Word pulled "2251," onto the line by
  shrinking the 10 breakable spaces to 2.00pt (natural 2.75 @ 11pt TNR, i.e.
  ~73%); no `wpJustification` compat flag. We only expand (`layout.rs`
  `overflows` / `extra_per_gap.max(0.0)`). Corpus-wide effect on justified
  text; the shrink limit needs calibration before touching it.
- **#233** needs Merriweather in the assets repo (underscore 0.835em vs Arial
  0.556em); nothing to do in code.
- streamnet p1 agenda table rows +0.43/+0.68pt each (table cell line height).
- dental_amalgam gained +22pp J with a max-descent rule but +10pp with the
  measured ascent-only rule — worth a look at what its markers are.

## Distributed Alignment (DONE — 2026-07-27)

`w:jc="distribute"` used to fall through `parse_alignment`'s
`_ => Left` arm, so distributed paragraphs rendered left-aligned. Added
`Alignment::Distribute`: it stretches *every* line including the last, and
spreads the slack between characters via `Tc` ("Distribute All Characters
Equally", §17.18.44) rather than between word gaps. The Tc divisor differs from
CJK justify by one gap: CJK justify keeps the grid's trailing cell gap (divide
by the char count), while `distribute` ends flush at both margins whatever the
script — Japanese 均等割り付け behaves the same way — so it divides by one gap
fewer (`char_justify_gaps` in `pdf/layout.rs`, unit-tested). Getting this wrong
left case77's CJK line 31pt short of the right margin.

Kashida variants (`mediumKashida`/`highKashida`/`lowKashida`) now map to plain
Justify instead of Left; true glyph elongation needs Arabic shaping we don't have.

Zero corpus fixtures exercise any of these values (verified across all 129 DOCX,
all XML parts), so corpus scores are flat — this is correctness-only.

### What case77's reference settled (2026-07-28)

case77 now has a Word reference. It confirmed three assumptions and killed one:

- distribute stretches every line including the last — our last line lands
  within 0.3pt of Word's.
- CJK distribute ends flush at both margins — validates the one-gap-fewer
  divisor; all 10 glyphs within 0.4pt of Word.
- `mediumKashida` on Latin text renders as ordinary justify.
- **`thaiDistribute` is NOT distribute.** Word leaves a Latin thaiDistribute
  line at its natural width while stretching a `distribute` line of the same
  shape, so it now maps to Justify. Whether Thai script triggers real
  distribution is untested — no Thai fixture exists.

case77 scores J 51.5% / SSIM 70.5% / TxtBnd 84.2%. The remaining gap is two
things, neither about distribute's core geometry:

1. **Distributed Latin lines with spaces** spread across one gap too few (Word
   counts spaces as distributable characters; we don't). Interior letters up to
   34pt off, margins still flush. See the `ponytail:` note on
   `char_justify_gaps` for why this wasn't chased.
2. **Kashida changes Word's line breaking.** With identical text, Word's
   `mediumKashida` paragraph breaks earlier than its `both` paragraph
   ("…industrious" vs "…industrious beaver"), displacing a whole line. We
   reproduce `both` breaking. Needs real kashida metrics, i.e. Arabic shaping.

Also note case77's own on-page label for sample 4 claims thaiDistribute gets the
"same treatment as distribute" — baked into input.docx before the reference
disproved it. Correcting it means regenerating the DOCX and the reference.

## New-Case Triage 2026-07-03 (10 fixtures added; fixes applied 2026-07-04)

Passing: streamnet_steering (J 54%), zimbabwe_broadcasting (J 53%). Fix round results (zero regressions across 218 cases):

1. **japanese_medical (J 2.8% → 4.3%, TB 5% → 46%)** — FIXED (a) CJK numbering formats `decimalEnclosedCircle` ①②③, `decimalFullWidth`, `aiueoFullWidth` in `format_number()`; (b) docGrid cell counting now uses sTypo metrics (`grid_snapped_line_h` in `pdf/layout.rs`) — win+lineGap (Yu Mincho 1.787) overshot the 18pt pitch and doubled every grid line (4 pages → 3, page 1 now matches ref). Remaining: sub-line drift through table rows (tables don't grid-snap; cell line heights slightly exceed Word's), one line still spills page 1 → 2.
2. **croatian_thesis (J 18.8% flat, SSIM 50.8% → 41.8%)** — FIXED the font bug: fontTable altName `SignPainter-HouseScript` (Word-for-Mac artifact for fonts missing on the authoring machine) is now rejected; falls to family fallback (Arial). Reference embeds real Merriweather — install it (Google font) or wait for Bundled Fallback Fonts for the rest. SSIM dip = more ink slightly misaligned; structure now correct. Ref page 2 is blank (trailing paragraph) — we emit 1 page.
3. **indigenous_innovation (J 12.2%, TB 0.7% — flat)** — FIXED indent precedence: numbering-level `w:ind` now outranks style ind when only some attrs are direct (§17.9.27, `docx/paragraph.rs`); recital numbers/text now align with ref. Score flat: 20 pages of justified-Arial wrap drift dominates.
4. **french_sexual_health (TB 75% → 82.7%)** — FIXED zone clobbering: a wrap float entirely outside the text column (margin QR code) no longer replaces an active in-column float zone (`pdf/mod.rs`). Body now wraps beside the top-right image. Remaining: ~2-line vertical offset (drift class).
5. **stiavnicke_bane (J 15.8% → 76.3%, SSIM 93%, TB 100%)** — FIXED: leading spaces on a line whose left region is blocked by a float now carry into the right region as an indent instead of triggering blank-line + x=0 (`pdf/layout.rs` build_lines).
6. **ut_koer (J 13.7% — open)** — tab-stop two-column contact header interleaved with a wrapTight header image: needs per-line segment layout with tab stops spanning the float's excluded band (`build_tabbed_line` has no float-zone awareness). Right column currently gets left-column content.
7. **physical_therapy (J 12.6%, TB 91.7% — open)** — no missing feature; constant small dx/dy glyph offset. Vertical Drift Investigation class.
8. **candidate_reference (J 21.3% — open, passing)** — minor table row-height drift (Table Row Height Deficit class).

## Hyphenation (PARKED — Word's online converter doesn't hyphenate)

`w:autoHyphenation` and `w:suppressAutoHyphens` are parsed from DOCX. `w:lang` is parsed on runs. However, **Word's online PDF converter does not perform syllable-break hyphenation** for any language, even when `autoHyphenation` is set. Every line-ending hyphen in reference PDFs comes from pre-existing hyphens in compound words (verified across 8 language-specific fixtures and all scraped fixtures).

The `hyphenation` crate (Knuth-Liang algorithm) was tested with 8 languages but its dictionaries don't match Word's — enabling it caused 40-50pp regressions across all languages because line breaks diverged. Removed for now.

**Prerequisites to revisit:** Reference PDFs generated by desktop Word (not the online converter) with proofing tools installed. The online converter ignores `autoHyphenation` entirely.

Test fixtures in `tests/fixtures/hyphenation/` (8 languages with Wikipedia text, `autoHyphenation` enabled) are ready for comparison when desktop-Word references become available.

## Image Cropping `a:srcRect` (DONE — 2026-09-04)

Cropped pictures used to render the full source squeezed into the frame. Now
`parse_src_rect` (`docx/images.rs`) reads l/t/r/b as 1/100000 fractions onto
`EmbeddedImage.src_rect` (None for absent, empty, all-zero or nothing-visible crops;
negative outward crops kept). It is applied through `apply_pic_props`, the one shared
step for outline, effects, clip geometry and crop that replaced three copy-pasted blocks
at the inline, anchored and paragraph-image parse sites. `embed_single_image`
(`pdf/images.rs`) wraps a cropped image in a Form XObject with BBox `[0 0 1 1]` drawing
the source through `crop_matrix` = `[1/(1-l-r), 0, 0, 1/(1-t-b), -l·sx, -b·sy]`; the inner
image lives only in the form's resources, not on the pages. The bbox clips, so negative
crops become blank padding for free and none of the eight draw sites changed; EMF forms
nest as-is. Downscaling and soft-edge radii see the effective source extent
(frame / visible fraction), so a heavily cropped photo keeps its resolution.

Verification: 9 unit tests (parser on the four real corpus elements plus edge cases,
matrix on identity/quarter/half/negative). Full suite: 220 fixtures, hashes changed only
on brazilian_logistics_study pages 9–10 and italian_evaluation_minutes page 7 — exactly
the pages holding the 4 real crops in the corpus (11 other fixtures have an empty
`<a:srcRect/>`, unaffected). All three pages match the reference crop visually. Scores
moved within noise (brazilian J 17.29 → 17.17, italian SSIM 43.83 → 43.82): the images
sit at drifted y positions, so the old squeezed image overlapped reference ink by accident.
`cases/case78` (generate.py + input.docx, grid PNG cropped four ways incl. negative) has
its Word reference and scores J 99.6% / SSIM 99.4%. It settled the open question: Word
renders a negative `a:srcRect` as blank padding inside the frame, exactly what the
unit-bbox form gives us for free.

Known ceilings (`ponytail:` note in `embed_single_image`): soft-edge and reflection masks
are still built on the uncropped source; SmartArt pictures use their own draw path. Also
seen while verifying: italian page 7's third signature is a bitmap EMF (`image3.emf`) that
`docx/emf.rs` rendered as nothing — fixed 2026-09-15 (annotation #228): `emf_to_raster`
wraps a lone EMR_STRETCHDIBITS DIB as a BMP, and inline pictures honour `a:xfrm@rot`
(quarter turns swap the layout box, `EmbeddedImage::layout_size`). See `minipdf.md` §1.2 and
§4.1 for the MiniPdf comparison and their corpus scan.

## Engine Comparison Findings (2026-09-04, `tools/engine_compare.py`)

Across 207 fixtures vs the Word reference we lead LibreOffice on mean Jaccard
(48.2 vs 39.5) and roughly tie on SSIM and text-boundary; MiniPdf is far behind
on all three. But LibreOffice beats us by 45+ points averaged over J/SSIM/TB on
a cluster of cases, i.e. we get the *structure* wrong, not just glyph placement:

| Case | ours J/S/TB | LibreOffice J/S/TB |
|---|---|---|
| cases/case13 (205 pp) | 8/21/1 | 46/90/100 |
| scraped/brazilian_logistics_study | 17/30/16 | 54/81/93 |
| scraped/russian_sports_ranking_decree | 10/20/42 | 43/89/100 |
| new/candidate_reference_check_form | 21/48/23 | 81/96/62 |
| new/family_kinship_lesson_plan | 23/39/32 | 58/87/94 |
| new/slovak_pedagogical_practice_agreement | 14/38/14 | 24/86/100 |
| new/school_meal_assistance_faq | 6/17/35 | 19/83/100 |

TB near 0 with LO at 100 means our line breaks or pagination diverge from page
one onward. These are the highest-value targets in the corpus; open
`comparison/index.html`, pick the case, and use the overlay to see where the
flow first departs.

**Page-count term in the compact report (DONE — 2026-09-05).** `run-tests.sh` now ends
its summary with `N/M page counts match`; per-case `ref_pages`/`gen_pages` sit in
`tests/output/latest_scores.json`. The visual test scores only the common pages, so a
generated PDF short of pages used to look fine. Not baselined (see `minipdf.md` §3.2).
Found while doing it: `tests/text_boundary.rs` has had no `#[test]` since fb9373b
(2026-09-03), so the TxtBnd values in `tests/baselines.json` are frozen; whether that
was intended is unverified.

## Picture Effects (PARTIALLY DONE)

**Done:** Smooth outer shadow (rasterized Gaussian blur mask via SMask), soft edge (edge-fade SMask on image), glow (centered blur), inner shadow (inverted blur mask), reflection (flipped image with gradient SMask). All use the same rasterized mask + SMask XObject infrastructure. Test fixtures: case56 (shadow variations), case57 (2D effects), case58 (3D effects — deferred).

**Remaining (deferred — no real-world fixtures use these):**
- **3D effects** (`a:scene3d`, `a:sp3d`) — bevel, metal frame, perspective rotation. Would require 3D lighting simulation. case58 has test fixtures ready.
- **Preset shadows** (`a:prstShdw`) — 20 built-in shadow presets. Need mapping table from preset names to parameters.
- **Theme color resolution in effects** — inner shadow with `a:schemeClr` falls back to black instead of resolving the theme accent color.

## CJK Rendering Polish (TODO — MEDIUM IMPACT)

Core CJK support is implemented: CIDFont/Identity-H/ToUnicode encoding, platform-specific font fallback chains (Hiragino/Noto/Yu Gothic), per-character font fallback at render time, script-based run splitting via `w:rFonts @eastAsia`, and vertical text rendering. CJK fixtures render readable output but score low (4-9% Jaccard) due to spacing/positioning precision issues:

1. **`w:firstLineChars`** (MEDIUM) — character-based indent (e.g. `firstLineChars="100"` = 1 character width). Not parsed; we only handle `w:firstLine` (twip-based). In practice, twip fallback is always present alongside firstLineChars.
2. **Vertical text centering** — `render_vertical_cjk_cell` uses a simplistic height calculation (chars x font_size) that doesn't account for paragraph spacing, causing vertical misalignment in merged cells.
3. **East Asian line height is 1.3× the font's Windows metrics** (DONE
   2026-09-14, was "Fallback line-height fidelity", annotation #8) — the ~1.73
   ratio seen in references is Malgun Gothic's 1.33 × 1.3. Word lays out any
   East Asian font (has CJK/Hangul/kana glyphs) at 1.3 × (winAscent+winDescent),
   no hhea lineGap, extra leading above the glyphs; exact-height boxes still
   bottom-align at winDescent; the docGrid counts cells with the same height
   (16pt YaHei on an 18pt grid → 2 cells, 10.5pt Yu Mincho → 1). Verified to
   0.1pt on line pitch in east_asia_conference_form, chinese_student,
   taiwanese_education, japanese_medical, tokyo_welfare. A run of nothing but
   spaces keeps the plain metrics so it cannot raise a Latin line
   (destination_loyalty: a lone MS Mincho space in a Times New Roman line);
   empty paragraph marks, tabs and the blank line after a break keep the real
   metrics (japanese_interlibrary_loan loses 20pp SSIM otherwise).
   `embed::compute_line_metrics`, `layout::run_line_metrics`.
   Open: where the extra leading sits. All-above matches exact boxes, but the
   first auto-spaced baselines in east_asia_conference_form (Batang 20pt after
   an empty paragraph) and japanese_land_development (heading 5, Yu Gothic
   10.5pt) land ~3pt higher than all-above predicts, closer to a half-above
   split. Needs a clean no-grid, no-header, text-first measurement.
4. **Linux CJK fallback list was Noto-only** (DONE 2026-09-14) — the Linux
   lists in `fonts/mod.rs` and `pdf/fonts.rs` named only Noto Sans CJK, which
   the CI runner lacks, so glyphs in runs whose font was missing were dropped
   outright (east_asia_conference_form on gh-pages: title shrank to "2024 발표",
   text boundary 0%). Locally the fixture looked fine only because
   `engine_compare.py` ran the binary without `DOCXSIDE_FONTS` (the
   `.cargo/config.toml` env applies only under cargo) and macOS system fonts
   covered the gap. Now one platform-independent list (`cjk_fallback_fonts`)
   leads with the vendored Word fonts, Apple/Noto faces trail, and the compare
   script passes `DOCXSIDE_FONTS`. The per-character rescue font
   (`cjk_rescue_fonts`) is ranked by glyph coverage of the missing characters.
5. **Theme `a:ea typeface=""`** — `asciiTheme/hAnsiTheme="minorEastAsia"`
   (47 table-label runs in east_asia_conference_form) resolves to an empty
   name and registers Helvetica. Word resolves the empty typeface through
   `a:font script="Hang"/"Jpan"` by the run's East Asian language (→ 맑은 고딕
   here); we should do the same.
6. **Missing CJK fonts substitute by charset + family** (DONE 2026-09-14) —
   the reference shows Word turned HY헤드라인M and 새굴림 (fontTable
   `charset=81`, `family=roman`) into Batang, rescuing kanji Batang lacks with
   MS Mincho. `classify_cjk_script` now reads the fontTable charset (0x80 JA,
   0x81 KO, 0x86 SC, 0x88 TC) before name hints, then Hangul/kana in the name or
   text; `cjk_fallback_fonts` orders serif vs sans by `w:family`. The fixture's
   font set now matches the reference except Helvetica for item 5.

## Bundled Fallback Fonts (TODO — MEDIUM IMPACT)

We rely entirely on system fonts and fall back to Helvetica Type1 as a last resort. This produces inconsistent output across environments (servers, Docker, CI). Should bundle metric-compatible open fonts behind a feature flag:
- **Carlito** — metric-compatible with Calibri (the most common Word font)
- **Caladea** — metric-compatible with Cambria
- **Liberation Sans/Serif/Mono** — metric-compatible with Arial/Times New Roman/Courier New

Metric compatibility means identical advance widths, so layout stays correct even with substitution. Ensures consistent output without requiring specific system fonts.

## Paginator Extraction (TODO — MEDIUM IMPACT, HIGH ARCHITECTURAL VALUE)

The `render()` function in `pdf/mod.rs` mixes pagination with rendering. widowControl, keepNext, keepLines, and tblHeader are already implemented inline, but extracting a dedicated pagination pass would:
1. **Clean up widow/orphan / keep-* logic** — currently embedded in the render loop with complex state tracking. A separate pass would be cleaner and more correct.
2. **Enable look-back wrapping** — paragraphs before a floating image anchor can't wrap beside it because the float zone isn't set until the anchor renders. Requires two-pass layout.
3. **Enable post-pagination field resolution** — PAGE/NUMPAGES fields could be resolved after layout instead of during rendering.

Architecture: a `Paginator` takes the document model and produces `Vec<Page>` where each `Page` contains positioned elements. The PDF renderer then simply draws them. This is a significant refactor but would simplify the render loop and enable features that require look-ahead/look-back.

## Vertical Drift Investigation (TODO — HIGH IMPACT)

**Root cause identified: glyph advance width precision.** Thorough investigation (April 2025) proved the drift is NOT from line height errors — line heights match Word exactly. The drift comes from our character advance widths being ~0.003pt/char wider than Word's at 12pt, causing ~1 fewer character per line on borderline lines. Over 48+ pages, this compounds into 1 extra page.

Evidence:
- Character-level comparison on case4 (Calibri 12pt): by char 89, our x-position is +0.27pt ahead of Word's (0.003pt/char average drift)
- Our widths match the font file exactly (verified via fontTools), but Word's widths are systematically narrower
- Removing hhea lineGap from line_h_ratio was tested and disproven — caused massive regressions with no benefit
- ceil() rounding of line heights was tested and disproven — too aggressive, destroyed all scores

**Disproven hypotheses:**
1. Line height formula (hhea lineGap inclusion) — disproven: removing it causes 80+ regressions
2. Line height rounding (ceil to whole points) — disproven: too aggressive, 90% regressions
3. Margin calculation error — disproven: our margins match DOCX spec exactly
4. Image paragraph height rounding — fixed in prior work
5. Table trailing spacing — disproven in prior work
6. Line-break tolerance (0.07–0.75pt) — tested April 2026: fragile, can't distinguish bias from genuine overflow. Any tolerance >0.07pt regresses Cambria-based cases (case11)
7. Global width correction factor — tested April 2026: helps Calibri/TNR but overcorrects Cambria. Magnitude varies by font and by font size.
8. Per-font width correction factor — tested April 2026: Calibri/TNR=0.99985, Arial=0.9999, others=1.0 gives 3 improvements, 0 regressions. Safe but captures only ~30% of needed correction. Can't go further because correction is size-dependent.

**Root cause confirmed:** Word's DirectWrite engine applies proprietary grid-fitting corrections that vary per glyph AND per font size (signs flip between sizes). These corrections are not in font data and can't be reproduced by FreeType or rustybuzz. The signed bias varies by font: Calibri +0.007pt/char, Arial +0.003pt/char, Cambria ~0pt at 12pt.

**Next steps — data-driven width correction (April 2026):**

The correction varies per glyph (some positive, some negative — not a uniform scale factor) and is likely ppem-level rounding from DirectWrite hinting. Plus, 6/9 inter-glyph adjustments in Word's TJ output aren't in font data at all. Rule-based reverse-engineering has hit a wall — data-driven learning is the natural next step.

**Phase 1 — Data collection pipeline:**
Build a synthetic DOCX generator producing controlled text for width extraction:
1. Single-glyph sheets: one character repeated per line (e.g., "TTTTTT...") at a specific font + size. TJ positions in Word's PDF give the exact advance width Word uses.
2. Bigram sheets: pairs like "THTHTH..." to capture inter-glyph adjustments (the proprietary DirectWrite corrections).
3. Font × size matrix: Calibri, TNR, Arial, Aptos, Cambria at sizes 8–24pt in 1pt steps.
Pipeline: `generate_width_sheets.py → .docx → Word conversion → extract_widths.py → width_corrections.json`

**Phase 2 — Analysis (formula or model?):**
Before any ML, test whether corrections follow a discoverable formula:
- ppem rounding: `round(advance * ppem / UPM) / ppem * fontSize` where `ppem = fontSize * 96 / 72`
- hdmx table: TNR has device-specific metrics — check if they match Word's widths
- Linear correction per font: maybe each font just needs a single scale factor per size
If a formula fits → implement directly. No model needed.

**Phase 3 — Correction table or model (if no formula):**
- Option A — Lookup table: `{font, ppem, glyph_id} → width_correction_pts`. ~20K entries × 4 bytes = 80KB. Simplest, most accurate.
- Option B — Small regression model: input = `[glyph_advance_units, lsb, rsb, ppem, font_class]`, output = `width_correction_pts`. 2-layer MLP (~1K params). Generalizes to unseen fonts.
- Option C — Per-font scale function: learn `correction(ppem) → scale_factor` per font. Small polynomial per font.

**Phase 4 — Kerning corrections (stretch goal):**
Same pipeline for bigrams. Input space is `glyphs²` but only ~500 common pairs matter. Sparse lookup table.

**Previous next-step ideas (status updated April 2026):**
- ~~Add a small configurable "text width tolerance"~~ — tested, fragile, regresses low-bias fonts
- ~~Test ppem-based rounding at various DPIs~~ — tested in March 2026 (see kerning_and_shaping.md), none match
- Create more diagnostic fixtures with different fonts/sizes — still valid, needed for Phase 1 data collection
- **Interim safe win:** ship per-font factor (Calibri/TNR=0.99985, Arial=0.9999, others=1.0) for 3 clean improvements while data pipeline is built
- **Analysis tooling:** `tools/experiments/width_analysis.py` extracts per-char signed width errors from reference PDFs

**Blocked annotations (triage 2026-07-03):** four open annotations diagnosed as
this drift class flipping a soft page break — no targeted per-case fix exists;
they should clear when width/height fidelity improves (matching triage notes
appended in `annotations.json`):
- **#59 brazilian_logistics_study p9** — pure width-drift: extra wrapped lines
  by page 8 spill ~4 blank spacer paragraphs above the Figura 2 caption (~82pt).
- **#82 czech_municipal_grant_form p2** — page-1 line/row heights ~28pt short;
  the intro paragraph Word overflows to page 2 (`lastRenderedPageBreak` on it)
  fits our page 1, so page-2 content sits ~26pt high. Row-height deficit class
  (see next section) as much as width drift.
- **#124 english_town_council_report p3** — 11pt TOC rows ~1.4pt/row short; the
  16-empty-paragraph stack straddling pages 2–3 fits our page 2 entirely, so the
  page-3 bordered box starts flush at the top margin (~25pt high, ~10pt of it
  from page-top space_before suppression once the box lands there).
- **#8 east_asia_conference_form p1** — different sub-class: the Korean fonts'
  CJK fallback has line ratio ~1.27 vs ~1.73 in the reference, so every
  `atLeast` row collapses to its trHeight while Word grows them (~116pt lost
  across one table). Belongs with Bundled Fallback Fonts / CJK metrics, not
  Latin width correction.

## Table Row Height Deficit (TODO — MEDIUM IMPACT, discovered 2026-07)

Our table rows run ~0.5–0.9pt shorter than Word's, compounding down a page of
stacked tables (case51: −6.5pt accumulated over 3 tables, measured via stext
anchor diffing). Consequence: content that Word pushes to the next page can
stay on ours. case51's reference has a blank page 2 (Word's implicit final
paragraph mark spills after the doc-ending table at 710.9pt); ours ends 8.2pt
higher so the mark fits and no page 2 is emitted — this alone costs case51
~22pp SSIM (missing page scores 0). The implicit final-¶ model and the
end-of-cell-mark suppression after nested tables are already in (2026-07);
only the per-row height accounting remains.

## Paragraph Border Groups (DONE — 2026-09-09, annotation #224)

Word joins adjacent paragraphs into one border group only when their `w:pBdr`
*and* indentation (left/right/hanging/firstLine) are identical; inside a group
no bottom/top rule or padding is drawn at the joins (only `between`).
`joins_border_group` in `pdf/helpers.rs` replaces the earlier "collapse only if
a paragraph is empty" heuristic from #122 — that heuristic was misreading
samtale p2 items 12/13, whose rules survive because item 13 has a direct
`w:ind left=1128` vs the numbering level's 1131 (3 twips → separate group).
samtale p1: the br-only spacer above "Medarbeiderens navn" no longer draws its
own rule; Din leder → name-line spacing 55.57 → 52.32pt (Word 52.56). Jaccard
-1.9pp because the -2.85pt drift accumulated above (br-only paragraph -1.22pt,
three bullets -0.53pt each) was previously masked by the bogus 3.25pt border
space. Corpus has no other adjacent identical-border/identical-indent
non-empty pair, so the other 14 pBdr fixtures are unchanged.

## Header Multi-Float Wrap (DONE — 2026-07-02, annotation #212)

`hdr_fz` is now a Vec of zones; all wrapping floats (same-paragraph + earlier
paragraphs) constrain the text bounds together, and paragraph indents are
measured from the column edge with float bounds clipping (Word semantics).
`parse_object_floating_image` honors `w10:wrap type="square|tight|through|
topAndBottom"`. Letterhead center now within ~5pt of reference. Remaining:
HR `o:hrpct` width should use the indent-adjusted paragraph box.

## Annotation Fixes 2026-07-03 round 2 (#121 #133 #167 #190 #219 — DONE)

- **#133 / #190 table row splitting**: the `row_h > page_content_h * 0.5` gate in
  `table.rs` blocked Word-style row splits. Word splits any non-cantSplit row that
  overflows the page remainder — EXCEPT rows with an explicit `trHeight` (exact or
  atLeast), which always migrate whole (verified: arizona/traditional all-trHeight
  tables never split in Word; isla/master_thesis no-trHeight rows do). New gate:
  no trHeight + multi-item cells + first chunk fits + `available_h > 50pt` sliver
  guard (our lines run a few pt short of Word's, so near-boundary rows see phantom
  space — victorian p8 had 43pt where Word had 6pt; lower once line-height
  fidelity improves). isla +5.8pp TxtBnd, master_thesis +24.1pp TxtBnd; collateral:
  carbon_farming +38pp TxtBnd, stem_partnership +9pp, english_town_council +7.2pp.
- **#167 floats in vAlign-centered cells**: the cell's vAlign centering offset was
  baked into the anchor base handed to `render_cell_floating_shapes` /
  cell floating images. Word anchors paragraph-relative floats to the cell content
  top. `render_cell_content` now takes `valign_off` and adds it back for float
  anchors only. The 50cm arrow in japanese_land_development_sign_form now spans
  table-bottom → ground-hatch exactly (scores flat — tiny ink area).
- **#219 exact line-rule baseline**: baselines were placed `font_size *
  ascender_ratio` below slot top; `ascender_ratio` folds in hhea lineGap, so a
  big-lineGap CJK substitute (Hiragino for 方正小标宋简体) pushed descenders out of
  the fixed `lineRule="exact"` box into the table border below. Word bottom-aligns
  the exact box: baseline = box bottom − winDescent (identity: `line_h_ratio −
  ascender_ratio`). `exact_baseline_base` in `render_paragraph_block` (both
  baseline sites). chinese_student_union +4.4pp SSIM/+2.3 Jaccard; polish_archery
  +8pp, auditor_regulatory +5.9pp Jaccard.
- **#121 trailing-break mark line**: the empty line a trailing `<w:br/>` leaves
  was sized with the break run's font (samtale: 26pt br), but it holds only the
  paragraph mark — Word sizes it by the mark's rPr (12pt here). Per-line loop in
  `render_paragraph_block` now uses `paragraph_mark_font_size` for the final
  break-created empty line when known (break char still sizes the line it
  terminates; intermediate br-created lines keep the break size). samtale +57.7pp
  TxtBnd / +10.9pp SSIM / +6.5pp Jaccard, german_mezzo_soprano +2.2pp.

## Annotation Fixes 2026-09-16 (#229 #230 #200 #232 #152 #238 — DONE)

Five fixes, one commit each, every run diffed against a snapshot of the
pre-round suite (221 fixtures, 203/221 page counts match); no fixture lost more
than 0.1pp, 15 improved.

- **#229 picture brightness/contrast** (italian_evaluation_minutes p7 "stamp"):
  the signature scan's `a:lum bright/contrast=30%` was ignored; Word's +30/+30
  washes the pale stamp inside the crop window to white. `parse_lum` +
  `apply_lum` (LibreOffice's DrawingML mapping: contrast scales about mid-grey,
  brightness offsets). Only italian and mongolian_human_rights_law changed.
- **#230 inline picture baseline** (italian signatures under their names): Word
  sits the picture bottom on the baseline and the picture top on the paragraph
  top; the picture line advances by picture height + descent of the runs with
  visible glyphs + multiple-spacing leading of the text run's font (the picture
  run's own `w:sz` never counts; a picture-only line has neither). Measured on
  italian (+2.2), english_town (+2.6 at 1.15), family_kinship (+7.6 at 1.5) and
  old_blue_truck (+0). `pdf/layout.rs`: `inline_image_line_extra`,
  `inline_line_advance`, `picture_line_bottom`; `render_paragraph_lines` takes
  the paragraph (ascent, descent). 12 fixtures improved (case16 J +5.7 / SSIM
  +16.4, russian_chess +2.6, croatian_thesis +2.6, english_town +1.6,
  polish_tender +1.3, ut_koer +1.1, usep +1.0). `after_image_boost` now only
  applies after block pictures (`para.image`), whose height is still bare.
- **#200 #232 floating tables** (croatian_grant p4-5 green box,
  indigenous_innovation p1-2 DEFINED TERM table): a `vertAnchor="text"` table
  with `tblpY ≥ 0` paginates like an inline table (rows split/migrate per the
  trHeight rule); only a negative-tblpY table (pendulum) moves whole with its
  anchor. `render_table`: `flows_inline` / `keep_with_anchor`. indigenous J +6.0
  / SSIM +7.7.
- **#152 pre-anchor wrap** (case41 p3): the paragraph before an image-only
  anchor paragraph wraps around the float positioned from its full-width
  layout; Word leaves the float there while the paragraph grows. Look-ahead
  installs the zone (top raised by the paragraph gap for its own geometry) and
  hands the anchor to the next paragraph via `pending_float_anchor`; replaces
  the "narrow the last line if the picture is under half the column" heuristic.
  Only paragraph-relative floats (Offset / AlignTop): `resolve_fi_y_top` puts an
  AlignTop-relative-to-paragraph float at the *page* top, which narrowed
  indonesian_benchmarking p6 until the look-ahead computed the top itself.
  case41 J +3.9 / SSIM +4.2.
- **#238 compressPunctuation** (taiwanese heading's lone 決): with
  `w:characterSpacingControl compressPunctuation` Word trims the full-width
  closing marks already on the line, evenly and by at most ¼ em, to keep one
  more character (marks on the taiwanese page advance 12–16pt at 16pt, never
  less). `docx/settings.rs` → `Document::compress_punctuation` →
  `RenderContext` → `CjkLayout` → `compress_punctuation()` in `pdf/layout.rs`.
  Gated: 191 fixtures say doNotCompress, only 6 compress. taiwanese J +3.8 /
  SSIM +9.1, tokyo_welfare +3.6 / +8.2, japanese_land_development −0.1 SSIM.

Left open with triage (2026-09-16; #66, #220, #158/#195, #237, #240, #241
closed in the 2026-09-18 round above): #233 (Merriweather not vendored), #186
(alfies p1 13.9pt high: OLE object paragraph + 24pt empty marks, not
investigated), #93 (Word compresses justified inter-word spaces to fit one
more word — see the 2026-09-18 notes), #185/#239 (vague), #8/#59/#82/#124
(systemic drift).

Follow-ups: `resolve_fi_y_top` should treat AlignTop-relative-to-paragraph as
the anchor top (then the look-ahead can drop its paragraph-relative filter);
opening brackets are not compressed; table cells still drop run-level inline
pictures (`EMPTY_INLINE_IMAGE_MAP`), so the picture-line rule does not reach
them; `Picture Effects` above can list `a:lum` as done. From the `/simplify`
review of this round: (1) one picture model — `docx/paragraph.rs` hoists a lone
inline picture into `Paragraph::image` (bare height + `after_image_boost` on
the next paragraph) while two or more stay in runs (`inline_line_advance`);
removing the hoist would give every picture the measured rule (~40
`para.image` renderer references); (2) a shared `decode_raster` so `a:lum`,
crop and soft-edge are applied once instead of per format branch, and reach
`embed_reflection`; (3) the floating-table keep-together could be geometric
(`fp.y > saved`) rather than reading raw `tblpY`, which would also decide
page/margin-anchored tables — no fixture evidence yet; (4) the look-ahead's
full-width lines could be reused when its zone reaches no line.

## Annotation Fixes 2026-09-15 (#231 #228 #236 #235 #223 #234 — DONE)

Five localized rendering bugs, one commit each; every fix changed only its own
fixture's visual hash, no score regressions across the 221 scored fixtures.

- **#231 EMF clip path** (indigenous footer logo as a black block): `pdf/emf.rs`
  skipped EMR_SELECTCLIPPATH, so the clip rectangle stayed an open PDF path and
  merged into the next FILLPATH. Now `W n`/`W* n` per the fill rule; ABORTPATH → `n`.
- **#228 bitmap EMF + inline rotation** (italian p7 signature missing): a lone
  EMR_STRETCHDIBITS is wrapped as BMP by `docx/emf.rs::emf_to_raster` (WMF-style);
  inline pictures keep `a:xfrm@rot`, draw turned about the frame centre and occupy
  the rotated bounding box (`EmbeddedImage::layout_size`) — Word gives a -90°
  56×108pt frame a 56pt line. `para.image` block pictures (a lone picture in its own
  paragraph) still ignore rotation. #230 stays open: Word puts the text baseline at
  an inline picture's bottom, we centre the picture on the text (`img_bottom = y +
  font_size - line_max_img_h` in `pdf/layout.rs`).
- **#236 / #235 table border inheritance** (croatian_grant_guidelines): inline
  `w:tblBorders` replaced the style's set wholesale; now merged per side
  (`merge_table_borders`), so Table Grid's insideH/insideV survive. Rule confirmed
  for #235: at a page split each row draws its own top/bottom border, which for
  inner rows is insideH — a table with insideH=nil shows no line at the split.
- **#223 spAutoFit in table cells** (japanese_land_development arrow hidden):
  `render_simple_textbox` ignored `AutoFit::Shape` and painted the white box at
  Word's 110.6pt default height. Height computation shared via `textbox_height`.
- **#234 footnote laid out as a table** (auditor_regulatory_report_template):
  `parse_notes_simple` read only `w:p` children. Table rows are flattened to one
  paragraph each, cells joined by a space (ponytail note in `headers_footers.rs`;
  real column geometry needs Block support in `Footnote`).

Triage notes for the annotations left open: #233 (Merriweather not vendored — font
availability, not code), #237 (row split is paragraph-granular; needs line-level
`find_cell_split`), #232 (floating `tblpPr` table pushed whole to the next page instead
of breaking), #229 (`a:srcRect` crop is parsed but the italian stamp still shows —
check the inline draw path), #238/#158/#195 (font/width class).

Follow-ups from the `/simplify` review of this round (not done): the EMF translator
still leaves immediate-mode segments (MoveTo/LineTo outside BeginPath) and mid-path
SaveDc/RestoreDc unhandled — a `path_pending` flag emitting `n` would generalise the
#231 fix; mixed vector+bitmap EMFs need the DIB placed as an image XObject inside the
form (bitmap-only EMFs take the raster path today); block pictures (`para.image`, five
draw copies across `pdf/mod.rs`, `table.rs`, `header_footer.rs`, `textbox_render.rs`)
want one shared `draw_embedded_image` that applies rotation + effects; body/header
flow still reserves `height_pt` for TopAndBottom textboxes while rendering uses
`textbox_height`; `TableStyleDef` parses no `pPr`/`tblCellMar`, so `has_tbl_style`
is a proxy for "style defines borders"; `Footnote { paragraphs }` should become
blocks so footnote tables keep their geometry.

## Annotation Fixes 2026-09-09 (#225 #226 — DONE)

- **#225 / #226 split-row borders**: `render_partial_row` drew the cell's top
  border only on the first fragment and the bottom border only on the last, and
  stretched non-final fragments to the body bottom (`fill_to_bottom_y`, added
  2026-04 without fixture evidence). Word closes every fragment as a complete
  box: master_thesis p2 ref bottom border at y=181.5 (fragment = 3-line
  paragraph + 1 empty paragraph, ends at an item boundary with ~9pt of body
  space left unused), p3 top border at the top margin (y=771); slovak_eu shows
  the same. Now every fragment draws all four borders and ends after its last
  fitted item. slovak_eu +5.6pp SSIM, isla +1.4pp, master_thesis +0.25pp; 10
  split-row fixtures scored, no regressions. Remaining on master_thesis p2: our
  body bottom sits ~19pt lower than Word's (footnote area: our separator 3pt
  below body bottom vs Word ~10.5pt; footnote text 15pt lower; last footnote
  line ends at the margin with no space-after), so we fit two extra empty
  paragraphs (26.9pt) and the bottom border lands at y=157 instead of 181.5.

## Annotation Fixes 2026-07-03 (#114 #118 #193 #214 #218 — DONE)

- **#114 ellipsis line breaks**: UAX #14 allows a break after U+2024/25/26 before
  digits, splitting TOC dot-leader tokens like `Preparation………45`. Word keeps
  them unbreakable; `split_preserving_spaces` now filters those break positions
  unless followed by whitespace (unit test in layout.rs).
- **#118 leading after tall inline image**: Word lays the line following a tall
  inline image one full line height below the image bottom (leading above the
  text). `after_image_boost` in `render_paragraph_block` extends the following
  paragraph's first baseline offset and block height by the missing leading
  (skipped for empty/grid-snapped/image paragraphs). brazilian_logistics p4 gap
  1.9pt → ~9pt (Word: 9.2pt).
- **#193 oversized list labels**: the ±1pt guard in `label_boosted_line_h` is
  gone (the "handled separately" path it referenced never existed) and the new
  `label_boosted_baseline_offset` drops the first baseline to the label's
  ascent — a 20pt number label on 10pt text now sizes the first line like Word.
  samtale +2.9pp SSIM, +40pp text-boundary; case16 +6.5pp, family_kinship +5.5pp SSIM.
- **#214 / #218**: see their sections (vAlign center, clear="all").

## Unimplemented Run Properties

### `w:emboss` / `w:imprint` / `w:shadow` (TODO — MEDIUM IMPACT)

636 hits across fixtures, 1 failing fixture. These are WML text effects (mutually exclusive per spec):
- **`w:emboss`** — raised/embossed appearance (highlight color on top-left, shadow on bottom-right)
- **`w:imprint`** — engraved/debossed appearance (inverse of emboss)
- **`w:shadow`** — drop shadow on text (offset copy in shadow color)

Not parsed, not rendered. Trivially implementable: parsing is `wml_bool`, rendering is offset/color-shift drawing passes.

### `w:outline` (legacy) (TODO — LOW IMPACT)

The legacy WML `w:rPr/w:outline` element (hollow text, no fill) is not parsed. We handle the modern `w14:textOutline` but not the pre-Word 2010 equivalent.

### `w:shd` on runs (TODO — LOW IMPACT)

Run-level shading (`w:rPr/w:shd`) is not parsed. We handle paragraph-level and cell-level `w:shd` but not run-level. Different from `w:highlight` (named colors) — `w:shd` supports arbitrary hex fill colors and patterns.

## Unimplemented Paragraph / Layout Features

### `w:jc val="distribute"` (TODO — MEDIUM IMPACT)

Distribute alignment (equal spacing including edges, different from justify). Currently silently treated as left-align — should at minimum fall back to justify.

### `w:mirrorMargins` (TODO — MEDIUM IMPACT)

Parsed from `word/settings.xml` and stored in `DocumentSettings.mirror_margins`, but **never applied to layout**. Fix: swap `margin_left`/`margin_right` on even-numbered pages.

### `w:gutter` (TODO — LOW IMPACT)

Gutter margin (`w:pgMar @gutter`) is not parsed. Adds extra space on the binding side for printed documents.

### `w:pgBorders` (TODO — LOW IMPACT)

Page borders (decorative borders around entire page) are not parsed or rendered. Defined in `w:sectPr/w:pgBorders` with per-side border definitions.

### `w:vAlign` on `sectPr` (TODO — LOW IMPACT)

Vertical alignment of text on the page (top/center/bottom/both). Not parsed from section properties. Mainly affects title pages and short documents.

### `w:textAlignment` (TODO — LOW IMPACT)

Vertical alignment of runs within a line (top/center/baseline/bottom/auto). Only superscript/subscript are handled; the paragraph-level `w:textAlignment` property for mixed-size runs is not.

### RTL / BiDi (TODO — HIGH EFFORT, MEDIUM IMPACT)

`w:bidi` (paragraph-level) and `w:rtl` (run-level) right-to-left support is completely absent. Requires implementing the Unicode BiDi algorithm (UAX #9) for correct visual reordering. Architecturally complex — affects line building, text rendering, and alignment.

## Unimplemented Table Features

### Cell paragraph `indent_right` in render pass (DONE — 2026-07-02, annotations #215/#217)

`table.rs` computed the render-time `text_w` without subtracting `para.indent_right`
while the wrap width in `table_layout.rs` did — centered cell text shifted right by
`indent_right/2` and justified text overshot the cell border. Both spots now match
the layout width (romanian_quality_evaluation_strategy SWOT headings).

### `w:vAlign="center"` text sits ~3pt high (DONE — 2026-07-03, annotation #214)

Root cause: baselines sit `font_size` below each line top, so a fallback font
with big leading (Hiragino Sans GB for 仿宋_GB2312: lineGap 0.5em) dangles that
leading below the ink of the last line, and centering the full block rode the
ink high. `cell_content_h_for_valign` now drops the last line's unused bottom
leading — but only when the font is a metric-changing substitution
(`FontEntry.is_substituted`): with the document's real font (Yu Mincho in
japanese_land_development_sign_form) the full-line-box centering already
matches Word, and subtracting regressed it −2.9pp. chinese_student_union +2.6pp SSIM.

### `w:tblLook` / `w:tblStylePr` (TODO — MEDIUM IMPACT)

Table conditional formatting (firstRow, lastRow, firstCol, lastCol, banded rows/cols). The table style is resolved for default borders but conditional formatting overrides (bold headers, alternating row shading, etc.) are not applied.

### Table auto-fit vs `tblW` (NO IMPACT — corpus check 2026-05)

Our `auto_fit_columns` uses `gridCol` widths from `tblGrid`, ignoring the specified `tblW` when `type="dxa"`. Word treats `tblW` as the authoritative total width and scales/caps columns to fit. This causes tables to render at full page width when python-docx (or other generators) emit oversized `gridCol` values alongside a smaller `tblW`.

**Verified empty in current corpus**: a sweep of all `tests/fixtures/scraped/*` and `tests/fixtures/new/*` documents found zero tables where `gridCol` total exceeds the `tblW` value (tolerance 100 twips). The bug is real per OOXML, but no fixture triggers it — implementing this clamp moves zero scores. Park until a real-world fixture exhibits the mismatch.

### Percent-based widths: `tcW`/`tblW` `type="pct"` (PARTIALLY DONE 2026-06)

`twips_attr` reads `w:w` as twips regardless of the `w:type` attribute. For `type="pct"` the value is in fiftieths of a percent (5000 = 100%). **Implemented**: `Table.width_pct` is parsed from `tblW type="pct"` and `apply_pct_width` scales columns to pct × content width — but ONLY for tables whose `tblGrid` is missing (grid inferred from row `tcW` values, which preserves pct proportions). When a real tblGrid exists, Word renders the grid widths as-is even when the pct width disagrees (observed: arizona 115%, zimbabwe 100% vs grid at 102% of content — scaling them regressed scores). **Remaining**: `tcW type="pct"` is still mis-read as twips for per-cell preferred widths; harmless today because grid widths dominate, but would matter for Word's full preferred-width algorithm (§17.18.87).

## Unimplemented Document Features

### Footnote pagination of split paragraphs (DONE — 2026-09-09, annotation #221)

Word puts a footnote in the footnote area of the page where its reference mark is laid out, and a body line fits on a page only together with the footnotes it references. The split-paragraph path in `render_paragraph_block` used to reserve every footnote of the paragraph on the first page and register them all on the continuation page after the flush (hole on page N, notes on page N+1, lines broken early). Now `WordChunk.footnote_id` records which `TextLine` carries which reference, `per_line_footnote_extra` charges each footnote to its line when computing `lines_that_fit`, and the first part's footnotes are registered before `advance_column_or_page`. Word also keeps one line for an empty footnote paragraph (sized by the paragraph mark) — `compute_footnote_height`/`render_notes_downward` count it. environmental_law_clinic_china Jaccard 0.085 → 0.210, russian_volunteerism_essay 0.247 → 0.670, czech_crisis_measure_notice 0.372 → 0.419.

**Remaining deviation**: when a reference line fits but its footnote does not, Word splits the footnote across pages with a continuation separator; we push the line to the next page instead (no overlap, rarely hit). Table rows (`table.rs` `row_fn_extra`) still reserve per row, which is right because rows are atomic.

### Endnotes (TODO — MEDIUM IMPACT)

`w:endnoteReference` is completely unimplemented. Footnotes already work — the plumbing (reference parsing, content parsing, rendering at page bottom) exists and could be adapted. Endnotes collect at the end of a section or document rather than at the page bottom.

### Additional Field Codes (TODO — LOW IMPACT)

Only PAGE, NUMPAGES, STYLEREF, and PAGEREF field codes are supported. Others (DATE, TIME, AUTHOR, FILENAME, IF, MERGEFIELD, SEQ, etc.) are silently dropped — only the cached display text is used. For static PDF export this is usually acceptable since Word pre-computes the display text, but dynamic fields (DATE, PAGE in headers) may show stale values.

## Anchored Shapes: Canvas/Group + Z-Order (PARTIALLY DONE — 2026-06)

**Done (2026-06):**
- **Drawing canvas (`wpc:wpc`) and shape groups (`wpg:wgp`/`wpg:grpSp`)** — flattened at parse
  time in `src/docx/group.rs`: composes `off/ext/chOff/chExt` child-space transforms recursively,
  emits leaf `wps:wsp` (textbox or connector), and `pic:pic` as independently positioned shapes.
  Fixes isla_language_lesson_plan venn diagram + grouped boxes (+2.6pp Jaccard). Fixtures with
  groups: isla, arizona_physical_education_standards (header), ukrainian_municipal_heating_resolution.
- **`a:noFill` overrides style-ref fill** — explicit noFill no longer falls through to the
  `fillRef` theme fill (was rendering noFill ellipses as solid accent-color shapes).
- **Style `lnRef` strokes on textbox shapes** — shapes without explicit `a:ln` color now get the
  shape-style stroke (previously only connectors did).
- **Z-order via `relativeHeight`** — `Textbox.z_index`/`ConnectorShape.z_index` parsed from
  `wp:anchor`; non-behindDoc textboxes and connectors render into per-shape buffers deferred to
  page flush, painted above the page text layer sorted by z (Word stacks floating shapes across
  paragraphs). Fixes lenten_prayer_unity white link on purple band; connectors must interleave
  with shapes by z or letter strokes drawn over gradient circles disappear
  (vaccines_history_chapter T/Y/B).
- **Connector presets stay connectors** — `parse_wsp_shape` declines line/straightConnector1/arc
  presets without text so they reach the connector parser (preset-geometry path loses
  flipH/flipV and arc sweeps; regressed vaccines_history letters when lnRef strokes made the
  textbox parse succeed).

**Remaining:**
- **Floating images don't participate in z-order** — they still paint inline at their anchor
  paragraph; e.g. lenten's white bird icon is covered by the purple band (icon z=251658243 >
  band 251658241). Same deferral treatment as textboxes/connectors would fix it.
- **behindDoc shapes from later paragraphs** can still paint over earlier paragraphs' text
  (needs pre-pass/paginator).
- **Group flips/rotation** — group-level flipH/flipV and rot are ignored (rare); leaf connector
  flips work.
- **Canvas/group inside paragraph-level mc:AlternateContent** — only the run-level path
  flattens groups; `collect_textboxes_from_paragraph` still grabs the first wsp.
- **Text in preset shapes placement** (case35 annotations) — text inside rightArrow etc. is
  positioned with plain rect insets, not the shape's text rectangle.

## Floating Image Positioning (TODO — MEDIUM IMPACT)

Floating images (`wp:anchor`) with large `posOffset` values can render off-page. Word appears to clamp or reflow these positions, but we render at the raw coordinates. Observed in `learning_cultures_dissertation` (rId14: column-relative offset 4702029 EMU = 370pt, placing a 334pt-wide image past the 612pt page edge). A naive right-edge clamp was tested but regressed `stem_partnerships_guide` — a more nuanced approach is needed (possibly only clamping when the image would be entirely off-page, or respecting wrap constraints).

Additionally, truncated/corrupt PNG images in DOCX files cause the `image` crate to fail with "unexpected end of file". Currently falls back to a 1x1 placeholder via `decode_png_raw` (using the `png` crate directly). Word renders these partially — investigate partial PNG decoding to match. Observed in `learning_cultures_dissertation` image1.png (216KB file, 2205 bytes short of complete IDAT data, no IEND chunk).

## `w:smallCaps` Rendering Accuracy (DONE — verified 2026-05)

`smallcaps_segments()` in `src/pdf/layout.rs:349` already applies the per-character rule correctly: only originally-lowercase characters are uppercased and rendered at `font_size - 2pt`; originally-uppercase characters render at full size. Unit tests at `src/pdf/layout.rs:1824+` cover mixed/upper/lower/non-letter cases.

## SmartArt Remaining Work

Basic fallback rendering via pre-flattened `dsp:drawing` shape trees is done, with full geometry engine support (all 187 preset shapes). Remaining:

1. **Group shapes** (MEDIUM EFFORT) — `dsp:grpSp` groups with nested transforms. Need recursive parsing.
2. **Connector shapes** (MEDIUM EFFORT) — `dsp:cxnSp` connectors between shapes (arrows, lines).
3. ~~**Image shapes**~~ (DONE) — `a:blipFill` image fills parsed from diagram-specific relationships, rendered with cover-fill scaling and shape clipping.
4. **Full layout engine** (VERY HIGH EFFORT) — implement the constraint-based layout algorithm that interprets ~200 XML layout recipes. Only needed for files that lack the `dsp:drawing` fallback. Not planned for the near term.

## Charts Remaining Work

All 8 chart types are supported (bar, line, pie, area, doughnut, radar, scatter, bubble). Remaining:

- **3D charts**: `c:bar3DChart`, `c:line3DChart`, `c:area3DChart`, `c:surface3DChart` — not parsed
- **Stock charts**: `c:stockChart` — not parsed
- **Combo charts**: two chart types overlaid on the same plot area — not handled
- **Stacked bar rendering**: parsed but rendering treats as clustered
- **Data labels**: not parsed or rendered
- **Chart title**: not parsed or rendered
- **Secondary axes**: not handled
- **Chart label positioning**: axis labels still have small offsets vs Word. `text_width_approx` (len x fs x 0.5) is crude — real font metrics would help.
- **Legend placement fine-tuning**: small positional offsets vs Word. Centering formula and spacing need per-chart-type calibration.
- **Font selection in chart labels**: picks arbitrary font from seen_fonts, not theme font

## Track Changes Remaining Work

Final mode (insertions included, deletions removed) is done. Remaining:

- **Markup mode** — rendering deletions with red strikethrough, insertions with red underline (for documents exported with markup visible)
- **Paragraph-level changes** — `w:ins`/`w:del` wrapping entire `w:p` elements at `w:body` level
- **Property changes** — `w:rPrChange`, `w:pPrChange`, `w:sectPrChange`, `w:tblPrChange` (formatting revisions)

## WordArt Remaining Work (LOW IMPACT)

Levels 1-4 are done (flat rendering, text effects, envelope warping, text-on-a-path).

**Level 5 — Legacy VML enhancement (TODO):** VML fill types (gradient/pattern), VML shadow, VML shapetype-to-prstTxWarp mapping. Basic flat rendering already done in Level 1.

## Image Drop Shadow Quality (TODO — LOW IMPACT)

Basic drop shadow rendering is implemented (`a:effectLst/a:outerShdw`): offset, color, alpha, and directional soft edge via layered transparent rectangles. Current limitations:
- **No real gaussian blur** — approximated with 10 stepped layers, visible banding at close zoom
- **Fallback paths lack alpha** — inline images in text lines, floating images, table/header images use pre-blended solid color instead of PDF ExtGState transparency (only body-level paragraph images get proper alpha)

## Bullet Line-Height Drift on macOS (TODO — font-metric blocked, found 2026-06)

case33 annotation #66: bulleted list paragraphs drift ~0.5pt LOWER per bullet vs the
Word reference (text above the list aligns perfectly; drift starts at the first bullet
and accumulates). Root cause precisely identified:

`label_boosted_line_h()` (`src/pdf/mod.rs`) boosts a bullet paragraph's line height to
`max(text_line_h, label_line_h)`, where `label_line_h` uses the bullet label font's
`line_h_ratio` (commit 8691a0d — Word includes the numbering label font in the
tallest-font-on-the-line calc; this fixed under-spacing, +1.5pp case33 / +11.8pp
polish_archery). The bullet font is **Symbol** (`w:numFmt="bullet"`, `w:rFonts ascii="Symbol"`).
On macOS we resolve `/System/Library/Fonts/Symbol.ttf`, whose `usWinAscent=1694`,
`usWinDescent=612` (upm 2048) give `line_h_ratio = 1.126` — anomalously tall (the win
descent is ~0.30em). The Windows Symbol font Word actually used yields ~1.08, so we
over-boost by ~0.046×fs ≈ 0.5pt per bullet.

This is the same class as the bundled-fonts gap: a precise fix needs authentic Windows
Symbol metrics, not the divergent macOS substitute. A hardcoded canonical ratio was
considered but rejected — it overfits and risks regressing `polish_archery_range`
(near threshold at 29.85% Jaccard), and the boost is a deliberately-tuned tradeoff.
Revisit alongside bundled fallback fonts (ship metric-stable Symbol metrics).

## Partially Implemented

- **Line spacing** — Auto and Exact work. AtLeast parsed but may not enforce minimum correctly.
- **Tab stops** — basic left/center/right tabs work but decimal alignment has precision issues.
- **Panose font matching** — fontTable.xml contains panose classification bytes; could use for more precise font substitution.

## Floating Image Wrapping — Remaining

- **wrapSquare height reserve gated on side-strip width (DONE — 2026-07-02)**: the anchor-paragraph reserve in `render_paragraph_block` now fires only when no usable side strip remains (`MIN_EMPTY_STRIP` = 18pt). With a real strip (brazilian_logistics_study, ~42pt) empty spacer paragraphs absorb through the float's span and the next real paragraph is displaced to `fz.bottom_y` — the old 48pt threshold double-counted the image height there. With no strip (sample500kB, image width == column width) Word stacks everything below, which the reserve reproduces. Note Word actually puts the anchor's own line box below the float too (ref gap 67.7pt vs our 51.8pt on sample500kB p4) — a first displacement attempt lost inter-paragraph gaps; revisit with the paginator.
- **`w:br type="textWrapping" clear="all"` (DONE — 2026-07-02, annotation #111; refined 2026-07-03, annotation #218)**: parsed into `Paragraph.clears_floats`; block loop drops the cursor to the float-zone bottom after such a paragraph. 2026-07-03: the cursor now drops one line height *below* the float bottom — the line following the break (the break paragraph's mark line) still occupies its full line height there, matching Word's ~16pt gap on indonesian_benchmarking_guide p7. Approximation: clear applies after the whole paragraph, not mid-paragraph (fine when the break is alone in its ¶, the common Word idiom).
- **Multiple floats per paragraph (PARTIALLY DONE — 2026-06)**: When one paragraph anchors 2+ wrapping floats (e.g. a logo on each side of a centered title, `pendulum_mechanics_oscillation_lab`), per-line geometry now subtracts every float's exclusion span and places text in the widest gap. Limitation: the page-level `float_zone` for *subsequent* paragraphs still tracks only the first float, so a following paragraph that overlaps only the second float won't wrap around it.
- **Remaining y-shift (page 2 only)**: Word places page 2's image (180x144pt) 14.8pt higher than all other images, despite identical `posOffset=0`. Pages 1,3,4,5,7 match perfectly (delta <0.02pt). Pages 2 and 6 (both cy=1828800/144pt) are the outliers. Likely Word snapping to grid/text boundaries based on image dimensions.
- **Look-back wrapping (TODO — MEDIUM IMPACT)**: Paragraphs BEFORE the image anchor cannot wrap beside the image because the float zone isn't set until the anchor paragraph renders. In Word, text from preceding paragraphs also wraps (e.g. case41 page 3 — the first paragraph's lower lines should wrap beside the centered image). Requires either a paginator or a two-pass layout with look-back.
- **Image in text paragraph**: Case41 page 6 — last line of text paragraph overlaps the image. Look-ahead only fires for the NEXT block, not same-paragraph floats.
- **Tight vs Through distinction**: Both currently use convex-hull polygon scanline. For Through wrapping, text should fill polygon concavities. Requires returning per-line interval segments instead of hull bounds. Rare in practice.
- **Word-break precision**: BothSides wrapping produces correct structure but slightly different word breaks from Word, causing ~2pp Jaccard differences on case41.
- **Polygon wrap text distribution**: Case42 (wrapTight + BothSides + complex 53-vertex polygon around Mario) scores ~46% Jaccard. Zone overlap detection is correct but line breaks differ from Word — likely font metric differences for Times New Roman causing different left/right text distribution. Text near concave polygon areas (Mario's arm) appears visually close to the image despite respecting the 9pt distL margin.

## Code Structure

### Duplication & extraction sweep (DONE — 2026-06-21)

Whole-repo over-engineering/duplication audit applied — see `extraction-audit.md`
for the full findings. All 30 verified survivors landed across 11 commits with
zero rendering regressions (208/208 scores unchanged). Highlights:

- **Shared parse helpers** in `docx/mod.rs`: `parse_on_off` (ST_OnOff), `parse_pt`
  (VML/CSS lengths), `is_wml` (namespace predicate), `merge_tab_stops`; plus
  `styles::{parse_font_size, parse_char_spacing, rfonts_ascii_name}` and
  `color::{resolve_dml_color reuse, parse_line_stroke}` now shared instead of
  re-inlined across runs/styles/paragraph/numbering/sections/tables/wordart/etc.
- **`FontEntry::encode`** replaces 5 copies of the char→gid/WinAnsi dispatch.
- **PDF emission helpers** in `pdf/`: `color::box_blur_3pass`, `helpers::draw_circle`,
  `images::{write_jpeg_xobject, write_gray_mask_xobject, write_solid_color_with_gray_mask}`,
  `table::render_table_rows` (nested + header/footer shared the same loop).
- **`render_chart` split** (`pdf/charts.rs`): 714→470 lines; the data-rendering
  match moved verbatim into `draw_chart_series(PlotRect, …)`.
- Dead code removed (`parse_tab_stops`, `FontEntry::char_width_1000_with_fallback`,
  EMF `color_at`, two `SampledBoundary` methods).

Deliberately NOT touched (see audit "leave alone"): the long-but-cohesive
god-functions (`render_paragraph_block`, `parse_table_node`, `render()`), the
generated geometry data tables, and the 3 intentionally-distinct path-command enums.

### Refactor `pdf/mod.rs` `render()` (see "Paginator Extraction")

The `render()` function in `pdf/mod.rs` is ~2400 lines with many closures and shared mutable state. The right fix is the paginator extraction described above. (Smaller in-file extractions are already done: `embed_single_image` is a free fn in `pdf/images.rs`; `label_for_paragraph` lives in `pdf/list_label.rs`.)

### Image-embedding cleanups (LOW IMPACT — deferred from textbox-image work)

Small consistency / efficiency wins in `pdf/images.rs` that were considered but skipped to avoid scope creep when adding textbox-internal image rendering:

- **Global Arc→pdf-name registry to dedupe XObjects across maps.** Same image data used in body + textbox (or table cell + textbox) is currently embedded as two separate PDF XObjects because each map (`inline_image_pdf_names`, `table_cell_image_names`, `textbox_image_names`, …) keys independently by `Arc::as_ptr`. A single global registry would let the second site reuse the first XObject. Wasteful in theory, accepted limitation in practice.
- **`build_paragraph_lines` / `build_tabbed_line` should accept `&HashMap<usize, &str>`.** The current `&HashMap<usize, String>` signature forces every caller (body, header/footer, table cell, textbox) to `.clone()` pdf names into a fresh per-paragraph map. Borrowing would eliminate the clones, but ripples through `pdf/layout.rs` and every caller.
- **Pair `image_names` + `effect_names` into a struct.** Every embedder (`hf_*`, `table_*`, `textbox_*`) threads the two maps as separate `&mut HashMap<…>` parameters. Pairing them would shrink signatures throughout `pdf/images.rs` and `pdf/textbox_render.rs`, but only worth doing alongside the dedup registry above (otherwise diverges from the established style without enough payoff).

## Performance

### Known Bottlenecks

- **Double font reads** — scan reads each font file for indexing, then `register_font` reads again for embedding. Keep the data from the first read.
- **Repeated WinAnsi conversion** — same text is converted in line-building, rendering, and table auto-fit. Pre-compute once and store in `WordChunk`.
- **String allocations** — `font_key()` allocates on every call; `WordChunk` clones font name strings per word. Use indices or interning.

### Parallelism (rayon)

- Font directory scanning — embarrassingly parallel, biggest win
- Font metric computation — parse face, compute widths per font independently
- Paragraph line wrapping — independent per paragraph once font metrics are ready
- ZIP decompression + XML parsing — read all entries into memory, parse in parallel

### Other

- Compress font file streams with FlateDecode (currently uncompressed)
- Memory usage for large DOCX files with many images

## Scraped Fixture Status

32 passing, 16 failing, 0 skipped out of 48 scraped fixtures. Breakdown of 16 failures by dominant issue:
- **text/layout only**: 8 fixtures
- **anchored images**: 4 fixtures
- **floating tables**: 3 fixtures
- **structured doc tags**: 2 fixtures (SDT content is extracted but wrapping may cause layout shifts)

Run `./tools/target/debug/analyze-fixtures --failing` for current breakdown.

## Test Harness: Surface Conversion Panics Loudly (TODO — HIGH PRIORITY, found 2026-06)

A library panic went unnoticed for an unknown number of runs: `new/construction_bathroom_accessories_spec` panicked in `cell_span_width` on every conversion, but the suite still reported "134 passed" with exit code 0. Three gaps compounded:

1. `tests/visual_comparison.rs` catches per-case panics and emits `[SKIP] <case>: conversion panicked` — visible only in `--verbose` output; the case silently gets no score, so the compact report's "N scored, N unchanged" looks green.
2. `run-tests.sh` greps `thread.*panicked` into a "Panics:" section, but the exit code stays 0 — nothing fails.
3. Conversion worker threads are unnamed, so panic messages show `thread '<unnamed>' panicked at src/...` with no case attribution — diagnosing required a separate verbose run.

Fixes:
- `run-tests.sh`: exit non-zero when the Panics section is non-empty.
- Harness/compact report: count panicked cases as failures and list them by name (`PANIC: new/construction_bat..`) in the compact output.
- Name conversion threads after the case (`std::thread::Builder::new().name(case.clone())`) so panic messages self-identify.

## Test Corpus Expansion

- Deep style inheritance (3+ level chains with run vs style vs paragraph conflicts) — **case50** (awaiting reference PDF)
- Nested tables (tables inside table cells, 2-level and 3-level nesting) — **case51** (awaiting reference PDF)
- Stacked bar chart rendering (stacked + percentStacked, vertical + horizontal) — **case52** (awaiting reference PDF)
- Charts with extreme data (50 categories, small/large/mixed-range values) — **case53** (awaiting reference PDF)
