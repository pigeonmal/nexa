use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node, walk::walk_ir};

use crate::generator::{
    engine::{features::Features, imports::ImportSet},
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
    out.line_at(
        depth,
        format_args!(
            "NexaAppearancePrimitive(mode = {}) {{",
            crate::generator::engine::expressions::expression(mode)
        ),
    );
}

pub(crate) fn render_theme_end(depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    out.push('}');
    out.push('\n');
}
