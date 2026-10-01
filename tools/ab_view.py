#!/usr/bin/env python3
"""Render fixtures with a given docxide CLI binary into a vdiff-ready tree and
print vdiff's per-page summary, so two builds can be compared side by side
without touching tests/output:

    python3 tools/ab_view.py <label> <docxide-cli> <group/case>... [-v]

Copy target/release/docxide-pdf aside (e.g. /tmp/docxide-before) before a
change to keep the old build. Output goes to tests/output/ab/<label>/; -v
prints vdiff's full line table. vdiff is looked up in $VDIFF, else
tools/target/debug/vdiff.
"""
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VDIFF = os.environ.get('VDIFF', str(ROOT / 'tools/target/debug/vdiff'))
SUMMARY = ('===', 'pages:', '  page', '  first')


def main() -> None:
    args = [a for a in sys.argv[1:] if a != '-v']
    verbose = '-v' in sys.argv
    label, cli, cases = args[0], args[1], args[2:]
    root = ROOT / 'tests/output/ab' / label
    (root / 'tests').mkdir(parents=True, exist_ok=True)
    (root / 'Cargo.toml').touch()  # vdiff looks for the project root
    if not (root / 'tests/fixtures').exists():
        (root / 'tests/fixtures').symlink_to(ROOT / 'tests/fixtures')
    env = dict(os.environ, DOCXSIDE_FONTS=str(ROOT / 'fonts'), DOCXSIDE_NO_FONT_CACHE='1')
    for gc in cases:
        out = root / 'tests/output' / gc
        out.mkdir(parents=True, exist_ok=True)
        pdf = out / 'generated.pdf'
        pdf.unlink(missing_ok=True)  # the CLI never overwrites an existing file
        subprocess.run([cli, str(ROOT / 'tests/fixtures' / gc / 'input.docx'), str(pdf)],
                       env=env, capture_output=True)
        report = subprocess.run([VDIFF, gc.split('/', 1)[1]] + (['--verbose'] if verbose else []),
                                cwd=root, capture_output=True, text=True).stdout
        lines = report.splitlines() if verbose else [l for l in report.splitlines() if l.startswith(SUMMARY)]
        print('\n'.join(lines))


if __name__ == '__main__':
    main()
