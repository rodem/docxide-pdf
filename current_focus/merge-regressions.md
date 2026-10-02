# Merge regressions — fix plan (handover)

Status as of 2026-10-02. The `layout-accuracy` round (43 layout rules, see
`layout-accuracy.md`) is on `main`: first merged, then rebased linearly onto
`origin/main` by another session, so its commits have new hashes on `main`
(the old ones live on in branch `layout-accuracy`, which is now superseded;
`layout-accuracy.md` still cites the old hashes and still says "not merged").
`main` is ~72 commits ahead of `origin/main` and **not pushed**.

Every fixture that scored lower after the merge was traced to the commit that
moved it and diagnosed. All scores and visual hashes, regressions included,
were accepted with the user's approval in `bc6d9ded`, so the suite is green
and **will not flag these regressions any more**: this file and the roadmap
section "Layout-accuracy merge: regressions traced, fix plan" are the record.
Mean Jaccard went 51.2 → 63.4 and SSIM 70.9 → 80.5 over the merge.

## Working rules

- **Never accept baselines without the user's explicit approval.** Show
  `./tools/target/debug/accept-baselines --dry-run` first. The tool writes
  scores and hashes together; it has no `--scores-only`/`--hashes-only`, and
  any other `--flag` is ignored, i.e. runs a full accept.
- One fix per commit, each verified by a full suite run
  (`./tools/run-tests.sh`), no substantial regressions. Report the scores
  that moved and why.
- `touch src/lib.rs` before every suite run (the harness reuses a
  `generated.pdf` newer than `src/`). Run the suite with the sandbox off, in
  the background, long timeout (~10 min).
- Other Claude sessions work in the main checkout at the same time (one
  rebased `main` mid-session). Check `git status`, `git log` and
  `.git/rebase-merge` before writing there; prefer a worktree. A worktree needs
  `ln -s ../../fonts <wt>/fonts` (gitignored) and `DOCXSIDE_NO_FONT_CACHE=1`,
  or every score collapses to Arial fallback.
- Derive rules from Word's PDFs across many documents; never fit one fixture.
  Query the OOXML spec (`mcp__local-rag__query_documents`) before
  implementing; it was disconnected this session, so items 1 and 3 have not
  been checked against the spec yet.
- Check which face Word used for a run per glyph (`mutool draw -F stext`),
  never from the PDF's font list: that mistake is what made the current
  missing-font rule (item 5) cite german_mezzo wrongly.
- Never run `cargo fmt` on the whole repo. Word conversions are the user's job.

## How it was measured

- **Bisection:** the debug CLI built at every source commit of the round,
  each regressed fixture converted and scored with
  `tools/target/release/page-metrics <ref.pdf> <gen.pdf> <ref_png_dir>
  <gen_png_dir>` (harness-identical Jaccard/SSIM; rasterize with `mutool draw
  -F png -r 150`; build it with `cd tools && cargo build --release --bin
  page-metrics`). The first commit where a score moves is the culprit.
- **Diagnosis:** `VDIFF=tools/target/debug/vdiff python3 tools/ab_view.py
  <label> <cli> <group/case> -v` for each build before/after the culprit;
  compare the per-line `dy` columns. `tools/pdf_lines.py` for border rules and
  baselines, `tools/line_diff.py` for where wrapping differs,
  `tools/docx_edit.py` for what-if edits (e.g. forcing a font).
- **Censuses** (small Python scripts, not kept): iterate
  `tests/fixtures/*/*/input.docx` + `reference.pdf`, parse `mutool draw -F
  stext` (`<font name size>` spans, `<char quad x y c>`; names are cut at 24
  characters). Font availability = PostScript names (fontTools, nameID 6) of
  every file in the directories listed in
  `~/Library/Caches/docxide-pdf/font-index.tsv` (`D` lines), which include
  `fonts/CloudFonts/*`.
- SSIM tolerates ±8px (±3.84pt) of vertical offset, and Jaccard at 150 dpi
  reacts to sub-pixel shifts: a correct rule can lower a score by removing an
  error that cancelled another one. Read vdiff before calling something a
  regression.

## Fix plan (in this order)

### 1. Character styles inherit through `w:basedOn`

- **Problem:** `parse_styles` (`src/docx/styles.rs`, the `Some("character")`
  arm) keeps only a character style's own rPr and never follows `basedOn`.
  Paragraph styles do (`resolve_based_on`).
- **Evidence:** case50's MidChar (basedOn BaseChar: Georgia, bold, 14pt,
  green; adds italic, 12pt) and LeafChar (basedOn MidChar) draw in Cambria
  12pt; Word draws Georgia-BoldItalic 12pt. The narrower text wraps later
  ("…green (inherited), then" where Word breaks after "green"), so the second
  lines lack the 12pt run and per-line heights make page 2 1.35pt short
  (J 58.3 → 54.4). Section 6's 20pt run is Georgia-Italic in Word, Times New
  Roman in ours.
