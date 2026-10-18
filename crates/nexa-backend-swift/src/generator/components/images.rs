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
    show_loading_indicator: bool,
    max_height: Option<f32>,
    fill_max_size: bool,
    shared_element: Option<&nexa_ir::Expr>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    match source {
        ImageSource::Asset(asset) => {
            out.push_str(&format!(
                "NexaAssetImagePrimitive(name: {}, description: {}, scale: {}, maxHeight: {}, fillMaxSize: {})",
                swift_string(asset),
                swift_string(description),
                image_scale(scale),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned()),
                fill_max_size
            ));
        }
        ImageSource::RemoteUrl(url) => {
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(url: {}, scale: {}, placeholder: {}, showLoadingIndicator: {}, allowsFile: false, description: {}, maxHeight: {}, fillMaxSize: {})",
                expression(url),
                image_scale(scale),
                placeholder
                    .map(swift_string)
                    .unwrap_or_else(|| "nil".to_owned()),
                show_loading_indicator,
                swift_string(description),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned()),
                fill_max_size
            ));
        }
        ImageSource::LocalFile(file) => {
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(url: {}, scale: {}, placeholder: nil, showLoadingIndicator: {}, allowsFile: true, description: {}, maxHeight: {}, fillMaxSize: {})",
                expression(file),
                image_scale(scale),
                show_loading_indicator,
                swift_string(description),
                max_height
                    .map(crate::generator::engine::utils::number)
                    .unwrap_or_else(|| "nil".to_owned()),
                fill_max_size
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
    out.push_str(
        r#"enum NexaImageScale { case fit; case fill }

struct NexaImageSizingModifier: ViewModifier {
    let scale: NexaImageScale
    let maxHeight: CGFloat?
    let fillMaxSize: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if fillMaxSize {
            content.frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let maxHeight {
            content
                .frame(maxWidth: scale == .fill ? .infinity : nil)
                .frame(height: maxHeight)
                .clipped()
        } else {
            content.frame(maxWidth: scale == .fill ? .infinity : nil)
        }
    }
}

"#,
    );
    out.push_str(
        r#"struct NexaImagePlaceholderPrimitive: View {
    let name: String
    let scale: NexaImageScale

    var body: some View {
        Image(name)
            .resizable()
            .aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            .accessibilityHidden(true)
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
    let fillMaxSize: Bool

    var body: some View {
        Image(name)
            .resizable()
            .aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            .modifier(NexaImageSizingModifier(scale: scale, maxHeight: maxHeight, fillMaxSize: fillMaxSize))
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
    let showLoadingIndicator: Bool
    let allowsFile: Bool
    let description: String
    let maxHeight: CGFloat?
    let fillMaxSize: Bool

    var body: some View {
        NexaRemoteImage(url: url, scale: scale, placeholder: placeholder, showLoadingIndicator: showLoadingIndicator, allowsFile: allowsFile)
            .modifier(NexaImageSizingModifier(scale: scale, maxHeight: maxHeight, fillMaxSize: fillMaxSize))
            .accessibilityLabel(description)
            .accessibilityHidden(description.isEmpty)
    }
}

"#,
        );
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, ImageScale, ImageSource};

    use super::{render_image, render_primitives};

    #[test]
    fn placeholder_primitive_is_shared_by_remote_loading_and_failure_states() {
        let mut output = SourceWriter::new();
        render_primitives(false, true, &mut output);
        let output = output.finish();

        assert!(output.contains("struct NexaImagePlaceholderPrimitive: View"));
        assert!(output.contains("struct NexaRemoteImagePrimitive: View"));
    }

    #[test]
    fn remote_images_forward_the_explicit_loading_indicator_option() {
        let source =
            ImageSource::RemoteUrl(Expr::String("https://example.test/image.png".to_owned()));
        let mut output = SourceWriter::new();

        render_image(
            &source,
            "",
            ImageScale::Fit,
            None,
            true,
            Some(80.0),
            false,
            None,
            0,
            &mut output,
        );

        assert!(output.finish().contains("showLoadingIndicator: true"));
    }

    #[test]
    fn bounded_images_use_a_clipped_fixed_height_frame_on_swiftui() {
        let mut output = SourceWriter::new();
        render_primitives(true, true, &mut output);
        let output = output.finish();

        assert!(output.contains("struct NexaImageSizingModifier: ViewModifier"));
        assert!(output.contains(".frame(height: maxHeight)"));
        assert!(output.contains(".clipped()"));
        assert!(output.matches(".modifier(NexaImageSizingModifier(").count() == 2);
    }
}
