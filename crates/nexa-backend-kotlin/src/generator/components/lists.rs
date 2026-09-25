use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, expressions::expression,
    features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_list,
        "androidx.compose.foundation.lazy.LazyColumn",
    );
    imports.add(
        features.uses_linear_list_end_reached
            || features.uses_linear_list_scroll_position
            || features.uses_linear_list_scroll_events,
        "androidx.compose.foundation.lazy.rememberLazyListState",
    );
    imports.add(
        features.uses_horizontal_list,
        "androidx.compose.foundation.lazy.LazyRow",
    );
    imports.add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.LazyVerticalGrid",
    );
    imports.add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.GridCells",
    );
    imports.add(
        features.uses_grid_end_reached
            || features.uses_grid_scroll_position
            || features.uses_grid_scroll_events,
        "androidx.compose.foundation.lazy.grid.rememberLazyGridState",
    );
    imports.add(
        features.uses_linear_list,
        "androidx.compose.foundation.lazy.items",
    );
    imports.add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.grid.items",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "androidx.compose.runtime.snapshotFlow",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "kotlinx.coroutines.flow.collect",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "kotlinx.coroutines.flow.distinctUntilChanged",
    );
    imports.add(
        features.uses_list_end_reached || features.uses_linear_list_end_reached,
        "androidx.compose.runtime.getValue",
    );
    imports.add(
        features.uses_list_end_reached || features.uses_linear_list_end_reached,
        "androidx.compose.runtime.setValue",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_list_scroll_position
            || features.uses_list_scroll_events,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        features.uses_list_end_reached,
        "androidx.compose.runtime.remember",
    );
}

/// Flat (count- or collection-backed) list data borrowed from a [`ListPlan`].
/// Sectioned plans return before this is built, so every field combination
/// here is renderable without further dispatch.
struct FlatPieces<'a> {
    axis: ListAxis,
    count: FlatCount<'a>,
    index: &'a str,
    item: Option<&'a str>,
    item_extent: Option<f32>,
    key: Option<&'a Expr>,
    children: &'a [Node],
    on_end_reached: Option<&'a [Action]>,
    on_scroll: Option<&'a [Action]>,
    scroll_position: Option<&'a str>,
    sticky_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
}

/// The row-count source of a flat list with exactly the bindings present.
enum FlatCount<'a> {
    Count(&'a Expr),
    Items {
        collection: &'a Expr,
        element_type: &'a nexa_ir::Type,
        item: &'a str,
    },
}

