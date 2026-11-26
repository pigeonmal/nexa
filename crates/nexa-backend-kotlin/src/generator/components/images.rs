use std::collections::BTreeSet;

use nexa_codegen::SourceWriter;
use nexa_ir::{Dimension, ImageScale, ImageSource, ViewStyle};

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
    imports.add(
        uses_images,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        uses_images,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    let uses_coil = features.uses_remote_image || features.uses_gif_asset;
    imports.add(uses_coil, "coil3.ImageLoader");
    imports.add(uses_coil, "coil3.gif.AnimatedImageDecoder");
    imports.add(uses_coil, "coil3.gif.GifDecoder");
    imports.add(uses_coil, "coil3.gif.repeatCount");
    imports.add(uses_coil, "coil3.memory.MemoryCache");
    imports.add(uses_coil, "coil3.request.ImageRequest");
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

fn fixed_dp_pair(style: &ViewStyle) -> (Option<f32>, Option<f32>) {
    let w = match style.width {
        Some(Dimension::Fixed(v)) => Some(v),
        _ => None,
    };
    let h = match style.height {
        Some(Dimension::Fixed(v)) => Some(v),
        _ => None,
    };
    (w, h)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_image(
    source: &ImageSource,
    description: &str,
    fit: ImageScale,
    placeholder: Option<&str>,
    show_loading_indicator: bool,
    auto_replay: bool,
    style: &ViewStyle,
    shared_element: Option<&nexa_ir::Expr>,
    known_drawables: &BTreeSet<String>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    let content_scale = match fit {
        ImageScale::Fit => "androidx.compose.ui.layout.ContentScale.Fit",
        ImageScale::Fill => "androidx.compose.ui.layout.ContentScale.Crop",
    };
    let content_description = if description.is_empty() {
        "null".to_owned()
    } else {
        kotlin_string(description)
    };
    let indent_str = "    ".repeat(depth);
    let mod_indent = "    ".repeat(depth + 2);
    let mut modifier_writer = SourceWriter::new();
    match shared_element {
        Some(id) => modifier_writer.push_str(&format!("nexaSharedElementModifier({})", expression(id))),
        None => modifier_writer.push_str("Modifier"),
    }
    if fit == ImageScale::Fill && style.width.is_none() && style.max_width.is_none() {
        modifier_writer.push_str(&format!("\n{mod_indent}.fillMaxWidth()"));
    }
    if style.has_modifiers() {
        super::layout::render_modifiers(style, depth + 2, &mut modifier_writer);
    }
    let modifier = modifier_writer.finish();
    let (decode_w, decode_h) = fixed_dp_pair(style);
    let decode_w_arg = decode_w
        .map(|w| format!("{}f", crate::generator::engine::utils::number(w)))
        .unwrap_or_else(|| "null".to_owned());
    let decode_h_arg = decode_h
        .map(|h| format!("{}f", crate::generator::engine::utils::number(h)))
        .unwrap_or_else(|| "null".to_owned());

    match source {
        ImageSource::Asset(asset) if super::assets::is_gif_asset(asset) => out.push_str(&format!(
            "NexaGifAssetImagePrimitive(\n{indent_str}    resourceId = {},\n{indent_str}    contentDescription = {content_description},\n{indent_str}    contentScale = {content_scale},\n{indent_str}    autoReplay = {auto_replay},\n{indent_str}    decodeWidthDp = {decode_w_arg},\n{indent_str}    decodeHeightDp = {decode_h_arg},\n{indent_str}    modifier = {modifier},\n{indent_str})",
            super::assets::raw_id(asset),
        )),
        ImageSource::Asset(asset) => out.push_str(&format!(
            "NexaAssetImagePrimitive(\n{indent_str}    resourceId = {},\n{indent_str}    contentDescription = {content_description},\n{indent_str}    contentScale = {content_scale},\n{indent_str}    modifier = {modifier},\n{indent_str})",
            super::assets::drawable_id(asset, known_drawables),
        )),
        ImageSource::RemoteUrl(url) | ImageSource::LocalFile(url) => {
            let allows_file = matches!(source, ImageSource::LocalFile(_));
            let placeholder = if allows_file { None } else { placeholder };
            let placeholder_id = placeholder.map_or_else(
                || "null".to_owned(),
                |name| super::assets::drawable_id(name, known_drawables),
            );
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(\n{indent_str}    url = {},\n{indent_str}    allowsFile = {allows_file},\n{indent_str}    placeholderResourceId = {placeholder_id},\n{indent_str}    showLoadingIndicator = {show_loading_indicator},\n{indent_str}    autoReplay = {auto_replay},\n{indent_str}    decodeWidthDp = {decode_w_arg},\n{indent_str}    decodeHeightDp = {decode_h_arg},\n{indent_str}    contentDescription = {content_description},\n{indent_str}    contentScale = {content_scale},\n{indent_str}    modifier = {modifier},\n{indent_str})",
                expression(url),
            ));
        }
    }
}

