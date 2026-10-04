//! Office Math (OMML) read aloud, for the `/Alt` of the Formula a math zone is
//! tagged as (PDF/UA 7.7-1): Word's export speaks `T=2π√(l/g)` as "cap T
//! equals 2 pi root 2 of l over g". The rules follow what Word writes.
// ponytail: the structures and symbols the corpus uses; matrices, accents and
// equation arrays are read as their contents in order

use super::MATH_NS;

/// The spoken form of an `m:oMath` zone; empty when it has nothing to say.
pub(super) fn speak(omath: roxmltree::Node) -> String {
    let mut words = Vec::new();
    speak_into(omath, &mut words);
    words.join(" ")
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children().find(|n| n.has_tag_name((MATH_NS, name)))
}

fn val<'a>(node: roxmltree::Node<'a, 'a>, pr: &str, name: &str) -> Option<&'a str> {
    child(child(node, pr)?, name)?.attribute((MATH_NS, "val"))
}

fn speak_into(node: roxmltree::Node, out: &mut Vec<String>) {
    for c in node
        .children()
        .filter(|n| n.tag_name().namespace() == Some(MATH_NS))
    {
        let part = |name: &str, out: &mut Vec<String>| {
            if let Some(n) = child(c, name) {
                speak_into(n, out);
            }
        };
        match c.tag_name().name() {
            "r" => speak_run(c, out),
            "f" => {
                part("num", out);
                out.push("over".into());
                part("den", out);
            }
            "rad" => {
                let deg = child(c, "deg").filter(|d| d.descendants().any(|n| n.is_text()));
                match deg {
                    Some(d)
                        if val(c, "radPr", "degHide").is_none_or(|v| v == "0" || v == "off") =>
                    {
                        out.push("root".into());
                        speak_into(d, out);
                        out.push("of".into());
                    }
                    _ => out.push("square root of".into()),
                }
                part("e", out);
            }
            "sSub" | "sSup" | "sSubSup" => {
                part("e", out);
                if child(c, "sub").is_some() {
                    out.push("sub".into());
                    part("sub", out);
                }
                if child(c, "sup").is_some() {
                    out.push("to the".into());
                    part("sup", out);
                }
            }
            "d" => {
                let beg = val(c, "dPr", "begChr").unwrap_or("(");
                let end = val(c, "dPr", "endChr").unwrap_or(")");
                out.extend(delimiter(beg, "open"));
                let mut first = true;
                for e in c.children().filter(|n| n.has_tag_name((MATH_NS, "e"))) {
                    if !first {
                        out.push("comma".into());
                    }
                    first = false;
                    speak_into(e, out);
                }
                out.extend(delimiter(end, "close"));
            }
            "nary" => {
                let op = match val(c, "naryPr", "chr").unwrap_or("∫") {
                    "∑" => "sum",
                    "∏" => "product",
                    "∮" => "contour integral",
                    "∫" => "integral",
                    other => other,
                };
                out.push(op.into());
                if child(c, "sub").is_some_and(|n| n.descendants().any(|t| t.is_text())) {
                    out.push("from".into());
                    part("sub", out);
                }
                if child(c, "sup").is_some_and(|n| n.descendants().any(|t| t.is_text())) {
                    out.push("to".into());
                    part("sup", out);
                }
                out.push("of".into());
                part("e", out);
            }
            name if name.ends_with("Pr") => {}
            _ => speak_into(c, out),
        }
    }
}

fn delimiter(chr: &str, side: &str) -> Option<String> {
    let name = match chr {
        "" => return None,
        "(" | ")" => "paren",
        "[" | "]" => "bracket",
        "{" | "}" => "brace",
        "|" => return Some("vertical bar".into()),
        other => return Some(other.into()),
    };
    Some(format!("{side} {name}"))
}

/// One `m:r`: its characters with the run's math style (`m:sty`), which Word
/// says before each letter ("bold italic cap T") and digit ("bold 1").
fn speak_run(r: roxmltree::Node, out: &mut Vec<String>) {
    let text: String = r
        .children()
        .filter(|n| n.has_tag_name((MATH_NS, "t")))
        .filter_map(|t| t.text())
        .collect();
    let sty = val(r, "rPr", "sty");
    // Plain (upright) text is a word: a function name or a unit.
    if sty == Some("p") || child(r, "rPr").and_then(|p| child(p, "nor")).is_some() {
        out.extend(text.split_whitespace().map(str::to_string));
        return;
    }
    let bold = matches!(sty, Some("b" | "bi"));
    let mut number = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() && !bold {
            number.push(ch);
            continue;
        }
        if !number.is_empty() {
            out.push(std::mem::take(&mut number));
        }
        if ch.is_whitespace() {
            continue;
        }
        let (word, letter) = match symbol_word(ch) {
            Some(w) => (w.to_string(), false),
            None => match letter_word(ch) {
                Some(w) => (w, true),
                None => (ch.to_string(), ch.is_ascii_digit()),
            },
        };
        let prefix = match (bold, letter && !ch.is_ascii_digit(), sty) {
            (true, true, Some("bi")) => Some("bold italic"),
            (true, true, _) => Some("bold"),
            (true, false, _) if ch.is_ascii_digit() => Some("bold"),
            _ => None,
        };
        out.extend(prefix.map(str::to_string));
        out.push(word);
    }
    if !number.is_empty() {
        out.push(number);
    }
}

