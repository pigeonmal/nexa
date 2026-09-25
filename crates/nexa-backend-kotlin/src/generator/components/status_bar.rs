use nexa_codegen::SourceWriter;
use nexa_ir::{ColorValue, StatusBarConfig};

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
    if let Some(ColorValue::Static(color)) = config.background {
        let argb = (u32::from(color.alpha) << 24)
            | (u32::from(color.red) << 16)
            | (u32::from(color.green) << 8)
            | u32::from(color.blue);
        out.push_str(&format!(
            "{deeply_nested_indent}nexaWindow.statusBarColor = android.graphics.Color.parseColor(\"#{argb:08X}\")\n"
        ));
    }
    out.push_str(&format!("{nested_indent}}}\n{indent}}}\n"));
}
