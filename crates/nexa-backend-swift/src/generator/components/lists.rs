use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::render_children,
    controls::{render_actions, render_actions_in_task_scope},
    engine::types::swift_type,
    expressions::expression,
    features::Features,
    utils::indent,
};

use super::RenderScope;

struct SectionedPieces<'a> {
    collection: &'a Expr,
    element_type: &'a nexa_ir::Type,
    uses_platform_list: bool,
    show_dividers: bool,
    item_extent: Option<f32>,
    spacing: f32,
    scroll_enabled: Option<&'a Expr>,
    section: &'a str,
    index: &'a str,
    item: &'a str,
    key: Option<&'a Expr>,
    children: &'a [Node],
    swipe_actions: Option<&'a [Node]>,
    section_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
    component_ref: Option<&'a Expr>,
}

struct OpenListConfig<'a> {
    axis: ListAxis,
    uses_platform_list: bool,
    reorder_enabled: bool,
    show_dividers: bool,
    row_count: String,
    row_key: Option<String>,
    item_extent: Option<f32>,
    spacing: f32,
    scroll_enabled: Option<&'a Expr>,
    on_end_reached: Option<&'a [Action]>,
    on_scroll: Option<&'a [Action]>,
    scroll_position: Option<&'a str>,
    reverse_layout: bool,
    page_snap: bool,
    sticky_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
    component_ref: Option<&'a Expr>,
}

struct ListConstructorConfig<'a> {
    axis: ListAxis,
    row_count: &'a str,
    row_key: Option<&'a str>,
    item_extent: Option<f32>,
    spacing: f32,
    scroll_enabled: &'a str,
    show_dividers: bool,
    reverse_layout: bool,
    page_snap: bool,
    component_ref: Option<&'a Expr>,
}

