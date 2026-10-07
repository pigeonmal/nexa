use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListMove, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, engine::types::kotlin_type,
    expressions::expression, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

use super::RenderScope;

struct SectionedPieces<'a> {
    collection: &'a Expr,
    element_type: &'a nexa_ir::Type,
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

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_list,
        "androidx.compose.foundation.lazy.LazyColumn",
    );
    imports.add(
        features.uses_linear_list || features.uses_grid_list,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_linear_list || features.uses_grid_list,
        "androidx.compose.ui.Modifier",
    );
    imports.add(
        features.uses_linear_list,
        "androidx.compose.material3.HorizontalDivider",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.pager.PagerDefaults",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.pager.PagerSnapDistance",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.pager.VerticalPager",
    );
    imports.add(
        features.uses_page_snap,
        "androidx.compose.foundation.pager.rememberPagerState",
    );
    imports.add(
        features.uses_linear_list_end_reached
            || features.uses_linear_list_scroll_position
            || features.uses_linear_list_scroll_events
            || features.uses_reverse_layout
            || features.facts.ui.lists.reorderable,
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
        features.uses_reverse_layout,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_reverse_layout
            || features.facts.ui.lists.reorderable,
        "androidx.compose.runtime.remember",
    );
    let swipe_actions = features.facts.ui.lists.swipe_actions;
    imports.add(swipe_actions, "androidx.compose.foundation.layout.Box");
    imports.add(swipe_actions, "androidx.compose.foundation.layout.Row");
    imports.add(
        swipe_actions,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(swipe_actions, "androidx.compose.foundation.background");
    imports.add(swipe_actions, "androidx.compose.material3.MaterialTheme");
    imports.add(
        swipe_actions,
        "androidx.compose.foundation.layout.Arrangement",
    );
    imports.add(swipe_actions, "androidx.compose.ui.Alignment");
    imports.add(swipe_actions, "androidx.compose.ui.Modifier");
    imports.add(
        swipe_actions,
        "androidx.compose.material3.SwipeToDismissBox",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.material3.SwipeToDismissBoxValue",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.material3.rememberSwipeToDismissBoxState",
    );
    imports.add(swipe_actions, "androidx.compose.runtime.LaunchedEffect");
    let reorderable = features.facts.ui.lists.reorderable;
    imports.add(
        reorderable,
        "androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress",
    );
    imports.add(reorderable, "androidx.compose.foundation.layout.Box");
    imports.add(reorderable, "androidx.compose.runtime.mutableFloatStateOf");
    imports.add(reorderable, "androidx.compose.runtime.remember");
    imports.add(reorderable, "androidx.compose.ui.Modifier");
    imports.add(reorderable, "androidx.compose.ui.graphics.graphicsLayer");
    imports.add(
        reorderable,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    imports.add(reorderable, "kotlin.math.abs");
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
    swipe_actions: Option<&'a [Node]>,
    on_end_reached: Option<&'a [Action]>,
    on_scroll: Option<&'a [Action]>,
    on_move: Option<&'a FastListMove>,
    scroll_position: Option<&'a str>,
    reverse_layout: bool,
    page_snap: bool,
    sticky_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
}

#[derive(Clone, Copy)]
struct FlatRowContent<'a> {
    sizing: ListRowSizing,
    axis: ListAxis,
    children: &'a [Node],
    swipe_actions: Option<&'a [Node]>,
    on_move: Option<&'a FastListMove>,
    index: Option<&'a str>,
    list_state: Option<&'a str>,
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
    out: &mut SourceWriter,
) {
    // Uniqueness seed for this list's generated state holders. Ids only need to
    // be distinct within the file being written, which is the writer's scope.
    let list_id = out.next_id();
    let pieces = match plan {
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
        ListPlan::Count { count, common } => FlatPieces {
            axis: common.axis,
            count: FlatCount::Count(count),
            index: &common.index,
            item: None,
            item_extent: common.item_extent,
            key: common.key.as_ref(),
            children: &common.children,
            swipe_actions: common.swipe_actions.as_deref(),
            on_end_reached: common.on_end_reached.as_deref(),
            on_scroll: common.on_scroll.as_deref(),
            on_move: common.on_move.as_ref(),
            scroll_position: common.scroll_position.as_deref(),
            reverse_layout: common.reverse_layout,
            page_snap: common.page_snap,
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
            swipe_actions: common.swipe_actions.as_deref(),
            on_end_reached: common.on_end_reached.as_deref(),
            on_scroll: common.on_scroll.as_deref(),
            on_move: common.on_move.as_ref(),
            scroll_position: common.scroll_position.as_deref(),
            reverse_layout: common.reverse_layout,
            page_snap: common.page_snap,
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
    let list_state = (pieces.reverse_layout
        || pieces.page_snap
        || pieces.on_end_reached.is_some()
        || pieces.on_scroll.is_some()
        || pieces.on_move.is_some()
        || pieces.scroll_position.is_some())
    .then(|| format!("nexaListState{list_id}"));
    let list_count = (pieces.on_end_reached.is_some() || pieces.reverse_layout || pieces.page_snap)
        .then(|| format!("nexaListCount{list_id}"));
    if let Some(list_state) = &list_state {
        let marker = pieces
            .on_end_reached
            .map(|_| format!("nexaEndReached{list_id}"));
        render_list_observers(
            &pieces,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            depth,
            out,
        );
    }
    if pieces.page_snap {
        if let Some(list_state) = list_state.as_deref() {
            render_page_snap_list(&pieces, module, features, list_state, depth, out);
        }
        return;
    }
    let state_argument = list_state
        .as_deref()
        .map(|state| format!("state = {state}"));
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    indent(out, list_depth);
    // Grid plans return through `render_grid_list` above; the remaining
    // axes select the linear container with a total comparison.
    let container = if pieces.axis == ListAxis::Horizontal {
        let mut arguments = vec!["modifier = Modifier.fillMaxWidth()".to_owned()];
        arguments.extend(state_argument);
        format!("LazyRow({})", arguments.join(", "))
    } else {
        let mut arguments = vec!["modifier = Modifier.fillMaxWidth()".to_owned()];
        arguments.extend(state_argument);
        if pieces.reverse_layout {
            arguments.push("reverseLayout = true".to_owned());
        }
        format!("LazyColumn({})", arguments.join(", "))
    };
    out.push_str(&format!("{container} {{\n"));
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
            out.line_at(list_depth + 2, format_args!("count = {count},"));
            indent(out, list_depth + 2);
            let row_position = reverse_row_position(pieces.reverse_layout, &count, "itemPosition");
            let key = pieces
                .key
                .map(|key| render_key(key, pieces.index, pieces.item, None, &row_position))
                .unwrap_or_else(|| row_position.clone());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            if pieces.reverse_layout {
                out.push_str(") { itemPosition ->\n");
                out.line_at(
                    list_depth + 2,
                    format_args!("val {}: Int = {row_position}", state_name(pieces.index)),
                );
            } else {
                out.line_at(
                    list_depth + 1,
                    format_args!(") {{ {} ->", state_name(pieces.index)),
                );
            }
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
            out.line_at(list_depth + 2, format_args!("count = {count},"));
            indent(out, list_depth + 2);
            let row_position = reverse_row_position(pieces.reverse_layout, &count, "itemPosition");
            let key = pieces
                .key
                .map(|key| {
                    render_key(
                        key,
                        pieces.index,
                        Some(item),
                        Some(&collection),
                        &row_position,
                    )
                })
                .unwrap_or_else(|| row_position.clone());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            if pieces.reverse_layout {
                out.push_str(") { itemPosition ->\n");
                out.line_at(
                    list_depth + 2,
                    format_args!("val {}: Int = {row_position}", state_name(pieces.index)),
                );
            } else {
                out.line_at(
                    list_depth + 1,
                    format_args!(") {{ {} ->", state_name(pieces.index)),
                );
            }
            out.line_at(
                list_depth + 2,
                format_args!(
                    "val {}: {} = {collection}[{}]",
                    state_name(item),
                    kotlin_type(element_type),
                    state_name(pieces.index)
                ),
            );
        }
    }
    let row_sizing = if pieces.page_snap {
        ListRowSizing::Viewport
    } else {
        pieces
            .item_extent
            .map(ListRowSizing::Fixed)
            .unwrap_or(ListRowSizing::Content)
    };
    render_row_content_with_actions(
        FlatRowContent {
            sizing: row_sizing,
            axis: pieces.axis,
            children: pieces.children,
            swipe_actions: pieces.swipe_actions,
            on_move: pieces.on_move,
            index: pieces.on_move.map(|_| pieces.index),
            list_state: list_state.as_deref(),
        },
        module,
        features,
        list_depth + 2,
        out,
    );
    if pieces.axis == ListAxis::Vertical {
        out.push('\n');
        out.line_at(list_depth + 2, format_args!("HorizontalDivider()"));
    }
    out.push('\n');
    indent(out, list_depth + 1);
    out.push_str("}\n");
    indent(out, list_depth);
    out.push('}');
    render_refresh_close(pieces.refresh, depth, out);
}

