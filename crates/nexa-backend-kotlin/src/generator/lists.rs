use nexa_codegen::names::state_name;
use nexa_ir::{Expr, ListAxis, ListSource, Module, Node};

use super::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

pub(super) fn render_virtualized_list(
    source: &ListSource,
    axis: ListAxis,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    if let ListAxis::Grid { columns } = axis {
        return render_grid_list(
            columns, source, index, item, key, children, module, features, depth, out,
        );
    }
    indent(out, depth);
    out.push_str(match axis {
        ListAxis::Vertical => "LazyColumn {\n",
        ListAxis::Horizontal => "LazyRow {\n",
        ListAxis::Grid { .. } => unreachable!("grid list handled above"),
    });
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
            let key = key
                .map(|key| render_key(key, index, item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
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
            let key = key
                .map(|key| render_key(key, index, Some(item), Some(&collection), "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
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

fn render_grid_list(
    columns: u32,
    source: &ListSource,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "LazyVerticalGrid(columns = GridCells.Fixed({columns})) {{\n"
    ));
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
            let key = key
                .map(|key| render_key(key, index, item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
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
            let key = key
                .map(|key| render_key(key, index, Some(item), Some(&collection), "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
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
