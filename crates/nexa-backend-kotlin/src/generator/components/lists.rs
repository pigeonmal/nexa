use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListMove, FastListRefresh, ListAxis, ListPlan, Module, Node};

use crate::generator::{
    components::{render_children, render_node},
    controls::render_actions,
    engine::types::kotlin_type,
    expressions::expression,
    features::Features,
    utils::indent,
};

use crate::generator::engine::imports::ImportSet;

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

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_fast_list,
        "androidx.compose.runtime.SideEffect",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.runtime.DisposableEffect",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.runtime.rememberCoroutineScope",
    );
    imports.add(features.uses_fast_list, "kotlinx.coroutines.launch");
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.gestures.animateScrollBy",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.gestures.scrollBy",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.ui.platform.LocalDensity",
    );
    imports.add(
        features.facts.ui.lists.refresh_fused,
        "androidx.compose.material3.pulltorefresh.PullToRefreshBox",
    );
    imports.add(features.uses_fast_list, "androidx.compose.ui.Modifier");
    imports.add(features.uses_fast_list, "androidx.compose.ui.Alignment");
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.lazy.LazyColumn",
    );
    imports.add(
        features.uses_linear_list,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_linear_list || features.uses_grid_list,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.defaultMinSize",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.height",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.foundation.background",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.material3.HorizontalDivider",
    );
    imports.add(
        features.uses_fast_list,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(features.uses_fast_list, "androidx.compose.ui.draw.clip");
    imports.add(features.uses_fast_list, "androidx.compose.ui.unit.dp");
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
        features.uses_linear_list
            || features.uses_linear_list_end_reached
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
        features.uses_horizontal_list,
        "androidx.compose.ui.input.nestedscroll.nestedScroll",
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
        features.uses_fast_list,
        "androidx.compose.foundation.layout.Arrangement",
    );
    imports.add(
        features.uses_grid_list
            || features.uses_grid_end_reached
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
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid",
    );
    imports.add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells",
    );
    imports.add(
        features.uses_grid_list,
        "androidx.compose.foundation.lazy.staggeredgrid.rememberLazyStaggeredGridState",
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
        features.uses_list_scroll_events,
        "androidx.compose.foundation.interaction.collectIsDraggedAsState",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_linear_list_end_reached
            || features.uses_list_scroll_position,
        "androidx.compose.runtime.getValue",
    );
    imports.add(
        features.uses_list_end_reached
            || features.uses_linear_list_end_reached
            || features.uses_list_scroll_position,
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
            || features.uses_list_scroll_position
            || features.uses_reverse_layout
            || features.facts.ui.lists.reorderable,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_list_end_reached,
        "androidx.compose.runtime.saveable.rememberSaveable",
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
        "androidx.compose.material3.rememberSwipeToDismissBoxState",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.foundation.gestures.awaitEachGesture",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.foundation.gestures.awaitFirstDown",
    );
    imports.add(
        swipe_actions,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    let reorderable = features.facts.ui.lists.reorderable;
    imports.add(
        reorderable,
        "androidx.compose.foundation.gestures.awaitEachGesture",
    );
    imports.add(
        reorderable,
        "androidx.compose.foundation.gestures.awaitFirstDown",
    );
    imports.add(reorderable, "androidx.compose.foundation.layout.Box");
    imports.add(reorderable, "androidx.compose.runtime.mutableFloatStateOf");
    imports.add(reorderable, "androidx.compose.runtime.mutableIntStateOf");
    imports.add(reorderable, "androidx.compose.runtime.remember");
    imports.add(reorderable, "androidx.compose.ui.Modifier");
    imports.add(reorderable, "androidx.compose.ui.graphics.graphicsLayer");
    imports.add(reorderable, "androidx.compose.ui.zIndex");
    imports.add(
        reorderable,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    imports.add(reorderable, "kotlinx.coroutines.withTimeoutOrNull");
    imports.add(reorderable, "kotlin.math.abs");
}

/// Flat (count- or collection-backed) list data borrowed from a [`ListPlan`].
/// Sectioned plans return before this is built, so every field combination
/// here is renderable without further dispatch.
struct FlatPieces<'a> {
    axis: ListAxis,
    uses_platform_list: bool,
    show_dividers: bool,
    count: FlatCount<'a>,
    index: &'a str,
    item: Option<&'a str>,
    item_extent: Option<f32>,
    spacing: f32,
    scroll_enabled: Option<&'a Expr>,
    key: Option<&'a Expr>,
    children: &'a [Node],
    swipe_actions: Option<&'a [Node]>,
    on_end_reached: Option<&'a [Action]>,
    on_scroll: Option<&'a [Action]>,
    on_scroll_begin_drag: Option<&'a [Action]>,
    on_move: Option<&'a FastListMove>,
    scroll_position: Option<&'a str>,
    reverse_layout: bool,
    page_snap: bool,
    sticky_header: Option<&'a [Node]>,
    refresh: Option<&'a FastListRefresh>,
    component_ref: Option<&'a Expr>,
}

#[derive(Clone, Copy)]
struct FlatRowContent<'a> {
    sizing: ListRowSizing,
    axis: ListAxis,
    animate_item: bool,
    list_index_offset: usize,
    dragged_index_state: Option<&'a str>,
    drag_offset_state: Option<&'a str>,
    children: &'a [Node],
    swipe_actions: Option<&'a [Node]>,
    reset_key: &'a str,
    native_list_row: bool,
    native_row_divider: bool,
    on_move: Option<&'a FastListMove>,
    index: Option<&'a str>,
    list_state: Option<&'a str>,
}

/// The row-count source of a flat list with exactly the bindings present.
enum FlatCount<'a> {
    Count(&'a Expr),
    Items {
        collection: String,
        element_type: &'a nexa_ir::Type,
        item: &'a str,
    },
}

fn scroll_enabled_expression(state: &str, enabled: Option<&Expr>, refresh_enabled: bool) -> String {
    let scroll_range = if refresh_enabled {
        "true".to_owned()
    } else {
        format!("({state}.canScrollForward || {state}.canScrollBackward)")
    };
    enabled.map_or(scroll_range.clone(), |value| {
        format!("{scroll_range} && {}", expression(value))
    })
}

