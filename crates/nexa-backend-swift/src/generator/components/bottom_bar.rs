use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module, Node, ToolbarPlacement, walk::walk_ir};

use crate::generator::{
    components::{
        input::{self, TextInputProps},
        render_children,
    },
    features::Features,
    utils::{indent, swift_string},
};

pub(crate) fn render_bottom_bar_helpers(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    out: &mut SourceWriter,
) {
    walk_ir(
        nodes,
        &mut |node| {
            if let Node::AppBottomBar { state, tabs, .. } = node {
                let prefix = format!("nexa_{}_tab", state_name(state));
                for (i, tab) in tabs.iter().enumerate() {
                    out.push_str(&format!(
                        "\n    @ViewBuilder\n    private func {prefix}_{i}() -> some View {{\n"
                    ));
                    if let Some(search_state) = &tab.search_state {
                        out.push_str("        VStack(spacing: 0) {\n");
                        input::render_text_input(
                            TextInputProps {
                                state: search_state,
                                placeholder: tab.search_prompt.as_deref().unwrap_or("Search"),
                                comment: None,
                                keyboard: nexa_ir::KeyboardType::Text,
                                secure: false,
                                multiline: false,
                                autofill: None,
                                return_key: Some(nexa_ir::ReturnKeyType::Search),
                                autocorrect: Some(false),
                                capitalization: Some(nexa_ir::Capitalization::None),
                                focused: None,
                                max_length: None,
                                font: None,
                                min_lines: None,
                                max_lines: Some(1),
                                searchable: true,
                                horizontal_padding: Some(16),
                                actions: &[],
                                on_change: None,
                            },
                            3,
                            out,
                        );
                        out.push('\n');
                    } else {
                        out.push_str("        Group {\n");
                    }
                    let content: Vec<_> = tab
                        .children
                        .iter()
                        .filter(|child| !matches!(child, Node::Toolbar { .. }))
                        .cloned()
                        .collect();
                    render_children(&content, module, features, 3, out);
                    out.push_str("\n        }\n    }\n");
                }
            } else if let Node::PagePager { state, pages } = node {
                let prefix = format!("nexa_{}_page", state_name(state));
                for (index, page) in pages.iter().enumerate() {
                    out.push_str(&format!(
                        "\n    @ViewBuilder\n    private func {prefix}_{index}() -> some View {{\n"
                    ));
                    render_children(page, module, features, 2, out);
                    out.push('\n');
                    out.push_str("    }\n");
                }
            }
        },
        &mut |_| {},
    );
}

