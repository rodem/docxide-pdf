#!/usr/bin/env python3
"""Render chosen corpus documents with a docxide CLI binary into a vdiff tree
and print vdiff's summary (and optionally the page metrics):

    python3 tools/corpus_view.py <corpus_dir> <label> <docxide-cli> <name-prefix>... [-v] [--score]

<corpus_dir> is laid out as for tools/corpus_score.py; a name prefix is enough
to pick a document. Output goes to tests/output/corpus_view/<label>/. -v
prints vdiff's full line table; --score adds Jaccard/SSIM via page-metrics.
vdiff is looked up in $VDIFF, else tools/target/debug/vdiff.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VDIFF = os.environ.get('VDIFF', str(ROOT / 'tools/target/debug/vdiff'))
PAGE_METRICS = ROOT / 'tools/target/release/page-metrics'
SUMMARY = ('===', 'pages:', '  page', '  first')
GROUP = 'new'


def raster(pdf: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    subprocess.run(['mutool', 'draw', '-F', 'png', '-r', '150', '-o', str(dest / 'page_%03d.png'), str(pdf)],
                   capture_output=True)


def main() -> None:
    flags = {'-v', '--score'}
    args = [a for a in sys.argv[1:] if a not in flags]
    verbose, want_score = '-v' in sys.argv, '--score' in sys.argv
    corpus, label, cli, prefixes = Path(args[0]), args[1], args[2], args[3:]
    root = ROOT / 'tests/output/corpus_view' / label
    # vdiff only searches the suite's fixture groups, so file documents under one.
    (root / 'tests/fixtures' / GROUP).mkdir(parents=True, exist_ok=True)
    (root / 'Cargo.toml').touch()  # vdiff looks for the project root
    env = dict(os.environ, DOCXSIDE_FONTS=str(ROOT / 'fonts'), DOCXSIDE_NO_FONT_CACHE='1')
    for prefix in prefixes:
        docx = next((corpus / 'docx').glob(prefix + '*.docx'), None)
        if docx is None:
            print(f'no document starting with {prefix}')
            continue
        name = docx.stem[:40]
        ref = corpus / 'pdf' / f'{docx.stem}.pdf'
        fx = root / 'tests/fixtures' / GROUP / name
        fx.mkdir(exist_ok=True)
        for link, src in (('input.docx', docx), ('reference.pdf', ref)):
            if not (fx / link).exists():
                (fx / link).symlink_to(src.resolve())
        out = root / 'tests/output' / GROUP / name
        out.mkdir(parents=True, exist_ok=True)
        pdf = out / 'generated.pdf'
        pdf.unlink(missing_ok=True)  # the CLI never overwrites an existing file
        subprocess.run([cli, str(docx), str(pdf)], env=env, capture_output=True)
        report = subprocess.run([VDIFF, name] + (['--verbose'] if verbose else []),
                                cwd=root, capture_output=True, text=True).stdout
        lines = report.splitlines() if verbose else [l for l in report.splitlines() if l.startswith(SUMMARY)]
        print('\n'.join(lines))
        if want_score and pdf.exists():
            tmp = Path(tempfile.mkdtemp())
            raster(ref, tmp / 'ref')
            raster(pdf, tmp / 'gen')
            r = subprocess.run([str(PAGE_METRICS), str(ref), str(pdf), str(tmp / 'ref'), str(tmp / 'gen')],
                               capture_output=True, text=True)
            shutil.rmtree(tmp, ignore_errors=True)
            try:
                m = json.loads(r.stdout)
                print(f"  J {m['jaccard'] * 100:.1f}  SSIM {m['ssim'] * 100:.1f}  pages {m.get('pages')}/{m.get('ref_pages')}")
            except (json.JSONDecodeError, KeyError, TypeError):
                print('  (no metrics)')


if __name__ == '__main__':
    main()
