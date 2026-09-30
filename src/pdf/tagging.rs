//! Tagged PDF (PDF/UA): a structure tree built while rendering and written at
//! assembly.
//!
//! Body page streams are artifacts by default: each opens `/Artifact BMC` when
//! it is created and assembly closes it. A tagged region closes the artifact,
//! opens its own marked content and reopens the artifact when it ends, so
//! everything drawn between regions (shading, borders, rules, separators) is
//! never untagged content.

use pdf_writer::writers::StructTreeRoot;
use pdf_writer::{Content, Name, Pdf, Ref};

pub(super) const ROOT: usize = 0;

enum Kid {
    Node(usize),
    Mcid { page: usize, mcid: i32 },
}

struct Node {
    kind: &'static str,
    parent: usize,
    kids: Vec<Kid>,
}

pub(crate) struct Tags {
    nodes: Vec<Node>,
    /// Marked-content ids are unique per page.
    next_mcid: Vec<i32>,
}

/// A fresh body page stream, inside the default artifact.
pub(super) fn artifact_content() -> Content {
    let mut content = Content::new();
    content.begin_marked_content(Name(b"Artifact"));
    content
}

/// Wrap a finished stream that holds no real content (headers, footers, page
/// borders, the comment pane) as one artifact.
pub(super) fn wrap_artifact(out: &mut Vec<u8>, raw: &[u8], pagination: bool) {
    out.extend_from_slice(if pagination {
        b"/Artifact <</Type /Pagination>> BDC\n"
    } else {
        b"/Artifact BMC\n"
    });
    out.extend_from_slice(raw);
    out.extend_from_slice(b"\nEMC\n");
}

/// Drop the `/Artifact BMC EMC` pairs left with nothing inside when tagged
/// regions follow each other.
pub(super) fn strip_empty_artifacts(raw: &[u8]) -> Vec<u8> {
    const EMPTY: &[u8] = b"/Artifact BMC\nEMC\n";
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i..].starts_with(EMPTY) {
            i += EMPTY.len();
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    out
}

impl Tags {
    pub(super) fn new() -> Self {
        Self {
            nodes: vec![Node { kind: "Document", parent: ROOT, kids: Vec::new() }],
            next_mcid: Vec::new(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }

    pub(super) fn add(&mut self, parent: usize, kind: &'static str) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { kind, parent, kids: Vec::new() });
        self.nodes[parent].kids.push(Kid::Node(id));
        id
    }

    /// Start a piece of `node`'s content on `page`; it runs until `end`.
    /// `content` must be a body stream (inside the default artifact).
    pub(super) fn begin(&mut self, content: &mut Content, page: usize, node: usize) {
        if self.next_mcid.len() <= page {
            self.next_mcid.resize(page + 1, 0);
        }
        let mcid = self.next_mcid[page];
        self.next_mcid[page] += 1;
        self.nodes[node].kids.push(Kid::Mcid { page, mcid });
        content.end_marked_content();
        content
            .begin_marked_content_with_properties(Name(self.nodes[node].kind.as_bytes()))
            .properties()
            .identify(mcid);
    }

    pub(super) fn end(content: &mut Content) {
        content.end_marked_content();
        content.begin_marked_content(Name(b"Artifact"));
    }

    /// Page `p`'s `/StructParents` key, if it has tagged content.
    pub(super) fn struct_parents(&self, page: usize) -> Option<i32> {
        (self.next_mcid.get(page).copied().unwrap_or(0) > 0).then_some(page as i32)
    }

    /// Write the elements and parent tree; returns the StructTreeRoot ref.
    pub(super) fn write(&self, pdf: &mut Pdf, alloc: &mut impl FnMut() -> Ref, page_ids: &[Ref]) -> Ref {
        let root = alloc();
        let refs: Vec<Ref> = self.nodes.iter().map(|_| alloc()).collect();
        let mut owners: Vec<Vec<Option<Ref>>> =
            self.next_mcid.iter().map(|&n| vec![None; n as usize]).collect();

        for (i, node) in self.nodes.iter().enumerate() {
            let mut elem = pdf.struct_element(refs[i]);
            elem.custom_kind(Name(node.kind.as_bytes()));
            elem.parent(if i == ROOT { root } else { refs[node.parent] });
            // Content on the element's own /Pg is a bare MCID; only a paragraph
            // continued on the next page needs full marked-content references.
            let first_page = node.kids.iter().find_map(|k| match k {
                Kid::Mcid { page, .. } => Some(*page),
                Kid::Node(_) => None,
            });
            if let Some(p) = first_page {
                elem.page(page_ids[p]);
            }
            let mut kids = elem.children();
            for kid in &node.kids {
                match *kid {
                    Kid::Node(c) => {
                        kids.struct_element(refs[c]);
                    }
                    Kid::Mcid { page, mcid } => {
                        owners[page][mcid as usize] = Some(refs[i]);
                        if Some(page) == first_page {
                            kids.marked_content_id(mcid);
                        } else {
                            kids.marked_content_ref().marked_content_id(mcid).page(page_ids[page]);
                        }
                    }
                }
            }
        }

        let arrays: Vec<(i32, Ref)> = owners
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.is_empty())
            .map(|(page, o)| {
                let id = alloc();
                pdf.indirect(id).array().items(o.iter().map(|r| r.expect("every MCID has an owner")));
                (page as i32, id)
            })
            .collect();
        let mut tree = pdf.indirect(root).start::<StructTreeRoot>();
        tree.child(refs[ROOT]);
        {
            let mut parent_tree = tree.parent_tree();
            let mut nums = parent_tree.nums();
            for (key, id) in arrays {
                nums.insert(key, id);
            }
        }
        tree.parent_tree_next_key(page_ids.len() as i32);
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagged_regions_leave_no_empty_artifacts() {
        let mut tags = Tags::new();
        let p = tags.add(ROOT, "P");
        let mut content = artifact_content();
        tags.begin(&mut content, 0, p);
        Tags::end(&mut content);
        content.rect(0.0, 0.0, 1.0, 1.0).fill_nonzero();
        tags.begin(&mut content, 0, p);
        Tags::end(&mut content);
        let mut raw = content.finish().to_vec();
        raw.extend_from_slice(b"\nEMC\n");
        let out = String::from_utf8(strip_empty_artifacts(&raw)).unwrap();
        // The rectangle keeps its artifact; the empty ones around the tags go.
        assert_eq!(out.matches("/Artifact BMC").count(), 1);
        assert_eq!(out.matches("BMC").count() + out.matches("BDC").count(), out.matches("EMC").count());
        assert_eq!(tags.struct_parents(0), Some(0));
    }
}
