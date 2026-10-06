use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{BottomBarTab, Module, Node, ToolbarPlacement, walk::walk_ir};

use crate::generator::{
    components::render_children,
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
                    out.push_str("        NavigationStack {\n            Group {\n");
                    let content: Vec<_> = tab
                        .children
                        .iter()
                        .filter(|child| !matches!(child, Node::Toolbar { .. }))
                        .cloned()
                        .collect();
                    render_children(&content, module, features, 4, out);
                    out.push_str("\n            }");
                    if let Some(title) = &tab.navigation_title {
                        out.push_str(&format!(
                            "\n                .navigationTitle(String(localized: {}))",
                            swift_string(title)
                        ));
                        if tab.large_title {
                            out.push_str("\n                .nexaLargeTitleDisplayMode()");
                        }
                    }
                    if let Some(search_state) = &tab.search_state {
                        let prompt = tab.search_prompt.as_deref().unwrap_or("Search");
                        out.push_str(&format!(
                            "\n                .searchable(text: ${}, prompt: String(localized: {}))\n                .nexaAvoidHidingSearchToolbar()",
                            state_name(search_state),
                            swift_string(prompt)
                        ));
                    }
                    out.push('\n');
                    for toolbar in tab
                        .children
                        .iter()
                        .filter(|child| matches!(child, Node::Toolbar { .. }))
                    {
                        render_navigation_toolbar(toolbar, module, features, 3, out);
                    }
                    out.push_str("        }\n");
                    out.push_str("    }\n");
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
    let placement = match placement {
        ToolbarPlacement::Leading => ".navigationBarLeading",
        ToolbarPlacement::Trailing => ".navigationBarTrailing",
    };
    out.line_at(depth, format_args!(".toolbar {{"));
    out.line_at(
        depth + 1,
        format_args!("ToolbarItemGroup(placement: {placement}) {{"),
    );
    for child in children {
        crate::generator::components::render_node(child, module, features, depth + 2, out);
        out.push('\n');
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_page_pager(
    state: &str,
    pages: &[Vec<Node>],
    depth: usize,
    out: &mut SourceWriter,
) {
    let prefix = format!("nexa_{}_page", state_name(state));
    out.line_at(
        depth,
        format_args!("TabView(selection: ${}) {{", state_name(state)),
    );
    for index in 0..pages.len() {
        out.line_at(
            depth + 1,
            format_args!("{prefix}_{index}().tag(Int32({index}))"),
        );
    }
    indent(out, depth);
    out.push_str(&format!(
        "}}.tabViewStyle(.page(indexDisplayMode: .always)).indexViewStyle(.page(backgroundDisplayMode: .always)).animation(.default, value: {})",
        state_name(state)
    ));
    out.push('\n');
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_app_bottom_bar(
    state: &str,
    tint: Option<&nexa_ir::ColorExpression>,
    tabs: &[BottomBarTab],
    _module: &Module,
    _features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let has_search = tabs.iter().any(|t| t.role.as_deref() == Some("search"));
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
    if has_search {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(".nexaSearchActivation()");
    }
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
    out.push('\n');
    indent(out, depth);
    out.push('}');
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
