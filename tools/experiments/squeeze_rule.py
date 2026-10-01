"""Measure Word 2013's justified-squeeze rule in reference PDFs (compat-15 docs
only); this is the evidence behind SPACE_SQUEEZE in src/pdf/layout.rs.

    python3 tools/experiments/squeeze_rule.py tests/fixtures/*/*/input.docx [more .docx...]

For each justified-looking line L followed by line N in the same paragraph block
(same left x or first-line indent, consecutive baselines), natural widths come
from the font hmtx tables:
  nat      = natural width of L's text (no trailing space)
  avail    = right edge of L's last glyph (Word justifies to the margin)
  spaces   = number of spaces in L, sw = natural space width
  w        = natural width of N's first word
Accepted squeeze: L itself is squeezed (nat > avail). Refused: nat + sw + w > avail
(did not fit naturally) and Word wrapped the word. Prints both populations and the
accuracy of candidate rules. Each .docx's PDF is reference.pdf beside input.docx,
or ../pdf/<name>.pdf for a docx/ + pdf/ corpus. Needs fontTools and mutool.
"""
import glob, html, re, subprocess, sys, zipfile
from pathlib import Path
from fontTools.ttLib import TTFont

FONT_DIRS = [str(Path(__file__).resolve().parents[2] / 'fonts'), '/System/Library/Fonts/Supplemental', '/Library/Fonts']
fonts = {}
for d in FONT_DIRS:
    for p in glob.glob(d + '/**/*', recursive=True):
        if not p.lower().endswith(('.ttf', '.otf')):
            continue
        try:
            f = TTFont(p, lazy=True)
            ps = f['name'].getDebugName(6)
            if ps and ps not in fonts:
                cm = f.getBestCmap()
                hm = f['hmtx']
                u = f['head'].unitsPerEm
                fonts[ps] = {cp: hm[g][0] / u for cp, g in cm.items()}
        except Exception:
            pass


def compat(docx):
    try:
        s = zipfile.ZipFile(docx).read('word/settings.xml').decode('utf8', 'ignore')
    except Exception:
        return None
    m = re.search(r'compatibilityMode"[^>]*w:val="(\d+)"', s)
    return int(m.group(1)) if m else 0


def page_lines(pdf):
    st = subprocess.run(['mutool', 'draw', '-F', 'stext', '-o', '-', pdf], capture_output=True, text=True).stdout
    for page in re.findall(r'<page .*?</page>', st, re.S):
        out = []
        for body in re.findall(r'<line [^>]*>(.*?)</line>', page, re.S):
            chars = []
            for fa, fb in re.findall(r'<font ([^>]*)>(.*?)</font>', body, re.S):
                name = re.search(r'name="([^"]*)"', fa).group(1).split('+')[-1]
                size = float(re.search(r'size="([\d.]+)"', fa).group(1))
                for q, x, y, c in re.findall(r'<char quad="([^"]*)" x="([\d.]+)" y="([\d.]+)"[^>]*c="([^"]*)"', fb):
                    qq = [float(v) for v in q.split()]
                    chars.append((float(x), float(y), html.unescape(c), name, size, qq[2]))
            if chars:
                out.append(chars)
        yield out


def nat_w(chars):
    tot = 0.0
    for x, y, c, name, size, qx in chars:
        m = fonts.get(name)
        if not m or ord(c) not in m:
            return None
        tot += m[ord(c)] * size
    return tot


acc, ref = [], []
docs = sys.argv[1:]
for docx in docs:
    pdf = docx.replace('input.docx', 'reference.pdf') if docx.endswith('input.docx') else docx.replace('/docx/', '/pdf/')[:-5] + '.pdf'
    if compat(docx) != 15:
        continue
    for lines in page_lines(pdf):
        for i in range(len(lines) - 1):
            L, N = lines[i], lines[i + 1]
            # strip trailing spaces
            while L and L[-1][2] == ' ':
                L = L[:-1]
            if len(L) < 5 or not N:
                continue
            if abs(N[0][1] - L[0][1]) > 40 or N[0][1] <= L[0][1]:
                continue
            if len({(c[3], c[4]) for c in L}) != 1:
                continue  # single font/size lines only, keeps the widths honest
            name, size = L[0][3], L[0][4]
            m = fonts.get(name)
            if not m or 32 not in m:
                continue
            sw = m[32] * size
            nspace = sum(1 for c in L if c[2] == ' ')
            if nspace < 3:
                continue
            nat = nat_w(L)
            if nat is None:
                continue
            x0 = L[0][0]
            right = L[-1][5]  # right edge of the last glyph's quad
            avail = right - x0
            # first word of N
            word = []
            for c in N:
                if c[2] == ' ':
                    break
                word.append(c)
            w = nat_w(word)
            if w is None or (word and (word[0][3], word[0][4]) != (name, size)):
                continue
            space_total = nspace * sw
            stretch = avail - nat  # >0 stretched, <0 squeezed
            if stretch < -0.3:
                need = nat - avail
                last = []
                for c in reversed(L):
                    if c[2] == ' ':
                        break
                    last.append(c)
                wl = nat_w(last) or 0.0
                # start of the kept word at natural widths, relative to the margin
                acc.append((need / (space_total), need, sw, size, name, nspace, wl, nat - wl - avail, pdf))
            elif stretch > 0.3:
                need = nat + sw + w - avail  # what pulling the next word would have needed
                if need > 0:
                    ref.append((need / (space_total + sw), need, sw, size, name, nspace, w, (nat + sw) - avail, pdf, ''.join(c[2] for c in L)[-40:], ''.join(c[2] for c in word)))

