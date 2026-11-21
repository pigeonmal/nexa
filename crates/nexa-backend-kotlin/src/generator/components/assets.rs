use std::collections::BTreeSet;

use crate::generator::{
    engine::{features::Features, imports::ImportSet},
    utils::kotlin_string,
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let uses_assets =
        features.uses_asset || features.uses_placeholder || features.uses_remote_image;
    imports.add(uses_assets, "androidx.compose.runtime.remember");
    imports.add(uses_assets, "androidx.compose.ui.graphics.Color");
    imports.add(
        uses_assets,
        "androidx.compose.ui.graphics.painter.ColorPainter",
    );
    imports.add(uses_assets, "androidx.compose.ui.graphics.painter.Painter");
    imports.add(uses_assets, "androidx.compose.ui.res.painterResource");
    imports.add(
        features.uses_remote_image,
        "androidx.compose.ui.platform.LocalContext",
    );
}

pub(crate) fn drawable_id(name: &str, known_drawables: &BTreeSet<String>) -> String {
    if known_drawables.contains(name) {
        format!("R.drawable.`{name}`")
    } else {
        format!("nexaDrawableId({})", kotlin_string(name))
    }
}

pub(crate) const DRAWABLE_PRIMITIVES: &str = r#"
// nexa-codegen: nexaDrawableId
@Composable
internal fun nexaDrawableId(name: String): Int {
    val context = androidx.compose.ui.platform.LocalContext.current
    return remember(context, name) {
        context.resources.getIdentifier(
            name,
            "drawable",
            context.packageName,
        )
    }
}

// nexa-codegen: nexaDrawablePainter
@Composable
internal fun nexaDrawablePainter(resourceId: Int): Painter {
    return if (resourceId == 0) {
        remember { ColorPainter(Color.Transparent) }
    } else {
        painterResource(resourceId)
    }
}

"#;
