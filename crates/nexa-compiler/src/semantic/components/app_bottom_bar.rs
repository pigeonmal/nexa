//! Typed app_bottom_bar component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_app_bottom_bar_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let is_pager = inv.name == "PagePager";
    let tab_cx = cx.with_navigation(!is_pager, cx.allow_navigation_back);
    let mut args = inv.arguments;
    let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "AppBottomBar tint", cx))
        .transpose()?;
    let (tabs, custom_bar) = match inv.children {
        ast::ChildBody::Tabs(tabs) => (tabs, None),
        ast::ChildBody::AppBottomBar { tabs, custom_bar } if !is_pager => (tabs, custom_bar),
        _ => return Err(child_mismatch(span)),
    };
    let state = require_mutable_binding(
        &selected,
        &Type::Numeric(NumericType::Int32),
        symbols,
        span,
        &inv.name,
    )?;
    let mut lowered_tabs = Vec::with_capacity(tabs.len());
    for tab in tabs {
        let (index, index_span) = match tab.index {
            ast::Expr::Number(raw, index_span) => (raw, index_span),
            expr => {
                return Err(CompileError::new(
                    expr.span(),
                    "Tab index must be a non-negative Int32 literal",
                ));
            }
        };
        let index = index.parse::<i32>().map_err(|_| {
            CompileError::new(index_span, "Tab index must be a non-negative Int32 literal")
        })?;
        if index < 0
            || lowered_tabs
                .iter()
                .any(|tab: &BottomBarTab| tab.index == index)
        {
            return Err(CompileError::new(
                index_span,
                "AppBottomBar tab indexes must be unique and non-negative",
            ));
        }
        let label = require_string_literal(&tab.label, "Tab label")?;
        let comment = tab.comment;
        let icon = tab
                    .icon
                    .as_ref()
                    .map(|icon| {
                        let name = require_string_literal(icon, "Tab icon")?;
                        cx.resolve_icon(&name).ok_or_else(|| {
                            CompileError::new(
                                icon.span(),
                                format!("unknown tab icon `{name}`; use a Nexa shared icon or an alias from `assets.icons`"),
                            )
                        })
                    })
                    .transpose()?;
        let badge = tab
            .badge
            .as_ref()
            .map(|badge| require_string_literal(badge, "Tab badge"))
            .transpose()?;
        let role = tab
            .role
            .as_ref()
            .map(|role| require_string_literal(role, "Tab role"))
            .transpose()?;
        let navigation_title = tab
            .navigation_title
            .as_ref()
            .map(|title| require_string_literal(title, "Tab title"))
            .transpose()?;
        let large_title = tab
            .large_title
            .as_ref()
            .map(|value| optional_bool(Some(value.clone()), false, "largeTitle"))
            .transpose()?
            .unwrap_or(false);
        let search_state = tab
            .searchable
            .as_ref()
            .map(|value| {
                require_mutable_binding(value, &Type::String, symbols, span, "Tab searchable")
            })
            .transpose()?;
        let search_prompt = tab
            .search_prompt
            .as_ref()
            .map(|prompt| require_string_literal(prompt, "Tab searchPrompt"))
            .transpose()?;
        let children = lower_nodes(tab.children, &tab_cx)?;
        lowered_tabs.push(BottomBarTab {
            index,
            label,
            comment,
            icon,
            badge,
            role,
            navigation_title,
            large_title,
            search_state,
            search_prompt,
            children,
        });
    }
    if is_pager {
        if lowered_tabs
            .iter()
            .enumerate()
            .any(|(index, tab)| tab.index != index as i32)
        {
            return Err(CompileError::new(
                span,
                "PagePager indexes must be contiguous, starting at zero",
            ));
        }
        return Ok(Node::PagePager {
            state,
            pages: lowered_tabs.into_iter().map(|tab| tab.children).collect(),
        });
    }
    Ok(Node::AppBottomBar {
        state,
        tint,
        tabs: lowered_tabs,
        custom_bar: custom_bar
            .map(|nodes| lower_nodes(nodes, &child_cx))
            .transpose()?,
    })
}
