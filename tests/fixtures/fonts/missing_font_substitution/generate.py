#!/usr/bin/env python3
"""Which face does Word draw for a font it doesn't have?

One row per condition: a Courier New label, then a sample run in the test
font. Fake "Zqx …" names are installed nowhere. The fontTable entries (altName,
family, panose, pitch, charset) are injected after python-docx saves, and the
theme is Verdana (body) / Trebuchet MS (headings) so a theme fallback can't be
mistaken for a built-in default such as Cambria or Calibri.

Export twice from Mac Word: "Best for printing" → reference.pdf (local
renderer), "Best for electronic distribution and accessibility (uses Microsoft
online service)" → reference_online.pdf (Windows Word on Microsoft's servers;
the PDF says Creator(Microsoft Word), not Quartz PDFContext).

Usage:
    uv run tests/fixtures/fonts/missing_font_substitution/generate.py
"""

import re
import zipfile
from pathlib import Path

from docx import Document
from docx.shared import Inches, Pt

OUT_DIR = Path(__file__).parent
SAMPLE = "Hamburgefonstiv quick brown fox 0123"
ARIAL_PANOSE = "020B0604020202020204"
TIMES_PANOSE = "02020603050405020304"
GEORGIA_PANOSE = "02040502050405020303"


def entry(name, family, alt=None, panose=None, pitch="variable", charset="00"):
    return dict(name=name, family=family, alt=alt, panose=panose, pitch=pitch, charset=charset)


# (label, run font, fontTable entry or None)
ROWS = [
    # A: no fontTable entry at all
    ("A1 no entry", "Zqx Alpha", None),
    ("A2 no entry, ;Arial", "Zqx Bravo;Arial", None),
    ("A3 no entry, ;sans-serif", "Zqx Charlie;sans-serif", None),
    ("A4 no entry, Georgia;Arial", "Georgia;Arial", None),
    ("A5 no entry, Helvetica", "Helvetica", None),
    # B: entry with only a family
    ("B1 family auto", "Zqx Bauto", entry("Zqx Bauto", "auto")),
    ("B2 family roman", "Zqx Broman", entry("Zqx Broman", "roman")),
    ("B3 family swiss", "Zqx Bswiss", entry("Zqx Bswiss", "swiss")),
    ("B4 family modern, fixed", "Zqx Bmodern", entry("Zqx Bmodern", "modern", pitch="fixed")),
    ("B5 family script", "Zqx Bscript", entry("Zqx Bscript", "script")),
    ("B6 family decorative", "Zqx Bdecor", entry("Zqx Bdecor", "decorative")),
    # C: family + panose
    ("C1 swiss, Arial panose", "Zqx Csans", entry("Zqx Csans", "swiss", panose=ARIAL_PANOSE)),
    ("C2 roman, Times panose", "Zqx Cserif", entry("Zqx Cserif", "roman", panose=TIMES_PANOSE)),
    ("C3 auto, Arial panose", "Zqx Cauto", entry("Zqx Cauto", "auto", panose=ARIAL_PANOSE)),
    ("C4 swiss, Georgia panose", "Zqx Cmix", entry("Zqx Cmix", "swiss", panose=GEORGIA_PANOSE)),
    # D: altName
    ("D1 swiss, alt Georgia", "Zqx Dgeorgia", entry("Zqx Dgeorgia", "swiss", alt="Georgia")),
    ("D2 auto, alt sans-serif", "Zqx Dbogus", entry("Zqx Dbogus", "auto", alt="sans-serif")),
    ("D3 run X;Arial, entry X alt Georgia", "Zqx Dlist;Arial",
     entry("Zqx Dlist", "roman", alt="Georgia")),
    ("D4 run X;sans-serif, entry X", "Zqx Elist;sans-serif",
     entry("Zqx Elist", "auto", alt="sans-serif", charset="01")),
    # E: installed fonts and the real LibreOffice cases
    ("E1 Calibri, alt Georgia", "Calibri", entry("Calibri", "swiss", alt="Georgia")),
    ("E2 Helvetica, alt Georgia", "Helvetica", entry("Helvetica", "swiss", alt="Georgia")),
    ("E3 Open Sans;Arial (sample500kB)", "Open Sans;Arial",
     entry("Open Sans", "roman", alt="Arial", charset="01")),
    ("E4 Archivo;sans-serif (german_mezzo)", "Archivo;sans-serif",
     entry("Archivo", "auto", alt="sans-serif", charset="01")),
    ("E5 Carlito, alt Calibri", "Carlito", entry("Carlito", "roman", alt="Calibri", charset="01")),
    # F: no entry, well-known names: does Word map names by itself?
    # Metric clones LibreOffice writes (not in Word's cloud catalog)
    ("F1 Liberation Sans", "Liberation Sans", None),
    ("F2 Liberation Serif", "Liberation Serif", None),
    ("F3 Liberation Mono", "Liberation Mono", None),
    ("F4 Carlito", "Carlito", None),
    ("F5 Caladea", "Caladea", None),
    ("F6 Arimo", "Arimo", None),
    ("F7 Tinos", "Tinos", None),
    # In Word's cloud catalog (Word may download them on open)
    ("F8 Open Sans", "Open Sans", None),
    ("F9 Roboto", "Roboto", None),
    ("F10 Lato", "Lato", None),
    ("F11 Montserrat", "Montserrat", None),
    # Nowhere
    ("F12 Inter", "Inter", None),
    # macOS-only system faces
    ("F13 Helvetica Neue", "Helvetica Neue", None),
    ("F14 Times", "Times", None),
    ("F15 Courier", "Courier", None),
    # ";" lists starting with a clone, a catalog font, a downloaded cloud font
    ("F16 Liberation Sans;Arial", "Liberation Sans;Arial", None),
    ("F17 Roboto;Arial", "Roboto;Arial", None),
    ("F18 Source Sans Pro;Arial", "Source Sans Pro;Arial", None),
]


