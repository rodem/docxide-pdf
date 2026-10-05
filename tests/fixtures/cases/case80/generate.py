"""Generate case80: Column Layout Variations

Tests:
- Continuous section breaks between 1, 2, 3, 4 and 5 column regions on one page
  (Word balances the columns of a region that ends at a continuous break)
- Unequal column widths (w:equalWidth="0" with explicit w:col widths)
- Separator lines beside empty columns: column breaks that skip a column and
  leave the last one empty
- A table and a bulleted list inside narrow columns
- A page break inside a two-column region (column 2 of that page stays empty)
- A landscape section that continues the two-column layout over two pages
- A next-page section and the document end that stop in the first column
  (no balancing)
- w:num="1" with w:sep="1" (nothing to separate)
- Zero column spacing with separators
"""

import io
import pathlib
import zipfile

from docx import Document
from docx.enum.section import WD_ORIENT, WD_SECTION
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_BREAK
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Inches, Pt
from lxml import etree

WML = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"

SENTENCES = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
    "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.",
    "Duis aute irure dolor in reprehenderit in voluptate velit esse.",
    "Excepteur sint occaecat cupidatat non proident, sunt in culpa.",
    "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit.",
    "Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet.",
    "Quis autem vel eum iure reprehenderit qui in ea voluptate velit.",
    "At vero eos et accusamus et iusto odio dignissimos ducimus.",
    "Nam libero tempore, cum soluta nobis est eligendi optio cumque.",
    "Temporibus autem quibusdam et aut officiis debitis aut rerum.",
    "Itaque earum rerum hic tenetur a sapiente delectus, ut aut reiciendis.",
]


def text(n, start=0):
    return " ".join(SENTENCES[(start + i) % len(SENTENCES)] for i in range(n))


def set_cols(section, num=1, space=720, sep=False, widths=None):
    """widths: list of (width_twips, space_after_twips) for unequal columns."""
    sect_pr = section._sectPr
    for old in sect_pr.findall(qn("w:cols")):
        sect_pr.remove(old)
    cols = OxmlElement("w:cols")
    if widths:
        cols.set(qn("w:num"), str(len(widths)))
        cols.set(qn("w:equalWidth"), "0")
    else:
        cols.set(qn("w:num"), str(num))
        cols.set(qn("w:space"), str(space))
    if sep:
        cols.set(qn("w:sep"), "1")
    for w, s in widths or []:
        col = OxmlElement("w:col")
        col.set(qn("w:w"), str(w))
        if s:
            col.set(qn("w:space"), str(s))
        cols.append(col)
    # Schema order: w:cols comes before w:docGrid
    grid = sect_pr.find(qn("w:docGrid"))
    if grid is not None:
        grid.addprevious(cols)
    else:
        sect_pr.append(cols)


def column_break(doc):
    doc.add_paragraph().add_run().add_break(WD_BREAK.COLUMN)


doc = Document()
s = doc.sections[0]
s.page_width, s.page_height = Inches(8.5), Inches(11)
s.top_margin = s.bottom_margin = s.left_margin = s.right_margin = Inches(1)

# --- Region 1: one column title ---
doc.add_heading("Column Layout Variations", level=1)
doc.add_paragraph(text(3))
set_cols(doc.sections[-1], num=1)

# --- Region 2: two balanced columns with separator ---
doc.add_section(WD_SECTION.CONTINUOUS)
for i in range(4):
    doc.add_paragraph(text(2, i * 2))
set_cols(doc.sections[-1], num=2, space=720, sep=True)

# --- Region 3: three unequal columns (narrow, wide, narrow) with a table ---
doc.add_section(WD_SECTION.CONTINUOUS)
doc.add_paragraph(text(2, 3))
column_break(doc)
doc.add_paragraph(text(3, 5))
table = doc.add_table(rows=3, cols=2)
table.style = "Table Grid"
for r, (k, v) in enumerate([("Item", "Qty"), ("Apples", "12"), ("Pears", "7")]):
    table.cell(r, 0).text = k
    table.cell(r, 1).text = v
