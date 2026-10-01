#!/usr/bin/env python3
"""Overlay one page of two PDFs as an ink diff image: blue = reference-only ink,
red = generated-only ink, grey = both (luma < 200, as the Jaccard metric counts
it, at 75 DPI):

    python3 tools/ink_diff.py <ref.pdf> <gen.pdf> <page> <out.png>

Needs mutool and Pillow.
"""
import subprocess, sys, tempfile, os
from PIL import Image

ref, gen, page, out = sys.argv[1:5]
d = tempfile.mkdtemp()
for name, pdf in (('r', ref), ('g', gen)):
    subprocess.run(['mutool', 'draw', '-F', 'png', '-r', '75', '-o', f'{d}/{name}.png', pdf, page], capture_output=True)
a = Image.open(f'{d}/r.png').convert('L')
b = Image.open(f'{d}/g.png').convert('L').resize(a.size)
o = Image.new('RGB', a.size, 'white')
pa, pb, po = a.load(), b.load(), o.load()
for y in range(a.size[1]):
    for x in range(a.size[0]):
        ia, ib = pa[x, y] < 200, pb[x, y] < 200
        if ia and ib:
            po[x, y] = (150, 150, 150)
        elif ia:
            po[x, y] = (0, 80, 255)
        elif ib:
            po[x, y] = (255, 40, 40)
o.save(out)
