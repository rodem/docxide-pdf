#!/usr/bin/env python3
"""Compare two tools/corpus_score.py runs: mean Jaccard, then the documents
that moved most in each direction.

    python3 tools/corpus_compare.py <before label> <after label> [--top N]

A rule that lifts the mean but sinks a few documents hard deserves a look at
those few before it is committed.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'tests/output/corpus'


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument('before')
    p.add_argument('after')
    p.add_argument('--top', type=int, default=8)
    args = p.parse_args()
    a = json.loads((OUT / f'{args.before}.json').read_text())
    b = json.loads((OUT / f'{args.after}.json').read_text())
    keys = [k for k in a if a[k] and b.get(k)]
    ja = sum((a[k]['jaccard'] or 0) for k in keys) / len(keys) * 100
    jb = sum((b[k]['jaccard'] or 0) for k in keys) / len(keys) * 100
    print(f"n={len(keys)} J {ja:.2f} -> {jb:.2f}")
    moves = sorted(((b[k]['jaccard'] or 0) - (a[k]['jaccard'] or 0), k) for k in keys)
    moves = [m for m in moves if abs(m[0]) > 0.005]
    shown = moves[:args.top] + [m for m in moves[-args.top:] if m not in moves[:args.top]]
    for d, k in shown:
        print(f"  {k[:40]:40s} {(a[k]['jaccard'] or 0) * 100:5.1f} -> {(b[k]['jaccard'] or 0) * 100:5.1f} ({d * 100:+.1f})")


if __name__ == '__main__':
    main()