pub(crate) fn render_virtualized_list(
    plan: &ListPlan,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let list_id = out.len();
    let pieces = match plan {
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
        ListPlan::Count { count, common } => FlatPieces {
            axis: common.axis,
            count: FlatCount::Count(count),
            index: &common.index,
            item: None,
            item_extent: common.item_extent,
            key: common.key.as_ref(),
            children: &common.children,
            on_end_reached: common.on_end_reached.as_deref(),
            on_scroll: common.on_scroll.as_deref(),
            scroll_position: common.scroll_position.as_deref(),
            sticky_header: common.sticky_header.as_deref(),
            refresh: common.refresh.as_ref(),
        },
        ListPlan::Items {
            collection,
            element_type,
            item,
            common,
        } => FlatPieces {
            axis: common.axis,
            count: FlatCount::Items {
                collection,
                element_type,
                item,
            },
            index: &common.index,
            item: Some(item),
            item_extent: common.item_extent,
            key: common.key.as_ref(),
            children: &common.children,
            on_end_reached: common.on_end_reached.as_deref(),
            on_scroll: common.on_scroll.as_deref(),
            scroll_position: common.scroll_position.as_deref(),
            sticky_header: common.sticky_header.as_deref(),
            refresh: common.refresh.as_ref(),
        },
    };
    match pieces.axis {
        ListAxis::Grid { columns } => {
            return render_grid_list(columns, &pieces, module, features, depth, list_id, out);
        }
        ListAxis::Vertical | ListAxis::Horizontal => {}
    }
    let list_state = (pieces.on_end_reached.is_some()
        || pieces.on_scroll.is_some()
        || pieces.scroll_position.is_some())
    .then(|| format!("nexaListState{list_id}"));
    let list_count = pieces
        .on_end_reached
        .map(|_| format!("nexaListCount{list_id}"));
    if let Some(list_state) = &list_state {
        let marker = pieces
            .on_end_reached
            .map(|_| format!("nexaEndReached{list_id}"));
        render_list_observers(
            pieces.axis,
            &pieces.count,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            pieces.on_end_reached,
            pieces.scroll_position,
            pieces.on_scroll,
            depth,
            out,
        );
    }
    let state_parameter = list_state
        .as_deref()
        .map_or_else(String::new, |state| format!("(state = {state})"));
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    indent(out, list_depth);
    // Grid plans return through `render_grid_list` above; the remaining
    // axes select the linear container with a total comparison.
    let container = if pieces.axis == ListAxis::Horizontal {
        "LazyRow"
    } else {
        "LazyColumn"
    };
    out.push_str(&format!("{container}{state_parameter} {{\n"));
    indent(out, list_depth + 1);
    if let Some(sticky_header) = pieces.sticky_header {
        debug_assert!(matches!(pieces.axis, ListAxis::Vertical));
        out.push_str("stickyHeader {\n");
        render_children(sticky_header, module, features, list_depth + 2, out);
        out.push('\n');
        indent(out, list_depth + 1);
        out.push_str("}\n");
        indent(out, list_depth + 1);
    }
    match &pieces.count {
        FlatCount::Count(count) => {
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("({}).coerceAtLeast(0)", expression(count)));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| render_key(key, pieces.index, pieces.item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(pieces.index)));
        }
        FlatCount::Items {
            collection,
            element_type,
            item,
        } => {
            let collection = expression(collection);
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{collection}.size"));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| {
                    render_key(key, pieces.index, Some(item), Some(&collection), "itemPosition")
                })
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(pieces.index)));
            indent(out, list_depth + 2);
            out.push_str(&format!(
                "val {}: {} = {collection}[{}]\n",
                state_name(item),
                element_type.kotlin(),
                state_name(pieces.index)
            ));
        }
    }
    render_row_content(
        pieces.item_extent,
        pieces.axis,
        pieces.children,
        module,
        features,
        list_depth + 2,
        out,
    );
    out.push('\n');
    indent(out, list_depth + 1);
    out.push_str("}\n");
    indent(out, list_depth);
    out.push('}');
    render_refresh_close(pieces.refresh, depth, out);
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
    let list_depth = render_refresh_open(refresh, depth, out);
    indent(out, list_depth);
    out.push_str("LazyColumn {\n");
    indent(out, list_depth + 1);
    out.push_str(&format!(
        "{collection}.forEachIndexed {{ sectionPosition, sectionItems ->\n"
    ));
    if let Some(header) = section_header {
        indent(out, list_depth + 2);
        out.push_str("stickyHeader {\n");
        indent(out, list_depth + 3);
        out.push_str(&format!(
            "val {}: Int = sectionPosition\n",
            state_name(section)
        ));
        render_children(header, module, features, list_depth + 3, out);
        out.push('\n');
        indent(out, list_depth + 2);
        out.push_str("}\n");
    }
    indent(out, list_depth + 2);
    out.push_str("items(\n");
    indent(out, list_depth + 3);
    out.push_str("count = sectionItems.size,\n");
    indent(out, list_depth + 3);
    let key = key
        .map(|key| {
            render_sectioned_key(
                key,
                section,
                index,
                item,
                "sectionItems",
                "sectionPosition",
                "itemPosition",
            )
        })
        .unwrap_or_else(|| "itemPosition".to_owned());
    out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
    indent(out, list_depth + 2);
    out.push_str(") { itemPosition ->\n");
    indent(out, list_depth + 3);
    out.push_str(&format!(
        "val {}: Int = sectionPosition\n",
        state_name(section)
    ));
    indent(out, list_depth + 3);
    out.push_str(&format!("val {}: Int = itemPosition\n", state_name(index)));
    indent(out, list_depth + 3);
    out.push_str(&format!(
        "val {}: {} = sectionItems[itemPosition]\n",
        state_name(item),
        element_type.kotlin()
    ));
    render_row_content(
        item_extent,
        ListAxis::Vertical,
        children,
        module,
        features,
        list_depth + 3,
        out,
    );
    out.push('\n');
    indent(out, list_depth + 2);
    out.push_str("}\n");
    indent(out, list_depth + 1);
    out.push_str("}\n");
    indent(out, list_depth);
    out.push('}');
    render_refresh_close(refresh, depth, out);
}

