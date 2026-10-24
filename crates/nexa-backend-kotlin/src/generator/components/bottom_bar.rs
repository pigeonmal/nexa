use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    BottomBarTab, Capitalization, KeyboardType, Module, Node, ReturnKeyType, ToolbarPlacement,
};

use crate::generator::{
    components::{
        RenderScope, accessibility, controls,
        input::{self, TextInputProps},
        navigation, render_children, render_node,
    },
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    // The DevRuntime host keeps this shared primitive ready for hot reload,
    // so its imports must not depend on the initial module using a large title.
    for value in [
        "androidx.compose.foundation.layout.Box",
        "androidx.compose.foundation.layout.BoxWithConstraints",
        "androidx.compose.foundation.layout.Column",
        "androidx.compose.foundation.layout.fillMaxSize",
        "androidx.compose.foundation.layout.fillMaxWidth",
        "androidx.compose.ui.input.nestedscroll.nestedScroll",
        "androidx.compose.material3.LargeTopAppBar",
        "androidx.compose.material3.TopAppBarDefaults",
        "androidx.compose.material3.rememberTopAppBarState",
        "androidx.compose.ui.graphics.TransformOrigin",
        "androidx.compose.ui.graphics.graphicsLayer",
        "androidx.compose.ui.layout.onSizeChanged",
        "androidx.compose.ui.platform.LocalDensity",
        "androidx.compose.runtime.mutableIntStateOf",
        "androidx.compose.runtime.remember",
        "androidx.compose.ui.text.style.TextOverflow",
        "androidx.compose.ui.text.font.FontWeight",
    ] {
        imports.add(true, value);
    }
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.clickable",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.pager.HorizontalPager",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.pager.rememberPagerState",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        features.uses_page_pager && features.facts.ui.lists.swipe_actions,
        "androidx.compose.runtime.mutableStateOf",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.runtime.rememberCoroutineScope",
    );
    imports.add(features.uses_page_pager, "kotlinx.coroutines.launch");
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.shape.CircleShape",
    );
    imports.add(features.uses_page_pager, "androidx.compose.ui.Alignment");
    imports.add(
        features.uses_page_pager,
        "androidx.compose.ui.graphics.Color",
    );
    imports.add(features.uses_page_pager, "androidx.compose.ui.draw.clip");
    imports.add(features.uses_page_pager, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.background",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.Arrangement",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_page_pager,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_page_pager, "androidx.compose.ui.unit.dp");
    imports.add(
        features.uses_page_pager,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.runtime.saveable.rememberSaveableStateHolder",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.Arrangement",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(features.uses_adaptive_tabs, "androidx.compose.ui.Alignment");
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_adaptive_tabs
            && (features.uses_form || features.facts.ui.bottom_bar.searchable),
        "androidx.compose.foundation.background",
    );
    imports.add(
        features.uses_adaptive_tabs && features.uses_form,
        "androidx.compose.runtime.SideEffect",
    );
    imports.add(
        features.uses_adaptive_tabs && features.uses_form,
        "androidx.compose.ui.platform.LocalView",
    );
    imports.add(
        features.uses_adaptive_tabs && features.uses_form,
        "androidx.compose.ui.graphics.toArgb",
    );
    imports.add(features.uses_adaptive_tabs, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_adaptive_tabs, "androidx.compose.ui.unit.dp");
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.ui.res.stringResource",
    );
    imports.add(
        features.uses_adaptive_tabs,
        "androidx.compose.material3.Text",
    );
    imports.add(
        features.facts.ui.bottom_bar.searchable,
        "androidx.compose.material3.TextField",
    );
    imports.add(
        features.facts.ui.bottom_bar.searchable,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.facts.ui.bottom_bar.searchable,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.facts.ui.bottom_bar.searchable,
        "androidx.compose.foundation.layout.imePadding",
    );
    imports.add(
        features.facts.ui.bottom_bar.searchable,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.text.font.FontWeight",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.unit.dp",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.foundation.layout.BoxWithConstraints",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.input.nestedscroll.nestedScroll",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.material3.LargeTopAppBar",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.material3.TopAppBarDefaults",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.material3.rememberTopAppBarState",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.graphics.TransformOrigin",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.graphics.Color",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.graphics.graphicsLayer",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.platform.LocalDensity",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.runtime.mutableIntStateOf",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.facts.ui.bottom_bar.navigation_title,
        "androidx.compose.ui.text.style.TextOverflow",
    );
    imports.add(features.uses_tab_badge, "androidx.compose.material3.Badge");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_app_bottom_bar(
    state: &str,
    tint: Option<&nexa_ir::ColorExpression>,
    tabs: &[BottomBarTab],
    custom_bar: Option<&[Node]>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let selected = state_name(state);
    let state_holder = format!("{}StateHolder", state_name(state));
    let item_colors = format!("{}BottomBarItemColors", state_name(state));
    out.line_at(
        depth,
        format_args!("val {state_holder} = rememberSaveableStateHolder()"),
    );
    let tint = tint
        .map(crate::generator::colors::expression_for_color)
        .unwrap_or_else(|| "null".to_owned());
    let content_depth = depth + if custom_bar.is_some() { 2 } else { 0 };
    let has_grouped_surface_tab = tabs
        .iter()
        .any(|tab| tab_contains_form(&tab.children) || tab_contains_native_list(&tab.children));
    out.line_at(depth, format_args!("NexaAppBottomBarPrimitive("));
    out.line_at(depth + 1, format_args!("tint = {tint},"));
    if custom_bar.is_some() {
        out.line_at(depth + 1, format_args!("showNavigationSuite = false,"));
    }
    out.line_at(
        depth + 1,
        format_args!("navigationSuiteItems = {{ {item_colors} ->"),
    );
    if custom_bar.is_none() {
        for tab in tabs {
            out.line_at(
                content_depth + 2,
                format_args!(
                    "item(selected = {selected} == {}, onClick = {{ {selected} = {} }}, icon = {{",
                    tab.index, tab.index
                ),
            );
            if let Some(icon) = &tab.icon {
                out.line_at(
                    content_depth + 3,
                    format_args!(
                        "Icon(imageVector = {}, contentDescription = null)",
                        icon.material_reference()
                    ),
                );
            } else {
                out.line_at(depth + 3, format_args!("Spacer(Modifier.size(24.dp))"));
            }
            out.line_at(
                content_depth + 2,
                format_args!(
                    "}}, label = {{ Text(stringResource(R.string.{})) }},",
                    nexa_codegen::names::localization_resource_name(&tab.label)
                ),
            );
            if let Some(badge) = &tab.badge {
                out.line_at(
                    content_depth + 2,
                    format_args!("badge = {{ Badge {{ Text({}) }} }},", kotlin_string(badge)),
                );
            }
            out.line_at(content_depth + 2, format_args!("alwaysShowLabel = true,"));
            out.line_at(content_depth + 2, format_args!("colors = {item_colors},"));
            out.line_at(content_depth + 2, format_args!(")"));
        }
    }
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(") {{"));
    if custom_bar.is_some() {
        out.line_at(depth + 1, format_args!("Column(Modifier.fillMaxSize()) {{"));
        out.line_at(
            depth + 2,
            format_args!("Box(Modifier.weight(1f).fillMaxWidth()) {{"),
        );
    }
    for tab in tabs {
        out.line_at(
            content_depth + 1,
            format_args!("if ({selected} == {}) {{", tab.index),
        );
        out.line_at(
            content_depth + 2,
            format_args!(
                "{state_holder}.SaveableStateProvider(key = {}) {{",
                kotlin_string(&format!("nexa-tab-{}", tab.index))
            ),
        );
        if has_grouped_surface_tab {
            let has_grouped_surface =
                tab_contains_form(&tab.children) || tab_contains_native_list(&tab.children);
            let tab_status_color = if has_grouped_surface {
                "(if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant)"
            } else {
                "MaterialTheme.colorScheme.background"
            };
            out.line_at(
                content_depth + 3,
                format_args!("val nexaTabBackdropView{} = LocalView.current", tab.index),
            );
            out.line_at(
                content_depth + 3,
                format_args!(
                    "val nexaTabBackdropColor{} = {tab_status_color}.toArgb()",
                    tab.index
                ),
            );
            out.line_at(content_depth + 3, format_args!("SideEffect {{"));
            out.line_at(
                content_depth + 4,
                format_args!(
                    "var nexaTabBackdropContext{}: android.content.Context = nexaTabBackdropView{}.context",
                    tab.index, tab.index
                ),
            );
            out.line_at(
                content_depth + 4,
                format_args!(
                    "while (nexaTabBackdropContext{} is android.content.ContextWrapper && nexaTabBackdropContext{} !is android.app.Activity) nexaTabBackdropContext{} = nexaTabBackdropContext{}.baseContext",
                    tab.index, tab.index, tab.index, tab.index
                ),
            );
            out.line_at(
                content_depth + 4,
                format_args!(
                    "(nexaTabBackdropContext{} as? android.app.Activity)?.window?.decorView?.setBackgroundColor(nexaTabBackdropColor{})",
                    tab.index, tab.index
                ),
            );
            out.line_at(content_depth + 3, format_args!("}}"));
        }
        let toolbars = tab_toolbars(&tab.children);
        let has_title_bar = tab.navigation_title.is_some() || !toolbars.is_empty();
        let has_search = tab.search_state.is_some();
        if has_title_bar {
            let has_grouped_surface =
                tab_contains_form(&tab.children) || tab_contains_native_list(&tab.children);
            let has_keyboard_inset = has_search;
            let content_background = if has_grouped_surface {
                "if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant"
            } else {
                "MaterialTheme.colorScheme.background"
            };
            let content_modifier = if has_keyboard_inset {
                format!("Modifier.fillMaxSize().background({content_background}).imePadding()")
            } else if has_grouped_surface {
                format!("Modifier.fillMaxSize().background({content_background})")
            } else {
                "Modifier.fillMaxSize()".to_owned()
            };
            let title = if let Some(title) = &tab.navigation_title {
                let resource = nexa_codegen::names::localization_resource_name(title);
                format!("stringResource(R.string.{resource})")
            } else {
                "\"\"".to_owned()
            };
            out.line_at(content_depth + 3, format_args!("NexaNavigationChrome("));
            out.line_at(
                content_depth + 4,
                format_args!("identity = \"nexa-tab-title-{}\",", tab.index),
            );
            out.line_at(content_depth + 4, format_args!("title = {title},"));
            out.line_at(
                content_depth + 4,
                format_args!(
                    "largeTitle = {},",
                    tab.large_title && tab.navigation_title.is_some()
                ),
            );
            out.line_at(
                content_depth + 4,
                format_args!(
                    "centerTitle = {},",
                    !toolbars
                        .iter()
                        .any(|(placement, _)| *placement == ToolbarPlacement::Leading)
                ),
            );
            out.line_at(
                content_depth + 4,
                format_args!("containerColor = {content_background},"),
            );
            out.line_at(
                content_depth + 4,
                format_args!("modifier = {content_modifier},"),
            );
            if toolbars
                .iter()
                .any(|(placement, _)| *placement == ToolbarPlacement::Leading)
            {
                out.line_at(content_depth + 4, format_args!("navigationIcon = {{"));
                render_tab_toolbar_items(
                    &toolbars,
                    ToolbarPlacement::Leading,
                    module,
                    features,
                    content_depth + 5,
                    out,
                );
                out.line_at(content_depth + 4, format_args!("}},"));
            }
            if toolbars
                .iter()
                .any(|(placement, _)| *placement == ToolbarPlacement::Trailing)
            {
                out.line_at(content_depth + 4, format_args!("actions = {{"));
                render_tab_toolbar_items(
                    &toolbars,
                    ToolbarPlacement::Trailing,
                    module,
                    features,
                    content_depth + 5,
                    out,
                );
                out.line_at(content_depth + 4, format_args!("}},"));
            }
            out.line_at(content_depth + 3, format_args!(") {{"));
            if let Some(search_state) = &tab.search_state {
                input::render_text_input(
                    TextInputProps {
                        state: search_state,
                        placeholder: tab.search_prompt.as_deref().unwrap_or("Search"),
                        keyboard: KeyboardType::Text,
                        secure: false,
                        multiline: false,
                        autofill: None,
                        return_key: Some(ReturnKeyType::Search),
                        autocorrect: Some(false),
                        capitalization: Some(Capitalization::None),
                        focused: None,
                        max_length: None,
                        font: None,
                        min_lines: None,
                        max_lines: Some(1),
                        searchable: true,
                        horizontal_padding: Some(16),
                        weight_in_row: false,
                        actions: &[],
                        on_change: None,
                    },
                    content_depth + 4,
                    out,
                );
                out.push('\n');
            }
            let content_children = without_tab_toolbars(&tab.children);
            render_children(&content_children, module, features, content_depth + 4, out);
            out.push('\n');
            indent(out, content_depth + 3);
            out.push_str("}\n");
        } else if has_search {
            let grouped_surface =
                tab_contains_form(&tab.children) || tab_contains_native_list(&tab.children);
            let keyboard_inset = ".imePadding()";
            let background = if grouped_surface {
                ".background(if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant)"
            } else {
                ""
            };
            out.line_at(
                content_depth + 3,
                format_args!(
                    "Column(modifier = Modifier.fillMaxSize(){keyboard_inset}{background}) {{"
                ),
            );
            input::render_text_input(
                TextInputProps {
                    state: tab.search_state.as_deref().unwrap_or_default(),
                    placeholder: tab.search_prompt.as_deref().unwrap_or("Search"),
                    keyboard: KeyboardType::Text,
                    secure: false,
                    multiline: false,
                    autofill: None,
                    return_key: Some(ReturnKeyType::Search),
                    autocorrect: Some(false),
                    capitalization: Some(Capitalization::None),
                    focused: None,
                    max_length: None,
                    font: None,
                    min_lines: None,
                    max_lines: Some(1),
                    searchable: true,
                    horizontal_padding: Some(16),
                    weight_in_row: false,
                    actions: &[],
                    on_change: None,
                },
                content_depth + 4,
                out,
            );
            out.push('\n');
            render_children(&tab.children, module, features, content_depth + 4, out);
            out.push('\n');
            indent(out, content_depth + 3);
            out.push_str("}\n");
        } else {
            render_children(&tab.children, module, features, content_depth + 3, out);
            out.push('\n');
        }
        indent(out, content_depth + 2);
        out.push_str("}\n");
        out.line_at(content_depth + 1, format_args!("}}"));
    }
    if let Some(custom_bar) = custom_bar {
        out.line_at(depth + 1, format_args!("}}"));
        for child in custom_bar {
            render_node(child, module, features, depth + 2, out);
            out.push('\n');
        }
        out.line_at(depth, format_args!("}}"));
    }
    out.line_at(depth, format_args!("}}"));
}

