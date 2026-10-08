use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, engine::types::swift_type,
    expressions::expression, features::Features, utils::indent,
};

use super::RenderScope;

struct SectionedPieces<'a> {
    collection: &'a Expr,
    element_type: &'a nexa_ir::Type,
    native: bool,
    item_extent: Option<f32>,
    section: &'a str,
    index: &'a str,
    item: &'a str,
    key: Option<&'a Expr>,
    children: &'a [Node],
    swipe_actions: Option<&'a [Node]>,
    section_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
}

struct OpenListConfig<'a> {
    axis: ListAxis,
    native: bool,
    row_count: String,
    key: String,
    item_extent: Option<f32>,
    on_end_reached: Option<&'a [Action]>,
    on_scroll: Option<&'a [Action]>,
    scroll_position: Option<&'a str>,
    reverse_layout: bool,
    page_snap: bool,
    sticky_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
}

pub(crate) fn render_virtualized_list(
    plan: &ListPlan,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
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
                SectionedPieces {
                    collection,
                    element_type,
                    native: common.native,
                    item_extent: common.item_extent,
                    section,
                    index: &common.index,
                    item,
                    key: common.key.as_ref(),
                    children: &common.children,
                    swipe_actions: common.swipe_actions.as_deref(),
                    section_header: common.section_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                },
                &RenderScope { module, features },
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
                OpenListConfig {
                    axis: common.axis,
                    native: common.native,
                    row_count: format!("max(0, Int({}))", expression(count)),
                    key,
                    item_extent: common.item_extent,
                    on_end_reached: common.on_end_reached.as_deref(),
                    on_scroll: common.on_scroll.as_deref(),
                    scroll_position: common.scroll_position.as_deref(),
                    reverse_layout: common.reverse_layout,
                    page_snap: common.page_snap,
                    sticky_header: common.sticky_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                },
                &RenderScope { module, features },
                depth,
                out,
            );
            if row_references_binding(&common.children, &common.index) {
                out.line_at(
                    depth + 1,
                    format_args!(
                        "let {}: Int32 = Int32(listPosition)",
                        state_name(&common.index)
                    ),
                );
            }
        }
        ListPlan::Items {
            collection,
            element_type,
            item,
            common,
        } => {
            let collection_expression = expression(collection);
            let collection = format!("nexa_list_values_{}", out.next_id());
            out.line_at(
                depth,
                format_args!(
                    "let {collection}: [{}] = {collection_expression}",
                    swift_type(element_type)
                ),
            );
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
                OpenListConfig {
                    axis: common.axis,
                    native: common.native,
                    row_count: format!("{collection}.count"),
                    key,
                    item_extent: common.item_extent,
                    on_end_reached: common.on_end_reached.as_deref(),
                    on_scroll: common.on_scroll.as_deref(),
                    scroll_position: common.scroll_position.as_deref(),
                    reverse_layout: common.reverse_layout,
                    page_snap: common.page_snap,
                    sticky_header: common.sticky_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                },
                &RenderScope { module, features },
                depth,
                out,
            );
            if row_references_binding(&common.children, &common.index) {
                out.line_at(
                    depth + 1,
                    format_args!(
                        "let {}: Int32 = Int32(clamping: listPosition)",
                        state_name(&common.index)
                    ),
                );
            }
            out.line_at(
                depth + 1,
                format_args!(
                    "let {}: {} = {collection}[listPosition]",
                    state_name(item),
                    swift_type(element_type)
                ),
            );
        }
    }
    let native = match plan {
        ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.native,
        ListPlan::Sections { .. } => false,
    };
    render_children(
        plan.children(),
        module,
        features,
        depth + if native { 2 } else { 1 },
        out,
    );
    if let Some(actions) = plan.swipe_actions() {
        out.push('\n');
        indent(out, depth + 2);
        out.push_str(".swipeActions(edge: .trailing) {\n");
        render_children(actions, module, features, depth + 3, out);
        out.push('\n');
        indent(out, depth + 2);
        out.push('}');
    }
    out.push('\n');
    if native {
        if let Some(on_move) = plan.on_move() {
            indent(out, depth + 1);
            out.push_str("}.onMove { source, destination in\n");
            out.line_at(
                depth + 2,
                format_args!(
                    "let {}: Int32 = Int32(source.first ?? 0)",
                    state_name(&on_move.from)
                ),
            );
            out.line_at(
                depth + 2,
                format_args!(
                    "let {}: Int32 = Int32(destination > (source.first ?? 0) ? destination - 1 : destination)",
                    state_name(&on_move.to)
                ),
            );
            render_actions(&on_move.actions, depth + 2, out);
            out.push('\n');
            indent(out, depth + 1);
            out.push_str("}\n");
            out.line_at(
                depth + 1,
                format_args!(".moveDisabled(!({}))", expression(&on_move.enabled)),
            );
        } else {
            indent(out, depth + 1);
            out.push_str("}\n");
        }
    }
    indent(out, depth);
    out.push('}');
}

