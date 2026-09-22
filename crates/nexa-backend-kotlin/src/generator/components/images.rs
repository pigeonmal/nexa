use nexa_ir::{ImageScale, ImageSource};

use crate::generator::{
    expressions::expression,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_asset, "androidx.compose.foundation.Image");
    imports.add(features.uses_remote_image, "coil3.compose.AsyncImage");
    imports.add(
        features.uses_remote_image,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_asset || features.uses_remote_image,
        "androidx.compose.ui.layout.ContentScale",
    );
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

pub(crate) fn render_image(
    source: &ImageSource,
    description: &str,
    scale: ImageScale,
    placeholder: Option<&str>,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    let content_scale = match scale {
        ImageScale::Fit => "ContentScale.Fit",
        ImageScale::Fill => "ContentScale.Crop",
    };
    let description = if description.is_empty() {
        "null".to_owned()
    } else {
        kotlin_string(description)
    };
    match source {
        ImageSource::Asset(asset) => {
            out.push_str(&format!(
                "Image(\n{}    painter = nexaDrawablePainter({}),\n{}    contentDescription = {description},\n{}    contentScale = {content_scale}\n{})",
                "    ".repeat(depth),
                kotlin_string(asset),
                "    ".repeat(depth),
                "    ".repeat(depth),
                "    ".repeat(depth),
            ));
        }
        ImageSource::RemoteUrl(url) => {
            let model = if matches!(url, nexa_ir::Expr::String(_)) {
                expression(url)
            } else {
                format!(
                    "({}).takeIf {{ it.startsWith(\"https://\") }}",
                    expression(url)
                )
            };
            out.push_str(&format!(
                "AsyncImage(\n{}    model = {model},\n{}    imageLoader = nexaImageLoader(),\n{}    contentDescription = {description},\n",
                "    ".repeat(depth),
                "    ".repeat(depth),
                "    ".repeat(depth),
            ));
            if let Some(placeholder) = placeholder {
                out.push_str(&format!(
                    "{}    placeholder = nexaDrawablePainter({}),\n{}    error = nexaDrawablePainter({}),\n",
                    "    ".repeat(depth),
                    kotlin_string(placeholder),
                    "    ".repeat(depth),
                    kotlin_string(placeholder),
                ));
            }
            out.push_str(&format!(
                "{}    contentScale = {content_scale}\n{})",
                "    ".repeat(depth),
                "    ".repeat(depth)
            ));
        }
    }
}