fn tab_contains_form(children: &[Node]) -> bool {
    children.iter().any(|child| match child {
        Node::Form { .. } => true,
        Node::Appearance { children, .. }
        | Node::Layout { children, .. }
        | Node::Toolbar { children, .. } => tab_contains_form(children),
        _ => false,
    })
}

fn tab_contains_native_list(children: &[Node]) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_ir(
        children,
        &mut |node| {
            if let Node::FastList { plan } = node {
                found |= plan.uses_platform_list();
            }
        },
        &mut |_| {},
    );
    found
}

pub(crate) fn tab_toolbars(children: &[Node]) -> Vec<(ToolbarPlacement, &[Node])> {
    let mut toolbars = Vec::new();
    for child in children {
        match child {
            Node::Toolbar {
                placement,
                children,
            } => toolbars.push((*placement, children.as_slice())),
            Node::Layout { children, .. } => toolbars.extend(tab_toolbars(children)),
            _ => {}
        }
    }
    toolbars
}

pub(crate) fn render_tab_toolbar_items(
    toolbars: &[(ToolbarPlacement, &[Node])],
    placement: ToolbarPlacement,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    for (_, children) in toolbars
        .iter()
        .filter(|(toolbar_placement, _)| *toolbar_placement == placement)
    {
        for child in *children {
            render_tab_toolbar_item(child, module, features, depth, out);
            out.push('\n');
        }
    }
}

