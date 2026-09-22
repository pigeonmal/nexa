use nexa_ir::{ImageScale, ImageSource};

use crate::generator::{
    expressions::expression,
    utils::{indent, swift_string},
};

pub(crate) fn render_image(
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
                "NexaRemoteImage(url: {}, scale: {}, placeholder: {})",
                expression(url),
                match scale {
                    ImageScale::Fit => "NexaImageScale.fit",
                    ImageScale::Fill => "NexaImageScale.fill",
                },
                placeholder
                    .map(swift_string)
                    .unwrap_or_else(|| "nil".to_owned())
            ));
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

pub(crate) fn append_resizable_image(out: &mut String, scale: ImageScale) {
    let content_mode = match scale {
        ImageScale::Fit => ".fit",
        ImageScale::Fill => ".fill",
    };
    out.push_str(&format!(
        ".resizable().aspectRatio(contentMode: {content_mode})"
    ));
}