print('accepted squeezes', len(acc), 'refused pulls', len(ref))
import collections
ha = collections.Counter(round(a[0], 2) for a in acc)
hr = collections.Counter(round(r[0], 2) for r in ref if r[0] < 0.4)
print('ratio   accepted  refused')
for k in sorted(set(ha) | set(hr)):
    if k <= 0.4:
        print(f"{k:5.2f}  {ha.get(k, 0):6d}  {hr.get(k, 0):6d}")
# feature: does the pulled word start past the margin?
def best_split(name, fa, fr):
    pts = sorted([(v, 1) for v in fa] + [(v, 0) for v in fr])
    best = (0, None)
    na, nr = len(fa), len(fr)
    ca = cr = 0  # counts <= threshold
    for v, lab in pts:
        if lab:
            ca += 1
        else:
            cr += 1
        acc_ = (ca + (nr - cr)) / (na + nr)  # accepted below, refused above
        if acc_ > best[0]:
            best = (acc_, v)
    print(f"{name:34s} best threshold {best[1]:8.3f} accuracy {best[0]:.3f}")


best_split('need / space total', [a[0] for a in acc], [r[0] for r in ref])
best_split('need (pt)', [a[1] for a in acc], [r[1] for r in ref])
best_split('need / font size', [a[1] / a[3] for a in acc], [r[1] / r[3] for r in ref])
best_split('need / space width', [a[1] / a[2] for a in acc], [r[1] / r[2] for r in ref])
best_split('need per space / size', [a[1] / a[5] / a[3] for a in acc], [r[1] / (r[5] + 1) / r[3] for r in ref])
def rule_acc(name, f):
    ta = sum(1 for a in acc if f(a))
    tr = sum(1 for r in ref if not f(r))
    print(f"{name:44s} accepted kept {ta}/{len(acc)}  refused kept out {tr}/{len(ref)}  acc {(ta + tr) / (len(acc) + len(ref)):.3f}")


rule_acc('ratio<=0.25', lambda x: x[0] <= 0.25)
rule_acc('ratio<=0.25 & starts inside', lambda x: x[0] <= 0.25 and x[7] <= 0)
rule_acc('ratio<=0.25 & starts inside+sw', lambda x: x[0] <= 0.25 and x[7] <= x[2])
rule_acc('ratio<=0.25 & need<=w/2', lambda x: x[0] <= 0.25 and x[1] <= x[6] / 2)
rule_acc('ratio<=0.25 & need<=w', lambda x: x[0] <= 0.25 and x[1] <= x[6])
for cap in (0.2, 0.25, 0.3, 1.0):
    for k in (0.3, 0.4, 0.5, 0.6, 0.75, 1.0, 9.0):
        rule_acc(f'ratio<={cap} & need<={k}w', lambda x, cap=cap, k=k: x[0] <= cap and x[1] <= k * x[6])
import collections
print('refused, ratio<=0.12, starts inside: samples')
for r in [r for r in ref if r[0] <= 0.12 and r[7] <= 0][:14]:
    print(f"  ratio {r[0]:.3f} need {r[1]:5.2f} w {r[6]:5.2f} start {r[7]:6.2f} sz {r[3]} {r[4][:16]} | ...{r[9]!r} + {r[10]!r}  {r[8].split('/')[-2] if 'reference' in r[8] else r[8].split('/')[-1][:14]}")
starts_past = [r for r in ref if r[0] <= 0.25]
print('refused with ratio<=0.25:', len(starts_past), ' of which word would start past margin:', sum(1 for r in starts_past if r[7] > 0))