fn safe_area_axis(axis: ListAxis) -> &'static str {
    match axis {
        ListAxis::Vertical => "Vertical",
        ListAxis::Horizontal => "Horizontal",
        ListAxis::Grid { .. } | ListAxis::AutoGrid { .. } | ListAxis::Masonry { .. } => "Both",
    }
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
                    uses_platform_list: common.swipe_actions.is_some(),
                    show_dividers: common.show_dividers,
                    item_extent: common.item_extent,
                    spacing: common.gap,
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
        ListPlan::Count { count, common } => FlatPieces {
            axis: common.axis,
            uses_platform_list: common.swipe_actions.is_some(),
            show_dividers: common.show_dividers,
            count: FlatCount::Count(count),
            index: &common.index,
            item: None,
            item_extent: common.item_extent,
            spacing: common.gap,
            scroll_enabled: common.scroll_enabled.as_ref(),
            key: common.key.as_ref(),
            children: &common.children,
            swipe_actions: common.swipe_actions.as_deref(),
            on_end_reached: common.on_end_reached.as_deref(),
            on_scroll: common.on_scroll.as_deref(),
            on_scroll_begin_drag: common.on_scroll_begin_drag.as_deref(),
            on_move: common.on_move.as_ref(),
            scroll_position: common.scroll_position.as_deref(),
            reverse_layout: common.reverse_layout,
            page_snap: common.page_snap,
            sticky_header: common.sticky_header.as_deref(),
            refresh: common.refresh.as_ref(),
            component_ref: common.component_ref.as_ref(),
        },
        ListPlan::Items {
            collection: source_collection,
            element_type,
            item,
            common,
        } => {
            let collection = format!("nexaListValues{list_id}");
            out.line_at(
                depth,
                format_args!("val {collection} = {}", expression(source_collection)),
            );
            FlatPieces {
                axis: common.axis,
                uses_platform_list: common.on_move.is_some() || common.swipe_actions.is_some(),
                show_dividers: common.show_dividers,
                count: FlatCount::Items {
                    collection,
                    element_type,
                    item,
                },
                index: &common.index,
                item: Some(item),
                item_extent: common.item_extent,
                spacing: common.gap,
                scroll_enabled: common.scroll_enabled.as_ref(),
                key: common.key.as_ref(),
                children: &common.children,
                swipe_actions: common.swipe_actions.as_deref(),
                on_end_reached: common.on_end_reached.as_deref(),
                on_scroll: common.on_scroll.as_deref(),
                on_scroll_begin_drag: common.on_scroll_begin_drag.as_deref(),
                on_move: common.on_move.as_ref(),
                scroll_position: common.scroll_position.as_deref(),
                reverse_layout: common.reverse_layout,
                page_snap: common.page_snap,
                sticky_header: common.sticky_header.as_deref(),
                refresh: common.refresh.as_ref(),
                component_ref: common.component_ref.as_ref(),
            }
        }
    };
    match pieces.axis {
        ListAxis::Grid { .. } | ListAxis::AutoGrid { .. } | ListAxis::Masonry { .. } => {
            return render_grid_list(pieces.axis, &pieces, module, features, depth, list_id, out);
        }
        ListAxis::Vertical | ListAxis::Horizontal => {}
    }
    let has_list_observers = pieces.reverse_layout
        || pieces.page_snap
        || pieces.on_end_reached.is_some()
        || pieces.on_scroll.is_some()
        || pieces.on_scroll_begin_drag.is_some()
        || pieces.on_move.is_some()
        || pieces.scroll_position.is_some()
        || pieces.component_ref.is_some();
    let list_state = (has_list_observers
        || (matches!(pieces.axis, ListAxis::Vertical | ListAxis::Horizontal) && !pieces.page_snap))
        .then(|| format!("nexaListState{list_id}"));
    let dragged_index_state = pieces.on_move.map(|_| format!("nexaDraggedIndex{list_id}"));
    let drag_offset_state = pieces.on_move.map(|_| format!("nexaDragOffset{list_id}"));
    if let Some(state) = dragged_index_state.as_deref() {
        out.line_at(
            depth,
            format_args!("val {state} = remember {{ mutableIntStateOf(-1) }}"),
        );
    }
    if let Some(state) = drag_offset_state.as_deref() {
        out.line_at(
            depth,
            format_args!("val {state} = remember {{ mutableFloatStateOf(0f) }}"),
        );
    }
    let list_count = flat_list_requires_count(&pieces).then(|| format!("nexaListCount{list_id}"));
    if let Some(list_state) = list_state.as_deref() {
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
    if let (Some(component_ref), Some(list_state)) = (pieces.component_ref, list_state.as_deref()) {
        render_list_component_ref(
            component_ref,
            list_state,
            pieces.axis,
            pieces.page_snap,
            &flat_count_expression(&pieces.count),
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
    let native_list_count = match &pieces.count {
        FlatCount::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        FlatCount::Items { collection, .. } => format!("{collection}.size"),
    };
    let spacing = format_float(pieces.spacing);
    let scroll_viewport_modifier = format!(
        "nexaSafeAreaScrollViewportModifier(\"{}\")",
        safe_area_axis(pieces.axis),
    );
    let container = if pieces.axis == ListAxis::Horizontal {
        let scroll_state = list_state.as_deref().unwrap_or_default();
        let mut arguments = vec![format!(
            "modifier = {scroll_viewport_modifier}.then(Modifier.fillMaxWidth().nestedScroll(NexaHorizontalListScrollBoundary))"
        )];
        arguments.push("contentPadding = LocalNexaScrollContentPadding.current".to_owned());
        if pieces.spacing > 0.0 {
            arguments.push(format!(
                "horizontalArrangement = Arrangement.spacedBy({spacing}.dp)"
            ));
        }
        arguments.extend(state_argument);
        arguments.push(format!(
            "userScrollEnabled = {}",
            scroll_enabled_expression(
                scroll_state,
                pieces.scroll_enabled,
                pieces.refresh.is_some(),
            )
        ));
        format!("LazyRow({})", arguments.join(", "))
    } else {
        let modifier = if pieces.uses_platform_list {
            "NexaNativeListContainerModifier()"
        } else {
            "Modifier.fillMaxWidth()"
        };
        let mut arguments = vec![format!(
            "modifier = {scroll_viewport_modifier}.then({modifier})"
        )];
        arguments.push("contentPadding = LocalNexaScrollContentPadding.current".to_owned());
        if pieces.spacing > 0.0 {
            arguments.push(format!(
                "verticalArrangement = Arrangement.spacedBy({spacing}.dp)"
            ));
        }
        arguments.extend(state_argument);
        let scroll_state = list_state.as_deref().unwrap_or_default();
        arguments.push(format!(
            "userScrollEnabled = {}",
            scroll_enabled_expression(
                scroll_state,
                pieces.scroll_enabled,
                pieces.refresh.is_some(),
            )
        ));
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
        out.line_at(
            list_depth + 2,
            format_args!("NexaScrollContentResetPrimitive {{"),
        );
        render_children(sticky_header, module, features, list_depth + 3, out);
        out.push('\n');
        out.line_at(list_depth + 2, format_args!("}}"));
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
            // LazyColumn(reverseLayout = true) already reverses item placement.
            // Preserve the source index instead of reversing the row data too.
            let row_position = "itemPosition".to_owned();
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
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{collection}.size"));
            out.push_str("items(\n");
            out.line_at(list_depth + 2, format_args!("count = {count},"));
            indent(out, list_depth + 2);
            // LazyColumn(reverseLayout = true) already reverses item placement.
            // Preserve the source index instead of reversing the row data too.
            let row_position = "itemPosition".to_owned();
            let key = pieces
                .key
                .map(|key| {
                    render_key(
                        key,
                        pieces.index,
                        Some(item),
                        Some(collection),
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
    if pieces.uses_platform_list && pieces.on_move.is_none() {
        out.line_at(
            list_depth + 2,
            format_args!(
                "NexaNativeListRowPrimitive(position = {}, count = {native_list_count}) {{",
                state_name(pieces.index)
            ),
        );
    }
    render_row_content_with_actions(
        FlatRowContent {
            sizing: row_sizing,
            axis: pieces.axis,
            animate_item: pieces.key.is_some() && pieces.axis == ListAxis::Vertical,
            list_index_offset: usize::from(pieces.sticky_header.is_some()),
            dragged_index_state: dragged_index_state.as_deref(),
            drag_offset_state: drag_offset_state.as_deref(),
            children: pieces.children,
            swipe_actions: pieces.swipe_actions,
            reset_key: &native_list_count,
            native_list_row: pieces.uses_platform_list && pieces.on_move.is_some(),
            native_row_divider: pieces.show_dividers && pieces.axis == ListAxis::Vertical,
            on_move: pieces.on_move,
            index: pieces.on_move.map(|_| pieces.index),
            list_state: list_state.as_deref(),
        },
        module,
        features,
        list_depth
            + if pieces.uses_platform_list && pieces.on_move.is_none() {
                3
            } else {
                2
            },
        out,
    );
    if pieces.axis == ListAxis::Vertical {
        out.push('\n');
        if pieces.uses_platform_list && pieces.on_move.is_none() {
            if pieces.show_dividers {
                out.line_at(
                    list_depth + 3,
                    format_args!(
                        "if ({} + 1 < {native_list_count}) NexaNativeListDividerPrimitive()",
                        state_name(pieces.index)
                    ),
                );
                out.push('\n');
            }
            indent(out, list_depth + 2);
            out.push('}');
        } else if pieces.show_dividers {
            out.line_at(
                list_depth + 2,
                format_args!(
                    "if ({} + 1 < {native_list_count}) NexaFastListDividerPrimitive()",
                    state_name(pieces.index)
                ),
            );
        }
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
        } => pieces.key.map(|key| {
            render_key(
                key,
                pieces.index,
                Some(item),
                Some(collection),
                "pagePosition",
            )
        }),
    };

    indent(out, list_depth);
    out.push_str("VerticalPager(\n");
    out.line_at(list_depth + 1, format_args!("state = {list_state},"));
    out.line_at(
        list_depth + 1,
        format_args!("contentPadding = LocalNexaScrollContentPadding.current,"),
    );
    out.line_at(
        list_depth + 1,
        format_args!(
            "modifier = nexaSafeAreaScrollViewportModifier(\"{}\").fillMaxSize(),",
            safe_area_axis(pieces.axis),
        ),
    );
    out.line_at(
        list_depth + 1,
        format_args!(
            "userScrollEnabled = {},",
            scroll_enabled_expression(list_state, pieces.scroll_enabled, pieces.refresh.is_some(),)
        ),
    );
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
    let collection_name = format!("nexaListSections{}", out.next_id());
    out.line_at(
        depth,
        format_args!("val {collection_name} = {}", expression(collection)),
    );
    let list_state = format!("nexaSectionedListState{}", out.next_id());
    out.line_at(
        depth,
        format_args!("val {list_state} = rememberLazyListState()"),
    );
    if let Some(component_ref) = component_ref {
        let row_count = format!("({collection_name}.sumOf {{ it.size }})");
        render_list_component_ref(
            component_ref,
            &list_state,
            ListAxis::Vertical,
            false,
            &row_count,
            depth,
            out,
        );
    }
    let section_collection = collection_name;
    let list_depth = render_refresh_open(refresh, depth, out);
    indent(out, list_depth);
    out.push_str(&format!("LazyColumn(state = {list_state}, "));
    out.push_str(&format!(
        "modifier = nexaSafeAreaScrollViewportModifier(\"Vertical\").then({}), contentPadding = LocalNexaScrollContentPadding.current, ",
        sectioned_list_modifier(uses_platform_list),
    ));
    if spacing > 0.0 {
        out.push_str(&format!(
            "verticalArrangement = Arrangement.spacedBy({}.dp), ",
            format_float(spacing),
        ));
    }
    out.push_str(&format!(
        "userScrollEnabled = {}",
        scroll_enabled_expression(&list_state, scroll_enabled, refresh.is_some())
    ));
    out.push_str(") {\n");
    out.line_at(
        list_depth + 1,
        format_args!("{section_collection}.forEachIndexed {{ sectionPosition, sectionItems ->"),
    );
    if let Some(header) = section_header {
        indent(out, list_depth + 2);
        out.push_str("stickyHeader {\n");
        out.line_at(
            list_depth + 3,
            format_args!("NexaScrollContentResetPrimitive {{"),
        );
        out.line_at(
            list_depth + 4,
            format_args!("val {}: Int = sectionPosition", state_name(section)),
        );
        out.line_at(
            list_depth + 4,
            format_args!("val {} = sectionItems", state_name("sectionItems")),
        );
        if uses_platform_list {
            out.line_at(
                list_depth + 4,
                format_args!("NexaNativeListSectionHeaderPrimitive {{"),
            );
            render_children(header, scope.module, scope.features, list_depth + 5, out);
            out.push('\n');
            indent(out, list_depth + 4);
            out.push_str("}\n");
        } else {
            render_children(header, scope.module, scope.features, list_depth + 4, out);
        }
        out.push('\n');
        out.line_at(list_depth + 3, format_args!("}}"));
        indent(out, list_depth + 2);
        out.push_str("}\n");
    }
    indent(out, list_depth + 2);
    out.push_str("items(\n");
    indent(out, list_depth + 3);
    out.push_str("count = sectionItems.size,\n");
    indent(out, list_depth + 3);
    let has_stable_key = key.is_some();
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
        .unwrap_or_else(|| "sectionPosition to itemPosition".to_owned());
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
    if uses_platform_list {
        out.line_at(
            list_depth + 3,
            format_args!(
                "NexaNativeListRowPrimitive(position = itemPosition, count = sectionItems.size) {{"
            ),
        );
    }
    render_row_content_with_actions(
        FlatRowContent {
            sizing: item_extent
                .map(ListRowSizing::Fixed)
                .unwrap_or(ListRowSizing::Content),
            axis: ListAxis::Vertical,
            animate_item: has_stable_key,
            list_index_offset: 0,
            dragged_index_state: None,
            drag_offset_state: None,
            children,
            swipe_actions,
            reset_key: "sectionItems.size",
            native_list_row: false,
            native_row_divider: false,
            on_move: None,
            index: None,
            list_state: None,
        },
        scope.module,
        scope.features,
        list_depth + if uses_platform_list { 4 } else { 3 },
        out,
    );
    out.push('\n');
    if uses_platform_list {
        if show_dividers {
            out.line_at(
                list_depth + 4,
                format_args!(
                    "if (itemPosition + 1 < sectionItems.size) NexaNativeListDividerPrimitive()"
                ),
            );
            out.push('\n');
        }
        indent(out, list_depth + 3);
        out.push_str("}\n");
    } else if show_dividers {
        out.line_at(
            list_depth + 3,
            format_args!(
                "if (itemPosition + 1 < sectionItems.size) NexaFastListDividerPrimitive()"
            ),
        );
        out.push('\n');
    }
    indent(out, list_depth + 2);
    out.push_str("}\n");
    if uses_platform_list {
        out.line_at(
            list_depth + 2,
            format_args!("if (sectionPosition + 1 < {section_collection}.size) {{"),
        );
        out.line_at(
            list_depth + 3,
            format_args!(
                "item(key = \"nexa-native-list-section-gap-$sectionPosition\") {{ NexaNativeListSectionSpacingPrimitive() }}"
            ),
        );
        out.line_at(list_depth + 2, format_args!("}}"));
    }
    indent(out, list_depth + 1);
    out.push_str("}\n");
    indent(out, list_depth);
    out.push('}');
    render_refresh_close(refresh, depth, out);
}

fn render_grid_list(
    axis: ListAxis,
    pieces: &FlatPieces<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    list_id: usize,
    out: &mut SourceWriter,
) {
    let list_state = format!("nexaGridState{list_id}");
    let list_count = flat_list_requires_count(pieces).then(|| format!("nexaGridCount{list_id}"));
    let marker = pieces
        .on_end_reached
        .map(|_| format!("nexaEndReached{list_id}"));
    render_list_observers(
        pieces,
        &list_state,
        list_count.as_deref(),
        marker.as_deref(),
        depth,
        out,
    );
    if let Some(component_ref) = pieces.component_ref {
        render_list_component_ref(
            component_ref,
            &list_state,
            axis,
            pieces.page_snap,
            &flat_count_expression(&pieces.count),
            depth,
            out,
        );
    }
    let state_parameter = format!("state = {list_state}, ");
    let scroll_state = list_state.as_str();
    let spacing = format_float(pieces.spacing);
    let list_depth = render_refresh_open(pieces.refresh, depth, out);
    let grid_cells = match axis {
        ListAxis::Grid { columns } => format!("GridCells.Fixed({columns})"),
        ListAxis::AutoGrid { min_cell_width } => {
            format!("GridCells.Adaptive(minSize = {min_cell_width}.dp)")
        }
        ListAxis::Masonry { columns } => format!("StaggeredGridCells.Fixed({columns})"),
        ListAxis::Vertical | ListAxis::Horizontal => "GridCells.Fixed(1)".to_owned(),
    };
    let is_masonry = matches!(axis, ListAxis::Masonry { .. });
    let list_name = if is_masonry {
        "LazyVerticalStaggeredGrid"
    } else {
        "LazyVerticalGrid"
    };
    let spacing_arguments = if pieces.spacing > 0.0 {
        if is_masonry {
            format!(
                ", horizontalArrangement = Arrangement.spacedBy({spacing}.dp), verticalItemSpacing = {spacing}.dp"
            )
        } else {
            format!(
                ", horizontalArrangement = Arrangement.spacedBy({spacing}.dp), verticalArrangement = Arrangement.spacedBy({spacing}.dp)"
            )
        }
    } else {
        String::new()
    };
    out.line_at(
        list_depth,
        format_args!(
            "{list_name}({state_parameter}modifier = nexaSafeAreaScrollViewportModifier(\"{}\").fillMaxWidth(), contentPadding = LocalNexaScrollContentPadding.current, userScrollEnabled = {}, columns = {grid_cells}{spacing_arguments}) {{",
            safe_area_axis(axis),
            scroll_enabled_expression(
                scroll_state,
                pieces.scroll_enabled,
                pieces.refresh.is_some(),
            ),
        ),
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
                        Some(collection),
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
        axis,
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

fn flat_list_requires_count(pieces: &FlatPieces<'_>) -> bool {
    pieces.on_end_reached.is_some() || pieces.reverse_layout || pieces.page_snap
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

fn emit_leading_visible_data_item_index(
    list_state: &str,
    sticky_header_offset: usize,
    reverse_layout: bool,
    axis: ListAxis,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!("fun leadingVisibleDataItemIndex(): Int {{"),
    );
    out.line_at(
        depth + 1,
        format_args!("val layoutInfo = {list_state}.layoutInfo"),
    );
    let (item_offset, item_size) = list_item_info_geometry(axis);
    out.line_at(
        depth + 1,
        format_args!("return nexaLeadingVisibleDataItemIndex("),
    );
    out.line_at(
        depth + 2,
        format_args!("visibleItems = layoutInfo.visibleItemsInfo,"),
    );
    out.line_at(
        depth + 2,
        format_args!("fallback = {list_state}.firstVisibleItemIndex,"),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "stickyHeaderOffset = {sticky_header_offset}, reverseLayout = {reverse_layout},"
        ),
    );
    out.line_at(depth + 2, format_args!("viewportStartOffset = layoutInfo.viewportStartOffset, viewportEndOffset = layoutInfo.viewportEndOffset,"));
    out.line_at(depth + 2, format_args!("itemIndex = {{ it.index }}, itemOffset = {{ it.{item_offset} }}, itemSize = {{ it.{item_size} }},"));
    out.line_at(depth + 1, format_args!(")"));
    out.line_at(depth, format_args!("}}"));
}

fn list_item_info_geometry(axis: ListAxis) -> (&'static str, &'static str) {
    match axis {
        ListAxis::Grid { .. } | ListAxis::AutoGrid { .. } | ListAxis::Masonry { .. } => {
            ("offset.y", "size.height")
        }
        ListAxis::Vertical | ListAxis::Horizontal => ("offset", "size"),
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
    let list_task_scope = end_actions
        .filter(|actions| nexa_ir::walk::actions_contain_task_launch(actions))
        .map(|_| format!("nexaListTaskScope{list_state}"));
    let scroll_position = pieces.scroll_position.map(state_name);
    let scroll_actions = pieces.on_scroll;
    let sticky_header_offset =
        usize::from(pieces.sticky_header.is_some() && matches!(axis, ListAxis::Vertical));
    let (item_offset, item_size) = list_item_info_geometry(axis);
    let state_initializer = match axis {
        ListAxis::Grid { .. } | ListAxis::AutoGrid { .. } => "rememberLazyGridState",
        ListAxis::Masonry { .. } => "rememberLazyStaggeredGridState",
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
        let initial_position = if sticky_header_offset > 0 {
            format!(
                "if ({initial_position} == 0) 0 else ({initial_position} + {sticky_header_offset})"
            )
        } else {
            initial_position
        };
        format!("{state_initializer}(initialFirstVisibleItemIndex = {initial_position})")
    } else {
        format!("{state_initializer}()")
    };
    out.line_at(
        depth,
        format_args!("val {list_state} = {state_initializer}"),
    );
    if let Some(actions) = pieces.on_scroll_begin_drag {
        let is_dragged = format!("nexaListIsDragged{list_state}");
        out.line_at(
            depth,
            format_args!(
                "val {is_dragged} = {list_state}.interactionSource.collectIsDraggedAsState()"
            ),
        );
        out.line_at(depth, format_args!("LaunchedEffect({is_dragged}.value) {{"));
        out.line_at(depth + 1, format_args!("if ({is_dragged}.value) {{"));
        render_actions(actions, depth + 2, out);
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth, format_args!("}}"));
    }
    if let Some(task_scope) = list_task_scope.as_deref() {
        out.line_at(
            depth,
            format_args!("val {task_scope} = rememberCoroutineScope()"),
        );
    }
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
            format_args!(
                "var {marker} by rememberSaveable({list_count}) {{ mutableIntStateOf(-1) }}"
            ),
        );
        out.line_at(
            depth,
            format_args!("LaunchedEffect({list_state}, {list_count}) {{"),
        );
        if pieces.page_snap {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ {list_state}.isScrollInProgress }}.distinctUntilChanged().collect {{ isScrolling ->"
            ));
            out.line_at(depth + 2, format_args!("if (!isScrolling) {{"));
            let end_visible = if pieces.page_snap {
                format!("{list_state}.settledPage >= {list_count} - 1")
            } else if sticky_header_offset > 0 {
                let final_item = format!("{list_count} + {sticky_header_offset} - 1");
                format!(
                    "run {{ val layoutInfo = {list_state}.layoutInfo; val visibleItems = layoutInfo.visibleItemsInfo; val stickyHeader = visibleItems.firstOrNull {{ it.index == 0 }}; visibleItems.any {{ item -> item.index >= {final_item} && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset && (stickyHeader == null || item.offset + item.size <= stickyHeader.offset || item.offset >= stickyHeader.offset + stickyHeader.size) }} }}"
                )
            } else {
                let final_item = format!("{list_count} + {sticky_header_offset} - 1");
                format!(
                    "run {{ val layoutInfo = {list_state}.layoutInfo; layoutInfo.visibleItemsInfo.any {{ item -> item.index >= {final_item} && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset }} }}"
                )
            };
            out.line_at(
                depth + 3,
                format_args!("val finalItemVisible = {end_visible}"),
            );
            out.line_at(
                depth + 3,
                format_args!(
                    "if ({list_count} > 0 && finalItemVisible && {marker} != {list_count}) {{"
                ),
            );
            if let Some(task_scope) = list_task_scope.as_deref() {
                out.line_at(depth + 4, format_args!("val nexaTaskScope = {task_scope}"));
            }
            out.line_at(depth + 4, format_args!("{marker} = {list_count}"));
            render_actions(actions, depth + 4, out);
            indent(out, depth + 3);
            out.push_str("}\n");
            indent(out, depth + 2);
            out.push_str("}\n");
        } else {
            let final_item = format!("{list_count} + {sticky_header_offset} - 1");
            let final_visible_expression = if sticky_header_offset > 0 {
                format!(
                    "run {{ val layoutInfo = {list_state}.layoutInfo; val visibleItems = layoutInfo.visibleItemsInfo; val stickyHeader = visibleItems.firstOrNull {{ it.index == 0 }}; visibleItems.any {{ item -> item.index >= {final_item} && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset && (stickyHeader == null || item.offset + item.size <= stickyHeader.offset || item.offset >= stickyHeader.offset + stickyHeader.size) }} }}"
                )
            } else {
                format!(
                    "run {{ val layoutInfo = {list_state}.layoutInfo; layoutInfo.visibleItemsInfo.any {{ item -> item.index >= {final_item} && item.{item_offset} < layoutInfo.viewportEndOffset && item.{item_offset} + item.{item_size} > layoutInfo.viewportStartOffset }} }}"
                )
            };
            out.line_at(depth + 1, format_args!("snapshotFlow {{ {final_visible_expression} }}.distinctUntilChanged().collect {{ finalItemVisible ->"
            ));
            out.line_at(
                depth + 2,
                format_args!(
                    "if ({list_count} > 0 && finalItemVisible && {marker} != {list_count}) {{"
                ),
            );
            if let Some(task_scope) = list_task_scope.as_deref() {
                out.line_at(depth + 3, format_args!("val nexaTaskScope = {task_scope}"));
            }
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
        let has_initial_position = format!("nexaHasInitialScrollPosition{list_state}");
        let scroll_position_echo = format!("nexaReportedScrollPosition{list_state}");
        let scroll_was_in_progress = format!("nexaScrollWasInProgress{list_state}");
        out.line_at(
            depth,
            format_args!(
                "val {has_initial_position} = remember({list_state}) {{ booleanArrayOf(false) }}"
            ),
        );
        if scroll_position.is_some() {
            out.line_at(
                depth,
                format_args!("var {scroll_position_echo} by remember {{ mutableIntStateOf(-1) }}"),
            );
        }
        if let Some(scroll_marker) = scroll_marker.as_deref() {
            out.line_at(
                depth,
                format_args!("var {scroll_marker} by remember {{ mutableIntStateOf(-1) }}"),
            );
            out.line_at(
                depth,
                format_args!("val {scroll_was_in_progress} = remember({list_state}) {{ booleanArrayOf(false) }}"),
            );
        }
        out.line_at(depth, format_args!("LaunchedEffect({list_state}) {{"));
        if pieces.page_snap {
            out.line_at(depth + 1, format_args!("snapshotFlow {{ if ({list_state}.isScrollInProgress) -1 else {list_state}.settledPage }}.distinctUntilChanged().collect {{ firstVisiblePosition ->"
            ));
            out.line_at(depth + 2, format_args!("if (firstVisiblePosition >= 0) {{"));
            indent(out, depth + 3);
        } else {
            emit_leading_visible_data_item_index(
                list_state,
                sticky_header_offset,
                pieces.reverse_layout,
                axis,
                depth + 1,
                out,
            );
            let first_visible_expression = "leadingVisibleDataItemIndex()";
            if scroll_actions.is_some() {
                out.line_at(depth + 1, format_args!("snapshotFlow {{ {list_state}.isScrollInProgress to {first_visible_expression} }}.distinctUntilChanged().collect {{ (isScrollInProgress, firstVisiblePosition) ->"
                ));
            } else {
                out.line_at(depth + 1, format_args!("snapshotFlow {{ {first_visible_expression} }}.distinctUntilChanged().collect {{ firstVisiblePosition ->"
                ));
            }
            indent(out, depth + 2);
        }
        let body_indent = depth + if pieces.page_snap { 3 } else { 2 };
        let visible_item_position = if sticky_header_offset > 0 {
            format!("(firstVisiblePosition - {sticky_header_offset}).coerceAtLeast(0)")
        } else {
            "firstVisiblePosition".to_owned()
        };
        let first_visible = if pieces.reverse_layout {
            format!(
                "({} - 1 - {visible_item_position}).coerceIn(0, ({} - 1).coerceAtLeast(0))",
                list_count.unwrap_or("0"),
                list_count.unwrap_or("0"),
            )
        } else {
            visible_item_position
        };
        out.line_at(
            body_indent,
            format_args!("val isInitialScrollPosition = !{has_initial_position}[0]"),
        );
        out.line_at(
            body_indent,
            format_args!("{has_initial_position}[0] = true"),
        );
        if !pieces.page_snap {
            if let Some(scroll_marker) = scroll_marker.as_deref() {
                out.line_at(
                    body_indent,
                    format_args!("val previousScrollWasInProgress = {scroll_was_in_progress}[0]"),
                );
                out.line_at(
                    body_indent,
                    format_args!("{scroll_was_in_progress}[0] = isScrollInProgress"),
                );
                out.line_at(
                    body_indent,
                    format_args!("if (isInitialScrollPosition) {scroll_marker} = {first_visible}"),
                );
            }
        } else if let Some(scroll_marker) = scroll_marker.as_deref() {
            out.line_at(
                body_indent,
                format_args!("if (!{has_initial_position}[0]) {scroll_marker} = {first_visible}"),
            );
        }
        out.line_at(
            body_indent,
            format_args!("if (!isInitialScrollPosition) {{"),
        );
        let callback_indent = body_indent + 1;
        if let Some(scroll_position) = scroll_position.as_deref() {
            out.line_at(
                callback_indent,
                format_args!("if ({first_visible} != {scroll_position}) {{"),
            );
            out.line_at(
                callback_indent + 1,
                format_args!("{scroll_position_echo} = {first_visible}"),
            );
            out.line_at(
                callback_indent + 1,
                format_args!("{scroll_position} = {first_visible}"),
            );
            indent(out, callback_indent);
            out.push_str("}\n");
        }
        if let (Some(scroll_marker), Some(scroll_actions)) =
            (scroll_marker.as_deref(), scroll_actions)
        {
            out.line_at(
                callback_indent,
                format_args!("if ({first_visible} != {scroll_marker}) {{"),
            );
            let body_depth = callback_indent + 1;
            if pieces.page_snap {
                out.line_at(
                    body_depth,
                    format_args!("{scroll_marker} = {first_visible}"),
                );
                render_actions(scroll_actions, body_depth, out);
            } else {
                let callback_count = format!("nexaScrollCallbackCount{list_state}");
                let previous_position = format!("nexaPreviousScrollPosition{list_state}");
                out.line_at(
                    body_depth,
                    format_args!("val {previous_position} = {scroll_marker}"),
                );
                out.line_at(body_depth, format_args!("val {callback_count} = if (isScrollInProgress || previousScrollWasInProgress) kotlin.math.abs({first_visible} - {previous_position}).coerceAtLeast(1) else 1"));
                out.line_at(
                    body_depth,
                    format_args!("{scroll_marker} = {first_visible}"),
                );
                out.line_at(body_depth, format_args!("repeat({callback_count}) {{"));
                render_actions(scroll_actions, body_depth + 1, out);
                indent(out, body_depth);
                out.push_str("}\n");
            }
            indent(out, callback_indent);
            out.push_str("}\n");
        }
        indent(out, body_indent);
        out.push_str("}\n");
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
            let target = if sticky_header_offset > 0 {
                format!("if ({target} == 0) 0 else ({target} + {sticky_header_offset})")
            } else {
                target
            };
            out.line_at(depth + 1, format_args!("val target = {target}"));
            let current_position = if pieces.page_snap {
                format!("{list_state}.settledPage")
            } else if pieces.reverse_layout {
                format!("{list_state}.layoutInfo.visibleItemsInfo.maxOfOrNull {{ it.index }} ?: -1")
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
                format_args!("if ({scroll_position_echo} == {scroll_position}) {{"),
            );
            out.line_at(depth + 2, format_args!("{scroll_position_echo} = -1"));
            out.line_at(
                depth + 1,
                format_args!("}} else if (target != {current_position}) {scroll_to}"),
            );
            indent(out, depth);
            out.push_str("}\n");
        }
    }
}

fn flat_count_expression(count: &FlatCount<'_>) -> String {
    match count {
        FlatCount::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        FlatCount::Items { collection, .. } => format!("{collection}.size"),
    }
}

fn render_list_component_ref(
    component_ref: &Expr,
    state: &str,
    axis: ListAxis,
    page_snap: bool,
    count: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let id = out.next_id();
    let reference = format!("nexaFastListRef{id}");
    let scope = format!("nexaFastListRefScope{id}");
    let density = format!("nexaFastListRefDensity{id}");
    let count = format!("({count}).coerceAtLeast(0)");
    let (item_offset, item_size) = list_item_info_geometry(axis);
    let scroll_to_index = if page_snap {
        format!(
            "if (animated) {state}.animateScrollToPage(index.coerceIn(0, ({count} - 1).coerceAtLeast(0))) else {state}.scrollToPage(index.coerceIn(0, ({count} - 1).coerceAtLeast(0)))"
        )
    } else {
        format!(
            "run {{ val targetIndex = index.coerceIn(0, ({count} - 1).coerceAtLeast(0)); if (animated) {state}.animateScrollToItem(targetIndex) else {state}.scrollToItem(targetIndex); val targetItem = {state}.layoutInfo.visibleItemsInfo.firstOrNull {{ it.index == targetIndex }}; if (targetItem != null) {{ val viewportStart = {state}.layoutInfo.viewportStartOffset; val viewportEnd = {state}.layoutInfo.viewportEndOffset; val availableSpace = (viewportEnd - viewportStart - targetItem.{item_size}).coerceAtLeast(0); val targetOffset = viewportStart + (availableSpace * viewPosition).toInt(); val delta = (targetItem.{item_offset} - targetOffset).toFloat(); if (animated) {state}.animateScrollBy(delta) else {state}.scrollBy(delta) }} }}"
        )
    };
    let scroll_to_top = if page_snap {
        format!("if (animated) {state}.animateScrollToPage(0) else {state}.scrollToPage(0)")
    } else {
        format!("if (animated) {state}.animateScrollToItem(0) else {state}.scrollToItem(0)")
    };
    let scroll_to_end = if page_snap {
        format!(
            "if (animated) {state}.animateScrollToPage(({count} - 1).coerceAtLeast(0)) else {state}.scrollToPage(({count} - 1).coerceAtLeast(0))"
        )
    } else {
        format!(
            "if (animated) {state}.animateScrollToItem(({count} - 1).coerceAtLeast(0)) else {state}.scrollToItem(({count} - 1).coerceAtLeast(0))"
        )
    };
    let scroll_to_offset = if page_snap {
        let viewport_axis = match axis {
            ListAxis::Horizontal => "width",
            _ => "height",
        };
        format!(
            "if (animated) {state}.animateScrollToPage((with({density}) {{ offset.toFloat().dp.roundToPx() }} / {state}.layoutInfo.viewportSize.{viewport_axis}.coerceAtLeast(1)).coerceAtLeast(0)) else {state}.scrollToPage((with({density}) {{ offset.toFloat().dp.roundToPx() }} / {state}.layoutInfo.viewportSize.{viewport_axis}.coerceAtLeast(1)).coerceAtLeast(0))"
        )
    } else {
        format!(
            "if (animated) {state}.animateScrollToItem(0, with({density}) {{ offset.toFloat().dp.roundToPx() }}.coerceAtLeast(0)) else {state}.scrollToItem(0, with({density}) {{ offset.toFloat().dp.roundToPx() }}.coerceAtLeast(0))"
        )
    };
    out.line_at(
        depth,
        format_args!("val {reference} = {}", expression(component_ref)),
    );
    out.line_at(
        depth,
        format_args!("val {scope} = rememberCoroutineScope()"),
    );
    out.line_at(depth, format_args!("val {density} = LocalDensity.current"));
    out.line_at(depth, format_args!("SideEffect {{"));
    out.line_at(depth + 1, format_args!("{reference}.attach("));
    out.line_at(
        depth + 2,
        format_args!("scrollToIndex = {{ index, animated, viewPosition -> {scope}.launch {{ {scroll_to_index} }} }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("scrollToTop = {{ animated -> {scope}.launch {{ {scroll_to_top} }} }},"),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "scrollToEnd = {{ animated -> if ({count} > 0) {scope}.launch {{ {scroll_to_end} }} }},"
        ),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "scrollToOffset = {{ offset, animated -> {scope}.launch {{ {scroll_to_offset} }} }}"
        ),
    );
    out.line_at(depth + 1, format_args!(")"));
    out.line_at(depth, format_args!("}}"));
    out.line_at(
        depth,
        format_args!("DisposableEffect({reference}, {state}) {{"),
    );
    out.line_at(
        depth + 1,
        format_args!("onDispose {{ {reference}.detach() }}"),
    );
    out.line_at(depth, format_args!("}}"));
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
    let modifier = row_modifier(sizing, axis);
    let horizontal_alignment = if children.len() > 1 {
        "CenterHorizontally"
    } else {
        "Start"
    };
    let vertical_arrangement = if matches!(sizing, ListRowSizing::PagerViewport) {
        ", verticalArrangement = Arrangement.Top"
    } else {
        ""
    };
    out.line_at(
        depth,
        format_args!(
            "NexaFastListRowPrimitive(modifier = {modifier}, horizontalAlignment = Alignment.{horizontal_alignment}{vertical_arrangement}) {{"
        ),
    );
    for (index, child) in children.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        render_node(child, module, features, depth + 1, out);
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn row_modifier(sizing: ListRowSizing, axis: ListAxis) -> String {
    match sizing {
        ListRowSizing::Content => "Modifier".to_owned(),
        ListRowSizing::Viewport => "Modifier.fillParentMaxSize()".to_owned(),
        ListRowSizing::PagerViewport => "Modifier.fillMaxSize()".to_owned(),
        ListRowSizing::Fixed(item_extent) => {
            let extent = format_float(item_extent);
            match axis {
                // `rowHeight` sets the cross-axis extent for a horizontal list.
                // Let row content choose its own width so wider children are not clipped.
                ListAxis::Horizontal => format!("Modifier.height({extent}.dp)"),
                ListAxis::Vertical
                | ListAxis::Grid { .. }
                | ListAxis::AutoGrid { .. }
                | ListAxis::Masonry { .. } => {
                    format!("Modifier.fillMaxWidth().height({extent}.dp)")
                }
            }
        }
    }
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
        render_swipe_row(row, module, features, depth + 1, out);
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
    let (
        Some(callback),
        Some(index),
        Some(list_state),
        Some(dragged_index_state),
        Some(drag_offset_state),
    ) = (
        row.on_move,
        row.index,
        row.list_state,
        row.dragged_index_state,
        row.drag_offset_state,
    )
    else {
        render_swipe_row(row, module, features, depth, out);
        return;
    };
    let header_index_offset = row.list_index_offset;
    out.line_at(
        depth,
        format_args!(
            "val isDragging = {dragged_index_state}.intValue == {}",
            state_name(index)
        ),
    );
    out.line_at(
        depth,
        format_args!("Box(modifier = Modifier.fillMaxWidth()"),
    );
    if row.animate_item {
        // Animated item offsets change while the collection is being reordered. Keep
        // the modifier attached but disable placement motion until the active drag ends.
        out.line_at(
            depth + 1,
            format_args!(
                ".animateItem(placementSpec = if ({}.intValue < 0) androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>() else null)",
                dragged_index_state
            ),
        );
    }
    out.line_at(
        depth + 1,
        format_args!(
            ".pointerInput({list_state}, {}) {{",
            expression(&callback.enabled)
        ),
    );
    // Keep pointerInput outside the translated graphics layer. Compose reports
    // pointer positions in the input node's local coordinates; placing it
    // inside the layer makes those coordinates move with the dragged row and
    // undercounts the drag distance as the row is translated.
    // Observe the Initial pass so the nested SwipeToDismissBox cannot claim the
    // long-press drag before the reorder handler receives it.
    out.line_at(depth + 2, format_args!("awaitEachGesture {{"));
    out.line_at(depth + 3, format_args!("val down = awaitFirstDown("));
    out.line_at(depth + 4, format_args!("requireUnconsumed = false,"));
    out.line_at(
        depth + 4,
        format_args!("pass = androidx.compose.ui.input.pointer.PointerEventPass.Initial,"),
    );
    out.line_at(depth + 3, format_args!(")"));
    out.line_at(
        depth + 3,
        format_args!("val releasedBeforeLongPress = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {{"),
    );
    out.line_at(depth + 4, format_args!("while (true) {{"));
    out.line_at(
        depth + 5,
        format_args!("val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)"),
    );
    out.line_at(
        depth + 5,
        format_args!("val change = event.changes.firstOrNull {{ it.id == down.id }} ?: return@withTimeoutOrNull true"),
    );
    out.line_at(
        depth + 5,
        format_args!("if (!change.pressed) return@withTimeoutOrNull true"),
    );
    out.line_at(
        depth + 5,
        format_args!("if (abs(change.position.x - down.position.x) > viewConfiguration.touchSlop || abs(change.position.y - down.position.y) > viewConfiguration.touchSlop) return@withTimeoutOrNull true"),
    );
    out.line_at(depth + 4, format_args!("}}"));
    out.line_at(depth + 4, format_args!("false"));
    out.line_at(depth + 3, format_args!("}}"));
    out.line_at(
        depth + 3,
        format_args!("if (releasedBeforeLongPress != null) return@awaitEachGesture"),
    );
    out.line_at(
        depth + 3,
        format_args!("{drag_offset_state}.floatValue = 0f"),
    );
    out.line_at(
        depth + 3,
        format_args!("{dragged_index_state}.intValue = {}", state_name(index)),
    );
    out.line_at(
        depth + 3,
        format_args!("var draggedIndex = {}", state_name(index)),
    );
    out.line_at(
        depth + 3,
        format_args!("var previousPosition = down.position"),
    );
    out.line_at(depth + 3, format_args!("try {{"));
    out.line_at(depth + 4, format_args!("while (true) {{"));
    out.line_at(
        depth + 5,
        format_args!("val event = awaitPointerEvent(androidx.compose.ui.input.pointer.PointerEventPass.Initial)"),
    );
    out.line_at(
        depth + 5,
        format_args!("val change = event.changes.firstOrNull {{ it.id == down.id }} ?: break"),
    );
    out.line_at(depth + 5, format_args!("if (!change.pressed) {{"));
    out.line_at(depth + 6, format_args!("break"));
    out.line_at(depth + 5, format_args!("}}"));
    out.line_at(
        depth + 5,
        format_args!("val dragAmount = change.position - previousPosition"),
    );
    out.line_at(depth + 5, format_args!("change.consume()"));
    out.line_at(
        depth + 5,
        format_args!("{drag_offset_state}.floatValue += dragAmount.y"),
    );
    out.line_at(
        depth + 5,
        format_args!("previousPosition = change.position"),
    );
    out.line_at(
        depth + 5,
        format_args!("val visibleItems = {list_state}.layoutInfo.visibleItemsInfo"),
    );
    out.line_at(
        depth + 5,
        format_args!("val draggedItem = visibleItems.firstOrNull {{ it.index == draggedIndex + {header_index_offset} }}"),
    );
    out.line_at(depth + 5, format_args!("if (draggedItem != null) {{"));
    out.line_at(
        depth + 6,
        format_args!("val draggedCenter = draggedItem.offset + draggedItem.size / 2 + {drag_offset_state}.floatValue.toInt()"),
    );
    out.line_at(depth + 6, format_args!("var targetIndex = -1"));
    out.line_at(depth + 6, format_args!("var targetCenter = 0"));
    out.line_at(
        depth + 6,
        format_args!("var targetDistance = Int.MAX_VALUE"),
    );
    out.line_at(
        depth + 6,
        format_args!("for (visibleIndex in visibleItems.indices) {{"),
    );
    out.line_at(
        depth + 7,
        format_args!("val item = visibleItems[visibleIndex]"),
    );
    out.line_at(
        depth + 7,
        format_args!("if (item.index < {header_index_offset} || item.index >= {header_index_offset} + {}) continue", row.reset_key),
    );
    out.line_at(
        depth + 7,
        format_args!("val itemCenter = item.offset + item.size / 2"),
    );
    out.line_at(
        depth + 7,
        format_args!("val distance = abs(itemCenter - draggedCenter)"),
    );
    out.line_at(depth + 7, format_args!("if (distance < targetDistance) {{"));
    out.line_at(depth + 8, format_args!("targetDistance = distance"));
    out.line_at(depth + 8, format_args!("targetCenter = itemCenter"));
    out.line_at(
        depth + 8,
        format_args!("targetIndex = item.index - {header_index_offset}"),
    );
    out.line_at(depth + 7, format_args!("}}"));
    out.line_at(depth + 6, format_args!("}}"));
    out.line_at(
        depth + 6,
        format_args!("if (targetIndex >= 0 && targetIndex != draggedIndex) {{"),
    );
    out.line_at(
        depth + 7,
        format_args!("val previousCenter = draggedItem.offset + draggedItem.size / 2"),
    );
    out.line_at(
        depth + 7,
        format_args!("val {}: Int = draggedIndex", state_name(&callback.from)),
    );
    out.line_at(
        depth + 7,
        format_args!("val {}: Int = targetIndex", state_name(&callback.to)),
    );
    render_actions(&callback.actions, depth + 7, out);
    out.line_at(depth + 7, format_args!("draggedIndex = targetIndex"));
    out.line_at(
        depth + 7,
        format_args!("{dragged_index_state}.intValue = targetIndex"),
    );
    out.line_at(
        depth + 7,
        format_args!("val layoutShift = (targetCenter - previousCenter).toFloat()"),
    );
    out.line_at(
        depth + 7,
        format_args!("{drag_offset_state}.floatValue -= layoutShift"),
    );
    if row.animate_item {
        out.line_at(
            depth + 7,
            format_args!("previousPosition = androidx.compose.ui.geometry.Offset(previousPosition.x, previousPosition.y - layoutShift)"),
        );
    }
    out.line_at(depth + 6, format_args!("}}"));
    out.line_at(depth + 5, format_args!("}}"));
    out.line_at(depth + 4, format_args!("}}"));
    out.line_at(depth + 3, format_args!("}} finally {{"));
    out.line_at(
        depth + 4,
        format_args!("{drag_offset_state}.floatValue = 0f"),
    );
    out.line_at(
        depth + 4,
        format_args!("{dragged_index_state}.intValue = -1"),
    );
    out.line_at(depth + 3, format_args!("}}"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth + 1, format_args!(".graphicsLayer {{"));
    out.line_at(
        depth + 2,
        format_args!("translationY = if (isDragging) {drag_offset_state}.floatValue else 0f"),
    );
    out.line_at(
        depth + 2,
        format_args!("scaleX = if (isDragging) 1.02f else 1f"),
    );
    out.line_at(
        depth + 2,
        format_args!("scaleY = if (isDragging) 1.02f else 1f"),
    );
    out.line_at(
        depth + 2,
        format_args!("shadowElevation = if (isDragging) 8.dp.toPx() else 0f"),
    );
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(".zIndex(if (isDragging) 1f else 0f)"),
    );
    out.line_at(depth + 1, format_args!(") {{"));
    // Keep reorder and swipe actions mutually exclusive so the swipe gesture
    // cannot consume a long press while row reordering is enabled.
    render_swipe_row(
        FlatRowContent {
            swipe_actions: None,
            ..row
        },
        module,
        features,
        depth + 2,
        out,
    );
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
}

fn render_swipe_row(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if row.native_list_row {
        let position = row.index.unwrap_or("itemPosition");
        out.line_at(
            depth,
            format_args!(
                "NexaNativeListRowPrimitive(position = {}, count = {}) {{",
                state_name(position),
                row.reset_key
            ),
        );
        render_swipe_row_content(
            FlatRowContent {
                native_list_row: false,
                native_row_divider: false,
                ..row
            },
            module,
            features,
            depth + 1,
            out,
        );
        out.push('\n');
        if row.native_row_divider {
            out.line_at(
                depth + 1,
                format_args!(
                    "if ({} + 1 < {}) NexaNativeListDividerPrimitive()",
                    state_name(position),
                    row.reset_key
                ),
            );
        }
        out.line_at(depth, format_args!("}}"));
        return;
    }
    render_swipe_row_content(row, module, features, depth, out);
}

fn render_swipe_row_content(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(actions) = row.swipe_actions else {
        render_row_content(
            row.sizing,
            row.axis,
            row.children,
            module,
            features,
            depth,
            out,
        );
        out.push('\n');
        return;
    };
    out.line_at(depth, format_args!("NexaFastListSwipePrimitive("));
    out.line_at(depth + 1, format_args!("resetKey = {},", row.reset_key));
    out.line_at(depth + 1, format_args!("backgroundContent = {{"));
    render_children(actions, module, features, depth + 3, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("content = {{"));
    render_row_content(
        row.sizing,
        row.axis,
        row.children,
        module,
        features,
        depth + 2,
        out,
    );
    out.line_at(depth + 1, format_args!("}}"));
    indent(out, depth);
    out.push(')');
    out.push('\n');
}

fn render_row_content_with_actions(
    row: FlatRowContent<'_>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if row.animate_item && row.on_move.is_none() {
        out.line_at(
            depth,
            format_args!("Box(modifier = Modifier.fillMaxWidth().animateItem()) {{"),
        );
        render_reorderable_row(
            FlatRowContent {
                animate_item: false,
                ..row
            },
            module,
            features,
            depth + 1,
            out,
        );
        out.push('\n');
        indent(out, depth);
        out.push('}');
        return;
    }
    render_reorderable_row(row, module, features, depth, out);
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

fn sectioned_list_modifier(uses_platform_list: bool) -> &'static str {
    if uses_platform_list {
        "NexaNativeListContainerModifier()"
    } else {
        "Modifier.fillMaxWidth()"
    }
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

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Action, Expr, FastListMove, Module, NumericType, TaskExecutor};

    use super::{
        FlatCount, FlatPieces, FlatRowContent, ListAxis, ListRowSizing, flat_list_requires_count,
        render_list_observers, render_reorderable_row_enabled, row_modifier,
        scroll_enabled_expression, sectioned_list_modifier,
    };

    #[test]
    fn horizontal_row_height_does_not_constrain_row_width() {
        assert_eq!(
            row_modifier(ListRowSizing::Fixed(96.0), ListAxis::Horizontal),
            "Modifier.height(96.dp)"
        );
        assert_eq!(
            row_modifier(ListRowSizing::Fixed(96.0), ListAxis::Vertical),
            "Modifier.fillMaxWidth().height(96.dp)"
        );
    }

    #[test]
    fn sectioned_lists_expand_to_the_available_width_in_both_modes() {
        assert_eq!(sectioned_list_modifier(false), "Modifier.fillMaxWidth()");
        assert_eq!(
            sectioned_list_modifier(true),
            "NexaNativeListContainerModifier()"
        );
    }

    #[test]
    fn refreshable_short_lists_keep_gesture_dispatch_enabled() {
        assert_eq!(scroll_enabled_expression("state", None, true), "true");
        assert_eq!(
            scroll_enabled_expression("state", Some(&Expr::Bool(false)), true),
            "true && false",
            "an explicit scrollEnabled=false still disables list gestures"
        );
        assert_eq!(
            scroll_enabled_expression("state", None, false),
            "(state.canScrollForward || state.canScrollBackward)"
        );
    }

    #[test]
    fn end_reached_once_marker_resets_when_item_count_changes() {
        let count = Expr::Number {
            raw: "20".to_owned(),
            ty: NumericType::Int32,
        };
        let end_actions = [Action::TaskLaunch {
            handle: None,
            executor: TaskExecutor::Main,
            actions: vec![Action::Expression(Expr::Bool(true))],
        }];
        let pieces = FlatPieces {
            axis: ListAxis::Vertical,
            uses_platform_list: false,
            show_dividers: false,
            count: FlatCount::Count(&count),
            index: "index",
            item: None,
            item_extent: None,
            spacing: 0.0,
            scroll_enabled: None,
            key: None,
            scroll_position: None,
            children: &[],
            swipe_actions: None,
            on_end_reached: Some(&end_actions),
            on_scroll: None,
            on_scroll_begin_drag: None,
            on_move: None,
            reverse_layout: false,
            page_snap: false,
            sticky_header: None,
            refresh: None,
            component_ref: None,
        };
        let mut generated = SourceWriter::new();
        render_list_observers(
            &pieces,
            "nexaListState0",
            Some("nexaListCount0"),
            Some("nexaEndReached0"),
            0,
            &mut generated,
        );

        assert!(generated.as_str().contains(
            "var nexaEndReached0 by rememberSaveable(nexaListCount0) { mutableIntStateOf(-1) }"
        ));
        assert!(
            generated
                .as_str()
                .contains("val nexaListTaskScopenexaListState0 = rememberCoroutineScope()")
        );
        assert!(
            generated
                .as_str()
                .contains("val nexaTaskScope = nexaListTaskScopenexaListState0")
        );
        assert!(generated.as_str().contains("nexaTaskScope.launch("));
        assert!(generated.as_str().contains(
            "layoutInfo.visibleItemsInfo.any { item -> item.index >= nexaListCount0 + 0 - 1 && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset }"
        ));
    }

    #[test]
    fn grid_scroll_observers_use_vertical_item_geometry() {
        let count = Expr::Number {
            raw: "12".to_owned(),
            ty: NumericType::Int32,
        };
        let end_actions: [Action; 0] = [];
        for axis in [
            ListAxis::Grid { columns: 2 },
            ListAxis::AutoGrid {
                min_cell_width: 160,
            },
            ListAxis::Masonry { columns: 2 },
        ] {
            let pieces = FlatPieces {
                axis,
                uses_platform_list: false,
                show_dividers: true,
                count: FlatCount::Count(&count),
                index: "index",
                item: None,
                item_extent: None,
                spacing: 4.0,
                scroll_enabled: None,
                key: None,
                scroll_position: Some("position"),
                children: &[],
                swipe_actions: None,
                on_end_reached: Some(&end_actions),
                on_scroll: None,
                on_scroll_begin_drag: None,
                on_move: None,
                reverse_layout: false,
                page_snap: false,
                sticky_header: None,
                refresh: None,
                component_ref: None,
            };
            assert!(flat_list_requires_count(&pieces), "{axis:?}");
            let mut generated = SourceWriter::new();
            render_list_observers(
                &pieces,
                "nexaGridState0",
                Some("nexaGridCount0"),
                Some("nexaEndReached0"),
                0,
                &mut generated,
            );

            assert!(
                generated
                    .as_str()
                    .contains("itemOffset = { it.offset.y }, itemSize = { it.size.height },"),
                "{axis:?}: {}",
                generated.as_str()
            );
            assert!(
                generated.as_str().contains(
                    "item.offset.y < layoutInfo.viewportEndOffset && item.offset.y + item.size.height > layoutInfo.viewportStartOffset"
                ),
                "{axis:?}: {}",
                generated.as_str()
            );
        }
    }

    #[test]
    fn reorder_pointer_input_uses_stationary_coordinates_outside_the_drag_layer() {
        let module = Module {
            app_name: String::new(),
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
        let callback = FastListMove {
            from: "from".to_owned(),
            to: "to".to_owned(),
            enabled: Expr::Bool(true),
            actions: Vec::new(),
        };
        let row = FlatRowContent {
            sizing: ListRowSizing::Content,
            axis: ListAxis::Vertical,
            animate_item: false,
            list_index_offset: 0,
            dragged_index_state: Some("draggedIndex"),
            drag_offset_state: Some("dragOffset"),
            children: &[],
            swipe_actions: None,
            reset_key: "count",
            native_list_row: false,
            native_row_divider: false,
            on_move: Some(&callback),
            index: Some("index"),
            list_state: Some("listState"),
        };
        let mut generated = SourceWriter::new();

        render_reorderable_row_enabled(
            row,
            &module,
            &crate::generator::features::Features::default(),
            0,
            &mut generated,
        );

        let source = generated.as_str();
        let drag_value = source
            .find("val isDragging = draggedIndex.intValue == ")
            .unwrap_or_else(|| panic!("drag state declaration missing:\n{source}"));
        let pointer_input = source.find(".pointerInput(").unwrap();
        let translated_layer = source.find(".graphicsLayer {").unwrap();
        let z_index = source.find(".zIndex(if (isDragging) 1f else 0f)").unwrap();
        assert!(
            drag_value < pointer_input,
            "drag state must be in scope for both modifier nodes:\n{source}"
        );
        assert!(
            pointer_input < translated_layer,
            "pointer input must remain outside the moving graphics layer so pointer deltas stay in stationary coordinates:\n{source}"
        );
        assert!(translated_layer < z_index, "{source}");
        assert!(source.contains("draggedIndex = targetIndex"));
        assert!(source.contains("draggedIndex.intValue = targetIndex"));
        assert!(source.contains("for (visibleIndex in visibleItems.indices)"));
        assert!(source.contains("targetIndex = item.index - 0"));
        assert!(!source.contains("asSequence()"));
        assert!(source.contains("val layoutShift = (targetCenter - previousCenter).toFloat()"));
        assert!(!source.contains("previousPosition = androidx.compose.ui.geometry.Offset"));

        let keyed_row = FlatRowContent {
            animate_item: true,
            ..row
        };
        let mut keyed_generated = SourceWriter::new();
        render_reorderable_row_enabled(
            keyed_row,
            &module,
            &crate::generator::features::Features::default(),
            0,
            &mut keyed_generated,
        );
        assert!(
            keyed_generated
                .as_str()
                .contains("previousPosition = androidx.compose.ui.geometry.Offset")
        );
        assert!(keyed_generated.as_str().contains(
            ".animateItem(placementSpec = if (draggedIndex.intValue < 0) androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>() else null)"
        ));
    }
}
