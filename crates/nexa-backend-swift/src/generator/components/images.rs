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
            out.push_str(&format!(
                "NexaAssetImagePrimitive(name: {}, description: {}, scale: {}, maxHeight: {})",
                swift_string(asset),
                swift_string(description),
                image_scale(scale),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned())
            ));
        }
        ImageSource::RemoteUrl(url) => {
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(url: {}, scale: {}, placeholder: {}, allowsFile: false, description: {}, maxHeight: {})",
                expression(url),
                image_scale(scale),
                placeholder
                    .map(swift_string)
                    .unwrap_or_else(|| "nil".to_owned()),
                swift_string(description),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned())
            ));
        }
        ImageSource::LocalFile(file) => {
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(url: {}, scale: {}, placeholder: nil, allowsFile: true, description: {}, maxHeight: {})",
                expression(file),
                image_scale(scale),
                swift_string(description),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned())
            ));
        }
    }
    if let Some(id) = shared_element {
        out.push_str(&format!(".nexaSharedElement(id: {})", expression(id)));
    }
}

fn image_scale(scale: ImageScale) -> &'static str {
    match scale {
        ImageScale::Fit => ".fit",
        ImageScale::Fill => ".fill",
    }
}

pub(crate) fn render_primitives(include_asset: bool, include_remote: bool, out: &mut SourceWriter) {
    out.push_str("enum NexaImageScale { case fit; case fill }\n\n");
    out.push_str(
        r#"struct NexaImageFallbackPrimitive: View {
    let placeholder: String?
    let description: String
    let scale: NexaImageScale
    let maxHeight: CGFloat?

    @ViewBuilder var body: some View {
        Group {
            if let placeholder {
                Image(placeholder).resizable().aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            } else {
                Image(systemName: "photo")
            }
        }
            .frame(maxHeight: maxHeight)
            .accessibilityLabel(description)
            .accessibilityHidden(description.isEmpty)
    }
}

"#,
    );
    if include_asset {
        out.push_str(
            r#"struct NexaAssetImagePrimitive: View {
    let name: String
    let description: String
    let scale: NexaImageScale
    let maxHeight: CGFloat?

    var body: some View {
        Image(name)
            .resizable()
            .aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            .frame(maxHeight: maxHeight)
            .accessibilityLabel(description)
            .accessibilityHidden(description.isEmpty)
    }
}

"#,
        );
    }
    if include_remote {
        out.push_str(
            r#"struct NexaRemoteImagePrimitive: View {
    let url: String
    let scale: NexaImageScale
    let placeholder: String?
    let allowsFile: Bool
    let description: String
    let maxHeight: CGFloat?

    var body: some View {
        NexaRemoteImage(url: url, scale: scale, placeholder: placeholder, allowsFile: allowsFile)
            .frame(maxHeight: maxHeight)
            .accessibilityLabel(description)
            .accessibilityHidden(description.isEmpty)
    }
}

"#,
        );
    }
}