pub(crate) fn render_primitives(
    include_asset: bool,
    include_gif_asset: bool,
    include_remote: bool,
    out: &mut SourceWriter,
) {
    if include_asset {
        out.push_str(
            r#"@Composable
internal fun NexaAssetImagePrimitive(
    resourceId: Int,
    contentDescription: String?,
    contentScale: androidx.compose.ui.layout.ContentScale,
    modifier: androidx.compose.ui.Modifier,
) {
    androidx.compose.foundation.Image(
        painter = nexaDrawablePainter(resourceId),
        contentDescription = contentDescription,
        contentScale = contentScale,
        modifier = modifier,
    )
}

"#,
        );
    }
    if include_gif_asset {
        out.push_str(
            r#"@Composable
internal fun NexaGifAssetImagePrimitive(
    resourceId: Int,
    contentDescription: String?,
    contentScale: androidx.compose.ui.layout.ContentScale,
    autoReplay: Boolean,
    decodeWidthDp: Float? = null,
    decodeHeightDp: Float? = null,
    modifier: androidx.compose.ui.Modifier,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val density = androidx.compose.ui.platform.LocalDensity.current
    val imageRequest = remember(context, density, resourceId, autoReplay, decodeWidthDp, decodeHeightDp) {
        val builder = ImageRequest.Builder(context)
            .data(resourceId)
            .repeatCount(if (autoReplay) -1 else 0)
        if (decodeWidthDp != null && decodeHeightDp != null) {
            with(density) {
                builder.size(decodeWidthDp.dp.roundToPx(), decodeHeightDp.dp.roundToPx())
            }
        }
        builder.build()
    }
    coil3.compose.AsyncImage(
        model = imageRequest,
        imageLoader = nexaImageLoader(),
        contentDescription = contentDescription,
        contentScale = contentScale,
        modifier = modifier,
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
    placeholderResourceId: Int?,
    showLoadingIndicator: Boolean,
    autoReplay: Boolean = true,
    decodeWidthDp: Float? = null,
    decodeHeightDp: Float? = null,
    contentDescription: String?,
    contentScale: androidx.compose.ui.layout.ContentScale,
    modifier: androidx.compose.ui.Modifier,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val density = androidx.compose.ui.platform.LocalDensity.current
    val safeModel = if (allowsFile) {
        url.takeIf { it.startsWith("file://", ignoreCase = true) }
    } else {
        url.takeIf { it.startsWith("https://", ignoreCase = true) }
    }
    val placeholderPainter = placeholderResourceId?.let { nexaDrawablePainter(it) }
    val imageRequest = remember(context, density, safeModel, autoReplay, decodeWidthDp, decodeHeightDp) {
        safeModel?.let { data ->
            val builder = ImageRequest.Builder(context)
                .data(data)
                .repeatCount(if (autoReplay) -1 else 0)
            if (decodeWidthDp != null && decodeHeightDp != null) {
                with(density) {
                    builder.size(decodeWidthDp.dp.roundToPx(), decodeHeightDp.dp.roundToPx())
                }
            }
            builder.build()
        }
    }
    if (showLoadingIndicator && placeholderPainter == null) {
        val imageLoading = remember(safeModel) { androidx.compose.runtime.mutableStateOf(safeModel != null) }
        val onImageLoading: (coil3.compose.AsyncImagePainter.State.Loading) -> Unit =
            remember(imageLoading) { { imageLoading.value = true } }
        val onImageSuccess: (coil3.compose.AsyncImagePainter.State.Success) -> Unit =
            remember(imageLoading) { { imageLoading.value = false } }
        val onImageError: (coil3.compose.AsyncImagePainter.State.Error) -> Unit =
            remember(imageLoading) { { imageLoading.value = false } }
        androidx.compose.foundation.layout.Box(
            modifier = modifier,
            contentAlignment = androidx.compose.ui.Alignment.Center,
        ) {
            coil3.compose.AsyncImage(
                model = imageRequest,
                imageLoader = nexaImageLoader(),
                contentDescription = contentDescription,
                contentScale = contentScale,
                placeholder = null,
                error = null,
                onLoading = onImageLoading,
                onSuccess = onImageSuccess,
                onError = onImageError,
                modifier = Modifier.matchParentSize(),
            )
            if (imageLoading.value) {
                androidx.compose.material3.CircularProgressIndicator(
                    modifier = Modifier.size(20.dp),
                    strokeWidth = 2.dp,
                )
            }
        }
    } else {
        coil3.compose.AsyncImage(
            model = imageRequest,
            imageLoader = nexaImageLoader(),
            contentDescription = contentDescription,
            contentScale = contentScale,
            placeholder = placeholderPainter,
            error = placeholderPainter,
            modifier = modifier,
        )
    }
}

"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nexa_codegen::SourceWriter;
    use nexa_ir::{Dimension, ImageScale, ImageSource, ViewStyle};

    use super::render_primitives;

    #[test]
    fn asset_image_primitive_owns_resource_fallback_sizing_and_accessibility() {
        let mut output = SourceWriter::new();
        render_primitives(true, false, false, &mut output);

        assert!(output.contains("internal fun NexaAssetImagePrimitive("));
        assert!(output.contains("painter = nexaDrawablePainter(resourceId)"));
        assert!(output.contains("contentDescription = contentDescription"));
        assert!(!output.contains("NexaRemoteImagePrimitive("));
    }

    #[test]
    fn remote_image_primitive_owns_scheme_validation_coil_and_state_fallbacks() {
        let mut output = SourceWriter::new();
        render_primitives(false, false, true, &mut output);

        assert!(output.contains("url.takeIf { it.startsWith(\"https://\", ignoreCase = true) }"));
        assert!(output.contains("url.takeIf { it.startsWith(\"file://\", ignoreCase = true) }"));
        assert!(output.contains("imageLoader = nexaImageLoader()"));
        assert!(output.contains("placeholder = placeholderPainter"));
        assert!(output.contains("error = placeholderPainter"));
        assert!(output.contains("CircularProgressIndicator("));
        assert!(output.contains("if (showLoadingIndicator && placeholderPainter == null)"));
        assert!(output.contains("repeatCount(if (autoReplay) -1 else 0)"));
        assert!(!output.contains("NexaAssetImagePrimitive("));
    }

    #[test]
    fn remote_images_forward_the_explicit_loading_indicator_option() {
        let source = ImageSource::RemoteUrl(nexa_ir::Expr::String(
            "https://example.test/image.png".to_owned(),
        ));
        let mut output = SourceWriter::new();
        let style = ViewStyle {
            width: Some(Dimension::Fixed(100.0)),
            height: Some(Dimension::Fixed(100.0)),
            corner_radius: Some(20.0),
            ..ViewStyle::default()
        };

        super::render_image(
            &source,
            "",
            ImageScale::Fit,
            None,
            true,
            false,
            &style,
            None,
            &BTreeSet::new(),
            0,
            &mut output,
        );

        let rendered = output.finish();
        assert!(rendered.contains("showLoadingIndicator = true"));
        assert!(rendered.contains("autoReplay = false"));
        assert!(rendered.contains("decodeWidthDp = 100f"));
        assert!(rendered.contains("decodeHeightDp = 100f"));
        assert!(rendered.contains("RoundedCornerShape(20.dp)"));
    }
}
