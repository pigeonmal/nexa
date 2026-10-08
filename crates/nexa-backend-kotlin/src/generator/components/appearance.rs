use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node, walk::walk_ir};

use crate::generator::{
    engine::{expressions::expression, features::Features, imports::ImportSet},
    utils::indent,
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    if !features.facts.ui.appearance {
        return;
    }
    imports.add(true, "androidx.compose.foundation.isSystemInDarkTheme");
    imports.add(true, "androidx.compose.foundation.layout.fillMaxSize");
    imports.add(true, "androidx.compose.material3.MaterialTheme");
    imports.add(true, "androidx.compose.material3.Surface");
    imports.add(true, "androidx.compose.material3.darkColorScheme");
    imports.add(true, "androidx.compose.material3.lightColorScheme");
    imports.add(true, "androidx.compose.ui.graphics.Color");
    imports.add(true, "androidx.compose.ui.graphics.luminance");
    imports.add(true, "androidx.compose.ui.graphics.toArgb");
    imports.add(true, "androidx.compose.ui.Modifier");
    imports.add(true, "androidx.compose.runtime.remember");
    imports.add(true, "androidx.compose.runtime.SideEffect");
    imports.add(true, "androidx.compose.ui.platform.LocalView");
    imports.add(true, "androidx.core.view.WindowCompat");
}

pub(crate) fn module_mode(module: &Module, root: nexa_ir::ScreenId) -> Option<Expr> {
    let root_screen = module.screens.iter().find(|screen| screen.id == root)?;
    let mut mode = None;
    walk_ir(
        &root_screen.body,
        &mut |node| {
            if mode.is_none()
                && let Node::Appearance {
                    mode: expression, ..
                } = node
            {
                mode = Some(expression.clone());
            }
        },
        &mut |_| {},
    );
    mode
}

pub(crate) fn render_theme_start(mode: &Expr, depth: usize, out: &mut SourceWriter) {
    let mode = expression(mode);
    let system_dark = format!("nexaSystemDarkTheme{}", out.next_id());
    let view = format!("nexaAppearanceView{}", out.next_id());
    let status_color = format!("nexaAppearanceStatusColor{}", out.next_id());
    let navigation_color = format!("nexaAppearanceNavigationColor{}", out.next_id());
    let dark_scheme = nexa_codegen::design_system::kotlin_color_scheme(true);
    let light_scheme = nexa_codegen::design_system::kotlin_color_scheme(false);
    out.line_at(
        depth,
        format_args!("val {system_dark} = isSystemInDarkTheme()"),
    );
    out.line_at(
        depth,
        format_args!("MaterialTheme(colorScheme = remember({mode}, {system_dark}) {{"),
    );
    out.line_at(depth + 1, format_args!("when ({mode}) {{"));
    out.line_at(depth + 2, format_args!("\"dark\" -> {dark_scheme}"));
    out.line_at(depth + 2, format_args!("\"light\" -> {light_scheme}"));
    out.line_at(
        depth + 2,
        format_args!("else -> if ({system_dark}) {dark_scheme} else {light_scheme}"),
    );
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth,
        format_args!(
            "}}, typography = MaterialTheme.typography, shapes = MaterialTheme.shapes) {{"
        ),
    );
    out.line_at(depth + 1, format_args!("val {view} = LocalView.current"));
    out.line_at(
        depth + 1,
        format_args!("val {status_color} = MaterialTheme.colorScheme.background"),
    );
    out.line_at(
        depth + 1,
        format_args!("val {navigation_color} = MaterialTheme.colorScheme.surfaceContainer"),
    );
    out.line_at(depth + 1, format_args!("SideEffect {{"));
    out.line_at(
        depth + 2,
        format_args!("var nexaAppearanceContext: android.content.Context = {view}.context"),
    );
    out.line_at(
        depth + 2,
        format_args!("while (nexaAppearanceContext is android.content.ContextWrapper && nexaAppearanceContext !is android.app.Activity) nexaAppearanceContext = nexaAppearanceContext.baseContext"),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "val nexaAppearanceWindow = (nexaAppearanceContext as? android.app.Activity)?.window"
        ),
    );
    out.line_at(
        depth + 2,
        format_args!("nexaAppearanceWindow?.let {{ window ->"),
    );
    out.line_at(
        depth + 3,
        format_args!(
            "if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) {{"
        ),
    );
    out.line_at(
        depth + 4,
        format_args!("window.isNavigationBarContrastEnforced = false"),
    );
    out.line_at(depth + 3, format_args!("}}"));
    out.line_at(
        depth + 3,
        format_args!("window.decorView.setBackgroundColor({status_color}.toArgb())"),
    );
    out.line_at(
        depth + 3,
        format_args!(
            "val nexaAppearanceInsetsController = WindowCompat.getInsetsController(window, {view})"
        ),
    );
    out.line_at(
        depth + 3,
        format_args!("nexaAppearanceInsetsController.isAppearanceLightStatusBars = {status_color}.luminance() > 0.5f"),
    );
    out.line_at(
        depth + 3,
        format_args!("nexaAppearanceInsetsController.isAppearanceLightNavigationBars = {navigation_color}.luminance() > 0.5f"),
    );
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(
            "Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background, contentColor = MaterialTheme.colorScheme.onBackground) {{"
        ),
    );
}

pub(crate) fn render_theme_end(depth: usize, out: &mut SourceWriter) {
    indent(out, depth + 1);
    out.push('}');
    out.push('\n');
    indent(out, depth);
    out.push('}');
    out.push('\n');
}
