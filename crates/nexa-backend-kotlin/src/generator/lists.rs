use nexa_codegen::names::state_name;
use nexa_ir::{ListSource, Module, Node};

use super::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

pub(super) fn render_virtualized_list(
    source: &ListSource,
    index: &str,
    item: Option<&str>,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("LazyColumn {\n");
    indent(out, depth + 1);
    match source {
        ListSource::Count(count) => {
            out.push_str("items(\n");
            indent(out, depth + 2);
            out.push_str(&format!(
                "count = ({}).coerceAtLeast(0),\n",
                expression(count)
            ));
            indent(out, depth + 2);
            out.push_str("key = { itemPosition -> itemPosition },\n");
            indent(out, depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
        }
        ListSource::Items {
            collection,
            element_type,
        } => {
            let item = item.unwrap_or("item");
            let collection = expression(collection);
            out.push_str("items(\n");
            indent(out, depth + 2);
            out.push_str(&format!("count = {collection}.size,\n"));
            indent(out, depth + 2);
            out.push_str("key = { itemPosition -> itemPosition },\n");
            indent(out, depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
            indent(out, depth + 2);
            out.push_str(&format!(
                "val {}: {} = {collection}[{}]\n",
                state_name(item),
                element_type.kotlin(),
                state_name(index)
            ));
        }
    }
    render_children(children, module, features, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
