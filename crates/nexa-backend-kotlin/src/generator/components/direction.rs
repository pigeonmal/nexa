use nexa_codegen::SourceWriter;
use nexa_ir::DirectionConfig;

use crate::generator::engine::imports::{ImportContext, ImportSet};

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    imports.add(
        context.has_direction,
        "androidx.compose.runtime.CompositionLocalProvider",
    );
    imports.add(
        context.has_direction,
        "androidx.compose.ui.platform.LocalLayoutDirection",
    );
    imports.add(
        context.has_direction,
        "androidx.compose.ui.unit.LayoutDirection",
    );
}

pub(crate) fn start(
    config: Option<DirectionConfig>,
    base_depth: usize,
    out: &mut SourceWriter,
) -> usize {
    let Some(config) = config else {
        return base_depth;
    };
    let direction = match config.style {
        nexa_ir::DirectionStyle::Ltr => "Ltr",
        nexa_ir::DirectionStyle::Rtl => "Rtl",
    };
    out.indent(base_depth);
    out.push_str("CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.");
    out.push_str(direction);
    out.push_str(") {\n");
    base_depth + 1
}

pub(crate) fn end(config: Option<DirectionConfig>, base_depth: usize, out: &mut SourceWriter) {
    if config.is_some() {
        out.push('\n');
        out.indent(base_depth);
        out.push('}');
    }
}
