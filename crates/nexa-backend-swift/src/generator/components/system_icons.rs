use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    utils::{indent, number, swift_string},
};

pub(crate) fn render(
    icon: SystemIcon,
    description: &str,
    size: f32,
    tint: nexa_ir::ColorValue,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str(&format!(
        "Image(systemName: {})\n{}.font(.system(size: {}))\n{}.foregroundStyle({})",
        swift_string(sf_symbol(icon)),
        "    ".repeat(depth + 1),
        number(size),
        "    ".repeat(depth + 1),
        colors::expression(tint),
    ));
    out.push_str(&format!(
        "\n{}.accessibilityLabel({})",
        "    ".repeat(depth + 1),
        swift_string(description)
    ));
}

fn sf_symbol(icon: SystemIcon) -> &'static str {
    match icon {
        SystemIcon::Home => "house.fill",
        SystemIcon::Search => "magnifyingglass",
        SystemIcon::Inbox => "tray",
        SystemIcon::Profile => "person",
        SystemIcon::Heart => "heart",
        SystemIcon::HeartFilled => "heart.fill",
        SystemIcon::Comment => "bubble.right",
        SystemIcon::CommentFilled => "bubble.right.fill",
        SystemIcon::Bookmark => "bookmark",
        SystemIcon::BookmarkFilled => "bookmark.fill",
        SystemIcon::Share => "arrowshape.turn.up.right",
        SystemIcon::Music => "music.note",
        SystemIcon::Back => "chevron.left",
        SystemIcon::Screen => "tv",
        SystemIcon::Layers => "rectangle.on.rectangle",
        SystemIcon::Plus => "plus",
        SystemIcon::Close => "xmark",
        SystemIcon::Checkmark => "checkmark",
        SystemIcon::Send => "paperplane",
        SystemIcon::Volume => "speaker.wave.2.fill",
        SystemIcon::VolumeMuted => "speaker.slash.fill",
    }
}
