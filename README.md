# docxide-pdf

Library and CLI for converting DOCX files to PDF, matching Microsoft Word's output as closely as possible.

[Try the demo!](https://docxide-demo.fly.dev/)

**🫶 Accessible:** Output PDFs should be just as accessible as Word's export, or better: tagged structure, reading order, language and metadata that pass the same PDF/UA checks.

**🎯 Accurate:** Given a `.docx` file, produce a `.pdf` that is visually identical to what Word would export.

**⚡️ Fast:** Typical conversions complete in under 100ms.

**🤏 Small files:** Output PDFs should be the same size or smaller than Word's export.

Reference PDFs are generated using Microsoft Word for Mac (16.106.1) with the "Best for electronic distribution and accessibility (uses Microsoft online service)" export option. (48 of the 222 references went through the macOS print path instead and are untagged; they are being re-exported — see `roadmap.md`.)

## ⚠️ Work in progress.

This crate **might** work for your production case, do give it a try! The API, output quality, and supported features are all actively changing.

## Comparison with other converters

[Side-by-side comparison](https://sverrejb.github.io/docxide-pdf/)

[Score table](https://sverrejb.github.io/docxide-pdf/#scores)

Every fixture in the test corpus rendered side by side by Word (the reference) and 

* docxide-pdf (duh)
* LibreOffice
* [MiniPdf](https://github.com/mini-software/MiniPdf)'s Rust crate
* [rdocx](https://crates.io/crates/rdocx)
* [office2pdf](https://github.com/developer0hye/office2pdf).

Each engine is scored against the Word reference with the same three metrics the test
suite uses:

| Metric | What it measures |
|---|---|
| **J** (Jaccard) | Overlap of ink pixels at 150 DPI. Strict: a one-line vertical shift sends it toward zero. |
| **SSIM** | Structural similarity on 8×8 windows with ±8 px vertical tolerance, so small drift is forgiven. |
| **TB** (text boundary) | Share of lines whose first and last word match the reference. Measures line breaking and pagination, independent of fonts. |


## Got a weird DOCX?

If you have a `.docx` file that produces ugly, broken, or just plain wrong output, send it to me! Real-world documents with surprising formatting are the best way to improve. Open an issue or PR with the file included and I will try to make it work.


## AI usage disclaimer 🤖

While the idea, architecture, testing strategy and validation of output are all human, the vast majority of the code as of now is written by various Claude models with access to the PDF specification (ISO-32000) and the Office Open XML File Formats specification (ECMA-376). This project was done as an exercise to get experience with the usage of coding agents.

## Supported features

- **Accessibility**: tagged PDF with a structure tree in reading order — paragraphs and headings (H1–H6 from outline levels and built-in heading styles), lists (L/LI/Lbl/LBody), tables (THead/TBody/TR/TH/TD with `/Scope` and `/ColSpan`), figures with alt text from `wp:docPr/@descr` (decorative pictures as artifacts), links, footnotes/endnotes as Notes, TOC/TOCI; headers, footers, line numbers and decorations as artifacts; document language plus per-passage `/Lang` from `w:lang`, XMP metadata, DisplayDocTitle, `/Tabs /S`, `/ActualText` for caps/small caps, real space glyphs between words. A PDF/UA-1 identifier is written only when the document passes every check (title, alt text, heading order, embedded fonts). Scored against Word's own tagged export, see [SCORING.md](SCORING.md)
- **Text**: font embedding (TTF/OTF/TTC), bold, italic, underline, strikethrough, double strikethrough, font size, text color, superscript/subscript, small caps, all caps, character spacing, text expansion/compression (`w:w`), hidden text (`w:vanish`), kerning (legacy kern table + GPOS PairAdjustment), vertical text (CJK), run borders with color/width/spacing, run shading (`w:shd`), legacy text effects (`w:outline`, `w:shadow`, `w:emboss`, `w:imprint`), UAX #14 line breaking
- **Paragraphs**: left/center/right/justify/distributed alignment (`distribute`), space before/after, line spacing (auto, exact, at-least), first-line and hanging indentation (including character-unit `w:firstLineChars`), left/right indentation, contextual spacing, keep-next, keep-lines, paragraph borders (top/bottom/left/right/between) with color and Word's border-group joining, paragraph shading, run highlighting
- **Styles**: paragraph and run style inheritance (`basedOn` chains), document defaults from `docDefaults` (all run properties: bold, italic, caps, smallCaps, vanish, strikethrough, dstrike, underline, color, char_spacing), theme fonts and colors
- **Lists**: bullet and numbered lists with multi-level nesting, custom number formats (incl. CJK: `decimalEnclosedCircle`, `decimalFullWidth`, `aiueoFullWidth`), list style inheritance, `w:lvlRestart`, `w:pStyle` level association
- **Tables**: column widths with auto-fit, percentage table widths, merged cells (horizontal `gridSpan` and vertical `vMerge`), row heights (exact and minimum), per-cell borders with color/width, inline `w:tblBorders`, cell shading, pattern/hatch shading, vertical alignment, cell text direction (rotated cells), cell margins, floating/positioned tables (`tblpPr`), nested tables, conditional formatting (`tblLook`/`tblStylePr` — banded rows/columns, first/last row and column), style and inline border merging per side, repeating header rows (`tblHeader`), Word-compatible row splitting across pages (between lines of a cell paragraph; rows with `trHeight` move whole)
- **CJK text**: CIDFont/Identity-H/ToUnicode encoding, Word's East Asian line height (1.3 × the font's Windows metrics) and document-grid cell counting, missing-font substitution by fontTable charset and family (Batang/MS Mincho/SimSun/PMingLiU, Malgun Gothic/MS Gothic/Microsoft YaHei), per-character rescue of glyphs the font lacks, script-based run splitting via `w:rFonts @eastAsia` and East Asian theme fonts, punctuation compression (`compressPunctuation`), distributed alignment
- **Images**: inline JPEG/PNG/BMP embedding with sizing and alpha transparency, grayscale and CMYK JPEG support, EMF/WMF vector translation to PDF form XObjects (bitmap-only EMFs as images), cropping (`a:srcRect`, including negative crops), brightness/contrast (`a:lum`), anchored/floating images with wrap modes (square, tight, through, topAndBottom), floating image positioning relative to page/margin/column, rotation (inline and floating), clipping to shape geometry, z-ordering with shapes via `relativeHeight` and behind-document placement
- **Picture effects**: outer shadow (`a:outerShdw`), inner shadow, glow, soft edges, reflection — rasterized blur masks via SMask
- **Text boxes**: DrawingML textboxes (`wps:txbx`) and VML fallback (`v:textbox`), shape fills (solid color with theme color support including lumMod/lumOff, linear gradients with multiple color stops), textbox body margins
- **WordArt**: modern DrawingML WordArt with all 40 `prstTxWarp` presets — two-path envelope warping (wave, slant, inflate, etc.) and single-path text-on-a-path (arch, circle), text outlines, shadows, glow effects, bold/italic font variant selection, VML WordArt fallback
- **Shapes & geometry**: all 187 OOXML preset shapes via formula-based geometry engine (guide formulas, adjustment values), custom geometry paths (`a:custGeom` with moveTo, lineTo, cubicBezTo, arcTo), shape fills and strokes, drawing canvases (`wpc:wpc`) and shape groups (`wpg:wgp`/`grpSp`) flattened with nested transforms, connectors
- **Charts**: bar (clustered/stacked/percent-stacked, vertical/horizontal), line, pie (incl. `pie3DChart` drawn flat), area, doughnut, radar, scatter, bubble — with axis labels, tick marks, gridlines, legends, series markers, bubble fill opacity
- **Math**: Office Math (OMML) equations, inline and display (`m:oMathPara`) with justification
- **Page layout**: page size, margins, gutter margins, document grid (`linePitch`), page borders (`w:pgBorders`), vertical page alignment (`w:vAlign`), line numbering (`w:lnNumType`), explicit page breaks, `pageBreakBefore`, automatic page breaking with widow/orphan control
- **Sections**: multiple sections with `nextPage`/`continuous`/`oddPage`/`evenPage` breaks, per-section page size and margins, blank page insertion for odd/even page alignment
- **Multi-column layout**: 2+ columns with custom widths and spacing, column breaks, column separators
- **Headers/footers**: default, first-page, and even/odd variants, per-section headers/footers, STYLEREF field resolution (spec-compliant backward search), page number and page count fields, images in headers/footers, correct z-ordering (behind body content)
- **Footnotes & endnotes**: footnote references and page-bottom rendering with separator line, each footnote placed on the page of the line that references it, endnotes flowed at document end, per-section mark numbering formats, reference marks in table cells, shading on reference marks
- **Comments**: `word/comments.xml` rendered in Word's right-hand review pane with callouts and body scaling
- **Fields**: PAGE, NUMPAGES, PAGEREF (with `\h` links), STYLEREF (with spec-compliant search order), TOC fields (cached entries, tagged as TOC/TOCI), cached results for all other fields
- **Hyperlinks**: clickable links in PDF output (external URI and internal bookmark links)
- **Tab stops**: left, center, right, decimal with leader dots
- **Track changes**: final mode (insertions included, deletions removed — matches Word's PDF export)
- **SmartArt**: rendering via pre-flattened drawing shapes (`dsp:drawing`) with full geometry engine support — all 187 preset shapes, custom geometry, fills (solid, gradient, image), strokes, and text
- **Document settings**: `word/settings.xml` parsing — even/odd headers, default tab stop interval, mirror margins
- **Compatibility**: `mc:AlternateContent` fallback, structured document tag (`w:sdt`) content extraction, `w:customXml` transparent wrappers, `altChunk` HTML content parsing, smart tag handling, VML fallbacks for shapes, textboxes, WordArt and `w:object` embeds
- **Fonts**: cross-platform font search (macOS/Linux/Windows), embedded DOCX font extraction and deobfuscation, font subsetting (CIDFont/Type0), disk-cached font index, font substitution via `fontTable.xml` altName, the theme body font and family-class fallback, a last-resort chain (Arial/Liberation Sans/Arimo/Helvetica/DejaVu Sans) before Type1 Helvetica, symbol fonts (Wingdings, Symbol) mapped to real Unicode for text extraction
- **Output**: font subsetting, content and font stream compression, object streams, byte-for-byte deterministic output

### Not yet supported

- **Text**: text shaping/ligatures (fi, fl), complex script shaping (Arabic, Devanagari, etc.), automatic hyphenation (parked — Word's online converter doesn't hyphenate either), squeezing justified spaces to fit one more word (Word does; we only stretch), Word's exact glyph advances (small width differences cause line-break drift on long documents)
- **Images**: wrapping text from paragraphs before the one directly above a float anchor, tight vs through wrapping distinction
- **Layout**: mirror margins (parsed but not applied to even pages), right-to-left (bidi) text, `w:textAlignment` (vertical alignment of mixed-size runs), kashida justification (`mediumKashida`/`highKashida`/`lowKashida` render as plain justify — glyph elongation needs Arabic shaping)
- **Charts**: 3D charts other than pie, stock charts, combo charts, data labels, chart titles, secondary axes
- **Shape effects**: 3D bevel/rotation (`a:scene3d`, `a:sp3d`), preset shadows (`a:prstShdw`), radial/path gradient fills (axial only), theme colors in picture effects (fall back to black), group flips and rotation
- **SmartArt**: groups and connectors inside the `dsp:drawing` fallback; no layout engine for documents missing that fallback (see roadmap)
- **Track changes**: markup mode (only the final view is rendered), paragraph-level `w:ins`/`w:del` around whole paragraphs, formatting revisions
- **Features**: table of contents generation (Word's cached TOC entries are used as-is), OLE objects beyond their preview picture
- **Fonts**: bundled metric-compatible fallback fonts (output depends on the fonts installed on the host)

## Examples

Every test case rendered as a Word reference and with docxide-pdf on the [comparison page](https://sverrejb.github.io/docxide-pdf/).

## Installation

```bash
# Install the CLI
cargo install docxide-pdf
```

## Usage

### CLI

```bash
# Convert a DOCX file to PDF
docxide-pdf input.docx

# Specify output path (defaults to input.pdf)
docxide-pdf input.docx output.pdf
```

### Library

```bash
cargo add docxide-pdf --no-default-features
```

This avoids pulling in the CLI dependency (`clap`).

```rust
use docxide_pdf::convert_docx_to_pdf;
use std::path::Path;

convert_docx_to_pdf(
    Path::new("input.docx"),
    Path::new("output.pdf"),
)?;
```

## Works well with `docxide-template`

[`docxide-template`](https://github.com/sverrejb/docxide-template) is a sibling crate for type-safe MS Word templates. It scans a folder of `.docx` files at compile time and generates a Rust struct per template, with `{Placeholder}` patterns turned into snake_case fields. Pair it with `docxide-pdf` to go from template → filled DOCX → PDF in a single, fully in-memory pipeline:

```rust
use docxide_pdf::convert_docx_bytes_to_pdf;
use docxide_template::generate_templates;
use std::path::Path;

generate_templates!("templates");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = HelloWorld {
        first_name: "Alice".into(),
        company: "Acme Corp".into(),
    };

    let docx_bytes = doc.to_bytes()?;
    convert_docx_bytes_to_pdf(&docx_bytes, Path::new("output/greeting.pdf"))?;
    Ok(())
}
```

100% Rust, end to end — no temporary files, and no Word or LibreOffice install required on the host. Fill the template in memory, hand the bytes to `convert_docx_bytes_to_pdf`, and write the PDF. Combined with `docxide-template`'s `embed` feature, you get a single self-contained binary that turns structured data into a polished PDF.

## Configuration

### Environment Variables

| Variable | Description |
|---|---|
| `DOCXSIDE_FONTS` | Additional font directories to search, colon-separated (`;` on Windows). Searched before system font directories. |
| `DOCXSIDE_NO_FONT_CACHE` | Set to any value to disable the font index disk cache. Forces a full font scan on every conversion. Useful for debugging font resolution issues. |

Font scanning results are cached to disk (per-directory, invalidated by mtime). The cache is stored at:
- **macOS**: `~/Library/Caches/docxide-pdf/font-index.tsv`
- **Linux**: `$XDG_CACHE_HOME/docxide-pdf/font-index.tsv` (default `~/.cache/`)
- **Windows**: `%LOCALAPPDATA%\docxide-pdf\cache\font-index.tsv`

## Testing

Tests require `mutool` on `PATH` for PDF-to-PNG rendering; the accessibility suite also needs veraPDF and Poppler (it skips with a notice when they are missing):

```bash
brew install mupdf verapdf poppler            # macOS
apt install mupdf-tools poppler-utils         # Debian/Ubuntu (veraPDF: https://verapdf.org)
```

The Word fonts the references were made with are not in this repository. Put them in `fonts/` (gitignored, passed to tests as `DOCXSIDE_FONTS` via `.cargo/config.toml`); without them scores drop wherever a font is substituted.

```bash
./tools/run-tests.sh                           # everything, compact report of what changed
./tools/run-tests.sh --test visual_comparison  # one suite (Jaccard + SSIM)
./tools/run-tests.sh --test accessibility      # PDF/UA-1 + structure/text parity vs Word
./tools/run-tests.sh --case case5              # one fixture
./tools/run-tests.sh --verbose                 # full cargo output
```

Scores are compared with the accepted ones in `tests/baselines.json`; `tools/target/debug/accept-baselines` accepts new ones. [SCORING.md](SCORING.md) explains every score: the visual metrics, the accessibility metrics and what fails the suite.

## Debugging Tools

Build the tools once:

```bash
cd tools && cargo build
```

Then run from the project root:

```bash
# Inspect XML inside a DOCX
./tools/target/debug/docx-inspect input.docx

# Print font information
./tools/target/debug/docx-fonts input.docx

# Compare two rendered pages
./tools/target/debug/jaccard a.png b.png

# Full fixture diff
./tools/target/debug/case-diff case1

# Feature and score overview of the fixture corpus
./tools/target/debug/analyze-fixtures --failing

# Interactive case browser (reference vs generated, overlay, annotations)
cargo compare

# Side-by-side with LibreOffice, MiniPdf, rdocx and office2pdf → comparison/index.html
python3 tools/engine_compare.py --open
```

## Contributing

Pull requests are welcome!

## License

Apache-2.0
