#!/usr/bin/env python3
"""Per text line (paired by order when both PDFs have the same words), the
start x of each word in the generated PDF minus the reference; prints the
lines whose words drift most. Finds width and justification errors that a
baseline (y) comparison misses:

    python3 tools/word_x_diff.py <ref.pdf> <gen.pdf> <page> [n]
"""
import re, subprocess, sys


def lines(pdf, page):
    st = subprocess.run(['mutool', 'draw', '-F', 'stext', '-o', '-', pdf, page], capture_output=True, text=True).stdout
    out = []
    for body in re.findall(r'<line [^>]*>(.*?)</line>', st, re.S):
        chars = [(float(x), float(y), c) for x, y, c in re.findall(r'<char [^>]*x="([\d.]+)" y="([\d.]+)"[^>]*c="([^"]*)"', body)]
        if not chars:
            continue
        words, cur, start = [], '', None
        for x, y, c in chars:
            if c == ' ':
                if cur:
                    words.append((start, cur))
                cur, start = '', None
            else:
                if start is None:
                    start = x
                cur += c
        if cur:
            words.append((start, cur))
        if words:
            out.append((chars[0][1], words))
    return sorted(out)


ref, gen = lines(sys.argv[1], sys.argv[3]), lines(sys.argv[2], sys.argv[3])
n = int(sys.argv[4]) if len(sys.argv) > 4 else 8
rows = []
for (ry, rw), (gy, gw) in zip(ref, gen):
    if [w for _, w in rw] != [w for _, w in gw]:
        continue
    d = [g[0] - r[0] for r, g in zip(rw, gw)]
    rows.append((max(abs(v) for v in d), ry, rw, d))
rows.sort(reverse=True)
for m, y, rw, d in rows[:n]:
    print(f"y {y:6.1f} max|dx| {m:5.2f}  " + ' '.join(f"{w[1][:8]}{dx:+.1f}" for w, dx in zip(rw, d))[:260])
print('lines compared', len(rows), 'mean max|dx|', round(sum(r[0] for r in rows) / max(1, len(rows)), 2))