fn render_page_snap_list(
    pieces: &FlatPieces<'_>,
    module: &Module,
    features: &Features,
    list_state: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    let key = match &pieces.count {
        FlatCount::Count(_) => pieces
            .key
            .map(|key| render_key(key, pieces.index, pieces.item, None, "pagePosition")),
        FlatCount::Items {
            collection, item, ..
        } => {
            let collection = expression(collection);
            pieces.key.map(|key| {
                render_key(
                    key,
                    pieces.index,
                    Some(item),
                    Some(&collection),
                    "pagePosition",
                )
            })
        }
    };

    indent(out, list_depth);
    out.push_str("VerticalPager(\n");
    out.line_at(list_depth + 1, format_args!("state = {list_state},"));
    out.line_at(
        list_depth + 1,
        format_args!(
            "flingBehavior = PagerDefaults.flingBehavior(state = {list_state}, pagerSnapDistance = PagerSnapDistance.atMost(1)),"
        ),
    );
    if let Some(key) = key {
        out.line_at(
            list_depth + 1,
            format_args!("key = {{ pagePosition -> {key} }},"),
        );
    }
    indent(out, list_depth);
    out.push_str(") {\n");

    match &pieces.count {
        FlatCount::Count(_) => {
            out.line_at(
                list_depth + 1,
                format_args!("{} ->", state_name(pieces.index)),
            );
        }
        FlatCount::Items {
            collection,
            element_type,
            item,
        } => {
            let collection = expression(collection);
            out.line_at(list_depth + 1, format_args!("pagePosition ->"));
            out.line_at(
                list_depth + 2,
                format_args!("val {}: Int = pagePosition", state_name(pieces.index)),
            );
            out.line_at(
                list_depth + 2,
                format_args!(
                    "val {}: {} = {collection}[{}]",
                    state_name(item),
                    kotlin_type(element_type),
                    state_name(pieces.index)
                ),
            );
        }
    }

    render_row_content(
        ListRowSizing::PagerViewport,
        ListAxis::Vertical,
        pieces.children,
        module,
        features,
        list_depth + 2,
        out,
    );
    out.push('\n');
    indent(out, list_depth + 1);
    out.push_str("}\n");
    render_refresh_close(pieces.refresh, depth, out);
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
    let collection = expression(collection);
    let list_depth = render_refresh_open(refresh, depth, out);
    indent(out, list_depth);
    out.push_str("LazyColumn {\n");
    out.line_at(
        list_depth + 1,
        format_args!("{collection}.forEachIndexed {{ sectionPosition, sectionItems ->"),
    );
    if let Some(header) = section_header {
        indent(out, list_depth + 2);
        out.push_str("stickyHeader {\n");
        out.line_at(
            list_depth + 3,
            format_args!("val {}: Int = sectionPosition", state_name(section)),
        );
        out.line_at(
            list_depth + 3,
            format_args!("val {} = sectionItems", state_name("sectionItems")),
        );
        render_children(header, scope.module, scope.features, list_depth + 3, out);
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
    out.line_at(
        list_depth + 3,
        format_args!("val {}: Int = sectionPosition", state_name(section)),
    );
    out.line_at(
        list_depth + 3,
        format_args!("val {}: Int = itemPosition", state_name(index)),
    );
    out.line_at(
        list_depth + 3,
        format_args!(
            "val {}: {} = sectionItems[itemPosition]",
            state_name(item),
            kotlin_type(element_type)
        ),
    );
    render_row_content_with_actions(
        FlatRowContent {
            sizing: item_extent
                .map(ListRowSizing::Fixed)
                .unwrap_or(ListRowSizing::Content),
            axis: ListAxis::Vertical,
            children,
            swipe_actions,
            on_move: None,
            index: None,
            list_state: None,
        },
        scope.module,
        scope.features,
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
    out: &mut SourceWriter,
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
            pieces,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            depth,
            out,
        );
    }
    let state_parameter = list_state
        .as_deref()
        .map_or_else(String::new, |state| format!("state = {state}, "));
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    out.line_at(
        list_depth,
        format_args!("LazyVerticalGrid({state_parameter}columns = GridCells.Fixed({columns})) {{"),
    );
    indent(out, list_depth + 1);
    match &pieces.count {
        FlatCount::Count(count) => {
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("({}).coerceAtLeast(0)", expression(count)));
            out.push_str("items(\n");
            out.line_at(list_depth + 2, format_args!("count = {count},"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| render_key(key, pieces.index, pieces.item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            out.line_at(
                list_depth + 1,
                format_args!(") {{ {} ->", state_name(pieces.index)),
            );
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
            out.line_at(list_depth + 2, format_args!("count = {count},"));
            indent(out, list_depth + 2);
            let key = pieces
                .key
                .map(|key| {
                    render_key(
                        key,
                        pieces.index,
                        Some(item),
                        Some(&collection),
                        "itemPosition",
                    )
                })
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            out.line_at(
                list_depth + 1,
                format_args!(") {{ {} ->", state_name(pieces.index)),
            );
            out.line_at(
                list_depth + 2,
                format_args!(
                    "val {}: {} = {collection}[{}]",
                    state_name(item),
                    kotlin_type(element_type),
                    state_name(pieces.index)
                ),
            );
        }
    }
    render_row_content(
        pieces
            .item_extent
            .map(ListRowSizing::Fixed)
            .unwrap_or(ListRowSizing::Content),
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

fn render_refresh_open(
    refresh: Option<&FastListRefresh>,
    depth: usize,
    out: &mut SourceWriter,
) -> usize {
    let Some(refresh) = refresh else {
        return depth;
    };
    indent(out, depth);
    out.push_str("PullToRefreshBox(\n");
    out.line_at(
        depth + 1,
        format_args!("isRefreshing = {},", state_name(&refresh.state)),
    );
    indent(out, depth + 1);
    out.push_str("onRefresh = {\n");
    render_actions(&refresh.actions, depth + 2, out);
    indent(out, depth + 1);
    out.push_str("},\n");
    indent(out, depth);
    out.push_str(") {\n");
    depth + 1
}

fn render_refresh_close(refresh: Option<&FastListRefresh>, depth: usize, out: &mut SourceWriter) {
    if refresh.is_some() {
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
}

fn render_list_observers(
    pieces: &FlatPieces<'_>,
    list_state: &str,
    list_count: Option<&str>,
    marker: Option<&str>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let axis = pieces.axis;
    let count = &pieces.count;
    let end_actions = pieces.on_end_reached;
    let scroll_position = pieces.scroll_position;
    let scroll_actions = pieces.on_scroll;
    let scroll_position = scroll_position.map(state_name);
    let state_initializer = match axis {
        ListAxis::Grid { .. } => "rememberLazyGridState",
        ListAxis::Vertical | ListAxis::Horizontal if pieces.page_snap => "rememberPagerState",
        ListAxis::Vertical | ListAxis::Horizontal => "rememberLazyListState",
    };
    if let Some(list_count) = list_count {
        let count_expression = flat_count_expression(count);
        out.line_at(depth, format_args!("val {list_count} = {count_expression}"));
    }
    let state_initializer = if pieces.page_snap {
        let page_count = list_count.unwrap_or("0");
        let initial_page = scroll_position
            .as_deref()
            .map(|position| format!("{position}.coerceAtLeast(0)"))
            .unwrap_or_else(|| "0".to_owned());
        format!("rememberPagerState(initialPage = {initial_page}) {{ {page_count} }}")
    } else if let Some(scroll_position) = scroll_position.as_deref() {
        let initial_position = if pieces.reverse_layout {
            let count = list_count.unwrap_or("0");
            format!(
                "({count} - 1 - {scroll_position}.coerceAtLeast(0)).coerceIn(0, ({count} - 1).coerceAtLeast(0))"
            )
        } else {
            format!("{scroll_position}.coerceAtLeast(0)")
        };
        format!("{state_initializer}(initialFirstVisibleItemIndex = {initial_position})")
    } else {
        format!("{state_initializer}()")
    };
    out.line_at(
        depth,
        format_args!("val {list_state} = {state_initializer}"),
    );
    if pieces.reverse_layout {
        let list_count = list_count.unwrap_or("0");
        let previous_count = format!("nexaPreviousCount{list_state}");
        out.line_at(
            depth,
            format_args!(
                "val {previous_count} = remember({list_state}) {{ intArrayOf({list_count}) }}"
            ),
        );
        out.line_at(
            depth,
            format_args!("LaunchedEffect({list_state}, {list_count}) {{"),
        );
        out.line_at(
            depth + 1,
            format_args!("val previousCount = {previous_count}[0]"),
        );
        out.line_at(
            depth + 1,
            format_args!("val delta = {list_count} - previousCount"),
        );
        out.line_at(
            depth + 1,
            format_args!("val firstVisible = {list_state}.firstVisibleItemIndex"),
        );
        out.line_at(
            depth + 1,
            format_args!("val firstVisibleOffset = {list_state}.firstVisibleItemScrollOffset"),
        );
        out.line_at(
            depth + 1,
            format_args!("if (delta > 0 && {list_count} > 0 && firstVisible == delta && firstVisibleOffset == 0) {{"),
        );
        out.line_at(depth + 2, format_args!("{list_state}.scrollToItem(0)"));
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(
            depth + 1,
            format_args!("{previous_count}[0] = {list_count}"),
        );
        out.line_at(depth, format_args!("}}"));
    }
    if let (Some(list_count), Some(marker), Some(actions)) = (list_count, marker, end_actions) {
        out.line_at(
            depth,
            format_args!("var {marker} by remember {{ mutableIntStateOf(-1) }}"),
        );
        out.line_at(
            depth,
            format_args!("LaunchedEffect({list_state}, {list_count}) {{"),
        );
        if pieces.page_snap {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ {list_state}.isScrollInProgress }}.distinctUntilChanged().collect {{ isScrolling ->"
            ));
            out.line_at(depth + 2, format_args!("if (!isScrolling) {{"));
            let last_visible = if pieces.page_snap {
                format!("{list_state}.settledPage")
            } else {
                format!("{list_state}.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1")
            };
            out.line_at(depth + 3, format_args!("val lastVisible = {last_visible}"));
            out.line_at(depth + 3, format_args!("if ({list_count} > 0 && lastVisible >= {list_count} - 1 && {marker} != {list_count}) {{"
            ));
            out.line_at(depth + 4, format_args!("{marker} = {list_count}"));
            render_actions(actions, depth + 4, out);
            indent(out, depth + 3);
            out.push_str("}\n");
            indent(out, depth + 2);
            out.push_str("}\n");
        } else {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ {list_state}.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }}.distinctUntilChanged().collect {{ lastVisible ->"
            ));
            out.line_at(depth + 2, format_args!("if ({list_count} > 0 && lastVisible >= {list_count} - 1 && {marker} != {list_count}) {{"
            ));
            out.line_at(depth + 3, format_args!("{marker} = {list_count}"));
            render_actions(actions, depth + 3, out);
            indent(out, depth + 2);
            out.push_str("}\n");
        }
        indent(out, depth + 1);
        out.push_str("}\n");
        indent(out, depth);
        out.push_str("}\n");
    }
    if scroll_position.is_some() || scroll_actions.is_some() {
        let scroll_marker = scroll_actions.map(|_| format!("nexaScrollEvent{list_state}"));
        if let Some(scroll_marker) = scroll_marker.as_deref() {
            out.line_at(
                depth,
                format_args!("var {scroll_marker} by remember {{ mutableIntStateOf(-1) }}"),
            );
        }
        out.line_at(depth, format_args!("LaunchedEffect({list_state}) {{"));
        if pieces.page_snap {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ if ({list_state}.isScrollInProgress) -1 else {list_state}.settledPage }}.distinctUntilChanged().collect {{ firstVisiblePosition ->"
            ));
            out.line_at(depth + 2, format_args!("if (firstVisiblePosition >= 0) {{"));
            indent(out, depth + 3);
        } else {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ {list_state}.firstVisibleItemIndex }}.distinctUntilChanged().collect {{ firstVisiblePosition ->"
            ));
            indent(out, depth + 2);
        }
        let body_indent = depth + if pieces.page_snap { 3 } else { 2 };
        let first_visible = if pieces.reverse_layout {
            format!(
                "({} - 1 - firstVisiblePosition).coerceAtLeast(0)",
                list_count.unwrap_or("0")
            )
        } else {
            "firstVisiblePosition".to_owned()
        };
        if let Some(scroll_position) = scroll_position.as_deref() {
            out.line_at(
                body_indent,
                format_args!(
                    "if ({first_visible} != {scroll_position}) {scroll_position} = {first_visible}"
                ),
            );
        }
        if let (Some(scroll_marker), Some(scroll_actions)) =
            (scroll_marker.as_deref(), scroll_actions)
        {
            out.line_at(
                body_indent,
                format_args!("if ({first_visible} != {scroll_marker}) {{"),
            );
            let body_depth = body_indent + 1;
            out.line_at(
                body_depth,
                format_args!("{scroll_marker} = {first_visible}"),
            );
            render_actions(scroll_actions, body_depth, out);
            indent(out, body_indent);
            out.push_str("}\n");
        }
        if pieces.page_snap {
            indent(out, depth + 2);
            out.push_str("}\n");
        }
        indent(out, depth + 1);
        out.push_str("}\n");
        indent(out, depth);
        out.push_str("}\n");
        if let Some(scroll_position) = scroll_position.as_deref() {
            out.line_at(depth, format_args!("LaunchedEffect({scroll_position}) {{"));
            let target = if pieces.reverse_layout {
                let count = list_count.unwrap_or("0");
                format!(
                    "({count} - 1 - {scroll_position}).coerceIn(0, ({count} - 1).coerceAtLeast(0))"
                )
            } else {
                format!("{scroll_position}.coerceAtLeast(0)")
            };
            out.line_at(depth + 1, format_args!("val target = {target}"));
            let current_position = if pieces.page_snap {
                format!("{list_state}.settledPage")
            } else {
                format!("{list_state}.firstVisibleItemIndex")
            };
            let scroll_to = if pieces.page_snap {
                format!("{list_state}.scrollToPage(target)")
            } else {
                format!("{list_state}.scrollToItem(target)")
            };
            out.line_at(
                depth + 1,
                format_args!("if (target != {current_position}) {scroll_to}"),
            );
            indent(out, depth);
            out.push_str("}\n");
        }
    }
}

