use nexa_codegen::SourceWriter;
use nexa_ir::StatusBarConfig;

use crate::generator::engine::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.uses_status_bar;
    imports.add(enabled, "android.app.Activity");
    imports.add(enabled, "androidx.core.view.WindowCompat");
    imports.add(enabled, "androidx.core.view.WindowInsetsCompat");
    imports.add(enabled, "androidx.compose.runtime.SideEffect");
    imports.add(enabled, "androidx.compose.ui.platform.LocalView");
}

pub(crate) fn render(
    config: Option<StatusBarConfig>,
    enabled: bool,
    depth: usize,
    out: &mut SourceWriter,
) {
    if !enabled {
        return;
    }
    let Some(config) = config else {
        return;
    };
    let indent = "    ".repeat(depth);
    let nested_indent = "    ".repeat(depth + 1);
    let deeply_nested_indent = "    ".repeat(depth + 2);
    out.push_str(&format!(
        "{indent}val nexaStatusBarView = LocalView.current\n"
    ));
    out.push_str(&format!("{indent}SideEffect {{\n"));
    out.push_str(&format!(
        "{nested_indent}val nexaWindow = (nexaStatusBarView.context as? Activity)?.window\n"
    ));
    out.push_str(&format!(
        "{nested_indent}nexaWindow?.let {{ nexaWindow ->\n{deeply_nested_indent}val nexaController = WindowCompat.getInsetsController(nexaWindow, nexaStatusBarView)\n"
    ));
    if config.hidden {
        out.push_str(&format!(
            "{deeply_nested_indent}nexaController.hide(WindowInsetsCompat.Type.statusBars())\n"
        ));
    } else {
        out.push_str(&format!(
            "{deeply_nested_indent}nexaController.show(WindowInsetsCompat.Type.statusBars())\n"
        ));
    }
    match config.style {
        nexa_ir::StatusBarStyle::Light => out.push_str(&format!(
            "{deeply_nested_indent}nexaController.isAppearanceLightStatusBars = false\n"
        )),
        nexa_ir::StatusBarStyle::Dark => out.push_str(&format!(
            "{deeply_nested_indent}nexaController.isAppearanceLightStatusBars = true\n"
        )),
        nexa_ir::StatusBarStyle::Default => {}
    }
    out.push_str(&format!("{nested_indent}}}\n{indent}}}\n"));
}

pub(crate) fn render_background_start(
    config: Option<StatusBarConfig>,
    depth: usize,
    out: &mut SourceWriter,
) -> bool {
    let Some(background) = config.and_then(|config| config.background) else {
        return false;
    };
    out.line_at(
        depth,
        format_args!(
            "NexaStatusBarBackgroundPrimitive(background = {}) {{",
            crate::generator::colors::expression(background)
        ),
    );
    true
}

pub(crate) fn render_background_end(depth: usize, out: &mut SourceWriter) {
    out.line_at(depth, format_args!("}}"));
}

#[cfg(test)]
mod tests {
    use super::{render, render_background_start};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorValue, StatusBarConfig, StatusBarStyle};

    fn config() -> StatusBarConfig {
        StatusBarConfig {
            style: StatusBarStyle::Dark,
            hidden: false,
            background: Some(ColorValue::Static(Color {
                red: 242,
                green: 242,
                blue: 247,
                alpha: 255,
            })),
        }
    }

    #[test]
    fn status_bar_background_is_drawn_in_compose_instead_of_window_color() {
        let mut output = SourceWriter::new();
        assert!(render_background_start(Some(config()), 1, &mut output));
        let generated = output.finish();

        assert!(generated.contains("NexaStatusBarBackgroundPrimitive(background ="));
        assert!(generated.contains("0xFFF2F2F7"));
    }

    #[test]
    fn status_bar_configuration_does_not_use_deprecated_window_color() {
        let mut output = SourceWriter::new();
        render(Some(config()), true, 1, &mut output);
        let generated = output.finish();

        assert!(generated.contains("isAppearanceLightStatusBars = true"));
        assert!(!generated.contains("statusBarColor"));
    }
}