fn row_references_binding(children: &[Node], binding: &str) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_ir(children, &mut |_| {}, &mut |expression| {
        if matches!(
            expression,
            Expr::State(name, _) | Expr::AnimatedState(name, _) if name == binding
        ) {
            found = true;
        }
    });
    found
}

fn render_sectioned_list(
    pieces: SectionedPieces<'_>,
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let SectionedPieces {
        collection,
        element_type,
        native,
        item_extent,
        section,
        index,
        item,
        key,
        children,
        swipe_actions,
        section_header,
        refresh,
    } = pieces;
    let collection_expression = expression(collection);
    let collection = format!("nexa_list_sections_{}", out.next_id());
    out.line_at(
        depth,
        format_args!(
            "let {collection}: [[{}]] = {collection_expression}",
            swift_type(element_type)
        ),
    );
    if native {
        indent(out, depth);
        out.push_str("List {\n");
        out.line_at(
            depth + 1,
            format_args!("ForEach({collection}.indices, id: \\.self) {{ sectionPosition in"),
        );
        out.line_at(
            depth + 2,
            format_args!(
                "let {}: [{}] = {collection}[sectionPosition]",
                state_name("sectionItems"),
                swift_type(element_type)
            ),
        );
        out.line_at(depth + 2, format_args!("Section {{"));
        out.line_at(
            depth + 3,
            format_args!(
                "ForEach({}.indices, id: \\.self) {{ itemPosition in",
                state_name("sectionItems")
            ),
        );
        out.line_at(
            depth + 4,
            format_args!(
                "let {}: Int32 = Int32(clamping: sectionPosition)",
                state_name(section)
            ),
        );
        out.line_at(
            depth + 4,
            format_args!(
                "let {}: Int32 = Int32(clamping: itemPosition)",
                state_name(index)
            ),
        );
        out.line_at(
            depth + 4,
            format_args!(
                "let {}: {} = {}[itemPosition]",
                state_name(item),
                swift_type(element_type),
                state_name("sectionItems")
            ),
        );
        out.line_at(depth + 4, format_args!("VStack(spacing: 0) {{"));
        render_children(children, scope.module, scope.features, depth + 5, out);
        out.push('\n');
        indent(out, depth + 4);
        out.push('}');
        if let Some(actions) = swipe_actions {
            out.push_str(".swipeActions(edge: .trailing) {\n");
            render_children(actions, scope.module, scope.features, depth + 5, out);
            out.push('\n');
            indent(out, depth + 4);
            out.push('}');
        }
        out.push('\n');
        indent(out, depth + 3);
        out.push_str("}\n");
        indent(out, depth + 2);
        if let Some(header) = section_header {
            out.push_str("} header: {\n");
            out.line_at(
                depth + 3,
                format_args!(
                    "let {}: [{}] = {collection}[sectionPosition]",
                    state_name("sectionItems"),
                    swift_type(element_type)
                ),
            );
            out.line_at(
                depth + 3,
                format_args!(
                    "let {}: Int32 = Int32(clamping: sectionPosition)",
                    state_name(section)
                ),
            );
            out.line_at(depth + 3, format_args!("VStack(spacing: 0) {{"));
            render_children(header, scope.module, scope.features, depth + 4, out);
            out.push('\n');
            indent(out, depth + 3);
            out.push_str("}\n");
            indent(out, depth + 2);
            out.push_str("}\n");
        } else {
            out.push_str("}\n");
        }
        indent(out, depth + 1);
        out.push_str("}\n");
        indent(out, depth);
        out.push('}');
        return;
    }
    indent(out, depth);
    if section_header.is_some() {
        out.push_str("NexaFastSectionedList(\n");
    } else {
        out.push_str("NexaFastSectionedList<_, EmptyView>(\n");
    }
    out.line_at(depth + 1, format_args!("sectionCount: {collection}.count,"));
    out.line_at(
        depth + 1,
        format_args!("sectionCounts: {collection}.map(\\.count),"),
    );
    if let Some(item_extent) = item_extent {
        out.line_at(
            depth + 1,
            format_args!("rowHeight: {},", format_float(item_extent)),
        );
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
        out.line_at(
            depth + 2,
            format_args!(
                "let {}: [{}] = {collection}[sectionPosition]",
                state_name("sectionItems"),
                swift_type(element_type)
            ),
        );
        indent(out, depth + 2);
        out.push_str("VStack(spacing: 0) {\n");
        out.line_at(
            depth + 3,
            format_args!(
                "let {}: Int32 = Int32(clamping: sectionPosition)",
                state_name(section)
            ),
        );
        render_children(header, scope.module, scope.features, depth + 3, out);
        out.push('\n');
        indent(out, depth + 2);
        out.push_str("}\n");
        indent(out, depth + 1);
        out.push_str("},\n");
    }
    indent(out, depth + 1);
    out.push_str("rowContent: { sectionPosition, itemPosition in\n");
    out.line_at(
        depth + 2,
        format_args!(
            "let {}: Int32 = Int32(clamping: sectionPosition)",
            state_name(section)
        ),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "let {}: Int32 = Int32(clamping: itemPosition)",
            state_name(index)
        ),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "let {}: {} = {collection}[sectionPosition][itemPosition]",
            state_name(item),
            swift_type(element_type)
        ),
    );
    render_children(children, scope.module, scope.features, depth + 2, out);
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
    _has_sticky_header: bool,
    reverse_layout: bool,
    page_snap: bool,
) -> String {
    let extent = item_extent
        .map(format_float)
        .unwrap_or_else(|| "nil".to_owned());
    match axis {
        ListAxis::Vertical => {
            let reverse = if reverse_layout {
                ", reverseLayout: true"
            } else {
                ""
            };
            let page_snap = if page_snap { ", pageSnap: true" } else { "" };
            format!(
                "NexaFastList(rowCount: {row_count}, rowHeight: {extent}{key}{reverse}{page_snap})"
            )
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
    config: OpenListConfig<'_>,
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let OpenListConfig {
        axis,
        native,
        row_count,
        key,
        item_extent,
        on_end_reached,
        on_scroll,
        scroll_position,
        reverse_layout,
        page_snap,
        sticky_header,
        refresh,
    } = config;
    if native {
        out.line_at(depth, format_args!("List {{"));
        out.line_at(
            depth + 1,
            format_args!("ForEach(0..<({row_count}), id: \\.self) {{ (listPosition: Int) in"),
        );
        return;
    }
    if on_end_reached.is_some()
        || on_scroll.is_some()
        || scroll_position.is_some()
        || reverse_layout
        || page_snap
        || sticky_header.is_some()
        || refresh.is_some()
    {
        let mut constructor = list_constructor(
            axis,
            row_count,
            &key,
            item_extent,
            sticky_header.is_some(),
            reverse_layout,
            page_snap,
        );
        constructor.pop();
        out.push_str(&constructor);
        if let Some(scroll_position) = scroll_position {
            out.push_str(", scrollPosition: ");
            out.push_str(&state_name(scroll_position));
            out.push_str(", onScrollPositionChanged: { position in\n");
            out.line_at(
                depth + 1,
                format_args!("{} = Int32(position)", state_name(scroll_position)),
            );
            indent(out, depth);
            out.push('}');
        }
        if let Some(sticky_header) = sticky_header {
            debug_assert!(matches!(axis, ListAxis::Vertical));
            out.push_str(", headerContent: {\n");
            indent(out, depth + 1);
            out.push_str("VStack(spacing: 0) {\n");
            render_children(sticky_header, scope.module, scope.features, depth + 2, out);
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
            &key,
            item_extent,
            sticky_header.is_some(),
            reverse_layout,
            page_snap,
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

#[cfg(test)]
mod tests {
    use nexa_ir::{Expr, Node, NumericType, TextStyle, Type};

    use super::row_references_binding;

    #[test]
    fn list_index_binding_is_only_emitted_when_the_row_uses_it() {
        let plain_row = [Node::Text {
            value: Expr::String("Quick dates".to_owned()),
            style: TextStyle::default(),
        }];
        assert!(!row_references_binding(&plain_row, "index"));

        let indexed_row = [Node::Text {
            value: Expr::State("index".to_owned(), Type::Numeric(NumericType::Int32)),
            style: TextStyle::default(),
        }];
        assert!(row_references_binding(&indexed_row, "index"));
    }
}
