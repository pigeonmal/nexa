//! Shared visual defaults used by native backends to keep generated apps aligned.

use nexa_ir::{ButtonShape, FontWeight, TextFontStyle};

pub const DEFAULT_ACCENT_ARGB: u32 = 0xFF00_7AFF;
pub const DARK_ACCENT_ARGB: u32 = 0xFF0A_84FF;
/// Dynamic iOS system blue matches the shared light and dark accent tokens.
pub const SWIFT_DEFAULT_ACCENT_COLOR: &str = "Color(uiColor: .systemBlue)";
pub const LIGHT_BACKGROUND_ARGB: u32 = 0xFFFF_FFFF;
pub const DARK_BACKGROUND_ARGB: u32 = 0xFF00_0000;
pub const LIGHT_SURFACE_ARGB: u32 = 0xFFFF_FFFF;
pub const DARK_SURFACE_ARGB: u32 = 0xFF00_0000;
pub const LIGHT_SURFACE_VARIANT_ARGB: u32 = 0xFFF2_F2F7;
pub const DARK_SURFACE_VARIANT_ARGB: u32 = 0xFF2C_2C2E;
pub const LIGHT_ON_SURFACE_ARGB: u32 = 0xFF00_0000;
pub const DARK_ON_SURFACE_ARGB: u32 = 0xFFFF_FFFF;
pub const LIGHT_PRIMARY_CONTAINER_ARGB: u32 = 0xFFD6_E8FF;
pub const DARK_PRIMARY_CONTAINER_ARGB: u32 = 0xFF00_3C7A;
pub const LIGHT_ON_PRIMARY_CONTAINER_ARGB: u32 = 0xFF00_1D36;
pub const DARK_ON_PRIMARY_CONTAINER_ARGB: u32 = 0xFFD6_E8FF;
pub const LIGHT_OUTLINE_ARGB: u32 = 0xFFC6_C6C8;
pub const DARK_OUTLINE_ARGB: u32 = 0xFF54_5458;
pub const MUTED_TEXT_ARGB: u32 = 0xFF8E_8E93;
pub const DEFAULT_ERROR_ARGB: u32 = 0xFFFF_3B30;
pub const DEFAULT_BODY_FONT_SIZE: u8 = 17;
/// Default Compose system icon canvas matched to the shared 24dp control size.
pub const DEFAULT_MATERIAL_ICON_SIZE: u8 = 24;
/// Material paths need a modest size adjustment to match SF Symbol metrics.
pub const MATERIAL_ICON_SIZE_SCALE: f32 = 1.2;
pub const DEFAULT_LINE_HEIGHT_MULTIPLIER: f32 = 1.2;
pub const BUTTON_MIN_TAP_TARGET: u8 = 48;
pub const BUTTON_MIN_WIDTH: u8 = 64;
pub const DEFAULT_BUTTON_SHAPE: ButtonShape = ButtonShape::Capsule;
pub const BUTTON_LARGE_MIN_HEIGHT: u8 = 50;
pub const BUTTON_SMALL_HORIZONTAL_PADDING: u8 = 12;
pub const BUTTON_SMALL_VERTICAL_PADDING: u8 = 4;
pub const BUTTON_LARGE_HORIZONTAL_PADDING: u8 = 20;
pub const BUTTON_LARGE_VERTICAL_PADDING: u8 = 12;
/// Grouped Form geometry shared by the SwiftUI and Compose renderers.
pub const FORM_SECTION_HORIZONTAL_INSET: u8 = 16;
pub const FORM_SECTION_TOP_PADDING: u8 = 8;
pub const FORM_SECTION_BOTTOM_PADDING: u8 = 20;
pub const FORM_ROW_HORIZONTAL_INSET: u8 = 16;
/// Shared baseline row size; content and explicit padding may expand it.
pub const FORM_ROW_MIN_HEIGHT: u8 = 56;
pub const FORM_DIVIDER_START_INSET: u8 = 56;
pub const FORM_DIVIDER_END_INSET: u8 = 16;
pub const FORM_FOOTER_START_INSET: u8 = 16;
pub const FORM_FOOTER_FONT_SIZE: u8 = 13;
pub const FORM_LARGE_TITLE_TOP_PADDING: u8 = 20;
pub const FORM_LARGE_TITLE_FONT_SIZE: u8 = 34;
/// Grouped Form section colors matching the native iOS system palette.
pub const FORM_LIGHT_SECTION_ARGB: u32 = 0xFFFF_FFFF;
pub const FORM_DARK_SECTION_ARGB: u32 = 0xFF1C_1C1E;
pub const FORM_LIGHT_SEPARATOR_ARGB: u32 = 0x493C_3C43;
pub const FORM_DARK_SEPARATOR_ARGB: u32 = 0xFF38_383A;
pub const FORM_SWITCH_OFF_TRACK_ARGB: u32 = 0xFF5E_5E62;
/// Shared gap between a system icon and its adjacent label.
pub const ICON_LABEL_SPACING: u8 = 8;
pub const PAGE_INDICATOR_SELECTED_SIZE: u8 = 8;
pub const PAGE_INDICATOR_UNSELECTED_SIZE: u8 = 6;
pub const PAGE_INDICATOR_SPACING: u8 = 8;
pub const PAGE_INDICATOR_BOTTOM_INSET: u8 = 24;
pub const PAGE_INDICATOR_INACTIVE_OPACITY: f32 = 0.45;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextMetrics {
    pub size: u8,
    pub weight: FontWeight,
}

