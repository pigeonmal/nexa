use nexa_codegen::SourceWriter;
use nexa_ir::{ImageScale, ImageSource};

use crate::generator::{
    expressions::expression,
    utils::{indent, swift_string},
};

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
    match source {
        ImageSource::Asset(asset) => {
            out.push_str(&format!("Image({})", swift_string(asset)));
            append_resizable_image(out, scale);
        }
        ImageSource::RemoteUrl(url) => {
            out.push_str(&format!(
                "NexaRemoteImage(url: {}, scale: {}, placeholder: {}, allowsFile: false)",
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
        ImageSource::LocalFile(file) => {
            out.push_str(&format!(
                "NexaRemoteImage(url: {}, scale: {}, placeholder: nil, allowsFile: true)",
                expression(file),
                match scale {
                    ImageScale::Fit => "NexaImageScale.fit",
                    ImageScale::Fill => "NexaImageScale.fill",
                }
            ));
        }
    }
    if let Some(max_height) = max_height {
        out.push_str(&format!(
            ".frame(maxHeight: {})",
            crate::generator::engine::utils::number(max_height)
        ));
    }
    if description.is_empty() {
        out.push_str(".accessibilityHidden(true)");
    } else {
        out.push_str(&format!(
            ".accessibilityLabel({})",
            swift_string(description)
        ));
    }
    if let Some(id) = shared_element {
        out.push_str(&format!(".nexaSharedElement(id: {})", expression(id)));
    }
}

pub(crate) fn append_resizable_image(out: &mut SourceWriter, scale: ImageScale) {
    let content_mode = match scale {
        ImageScale::Fit => ".fit",
        ImageScale::Fill => ".fill",
    };
    out.push_str(&format!(
        ".resizable().aspectRatio(contentMode: {content_mode})"
    ));
}