fn render_tab_toolbar_item(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match node {
        Node::NavigationLink {
            destination,
            arguments,
            guard,
            children,
        } => navigation::render_toolbar_link(
            *destination,
            arguments,
            guard.as_ref(),
            children,
            &RenderScope { module, features },
            depth,
            out,
        ),
        Node::Picker {
            items,
            state,
            icon,
            item_icons,
            label,
            tint,
        } => controls::render_picker(
            items,
            item_icons,
            state,
            icon.as_ref(),
            label.as_ref(),
            tint.as_ref(),
            false,
            depth,
            out,
        ),
        Node::Accessibility {
            label,
            hint,
            value,
            role,
            children,
        } => accessibility::render_accessibility_with_children(
            label,
            hint.as_ref(),
            value.as_ref(),
            *role,
            children,
            depth,
            out,
            |children, child_depth, out| {
                for child in children {
                    render_tab_toolbar_item(child, module, features, child_depth, out);
                    out.push('\n');
                }
            },
        ),
        _ => render_node(node, module, features, depth, out),
    }
}

pub(crate) fn without_tab_toolbars(children: &[Node]) -> Vec<Node> {
    children
        .iter()
        .filter_map(|child| match child {
            Node::Toolbar { .. } => None,
            Node::Layout {
                kind,
                spacing,
                style,
                children,
            } => Some(Node::Layout {
                kind: *kind,
                spacing: *spacing,
                style: *style,
                children: without_tab_toolbars(children),
            }),
            _ => Some(child.clone()),
        })
        .collect()
}

