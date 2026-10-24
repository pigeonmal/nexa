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
                    out.push_str(
                        "\n        }.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)",
                    );
                    if tab_contains_grouped_surface(&tab.children) {
                        out.push_str(
                            "\n            .background(Color(uiColor: .systemGroupedBackground).ignoresSafeArea())",
                        );
                    }
                    out.push_str("\n    }\n");
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

fn tab_contains_grouped_surface(children: &[Node]) -> bool {
    let mut found = false;
    walk_ir(
        children,
        &mut |node| {
            found |= matches!(node, Node::Form { .. })
                || matches!(node, Node::FastList { plan } if plan.uses_platform_list());
        },
        &mut |_| {},
    );
    found
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
    out.line_at(depth, format_args!("VStack(spacing: 0) {{"));
    out.line_at(
        depth + 1,
        format_args!("TabView(selection: ${selected}) {{"),
    );
    for index in 0..pages.len() {
        out.line_at(
            depth + 2,
            format_args!(
                "{prefix}_{index}().frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top).tag(Int32({index}))"
            ),
        );
    }
    indent(out, depth + 1);
    out.push_str(&format!(
        "}}.tabViewStyle(.page(indexDisplayMode: .never)).animation(.default, value: {selected}).frame(maxWidth: .infinity, maxHeight: .infinity)\n"
    ));
    out.line_at(
        depth + 1,
        format_args!(
            "NexaPageIndicatorRow(pageCount: {}, selectedIndex: Int({selected})) {{ index in {selected} = Int32(index) }}",
            pages.len()
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
    custom_bar: Option<&[Node]>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let prefix = format!("nexa_{}_tab", state_name(state));
    let tint = tint
        .map(crate::generator::colors::expression_for_color)
        .unwrap_or_else(|| "nil".to_owned());
    let activates_search = tabs.iter().any(|tab| tab.role.as_deref() == Some("search"));

    out.line_at(depth, format_args!("if #available(iOS 18.0, *) {{"));
    out.line_at(
        depth + 1,
        format_args!("TabView(selection: ${}) {{", state_name(state)),
    );
    for (i, tab) in tabs.iter().enumerate() {
        render_modern_tab_ref(tab, &prefix, i, depth + 2, out);
    }
    indent(out, depth + 1);
    out.push_str(&format!(
        "}}.modifier(NexaAppBottomBarPrimitive(tint: {tint}, adaptiveStyle: {}, activatesSearchTab: {activates_search}))",
        custom_bar.is_none()
    ));
    if let Some(custom_bar) = custom_bar {
        out.push_str(".toolbar(.hidden, for: .tabBar).safeAreaInset(edge: .bottom, spacing: 0) {");
        out.push('\n');
        for child in custom_bar {
            crate::generator::components::render_node(child, module, features, depth + 2, out);
            out.push('\n');
        }
        out.line_at(depth + 1, format_args!("}}"));
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
    out.push_str(&format!(
        ".modifier(NexaAppBottomBarPrimitive(tint: {tint}, adaptiveStyle: false, activatesSearchTab: false))"
    ));
    if let Some(custom_bar) = custom_bar {
        out.push_str(".toolbar(.hidden, for: .tabBar).safeAreaInset(edge: .bottom, spacing: 0) {");
        out.push('\n');
        for child in custom_bar {
            crate::generator::components::render_node(child, module, features, depth + 2, out);
            out.push('\n');
        }
        out.line_at(depth + 1, format_args!("}}"));
    }
    render_app_tab_navigation_modifiers(state, tabs, module, features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
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
    let has_tab_toolbar = tabs
        .iter()
        .any(|tab| !collect_navigation_toolbars(&tab.children).is_empty());
    if !titled_tabs.is_empty() || has_tab_toolbar {
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
        let large = titled_tabs
            .iter()
            .rev()
            .fold("false".to_owned(), |fallback, (tab, _)| {
                format!(
                    "({selected} == Int32({} ) ? {} : {fallback})",
                    tab.index, tab.large_title
                )
            });
        out.push_str(&format!(
            "\n.navigationTitle({title})\n.modifier(NexaNavigationTitleDisplayModeModifier(inline: !{large}))\n.toolbar {{\n"
        ));
        for tab in tabs {
            let toolbars = collect_navigation_toolbars(&tab.children);
            if toolbars.is_empty() {
                continue;
            }
            out.line_at(
                depth + 1,
                format_args!("if {selected} == Int32({}) {{", tab.index),
            );
            for (placement, swift_placement) in [
                (ToolbarPlacement::Leading, ".navigationBarLeading"),
                (ToolbarPlacement::Trailing, ".navigationBarTrailing"),
            ] {
                let children = toolbars
                    .iter()
                    .filter_map(|toolbar| match toolbar {
                        Node::Toolbar {
                            placement: actual,
                            children,
                        } if *actual == placement => Some(children),
                        _ => None,
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                if children.is_empty() {
                    continue;
                }
                out.line_at(
                    depth + 2,
                    format_args!("ToolbarItemGroup(placement: {swift_placement}) {{"),
                );
                for child in children {
                    crate::generator::components::render_node(
                        child,
                        module,
                        features,
                        depth + 3,
                        out,
                    );
                    out.push('\n');
                }
                out.line_at(depth + 2, format_args!("}}"));
            }
            out.line_at(depth + 1, format_args!("}}"));
        }
        out.line_at(depth, format_args!("}}"));
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
        ToolbarPlacement::Leading => ".topBarLeading",
        ToolbarPlacement::Trailing => ".topBarTrailing",
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
            "NexaTabLabelPrimitive(title: {text}, systemImage: {})",
            swift_string(&icon.sf_symbol_name())
        )
    })
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{BottomBarTab, Module, Node, ToolbarPlacement};

    use crate::generator::engine::features::Features;

    use super::{render_app_bottom_bar, render_bottom_bar_helpers, render_page_pager};

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
                custom_bar: None,
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
    fn page_indicator_calls_the_shared_native_row_primitive() {
        let mut output = SourceWriter::new();
        render_page_pager("currentPage", &[Vec::new(), Vec::new()], 0, &mut output);

        assert!(
            output.contains(
                "NexaPageIndicatorRow(pageCount: 2, selectedIndex: Int(nexa_currentPage))"
            )
        );
    }

    #[test]
    fn page_pager_pages_fill_the_viewport_and_align_to_the_top() {
        let mut output = SourceWriter::new();
        render_page_pager("currentPage", &[Vec::new(), Vec::new()], 0, &mut output);

        assert!(output.contains(
            "nexa_currentPage_page_0().frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top).tag(Int32(0))"
        ));
        assert!(output.contains(
            "nexa_currentPage_page_1().frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top).tag(Int32(1))"
        ));
    }

    #[test]
    fn searchable_tab_renders_a_visible_native_text_input() {
        let module = empty_module_with_search_tab();
        let mut output = SourceWriter::new();
        render_bottom_bar_helpers(&module.body, &module, &Features::default(), &mut output);

        assert!(output.contains("VStack(spacing: 0) {"));
        assert!(output.contains("NexaTextInputPrimitive(text: $nexa_query"));
        assert!(output.contains("placeholder: Text(\"Search tasks...\")"));
        assert!(output.contains("searchable: true"));
        assert!(output.contains(".padding(.horizontal, 16)"));
        assert!(
            output.contains(
                ".frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)"
            )
        );
        assert!(!output.contains(".searchable(text:"));
    }

    #[test]
    fn app_tab_content_aligns_to_top_leading_like_android() {
        let mut module = empty_module_with_search_tab();
        let Node::AppBottomBar { tabs, .. } = &mut module.body[0] else {
            unreachable!("test module contains an app bottom bar")
        };
        tabs[0].children.push(Node::Text {
            value: nexa_ir::Expr::String("Inbox tab content".to_owned()),
            style: nexa_ir::TextStyle::default(),
        });

        let mut output = SourceWriter::new();
        render_bottom_bar_helpers(&module.body, &module, &Features::default(), &mut output);

        assert!(output.contains(
            "}.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)"
        ));
    }

    #[test]
    fn app_bottom_bar_calls_the_shared_native_tab_shell() {
        let module = empty_module_with_search_tab();
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            match &module.body[0] {
                Node::AppBottomBar { tabs, .. } => tabs,
                _ => unreachable!(),
            },
            None,
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("NexaAppBottomBarPrimitive("));
        assert!(output.contains(
            ".modifier(NexaAppBottomBarPrimitive(tint: nil, adaptiveStyle: true, activatesSearchTab: true))"
        ));
        assert!(output.contains(
            ".modifier(NexaAppBottomBarPrimitive(tint: nil, adaptiveStyle: false, activatesSearchTab: false))"
        ));
        assert!(output.contains("if #available(iOS 18.0"));
    }

    #[test]
    fn titled_tabs_use_the_shared_navigation_title_primitive() {
        let mut module = empty_module_with_search_tab();
        let Node::AppBottomBar { tabs, .. } = &mut module.body[0] else {
            unreachable!("test module contains an app bottom bar")
        };
        tabs[0].children.push(Node::Toolbar {
            placement: ToolbarPlacement::Trailing,
            children: vec![Node::Text {
                value: nexa_ir::Expr::String("Done".to_owned()),
                style: nexa_ir::TextStyle::default(),
            }],
        });
        let Node::AppBottomBar {
            state,
            tint,
            tabs,
            custom_bar,
        } = &module.body[0]
        else {
            unreachable!("test module contains an app bottom bar")
        };
        let mut output = SourceWriter::new();
        render_app_bottom_bar(
            state,
            tint.as_ref(),
            tabs,
            custom_bar.as_deref(),
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains(".navigationTitle("));
        assert!(output.contains("NexaNavigationTitleDisplayModeModifier(inline: !"));
        assert!(output.contains(".toolbar {"));
        assert!(output.contains("ToolbarItemGroup(placement: .navigationBarTrailing)"));
        assert!(!output.contains("NexaNavigationChromeModifier"));
    }

    #[test]
    fn custom_tab_bar_hides_native_chrome_but_keeps_the_tab_view() {
        let module = empty_module_with_search_tab();
        let tab = match &module.body[0] {
            Node::AppBottomBar { tabs, .. } => tabs,
            _ => unreachable!(),
        };
        let custom_bar = vec![Node::Text {
            value: nexa_ir::Expr::String("Custom bar".to_owned()),
            style: nexa_ir::TextStyle::default(),
        }];
        let mut output = SourceWriter::new();

        render_app_bottom_bar(
            "selectedTab",
            None,
            tab,
            Some(&custom_bar),
            &module,
            &Features::default(),
            0,
            &mut output,
        );

        assert!(output.contains("TabView(selection: $nexa_selectedTab)"));
        assert!(output.contains(".toolbar(.hidden, for: .tabBar).safeAreaInset(edge: .bottom"));
        assert!(output.contains("Text(\"Custom bar\")"));
        assert!(output.contains("adaptiveStyle: false"));
    }
}
