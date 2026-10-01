#!/usr/bin/env python3
"""Copy a .docx with literal text replacements in one part, for what-if
experiments (does this property really drive that layout?):

    python3 tools/docx_edit.py <in.docx> <out.docx> <part> <old> <new> [<old> <new>...]

e.g. tools/docx_edit.py in.docx /tmp/t.docx word/styles.xml 'w:line="259"' 'w:line="480"'.
Fails if an <old> string is not found.
"""
import sys
import zipfile


def main() -> None:
    src, dst, part = sys.argv[1:4]
    pairs = list(zip(sys.argv[4::2], sys.argv[5::2]))
    zin = zipfile.ZipFile(src)
    with zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename == part:
                text = data.decode('utf8')
                for old, new in pairs:
                    if old not in text:
                        raise SystemExit(f'not found in {part}: {old}')
                    text = text.replace(old, new)
                data = text.encode('utf8')
            zout.writestr(item, data)


if __name__ == '__main__':
    main()