def font_xml(e):
    parts = [f'<w:font w:name="{e["name"]}">']
    if e["alt"]:
        parts.append(f'<w:altName w:val="{e["alt"]}"/>')
    if e["panose"]:
        parts.append(f'<w:panose1 w:val="{e["panose"]}"/>')
    parts.append(f'<w:charset w:val="{e["charset"]}"/>')
    parts.append(f'<w:family w:val="{e["family"]}"/>')
    parts.append(f'<w:pitch w:val="{e["pitch"]}"/>')
    parts.append("</w:font>")
    return "".join(parts)


def patch(path):
    entries = "".join(font_xml(e) for _, _, e in ROWS if e)
    with zipfile.ZipFile(path) as z:
        parts = {n: z.read(n) for n in z.namelist()}
    fonts = parts["word/fontTable.xml"].decode()
    parts["word/fontTable.xml"] = fonts.replace("</w:fonts>", entries + "</w:fonts>").encode()
    theme = parts["word/theme/theme1.xml"].decode()
    theme = re.sub(r'(<a:majorFont>\s*<a:latin typeface=")[^"]*', r"\1Trebuchet MS", theme)
    theme = re.sub(r'(<a:minorFont>\s*<a:latin typeface=")[^"]*', r"\1Verdana", theme)
    parts["word/theme/theme1.xml"] = theme.encode()
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for name, data in parts.items():
            z.writestr(name, data)


def main():
    doc = Document()
    for section in doc.sections:
        section.page_width, section.page_height = Inches(8.5), Inches(11)
        section.top_margin = section.bottom_margin = Inches(0.8)
        section.left_margin = section.right_margin = Inches(0.8)

    for label, font, _ in ROWS:
        p = doc.add_paragraph()
        p.paragraph_format.space_after = Pt(6)
        tag = p.add_run(label + ": ")
        tag.font.name = "Courier New"
        tag.font.size = Pt(8)
        sample = p.add_run(SAMPLE)
        sample.font.name = font
        sample.font.size = Pt(12)

    out = OUT_DIR / "input.docx"
    doc.save(out)
    patch(out)
    print(f"Wrote {out}")


if __name__ == "__main__":
    main()
