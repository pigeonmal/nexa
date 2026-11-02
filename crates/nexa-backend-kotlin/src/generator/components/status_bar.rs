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
    imports.add(enabled, "androidx.compose.ui.graphics.luminance");
    imports.add(
        enabled,
        "androidx.compose.foundation.layout.windowInsetsTopHeight",
    );
    imports.add(
        enabled,
        "androidx.compose.foundation.layout.windowInsetsBottomHeight",
    );
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
    out.push_str(&format!(
        "{indent}val nexaStatusBarWindow = androidx.compose.runtime.remember(nexaStatusBarView) {{\n"
    ));
    out.push_str(&format!(
        "{nested_indent}var nexaStatusBarContext: android.content.Context = nexaStatusBarView.context\n"
    ));
    out.push_str(&format!(
        "{nested_indent}while (nexaStatusBarContext is android.content.ContextWrapper && nexaStatusBarContext !is Activity) nexaStatusBarContext = nexaStatusBarContext.baseContext\n"
    ));
    out.push_str(&format!(
        "{nested_indent}(nexaStatusBarContext as? Activity)?.window\n{indent}}}\n"
    ));
    out.push_str(&format!(
        "{indent}androidx.compose.runtime.DisposableEffect(nexaStatusBarWindow, nexaStatusBarView) {{\n"
    ));
    out.push_str(&format!(
        "{nested_indent}val nexaStatusBarController = nexaStatusBarWindow?.let {{ WindowCompat.getInsetsController(it, nexaStatusBarView) }}\n"
    ));
    out.push_str(&format!(
        "{nested_indent}val nexaInitialStatusBarStyle = nexaStatusBarController?.isAppearanceLightStatusBars\n"
    ));
    out.push_str(&format!(
        "{nested_indent}val nexaInitialNavigationBarStyle = nexaStatusBarController?.isAppearanceLightNavigationBars\n"
    ));
    out.push_str(&format!(
        "{nested_indent}val nexaInitialStatusBarVisibility = androidx.core.view.ViewCompat.getRootWindowInsets(nexaStatusBarView)?.isVisible(WindowInsetsCompat.Type.statusBars())\n"
    ));
    out.push_str(&format!("{nested_indent}onDispose {{\n"));
    out.push_str(&format!(
        "{deeply_nested_indent}nexaStatusBarController?.let {{ controller ->\n"
    ));
    out.push_str(&format!(
        "{}nexaInitialStatusBarVisibility?.let {{ visible -> if (visible) controller.show(WindowInsetsCompat.Type.statusBars()) else controller.hide(WindowInsetsCompat.Type.statusBars()) }}\n",
        "    ".repeat(depth + 3)
    ));
    out.push_str(&format!(
        "{}nexaInitialStatusBarStyle?.let {{ controller.isAppearanceLightStatusBars = it }}\n",
        "    ".repeat(depth + 3)
    ));
    out.push_str(&format!(
        "{}nexaInitialNavigationBarStyle?.let {{ controller.isAppearanceLightNavigationBars = it }}\n",
        "    ".repeat(depth + 3)
    ));
    out.push_str(&format!("{deeply_nested_indent}}}\n"));
    out.push_str(&format!("{nested_indent}}}\n"));
    out.push_str(&format!("{indent}}}\n"));
    out.push_str(&format!("{indent}SideEffect {{\n"));
    out.push_str(&format!(
        "{nested_indent}nexaStatusBarWindow?.let {{ nexaWindow ->\n{deeply_nested_indent}val nexaController = WindowCompat.getInsetsController(nexaWindow, nexaStatusBarView)\n"
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
    if let Some(background) = config.navigation_bar_background {
        let background = crate::generator::colors::expression(background);
        // Keep the system navigation bar transparent so edge-to-edge content
        // remains visible underneath it. The color itself is drawn by
        // NexaStatusBarBackgroundPrimitive inside the Compose hierarchy.
        out.push_str(&format!(
            "{deeply_nested_indent}nexaController.isAppearanceLightNavigationBars = {background}.luminance() > 0.5f\n"
        ));
    }
    out.push_str(&format!("{nested_indent}}}\n{indent}}}\n"));
}

pub(crate) fn render_background_start(
    config: Option<StatusBarConfig>,
    depth: usize,
    out: &mut SourceWriter,
) -> bool {
    let Some(config) = config else {
        return false;
    };
    if config.background.is_none() && config.navigation_bar_background.is_none() {
        return false;
    }
    out.line_at(
        depth,
        format_args!(
            "NexaStatusBarBackgroundPrimitive(statusBarBackground = {}, navigationBarBackground = {}) {{",
            config.background.map(crate::generator::colors::expression).unwrap_or_else(|| "null".to_owned()),
            config.navigation_bar_background.map(crate::generator::colors::expression).unwrap_or_else(|| "null".to_owned()),
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
            navigation_bar_background: None,
        }
    }

    #[test]
    fn status_bar_background_is_drawn_in_compose_instead_of_window_color() {
        let mut output = SourceWriter::new();
        assert!(render_background_start(Some(config()), 1, &mut output));
        let generated = output.finish();

        assert!(generated.contains("NexaStatusBarBackgroundPrimitive(statusBarBackground ="));
        assert!(generated.contains("0xFFF2F2F7"));
    }

    #[test]
    fn navigation_bar_background_is_passed_to_the_shared_native_inset_primitive() {
        let mut config = config();
        config.navigation_bar_background = Some(ColorValue::Static(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        }));
        let mut output = SourceWriter::new();

        assert!(render_background_start(Some(config), 1, &mut output));
        let generated = output.finish();

        assert!(generated.contains("navigationBarBackground = Color(0xFF000000)) {"));
    }

    #[test]
    fn status_bar_configuration_does_not_use_deprecated_window_color() {
        let mut output = SourceWriter::new();
        render(Some(config()), true, 1, &mut output);
        let generated = output.finish();

        assert!(generated.contains("isAppearanceLightStatusBars = true"));
        assert!(!generated.contains("statusBarColor"));
        assert!(generated.contains("DisposableEffect(nexaStatusBarWindow, nexaStatusBarView)"));
        assert!(generated.contains("ViewCompat.getRootWindowInsets(nexaStatusBarView)?.isVisible"));
        assert!(generated.contains("nexaInitialStatusBarStyle?.let"));
        assert!(generated.contains("onDispose"));
    }

    #[test]
    fn navigation_bar_configuration_keeps_edge_to_edge_and_sets_icon_style() {
        let mut config = config();
        config.navigation_bar_background = Some(ColorValue::Static(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        }));
        let mut output = SourceWriter::new();
        render(Some(config), true, 0, &mut output);
        let generated = output.finish();

        assert!(!generated.contains("navigationBarColor"));
        assert!(
            generated
                .contains("isAppearanceLightNavigationBars = Color(0xFF000000).luminance() > 0.5f")
        );
        assert!(generated.contains("nexaInitialNavigationBarStyle"));
    }
}
