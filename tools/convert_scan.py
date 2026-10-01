#!/usr/bin/env python3
"""Convert every fixture with a docxide CLI binary under a per-file timeout
and report hangs and crashes (a suite run that never finishes is usually one
fixture looping in layout):

    python3 tools/convert_scan.py [<docxide-cli>] [--timeout SECONDS]
"""
import argparse
import os
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument('cli', nargs='?', default=str(ROOT / 'target/release/docxide-pdf'))
    p.add_argument('--timeout', type=float, default=20)
    args = p.parse_args()
    env = dict(os.environ, DOCXSIDE_FONTS=str(ROOT / 'fonts'), DOCXSIDE_NO_FONT_CACHE='1')
    out = Path(tempfile.mkdtemp())
    docs = sorted((ROOT / 'tests/fixtures').glob('*/*/input.docx'))

    def one(docx):
        name = f"{docx.parent.parent.name}/{docx.parent.name}"
        pdf = out / (name.replace('/', '_') + '.pdf')
        try:
            r = subprocess.run([args.cli, str(docx), str(pdf)], env=env, capture_output=True,
                               timeout=args.timeout)
            return name, f'crash (exit {r.returncode})' if r.returncode else None
        except subprocess.TimeoutExpired:
            return name, 'HANG'

    with ThreadPoolExecutor(8) as ex:
        bad = [(n, s) for n, s in ex.map(one, docs) if s]
    for n, s in bad:
        print(f'{s:16s} {n}')
    print(f'{len(docs)} fixtures, {len(bad)} problems')


if __name__ == '__main__':
    main()
