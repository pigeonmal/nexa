use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    BottomBarTab, Capitalization, KeyboardType, Module, Node, ReturnKeyType, ToolbarPlacement,
};

use crate::generator::{
    components::{
        input::{self, TextInputProps},
        render_children, render_node,
    },
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
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
        "androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold",
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
        features.uses_adaptive_tabs && features.uses_form,
        "androidx.compose.foundation.background",
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
    imports.add(features.uses_tab_badge, "androidx.compose.material3.Badge");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_app_bottom_bar(
    state: &str,
    tint: Option<&nexa_ir::ColorExpression>,
    tabs: &[BottomBarTab],
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
    if let Some(tint) = tint {
        let tint = crate::generator::colors::expression_for_color(tint);
        out.line_at(
            depth,
            format_args!(
                "val {item_colors} = androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteDefaults.itemColors("
            ),
        );
        out.line_at(
            depth + 1,
            format_args!(
                "navigationBarItemColors = androidx.compose.material3.NavigationBarItemDefaults.colors(selectedIconColor = {tint}, selectedTextColor = {tint}, indicatorColor = {tint}.copy(alpha = 0.12f)),"
            ),
        );
        out.line_at(
            depth + 1,
            format_args!(
                "navigationRailItemColors = androidx.compose.material3.NavigationRailItemDefaults.colors(selectedIconColor = {tint}, selectedTextColor = {tint}, indicatorColor = {tint}.copy(alpha = 0.12f)),"
            ),
        );
        out.line_at(
            depth + 1,
            format_args!(
                "navigationDrawerItemColors = androidx.compose.material3.NavigationDrawerItemDefaults.colors(selectedIconColor = {tint}, selectedTextColor = {tint}, selectedContainerColor = {tint}.copy(alpha = 0.12f))"
            ),
        );
        out.line_at(depth, format_args!(")"));
    }
    let content_depth = depth + usize::from(tint.is_some());
    if let Some(tint) = tint {
        let tint = crate::generator::colors::expression_for_color(tint);
        out.line_at(
            depth,
            format_args!(
                "MaterialTheme(colorScheme = MaterialTheme.colorScheme.copy(primary = {tint}), typography = MaterialTheme.typography, shapes = MaterialTheme.shapes) {{"
            ),
        );
    }
    out.line_at(content_depth, format_args!("NavigationSuiteScaffold("));
    out.line_at(content_depth + 1, format_args!("navigationSuiteItems = {{"));
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
        if tint.is_some() {
            out.line_at(content_depth + 2, format_args!("colors = {item_colors},"));
        }
        out.line_at(content_depth + 2, format_args!(")"));
    }
    out.line_at(content_depth + 1, format_args!("}}"));
    out.line_at(content_depth, format_args!(") {{"));
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
        let toolbars = tab_toolbars(&tab.children);
        if tab.navigation_title.is_some() || tab.search_state.is_some() || !toolbars.is_empty() {
            let content_modifier = if tab_contains_form(&tab.children) {
                "Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant)"
            } else {
                "Modifier.fillMaxSize()"
            };
            out.line_at(
                content_depth + 3,
                format_args!("Column(modifier = {content_modifier}) {{"),
            );
            if tab.navigation_title.is_some() || !toolbars.is_empty() {
                out.line_at(
                    content_depth + 4,
                    format_args!("Row(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {{"),
                );
                if toolbars
                    .iter()
                    .any(|(placement, _)| *placement == ToolbarPlacement::Leading)
                {
                    render_tab_toolbar_items(
                        &toolbars,
                        ToolbarPlacement::Leading,
                        module,
                        features,
                        content_depth + 5,
                        out,
                    );
                    out.line_at(
                        content_depth + 5,
                        format_args!("Spacer(modifier = Modifier.size(8.dp))"),
                    );
                }
                if let Some(title) = &tab.navigation_title {
                    let resource = nexa_codegen::names::localization_resource_name(title);
                    let style = if tab.large_title {
                        "MaterialTheme.typography.headlineLarge.copy(fontWeight = FontWeight.Bold)"
                    } else {
                        "MaterialTheme.typography.titleLarge"
                    };
                    out.line_at(
                        content_depth + 5,
                        format_args!("Text(stringResource(R.string.{resource}), style = {style})"),
                    );
                }
                if toolbars
                    .iter()
                    .any(|(placement, _)| *placement == ToolbarPlacement::Trailing)
                {
                    if tab.navigation_title.is_some()
                        || toolbars
                            .iter()
                            .any(|(placement, _)| *placement == ToolbarPlacement::Leading)
                    {
                        out.line_at(
                            content_depth + 5,
                            format_args!("Spacer(modifier = Modifier.weight(1f))"),
                        );
                    }
                    render_tab_toolbar_items(
                        &toolbars,
                        ToolbarPlacement::Trailing,
                        module,
                        features,
                        content_depth + 5,
                        out,
                    );
                }
                out.line_at(content_depth + 4, format_args!("}}"));
            }
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
        } else {
            render_children(&tab.children, module, features, content_depth + 3, out);
            out.push('\n');
        }
        indent(out, content_depth + 2);
        out.push_str("}\n");
        out.line_at(content_depth + 1, format_args!("}}"));
    }
    out.line_at(content_depth, format_args!("}}"));
    if tint.is_some() {
        out.line_at(depth, format_args!("}}"));
    }
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

