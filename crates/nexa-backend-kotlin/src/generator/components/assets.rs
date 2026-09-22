/// Emits the small, feature-gated drawable resolver used by generated images.
///
/// Asset names stay source-level strings so projects can provide platform
/// resources without changing generated Kotlin. Resolution is remembered per
/// composition and falls back to a transparent painter when an optional asset
/// is not present, keeping generated projects buildable before assets are added.
use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let uses_assets = features.uses_asset
        || features.uses_tab_icon
        || features.uses_button_icon
        || features.uses_placeholder;
    imports.add(uses_assets, "androidx.compose.runtime.remember");
    imports.add(uses_assets, "androidx.compose.ui.graphics.Color");
    imports.add(
        uses_assets,
        "androidx.compose.ui.graphics.painter.ColorPainter",
    );
    imports.add(uses_assets, "androidx.compose.ui.graphics.painter.Painter");
    imports.add(uses_assets, "androidx.compose.ui.res.painterResource");
    imports.add(
        uses_assets || features.uses_remote_image,
        "androidx.compose.ui.platform.LocalContext",
    );
}

pub(crate) fn render(out: &mut String) {
    out.push_str(
        r#"
@Composable
private fun nexaDrawablePainter(name: String): Painter {
    val context = LocalContext.current
    val resourceId = remember(context, name) {
        context.resources.getIdentifier(
            name,
            "drawable",
            context.packageName,
        )
    }
    return if (resourceId == 0) {
        ColorPainter(Color.Transparent)
    } else {
        painterResource(resourceId)
    }
}

"#,
    );
}
