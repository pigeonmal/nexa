use nexa_ir::{ImageScale, ImageSource};

use super::{
    expressions::expression,
    utils::{indent, kotlin_string},
};

pub(super) fn render_image(
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
    let model = match source {
        ImageSource::Asset(asset) => format!("R.drawable.{asset}"),
        ImageSource::RemoteUrl(url) => {
            if matches!(url, nexa_ir::Expr::String(_)) {
                expression(url)
            } else {
                format!(
                    "({}).takeIf {{ it.startsWith(\"https://\") }}",
                    expression(url)
                )
            }
        }
    };
    let description = if description.is_empty() {
        "null".to_owned()
    } else {
        kotlin_string(description)
    };
    out.push_str(&format!(
        "AsyncImage(\n{}    model = {model},\n",
        "    ".repeat(depth)
    ));
    if matches!(source, ImageSource::RemoteUrl(_)) {
        out.push_str(&format!(
            "{}    imageLoader = nexaImageLoader(),\n",
            "    ".repeat(depth)
        ));
    }
    out.push_str(&format!(
        "{}    contentDescription = {description},\n",
        "    ".repeat(depth)
    ));
    if let Some(placeholder) = placeholder {
        out.push_str(&format!(
            "{}    placeholder = painterResource(R.drawable.{placeholder}),\n{}    error = painterResource(R.drawable.{placeholder}),\n",
            "    ".repeat(depth),
            "    ".repeat(depth)
        ));
    }
    out.push_str(&format!(
        "{}    contentScale = {content_scale}\n{})",
        "    ".repeat(depth),
        "    ".repeat(depth)
    ));
}
