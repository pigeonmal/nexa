use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, FastListRefresh, ListAxis, ListSource, Module, Node};

use super::{
    components::render_children, controls::render_actions, expressions::expression,
    features::Features, utils::indent,
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
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let list_id = out.len();
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
            refresh,
            module,
            features,
            depth,
            list_id,
            out,
        );
    }
    let list_state = on_end_reached.map(|_| format!("nexaListState{list_id}"));
    let list_count = on_end_reached.map(|_| format!("nexaListCount{list_id}"));
    if let (Some(list_state), Some(list_count), Some(actions)) =
        (&list_state, &list_count, on_end_reached)
    {
        render_end_reached_setup(
            axis,
            source,
            list_state,
            list_count,
            &format!("nexaEndReached{list_id}"),
            actions,
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

fn render_grid_list(
    columns: u32,
    source: &ListSource,
    item_extent: Option<f32>,
    index: &str,
    item: Option<&str>,
    key: Option<&Expr>,
    children: &[Node],
    on_end_reached: Option<&[Action]>,
    refresh: Option<&FastListRefresh>,
    module: &Module,
    features: &Features,
    depth: usize,
    list_id: usize,
    out: &mut String,
) {
    let list_state = on_end_reached.map(|_| format!("nexaGridState{list_id}"));
    let list_count = on_end_reached.map(|_| format!("nexaGridCount{list_id}"));
    if let (Some(list_state), Some(list_count), Some(actions)) =
        (&list_state, &list_count, on_end_reached)
    {
        render_end_reached_setup(
            ListAxis::Grid { columns },
            source,
            list_state,
            list_count,
            &format!("nexaEndReached{list_id}"),
            actions,
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

fn render_end_reached_setup(
    axis: ListAxis,
    source: &ListSource,
    list_state: &str,
    list_count: &str,
    marker: &str,
    actions: &[Action],
    depth: usize,
    out: &mut String,
) {
    let state_initializer = match axis {
        ListAxis::Grid { .. } => "rememberLazyGridState()",
        ListAxis::Vertical | ListAxis::Horizontal => "rememberLazyListState()",
    };
    let count_expression = source_count_expression(source);
    indent(out, depth);
    out.push_str(&format!("val {list_count} = {count_expression}\n"));
    indent(out, depth);
    out.push_str(&format!("val {list_state} = {state_initializer}\n"));
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

fn source_count_expression(source: &ListSource) -> String {
    match source {
        ListSource::Count(count) => format!("({}).coerceAtLeast(0)", expression(count)),
        ListSource::Items { collection, .. } => format!("{}.size", expression(collection)),
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
