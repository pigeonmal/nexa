use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListSource, Module, Node};

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
        features.uses_sticky_header,
        "androidx.compose.foundation.lazy.stickyHeader",
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

pub(crate) fn render_virtualized_list(
    source: &ListSource,
    axis: ListAxis,
    item_extent: Option<f32>,
    section: Option<&str>,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    on_end_reached: Option<&[Action]>,
    on_scroll: Option<&[Action]>,
    scroll_position: Option<&str>,
    sticky_header: Option<&[Node]>,
    section_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let list_id = out.len();
    if let ListSource::Sections {
        collection,
        element_type,
    } = source
    {
        return render_sectioned_list(
            collection,
            element_type,
            item_extent,
            section.expect("semantic lowering always provides a section binding"),
            index,
            item.expect("semantic lowering always provides an item binding"),
            key,
            children,
            section_header,
            refresh,
            module,
            features,
            depth,
            out,
        );
    }
    if let ListAxis::Grid { columns } = axis {
        return render_grid_list(
            columns,
            source,
            item_extent,
            index,
            item,
            key,
            children,
            on_end_reached,
            on_scroll,
            scroll_position,
            sticky_header,
            refresh,
            module,
            features,
            depth,
            list_id,
            out,
        );
    }
    let list_state = (on_end_reached.is_some() || on_scroll.is_some() || scroll_position.is_some())
        .then(|| format!("nexaListState{list_id}"));
    let list_count = on_end_reached.map(|_| format!("nexaListCount{list_id}"));
    if let Some(list_state) = &list_state {
        let marker = on_end_reached.map(|_| format!("nexaEndReached{list_id}"));
        render_list_observers(
            axis,
            source,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            on_end_reached,
            scroll_position,
            on_scroll,
            depth,
            out,
        );
    }
    let state_parameter = list_state
        .as_deref()
        .map_or_else(String::new, |state| format!("(state = {state})"));
    let list_depth = render_refresh_open(refresh, depth, out);
    indent(out, list_depth);
    out.push_str(
        match axis {
            ListAxis::Vertical => format!("LazyColumn{state_parameter} {{\n"),
            ListAxis::Horizontal => format!("LazyRow{state_parameter} {{\n"),
            ListAxis::Grid { .. } => unreachable!("grid list handled above"),
        }
        .as_str(),
    );
    indent(out, list_depth + 1);
    if let Some(sticky_header) = sticky_header {
        debug_assert!(matches!(axis, ListAxis::Vertical));
        out.push_str("stickyHeader {\n");
        render_children(sticky_header, module, features, list_depth + 2, out);
        out.push('\n');
        indent(out, list_depth + 1);
        out.push_str("}\n");
        indent(out, list_depth + 1);
    }
    match source {
        ListSource::Count(count) => {
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("({}).coerceAtLeast(0)", expression(count)));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = key
                .map(|key| render_key(key, index, item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
        }
        ListSource::Items {
            collection,
            element_type,
        } => {
            let item = item.unwrap_or("item");
            let collection = expression(collection);
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{collection}.size"));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = key
                .map(|key| render_key(key, index, Some(item), Some(&collection), "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
            indent(out, list_depth + 2);
            out.push_str(&format!(
                "val {}: {} = {collection}[{}]\n",
                state_name(item),
                element_type.kotlin(),
                state_name(index)
            ));
        }
        ListSource::Sections { .. } => unreachable!("sectioned list handled above"),
    }
    render_row_content(
        item_extent,
        axis,
        children,
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
    render_refresh_close(refresh, depth, out);
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
    source: &ListSource,
    item_extent: Option<f32>,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    on_end_reached: Option<&[Action]>,
    on_scroll: Option<&[Action]>,
    scroll_position: Option<&str>,
    _sticky_header: Option<&[Node]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    list_id: usize,
    out: &mut String,
) {
    let list_state = (on_end_reached.is_some() || on_scroll.is_some() || scroll_position.is_some())
        .then(|| format!("nexaGridState{list_id}"));
    let list_count = on_end_reached.map(|_| format!("nexaGridCount{list_id}"));
    if let Some(list_state) = &list_state {
        let marker = on_end_reached.map(|_| format!("nexaEndReached{list_id}"));
        render_list_observers(
            ListAxis::Grid { columns },
            source,
            list_state,
            list_count.as_deref(),
            marker.as_deref(),
            on_end_reached,
            scroll_position,
            on_scroll,
            depth,
            out,
        );
    }
    let state_parameter = list_state
        .as_deref()
        .map_or_else(String::new, |state| format!("state = {state}, "));
    let list_depth = render_refresh_open(refresh, depth, out);
    indent(out, list_depth);
    out.push_str(&format!(
        "LazyVerticalGrid({state_parameter}columns = GridCells.Fixed({columns})) {{\n"
    ));
    indent(out, list_depth + 1);
    match source {
        ListSource::Count(count) => {
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("({}).coerceAtLeast(0)", expression(count)));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = key
                .map(|key| render_key(key, index, item, None, "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
        }
        ListSource::Items {
            collection,
            element_type,
        } => {
            let item = item.unwrap_or("item");
            let collection = expression(collection);
            let count = list_count
                .as_deref()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{collection}.size"));
            out.push_str("items(\n");
            indent(out, list_depth + 2);
            out.push_str(&format!("count = {count},\n"));
            indent(out, list_depth + 2);
            let key = key
                .map(|key| render_key(key, index, Some(item), Some(&collection), "itemPosition"))
                .unwrap_or_else(|| "itemPosition".to_owned());
            out.push_str(&format!("key = {{ itemPosition -> {key} }},\n"));
            indent(out, list_depth + 1);
            out.push_str(&format!(") {{ {} ->\n", state_name(index)));
            indent(out, list_depth + 2);
            out.push_str(&format!(
                "val {}: {} = {collection}[{}]\n",
                state_name(item),
                element_type.kotlin(),
                state_name(index)
            ));
        }
        ListSource::Sections { .. } => unreachable!("sectioned list handled above"),
    }
    render_row_content(
        item_extent,
        ListAxis::Grid { columns },
        children,
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
    render_refresh_close(refresh, depth, out);
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
    source: &ListSource,
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
        let count_expression = source_count_expression(source);
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
            "snapshotFlow {{ {list_state}.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }}.collect {{ lastVisible ->\n"
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
            "snapshotFlow {{ {list_state}.firstVisibleItemIndex }}.collect {{ firstVisible ->\n"
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

fn source_count_expression(source: &ListSource) -> String {
    match source {
        ListSource::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        ListSource::Items { collection, .. } => format!("{}.size", expression(collection)),
        ListSource::Sections { .. } => unreachable!("sectioned list has no flat count"),
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
