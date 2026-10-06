use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    utils::{indent, number, swift_string},
};

pub(crate) fn render(
    icon: &SystemIcon,
    description: &str,
    size: f32,
    tint: &nexa_ir::ColorExpression,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str(&format!(
        "Image(systemName: {})\n{}.font(.system(size: {}))\n{}.foregroundStyle({})",
        swift_string(&icon.sf_symbol_name()),
        "    ".repeat(depth + 1),
        number(size),
        "    ".repeat(depth + 1),
        colors::expression_for_color(tint),
    ));
    if description.is_empty() {
        out.push_str(&format!(
            "\n{}.accessibilityHidden(true)",
            "    ".repeat(depth + 1)
        ));
    } else {
        out.push_str(&format!(
            "\n{}.accessibilityLabel({})",
            "    ".repeat(depth + 1),
            swift_string(description)
        ));
    }
}
