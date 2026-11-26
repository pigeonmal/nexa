use nexa_codegen::SourceWriter;
use nexa_ir::{Dimension, ImageScale, ImageSource, ViewStyle};

use crate::generator::{
    expressions::expression,
    utils::{indent, swift_string},
};

use super::layout;

fn image_decode_height(style: &ViewStyle) -> Option<f32> {
    let height = match style.height {
        Some(Dimension::Fixed(h)) => Some(h),
        _ => match style.max_height {
            Some(Dimension::Fixed(h)) => Some(h),
            _ => None,
        },
    };
    let width = match style.width {
        Some(Dimension::Fixed(w)) => Some(w),
        _ => None,
    };
    match (width, height) {
        (Some(w), Some(h)) => Some(w.max(h)),
        (None, Some(h)) => Some(h),
        (Some(w), None) => Some(w),
        (None, None) => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_image_primitive(
    source: &ImageSource,
    description: &str,
    fit: ImageScale,
    placeholder: Option<&str>,
    show_loading_indicator: bool,
    auto_replay: bool,
    fit_height: Option<f32>,
    shared_element: Option<&nexa_ir::Expr>,
    out: &mut SourceWriter,
) {
    let fit_height_arg = fit_height
        .map(crate::generator::engine::utils::number)
        .unwrap_or_else(|| "nil".to_owned());
    match source {
        ImageSource::Asset(asset) => {
            out.push_str(&format!(
                "NexaAssetImagePrimitive(name: {}, description: {}, scale: {}, autoReplay: {auto_replay}, fitHeight: {fit_height_arg})",
                swift_string(asset),
                swift_string(description),
                image_scale(fit),
            ));
        }
        ImageSource::RemoteUrl(url) | ImageSource::LocalFile(url) => {
            let allows_file = matches!(source, ImageSource::LocalFile(_));
            let placeholder = if allows_file { None } else { placeholder };
            let shared_image_arg = if shared_element.is_some() {
                ", usesSharedElement: true"
            } else {
                ""
            };
            out.push_str(&format!(
                "NexaRemoteImagePrimitive(url: {}, scale: {}, placeholder: {}, showLoadingIndicator: {show_loading_indicator}, autoReplay: {auto_replay}, allowsFile: {allows_file}, description: {}, fitHeight: {fit_height_arg}{shared_image_arg})",
                expression(url),
                image_scale(fit),
                placeholder
                    .map(swift_string)
                    .unwrap_or_else(|| "nil".to_owned()),
                swift_string(description),
            ));
        }
    }
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
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    let fit_height = image_decode_height(style);
    let has_height_bound = style.height.is_some() || style.max_height.is_some();
    if fit == ImageScale::Fill && has_height_bound {
        // Bound `.fill` inside a `Color.clear` sizing container so SwiftUI's
        // `.aspectRatio(contentMode: .fill)` cannot expand layout or accessibility bounds.
        out.push_str("Color.clear");
        let mut size_style = ViewStyle {
            width: style.width,
            height: style.height,
            min_width: style.min_width,
            max_width: style.max_width,
            min_height: style.min_height,
            max_height: style.max_height,
            ..ViewStyle::default()
        };
        if size_style.width.is_none() && size_style.max_width.is_none() {
            size_style.width = Some(Dimension::Fill);
        }
        if size_style.height.is_none() {
            size_style.height = size_style.max_height;
        }
        layout::append_size_modifiers(out, depth, &size_style, ".center");
        out.push_str(&format!("\n{}.overlay(", "    ".repeat(depth + 1)));
        emit_image_primitive(
            source,
            description,
            fit,
            placeholder,
            show_loading_indicator,
            auto_replay,
            fit_height,
            shared_element,
            out,
        );
        out.push(')');
        out.push_str(&format!("\n{}.clipped()", "    ".repeat(depth + 1)));
        out.push_str(&format!("\n{}.contentShape(Rectangle())", "    ".repeat(depth + 1)));
        layout::append_surface_and_effect_modifiers(out, depth, style);
    } else {
        emit_image_primitive(
            source,
            description,
            fit,
            placeholder,
            show_loading_indicator,
            auto_replay,
            fit_height,
            shared_element,
            out,
        );
        if fit == ImageScale::Fill && style.width.is_none() && style.max_width.is_none() {
            out.push_str(&format!(
                "\n{}.frame(maxWidth: .infinity).clipped()",
                "    ".repeat(depth + 1)
            ));
        }
        layout::append_style(out, depth, style, ".center");
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

pub(crate) fn render_primitives(
    include_asset: bool,
    include_gif_asset: bool,
    include_remote: bool,
    out: &mut SourceWriter,
) {
    out.push_str(
        r#"enum NexaImageScale { case fit; case fill }

private func nexaImageAssetStem(_ name: String) -> String {
    (name as NSString).deletingPathExtension
}

private func nexaImageAssetIsGif(_ name: String) -> Bool {
    name.lowercased().hasSuffix(".gif")
}

"#,
    );
    if include_gif_asset || include_remote {
        out.push_str(
            r#"struct NexaAnimatedGifView: UIViewRepresentable {
    let data: Data
    let scale: NexaImageScale
    let autoReplay: Bool
    let maxPixelHeight: Int

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeUIView(context: Context) -> UIImageView {
        let view = UIImageView()
        view.clipsToBounds = true
        view.contentMode = scale == .fit ? .scaleAspectFit : .scaleAspectFill
        view.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        view.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        view.setContentHuggingPriority(.defaultLow, for: .horizontal)
        view.setContentHuggingPriority(.defaultLow, for: .vertical)
        context.coordinator.configure(
            view: view,
            data: data,
            autoReplay: autoReplay,
            maxPixelHeight: maxPixelHeight
        )
        return view
    }

    func updateUIView(_ view: UIImageView, context: Context) {
        view.contentMode = scale == .fit ? .scaleAspectFit : .scaleAspectFill
        context.coordinator.configure(
            view: view,
            data: data,
            autoReplay: autoReplay,
            maxPixelHeight: maxPixelHeight
        )
    }

    static func dismantleUIView(_ uiView: UIImageView, coordinator: Coordinator) {
        coordinator.stop()
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: UIImageView, context: Context) -> CGSize? {
        guard scale == .fit, let imageSize = uiView.image?.size, imageSize.width > 0, imageSize.height > 0 else {
            return nil
        }
        let aspect = imageSize.width / imageSize.height
        if let maxH = proposal.height, maxH.isFinite {
            let w = min(proposal.width ?? (maxH * aspect), maxH * aspect)
            return CGSize(width: w, height: w / aspect)
        }
        if let maxW = proposal.width, maxW.isFinite {
            return CGSize(width: maxW, height: maxW / aspect)
        }
        return imageSize
    }

    final class Coordinator: NSObject {
        private weak var imageView: UIImageView?
        private var source: CGImageSource?
        private var dataIdentifier: Int = 0
        private var frameCount: Int = 0
        private var delays: [CFTimeInterval] = []
        private var currentIndex: Int = 0
        private var accumulator: CFTimeInterval = 0
        private var autoReplay: Bool = true
        private var finishedOnce: Bool = false
        private var maxPixelHeight: Int = 2048
        private var displayLink: CADisplayLink?

        func configure(view: UIImageView, data: Data, autoReplay: Bool, maxPixelHeight: Int) {
            self.imageView = view
            let identifier = data.hashValue ^ data.count
            let replayChanged = self.autoReplay != autoReplay
            self.autoReplay = autoReplay
            self.maxPixelHeight = maxPixelHeight
            if identifier == dataIdentifier, source != nil {
                if replayChanged && autoReplay && finishedOnce {
                    finishedOnce = false
                    currentIndex = 0
                    accumulator = 0
                    displayLink?.isPaused = false
                }
                return
            }
            stop()
            dataIdentifier = identifier
            guard let cgSource = CGImageSourceCreateWithData(
                data as CFData, [kCGImageSourceShouldCache: false] as CFDictionary
            ) else { return }
            let count = CGImageSourceGetCount(cgSource)
            guard count > 0 else { return }
            source = cgSource
            frameCount = count
            delays = (0..<count).map { Self.frameDelay(source: cgSource, index: $0) }
            currentIndex = 0
            accumulator = 0
            finishedOnce = false
            view.image = decodeFrame(index: 0)
            guard count > 1 else { return }
            let link = CADisplayLink(target: self, selector: #selector(step(_:)))
            link.add(to: .main, forMode: .common)
            displayLink = link
        }

        func stop() {
            displayLink?.invalidate()
            displayLink = nil
            source = nil
            delays = []
            frameCount = 0
            dataIdentifier = 0
        }

        @objc private func step(_ link: CADisplayLink) {
            guard !finishedOnce, frameCount > 1 else {
                link.isPaused = true
                return
            }
            let dt = min(max(0, link.targetTimestamp - link.timestamp), 0.1)
            accumulator += dt
            let frameDuration = delays[currentIndex]
            guard accumulator >= frameDuration else { return }
            accumulator -= frameDuration
            let nextIndex = currentIndex + 1
            if nextIndex >= frameCount {
                if !autoReplay {
                    finishedOnce = true
                    link.isPaused = true
                    return
                }
                currentIndex = 0
            } else {
                currentIndex = nextIndex
            }
            if let image = decodeFrame(index: currentIndex) {
                imageView?.image = image
            }
        }

        private func decodeFrame(index: Int) -> UIImage? {
            guard let source else { return nil }
            if maxPixelHeight < 2048 {
                let options: [CFString: Any] = [
                    kCGImageSourceCreateThumbnailFromImageAlways: true,
                    kCGImageSourceShouldCacheImmediately: true,
                    kCGImageSourceCreateThumbnailWithTransform: true,
                    kCGImageSourceThumbnailMaxPixelSize: maxPixelHeight
                ]
                if let cgImage = CGImageSourceCreateThumbnailAtIndex(source, index, options as CFDictionary) {
                    return UIImage(cgImage: cgImage)
                }
            }
            if let cgImage = CGImageSourceCreateImageAtIndex(source, index, nil) {
                return UIImage(cgImage: cgImage)
            }
            return nil
        }

        private static func frameDelay(source: CGImageSource, index: Int) -> CFTimeInterval {
            guard let props = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any],
                  let gif = props[kCGImagePropertyGIFDictionary] as? [CFString: Any]
            else { return 0.1 }
            let raw = (gif[kCGImagePropertyGIFUnclampedDelayTime] as? Double)
                ?? (gif[kCGImagePropertyGIFDelayTime] as? Double)
                ?? 0.1
            return raw <= 0.01 ? 0.1 : raw
        }
    }
}

"#,
        );
    }
    out.push_str(
        r#"struct NexaImagePlaceholderPrimitive: View {
    let name: String
    let scale: NexaImageScale
    var autoReplay: Bool = true

    var body: some View {
        let stem = nexaImageAssetStem(name)
        Image(stem)
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
    var autoReplay: Bool = true
    var fitHeight: CGFloat? = nil
    @Environment(\.displayScale) private var displayScale

    var body: some View {
        Group {
            if nexaImageAssetIsGif(name),
               let data = NSDataAsset(name: "\(nexaImageAssetStem(name))_gif")?.data {
                let maxPixels = fitHeight.map { Int(($0 * displayScale).rounded(.up)) } ?? 2048
                NexaAnimatedGifView(
                    data: data,
                    scale: scale,
                    autoReplay: autoReplay,
                    maxPixelHeight: max(1, min(maxPixels, 2048))
                )
            } else {
                Image(nexaImageAssetStem(name))
                    .resizable()
                    .aspectRatio(contentMode: scale == .fit ? .fit : .fill)
            }
        }
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
    var autoReplay: Bool = true
    let allowsFile: Bool
    let description: String
    var fitHeight: CGFloat? = nil
    var usesSharedElement = false

    var body: some View {
        NexaRemoteImage(
            url: url, scale: scale, placeholder: placeholder,
            showLoadingIndicator: showLoadingIndicator, autoReplay: autoReplay, allowsFile: allowsFile,
            fitHeight: !usesSharedElement ? fitHeight : nil
        )
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
    use nexa_ir::{Dimension, Expr, ImageScale, ImageSource, ViewStyle};

    use super::{render_image, render_primitives};

    #[test]
    fn placeholder_primitive_is_shared_by_remote_loading_and_failure_states() {
        let mut output = SourceWriter::new();
        render_primitives(false, false, true, &mut output);
        let output = output.finish();

        assert!(output.contains("struct NexaImagePlaceholderPrimitive: View"));
        assert!(output.contains("struct NexaRemoteImagePrimitive: View"));
        assert!(output.contains("struct NexaAnimatedGifView: UIViewRepresentable"));
    }

    #[test]
    fn remote_images_forward_the_explicit_loading_indicator_option() {
        let source =
            ImageSource::RemoteUrl(Expr::String("https://example.test/image.png".to_owned()));
        let mut output = SourceWriter::new();
        let style = ViewStyle {
            max_height: Some(Dimension::Fixed(80.0)),
            ..ViewStyle::default()
        };

        render_image(
            &source,
            "",
            ImageScale::Fit,
            None,
            true,
            false,
            &style,
            None,
            0,
            &mut output,
        );

        let rendered = output.finish();
        assert!(rendered.contains("showLoadingIndicator: true"));
        assert!(rendered.contains("autoReplay: false"));
        assert!(rendered.contains("fitHeight: 80"));
    }

    #[test]
    fn bounded_fill_images_use_clipped_clear_container_on_swiftui() {
        let source = ImageSource::Asset("hero.png".to_owned());
        let mut output = SourceWriter::new();
        let style = ViewStyle {
            width: Some(Dimension::Fixed(100.0)),
            height: Some(Dimension::Fixed(100.0)),
            corner_radius: Some(20.0),
            ..ViewStyle::default()
        };

        render_image(
            &source,
            "Hero",
            ImageScale::Fill,
            None,
            false,
            true,
            &style,
            None,
            0,
            &mut output,
        );

        let rendered = output.finish();
        assert!(rendered.contains("Color.clear"));
        assert!(rendered.contains(".overlay(NexaAssetImagePrimitive("));
        assert!(rendered.contains(".clipped()"));
        assert!(rendered.contains("RoundedRectangle(cornerRadius: 20)"));
    }

    #[test]
    fn shared_images_keep_full_resolution_in_the_transition() {
        let source = ImageSource::LocalFile(Expr::String("file:///cover.jpg".to_owned()));
        let key = Expr::String("album-cover".to_owned());
        let mut output = SourceWriter::new();
        let style = ViewStyle {
            max_height: Some(Dimension::Fixed(64.0)),
            ..ViewStyle::default()
        };
        render_image(
            &source,
            "Album cover",
            ImageScale::Fit,
            None,
            false,
            true,
            &style,
            Some(&key),
            0,
            &mut output,
        );
        assert!(
            output
                .finish()
                .contains("usesSharedElement: true")
        );
    }
}