fn render_grid_list(
    columns: u32,
    pieces: &FlatPieces<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    list_id: usize,
    out: &mut String,
) {
    let list_state = (pieces.on_end_reached.is_some()
        || pieces.on_scroll.is_some()
        || pieces.scroll_position.is_some())
    .then(|| format!("nexaGridState{list_id}"));
    let list_count = pieces
        .on_end_reached
        .map(|_| format!("nexaGridCount{list_id}"));
    if let Some(list_state) = &list_state {
        let marker = pieces
            .on_end_reached
            .map(|_| format!("nexaEndReached{list_id}"));
        render_list_observers(
            ListAxis::Grid { columns },
            &pieces.count,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            pieces.on_end_reached,
            pieces.scroll_position,
            pieces.on_scroll,
            depth,
            out,
        );
    }
    let state_parameter = list_state
        .as_deref()
        .map_or_else(String::new, |state| format!("state = {state}, "));
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    indent(out, list_depth);
    out.push_str(&format!(
        "LazyVerticalGrid({state_parameter}columns = GridCells.Fixed({columns})) {{\n"
    ));
    indent(out, list_depth + 1);
    match &pieces.count {
        FlatCount::Count(count) => {
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("({}).coerceAtLeast(0)", expression(count)));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| render_key(key, pieces.index, pieces.item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(pieces.index)));
        }
        FlatCount::Items {
            collection,
            element_type,
            item,
        } => {
            let collection = expression(collection);
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{collection}.size"));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| {
                    render_key(key, pieces.index, Some(item), Some(&collection), "itemPosition")
                })
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(pieces.index)));
            indent(out, list_depth + 2);
            out.push_str(&format!(
                "val {}: {} = {collection}[{}]\n",
                state_name(item),
                element_type.kotlin(),
                state_name(pieces.index)
            ));
        }
    }
    render_row_content(
        pieces.item_extent,
        ListAxis::Grid { columns },
        pieces.children,
        module,
        features,
        list_depth + 2,
        out,
    );
    out.push('\n');
    indent(out, list_depth + 1);
    out.push_str("}\n");
    indent(out, list_depth);
    out.push('}');
    render_refresh_close(pieces.refresh, depth, out);
}

fn render_refresh_open(refresh: Option<&FastListRefresh>, depth: usize, out: &mut String) -> usize {
    let Some(refresh) = refresh else {
        return depth;
    };
    indent(out, depth);
    out.push_str("PullToRefreshBox(\n");
    indent(out, depth + 1);
    out.push_str(&format!("isRefreshing = {},\n", state_name(&refresh.state)));
    indent(out, depth + 1);
    out.push_str("onRefresh = {\n");
    render_actions(&refresh.actions, depth + 2, out);
    indent(out, depth + 1);
    out.push_str("},\n");
    indent(out, depth);
    out.push_str(") {\n");
    depth + 1
}

