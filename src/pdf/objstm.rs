//! Compressed object streams (PDF 1.5) as a post-pass over pdf-writer output.
//!
//! pdf-writer writes every object plain and ends with a classic xref table. A
//! tagged document adds thousands of small StructElem dictionaries, each paying
//! for its `obj`/`endobj` framing and a 20-byte xref line. This pass moves every
//! non-stream object into Flate-compressed `/ObjStm` streams and replaces the
//! xref table with a compressed cross-reference stream, as Word's export does.
//! Offsets come from pdf-writer's own xref table, so no content is parsed; if
//! the file doesn't look exactly like pdf-writer output it is returned as is.

use std::fmt::Write as _;

const OBJECTS_PER_STREAM: usize = 256;

pub(super) fn pack(pdf: Vec<u8>) -> Vec<u8> {
    try_pack(&pdf).unwrap_or(pdf)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn leading_int(s: &[u8]) -> Option<usize> {
    let digits = s.iter().take_while(|b| b.is_ascii_digit()).count();
    std::str::from_utf8(&s[..digits]).ok()?.parse().ok()
}

fn try_pack(pdf: &[u8]) -> Option<Vec<u8>> {
    let startxref = rfind(pdf, b"startxref\n")?;
    let xref_at = leading_int(&pdf[startxref + 10..])?;
    let xref = pdf.get(xref_at..startxref)?;
    let xref = xref.strip_prefix(b"xref\n0 ")?;
    let size = leading_int(xref)?;
    let entries_at = find(xref, b"\n")? + 1;
    let entries = xref.get(entries_at..entries_at + size * 20)?;
    let trailer = xref.get(entries_at + size * 20..)?.strip_prefix(b"trailer\n")?;

    // (id, offset) of in-use objects, in file order.
    let mut objects: Vec<(usize, usize)> = Vec::new();
    for (id, line) in entries.chunks_exact(20).enumerate() {
        if line[17] == b'n' {
            objects.push((id, leading_int(line)?));
        }
    }
    objects.sort_by_key(|&(_, off)| off);
    let first_obj = objects.first()?.1;

    let mut out = pdf[..first_obj].to_vec();
    let mut offsets = vec![None; size]; // type-1 entries: byte offset
    let mut packed: Vec<(usize, &[u8])> = Vec::new();
    for (i, &(id, off)) in objects.iter().enumerate() {
        let end = objects.get(i + 1).map_or(xref_at, |o| o.1);
        let raw = pdf.get(off..end)?;
        let header = format!("{id} 0 obj\n");
        let body = raw.strip_prefix(header.as_bytes())?;
        let body = &body[..rfind(body, b"endobj")?];
        let body = body.strip_suffix(b"\n").unwrap_or(body);
        // Streams can't live in object streams; anything that might be one stays.
        if find(body, b"stream\n").is_some() {
            offsets[id] = Some(out.len());
            out.extend_from_slice(raw);
        } else {
            packed.push((id, body));
        }
    }
    if packed.is_empty() {
        return None;
    }

    let mut in_stream = vec![None; size]; // type-2 entries: (stream id, index)
    let mut next_id = size;
    for group in packed.chunks(OBJECTS_PER_STREAM) {
        let stream_id = next_id;
        next_id += 1;
        let mut index = String::new();
        let mut data = Vec::new();
        for (i, &(id, body)) in group.iter().enumerate() {
            let _ = write!(index, "{id} {} ", data.len());
            data.extend_from_slice(body);
            data.push(b'\n');
            in_stream[id] = Some((stream_id, i));
        }
        let mut plain = index.into_bytes();
        let first = plain.len();
        plain.extend_from_slice(&data);
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&plain, 6);
        offsets.push(Some(out.len()));
        in_stream.push(None);
        let _ = write!(
            ByteWriter(&mut out),
            "{stream_id} 0 obj\n<< /Type /ObjStm /N {} /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
            group.len(),
            compressed.len()
        );
        out.extend_from_slice(&compressed);
        out.extend_from_slice(b"\nendstream\nendobj\n\n");
    }

    // Cross-reference stream: W [1 4 2] = type, offset or stream id, index.
    let xref_id = next_id;
    let xref_size = xref_id + 1;
    offsets.push(Some(out.len()));
    in_stream.push(None);
    let mut rows = Vec::with_capacity(xref_size * 7);
    for id in 0..xref_size {
        let (kind, a, b) = match (offsets[id], in_stream[id]) {
            (Some(off), _) => (1u8, u32::try_from(off).ok()?, 0u16),
            (None, Some((sid, idx))) => (2, sid as u32, u16::try_from(idx).ok()?),
            (None, None) => (0, 0, if id == 0 { 65535 } else { 0 }),
        };
        rows.push(kind);
        rows.extend_from_slice(&a.to_be_bytes());
        rows.extend_from_slice(&b.to_be_bytes());
    }
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&rows, 6);
    // Keep the trailer's own keys (Root, Info, ID) minus the old /Size.
    let trailer = std::str::from_utf8(&trailer[..rfind(trailer, b">>")?]).ok()?;
    let keys: String = trailer
        .trim_start_matches("<<")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("/Size"))
        .map(|l| format!(" {l}"))
        .collect();
    let xref_at = out.len();
    let _ = write!(
        ByteWriter(&mut out),
        "{xref_id} 0 obj\n<< /Type /XRef /Size {xref_size} /W [1 4 2]{keys} /Filter /FlateDecode /Length {} >>\nstream\n",
        compressed.len()
    );
    out.extend_from_slice(&compressed);
    let _ = write!(ByteWriter(&mut out), "\nendstream\nendobj\n\nstartxref\n{xref_at}\n%%EOF");
    Some(out)
}

struct ByteWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for ByteWriter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_writer::{Name, Pdf, Rect, Ref};

    #[test]
    fn packs_dictionaries_and_keeps_streams() {
        let mut pdf = Pdf::new();
        pdf.catalog(Ref::new(1)).pages(Ref::new(2));
        pdf.pages(Ref::new(2)).kids([Ref::new(3)]).count(1);
        pdf.page(Ref::new(3))
            .parent(Ref::new(2))
            .media_box(Rect::new(0.0, 0.0, 100.0, 100.0))
            .contents(Ref::new(4));
        pdf.stream(Ref::new(4), b"0 0 10 10 re f");
        pdf.struct_element(Ref::new(6)).custom_kind(Name(b"P"));
        let packed = pack(pdf.finish());
        let text = String::from_utf8_lossy(&packed);
        assert!(text.contains("/Type /ObjStm /N 4"), "catalog, pages, page, struct elem");
        assert!(text.contains("4 0 obj"), "the content stream stays a plain object");
        assert!(text.contains("/Type /XRef /Size 9 /W [1 4 2] /Root 1 0 R"));
        let startxref = rfind(&packed, b"startxref\n").unwrap();
        let at = leading_int(&packed[startxref + 10..]).unwrap();
        assert!(packed[at..].starts_with(b"8 0 obj\n<< /Type /XRef"));
        assert!(!text.contains("\nxref\n"));
    }
}
