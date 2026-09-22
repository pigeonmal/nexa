use nexa_codegen::names::state_name;
use nexa_ir::{Expr, ListAxis, ListSource, Module, Node};

use super::{components::render_children, expressions::expression, utils::indent};

pub(super) fn render_virtualized_list(
    source: &ListSource,
    axis: ListAxis,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    let list_view = match axis {
        ListAxis::Vertical => "NexaFastList",
        ListAxis::Horizontal => "NexaFastHorizontalList",
    };
    match source {
        ListSource::Count(count) => {
            let key = key
                .map(|key| {
                    format!(
                        ", rowKey: {{ listPosition in AnyHashable({}) }}",
                        render_key(key, index, item, None, "listPosition")
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "{list_view}(rowCount: max(0, Int({})){}) {{ listPosition in\n",
                expression(count),
                key
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
            let key = key
                .map(|key| {
                    format!(
                        ", rowKey: {{ listPosition in AnyHashable({}) }}",
                        render_key(key, index, Some(item), Some(&collection), "listPosition")
                    )
                })
                .unwrap_or_default();
            out.push_str(&format!(
                "{list_view}(rowCount: {collection}.count{key}) {{ listPosition in\n"
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

fn render_key(
    key: &Expr,
    index: &str,
    item: Option<&str>,
    collection: Option<&str>,
    position_name: &str,
) -> String {
    let mut rendered = expression(key);
    rendered = replace_identifier(&rendered, &state_name(index), position_name);
    if let (Some(collection), Some(item)) = (collection, item) {
        rendered = replace_identifier(
            &rendered,
            &state_name(item),
            &format!("{collection}[{position_name}]"),
        );
    }
    rendered
}

fn replace_identifier(source: &str, identifier: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find(identifier) {
        let start = cursor + relative;
        let end = start + identifier.len();
        let before = source[..start].chars().next_back();
        let after = source[end..].chars().next();
        let is_boundary = |character: Option<char>| {
            character
                .is_none_or(|character| !(character.is_ascii_alphanumeric() || character == '_'))
        };
        if is_boundary(before) && is_boundary(after) {
            result.push_str(&source[cursor..start]);
            result.push_str(replacement);
            cursor = end;
        } else {
            result.push_str(&source[cursor..end]);
            cursor = end;
        }
    }
    result.push_str(&source[cursor..]);
    result
}
