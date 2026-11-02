use nexa_codegen::SourceWriter;
use nexa_ir::StatusBarConfig;

use crate::generator::engine::colors;

/// Renders status-bar visibility, content style, and background modifiers.
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
        // Keep the status-bar style scoped to the system bar. Using
        // preferredColorScheme here would also recolor the entire screen.
        nexa_ir::StatusBarStyle::Light => Some("dark"),
        nexa_ir::StatusBarStyle::Dark => Some("light"),
    };
    if let Some(scheme) = scheme {
        out.push('\n');
        out.text_at(
            depth,
            format_args!(".modifier(NexaStatusBarStylePrimitive(colorScheme: .{scheme}))"),
        );
    }
    if let Some(background) = config.background {
        out.push('\n');
        out.text_at(
            depth,
            format_args!(
                ".overlay(alignment: .top) {{ NexaStatusBarBackgroundPrimitive(color: {}) }}",
                colors::expression(background)
            ),
        );
        // A navigation destination's native bar otherwise paints an opaque
        // default surface over the status-area background. Keep the native
        // title and controls, but let this screen's safe-area overlay show
        // through behind the system status items.
        out.push('\n');
        out.text_at(
            depth,
            format_args!(".toolbarBackground(.hidden, for: .navigationBar)"),
        );
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorValue, StatusBarConfig, StatusBarStyle};

    use super::render;

    #[test]
    fn status_bar_background_reveals_the_safe_area_overlay_under_native_navigation_chrome() {
        let config = StatusBarConfig {
            style: StatusBarStyle::Default,
            hidden: false,
            background: Some(ColorValue::Static(Color {
                red: 242,
                green: 242,
                blue: 247,
                alpha: 255,
            })),
            navigation_bar_background: None,
        };
        let mut output = SourceWriter::new();

        render(Some(config), 1, &mut output);

        let generated = output.finish();
        assert!(generated.contains(".toolbarBackground(.hidden, for: .navigationBar)"));
        assert!(generated.contains("NexaStatusBarBackgroundPrimitive(color:"));
        assert!(!generated.contains("statusBarColor"));
    }

    #[test]
    fn explicit_status_bar_style_does_not_override_the_screen_color_scheme() {
        let config = StatusBarConfig {
            style: StatusBarStyle::Light,
            hidden: false,
            background: None,
            navigation_bar_background: None,
        };
        let mut output = SourceWriter::new();

        render(Some(config), 1, &mut output);

        let generated = output.finish();
        assert!(generated.contains(".modifier(NexaStatusBarStylePrimitive(colorScheme: .dark))"));
        assert!(!generated.contains(".preferredColorScheme("));
    }
}
