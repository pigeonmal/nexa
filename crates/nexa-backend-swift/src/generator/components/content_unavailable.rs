use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, SystemIcon};

use crate::generator::engine::expressions::localized_text_view;

pub(crate) fn render(
    title: &Expr,
    icon: &SystemIcon,
    description: &Expr,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaContentUnavailablePrimitive(title: {}, symbol: {}, description: {})",
            localized_text_view(title, None),
            crate::generator::utils::swift_string(&icon.sf_symbol_name()),
            localized_text_view(description, None),
        ),
    );
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, SystemIcon};

    use super::render;

    #[test]
    fn content_unavailable_uses_shared_native_primitive() {
        let mut out = SourceWriter::new();
        render(
            &Expr::String("Nothing here".to_owned()),
            &SystemIcon::shared("inbox").expect("catalog icon"),
            &Expr::String("Add an item to begin.".to_owned()),
            0,
            &mut out,
        );
        let swift = out.finish();
        assert!(swift.contains("NexaContentUnavailablePrimitive("));
        assert!(swift.contains("symbol: \"tray.fill\""));
    }
}