fn render_swipe_action_content(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    for node in nodes {
        if matches!(node, Node::Button { .. }) {
            super::controls::render_swipe_action_button(node, depth, out);
        } else {
            render_children(std::slice::from_ref(node), module, features, depth, out);
        }
    }
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
                    uses_platform_list: common.swipe_actions.is_some(),
                    show_dividers: common.show_dividers,
                    item_extent: common.item_extent,
                    spacing: common.spacing,
                    scroll_enabled: common.scroll_enabled.as_ref(),
                    section,
                    index: &common.index,
                    item,
                    key: common.key.as_ref(),
                    children: &common.children,
                    swipe_actions: common.swipe_actions.as_deref(),
                    section_header: common.section_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                    component_ref: common.component_ref.as_ref(),
                },
                &RenderScope { module, features },
                depth,
                out,
            );
            return;
        }
        ListPlan::Count { count, common } => {
            let uses_platform_list = common.swipe_actions.is_some();
            let row_key = common.key.as_ref().map(|key| {
                format!(
                    "AnyHashable({})",
                    render_key(key, &common.index, None, None, "listPosition")
                )
            });
            open_list(
                OpenListConfig {
                    axis: common.axis,
                    uses_platform_list,
                    reorder_enabled: common.on_move.is_some(),
                    show_dividers: common.show_dividers,
                    row_count: format!("max(0, Int({}))", expression(count)),
                    row_key,
                    item_extent: common.item_extent,
                    spacing: common.spacing,
                    scroll_enabled: common.scroll_enabled.as_ref(),
                    on_end_reached: common.on_end_reached.as_deref(),
                    on_scroll: common.on_scroll.as_deref(),
                    scroll_position: common.scroll_position.as_deref(),
                    reverse_layout: common.reverse_layout,
                    page_snap: common.page_snap,
                    sticky_header: common.sticky_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                    component_ref: common.component_ref.as_ref(),
                },
                &RenderScope { module, features },
                depth,
                out,
            );
            if row_references_binding(&common.children, &common.index) {
                out.line_at(
                    depth + if uses_platform_list { 2 } else { 1 },
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
            let uses_platform_list = common.on_move.is_some() || common.swipe_actions.is_some();
            let collection_expression = expression(collection);
            let collection = format!("nexa_list_values_{}", out.next_id());
            out.line_at(
                depth,
                format_args!(
                    "let {collection}: [{}] = {collection_expression}",
                    swift_type(element_type)
                ),
            );
            let row_key = common.key.as_ref().map(|key| {
                format!(
                    "AnyHashable({})",
                    render_key(
                        key,
                        &common.index,
                        Some(item),
                        Some(&collection),
                        "listPosition"
                    )
                )
            });
            open_list(
                OpenListConfig {
                    axis: common.axis,
                    uses_platform_list,
                    reorder_enabled: common.on_move.is_some(),
                    show_dividers: common.show_dividers,
                    row_count: format!("{collection}.count"),
                    row_key,
                    item_extent: common.item_extent,
                    spacing: common.spacing,
                    scroll_enabled: common.scroll_enabled.as_ref(),
                    on_end_reached: common.on_end_reached.as_deref(),
                    on_scroll: common.on_scroll.as_deref(),
                    scroll_position: common.scroll_position.as_deref(),
                    reverse_layout: common.reverse_layout,
                    page_snap: common.page_snap,
                    sticky_header: common.sticky_header.as_deref(),
                    refresh: common.refresh.as_ref(),
                    component_ref: common.component_ref.as_ref(),
                },
                &RenderScope { module, features },
                depth,
                out,
            );
            if row_references_binding(&common.children, &common.index) {
                out.line_at(
                    depth + if uses_platform_list { 2 } else { 1 },
                    format_args!(
                        "let {}: Int32 = Int32(clamping: listPosition)",
                        state_name(&common.index)
                    ),
                );
            }
            out.line_at(
                depth + if uses_platform_list { 2 } else { 1 },
                format_args!(
                    "let {}: {} = {collection}[listPosition]",
                    state_name(item),
                    swift_type(element_type)
                ),
            );
        }
    }
    let uses_platform_list = plan.uses_platform_list();
    let show_dividers = match plan {
        ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.show_dividers,
        ListPlan::Sections { .. } => true,
    };
    let prioritize_swipe_actions = plan.swipe_actions().is_some();
    let reorder = plan.on_move();
    if uses_platform_list {
        out.line_at(
            depth + 2,
            format_args!("NexaNativeListRowPrimitive(position: listPosition, showDividers: {show_dividers}, prioritizeSwipeActions: {prioritize_swipe_actions}, reorderIndex: {}, reorderEnabled: {}, onReorder: {{ source, destination in", reorder.map(|_| "listPosition").unwrap_or("nil"), reorder.map(|move_list| expression(&move_list.enabled)).unwrap_or_else(|| "false".to_owned())),
        );
        if let Some(on_move) = reorder {
            out.line_at(
                depth + 3,
                format_args!("let {}: Int32 = Int32(source)", state_name(&on_move.from)),
            );
            out.line_at(
                depth + 3,
                format_args!(
                    "let {}: Int32 = Int32(destination > source ? destination - 1 : destination)",
                    state_name(&on_move.to)
                ),
            );
            render_actions(&on_move.actions, depth + 3, out);
            out.line_at(depth + 2, format_args!("}}) {{"));
        } else {
            out.line_at(depth + 2, format_args!("}}) {{"));
        }
    }
    render_children(
        plan.children(),
        module,
        features,
        depth + if uses_platform_list { 3 } else { 1 },
        out,
    );
    if uses_platform_list {
        out.push('\n');
        indent(out, depth + 2);
        out.push('}');
    }
    if let Some(actions) = plan.swipe_actions() {
        out.push('\n');
        indent(out, depth + 2);
        out.push_str(".swipeActions(edge: .trailing, allowsFullSwipe: false) {\n");
        if let Some(on_move) = plan.on_move() {
            out.line_at(
                depth + 3,
                format_args!("if !({}) {{", expression(&on_move.enabled)),
            );
            render_swipe_action_content(actions, module, features, depth + 4, out);
            out.line_at(depth + 3, format_args!("}}"));
        } else {
            render_swipe_action_content(actions, module, features, depth + 3, out);
        }
        out.push('\n');
        indent(out, depth + 2);
        out.push('}');
    }
    out.push('\n');
    if uses_platform_list {
        indent(out, depth + 1);
        out.push_str("}\n");
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
        uses_platform_list,
        show_dividers,
        item_extent,
        spacing,
        scroll_enabled,
        section,
        index,
        item,
        key,
        children,
        swipe_actions,
        section_header,
        refresh,
        component_ref,
    } = pieces;
    let scroll_enabled = scroll_enabled
        .map(expression)
        .unwrap_or_else(|| "true".to_owned());
    let collection_expression = expression(collection);
    let collection = format!("nexa_list_sections_{}", out.next_id());
    out.line_at(
        depth,
        format_args!(
            "let {collection}: [[{}]] = {collection_expression}",
            swift_type(element_type)
        ),
    );
    let section_starts = format!("nexa_list_section_starts_{}", out.next_id());
    if uses_platform_list {
        out.line_at(depth, format_args!("let {section_starts}: [Int] = {{"));
        out.line_at(depth + 1, format_args!("var starts: [Int] = []"));
        out.line_at(
            depth + 1,
            format_args!("starts.reserveCapacity({collection}.count)"),
        );
        out.line_at(depth + 1, format_args!("var total = 0"));
        out.line_at(
            depth + 1,
            format_args!("for sectionItems in {collection} {{"),
        );
        out.line_at(depth + 2, format_args!("starts.append(total)"));
        out.line_at(depth + 2, format_args!("total += sectionItems.count"));
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth + 1, format_args!("return starts"));
        out.line_at(depth, format_args!("}}()"));
    }
    if uses_platform_list {
        let item_identity = key.map(|key| {
            render_sectioned_key(
                key,
                section,
                index,
                item,
                &collection,
                "sectionPosition",
                "itemPosition",
            )
        });
        indent(out, depth);
        out.push_str("NexaNativeListPrimitive(");
        out.push_str(&format!(
            "spacing: {}, scrollEnabled: {scroll_enabled}, rowCount: {collection}.reduce(0) {{ $0 + $1.count }}, componentRef: {}",
            format_float(spacing),
            component_ref
                .map(expression)
                .unwrap_or_else(|| "nil".to_owned())
        ));
        out.push_str(") {\n");
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
        let identified_items = item_identity.map(|identity| {
            let rows = format!("nexa_section_rows_{}", out.next_id());
            out.line_at(
                depth + 2,
                format_args!(
                    "let {rows} = {}.indices.map {{ itemPosition in",
                    state_name("sectionItems")
                ),
            );
            out.line_at(
                depth + 3,
                format_args!(
                    "NexaIdentifiedListRow(id: AnyHashable({identity}), position: itemPosition)"
                ),
            );
            out.line_at(depth + 2, format_args!("}}"));
            rows
        });
        out.line_at(depth + 2, format_args!("Section {{"));
        if let Some(rows) = identified_items {
            out.line_at(
                depth + 3,
                format_args!("ForEach({rows}) {{ nexaSectionRow in"),
            );
            out.line_at(
                depth + 4,
                format_args!("let itemPosition = nexaSectionRow.position"),
            );
        } else {
            out.line_at(
                depth + 3,
                format_args!(
                    "ForEach({}.indices, id: \\.self) {{ itemPosition in",
                    state_name("sectionItems")
                ),
            );
        }
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
        out.line_at(
            depth + 4,
            format_args!("NexaNativeListRowPrimitive(position: {section_starts}[sectionPosition] + itemPosition, showDividers: {show_dividers}, prioritizeSwipeActions: {}) {{", swipe_actions.is_some()),
        );
        out.line_at(depth + 5, format_args!("VStack(spacing: 0) {{"));
        render_children(children, scope.module, scope.features, depth + 6, out);
        out.push('\n');
        indent(out, depth + 5);
        out.push('}');
        out.push('\n');
        indent(out, depth + 4);
        out.push('}');
        if let Some(actions) = swipe_actions {
            out.push_str(".swipeActions(edge: .trailing, allowsFullSwipe: false) {\n");
            render_swipe_action_content(actions, scope.module, scope.features, depth + 5, out);
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
        out.push_str("NexaFastSectionedListPrimitive(\n");
    } else {
        out.push_str("NexaFastSectionedListPrimitive<_, EmptyView>(\n");
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
    if spacing > 0.0 {
        out.line_at(
            depth + 1,
            format_args!("spacing: {},", format_float(spacing)),
        );
    }
    out.line_at(depth + 1, format_args!("showDividers: {show_dividers},"));
    out.line_at(depth + 1, format_args!("scrollEnabled: {scroll_enabled},"));
    if let Some(component_ref) = component_ref {
        out.line_at(
            depth + 1,
            format_args!("componentRef: {},", expression(component_ref)),
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
        out.push_str("return VStack(spacing: 0) {\n");
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

fn list_constructor(config: ListConstructorConfig<'_>) -> String {
    let ListConstructorConfig {
        axis,
        row_count,
        row_key,
        item_extent,
        spacing,
        scroll_enabled,
        show_dividers,
        reverse_layout,
        page_snap,
        component_ref,
    } = config;
    let key = row_key.map_or_else(String::new, |key| {
        format!(", rowKey: {{ listPosition in {key} }}")
    });
    let extent = item_extent
        .map(format_float)
        .unwrap_or_else(|| "nil".to_owned());
    let spacing_argument = if spacing > 0.0 {
        format!(", spacing: {}", format_float(spacing))
    } else {
        String::new()
    };
    let scroll_enabled_argument = format!(", scrollEnabled: {scroll_enabled}");
    let component_ref_argument = component_ref
        .map(|reference| format!(", componentRef: {}", expression(reference)))
        .unwrap_or_default();
    match axis {
        ListAxis::Vertical => {
            let reverse = if reverse_layout {
                ", reverseLayout: true"
            } else {
                ""
            };
            let page_snap = if page_snap { ", pageSnap: true" } else { "" };
            format!(
                "NexaFastListPrimitive(rowCount: {row_count}, rowHeight: {extent}{spacing_argument}, showDividers: {show_dividers}{scroll_enabled_argument}{key}{reverse}{page_snap}{component_ref_argument})"
            )
        }
        ListAxis::Horizontal => {
            format!(
                "NexaFastHorizontalListPrimitive(rowCount: {row_count}, itemExtent: {extent}{spacing_argument}{scroll_enabled_argument}{key}{component_ref_argument})"
            )
        }
        ListAxis::Grid { columns } => {
            format!(
                "NexaFastGridListPrimitive(rowCount: {row_count}, columns: {columns}, itemHeight: {extent}{spacing_argument}{scroll_enabled_argument}{key}{component_ref_argument})"
            )
        }
        ListAxis::AutoGrid { min_cell_width } => {
            format!(
                "NexaFastGridListPrimitive(rowCount: {row_count}, columns: 1, minCellWidth: {min_cell_width}, itemHeight: {extent}{spacing_argument}{scroll_enabled_argument}{key}{component_ref_argument})"
            )
        }
        ListAxis::Masonry { columns } => {
            format!(
                "NexaFastGridListPrimitive(rowCount: {row_count}, columns: {columns}, itemHeight: {extent}{spacing_argument}{scroll_enabled_argument}{key}{component_ref_argument}, masonry: true)"
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
        uses_platform_list,
        reorder_enabled,
        show_dividers,
        row_count,
        row_key,
        item_extent,
        spacing,
        scroll_enabled,
        on_end_reached,
        on_scroll,
        scroll_position,
        reverse_layout,
        page_snap,
        sticky_header,
        refresh,
        component_ref,
    } = config;
    let scroll_enabled = scroll_enabled
        .map(expression)
        .unwrap_or_else(|| "true".to_owned());
    if uses_platform_list {
        let identified_rows = row_key.map(|identity| {
            let rows = format!("nexa_list_rows_{}", out.next_id());
            out.line_at(
                depth,
                format_args!("let {rows} = (0..<({row_count})).map {{ listPosition in"),
            );
            out.line_at(
                depth + 1,
                format_args!("NexaIdentifiedListRow(id: {identity}, position: listPosition)"),
            );
            out.line_at(depth, format_args!("}}"));
            rows
        });
        let reorder_argument = if reorder_enabled {
            ", reorderEnabled: true"
        } else {
            ""
        };
        let arguments = if spacing > 0.0 {
            format!(
                "spacing: {}, scrollEnabled: {scroll_enabled}{reorder_argument}, rowCount: {row_count}{}",
                format_float(spacing),
                component_ref
                    .map(|reference| format!(", componentRef: {}", expression(reference)))
                    .unwrap_or_default(),
            )
        } else {
            format!(
                "scrollEnabled: {scroll_enabled}{reorder_argument}, rowCount: {row_count}{}",
                component_ref
                    .map(|reference| format!(", componentRef: {}", expression(reference)))
                    .unwrap_or_default(),
            )
        };
        out.line_at(
            depth,
            format_args!("NexaNativeListPrimitive({arguments}) {{"),
        );
        if let Some(rows) = identified_rows {
            out.line_at(depth + 1, format_args!("ForEach({rows}) {{ nexaListRow in"));
            out.line_at(
                depth + 2,
                format_args!("let listPosition = nexaListRow.position"),
            );
        } else {
            out.line_at(
                depth + 1,
                format_args!("ForEach(0..<({row_count}), id: \\.self) {{ (listPosition: Int) in"),
            );
        }
        return;
    }
    if on_end_reached.is_some()
        || on_scroll.is_some()
        || scroll_position.is_some()
        || reverse_layout
        || page_snap
        || spacing > 0.0
        || sticky_header.is_some()
        || refresh.is_some()
        || component_ref.is_some()
    {
        let mut constructor = list_constructor(ListConstructorConfig {
            axis,
            row_count: &row_count,
            row_key: row_key.as_deref(),
            item_extent,
            spacing,
            scroll_enabled: &scroll_enabled,
            show_dividers,
            reverse_layout,
            page_snap,
            component_ref: None,
        });
        constructor.pop();
        out.push_str(&constructor);
        if let Some(scroll_position) = scroll_position {
            out.push_str(", scrollPosition: ");
            out.push_str(&state_name(scroll_position));
        }
        if let Some(component_ref) = component_ref {
            out.push_str(", componentRef: ");
            out.push_str(&expression(component_ref));
        }
        if let Some(scroll_position) = scroll_position {
            out.push_str(", onScrollPositionChanged: { position in\n");
            out.line_at(
                depth + 1,
                format_args!("{} = Int32(position)", state_name(scroll_position)),
            );
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
            render_actions_in_task_scope(actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
        if let Some(actions) = on_scroll {
            out.push_str(", onScroll: {\n");
            render_actions(actions, depth + 1, out);
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
        out.push(')');
    } else {
        out.push_str(&list_constructor(ListConstructorConfig {
            axis,
            row_count: &row_count,
            row_key: row_key.as_deref(),
            item_extent,
            spacing,
            scroll_enabled: &scroll_enabled,
            show_dividers,
            reverse_layout,
            page_snap,
            component_ref,
        }));
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
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, ListAxis, Module, Node, NumericType, TextStyle, Type};

    use super::{OpenListConfig, open_list, row_references_binding};
    use crate::generator::{components::RenderScope, engine::features::Features};

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

    #[test]
    fn controlled_list_ref_precedes_scroll_callback_arguments() {
        let module = Module {
            app_name: "Parity".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let features = Features::default();
        let reference = Expr::State(
            "listRef".to_owned(),
            Type::ComponentRef {
                namespace: "Nexa".to_owned(),
                name: "FastList".to_owned(),
            },
        );
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let mut output = SourceWriter::new();

        open_list(
            OpenListConfig {
                axis: ListAxis::Horizontal,
                uses_platform_list: false,
                reorder_enabled: false,
                show_dividers: false,
                row_count: "4".to_owned(),
                row_key: None,
                item_extent: None,
                spacing: 0.0,
                scroll_enabled: None,
                on_end_reached: None,
                on_scroll: None,
                scroll_position: Some("position"),
                reverse_layout: false,
                page_snap: false,
                sticky_header: None,
                refresh: None,
                component_ref: Some(&reference),
            },
            &scope,
            0,
            &mut output,
        );

        let generated = output.finish();
        let scroll_position = generated
            .find("scrollPosition: nexa_position")
            .expect("emit the controlled scroll position");
        let component_ref = generated
            .find("componentRef: nexa_listRef")
            .expect("emit the component ref");
        let callback = generated
            .find("onScrollPositionChanged:")
            .expect("emit the scroll callback");
        assert!(
            scroll_position < component_ref && component_ref < callback,
            "{generated}"
        );
    }
}
