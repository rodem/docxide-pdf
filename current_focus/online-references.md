# Online references — current focus

Handover for the switch of every fixture reference to Word's **online** PDF
export, and for the accuracy gap that switch exposed. Written 2026-10-03 at
the end of the session that did the work. Short version: the references are
now all online (one exception), two layout rules that differ online are
fixed, and three big gaps remain (glyph widths, the comment/markup pane, and a
handful of fonts). Baselines have **not** been accepted since the switch.

## 1. Status

- `main`, not pushed. Commits of this round, oldest first:

| Commit | What |
|---|---|
| `54b917ce` | Glued over-wide words break at the margin; page numbers follow the section at the page top (strategi work; committed by another session, squashed) |
| `9a83c1cf` | strategi: stray bytes removed, reference re-exported (local preset then) |
| `0875a1ef` | education_consultant: same |
| `9cb1039f` | The 110 other scraped inputs with stray bytes re-downloaded |
| `82ba9482` | `word_export.py --preset online\|print`, default online |
| `802b1d93` | 38 scraped references: local → online (includes strategi, education_consultant) |
| `dc0fc19f` | 20 online references refreshed (service drift, see §3) |
| `de03ae5d` | The remaining 32 local references → online |
| `a9746f5f` | Keep-with-next chain ends at a paragraph that opens with a page break |
| `8e7d5782` | A page break ending a section's last paragraph is dropped |
| `2cc66a38` | Notes in `layout-accuracy.md` |

- Every `reference.pdf` is now Word's online "Best for electronic distribution
  and accessibility" export (tagged, Creator "Microsoft Word") **except
  `fonts/missing_font_substitution`**, held back on purpose (§6).
- **Baselines are stale.** `tests/baselines.json` / `visual_hashes.json` still
  hold scores against the old references, so the suite reports ~60
  regressions and fails `visual_comparison`. Nothing is broken; accepting needs
  the user's go-ahead (show `accept-baselines --dry-run` first).
- `README.md` has uncommitted changes that are not from this work (another
  session). Leave them alone.

**Scores** (full visual suite, `tools/score_snapshot.sh`; snapshots in
`tests/output/snapshots/`, gitignored):

| | mean J, 244 fixtures |
|---|---|
| old references (`tests/baselines.json`) | 64.30 |
| new references, before the two rules (`online-pre-kn`) | 61.08 |
| new references, after the two rules (`online-kn`) | **61.41** |

66 fixtures moved more than 0.5 J with the new references: 59 down, 7 up.
Biggest movers (baseline → now, J):

| fixture | J | cause (§4) |
|---|---|---|
| door_air_cooling_unit_spec | 82.4 → 5.2 | markup pane |
| cases/case64 | 74.9 → 2.7 | markup pane |
| cases/case63 | 35.1 → 1.0 | markup pane |
| indonesian_school_admission_checklist | 73.5 → 26.2 | not investigated |
| dutch_council_member_resignation | 94.7 → 58.2 | glyph widths |
| slovak_misdemeanor_amendment | 88.3 → 57.5 | glyph widths |
| greek_history_lecture_press_release | 79.1 → 49.0 | fonts (real Comic Sans italic online) |
| czech_census_2021_instructions | 80.2 → 51.2 | glyph widths |
| samples/sample500kB | 38.4 → 14.6 | fonts (DejaVu Sans → Sylfaen online) |
| welsh_palliative_care_abstract_form | 77.3 → 54.4 | not investigated (mean word drift 4.2pt) |
| lithuanian_food_quality_order | 84.9 → 63.0 | glyph widths |
| family_kinship_lesson_plan | 91.3 → 71.2 | glyph widths + one rewrap |
| mandated_reporter_child_abuse | 79.4 → 59.3 | glyph widths |
| samples/double-underline | 68.7 → 50.4 | not investigated |
| feminist_voice_dissertation | 78.7 → 61.6 | TOC differs (not investigated) |
| usep_handbook | 87.7 → 79.5 | was 41.1 before `a9746f5f` |
| stem_partnerships_guide | 50.1 → 66.6 | was 23.9 before `8e7d5782` |
| master_thesis_learning_agreement | 23.3 → 42.0 | online ref refresh |

## 2. The stray bytes (done)

- 112 scraped inputs ended in `\r\n\r\n` after the zip's end-of-central-
  directory record. Zip tools and our parser ignore it; **Word for Mac asks
  "Word found unreadable content… recover?"** and lays out the recovered copy.
