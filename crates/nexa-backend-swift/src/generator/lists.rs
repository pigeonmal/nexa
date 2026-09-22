use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListSource, Module, Node};

use super::{
    components::render_children, controls::render_actions, expressions::expression, utils::indent,
};

pub(super) fn render_virtualized_list(
    source: &ListSource,
    axis: ListAxis,
    item_extent: Option<f32>,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    on_end_reached: Option<&[Action]>,
    scroll_position: Option<&str>,
    sticky_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
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
            open_list(
                axis,
                format!("max(0, Int({}))", expression(count)),
                &key,
                item_extent,
                on_end_reached,
                scroll_position,
                sticky_header,
                refresh,
                module,
                depth,
                out,
            );
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
            open_list(
                axis,
                format!("{collection}.count"),
                &key,
                item_extent,
                on_end_reached,
                scroll_position,
                sticky_header,
                refresh,
                module,
                depth,
                out,
            );
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

fn list_constructor(
    axis: ListAxis,
    row_count: String,
    key: &str,
    item_extent: Option<f32>,
) -> String {
    let extent = item_extent
        .map(format_float)
        .unwrap_or_else(|| "nil".to_owned());
    match axis {
        ListAxis::Vertical => {
            format!("NexaFastList(rowCount: {row_count}, rowHeight: {extent}{key})")
        }
        ListAxis::Horizontal => {
            format!("NexaFastHorizontalList(rowCount: {row_count}, itemExtent: {extent}{key})")
        }
        ListAxis::Grid { columns } => {
            format!(
                "NexaFastGridList(rowCount: {row_count}, columns: {columns}, itemHeight: {extent}{key})"
            )
        }
    }
}

fn open_list(
    axis: ListAxis,
    row_count: String,
    key: &str,
    item_extent: Option<f32>,
    on_end_reached: Option<&[Action]>,
    scroll_position: Option<&str>,
    sticky_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    if on_end_reached.is_some()
        || scroll_position.is_some()
        || sticky_header.is_some()
        || refresh.is_some()
    {
        let mut constructor = list_constructor(axis, row_count, key, item_extent);
        constructor.pop();
        out.push_str(&constructor);
        if let Some(scroll_position) = scroll_position {
            out.push_str(", scrollPosition: ");
            out.push_str(&state_name(scroll_position));
            out.push_str(", onScrollPositionChanged: { position in\n");
            indent(out, depth + 1);
            out.push_str(&format!(
                "{} = Int32(position)\n",
                state_name(scroll_position)
            ));
            indent(out, depth);
            out.push('}');
        }
        if let Some(sticky_header) = sticky_header {
            debug_assert!(matches!(axis, ListAxis::Vertical));
            out.push_str(", headerContent: {\n");
            indent(out, depth + 1);
            out.push_str("AnyView(VStack(spacing: 0) {\n");
            render_children(sticky_header, module, depth + 2, out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("})\n");
            indent(out, depth);
            out.push('}');
        }
        if let Some(refresh) = refresh {
            out.push_str(", isRefreshing: ");
            out.push_str(&state_name(&refresh.state));
            out.push_str(", onRefresh: {\n");
            render_actions(&refresh.actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
        if let Some(actions) = on_end_reached {
            out.push_str(", onEndReached: {\n");
            render_actions(actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
        out.push(')');
    } else {
        out.push_str(&list_constructor(axis, row_count, key, item_extent));
    }
    out.push_str(" { listPosition in\n");
}

fn format_float(value: f32) -> String {
    let mut formatted = format!("{value:.6}");
    while formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
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
