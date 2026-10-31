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
    follows_locale: bool,
    base_depth: usize,
    out: &mut SourceWriter,
) -> usize {
    if config.is_none() && !follows_locale {
        return base_depth;
    };
    let direction = match config.map(|config| config.style) {
        Some(nexa_ir::DirectionStyle::Ltr) => "LayoutDirection.Ltr".to_owned(),
        Some(nexa_ir::DirectionStyle::Rtl) => "LayoutDirection.Rtl".to_owned(),
        None => "if (NexaLocale.isRightToLeft()) LayoutDirection.Rtl else LayoutDirection.Ltr"
            .to_owned(),
    };
    out.indent(base_depth);
    out.push_str("CompositionLocalProvider(LocalLayoutDirection provides ");
    out.push_str(&direction);
    out.push_str(") {\n");
    base_depth + 1
}

pub(crate) fn end(
    config: Option<DirectionConfig>,
    follows_locale: bool,
    base_depth: usize,
    out: &mut SourceWriter,
) {
    if config.is_some() || follows_locale {
        out.push('\n');
        out.indent(base_depth);
        out.push('}');
    }
}
