use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    utils::{indent, kotlin_string},
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let icons = &features.facts.ui.system_icons;
    if icons.is_empty() {
        return;
    }

    imports.add(true, "androidx.compose.material.icons.Icons");
    imports.add(true, "androidx.compose.material3.Icon");
    imports.add(true, "androidx.compose.foundation.layout.size");
    imports.add(true, "androidx.compose.ui.Modifier");
    imports.add(true, "androidx.compose.ui.unit.dp");
    for icon in icons {
        imports.add(true, icon_import(*icon));
    }
}

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
        "Icon(imageVector = {}, contentDescription = {}, modifier = Modifier.size({}.dp), tint = {})",
        icon_ref(icon),
        kotlin_string(description),
        crate::generator::engine::utils::number(size),
        colors::expression(tint),
    ));
}

fn icon_import(icon: SystemIcon) -> &'static str {
    match icon {
        SystemIcon::Heart => "androidx.compose.material.icons.outlined.FavoriteBorder",
        SystemIcon::Comment => "androidx.compose.material.icons.outlined.ChatBubbleOutline",
        SystemIcon::Bookmark => "androidx.compose.material.icons.outlined.BookmarkBorder",
        SystemIcon::Home => "androidx.compose.material.icons.filled.Home",
        SystemIcon::Search => "androidx.compose.material.icons.filled.Search",
        SystemIcon::Inbox => "androidx.compose.material.icons.filled.Inbox",
        SystemIcon::Profile => "androidx.compose.material.icons.filled.Person",
        SystemIcon::HeartFilled => "androidx.compose.material.icons.filled.Favorite",
        SystemIcon::CommentFilled => "androidx.compose.material.icons.filled.ChatBubble",
        SystemIcon::BookmarkFilled => "androidx.compose.material.icons.filled.Bookmark",
        SystemIcon::Share => "androidx.compose.material.icons.filled.Share",
        SystemIcon::Music => "androidx.compose.material.icons.filled.MusicNote",
        SystemIcon::Back => "androidx.compose.material.icons.automirrored.filled.ArrowBack",
        SystemIcon::Screen => "androidx.compose.material.icons.filled.Tv",
        SystemIcon::Layers => "androidx.compose.material.icons.filled.Layers",
        SystemIcon::Plus => "androidx.compose.material.icons.filled.Add",
        SystemIcon::Close => "androidx.compose.material.icons.filled.Close",
        SystemIcon::Checkmark => "androidx.compose.material.icons.filled.Check",
        SystemIcon::Send => "androidx.compose.material.icons.filled.Send",
        SystemIcon::Volume => "androidx.compose.material.icons.filled.VolumeUp",
        SystemIcon::VolumeMuted => "androidx.compose.material.icons.automirrored.filled.VolumeOff",
    }
}

fn icon_ref(icon: SystemIcon) -> &'static str {
    match icon {
        SystemIcon::Home => "Icons.Filled.Home",
        SystemIcon::Search => "Icons.Filled.Search",
        SystemIcon::Inbox => "Icons.Filled.Inbox",
        SystemIcon::Profile => "Icons.Filled.Person",
        SystemIcon::Heart => "Icons.Outlined.FavoriteBorder",
        SystemIcon::HeartFilled => "Icons.Filled.Favorite",
        SystemIcon::Comment => "Icons.Outlined.ChatBubbleOutline",
        SystemIcon::CommentFilled => "Icons.Filled.ChatBubble",
        SystemIcon::Bookmark => "Icons.Outlined.BookmarkBorder",
        SystemIcon::BookmarkFilled => "Icons.Filled.Bookmark",
        SystemIcon::Share => "Icons.Filled.Share",
        SystemIcon::Music => "Icons.Filled.MusicNote",
        SystemIcon::Back => "Icons.AutoMirrored.Filled.ArrowBack",
        SystemIcon::Screen => "Icons.Filled.Tv",
        SystemIcon::Layers => "Icons.Filled.Layers",
        SystemIcon::Plus => "Icons.Filled.Add",
        SystemIcon::Close => "Icons.Filled.Close",
        SystemIcon::Checkmark => "Icons.Filled.Check",
        SystemIcon::Send => "Icons.Filled.Send",
        SystemIcon::Volume => "Icons.Filled.VolumeUp",
        SystemIcon::VolumeMuted => "Icons.AutoMirrored.Filled.VolumeOff",
    }
}
