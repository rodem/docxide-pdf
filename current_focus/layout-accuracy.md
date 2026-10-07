# Layout accuracy — current focus

Working document for making our layout match Word's: where it stands, what
to do next, how to measure, the open queue and the findings that still
matter. Finished rules are not listed here: `git log` has one commit per rule
with its evidence, and `roadmap.md` summarises each round ("Layout accuracy
round", "Large-corpus round", "Focus-fixture round").

## 1. Status (2026-10-08, `main` at `cc0c4c45` + this doc)

Everything is on `main`; no open branch. Fixture means (Jaccard, snapshot
`tests/output/snapshots/ftable2.json`):

| group | n | J |
|---|---|---|
| cases | 79 | 75.36 |
| scraped | 148 | 63.69 |
| fonts | 7 | 66.46 |
| hyphenation | 8 | 63.71 |
| samples | 5 | 55.03 |
| excluded (local focus set, §4) | 11 | 44.57 |

**Baselines are not yet accepted** for the focus-fixture round (10 rule
commits `f912a033..40275abe`). The suite reports estonian and ukrainian as
regressions until the user approves new baselines or §2.1 fixes them.

External clean corpus (2,480 Word for Mac documents): 69.37 J before the
focus-fixture round; not re-scored as a whole since (each rule was A/B-tested
on its own subset, §3).

## 2. Next steps, in order

### 2.1 The two fixture regressions of the floating-table rule (`40275abe`)

That rule pushes a paragraph whose first line reaches a floating table's top
below the table. It is right for both documents; each regression is a second,
pre-existing error that the old (wrong) placement hid. Fix these first.

- **scraped/estonian_community_development_grant: 50.3 → 42.5.** Page 3: the
  floating table's empty anchor now goes below the table, so the heading
  "4. Näitajad" sits at 762.14 (Word 760.73; it was 737.41, 23pt high). The
  next paragraph ("Lehel saate sisestada…") needs two lines at the page foot
  (widow control); Word fits them at 776.73 and 789.23, ours land 1.4pt lower,
  the second misses, and the whole paragraph moves to page 4, shifting the
  rest of the document. The 1.4pt: the table above ends 0.65pt lower than
  Word's (717.85 vs 717.20) and the gap to the heading is 0.76pt larger
  (44.29 vs 43.53). §12 of the old doc also saw this document drift 0.15–0.4pt
  per line through its opening paragraphs. Find which of those offsets is a
  rule (cell/row heights, the anchor line's height or spacing), not a fudge.
- **scraped/ukrainian_municipal_heating_resolution: 56.7 → 53.0.** Page 3
  opens with a paragraph we wrap onto one more line than Word (Word: lines at
  85.78, 102.03, 118.03; ours 85.87, 101.97, 118.07, 134.16), so everything
  below is 16pt low. Word also pushes the floating table's empty anchor below
  the table (text resumes 29.25pt under it); before the rule our missing
  anchor line made up for the extra wrapped line. Fix the wrap (check its
  justification and tabs: `build_tabbed_line` has no compat-15 space squeeze,
  §5 item 3).

Commands: `python3 tools/line_diff.py <ref> <gen>`, `python3 tools/pdf_lines.py
<pdf> <page> [ymin ymax]`, `python3 tools/word_x_diff.py <ref> <gen> <page>`.

### 2.2 Remaining focus fixtures (§4)

1. **potamites (14.0): EMF drawing.** `docx/emf.rs` parses only paths, pens
   and brushes (`EmfRecord`). Needed: font objects
   (`EMR_EXTCREATEFONTINDIRECTW`) and text (`EMR_EXTTEXTOUTW`, through our font
   embedding), standalone `MoveToEx`/`LineTo` strokes outside paths,
   `EMR_RECTANGLE`, and `EMR_BITBLT`/PatBlt fills without a source. Then apply
   the parked `accuracy_push_local/patches/wmf-embedded-emf.patch` (use a WMF's
   embedded EMF, skip pattern blits): it scores lower until EMF text and lines
   render. 54 corpus documents carry text in metafiles.
2. **cyprus (32.3, 2 pages vs Word's 3):** the footer float no longer inflates
   the footer; a second cause leaves us one page short. Not diagnosed.
3. **maine (5.0):** nested layout tables with `tblCellSpacing`; column widths
   (text 581pt wide where Word wraps at 555) and cell-spacing row heights
   (page 1 rows 1.7–6pt short). Two nested floating tables (`tblpPr` in a
   cell) are stacked instead of side by side (`render_nested_table` ignores
   `position`; 8 corpus documents).
4. **arabic (3.0):** complex script. `rFonts@cs`, `w:rtl`, `szCs`/`bCs` are
   never read (`docx/styles.rs`, "ponytail: w:bidi ignored"); Word draws the
   text in the cs font (B Nazanin → its altName, Faruma → MV Boli, B Zar /
   IRANYekan / none → Arial). The font alone gains little: the text needs UAX
   #9 reordering and Arabic shaping (roadmap "RTL / BiDi", high effort). Ask
   the user before starting.

## 3. How to measure and work

```bash
tools/score_snapshot.sh <label> [<prev>]       # full suite → tests/output/snapshots/<label>.json + comparison
python3 tools/compare_scores.py a.json b.json  # group means + every moved case
python3 tools/corpus_score.py <dir> <cli> <label>   # <dir>/docx + <dir>/pdf → tests/output/corpus/<label>.json
python3 tools/corpus_compare.py <before> <after>    # labels, not paths
python3 tools/word_export.py <docx>... --preset online   # Word probe / reference (sandbox off)
python3 tools/docx_edit.py <in> <out> <part> <old> <new> # literal what-if edit
```

Diagnostics: `pdf_lines.py` (rules and baselines, y from page top),
`line_diff.py` (where reflow starts), `word_x_diff.py`, `ink_diff.py`. vdiff
(per-line baseline drift) lives on branch `systemic_work`; a binary is at
`tools/target/debug/vdiff`.

**Local, untracked resources** (`.worktrees/layout-accuracy/accuracy_push_local/`):
- the 2,480-document clean corpus (its path is in the local handover
  `tests/fixtures/excluded/HANDOVER.md`; references are Word's **print**
  export, fixtures use **online** ones).
- `subsets/<construct>/` — corpus subsets per rule (docx + pdf), e.g.
  `float_table`, `keep_lines`, `row_keep`, `hf_push`, `hm_all`.
- `bin/docxide-<label>` — pinned CLI builds; `docxide-ftable2` is current
  `main`'s rule set.
- `diagnosis/{page1_offset,page_drift,missing_text,fonts}.md` — per-cause
  evidence from the corpus triage (corpus IDs inside: never copy them out).
- `patches/wmf-embedded-emf.patch` — parked (§2.2).

**Per rule:**
1. Diagnose on the fixture; confirm Word's behaviour with a probe document
   (python-docx, or `docx_edit.py` variants of the fixture) exported by
   `word_export.py`. Diagnoses have been wrong: three of this round's were
   (hideMark, header floats, keepNext rows); probe before coding.
2. Before editing: pin the current CLI (`cp target/release/docxide-pdf
   <local>/bin/docxide-<label>`).
3. Implement; check the fixture with the release CLI (`DOCXSIDE_FONTS=$PWD/fonts`,
   delete the output first: the CLI never overwrites).
4. Build a corpus subset of documents carrying the construct; A/B the two
   pinned builds; explain every document that drops (usually a second error
   the old behaviour hid; if not, narrow the rule).
5. `tools/score_snapshot.sh <new> <prev>`; explain every fixture that moves.
6. Commit the rule alone, evidence in the message.

**Working rules and gotchas:**
- Derive rules from Word's output across documents; never fit one document.
  Document names in comments are examples, never conditions.
- Never accept baselines without the user's approval (scores and hashes are
  separate approvals).
- Never put corpus document IDs, the corpus's source or any competitor name in
  commits, tracked files or comments. Fixture names are fine.
- The snapshot builds from the working tree. To measure one rule while
  another is uncommitted, set the other aside (`git diff <files> > patch;
  git checkout <files>`), start the snapshot, wait until the
  `visual_comparison-*` binary runs (`pgrep -f deps/visual_comparison-`),
  then `git apply` it back. Never edit `src/` while it compiles.
- cargo, the suite, `word_export.py` and `mkdir`/heredocs in the repo need the
  sandbox off here. System `python3` is 3.9.
- Corpus references are print exports and can disagree with online ones (a
  Book Antiqua document: 10.8 vs print, 77.3 vs online). Trust a fixture's
  online reference; check corpus-only findings against an online export.
- Jaccard at 150 DPI punishes 1–2pt offsets and is non-monotonic (13pt off
  can beat 6pt off). Compare baselines (`pdf_lines.py`) before calling a
  change a regression.
- Fonts in `~/Library/Fonts` outrank system fonts in our discovery; a big
  user font folder collapses the suite. Worktrees need `fonts -> ../../fonts`.
- `cargo fmt` on `main` reformats two pre-existing spots in
  `src/pdf/layout.rs` (`push_decoration` and its call); leave them out of rule
  commits or format them in a commit of their own.
- Word's PDFs carry a whitespace-only text line per empty paragraph mark;
  match lines by text, not by "first line". Word count beats text recall for
  Arabic, CJK and ligatures. Compare fonts by visible glyphs (Word embeds
  faces that draw only spaces).

## 4. Focus fixtures (`tests/fixtures/excluded/`, local only)

Corpus documents copied in under descriptive names, one per open cause;
gitignored, never committed. References are online exports. The suite runs
them; without baselines they show as "new" and never fail.

| fixture | J (start → now) | status |
|---|---|---|
| chiseldon_parish_planning_agenda | 5.7 → 69.0 | done |
| bulgarian_farmland_allocation_order | 5.2 → 66.0 | done |
| renewable_dispatch_committee_agenda | 49.5 → 71.0 | done ("Agenda" text box 2.6pt low, untriaged) |
| physical_education_curriculum_map | 27.9 → 68.5 | done |
| australian_higher_education_guidelines | 34.2 → 65.3 | done (35 pages as Word) |
| czech_village_budget_commentary | 11.3 → 51.1 | header 1.1pt high; a later header line ("IČO … e-mail") lays out differently |
| italian_academic_cv_form | 28.1 → 45.1 | page breaks as Word; remaining gap untriaged |
| cyprus_ucits_marketing_registry | 29.9 → 32.3 | §2.2 item 2 |
| potamites_genetic_distance_table | 14.0 | §2.2 item 1 |
| maine_criminal_history_record | 4.5 → 5.0 | §2.2 item 3 |
| arabic_rice_benefits_article | 3.0 | §2.2 item 4 |

## 5. Open queue (diagnosed, not fixed)

Ordered roughly by expected gain. Corpus counts are clean documents.

**Pages and flow**
1. A paragraph holding only a page break on a full page lays out its line
   first, so a blank page follows (one 107-page document; what-if 11 → 76).
2. Page-relative topAndBottom floats reserve their absolute offset in the
   anchor paragraph (5 documents).
3. No compat-15 space squeeze in tabbed justified lines (`build_tabbed_line`
   wraps on plain overflow; count only the gaps after the last tab; 24
   fixtures have justified tab paragraphs). Likely ukrainian's extra line.
4. Endnotes never continue onto a new page; a paragraph taller than a page at
   the page top never splits.
5. ~18 corpus documents drift 1–3.5% per line (Book Antiqua 17.40 vs 16.80,
   Helvetica, Verdana): line-height metrics for those faces. Check against
   online exports first.
6. croatian_grant (71 vs 65 pages): page 28 starts with an extra line in Word.
7. Inline picture `distT`/`distB` are ignored by Word (8 corpus documents, +9
   and +18pt shifts); a trailing `w:br` after an inline picture keeps a line
   sized by the mark (5 documents).

**Tables**
8. Tables and content controls inside text boxes are dropped
   (`parse_txbx_content_paragraphs` reads only `w:p`; 9 documents).
9. Table cells and footnotes build paragraphs by hand (`docx/tables.rs`,
   `headers_footers::parse_notes_simple`), not via `paragraph::build_paragraph`:
   cells miss keepLines, widowControl, the mark font, paragraph shading and
   borders, tab stops (incl. the implicit hanging stop); footnotes miss
   `style_id`/contextualSpacing. keepNext was added by hand in `d8603f41`.
   Switching moves many fixtures: own commit, full snapshot.
10. Table row keepNext uses a first-cell heuristic (`d8603f41`, three measured
    documents); probe a row with a mixed first cell.
11. dutch_government: a page-anchored floating table moves the next body table
    2.4pt down in Word, not to the float's bottom.
12. radiographer: Word also splits inside a nested row between its lines; a
    list label hanging left of its cell is not drawn by Word (cell clipping?
    probe first).
13. Legacy table-style font size (`overrideTableStyleFontSizeAndJustification`
    absent, compat < 15: the table style's size wins unless 10pt).
14. romanian_quality: an at-least row is trHeight + both cell margins (67.5 =
    60.2 + 2 × 3.6) though the content is shorter; one sample.

**Headers, floats, frames**
15. A floating header table neither wraps header text nor extends the header
    (no fixture shows Word doing either; one corpus header case pushes the next
    header paragraph to 49.5).
16. `framePr yAlign="inline"` with a width or position (none seen) is treated
    as a plain paragraph (`docx/mod.rs` ponytail).
17. Header float push (`46aaf1ac`) uses "no room = float as wide as the text";
    measure side gaps if a partly covering float turns up.

**Text and fonts**
18. Online references place glyphs with rounded advances (Calibri 'o' 6.003pt
    at 11.5pt vs 6.06); an empirical 0.07% shrink gains +0.6 J but is not a
    rule yet (`current_focus/online-references.md` §4a).
19. `w:lvlJc` is not parsed: right-aligned "1." markers end at the indent in
    Word, start there in ours.
20. Missing-font substitution: Cambria/Calibri holds for ~95% of 103 truly
    missing fonts; PANOSE is not Word's rule; a few names map specially
    (TimesLT → Times New Roman, Myriad Pro → Segoe UI). 14 Word probes are
    prepared in `diagnosis/scratch/fonts/probe/`, not run.
21. docDefaults without `rFonts` → Times New Roman (not the theme font); the
    bidi theme slot defaults to "Arab".
22. Highlights fill the line box (ours: y − 0.2 fs, 1.15 fs); a highlighted
    mark also covers the list label; small-caps spaces drawn at 80%.
23. Connector `relativeFrom` and `cmpd="thickThin"` ignored.
24. sao_paulo: NBSP stretching in compat-14 justified lines (3 samples).
25. Mac Word's synthetic bold: stroke 0.02 · size + 0.12pt and a 1/1.02
    vertical squash (6 Mac references).

**Undiagnosed low fixtures** (J now; scraped/ unless noted):
samples/sample500kB 14.7, pendulum_mechanics_oscillation_lab 18.3,
chinese_costume_design_course 20.5, cases/case82 21.2 (3 vs 4 pages),
slovak_pedagogical_practice_agreement 21.6, cases/case37 23.2,
indonesian_school_admission_checklist 27.3, dutch_government_budget_letter
28.7, czech_municipal_grant_form 34.5, scottish_fundraising_awards_campaign
34.9, fonts/multi_font 34.9, japanese_land_development_sign_form 35.7,
uk_commercial_lease_template 37.9 (78 vs 79 pages),
polish_ministry_accessibility_report 37.9.

**Out of this topic:** tracked changes (redline markup, roadmap item; tracked
corpus states score 15–30), metafile text beyond EMF (vector WMFs come out as
a brush-sized box), cloud fonts we don't vendor (~43 corpus documents:
Poppins, Ubuntu, Segoe UI Light/Semibold, Roboto Light/Medium, …).

## 6. Findings that still matter

- **Online vs local Word.** Online references kern with the font's pair
  kerning (TJ kerning shipped) and round glyph advances; local (print)
  exports use plain advances. Lines step on a 0.25pt grid online, 0.24pt
  locally.
- **Reference artifacts.** 4 stray bytes after a zip's end-of-central-directory
  trip Word's repair prompt; a repaired reference can differ from Word's real
  layout (strategi, education_consultant were replaced). Strip before
  exporting.
- **East Asian lines:** 1.3 × the hhea box with the leading split above and
  below (online); cells use 1.0 em for the first baseline; grid documents
  centre the glyph box in the snapped cells (see `CLAUDE.md`).
- **Word fonts on Mac:** Mac Word draws its own Microsoft Symbol and the
  macOS Times New Roman, never Apple's Times; cloud fonts download into
  `~/Library/Group Containers/UBF8T346G9.Office/FontCache/4/CloudFonts/`
  (vendored ones live in `fonts/CloudFonts/` and the private assets repo).
- **Keep-with-next (this round):** each kept paragraph brings the lines that
  must stay together (all of keepLines, else the widow-control count); a chain
  passes only through a paragraph that stays whole; a chain longer than a page
  starts a page and its later members keep only their own link. Word probes
  for hideMark: only the document's first mark is never hidden; a hidden
  trailing mark drops its spacing.
- **Header floats (probes):** topAndBottom pushes header text at any width,
  square only with no room beside it; a float starting below the paragraph top
  keeps the first line above it; behindDoc never pushes.

## 7. Decisions

1. Merges into `main` happen when the user says so; this round worked on
   `main` directly.
2. Baselines for the focus-fixture round: waiting for the user.
3. Tracked-changes rendering: on the roadmap; markup vs final text as the
   default is open.
4. The focus fixtures stay local (gitignored); the corpus and its IDs never
   leave the machine.
