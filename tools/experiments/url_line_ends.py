"""Where does Word break URLs? For every line in the given PDFs that ends inside a
URL-like token, print the line end and the next line's start: the evidence behind
`drop_url_breaks` in src/pdf/layout.rs (Word wraps a URL after a hyphen or at
the margin, almost never after a '/').

    python3 tools/experiments/url_line_ends.py [pdf...]   (default: all fixture references)
"""
import glob
import html
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def lines_of(pdf):
    st = subprocess.run(['mutool', 'draw', '-F', 'stext', '-o', '-', pdf], capture_output=True, text=True).stdout
    for bbox, body in re.findall(r'<line bbox="([^"]*)"[^>]*>(.*?)</line>', st, re.S):
        yield [float(v) for v in bbox.split()], html.unescape(''.join(re.findall(r'c="([^"]*)"', body)))


def main():
    pdfs = sys.argv[1:] or sorted(glob.glob(str(ROOT / 'tests/fixtures/*/*/reference.pdf')))
    ends = slash = 0
    for pdf in pdfs:
        lines = list(lines_of(pdf))
        for (b, t), (_, nt) in zip(lines, lines[1:]):
            last = t.rstrip().split(' ')[-1] if t.strip() else ''
            if ('://' in last or last.startswith('www.')) and not t.endswith(' ') and nt and not nt[0].isspace():
                ends += 1
                slash += last.endswith('/')
                print(f"{Path(pdf).parent.name[:30]:30s} width {b[2] - b[0]:6.1f} | ...{t.rstrip()[-45:]!r} || {nt[:35]!r}")
    print(f'{ends} URL line ends, {slash} right after a /')


if __name__ == '__main__':
    main()
