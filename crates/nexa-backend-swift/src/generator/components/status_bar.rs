use nexa_codegen::SourceWriter;
use nexa_ir::StatusBarConfig;

use crate::generator::engine::colors;

/// Renders the status-bar visibility, content style, and background modifiers.
pub(crate) fn render(config: Option<StatusBarConfig>, depth: usize, out: &mut SourceWriter) {
    let Some(config) = config else {
        return;
    };
    if config.hidden {
        out.push('\n');
        out.text_at(depth, format_args!(".statusBarHidden(true)"));
    }
    let scheme = match config.style {
        nexa_ir::StatusBarStyle::Default => None,
        // SwiftUI has no status-bar content color of its own, so the scheme is
        // inverted to steer the native bar toward the wanted foreground.
        nexa_ir::StatusBarStyle::Light => Some("dark"),
        nexa_ir::StatusBarStyle::Dark => Some("light"),
    };
    if let Some(scheme) = scheme {
        out.push('\n');
        out.text_at(depth, format_args!(".preferredColorScheme(.{scheme})"));
    }
    if let Some(background) = config.background {
        out.push('\n');
        out.text_at(depth, format_args!(".background(alignment: .top) {{ GeometryReader {{ proxy in {}.frame(height: proxy.safeAreaInsets.top) }}.ignoresSafeArea(edges: .top) }}",
            colors::expression(background)
        ));
    }
}
