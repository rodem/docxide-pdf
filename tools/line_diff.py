#!/usr/bin/env python3
"""The first n places where the two PDFs' text lines differ (whole document,
reading order by page and y), with x extents. Shows where a line breaks
differently, i.e. where reflow starts:

    python3 tools/line_diff.py <ref.pdf> <gen.pdf> [n]
"""
import difflib, re, subprocess, sys, html


def lines(pdf):
    st = subprocess.run(['mutool', 'draw', '-F', 'stext', '-o', '-', pdf], capture_output=True, text=True).stdout
    out = []
    for pno, page in enumerate(re.findall(r'<page .*?</page>', st, re.S), 1):
        rows = []
        for bbox, body in re.findall(r'<line bbox="([^"]*)"[^>]*>(.*?)</line>', page, re.S):
            b = [float(v) for v in bbox.split()]
            t = html.unescape(''.join(re.findall(r'c="([^"]*)"', body))).strip()
            if t:
                rows.append((round(b[1]), b[0], b[2], t))
        # merge fragments on the same baseline row
        rows.sort()
        merged = []
        for y, x0, x1, t in rows:
            if merged and abs(merged[-1][0] - y) <= 2:
                py, px0, px1, pt = merged[-1]
                merged[-1] = (py, min(px0, x0), max(px1, x1), pt + ' ' + t)
            else:
                merged.append((y, x0, x1, t))
        out += [(pno, y, x0, x1, re.sub(r'\s+', ' ', t)) for y, x0, x1, t in merged]
    return out


a, b = lines(sys.argv[1]), lines(sys.argv[2])
n = int(sys.argv[3]) if len(sys.argv) > 3 else 3
sm = difflib.SequenceMatcher(a=[l[4] for l in a], b=[l[4] for l in b], autojunk=False)
shown = 0
for tag, i1, i2, j1, j2 in sm.get_opcodes():
    if tag == 'equal':
        continue
    print(f"--- {tag}")
    for l in a[i1:i2][:4]:
        print(f"  ref p{l[0]} y{l[1]:4d} x{l[2]:6.1f}-{l[3]:6.1f} | {l[4][:110]}")
    for l in b[j1:j2][:4]:
        print(f"  gen p{l[0]} y{l[1]:4d} x{l[2]:6.1f}-{l[3]:6.1f} | {l[4][:110]}")
    shown += 1
    if shown >= n:
        break
