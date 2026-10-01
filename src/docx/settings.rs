use std::io::{Read, Seek};

use super::{WML_NS, read_zip_text, twips_to_pts, wml, wml_attr, wml_bool};

pub(super) struct DocumentSettings {
    pub even_and_odd_headers: bool,
    pub default_tab_stop: f32,
    #[allow(dead_code)]
    pub mirror_margins: bool,
    pub gutter_at_top: bool,
    pub east_asia_lang: Option<String>,
    /// `w:themeFontLang @bidi`: picks the theme's complex-script font.
    pub bidi_lang: Option<String>,
    pub auto_hyphenation: bool,
    pub default_lang: Option<String>,
    /// §17.15.1.15 `w:characterSpacingControl` is `compressPunctuation` or
    /// `compressPunctuationAndJapaneseKana`: Word may squeeze full-width East
    /// Asian punctuation to keep one more character on a line.
    pub compress_punctuation: bool,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            even_and_odd_headers: false,
            default_tab_stop: 36.0, // 0.5 inches = 720 twips = 36pt
            mirror_margins: false,
            gutter_at_top: false,
            east_asia_lang: None,
            bidi_lang: None,
            auto_hyphenation: false,
            default_lang: None,
            compress_punctuation: false,
        }
    }
}

pub(super) fn parse_settings<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
) -> DocumentSettings {
    let Some(xml_text) = read_zip_text(zip, "word/settings.xml") else {
        return DocumentSettings::default();
    };
    let Ok(doc) = roxmltree::Document::parse(&xml_text) else {
        return DocumentSettings::default();
    };
    let root = doc.root_element();

    let default_tab_stop = wml_attr(root, "defaultTabStop")
        .and_then(|v| v.parse::<f32>().ok())
        .map(twips_to_pts)
        .unwrap_or(36.0);

    let theme_font_lang = wml(root, "themeFontLang");
    let lang = |attr| {
        theme_font_lang
            .and_then(|n| n.attribute((WML_NS, attr)))
            .map(str::to_string)
    };
    let east_asia_lang = lang("eastAsia");
    let default_lang = lang("val");
    let bidi_lang = lang("bidi");

    DocumentSettings {
        even_and_odd_headers: wml_bool(root, "evenAndOddHeaders").unwrap_or(false),
        default_tab_stop,
        mirror_margins: wml_bool(root, "mirrorMargins").unwrap_or(false),
        gutter_at_top: wml_bool(root, "gutterAtTop").unwrap_or(false),
        east_asia_lang,
        bidi_lang,
        auto_hyphenation: wml_bool(root, "autoHyphenation").unwrap_or(false),
        default_lang,
        compress_punctuation: wml_attr(root, "characterSpacingControl")
            .is_some_and(|v| v.starts_with("compressPunctuation")),
    }
}
