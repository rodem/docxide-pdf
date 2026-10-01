#!/usr/bin/env python3
"""Score a docxide CLI build on an external corpus of Word documents with the
suite's own page metrics (Jaccard / SSIM / text boundary):

    python3 tools/corpus_score.py <corpus_dir> <docxide-cli> <label> [--jobs N]

<corpus_dir> holds docx/<name>.docx and pdf/<name>.pdf pairs (Word's export
of each document). Results go to tests/output/corpus/<label>.json keyed by
<name>; generated PDFs to tests/output/corpus/<label>/. Reference pages are
rasterised once into tests/output/corpus/_refpng/. Build the scorer first:
`cd tools && cargo build --release --bin page-metrics`.

Fixtures only cover so much; a few hundred extra documents catch rules that
help one fixture and hurt elsewhere. Compare runs with tools/corpus_compare.py.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE_METRICS = ROOT / 'tools/target/release/page-metrics'
OUT = ROOT / 'tests/output/corpus'


def raster(pdf: Path, dest: Path) -> None:
    if not dest.exists():
        dest.mkdir(parents=True)
        subprocess.run(['mutool', 'draw', '-F', 'png', '-r', '150', '-o', str(dest / 'page_%03d.png'), str(pdf)],
                       capture_output=True)


def score(docx: Path, ref: Path, cli: str, out: Path) -> dict | None:
    name = docx.stem
    work = out / name
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir(parents=True)
    pdf = work / 'generated.pdf'
    env = dict(os.environ, DOCXSIDE_FONTS=str(ROOT / 'fonts'), DOCXSIDE_NO_FONT_CACHE='1')
    try:
        subprocess.run([cli, str(docx), str(pdf)], env=env, capture_output=True, timeout=300)
    except subprocess.TimeoutExpired:
        pass
    if not pdf.exists():
        return {'jaccard': 0, 'ssim': 0, 'failed': True}
    refpng = OUT / '_refpng' / name
    raster(ref, refpng)
    raster(pdf, work / 'png')
    r = subprocess.run([str(PAGE_METRICS), str(ref), str(pdf), str(refpng), str(work / 'png')],
                       capture_output=True, text=True)
    shutil.rmtree(work / 'png', ignore_errors=True)
    try:
        return json.loads(r.stdout)
    except json.JSONDecodeError:
        return {'jaccard': 0, 'ssim': 0, 'failed': True}


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument('corpus_dir', type=Path)
    p.add_argument('cli')
    p.add_argument('label')
    p.add_argument('--jobs', type=int, default=8)
    args = p.parse_args()
    if not PAGE_METRICS.exists():
        raise SystemExit(f'missing {PAGE_METRICS}: cd tools && cargo build --release --bin page-metrics')
    pairs = [(d, args.corpus_dir / 'pdf' / f'{d.stem}.pdf') for d in sorted((args.corpus_dir / 'docx').glob('*.docx'))]
    pairs = [(d, r) for d, r in pairs if r.exists()]
    out = OUT / args.label
    with ThreadPoolExecutor(args.jobs) as ex:
        results = dict(zip((d.stem for d, _ in pairs),
                           ex.map(lambda pr: score(pr[0], pr[1], args.cli, out), pairs)))
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / f'{args.label}.json').write_text(json.dumps(results, indent=1))
    ok = [r for r in results.values() if r]
    print(f"n={len(ok)} J {statistics.mean((r['jaccard'] or 0) for r in ok) * 100:.2f} "
          f"SSIM {statistics.mean((r['ssim'] or 0) for r in ok) * 100:.2f} "
          f"failed {sum(1 for r in ok if r.get('failed'))} "
          f"page mismatch {sum(1 for r in ok if r.get('pages') != r.get('ref_pages'))}")


if __name__ == '__main__':
    main()