pub(crate) fn render_navigation_toolbar(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Node::Toolbar {
        placement,
        children,
    } = node
    else {
        return;
    };
    out.line_at(depth, format_args!(".toolbar {{"));
    render_navigation_toolbar_items(
        &Node::Toolbar {
            placement: *placement,
            children: children.clone(),
        },
        module,
        features,
        depth + 1,
        out,
    );
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_page_pager(
    state: &str,
    pages: &[Vec<Node>],
    depth: usize,
    out: &mut SourceWriter,
) {
    let prefix = format!("nexa_{}_page", state_name(state));
    let selected = state_name(state);
    let active_dot = nexa_codegen::design_system::PAGE_INDICATOR_SELECTED_SIZE;
    let inactive_dot = nexa_codegen::design_system::PAGE_INDICATOR_UNSELECTED_SIZE;
    let muted_dot = crate::generator::engine::colors::expression_from_argb(
        nexa_codegen::design_system::MUTED_TEXT_ARGB,
    );
    out.line_at(depth, format_args!("VStack(spacing: 0) {{"));
    out.line_at(
        depth + 1,
        format_args!("TabView(selection: ${selected}) {{"),
    );
    for index in 0..pages.len() {
        out.line_at(
            depth + 2,
            format_args!("{prefix}_{index}().tag(Int32({index}))"),
        );
    }
    indent(out, depth + 1);
    out.push_str(&format!(
        "}}.tabViewStyle(.page(indexDisplayMode: .never)).animation(.default, value: {selected}).frame(maxWidth: .infinity, maxHeight: .infinity)\n"
    ));
    out.line_at(
        depth + 1,
        format_args!(
            "HStack(spacing: {}) {{",
            nexa_codegen::design_system::PAGE_INDICATOR_SPACING,
        ),
    );
    out.line_at(
        depth + 2,
        format_args!(
            "ForEach(0..<{pages_len}, id: \\.self) {{ index in",
            pages_len = pages.len()
        ),
    );
    out.line_at(
        depth + 3,
        format_args!("Button {{ {selected} = Int32(index) }} label: {{"),
    );
    out.line_at(depth + 4, format_args!("Circle()"));
    out.line_at(
        depth + 5,
        format_args!(
            ".fill({selected} == Int32(index) ? {} : {muted_dot}.opacity({}))",
            nexa_codegen::design_system::SWIFT_DEFAULT_ACCENT_COLOR,
            nexa_codegen::design_system::PAGE_INDICATOR_INACTIVE_OPACITY,
        ),
    );
    out.line_at(
        depth + 5,
        format_args!(
            ".frame(width: {selected} == Int32(index) ? {active_dot} : {inactive_dot}, height: {selected} == Int32(index) ? {active_dot} : {inactive_dot})"
        ),
    );
    out.line_at(depth + 3, format_args!("}}.buttonStyle(.plain)"));
    out.line_at(
        depth + 4,
        format_args!(".animation(.default, value: {selected})"),
    );
    out.line_at(
        depth + 4,
        format_args!(".accessibilityLabel(\"Page \\(index + 1)\")"),
    );
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(
        depth + 1,
        format_args!(
            "}}.padding(.bottom, {})",
            nexa_codegen::design_system::PAGE_INDICATOR_BOTTOM_INSET
        ),
    );
    out.line_at(
        depth,
        format_args!("}}.frame(maxWidth: .infinity, maxHeight: .infinity)"),
    );
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
    let prefix = format!("nexa_{}_tab", state_name(state));

    out.line_at(depth, format_args!("if #available(iOS 18.0, *) {{"));
    out.line_at(
        depth + 1,
        format_args!("TabView(selection: ${}) {{", state_name(state)),
    );
    for (i, tab) in tabs.iter().enumerate() {
        render_modern_tab_ref(tab, &prefix, i, depth + 2, out);
    }
    indent(out, depth + 1);
    out.push_str("}.tabViewStyle(.sidebarAdaptable)");
    if let Some(tint) = tint {
        out.push_str(&format!(
            ".tint({})",
            crate::generator::colors::expression_for_color(tint)
        ));
    }
    render_app_tab_navigation_modifiers(state, tabs, module, features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}} else {{"));
    out.line_at(
        depth + 1,
        format_args!("TabView(selection: ${}) {{", state_name(state)),
    );
    for (i, tab) in tabs.iter().enumerate() {
        render_legacy_tab_ref(tab, &prefix, i, depth + 2, out);
    }
    indent(out, depth + 1);
    out.push('}');
    if let Some(tint) = tint {
        out.push_str(&format!(
            ".tint({})",
            crate::generator::colors::expression_for_color(tint)
        ));
    }
    render_app_tab_navigation_modifiers(state, tabs, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_app_tab_navigation_modifiers(
    state: &str,
    tabs: &[BottomBarTab],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let selected = state_name(state);
    let titled_tabs = tabs
        .iter()
        .filter_map(|tab| tab.navigation_title.as_ref().map(|title| (tab, title)))
        .collect::<Vec<_>>();
    if !titled_tabs.is_empty() {
        let title = titled_tabs
            .iter()
            .rev()
            .fold("\"\"".to_owned(), |fallback, (tab, title)| {
                format!(
                    "({selected} == Int32({} ) ? String(localized: {}) : {fallback})",
                    tab.index,
                    swift_string(title)
                )
            });
        out.push_str(&format!("\n.navigationTitle({title})"));
        if titled_tabs.iter().all(|(tab, _)| tab.large_title) {
            out.push_str("\n.nexaLargeTitleDisplayMode()");
        }
    }

    if tabs
        .iter()
        .any(|tab| !collect_navigation_toolbars(&tab.children).is_empty())
    {
        out.push_str("\n.toolbar {");
        for tab in tabs {
            let toolbars = collect_navigation_toolbars(&tab.children);
            if toolbars.is_empty() {
                continue;
            }
            out.push('\n');
            indent(out, depth + 1);
            out.push_str(&format!("if {selected} == Int32({}) {{", tab.index));
            for toolbar in &toolbars {
                out.push('\n');
                render_navigation_toolbar_items(toolbar, module, features, depth + 2, out);
            }
            out.push('\n');
            indent(out, depth + 1);
            out.push('}');
        }
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
}

fn collect_navigation_toolbars(nodes: &[Node]) -> Vec<Node> {
    let mut toolbars = Vec::new();
    walk_ir(
        nodes,
        &mut |node| {
            if matches!(node, Node::Toolbar { .. }) {
                toolbars.push(node.clone());
            }
        },
        &mut |_| {},
    );
    toolbars
}

pub(crate) fn render_navigation_toolbar_items(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Node::Toolbar {
        placement,
        children,
    } = node
    else {
        return;
    };
    let placement = match placement {
        ToolbarPlacement::Leading => ".navigationBarLeading",
        ToolbarPlacement::Trailing => ".navigationBarTrailing",
    };
    for child in children {
        out.line_at(
            depth,
            format_args!("ToolbarItem(placement: {placement}) {{"),
        );
        crate::generator::components::render_node(child, module, features, depth + 1, out);
        out.line_at(depth, format_args!("}}"));
    }
}

fn render_modern_tab_ref(
    tab: &BottomBarTab,
    prefix: &str,
    index: usize,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    let role_part = if tab.role.as_deref() == Some("search") {
        ", role: .search"
    } else {
        ""
    };
    out.push_str(&format!(
        "Tab(value: Int32({}){}) {{\n{}{prefix}_{index}()\n{}}} label: {{\n{}{}\n{}}}",
        tab.index,
        role_part,
        "    ".repeat(depth + 1),
        "    ".repeat(depth),
        "    ".repeat(depth + 1),
        swift_tab_label(tab),
        "    ".repeat(depth)
    ));
    if let Some(badge) = &tab.badge {
        out.push_str(&format!(".badge({})", swift_string(badge)));
    }
    out.push('\n');
}

fn render_legacy_tab_ref(
    tab: &BottomBarTab,
    prefix: &str,
    index: usize,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str(&format!("{prefix}_{index}()\n"));
    indent(out, depth + 1);
    out.push_str(".tabItem {\n");
    indent(out, depth + 2);
    out.push_str(&format!("{}\n", swift_tab_label(tab)));
    indent(out, depth + 1);
    out.push('}');
    out.push_str(&format!(
        "\n{}.tag(Int32({}))",
        "    ".repeat(depth),
        tab.index
    ));
    if let Some(badge) = &tab.badge {
        out.push_str(&format!(
            "\n{}.badge({})",
            "    ".repeat(depth),
            swift_string(badge)
        ));
    }
    out.push('\n');
}

fn swift_tab_label(tab: &BottomBarTab) -> String {
    let source = nexa_ir::Expr::String(tab.label.clone());
    let text = match tab.comment.as_deref() {
        Some(comment) => crate::generator::expressions::localized_text_view(&source, Some(comment)),
        None => crate::generator::expressions::localized_text_view(&source, None),
    };
    tab.icon.as_ref().map_or(text.clone(), |icon| {
        format!(
            "Label {{ {text} }} icon: {{ Image(systemName: {}) }}",
            swift_string(&icon.sf_symbol_name())
        )
    })
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{BottomBarTab, Module, Node};

    use crate::generator::engine::features::Features;

    use super::{render_bottom_bar_helpers, render_page_pager};

    fn empty_module_with_search_tab() -> Module {
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
        Module {
            app_name: "SearchParity".to_owned(),
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
            body: vec![Node::AppBottomBar {
                state: "selectedTab".to_owned(),
                tint: None,
                tabs: vec![tab],
            }],
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
    fn page_indicator_uses_the_shared_dynamic_default_accent() {
        let mut output = SourceWriter::new();
        render_page_pager("currentPage", &[Vec::new(), Vec::new()], 0, &mut output);
        let selected = nexa_codegen::names::state_name("currentPage");

        assert!(output.contains(&format!(
            ".fill({selected} == Int32(index) ? Color(uiColor: .systemBlue) :"
        )));
        assert!(output.contains("HStack(spacing: 8)"));
    }

    #[test]
    fn searchable_tab_renders_a_visible_native_text_input() {
        let module = empty_module_with_search_tab();
        let mut output = SourceWriter::new();
        render_bottom_bar_helpers(&module.body, &module, &Features::default(), &mut output);

        assert!(output.contains("VStack(spacing: 0) {"));
        assert!(output.contains("Image(systemName: \"magnifyingglass\")"));
        assert!(output.contains("TextField(\"Search tasks...\", text: $nexa_query)"));
        assert!(output.contains(".padding(.horizontal, 16)"));
        assert!(!output.contains(".searchable(text:"));
    }
}
