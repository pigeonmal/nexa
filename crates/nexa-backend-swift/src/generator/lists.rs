use nexa_codegen::names::state_name;
use nexa_ir::{ListSource, Module, Node};

use super::{components::render_children, expressions::expression, utils::indent};

pub(super) fn render_virtualized_list(
    source: &ListSource,
    index: &str,
    item: Option<&str>,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    match source {
        ListSource::Count(count) => {
            out.push_str(&format!(
                "List(0..<max(0, Int({})), id: \\.self) {{ listPosition in\n",
                expression(count)
            ));
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: Int32 = Int32(listPosition)\n",
                state_name(index)
            ));
        }
        ListSource::Items {
            collection,
            element_type,
        } => {
            let item = item.unwrap_or("item");
            let collection = expression(collection);
            out.push_str(&format!(
                "List({collection}.indices, id: \\.self) {{ listPosition in\n"
            ));
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: Int32 = Int32(clamping: listPosition)\n",
                state_name(index)
            ));
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: {} = {collection}[listPosition]\n",
                state_name(item),
                element_type.swift()
            ));
        }
    }
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
