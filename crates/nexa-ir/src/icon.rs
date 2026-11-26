//! Typed icon values shared by built-in component icon slots.

use crate::{ColorExpression, SystemIcon};
use serde::{Deserialize, Serialize};

/// One monochrome glyph layer from an SVG custom icon, with the source fill
/// kept separately so native text can preserve it or apply a uniform tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconGlyphLayer {
    pub glyph: u32,
    pub rgba: [u8; 4],
    #[serde(default)]
    pub tintable: bool,
}

/// The compile-time source for an icon value. Platform icons and project
/// glyphs remain distinct so feature analysis never mistakes a custom font
/// glyph for a system icon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IconSource {
    System(SystemIcon),
    FontGlyph {
        font_family: String,
        font_resource: String,
        glyph: u32,
        #[serde(default)]
        layers: Vec<IconGlyphLayer>,
    },
}

/// A value accepted by a component's `icon:` slot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IconSpec {
    pub source: IconSource,
    /// Optional local accessibility override; component context supplies the
    /// default when this is omitted.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional size in points/dp; component context supplies the default.
    #[serde(default)]
    pub size: Option<f32>,
    /// Explicit tint. When absent, system icons inherit native component
    /// tint and custom fonts retain their original glyph paint.
    #[serde(default)]
    pub color: Option<ColorExpression>,
    /// Per-layer color overrides for multilayer SVG icons (`colors: [...]`).
    #[serde(default)]
    pub layer_colors: Vec<ColorExpression>,
}

impl IconSpec {
    pub fn system(icon: SystemIcon) -> Self {
        Self {
            source: IconSource::System(icon),
            description: None,
            size: None,
            color: None,
            layer_colors: Vec::new(),
        }
    }

    pub fn font_glyph(
        font_family: impl Into<String>,
        font_resource: impl Into<String>,
        glyph: u32,
    ) -> Self {
        Self {
            source: IconSource::FontGlyph {
                font_family: font_family.into(),
                font_resource: font_resource.into(),
                glyph,
                layers: Vec::new(),
            },
            description: None,
            size: None,
            color: None,
            layer_colors: Vec::new(),
        }
    }
}