pub(crate) fn render_page_pager(
    state: &str,
    pages: &[Vec<nexa_ir::Node>],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let selected = state_name(state);
    let page_state = format!("{}Pager", state_name(state));
    let coroutine_scope = format!("{}PagerScope", state_name(state));
    let swipe_row_lock = features.facts.ui.lists.swipe_actions;
    if swipe_row_lock {
        out.line_at(
            depth,
            format_args!("val nexaSwipeRowGestureActive = remember {{ mutableStateOf(false) }}"),
        );
    }
    out.line_at(
        depth,
        format_args!("val {coroutine_scope} = rememberCoroutineScope()"),
    );
    out.line_at(
        depth,
        format_args!(
            "val {page_state} = rememberPagerState(initialPage = {selected}, pageCount = {{ {} }})",
            pages.len()
        ),
    );
    out.line_at(
        depth,
        format_args!(
            "LaunchedEffect({selected}) {{ if ({page_state}.settledPage != {selected}) {page_state}.animateScrollToPage({selected}) }}"
        ),
    );
    out.line_at(
        depth,
        format_args!(
            "LaunchedEffect({page_state}.settledPage) {{ {selected} = {page_state}.settledPage }}"
        ),
    );
    out.line_at(
        depth,
        format_args!("Column(modifier = Modifier.fillMaxSize()) {{"),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "HorizontalPager(state = {page_state}, modifier = Modifier.weight(1f), overscrollEffect = null, userScrollEnabled = {}, pageNestedScrollConnection = object : androidx.compose.ui.input.nestedscroll.NestedScrollConnection {{}}) {{ page ->",
            if swipe_row_lock { "!nexaSwipeRowGestureActive.value" } else { "true" },
        ),
    );
    if swipe_row_lock {
        out.line_at(
            depth + 2,
            format_args!(
                "CompositionLocalProvider(LocalNexaFastListSwipePagerLock provides nexaSwipeRowGestureActive) {{"
            ),
        );
    }
    let page_box_depth = depth + if swipe_row_lock { 3 } else { 2 };
    out.line_at(
        page_box_depth,
        format_args!(
            "Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center, propagateMinConstraints = true) {{"
        ),
    );
    let page_when_depth = page_box_depth + 1;
    out.line_at(page_when_depth, format_args!("when (page) {{"));
    for (index, page) in pages.iter().enumerate() {
        out.line_at(page_when_depth + 1, format_args!("{index} -> {{"));
        render_children(page, module, features, page_when_depth + 2, out);
        out.push('\n');
        indent(out, page_when_depth + 1);
        out.push_str("}\n");
    }
    out.line_at(page_when_depth + 1, format_args!("else -> {{}}"));
    out.line_at(page_when_depth, format_args!("}}"));
    out.line_at(page_box_depth, format_args!("}}"));
    if swipe_row_lock {
        out.line_at(depth + 2, format_args!("}}"));
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(
            "NexaPageIndicatorRow(pageCount = {}, currentPage = {page_state}.settledPage) {{ page ->",
            pages.len()
        ),
    );
    out.line_at(
        depth + 2,
        format_args!("{coroutine_scope}.launch {{ {page_state}.animateScrollToPage(page) }}"),
    );
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        AccessibilityRole, BottomBarTab, Expr, Module, Node, Screen, ScreenId, SystemIcon,
        ToolbarPlacement, Type,
    };

    use crate::generator::features::Features;

    use super::{imports, render_app_bottom_bar, render_page_pager};

    fn empty_module() -> Module {
        Module {
            app_name: "PagerParity".to_owned(),
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
        }
    }

    #[test]
    fn large_navigation_title_collapses_and_centers_as_content_scrolls() {
        let tab = BottomBarTab {
            index: 0,
            label: "Today".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Today".to_owned()),
            large_title: true,
            search_state: None,
            search_prompt: None,
            children: Vec::new(),
        };
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &empty_module(),
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("NexaNavigationChrome("));
        assert!(output.contains("largeTitle = true,"));
        assert!(output.contains("identity = \"nexa-tab-title-0\""));
        let title_resource = nexa_codegen::names::localization_resource_name("Today");
        assert!(output.contains(&format!(
            "title = stringResource(R.string.{title_resource})"
        )));
        assert!(output.contains("containerColor = MaterialTheme.colorScheme.background"));
        assert!(output.contains("modifier = Modifier.fillMaxSize()"));
    }

    #[test]
    fn inline_navigation_title_uses_the_shared_material_title_system() {
        let tab = BottomBarTab {
            index: 2,
            label: "Inbox".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Inbox".to_owned()),
            large_title: false,
            search_state: None,
            search_prompt: None,
            children: Vec::new(),
        };
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &empty_module(),
            &Features::default(),
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains("NexaNavigationChrome("));
        assert!(generated.contains("largeTitle = false,"));
        assert!(generated.contains("centerTitle = true,"));
        assert!(!generated.contains("title_top_padding"));
    }

    #[test]
    fn trailing_tab_actions_keep_the_native_centered_title_alignment() {
        let tab = BottomBarTab {
            index: 0,
            label: "Inbox".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Inbox".to_owned()),
            large_title: false,
            search_state: None,
            search_prompt: None,
            children: vec![Node::Toolbar {
                placement: ToolbarPlacement::Trailing,
                children: vec![Node::Text {
                    value: Expr::String("Sort".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
            }],
        };
        let mut output = SourceWriter::new();
        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &empty_module(),
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("centerTitle = true,"));
    }

    #[test]
    fn navigation_links_in_large_title_toolbars_use_compact_icon_buttons() {
        let mut module = empty_module();
        module.screens.push(Screen {
            id: ScreenId(0),
            name: "Completed".to_owned(),
            parameters: Vec::new(),
            states: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
        });
        let tab = BottomBarTab {
            index: 0,
            label: "Inbox".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Inbox".to_owned()),
            large_title: true,
            search_state: None,
            search_prompt: None,
            children: vec![Node::Toolbar {
                placement: ToolbarPlacement::Trailing,
                children: vec![Node::NavigationLink {
                    destination: ScreenId(0),
                    arguments: Vec::new(),
                    guard: None,
                    children: Vec::new(),
                }],
            }],
        };
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains(
            "IconButton(onClick = { navController.navigate(\"nexa_screen_0\") }, enabled = true) {"
        ));
        assert!(!output.contains("ChevronRight"));
        assert!(!output.contains("Modifier.fillMaxWidth().clickable { navController.navigate"));
    }

    #[test]
    fn toolbar_picker_wraps_content_instead_of_consuming_the_actions_slot() {
        let tab = BottomBarTab {
            index: 0,
            label: "Today".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Today".to_owned()),
            large_title: true,
            search_state: None,
            search_prompt: None,
            children: vec![Node::Toolbar {
                placement: ToolbarPlacement::Trailing,
                children: vec![Node::Accessibility {
                    label: Expr::String("Sort".to_owned()),
                    hint: None,
                    value: Some(Expr::State("sortSelection".to_owned(), Type::String)),
                    role: AccessibilityRole::None,
                    children: vec![Node::Picker {
                        items: Expr::State(
                            "sortOptions".to_owned(),
                            Type::Array(Box::new(Type::String)),
                        ),
                        state: "sortSelection".to_owned(),
                        icon: Some(SystemIcon::Shared("sort".to_owned())),
                        item_icons: Vec::new(),
                        label: None,
                        tint: None,
                    }],
                }],
            }],
        };
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &empty_module(),
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains(
            "NexaPickerPrimitive(items = nexa_sortOptions, itemIcons = emptyList(), selected = nexa_sortSelection, icon = Icons.AutoMirrored.Filled.Sort, label = null, tint = null, fillMaxWidth = false)"
        ));
    }

    #[test]
    fn form_tab_extends_grouped_surface_behind_navigation_title() {
        let tab = BottomBarTab {
            index: 3,
            label: "Settings".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: Some("Settings".to_owned()),
            large_title: true,
            search_state: None,
            search_prompt: None,
            children: vec![Node::Form {
                show_dividers: true,
                background: None,
                children: Vec::new(),
            }],
        };
        let mut features = Features::default();
        features.uses_form = true;
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &empty_module(),
            &features,
            0,
            &mut output,
        );

        assert!(output.contains("NexaNavigationChrome("));
        assert!(output.contains(
            "containerColor = if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant,"
        ));
        assert!(output.contains(
            "modifier = Modifier.fillMaxSize().background(if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant),"
        ));
        assert!(output.contains(
            "val nexaTabBackdropColor3 = (if (MaterialTheme.colorScheme.background == Color.Black) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.surfaceVariant).toArgb()"
        ));
    }

    #[test]
    fn searchable_tab_uses_the_shared_text_input_renderer() {
        let tab = BottomBarTab {
            index: 4,
            label: "Search".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: Some("search".to_owned()),
            navigation_title: Some("Search".to_owned()),
            large_title: true,
            search_state: Some("query".to_owned()),
            search_prompt: Some("Search tasks...".to_owned()),
            children: Vec::new(),
        };
        let mut module = empty_module();
        module.body = vec![Node::AppBottomBar {
            state: "selectedTab".to_owned(),
            tint: None,
            tabs: vec![tab.clone()],
            custom_bar: None,
        }];
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            None,
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("NexaTextInputPrimitive("));
        assert!(output.contains("searchIcon = Icons.Filled.Search"));
        assert!(output.contains("modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)"));
        assert!(output.contains(
            "modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).imePadding(),"
        ));
        assert!(output.contains("imeAction = ImeAction.Search"));

        let features = Features::analyze(&module);
        let mut generated_imports = crate::generator::engine::imports::ImportSet::default();
        imports(&features, &mut generated_imports);
        let rendered_imports = generated_imports.render();
        assert!(rendered_imports.contains("import androidx.compose.foundation.layout.imePadding"));
        assert!(rendered_imports.contains("import androidx.compose.foundation.background"));
    }

    #[test]
    fn page_pager_gives_each_page_a_full_size_centered_native_canvas() {
        let module = empty_module();
        let features = Features::default();
        let mut output = SourceWriter::new();

        render_page_pager(
            "currentPage",
            &[Vec::new()],
            &module,
            &features,
            0,
            &mut output,
        );

        assert!(output.contains(
            "Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center, propagateMinConstraints = true) {"
        ));
        assert!(output.contains(
            "NexaPageIndicatorRow(pageCount = 1, currentPage = nexa_currentPagePager.settledPage) { page ->"
        ));
        assert!(output.contains("rememberPagerState(initialPage = nexa_currentPage,"));
        assert!(output.contains(
            "LaunchedEffect(nexa_currentPagePager.settledPage) { nexa_currentPage = nexa_currentPagePager.settledPage }"
        ));
        assert!(output.contains(
            "LaunchedEffect(nexa_currentPage) { if (nexa_currentPagePager.settledPage != nexa_currentPage) nexa_currentPagePager.animateScrollToPage(nexa_currentPage) }"
        ));
        assert!(!output.contains("LaunchedEffect(nexa_currentPagePager.currentPage)"));
        assert!(output.contains(
            "pageNestedScrollConnection = object : androidx.compose.ui.input.nestedscroll.NestedScrollConnection {}"
        ));
        assert!(output.contains("overscrollEffect = null"));
        assert!(!output.contains(".toInt()"));
    }

    #[test]
    fn custom_tab_bar_keeps_real_destinations_and_omits_adaptive_chrome() {
        let tab = BottomBarTab {
            index: 0,
            label: "Home".to_owned(),
            comment: None,
            icon: None,
            badge: None,
            role: None,
            navigation_title: None,
            large_title: false,
            search_state: None,
            search_prompt: None,
            children: vec![Node::Text {
                value: nexa_ir::Expr::String("Destination".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        };
        let custom_bar = vec![Node::Text {
            value: nexa_ir::Expr::String("Custom bar".to_owned()),
            style: nexa_ir::TextStyle::default(),
        }];
        let module = empty_module();
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            Some(&custom_bar),
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("showNavigationSuite = false"));
        assert!(output.contains("Box(Modifier.weight(1f).fillMaxWidth())"));
        assert!(output.contains("NexaTextPrimitive(text = \"Destination\")"));
        assert!(output.contains("NexaTextPrimitive(text = \"Custom bar\")"));
    }
}
