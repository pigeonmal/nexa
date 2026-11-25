use std::collections::BTreeSet;

use skera::{DEFAULT_DROP_TABLES, DEFAULT_LAYOUT_FEATURES, Plan, SubsetFlags, subset_font};
use write_fonts::{
    read::{FontRef, collections::IntSet},
    types::{NameId, Tag},
};

/// Subsets the native resource without changing its codepoints or name.
/// A font that cannot preserve its artwork or metrics is left intact.
pub fn subset_icon_font(bytes: &[u8], codepoints: &BTreeSet<u32>) -> Option<Vec<u8>> {
    let original = ttf_parser::Face::parse(bytes, 0).ok()?;
    if codepoints.is_empty() || !original.is_subsetting_allowed() {
        return None;
    }

    // The subsetter's default policy drops SVG and legacy bitmap tables.
    // Keep those fonts intact so their original native rendering survives.
    if [b"SVG ", b"EBDT", b"EBLC", b"sbix"].into_iter().any(|tag| {
        original
            .raw_face()
            .table(ttf_parser::Tag::from_bytes(tag))
            .is_some()
    }) {
        return None;
    }

    let font = FontRef::new(bytes).ok()?;
    let mut name_ids = IntSet::<NameId>::empty();
    name_ids.invert();
    let mut name_languages = IntSet::<u16>::empty();
    name_languages.invert();
    let mut layout_scripts = IntSet::<Tag>::empty();
    layout_scripts.invert();
    let plan = Plan::new(
        &IntSet::empty(),
        &codepoints.iter().copied().collect(),
        &font,
        SubsetFlags::default(),
        &DEFAULT_DROP_TABLES.iter().copied().collect(),
        &layout_scripts,
        &DEFAULT_LAYOUT_FEATURES.iter().copied().collect(),
        &name_ids,
        &name_languages,
    );
    let subset = subset_font(&font, &plan).ok()?;
    if subset.len() >= bytes.len() {
        return None;
    }
    let face = ttf_parser::Face::parse(&subset, 0).ok()?;
    let postscript_names = |font: &ttf_parser::Face<'_>| {
        font.names()
            .into_iter()
            .filter(|name| name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
            .filter_map(|name| name.to_string())
            .collect::<BTreeSet<_>>()
    };
    if postscript_names(&original) != postscript_names(&face) {
        return None;
    }
    for &codepoint in codepoints {
        let character = char::from_u32(codepoint)?;
        let old = original.glyph_index(character)?;
        let new = face.glyph_index(character)?;
        if original.glyph_hor_advance(old) != face.glyph_hor_advance(new)
            || original.glyph_bounding_box(old) != face.glyph_bounding_box(new)
            || original.is_color_glyph(old) != face.is_color_glyph(new)
        {
            return None;
        }
    }
    Some(subset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsets_a_prebuilt_font_without_renaming_its_face() {
        let input = include_bytes!("../../../examples/component-parity/assets/fonts/mazicon.ttf");
        let bytes = subset_icon_font(input, &BTreeSet::from([0xF100])).expect("subset Mazicon");
        assert!(bytes.len() < input.len() / 4);
        let face = ttf_parser::Face::parse(&bytes, 0).expect("parse subset");
        assert!(face.glyph_index('\u{F100}').is_some());
        assert!(face.names().into_iter().any(|name| {
            name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME
                && name.to_string().as_deref() == Some("Mazicon")
        }));
    }
}
