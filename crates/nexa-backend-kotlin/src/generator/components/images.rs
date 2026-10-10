use nexa_codegen::SourceWriter;
use nexa_ir::{ImageScale, ImageSource};

use crate::generator::{
    engine::{features::Features, imports::ImportSet},
    expressions::expression,
    utils::{indent, kotlin_string},
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let uses_images =
        features.uses_asset || features.uses_placeholder || features.uses_remote_image;
    imports.add(uses_images, "androidx.compose.ui.Modifier");
    imports.add(uses_images, "androidx.compose.ui.unit.dp");
    imports.add(features.uses_remote_image, "coil3.ImageLoader");
    imports.add(features.uses_remote_image, "coil3.network.NetworkClient");
    imports.add(features.uses_remote_image, "coil3.network.NetworkFetcher");
    imports.add(features.uses_remote_image, "coil3.network.NetworkHeaders");
    imports.add(features.uses_remote_image, "coil3.network.NetworkRequest");
    imports.add(features.uses_remote_image, "coil3.network.NetworkResponse");
    imports.add(
        features.uses_remote_image,
        "coil3.network.NetworkResponseBody",
    );
    imports.add(features.uses_remote_image, "okio.Buffer");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_image(
    source: &ImageSource,
    description: &str,
    scale: ImageScale,
    placeholder: Option<&str>,
    max_height: Option<f32>,
    shared_element: Option<&nexa_ir::Expr>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    let content_scale = match scale {
        ImageScale::Fit => "androidx.compose.ui.layout.ContentScale.Fit",
        ImageScale::Fill => "androidx.compose.ui.layout.ContentScale.Crop",
    };
    let content_description = if description.is_empty() {
        "null".to_owned()
    } else {
        kotlin_string(description)
    };
    let modifier = shared_element.map_or_else(
        || "Modifier".to_owned(),
        |id| format!("nexaSharedElementModifier({})", expression(id)),
    );
    let max_height = max_height.map_or_else(
        || "null".to_owned(),
        |height| format!("{}.dp", crate::generator::engine::utils::number(height)),
    );
    let indent = "    ".repeat(depth);
    match source {
        ImageSource::Asset(asset) => out.push_str(&format!(
            "NexaAssetImagePrimitive(\n{indent}    name = {},\n{indent}    contentDescription = {content_description},\n{indent}    contentScale = {content_scale},\n{indent}    maxHeight = {max_height},\n{indent}    modifier = {modifier},\n{indent})",
            kotlin_string(asset),
        )),
        ImageSource::RemoteUrl(url) => out.push_str(&format!(
            "NexaRemoteImagePrimitive(\n{indent}    url = {},\n{indent}    allowsFile = false,\n{indent}    placeholder = {},\n{indent}    contentDescription = {content_description},\n{indent}    contentScale = {content_scale},\n{indent}    maxHeight = {max_height},\n{indent}    modifier = {modifier},\n{indent})",
            expression(url),
            placeholder.map(kotlin_string).unwrap_or_else(|| "null".to_owned()),
        )),
        ImageSource::LocalFile(file) => out.push_str(&format!(
            "NexaRemoteImagePrimitive(\n{indent}    url = {},\n{indent}    allowsFile = true,\n{indent}    placeholder = null,\n{indent}    contentDescription = {content_description},\n{indent}    contentScale = {content_scale},\n{indent}    maxHeight = {max_height},\n{indent}    modifier = {modifier},\n{indent})",
            expression(file),
        )),
    }
}

pub(crate) fn render_primitives(include_asset: bool, include_remote: bool, out: &mut SourceWriter) {
    if include_asset {
        out.push_str(
            r#"@Composable
internal fun NexaAssetImagePrimitive(
    name: String,
    contentDescription: String?,
    contentScale: androidx.compose.ui.layout.ContentScale,
    maxHeight: androidx.compose.ui.unit.Dp?,
    modifier: androidx.compose.ui.Modifier,
) {
    val imageModifier = modifier.then(
        maxHeight?.let { androidx.compose.ui.Modifier.heightIn(max = it) }
            ?: androidx.compose.ui.Modifier
    )
    androidx.compose.foundation.Image(
        painter = nexaDrawablePainter(name),
        contentDescription = contentDescription,
        contentScale = contentScale,
        modifier = imageModifier,
    )
}

"#,
        );
    }
    if include_remote {
        out.push_str(
            r#"@Composable
internal fun NexaRemoteImagePrimitive(
    url: String,
    allowsFile: Boolean,
    placeholder: String?,
    contentDescription: String?,
    contentScale: androidx.compose.ui.layout.ContentScale,
    maxHeight: androidx.compose.ui.unit.Dp?,
    modifier: androidx.compose.ui.Modifier,
) {
    val safeModel = if (allowsFile) {
        url.takeIf { it.startsWith("file://") }
    } else {
        url.takeIf { it.startsWith("https://") }
    }
    val placeholderPainter = if (placeholder == null) null else nexaDrawablePainter(placeholder)
    val imageModifier = modifier.then(
        maxHeight?.let { androidx.compose.ui.Modifier.heightIn(max = it) }
            ?: androidx.compose.ui.Modifier
    )
    coil3.compose.AsyncImage(
        model = safeModel,
        imageLoader = nexaImageLoader(),
        contentDescription = contentDescription,
        contentScale = contentScale,
        placeholder = placeholderPainter,
        error = placeholderPainter,
        modifier = imageModifier,
    )
}

"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;

    use super::render_primitives;

    #[test]
    fn asset_image_primitive_owns_resource_fallback_sizing_and_accessibility() {
        let mut output = SourceWriter::new();
        render_primitives(true, false, &mut output);

        assert!(output.contains("internal fun NexaAssetImagePrimitive("));
        assert!(output.contains("painter = nexaDrawablePainter(name)"));
        assert!(output.contains("Modifier.heightIn(max = it)"));
        assert!(output.contains("contentDescription = contentDescription"));
        assert!(!output.contains("NexaRemoteImagePrimitive("));
    }

    #[test]
    fn remote_image_primitive_owns_scheme_validation_coil_and_placeholder() {
        let mut output = SourceWriter::new();
        render_primitives(false, true, &mut output);

        assert!(output.contains("url.takeIf { it.startsWith(\"https://\") }"));
        assert!(output.contains("url.takeIf { it.startsWith(\"file://\") }"));
        assert!(output.contains("imageLoader = nexaImageLoader()"));
        assert!(output.contains("placeholder = placeholderPainter"));
        assert!(output.contains("error = placeholderPainter"));
        assert!(!output.contains("NexaAssetImagePrimitive("));
    }
}