- **Change:** resolve each character style's chain after parsing (all
  `CharacterStyle` fields, own value wins). Toggle properties (b, i, caps…)
  have their own combination rules across style types (§17.7.3); check the
  spec for how they behave inside a basedOn chain before coding it. case50
  section 6 (paragraph style bold, LeafChar bold via BaseChar) is not bold in
  Word, but its run also sets `w:b w:val="0"`, so it does not settle the
  question.
- **Verify:** unit test in `styles.rs`; case50, carbon_farming_initiative_rule
  (5 runs), traditional_skills_job_form (4) are the only fixtures whose used
  character styles inherit formatting.

### 2. Super/subscript size: two thirds, rounded down to a half point

- **Problem:** `effective_font_size` (`src/pdf/layout.rs`) uses a flat 0.58
  for both.
- **Evidence:** polish_municipal "8³⁰" (12pt): Word's superscript digits are
  4.06pt wide against the 8's 6.08 (2/3 size = 8pt); ours 3.53 (0.58). Census
  of the 30 fixtures with `w:vertAlign` runs, small spans off the baseline
  over the line's main size: 0.667 (51), 0.636 = 7/11 (29), 0.65 = 6.5/10
  (18), 0.632 = 6/9.5 (7). Outliers: case10 0.583 (5),
  environmental_law_clinic_china 0.583/0.6 (83/82), pendulum 0.719 (8),
  brazilian_logistics 0.8 (8): check those for an explicit `w:sz` on the
  vertAlign run before trusting them.
- **Change:** size = floor(2/3 × size to 0.5pt), if the outliers agree. Also
  check the raise/lower offsets in `vert_y_offset` (+0.35 / −0.14 × size)
  against the same references: Word's superscript baseline in polish_municipal
  is 4.0pt above the line's (12pt text).
- **Verify:** ~30 fixtures move; widths of words with superscripts and line
  breaks around them.

### 3. Every space character is a justification stretch point

- **Problem:** a justified line spreads its slack over gaps, and a run of
  several spaces counts as one gap.
- **Evidence:** polish_municipal's "8³⁰  w sali…" (two spaces after 8³⁰):
  Word's double space is 7.44pt = 2 × (3.0 + 0.72 slack); single gaps on the
  line are ~3.72–3.9. Ours: 6.69pt for the double space, 3.98 for single
  gaps. With item 2 that accounts for the words after it sitting up to 1.3pt
  left of Word's (J 67.0 → 65.1, SSIM 85.2 → 83.0).
- **Change:** count space characters, not gaps, in the slack division
  (`left_gaps` / `extra_per_gap` in the render path). The `ponytail:` note on
  `char_justify_gaps` (top of `layout.rs`) describes the same missing per-chunk
  space count for `distribute` (`pending_space_w` keeps only the width, 5 push
  sites, ~8 resets); one carried count fixes both.
- **Verify:** find justified paragraphs with consecutive spaces across the
  fixtures first; check the squeeze rule (`SPACE_SQUEEZE`) still counts the
  same way Word does.

### 4. Line height for lines that mix East Asian and Latin runs

- **Problem:** since fix 11 (`5e613494`, "Size a line by its highest top and
  lowest bottom across runs") a line is max ascent + max descent over its
  runs. An East Asian run's ascent carries all of its 1.3× leading (it sits
  above the glyphs), so a Latin run's deeper descent stacks under it.
- **Evidence:** usep_handbook p5 checkbox lines (☐ in MS Gothic 12pt, text in
  Calibri 12pt, 6pt after): Word steps 21.6 = MS Gothic's full 1.3 × 12 =
  15.6pt line + 6. Ours 23.135 = MS Gothic's 13.91 (15.6 − win descent 1.69)
  + Calibri's win descent 3.22 + 6. +1.53pt per line, +19pt by the end of
  page 5 (SSIM 94.8 → 92.0).
- **Change (candidate):** line = max(Latin max-ascent + max-descent, each East
  Asian run's 1.3× box). `run_line_metrics` (`src/pdf/layout.rs`) and
  `compute_line_metrics` (`src/fonts/embed.rs`).
- **Verify:** case79 (25 fonts and sizes on docGrid, all 36 lines within 0.4pt
  of Word today) must not move; then the CJK fixtures. Grid-snapped lines use
  a different path (`grid_baseline_shift`).

### 5. Missing fonts in LibreOffice documents (census first)

- **Problem:** `register_font` (`src/fonts/mod.rs`) tries the fontTable
  altName, then each `;`-separated name, then aliases, then (family auto) the
  theme body font. Word does something else for these documents.
- **Evidence:** german_mezzo's "Archivo;sans-serif" (fontTable entry
  "Archivo", altName "sans-serif", family auto; theme Arial) is **Cambria** in
  Word's PDF (the "D" of "Diese Frau…" is the only visible glyph in it; the
  ~15 empty and break-only lines after the floating picture use it too).
  We draw Arial: each empty line ~0.26pt short, body 5.7pt high, outside SSIM's
  tolerance (SSIM 78.2 → 52.2 once fix 17, `4a2d9773`, sized those lines by
  the mark). **What-if:** the same docx with the font renamed to Cambria lands
  within 0.07pt: J 69.0, SSIM 81.2 (main was 55.5 / 78.2). sample500kB's "Open
  Sans;Arial" (fontTable "Open Sans", altName Arial, family roman) is
  **Segoe UI** in Word, Arial in ours. Both documents come from LibreOffice and
  Word ignored their altName. Cambria and Segoe UI are installed: this is a
  rule, not a missing file.