fn tab_toolbars(children: &[Node]) -> Vec<(ToolbarPlacement, &[Node])> {
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

fn render_tab_toolbar_items(
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
            render_node(child, module, features, depth, out);
            out.push('\n');
        }
    }
}

fn without_tab_toolbars(children: &[Node]) -> Vec<Node> {
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
    out.line_at(
        depth,
        format_args!("val {coroutine_scope} = rememberCoroutineScope()"),
    );
    out.line_at(
        depth,
        format_args!(
            "val {page_state} = rememberPagerState(initialPage = {selected}.toInt(), pageCount = {{ {} }})",
            pages.len()
        ),
    );
    out.line_at(
        depth,
        format_args!(
            "LaunchedEffect({selected}) {{ if ({page_state}.currentPage != {selected}.toInt()) {page_state}.animateScrollToPage({selected}.toInt()) }}"
        ),
    );
    out.line_at(
        depth,
        format_args!(
            "LaunchedEffect({page_state}.currentPage) {{ {selected} = {page_state}.currentPage.toInt() }}"
        ),
    );
    out.line_at(
        depth,
        format_args!("Column(modifier = Modifier.fillMaxSize()) {{"),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "HorizontalPager(state = {page_state}, modifier = Modifier.weight(1f)) {{ page ->"
        ),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center, propagateMinConstraints = true) {{"
        ),
    );
    out.line_at(depth + 3, format_args!("when (page) {{"));
    for (index, page) in pages.iter().enumerate() {
        out.line_at(depth + 4, format_args!("{index} -> {{"));
        render_children(page, module, features, depth + 5, out);
        out.push('\n');
        indent(out, depth + 4);
        out.push_str("}\n");
    }
    out.line_at(depth + 4, format_args!("else -> {{}}"));
    out.line_at(depth + 3, format_args!("}}"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(
            "Row(modifier = Modifier.fillMaxWidth().padding(bottom = {}.dp), horizontalArrangement = Arrangement.spacedBy({}.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {{",
            nexa_codegen::design_system::PAGE_INDICATOR_BOTTOM_INSET,
            nexa_codegen::design_system::PAGE_INDICATOR_SPACING,
        ),
    );
    for (index, _) in pages.iter().enumerate() {
        out.line_at(
            depth + 2,
            format_args!(
                "Box(modifier = Modifier.size(if ({page_state}.currentPage == {index}) {active_dot}.dp else {inactive_dot}.dp).clip(CircleShape).background(if ({page_state}.currentPage == {index}) MaterialTheme.colorScheme.primary else Color(0x{muted_dot:08X}).copy(alpha = {inactive_opacity}f)).clickable(role = androidx.compose.ui.semantics.Role.Button, onClickLabel = {page_label}) {{ {coroutine_scope}.launch {{ {page_state}.animateScrollToPage({index}) }} }})",
                active_dot = nexa_codegen::design_system::PAGE_INDICATOR_SELECTED_SIZE,
                inactive_dot = nexa_codegen::design_system::PAGE_INDICATOR_UNSELECTED_SIZE,
                inactive_opacity = nexa_codegen::design_system::PAGE_INDICATOR_INACTIVE_OPACITY,
                muted_dot = nexa_codegen::design_system::MUTED_TEXT_ARGB,
                page_label = kotlin_string(&format!("Page {}", index + 1)),
            ),
        );
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{BottomBarTab, Module, Node};

    use crate::generator::features::Features;

    use super::{render_app_bottom_bar, render_page_pager};

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
    fn large_navigation_title_uses_bold_headline_typography() {
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
            &empty_module(),
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains(
            "style = MaterialTheme.typography.headlineLarge.copy(fontWeight = FontWeight.Bold)"
        ));
        assert!(output.contains("Column(modifier = Modifier.fillMaxSize()) {"));
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
            &empty_module(),
            &features,
            0,
            &mut output,
        );

        assert!(output.contains(
            "Column(modifier = Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surfaceVariant)) {"
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
        }];
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            &[tab],
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("leadingIcon = { Icon(imageVector = Icons.Filled.Search"));
        assert!(output.contains("modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)"));
        assert!(output.contains("imeAction = ImeAction.Search"));
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
            "Row(modifier = Modifier.fillMaxWidth().padding(bottom = 24.dp), horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {"
        ));
    }
}
