use nexa_ir::{ImageScale, ImageSource};

use super::utils::{indent, swift_string};

pub(super) fn render_image(
    source: &ImageSource,
    description: &str,
    scale: ImageScale,
    placeholder: Option<&str>,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    match source {
        ImageSource::Asset(asset) => {
            out.push_str(&format!("Image({})", swift_string(asset)));
            append_resizable_image(out, scale);
        }
        ImageSource::RemoteUrl(url) => {
            out.push_str(&format!(
                "AsyncImage(url: URL(string: {})) {{ phase in\n",
                swift_string(url)
            ));
            indent(out, depth + 1);
            out.push_str("if let image = phase.image {\n");
            indent(out, depth + 2);
            out.push_str("image");
            append_resizable_image(out, scale);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("} else if phase.error != nil {\n");
            indent(out, depth + 2);
            if let Some(placeholder) = placeholder {
                out.push_str(&format!("Image({})\n", swift_string(placeholder)));
            } else {
                out.push_str("Image(systemName: \"photo\")\n");
            }
            indent(out, depth + 1);
            out.push_str("} else {\n");
            indent(out, depth + 2);
            if let Some(placeholder) = placeholder {
                out.push_str(&format!("Image({})", swift_string(placeholder)));
                append_resizable_image(out, scale);
                out.push('\n');
            } else {
                out.push_str("ProgressView()\n");
            }
            indent(out, depth + 1);
            out.push_str("}\n");
            indent(out, depth);
            out.push('}');
        }
    }
    if description.is_empty() {
        out.push_str(".accessibilityHidden(true)");
    } else {
        out.push_str(&format!(
            ".accessibilityLabel({})",
            swift_string(description)
        ));
    }
}

pub(super) fn append_resizable_image(out: &mut String, scale: ImageScale) {
    let content_mode = match scale {
        ImageScale::Fit => ".fit",
        ImageScale::Fill => ".fill",
    };
    out.push_str(&format!(
        ".resizable().aspectRatio(contentMode: {content_mode})"
    ));
}