- **Caution:** the theme-body rule's code comment and its 2026-09-18 roadmap
  entry (#158 #195) cite "german_mezzo (theme Arial) embeds Arial". That was
  read off the PDF's font list (ArialMT comes from its explicit Arial runs).
  bosch (theme Calibri → Calibri) and three Calibri-theme fixtures still
  support the rule. Correct the comment in the commit that changes the rule.
- **Do:** census every run font that resolves nowhere (fixtures and the
  external corpus in `accuracy_push_local/`, see `layout-accuracy.md`) against
  the face Word drew per glyph, with fontTable altName/family/panose/charset
  for each; derive the rule; then change `register_font`.

### 6. Per-line heights in headers

- **Evidence:** japanese_land_development's header block (10.5pt CJK lines)
  steps 17.67pt in ours, 15.12 in Word, so the body starts 10.8pt low.
  (Its table rows are right since fix 14, `4e8910eb`: 27.50 vs 27.60, 26.60 vs
  26.64… One content-driven row, trHeight 520, is 35.64 vs Word's 30.48, a
  separate error.) SSIM 46.7 → 26.8 came from the correct rows no longer
  cancelling the header offset.
- Same item as pending fix 3 in `layout-accuracy.md` §7 (headers and cells
  don't use per-line heights).

### 7. Paginate inline endnotes

- erasmus_plus_staff_mobility_agreement: the correct endnote spacing
  (`9ecbcea4`, #185) makes the inline endnote block run past page 3's bottom
  margin, where the body already sits 30pt low (SSIM 55.9 → 52.4). See the
  `ponytail:` note on `render_endnotes_inline` (`src/pdf/footnotes.rs`).

### 8. Conference forms: 7pt short above the table

- east_asia_conference_form: every table row is within 0.015pt of Word, but
  the gap from the title block into the table is ~7pt short, and it fits on
  one page where Word has two.

### 9. Font files

Census of every face Word drew per glyph in the 223 references against all
indexed font files: only two fixtures use faces we lack. Look in Word's
cloud font cache first (`~/Library/Group
Containers/UBF8T346G9.Office/FontCache/4/CloudFonts/`, `diff -rq` it against
`fonts/CloudFonts/`).
- croatian_thesis_topic_approval_form: **done 2026-10-02.** Word's cloud
  cache held the static Merriweather 2.002 (Regular, Bold, Bold Italic); all
  42 widths in the reference match it. Copied into `fonts/CloudFonts/` and
  the assets repo (`59143d9`): 1 → 2 pages like Word, J 23.1 → 34.1, SSIM
  65.0 → 94.9 (release CLI + page-metrics, not yet a suite run).
- multi_font: Copperplate Gothic Light (81 glyphs). It is in Word's cloud
  catalog (`FontCache/4/Catalog/ListAll_hier.Json`, id 36453684816), but
  neither opening the document nor Mac Word's font menu offers it, though its
  catalog flags equal those of fonts Word did download (Algerian, Jokerman).
  Likely cause: Word ships Copperplate Gothic Bold, so the family counts as
  installed. The menu's "Copperplate" (Light/Regular/Bold) is Apple's
  `Copperplate.ttc`, a different design; "Copperplate Gothic" is the shipped
  Bold. A Windows Office machine has it as `COPRGTL.TTF`; copy it into
  `fonts/CloudFonts/` and the assets repo. Bodoni MT is already in
  `fonts/CloudFonts`.
- eco_int's 14 "Helvetica,Italic" glyphs are macOS Helvetica Oblique under
  another name, not a missing file.

## Known, not planned here

Older errors that correct rules exposed, and metric artifacts (details in the
roadmap section):
- air_pollution_permit_form: a 13.9pt excess at the top of the checklist text
  box (unchanged by fix 13, which made every auto-spacing gap match Word).
- dutch_government_budget_letter: a borderless one-row table ~2pt short
  (roadmap "Table Row Height Deficit"); the old +0.5pt/row fudge hid part.
- case61: text-to-border gap 0.25–0.5pt off; table 2's first column 9.6pt
  too wide.
- Positions as good or better, score lower: arizona_physical_education
  (fix 41, `0c5c92bd`), case67, case6, turkish_ancient_religions_plan,
  vaccines_history_chapter, lenten_prayer_unity,
  chinese_student_union_nomination_form, croatian_regulations_altchunk,
  construction_bathroom_accessories_spec.
- multi_font's round-4 drop (`96fbce99`): Copperplate Gothic Light falls back
  to Arial with real widths and fits on one line where Word wraps (item 9).
