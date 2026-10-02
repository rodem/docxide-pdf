use std::io::{Read, Seek};

use super::{WML_NS, read_zip_text, twips_attr, wml, wml_attr, wml_bool};

pub(super) struct DocumentSettings {
    pub even_and_odd_headers: bool,
    pub default_tab_stop: f32,
    pub gutter_at_top: bool,
    pub east_asia_lang: Option<String>,
    /// `w:themeFontLang @bidi`: picks the theme's complex-script font.
    pub bidi_lang: Option<String>,
    pub default_lang: Option<String>,
    /// §17.15.1.15 `w:characterSpacingControl` is `compressPunctuation` or
    /// `compressPunctuationAndJapaneseKana`: Word may squeeze full-width East
    /// Asian punctuation to keep one more character on a line.
    pub compress_punctuation: bool,
    /// `w:compat/w:compatSetting[@w:name="compatibilityMode"]`: 15 = Word 2013+
    /// layout, 14 = Word 2010, lower or 0 (absent) = older.
    pub compat_mode: u32,
    /// §17.15.1.57 `w:linkStyles` with no `w:attachedTemplate`: Word refreshes
    /// the styles from its own Normal.dotm when it opens the file. A named
    /// template lives on the author's machine and can't be loaded.
    pub styles_from_normal_template: bool,
    /// `w:compat/w:doNotExpandShiftReturn`: a justified line ending in a
    /// manual break keeps its natural width.
    pub do_not_expand_shift_return: bool,
    /// `w:compat/w:adjustLineHeightInTable`: table cell lines snap to the
    /// document grid too (§17.15.3.1).
    pub adjust_line_height_in_table: bool,
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            even_and_odd_headers: false,
            default_tab_stop: 36.0, // 0.5 inches = 720 twips = 36pt
            gutter_at_top: false,
            east_asia_lang: None,
            bidi_lang: None,
            default_lang: None,
            compress_punctuation: false,
            compat_mode: 0,
            styles_from_normal_template: false,
            do_not_expand_shift_return: false,
            adjust_line_height_in_table: false,
        }
    }
}

pub(super) fn parse_settings<R: Read + Seek>(zip: &mut zip::ZipArchive<R>) -> DocumentSettings {
    let Some(xml_text) = read_zip_text(zip, "word/settings.xml") else {
        return DocumentSettings::default();
    };
    let Ok(doc) = roxmltree::Document::parse(&xml_text) else {
        return DocumentSettings::default();
    };
    let root = doc.root_element();

    let default_tab_stop = wml(root, "defaultTabStop")
        .and_then(|n| twips_attr(n, "val"))
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
        gutter_at_top: wml_bool(root, "gutterAtTop").unwrap_or(false),
        east_asia_lang,
        bidi_lang,
        default_lang,
        compress_punctuation: wml_attr(root, "characterSpacingControl")
            .is_some_and(|v| v.starts_with("compressPunctuation")),
        compat_mode: wml(root, "compat")
            .into_iter()
            .flat_map(|c| c.children())
            .find(|n| n.attribute((WML_NS, "name")) == Some("compatibilityMode"))
            .and_then(|n| n.attribute((WML_NS, "val")))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        styles_from_normal_template: wml_bool(root, "linkStyles").unwrap_or(false)
            && wml(root, "attachedTemplate").is_none(),
        do_not_expand_shift_return: wml(root, "compat")
            .and_then(|c| wml_bool(c, "doNotExpandShiftReturn"))
            .unwrap_or(false),
        adjust_line_height_in_table: wml(root, "compat")
            .and_then(|c| wml_bool(c, "adjustLineHeightInTable"))
            .unwrap_or(false),
    }
}
