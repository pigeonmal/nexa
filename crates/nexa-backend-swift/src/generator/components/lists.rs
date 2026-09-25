use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, expressions::expression,
    features::Features, utils::indent,
};

pub(crate) fn render_virtualized_list(
    plan: &ListPlan,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    match plan {
        ListPlan::Sections {
            collection,
            element_type,
            section,
            item,
            common,
        } => {
            render_sectioned_list(
                collection,
                element_type,
                common.item_extent,
                section,
                &common.index,
                item,
                common.key.as_ref(),
                &common.children,
                common.section_header.as_deref(),
                common.refresh.as_ref(),
                module,
                features,
                depth,
                out,
            );
            return;
        }
        ListPlan::Count { count, common } => {
            let key = common
                .key
                .as_ref()
                .map(|key| {
                    format!(
                        ", rowKey: {{ listPosition in AnyHashable({}) }}",
                        render_key(key, &common.index, None, None, "listPosition")
                    )
                })
                .unwrap_or_default();
            open_list(
                common.axis,
                format!("max(0, Int({}))", expression(count)),
                &key,
                common.item_extent,
                common.on_end_reached.as_deref(),
                common.on_scroll.as_deref(),
                common.scroll_position.as_deref(),
                common.sticky_header.as_deref(),
                common.refresh.as_ref(),
                module,
                features,
                depth,
                out,
            );
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: Int32 = Int32(listPosition)\n",
                state_name(&common.index)
            ));
        }
        ListPlan::Items {
            collection,
            element_type,
            item,
            common,
        } => {
            let collection = expression(collection);
            let key = common
                .key
                .as_ref()
                .map(|key| {
                    format!(
                        ", rowKey: {{ listPosition in AnyHashable({}) }}",
                        render_key(
                            key,
                            &common.index,
                            Some(item),
                            Some(&collection),
                            "listPosition"
                        )
                    )
                })
                .unwrap_or_default();
            open_list(
                common.axis,
                format!("{collection}.count"),
                &key,
                common.item_extent,
                common.on_end_reached.as_deref(),
                common.on_scroll.as_deref(),
                common.scroll_position.as_deref(),
                common.sticky_header.as_deref(),
                common.refresh.as_ref(),
                module,
                features,
                depth,
                out,
            );
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: Int32 = Int32(clamping: listPosition)\n",
                state_name(&common.index)
            ));
            indent(out, depth + 1);
            out.push_str(&format!(
                "let {}: {} = {collection}[listPosition]\n",
                state_name(item),
                element_type.swift()
            ));
        }
    }
    render_children(plan.children(), module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_sectioned_list(
    collection: &Expr,
    element_type: &nexa_ir::Type,
    item_extent: Option<f32>,
    section: &str,
    index: &str,
    item: &str,
    key: Option<&Expr>,
    children: &[Node],
    section_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let collection = expression(collection);
    indent(out, depth);
    if section_header.is_some() {
        out.push_str("NexaFastSectionedList(\n");
    } else {
        out.push_str("NexaFastSectionedList<_, EmptyView>(\n");
    }
    indent(out, depth + 1);
    out.push_str(&format!("sectionCount: {collection}.count,\n"));
    indent(out, depth + 1);
    out.push_str(&format!("sectionCounts: {collection}.map(\\.count),\n"));
    if let Some(item_extent) = item_extent {
        indent(out, depth + 1);
        out.push_str(&format!("rowHeight: {},\n", format_float(item_extent)));
    }
    if let Some(key) = key {
        indent(out, depth + 1);
        out.push_str("rowKey: { sectionPosition, itemPosition in AnyHashable(");
        out.push_str(&render_sectioned_key(
            key,
            section,
            index,
            item,
            &collection,
            "sectionPosition",
            "itemPosition",
        ));
        out.push_str(") },\n");
    }
    if let Some(refresh) = refresh {
        indent(out, depth + 1);
        out.push_str("isRefreshing: ");
        out.push_str(&state_name(&refresh.state));
        out.push_str(",\n");
        indent(out, depth + 1);
        out.push_str("onRefresh: {\n");
        render_actions(&refresh.actions, depth + 2, out);
        indent(out, depth + 1);
        out.push_str("},\n");
    }
    if let Some(header) = section_header {
        indent(out, depth + 1);
        out.push_str("headerContent: { sectionPosition in\n");
        indent(out, depth + 2);
        out.push_str("VStack(spacing: 0) {\n");
        indent(out, depth + 3);
        out.push_str(&format!(
            "let {}: Int32 = Int32(clamping: sectionPosition)\n",
            state_name(section)
        ));
        render_children(header, module, features, depth + 3, out);
        out.push('\n');
        indent(out, depth + 2);
        out.push_str("}\n");
        indent(out, depth + 1);
        out.push_str("},\n");
    }
    indent(out, depth + 1);
    out.push_str("rowContent: { sectionPosition, itemPosition in\n");
    indent(out, depth + 2);
    out.push_str(&format!(
        "let {}: Int32 = Int32(clamping: sectionPosition)\n",
        state_name(section)
    ));
    indent(out, depth + 2);
    out.push_str(&format!(
        "let {}: Int32 = Int32(clamping: itemPosition)\n",
        state_name(index)
    ));
    indent(out, depth + 2);
    out.push_str(&format!(
        "let {}: {} = {collection}[sectionPosition][itemPosition]\n",
        state_name(item),
        element_type.swift()
    ));
    render_children(children, module, features, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push(')');
}

fn list_constructor(
    axis: ListAxis,
    row_count: String,
    key: &str,
    item_extent: Option<f32>,
    has_sticky_header: bool,
) -> String {
    let extent = item_extent
        .map(format_float)
        .unwrap_or_else(|| "nil".to_owned());
    match axis {
        ListAxis::Vertical => {
            let type_arguments = if has_sticky_header {
                ""
            } else {
                "<_, EmptyView>"
            };
            format!("NexaFastList{type_arguments}(rowCount: {row_count}, rowHeight: {extent}{key})")
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
    on_scroll: Option<&[Action]>,
    scroll_position: Option<&str>,
    sticky_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    if on_end_reached.is_some()
        || on_scroll.is_some()
        || scroll_position.is_some()
        || sticky_header.is_some()
        || refresh.is_some()
    {
        let mut constructor =
            list_constructor(axis, row_count, key, item_extent, sticky_header.is_some());
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
            out.push_str("VStack(spacing: 0) {\n");
            render_children(sticky_header, module, features, depth + 2, out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("}\n");
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
        if let Some(actions) = on_scroll {
            out.push_str(", onScroll: {\n");
            render_actions(actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
        out.push(')');
    } else {
        out.push_str(&list_constructor(
            axis,
            row_count,
            key,
            item_extent,
            sticky_header.is_some(),
        ));
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

fn render_sectioned_key(
    key: &Expr,
    section: &str,
    index: &str,
    item: &str,
    collection: &str,
    section_position: &str,
    item_position: &str,
) -> String {
    let mut rendered = expression(key);
    rendered = replace_identifier(&rendered, &state_name(section), section_position);
    rendered = replace_identifier(&rendered, &state_name(index), item_position);
    rendered = replace_identifier(
        &rendered,
        &state_name(item),
        &format!("{collection}[{section_position}][{item_position}]"),
    );
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