- Origin: docxcorp.us. The corpus manifest (`../docx-corpus/manifest.txt`)
  holds their SHA-256 *including* the bytes; the site now serves the same
  hashes without them, and fresh downloads are clean (12 checked). The 58
  clean scraped fixtures are not in that manifest (another source).
- All 112 inputs were replaced by re-downloads (`9cb1039f`, plus the two
  earlier ones); each was checked to equal the old file minus the 4 bytes.
- Did the repair damage references? Method that changes one thing at a time:
  export the *original* file **accepting** the repair with today's Word and
  compare it with today's export of the clean file (only the bytes differ),
  and with the old reference (only Word's version differs).
  - Local references: only **strategi** and **education_consultant** were
    damaged (repaired-today = old ref pixel for pixel, ≠ clean; Word lists a
    "Footnotes" repair). Of the other 36, 31 matched exactly and 5 differed
    only by Word-version drift (stem_partnerships, usep, alpharetta,
    east_asia, russian_university; repaired = clean today).
  - Online references: none damaged; 7 that differed were service drift.
- Mechanics of a repaired export: Word opens the recovered copy as an unsaved
  **"Document1"**, then shows a **"Show Repairs"** window that blocks
  AppleScript until closed (its list is readable through System Events:
  `table 1 of scroll area 1 of window "Show Repairs"`). Script:
  `tests/output/rezip/export_repaired.sh` (local, gitignored).

## 3. Exporting references

- `tools/word_export.py` now picks the PDF preset itself: Word stores the
  Save As "Optimize for" radio button in Office's settings database,
  `~/Library/Group Containers/UBF8T346G9.Office/MicrosoftRegistrationDB/MicrosoftRegistrationDB_*.reg`
  (SQLite), value **"Use BCS Service to create tagged PDF"** under
  `Software\Microsoft\Office\16.0\Word\Options` (1 = online, 0 = print).
  `--preset online` (default) / `--preset print` write it only when it
  differs, and refuse while Word is running (Word may read it only at launch;
  the tool never quits a Word the user has documents open in).
  - The value is 1 now, so normal runs only read it.
  - Writing it needs the user's permission: the auto-mode classifier blocked
    one write as "persistence". A backup from before the first change is in
    `tests/output/scrape_test/regbackup/`.
  - Verified: an online export with the tool is pixel-identical to a committed
    online reference (czech_cafeteria_notice).
- Pacing: 20 s between files, 120 s back-off and one retry after a failure.
  ~150 online exports in this session hit **no rate limit**. Runner:
  `tests/output/online_reconvert*/run.sh` (one `word_export.py` call per file,
  `--keep-word`, skips files already exported).
- Online exports are ~2–20 s each. They go through Microsoft's service, so
  the document leaves the machine (fixtures are public documents).
- **Only one process may drive Word.** Two sessions exporting at once close
  each other's documents (`close` errors, "Word did not open"). Check
  `pgrep -lf word_export` first. The other session's probes now also go
  online unless it passes `--preset print`.
- The tool's watchdog answers the repair prompt "No"; with clean inputs it no
  longer appears.

## 4. Why online references score lower

### 4a. Glyph widths (the big one, open)

- Online Word places glyphs with **rounded advances**; local Word and we use
  the font's exact widths. Measured on dutch_council_member_resignation
  (Calibri 11.5pt, no kerning, compat 15), first line, advance per glyph:

| glyph (units/1000) | exact at 11.5pt | ours | local Word | online Word |
|---|---|---|---|---|
| D (615) | 7.07 | 7.073 | 7.076 | **7.003** |
| o (527) | 6.06 | 6.061 | 6.062 | **6.003** |
| d (525) | 6.04 | 6.037 | 6.039 | **6.003** |
| r (349) | 4.01 | 4.014 | 4.011 | 4.014 |
| e (498) | 5.73 | 5.727 | 5.716 | 5.727 |
| i (229) | 2.63 | 2.634 | 2.640 | **2.737** |

  - The embedded /Widths are identical in both exports (same Calibri), so it
    is positioning, not the font.
  - Local Word draws this text at **11.52pt** (a 0.24 multiple, like its line
    grid); online draws exactly 11.5pt.
  - Per-line word drift (`tools/word_x_diff.py`, mean of max |dx| per line):
    ours vs local 0.04–0.11pt, ours vs online **~0.38pt**, on dutch_council,
    czech_census, lithuanian_food, mandated_reporter. Jaccard punishes this
    hard: dutch_council lost 36.6 J with every line in place.
  - This affects **all** online references, so it likely caps many of the
    ~174 that were online before the switch too.
- Hypotheses to test: advances rounded to whole pixels at some ppem (96 dpi
  does not fit 'D'), the font's `hdmx` device widths, GDI-style hinted
  advances, or rounding of absolute pen positions (which would make the same
  glyph's advance vary along a line, as the space does: 2.484 / 2.495).
- How to measure: `tests/output/dig/adv.py <pdf>` prints per-glyph advances of
  the longest line from `mutool draw -F stext`; collect (font, size, glyph,
  online advance) across many online references and fit a rule. Check
  `memory/kerning_and_shaping.md` for earlier width work first.

### 4b. Markup / comment pane (open)

- The online export prints **without markup**: no comment pane, no balloons
  for comments or tracked deletions, page at full size. The local export (and
  our renderer, rule `62240f49` measured on local exports) draws the scaled
  page plus the pane. That is the whole of door_air_cooling (82.4 → 5.2),
  case63 (35.1 → 1.0) and case64 (74.9 → 2.7).
- Side by side: `tests/output/dig/hand/*_side.png`. Decision needed: drop the
  pane to match online (and keep the code behind a switch?), or keep it.

### 4c. Fonts (open)

- The online service has Microsoft's font set, not the Mac's:
  - greek_history: real ComicSansMS-Bold/-Italic online, where local Word
    (and our `f47b2afb` rule) shears the regular face.
  - sample500kB: DejaVu Sans (local) → Sylfaen (online).
  - learning_cultures, wa_child: Times / Sylfaen appear online.
  - estonian: Arial Narrow Italic/BoldItalic online, synthesized locally.
- Rules measured on local exports may need re-checking against online:
  missing-font substitution (`memory/font-family-auto-substitution.md`),
  synthetic italic, Mac-only faces.

### 4d. Layout rules (two fixed, more to find)

Probes in `tests/output/probe_kn/` (generator `mk.py`, exported online):

| probe | online Word | us before | fix |
|---|---|---|---|
| keepNext heading → paragraph holding only a page break (own or Normal style) | heading stays | heading moved, extra page | `a9746f5f` |
| keepNext heading → page break + text in one paragraph | heading stays | moved | `a9746f5f` |
| keepNext heading → "text + page break" | heading stays | same | — |
| "text + page break" ends a normal paragraph | break | same | — |
| "text + page break" ends a section; next continuous | **no break** | break | `8e7d5782` |
| same; next section new page | one new page | two | `8e7d5782` |

- Untested: keepNext into a paragraph with the `pageBreakBefore` *property*
  (still treated as unkeepable); a page break followed by an empty paragraph
  that holds the sectPr; a page break ending the document.
- Other online-vs-local layout differences seen but not investigated:
  chinese_costume (3 pages local, 2 online), feminist_voice (TOC),
  family_kinship and russian_regional (one line wraps differently), usep's
  later pages.
- Many `line_diff` first divergences between local and online are list labels
  extracted separately in the tagged PDF ("3. Údaje" vs "Údaje"), not layout.

## 5. How to measure

- Suite: `tools/score_snapshot.sh <label> [<prev>]` (full visual suite →
  snapshot JSON, prints group means and movers). Before/after a code change,
  park the change as a patch (`git diff -- files > x.patch; git checkout --
  files`), snapshot, `git apply`, snapshot again, so the references are the
  same in both runs. Always `touch src/lib.rs` before a run.
- Old local references for comparison: `git show 9cb1039f:<path>` for the 38,
  `git show de03ae5d~1:<path>` for the 32.
- Local scratch from this session (gitignored, may be cleaned): survey scripts
  `tests/output/dig/survey*/`, glyph-advance script `tests/output/dig/adv.py`,
  comparison scripts `tests/output/rezip/pair.py`,
  `tests/output/online_reconvert/compare.py`.

## 6. Open decisions for the user

1. Accept baselines for the new references (scores + hashes)?
2. `fonts/missing_font_substitution`: convert to online? It records which
   font Word substitutes for missing ones; online picks from Microsoft's font
   set (e.g. Times/Sylfaen where local chose Helvetica Neue), so the fixture
   would test a different thing.
3. Markup pane: match online (no pane) or keep it?
4. Next accuracy target: glyph widths (biggest, affects every online
   reference) vs the smaller items above.