doc.add_paragraph(text(1, 8))
column_break(doc)
for i in range(3):
    doc.add_paragraph(SENTENCES[i], style="List Bullet")
# 1.25" + 0.25" + 3.5" + 0.25" + 1.25" = 6.5"
set_cols(doc.sections[-1], widths=[(1800, 360), (5040, 360), (1800, 0)], sep=True)

# --- Region 4: one column interlude ---
doc.add_section(WD_SECTION.CONTINUOUS)
p = doc.add_paragraph("Interlude between column regions")
p.alignment = WD_ALIGN_PARAGRAPH.CENTER
p.runs[0].bold = True
set_cols(doc.sections[-1], num=1)

# --- Region 5: four columns, column 2 and 4 left empty by column breaks ---
doc.add_section(WD_SECTION.CONTINUOUS)
doc.add_paragraph(text(2, 1))
column_break(doc)
column_break(doc)
doc.add_paragraph(text(2, 6))
set_cols(doc.sections[-1], num=4, space=288, sep=True)

# --- Region 6: five narrow columns, no separator ---
doc.add_section(WD_SECTION.CONTINUOUS)
for i in range(5):
    doc.add_paragraph(text(1, i))
set_cols(doc.sections[-1], num=5, space=144, sep=False)

# --- Region 7 (new page): two columns, a page break in column 1, then a long
# run that stops in column 1 at the next-page section break ---
doc.add_section(WD_SECTION.NEW_PAGE)
doc.add_heading("Page break inside columns", level=2)
doc.add_paragraph(text(4, 2))
doc.add_paragraph().add_run().add_break(WD_BREAK.PAGE)
for i in range(14):
    doc.add_paragraph(text(3, i))
set_cols(doc.sections[-1], num=2, space=720, sep=True)

# --- Region 8 (new page, landscape): the two-column layout continues on a
# wider page and overflows onto a second landscape page ---
doc.add_section(WD_SECTION.NEW_PAGE)
ls = doc.sections[-1]
ls.orientation = WD_ORIENT.LANDSCAPE
ls.page_width, ls.page_height = Inches(11), Inches(8.5)
for i in range(16):
    doc.add_paragraph(text(3, i + 5))
set_cols(ls, num=2, space=720, sep=True)

# --- Region 9 (new page, portrait again): one column with sep="1" ---
doc.add_section(WD_SECTION.NEW_PAGE)
pt = doc.sections[-1]
pt.orientation = WD_ORIENT.PORTRAIT
pt.page_width, pt.page_height = Inches(8.5), Inches(11)
doc.add_paragraph("A single column that asks for a separator: " + text(3, 4))
set_cols(doc.sections[-1], num=1, sep=True)

# --- Region 10: three columns, zero spacing, document ends in column 1 ---
doc.add_section(WD_SECTION.CONTINUOUS)
for i in range(3):
    doc.add_paragraph(text(2, i + 3))
set_cols(doc.sections[-1], num=3, space=0, sep=True)

tmp_buf = io.BytesIO()
doc.save(tmp_buf)
tmp_buf.seek(0)

out_buf = io.BytesIO()
with zipfile.ZipFile(tmp_buf, "r") as zin, zipfile.ZipFile(out_buf, "w", zipfile.ZIP_DEFLATED) as zout:
    for item in zin.infolist():
        data = zin.read(item.filename)
        if item.filename == "word/settings.xml":
            tree = etree.fromstring(data)
            for compat_setting in tree.iter("{%s}compatSetting" % WML):
                if compat_setting.get(qn("w:name")) == "compatibilityMode":
                    compat_setting.set(qn("w:val"), "15")
            data = etree.tostring(tree, xml_declaration=True, encoding="UTF-8", standalone=True)
        zout.writestr(item, data)

out_path = pathlib.Path(__file__).parent / "input.docx"
out_path.write_bytes(out_buf.getvalue())
print(f"Wrote {out_path} ({out_path.stat().st_size} bytes)")
