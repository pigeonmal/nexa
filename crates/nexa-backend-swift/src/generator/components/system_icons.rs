use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{colors, utils::swift_string};

pub(crate) fn render(
    icon: &SystemIcon,
    description: &str,
    size: f32,
    tint: &nexa_ir::ColorExpression,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaSystemIconPrimitive(symbol: {}, description: {}, size: {}, tint: {})",
            swift_string(&icon.sf_symbol_name()),
            swift_string(description),
            size,
            colors::expression_for_color(tint),
        ),
    );
}