/// A Latin or Greek letter as itself or its name, capitals with "cap" before.
fn letter_word(ch: char) -> Option<String> {
    const GREEK: [&str; 25] = [
        "alpha",
        "beta",
        "gamma",
        "delta",
        "epsilon",
        "zeta",
        "eta",
        "theta",
        "iota",
        "kappa",
        "lambda",
        "mu",
        "nu",
        "xi",
        "omicron",
        "pi",
        "rho",
        "final sigma",
        "sigma",
        "tau",
        "upsilon",
        "phi",
        "chi",
        "psi",
        "omega",
    ];
    let (name, cap) = match ch {
        'a'..='z' => (ch.to_string(), false),
        'A'..='Z' => (ch.to_string(), true),
        'α'..='ω' => (GREEK[ch as usize - 'α' as usize].to_string(), false),
        // U+03A2 is unassigned, so the capitals line up with GREEK.
        'Α'..='Ω' if ch != '\u{3A2}' => (GREEK[ch as usize - 'Α' as usize].to_string(), true),
        'ϑ' => ("script theta".into(), false),
        'ϕ' => ("phi".into(), false),
        'ϵ' => ("epsilon".into(), false),
        _ => return None,
    };
    Some(if cap { format!("cap {name}") } else { name })
}

fn symbol_word(ch: char) -> Option<&'static str> {
    Some(match ch {
        '=' => "equals",
        '+' => "plus",
        '-' | '−' => "minus",
        '±' => "plus or minus",
        '∓' => "minus or plus",
        '×' => "times",
        '·' | '⋅' | '∙' => "dot",
        '÷' => "divided by",
        '/' => "slash",
        '<' => "less than",
        '>' => "greater than",
        '≤' => "less than or equal to",
        '≥' => "greater than or equal to",
        '≪' => "much less than",
        '≫' => "much greater than",
        '≠' => "not equal to",
        '≈' => "almost equal to",
        '≡' => "identical to",
        '∝' => "proportional to",
        '∞' => "infinity",
        '∆' => "increment",
        '∂' => "partial",
        '∇' => "nabla",
        '→' => "right arrow",
        '√' => "square root",
        '%' => "percent",
        '°' => "degrees",
        '′' => "prime",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spoken(omml: &str) -> String {
        let xml = format!(
            r#"<m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">{omml}</m:oMath>"#
        );
        let doc = roxmltree::Document::parse(&xml).unwrap();
        speak(doc.root_element())
    }

    // Expected strings are Word's own /Alt in pendulum_mechanics_oscillation_lab.
    #[test]
    fn speaks_like_word() {
        assert_eq!(
            spoken(
                "<m:r><m:t>T=2π</m:t></m:r><m:rad><m:deg><m:r><m:t>2</m:t></m:r></m:deg>\
                 <m:e><m:f><m:num><m:r><m:t>l</m:t></m:r></m:num>\
                 <m:den><m:r><m:t>g</m:t></m:r></m:den></m:f></m:e></m:rad>"
            ),
            "cap T equals 2 pi root 2 of l over g"
        );
        assert_eq!(
            spoken(
                "<m:r><m:t>ϑ≪</m:t></m:r><m:f><m:num><m:r><m:t>π</m:t></m:r></m:num>\
                 <m:den><m:r><m:t>2</m:t></m:r></m:den></m:f>"
            ),
            "script theta much less than pi over 2"
        );
        let bi = r#"<m:rPr><m:sty m:val="bi"/></m:rPr>"#;
        let b = r#"<m:rPr><m:sty m:val="b"/></m:rPr>"#;
        assert_eq!(
            spoken(&format!("<m:r>{bi}<m:t>l±∆l</m:t></m:r>")),
            "bold italic l plus or minus increment bold italic l"
        );
        let t10 = format!(
            "<m:sSub><m:e><m:r>{bi}<m:t>T</m:t></m:r></m:e><m:sub><m:r>{bi}<m:t>10</m:t></m:r></m:sub></m:sSub>"
        );
        assert_eq!(
            spoken(&format!(
                "{t10}<m:r>{bi}<m:t>±</m:t></m:r><m:r>{b}<m:t>Δ</m:t></m:r>{t10}"
            )),
            "bold italic cap T sub bold 1 bold 0 plus or minus bold cap delta \
             bold italic cap T sub bold 1 bold 0"
        );
        assert_eq!(
            spoken(&format!(
                "<m:sSubSup><m:e><m:r>{bi}<m:t>T</m:t></m:r></m:e><m:sub><m:r>{bi}<m:t>1</m:t></m:r></m:sub>\
                 <m:sup><m:r>{bi}<m:t>2</m:t></m:r></m:sup></m:sSubSup>"
            )),
            "bold italic cap T sub bold 1 to the bold 2"
        );
        assert_eq!(spoken("<m:r><m:t>l </m:t></m:r>"), "l");
        assert_eq!(spoken("<m:r><m:t> </m:t></m:r>"), "");
    }
}