/// Returns the default iOS system-text metrics in platform-independent units.
pub const fn text_metrics(style: TextFontStyle) -> TextMetrics {
    let (size, weight) = match style {
        TextFontStyle::LargeTitle => (34, FontWeight::Normal),
        TextFontStyle::Title => (28, FontWeight::Normal),
        TextFontStyle::Title2 => (22, FontWeight::Normal),
        TextFontStyle::Title3 => (20, FontWeight::Normal),
        TextFontStyle::Headline => (17, FontWeight::Semibold),
        TextFontStyle::Subheadline => (15, FontWeight::Normal),
        TextFontStyle::Body => (17, FontWeight::Normal),
        TextFontStyle::Callout => (16, FontWeight::Normal),
        TextFontStyle::Footnote => (13, FontWeight::Normal),
        TextFontStyle::Caption => (12, FontWeight::Normal),
        TextFontStyle::Caption2 => (11, FontWeight::Normal),
    };
    TextMetrics { size, weight }
}

/// Emits the Nexa baseline Material 3 palette, chosen to match the default iOS system palette.
pub fn kotlin_color_scheme(dark: bool) -> String {
    let (
        name,
        accent,
        background,
        surface,
        surface_variant,
        on_surface,
        primary_container,
        on_primary_container,
        outline,
    ) = if dark {
        (
            "darkColorScheme",
            DARK_ACCENT_ARGB,
            DARK_BACKGROUND_ARGB,
            DARK_SURFACE_ARGB,
            DARK_SURFACE_VARIANT_ARGB,
            DARK_ON_SURFACE_ARGB,
            DARK_PRIMARY_CONTAINER_ARGB,
            DARK_ON_PRIMARY_CONTAINER_ARGB,
            DARK_OUTLINE_ARGB,
        )
    } else {
        (
            "lightColorScheme",
            DEFAULT_ACCENT_ARGB,
            LIGHT_BACKGROUND_ARGB,
            LIGHT_SURFACE_ARGB,
            LIGHT_SURFACE_VARIANT_ARGB,
            LIGHT_ON_SURFACE_ARGB,
            LIGHT_PRIMARY_CONTAINER_ARGB,
            LIGHT_ON_PRIMARY_CONTAINER_ARGB,
            LIGHT_OUTLINE_ARGB,
        )
    };
    format!(
        "{name}(primary = Color(0x{accent:08X}), onPrimary = Color.White, primaryContainer = Color(0x{primary_container:08X}), onPrimaryContainer = Color(0x{on_primary_container:08X}), secondary = Color(0x{muted:08X}), onSecondary = Color.White, background = Color(0x{background:08X}), onBackground = Color(0x{on_surface:08X}), surface = Color(0x{surface:08X}), onSurface = Color(0x{on_surface:08X}), surfaceVariant = Color(0x{surface_variant:08X}), onSurfaceVariant = Color(0x{muted:08X}), outline = Color(0x{outline:08X}), error = Color(0x{error:08X}), onError = Color.White)",
        muted = MUTED_TEXT_ARGB,
        error = DEFAULT_ERROR_ARGB,
    )
}