fn reverse_row_position(reverse_layout: bool, count: &str, position: &str) -> String {
    if reverse_layout {
        format!("({count} - 1 - {position})")
    } else {
        position.to_owned()
    }
}

fn flat_count_expression(count: &FlatCount<'_>) -> String {
    match count {
        FlatCount::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        FlatCount::Items { collection, .. } => format!("{}.size", expression(collection)),
    }
}

#[derive(Clone, Copy)]
enum ListRowSizing {
    Content,
    Fixed(f32),
    Viewport,
    PagerViewport,
}

fn render_row_content(
    sizing: ListRowSizing,
    axis: ListAxis,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let modifier = match sizing {
        ListRowSizing::Content => {
            render_children(children, module, features, depth, out);
            return;
        }
        ListRowSizing::Viewport => "Modifier.fillParentMaxSize()".to_owned(),
        ListRowSizing::PagerViewport => "Modifier.fillMaxSize()".to_owned(),
        ListRowSizing::Fixed(item_extent) => {
            let extent = format_float(item_extent);
            match axis {
                ListAxis::Horizontal => {
                    format!("Modifier.width({extent}.dp).height({extent}.dp)")
                }
                ListAxis::Vertical | ListAxis::Grid { .. } => {
                    format!("Modifier.fillMaxWidth().height({extent}.dp)")
                }
            }
        }
    };
    out.line_at(depth, format_args!("Box(modifier = {modifier}) {{"));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_reorderable_row(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if let Some(callback) = row.on_move
        && !matches!(callback.enabled, Expr::Bool(true))
    {
        out.line_at(
            depth,
            format_args!("if ({}) {{", expression(&callback.enabled)),
        );
        render_reorderable_row_enabled(row, module, features, depth + 1, out);
        out.line_at(depth, format_args!("}} else {{"));
        render_row_content(
            row.sizing,
            row.axis,
            row.children,
            module,
            features,
            depth + 1,
            out,
        );
        out.line_at(depth, format_args!("}}"));
        return;
    }
    render_reorderable_row_enabled(row, module, features, depth, out);
}

fn render_reorderable_row_enabled(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let (Some(callback), Some(index), Some(list_state)) = (row.on_move, row.index, row.list_state)
    else {
        render_row_content(
            row.sizing,
            row.axis,
            row.children,
            module,
            features,
            depth,
            out,
        );
        return;
    };
    let drag_offset = format!("nexaDragOffset{}", out.next_id());
    out.line_at(
        depth,
        format_args!(
            "val {drag_offset} = remember({}) {{ mutableFloatStateOf(0f) }}",
            state_name(index)
        ),
    );
    out.line_at(depth, format_args!("Box(modifier = Modifier"));
    out.line_at(
        depth + 1,
        format_args!(".graphicsLayer {{ translationY = {drag_offset}.floatValue }}"),
    );
    out.line_at(
        depth + 1,
        format_args!(".pointerInput({}) {{", state_name(index)),
    );
    out.line_at(depth + 2, format_args!("detectDragGesturesAfterLongPress("));
    out.line_at(
        depth + 3,
        format_args!("onDragStart = {{ {drag_offset}.floatValue = 0f }},"),
    );
    out.line_at(depth + 3, format_args!("onDragEnd = {{"));
    out.line_at(
        depth + 4,
        format_args!("val draggedItem = {list_state}.layoutInfo.visibleItemsInfo.firstOrNull {{ it.index == {} }}", state_name(index)),
    );
    out.line_at(depth + 4, format_args!("if (draggedItem != null) {{"));
    out.line_at(
        depth + 5,
        format_args!("val draggedCenter = draggedItem.offset + draggedItem.size / 2 + {drag_offset}.floatValue.toInt()"),
    );
    out.line_at(
        depth + 5,
        format_args!("val targetIndex = {list_state}.layoutInfo.visibleItemsInfo.minByOrNull {{ abs(it.offset + it.size / 2 - draggedCenter) }}?.index"),
    );
    out.line_at(
        depth + 5,
        format_args!(
            "if (targetIndex != null && targetIndex != {}) {{",
            state_name(index)
        ),
    );
    out.line_at(
        depth + 6,
        format_args!(
            "val {}: Int = {}",
            state_name(&callback.from),
            state_name(index)
        ),
    );
    out.line_at(
        depth + 6,
        format_args!("val {}: Int = targetIndex", state_name(&callback.to)),
    );
    render_actions(&callback.actions, depth + 6, out);
    out.line_at(depth + 5, format_args!("}}"));
    out.line_at(depth + 4, format_args!("}}"));
    out.line_at(depth + 4, format_args!("{drag_offset}.floatValue = 0f"));
    out.line_at(depth + 3, format_args!("}},"));
    out.line_at(
        depth + 3,
        format_args!("onDragCancel = {{ {drag_offset}.floatValue = 0f }},"),
    );
    out.line_at(depth + 3, format_args!("onDrag = {{ change, dragAmount ->"));
    out.line_at(depth + 4, format_args!("change.consume()"));
    out.line_at(
        depth + 4,
        format_args!("{drag_offset}.floatValue += dragAmount.y"),
    );
    out.line_at(depth + 3, format_args!("}}"));
    out.line_at(depth + 2, format_args!(")"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth + 1, format_args!(") {{"));
    render_row_content(
        row.sizing,
        row.axis,
        row.children,
        module,
        features,
        depth + 2,
        out,
    );
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
}

fn render_row_content_with_actions(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(actions) = row.swipe_actions else {
        render_reorderable_row(row, module, features, depth, out);
        return;
    };
    let swipe_id = out.next_id();
    let state = format!("nexaSwipeState{swipe_id}");
    out.line_at(
        depth,
        format_args!("val {state} = rememberSwipeToDismissBoxState()"),
    );
    out.line_at(
        depth,
        format_args!("LaunchedEffect({state}.currentValue) {{"),
    );
    out.line_at(
        depth + 1,
        format_args!("if ({state}.currentValue == SwipeToDismissBoxValue.EndToStart) {{"),
    );
    if let Some(Node::Button { actions, .. }) = actions.first() {
        render_actions(actions, depth + 2, out);
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
    out.line_at(depth, format_args!("SwipeToDismissBox("));
    out.line_at(depth + 1, format_args!("state = {state},"));
    out.line_at(depth + 1, format_args!("backgroundContent = {{"));
    out.line_at(
        depth + 2,
        format_args!("Row(modifier = Modifier.fillMaxSize(), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {{"),
    );
    render_children(actions, module, features, depth + 3, out);
    out.push('\n');
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("content = {{"));
    out.line_at(
        depth + 2,
        format_args!("Box(modifier = Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surface)) {{"),
    );
    render_reorderable_row(row, module, features, depth + 2, out);
    out.push('\n');
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    indent(out, depth);
    out.push(')');
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
