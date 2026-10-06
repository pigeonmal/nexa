use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module};

use crate::generator::{
    components::render_children,
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
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
        "androidx.compose.foundation.layout.fillMaxSize",
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
    out.line_at(depth, format_args!("NavigationSuiteScaffold("));
    out.line_at(depth + 1, format_args!("navigationSuiteItems = {{"));
    for tab in tabs {
        out.line_at(
            depth + 2,
            format_args!(
                "item(selected = {selected} == {}, onClick = {{ {selected} = {} }}, icon = {{",
                tab.index, tab.index
            ),
        );
        if let Some(icon) = &tab.icon {
            out.line_at(
                depth + 3,
                format_args!(
                    "Icon(imageVector = {}, contentDescription = null)",
                    icon.material_reference()
                ),
            );
        } else {
            out.line_at(depth + 3, format_args!("Spacer(Modifier.size(24.dp))"));
        }
        out.line_at(
            depth + 2,
            format_args!(
                "}}, label = {{ Text(stringResource(R.string.{})) }},",
                nexa_codegen::names::localization_resource_name(&tab.label)
            ),
        );
        if let Some(badge) = &tab.badge {
            out.line_at(
                depth + 2,
                format_args!("badge = {{ Badge {{ Text({}) }} }},", kotlin_string(badge)),
            );
        }
        out.line_at(depth + 2, format_args!("alwaysShowLabel = true,"));
        if tint.is_some() {
            out.line_at(depth + 2, format_args!("colors = {item_colors},"));
        }
        out.line_at(depth + 2, format_args!(")"));
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!(") {{"));
    for tab in tabs {
        out.line_at(
            depth + 1,
            format_args!("if ({selected} == {}) {{", tab.index),
        );
        out.line_at(
            depth + 2,
            format_args!(
                "{state_holder}.SaveableStateProvider(key = {}) {{",
                kotlin_string(&format!("nexa-tab-{}", tab.index))
            ),
        );
        if tab.navigation_title.is_some() || tab.search_state.is_some() {
            out.line_at(
                depth + 3,
                format_args!("Column(modifier = Modifier.fillMaxSize()) {{"),
            );
            if let Some(title) = &tab.navigation_title {
                let resource = nexa_codegen::names::localization_resource_name(title);
                out.line_at(
                    depth + 4,
                    format_args!(
                        "Text(stringResource(R.string.{resource}), style = MaterialTheme.typography.headlineLarge, modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp))"
                    ),
                );
            }
            if let Some(search_state) = &tab.search_state {
                let prompt = nexa_codegen::names::localization_resource_name(
                    tab.search_prompt.as_deref().unwrap_or("Search"),
                );
                out.line_at(
                    depth + 4,
                    format_args!(
                        "TextField(value = {}, onValueChange = {{ {} = it }}, modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp), placeholder = {{ Text(stringResource(R.string.{prompt})) }}, singleLine = true)",
                        state_name(search_state),
                        state_name(search_state)
                    ),
                );
            }
            render_children(&tab.children, module, features, depth + 4, out);
            out.push('\n');
            indent(out, depth + 3);
            out.push_str("}\n");
        } else {
            render_children(&tab.children, module, features, depth + 3, out);
            out.push('\n');
        }
        indent(out, depth + 2);
        out.push_str("}\n");
        out.line_at(depth + 1, format_args!("}}"));
    }
    out.line_at(depth, format_args!("}}"));
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
    out.line_at(depth + 2, format_args!("when (page) {{"));
    for (index, page) in pages.iter().enumerate() {
        out.line_at(depth + 3, format_args!("{index} -> {{"));
        render_children(page, module, features, depth + 4, out);
        out.push('\n');
        indent(out, depth + 3);
        out.push_str("}\n");
    }
    out.line_at(depth + 3, format_args!("else -> {{}}"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(
            "Row(modifier = Modifier.padding(bottom = 24.dp).align(Alignment.CenterHorizontally), horizontalArrangement = Arrangement.spacedBy(8.dp)) {{"
        ),
    );
    for (index, _) in pages.iter().enumerate() {
        out.line_at(
            depth + 2,
            format_args!(
                "Box(modifier = Modifier.size(if ({page_state}.currentPage == {index}) 8.dp else 6.dp).clip(CircleShape).background(if ({page_state}.currentPage == {index}) MaterialTheme.colorScheme.primary else Color.Gray.copy(alpha = 0.45f)))"
            ),
        );
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
}
