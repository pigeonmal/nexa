use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, SystemIcon};

use crate::generator::{engine::expressions::localized_text_view, utils::indent};

pub(crate) fn render(
    title: &Expr,
    icon: &SystemIcon,
    description: &Expr,
    depth: usize,
    out: &mut SourceWriter,
) {
    let symbol = crate::generator::utils::swift_string(&icon.sf_symbol_name());
    let title = localized_text_view(title, None);
    let description = localized_text_view(description, None);

    out.line_at(depth, format_args!("if #available(iOS 17.0, *) {{"));
    out.line_at(depth + 1, format_args!("ContentUnavailableView {{"));
    out.line_at(depth + 2, format_args!("Label {{"));
    out.line_at(depth + 3, format_args!("{title}"));
    out.line_at(depth + 2, format_args!("}} icon: {{"));
    out.line_at(depth + 3, format_args!("Image(systemName: {symbol})"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}} description: {{"));
    out.line_at(depth + 2, format_args!("{description}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(".frame(maxWidth: .infinity, maxHeight: .infinity)"),
    );
    out.line_at(depth, format_args!("}} else {{"));
    out.line_at(depth + 1, format_args!("VStack(spacing: 8) {{"));
    out.line_at(depth + 2, format_args!("Label {{"));
    out.line_at(depth + 3, format_args!("{title}"));
    out.line_at(depth + 2, format_args!("}} icon: {{"));
    out.line_at(depth + 3, format_args!("Image(systemName: {symbol})"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 2, format_args!(".font(.title3)"));
    out.line_at(
        depth + 2,
        format_args!("{description}.foregroundStyle(.secondary).multilineTextAlignment(.center)"),
    );
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(".frame(maxWidth: .infinity, maxHeight: .infinity)"),
    );
    out.line_at(depth + 1, format_args!(".padding(32)"));
    indent(out, depth);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, SystemIcon};

    use super::render;

    #[test]
    fn content_unavailable_uses_native_view_with_back_deployment_fallback() {
        let mut out = SourceWriter::new();
        render(
            &Expr::String("Nothing here".to_owned()),
            &SystemIcon::shared("inbox").expect("catalog icon"),
            &Expr::String("Add an item to begin.".to_owned()),
            0,
            &mut out,
        );
        let swift = out.finish();
        assert!(swift.contains("if #available(iOS 17.0, *)"));
        assert!(swift.contains("ContentUnavailableView {"));
        assert!(swift.contains("Image(systemName: \"tray\")"));
        assert!(swift.contains("} else {\n    VStack(spacing: 8)"));
    }
}
