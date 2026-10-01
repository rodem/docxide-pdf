#!/usr/bin/env python3
"""List a PDF page's horizontal rules (table and paragraph borders) and text
baselines, y measured from the page top in points, sorted by y:

    python3 tools/pdf_lines.py <pdf> <page> [ymin ymax]

Run it on reference.pdf and generated.pdf to measure where Word puts border
bands and baselines (row pitch, band thickness, text inset). Needs mutool.
"""
import re
import subprocess
import sys


def transform(attrs):
    m = re.search(r'transform="([^"]*)"', attrs)
    return [float(v) for v in m.group(1).split()] if m else [1, 0, 0, 1, 0, 0]


def rules(pdf, page):
    trace = subprocess.run(['mutool', 'draw', '-F', 'trace', '-o', '-', pdf, page],
                           capture_output=True, text=True).stdout
    for m in re.finditer(r'<(fill_path|stroke_path)([^>]*)>(.*?)</\1>', trace, re.S):
        kind, attrs, body = m.groups()
        a, b, c, d, e, f = transform(attrs)
        pts = [(a * x + c * y + e, b * x + d * y + f) for x, y in
               ((float(x), float(y)) for x, y in
                re.findall(r'<(?:moveto|lineto) x="([-\d.e]+)" y="([-\d.e]+)"', body))]
        if not pts:
            continue
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        w, h = max(xs) - min(xs), max(ys) - min(ys)
        if w > 15 and h < 5:  # a horizontal rule, not a box or a glyph
            lw = re.search(r'linewidth="([\d.e-]+)"', attrs)
            thick = h if kind == 'fill_path' else (float(lw.group(1)) * abs(d) if lw else 0)
            mid = (max(ys) + min(ys)) / 2
            yield mid, (f"RULE {kind[:4]} x {min(xs):6.1f}-{max(xs):6.1f} "
                        f"y {min(ys):7.2f}-{max(ys):7.2f} (centre {mid:7.2f}) thick {thick:.2f}")


def baselines(pdf, page):
    st = subprocess.run(['mutool', 'draw', '-F', 'stext', '-o', '-', pdf, page],
                        capture_output=True, text=True).stdout
    pattern = (r'<line bbox="([^"]*)"[^>]*>\s*<font [^>]*size="([\d.]+)"[^>]*>\s*'
               r'<char quad="[^"]*" x="([\d.]+)" y="([\d.]+)"[^>]*c="([^"]*)"')
    for m in re.finditer(pattern, st):
        bb = [float(v) for v in m.group(1).split()]
        y = float(m.group(4))
        yield y, (f"TEXT base {y:7.2f} top {bb[1]:7.2f} bot {bb[3]:7.2f} "
                  f"x {float(m.group(3)):6.1f} sz {m.group(2)} '{m.group(5)}'")


def main() -> None:
    pdf, page = sys.argv[1], sys.argv[2]
    ymin, ymax = (float(sys.argv[3]), float(sys.argv[4])) if len(sys.argv) > 4 else (0, 1e9)
    for y, s in sorted(list(rules(pdf, page)) + list(baselines(pdf, page))):
        if ymin <= y <= ymax:
            print(f"{y:8.2f}  {s}")


if __name__ == '__main__':
    main()