fn render_refresh_close(refresh: Option<&FastListRefresh>, depth: usize, out: &mut String) {
    if refresh.is_some() {
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
}

fn render_list_observers(
    axis: ListAxis,
    count: &FlatCount<'_>,
    list_state: &str,
    list_count: Option<&str>,
    marker: Option<&str>,
    end_actions: Option<&[Action]>,
    scroll_position: Option<&str>,
    scroll_actions: Option<&[Action]>,
    depth: usize,
    out: &mut String,
) {
    let scroll_position = scroll_position.map(state_name);
    let state_initializer = match axis {
        ListAxis::Grid { .. } => "rememberLazyGridState",
        ListAxis::Vertical | ListAxis::Horizontal => "rememberLazyListState",
    };
    if let Some(list_count) = list_count {
        let count_expression = flat_count_expression(count);
        indent(out, depth);
        out.push_str(&format!("val {list_count} = {count_expression}\n"));
    }
    let state_initializer = if let Some(scroll_position) = scroll_position.as_deref() {
        format!(
            "{state_initializer}(initialFirstVisibleItemIndex = {scroll_position}.coerceAtLeast(0))"
        )
    } else {
        format!("{state_initializer}()")
    };
    indent(out, depth);
    out.push_str(&format!("val {list_state} = {state_initializer}\n"));
    if let (Some(list_count), Some(marker), Some(actions)) = (list_count, marker, end_actions) {
        indent(out, depth);
        out.push_str(&format!(
            "var {marker} by remember {{ mutableIntStateOf(-1) }}\n"
        ));
        indent(out, depth);
        out.push_str(&format!("LaunchedEffect({list_state}, {list_count}) {{\n"));
        indent(out, depth + 1);
        out.push_str(&format!(
            "snapshotFlow {{ {list_state}.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }}.distinctUntilChanged().collect {{ lastVisible ->\n"
        ));
        indent(out, depth + 2);
        out.push_str(&format!(
            "if ({list_count} > 0 && lastVisible >= {list_count} - 1 && {marker} != {list_count}) {{\n"
        ));
        indent(out, depth + 3);
        out.push_str(&format!("{marker} = {list_count}\n"));
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("}\n");
        indent(out, depth + 1);
        out.push_str("}\n");
        indent(out, depth);
        out.push_str("}\n");
    }
    if scroll_position.is_some() || scroll_actions.is_some() {
        let scroll_marker = scroll_actions.map(|_| format!("nexaScrollEvent{list_state}"));
        if let Some(scroll_marker) = scroll_marker.as_deref() {
            indent(out, depth);
            out.push_str(&format!(
                "var {scroll_marker} by remember {{ mutableIntStateOf(-1) }}\n"
            ));
        }
        indent(out, depth);
        out.push_str(&format!("LaunchedEffect({list_state}) {{\n"));
        indent(out, depth + 1);
        out.push_str(&format!(
            "snapshotFlow {{ {list_state}.firstVisibleItemIndex }}.distinctUntilChanged().collect {{ firstVisible ->\n"
        ));
        indent(out, depth + 2);
        if let Some(scroll_position) = scroll_position.as_deref() {
            out.push_str(&format!(
                "if (firstVisible != {scroll_position}) {scroll_position} = firstVisible\n"
            ));
        }
        if let (Some(scroll_marker), Some(scroll_actions)) =
            (scroll_marker.as_deref(), scroll_actions)
        {
            out.push_str(&format!("if (firstVisible != {scroll_marker}) {{\n"));
            indent(out, depth + 3);
            out.push_str(&format!("{scroll_marker} = firstVisible\n"));
            render_actions(scroll_actions, depth + 3, out);
            indent(out, depth + 2);
            out.push_str("}\n");
        }
        indent(out, depth + 1);
        out.push_str("}\n");
        indent(out, depth);
        out.push_str("}\n");
        if let Some(scroll_position) = scroll_position.as_deref() {
            indent(out, depth);
            out.push_str(&format!("LaunchedEffect({scroll_position}) {{\n"));
            indent(out, depth + 1);
            out.push_str(&format!(
                "val target = {scroll_position}.coerceAtLeast(0)\n"
            ));
            indent(out, depth + 1);
            out.push_str(&format!(
                "if (target != {list_state}.firstVisibleItemIndex) {list_state}.scrollToItem(target)\n"
            ));
            indent(out, depth);
            out.push_str("}\n");
        }
    }
}

fn flat_count_expression(count: &FlatCount<'_>) -> String {
    match count {
        FlatCount::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        FlatCount::Items { collection, .. } => format!("{}.size", expression(collection)),
    }
}

fn render_row_content(
    item_extent: Option<f32>,
    axis: ListAxis,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let Some(item_extent) = item_extent else {
        render_children(children, module, features, depth, out);
        return;
    };
    let extent = format_float(item_extent);
    let modifier = match axis {
        ListAxis::Horizontal => format!("Modifier.width({extent}.dp).height({extent}.dp)"),
        ListAxis::Vertical | ListAxis::Grid { .. } => format!("Modifier.height({extent}.dp)"),
    };
    indent(out, depth);
    out.push_str(&format!("Box(modifier = {modifier}) {{\n"));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
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
        &format!("{collection}[{item_position}]"),
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
