//! Typed navigation component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_toolbar_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let placement = match args.remove("placement") {
        Some(ast::Expr::Name(name, _)) if name == "Leading" => ToolbarPlacement::Leading,
        Some(ast::Expr::Name(name, _)) if name == "Trailing" => ToolbarPlacement::Trailing,
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "Toolbar placement must be `Leading` or `Trailing`",
            ));
        }
        None => ToolbarPlacement::Trailing,
    };
    let children = match inv.children {
        ast::ChildBody::Nodes(nodes) => lower_nodes(nodes, &child_cx)?,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::Toolbar {
        placement,
        children,
    })
}

#[inline(never)]
pub(super) fn lower_navigation_stack_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let screen_ids = cx.screen_ids;
    let functions = cx.functions;
    let allow_navigation_stack = cx.allow_navigation_stack;

    let span = inv.span;
    let mut args = inv.arguments;
    let root = take_required_arg(&mut args, &inv.name, "root", span)?;
    let show_navigation_bar = optional_bool(
        args.remove("showNavigationBar"),
        true,
        "NavigationStack showNavigationBar",
    )?;
    let title_mode = match args.remove("titleMode") {
        Some(ast::Expr::Name(name, _)) if name == "Inline" => nexa_ir::NavigationTitleMode::Inline,
        Some(ast::Expr::Name(name, _)) if name == "Large" => nexa_ir::NavigationTitleMode::Large,
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "NavigationStack titleMode must be `Inline` or `Large`",
            ));
        }
        None => nexa_ir::NavigationTitleMode::default(),
    };
    let title_alignment = match args.remove("titleAlignment") {
        Some(ast::Expr::Name(name, _)) if name == "Automatic" => {
            nexa_ir::NavigationTitleAlignment::Automatic
        }
        Some(ast::Expr::Name(name, _)) if name == "Leading" => {
            nexa_ir::NavigationTitleAlignment::Leading
        }
        Some(ast::Expr::Name(name, _)) if name == "Center" => {
            nexa_ir::NavigationTitleAlignment::Center
        }
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "NavigationStack titleAlignment must be `Automatic`, `Leading`, or `Center`",
            ));
        }
        None => nexa_ir::NavigationTitleAlignment::default(),
    };
    let (root, arguments) = ast::split_navigation_target(root);
    if !allow_navigation_stack {
        return Err(CompileError::new(
            span,
            "NavigationStack is only allowed at the app body or at the root of an AppBottomBar tab",
        ));
    }
    let root = lower_screen_target(
        &root,
        arguments,
        screen_ids,
        symbols,
        functions,
        cx.type_registries(),
        "NavigationStack root",
    )?;
    Ok(Node::NavigationStack {
        root: root.0,
        arguments: root.1,
        show_navigation_bar,
        title_mode,
        title_alignment,
    })
}

#[inline(never)]
pub(super) fn lower_navigation_split_view_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let detail_visible = take_required_arg(&mut args, &inv.name, "detailVisible", span)?;
    let detail_visible = require_mutable_binding(
        &detail_visible,
        &Type::Bool,
        symbols,
        span,
        "NavigationSplitView detailVisible",
    )?;
    let ast::ChildBody::SplitPanes { sidebar, detail } = inv.children else {
        return Err(child_mismatch(span));
    };
    Ok(Node::NavigationSplitView {
        detail_visible,
        sidebar: lower_nodes(sidebar, &child_cx)?,
        detail: lower_nodes(detail, &child_cx)?,
    })
}

#[inline(never)]
pub(super) fn lower_navigation_back_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let allow_navigation_back = cx.allow_navigation_back;

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "NavigationBack")?;
    let label = args.remove("label");
    if !allow_navigation_back {
        return Err(CompileError::new(
            span,
            "NavigationBack is only allowed inside a declared screen",
        ));
    }
    let label = label
        .map(|label| lower_expr(&label, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .unwrap_or_else(|| Expr::String("Back".to_owned()));
    Ok(Node::NavigationBack {
        label: localize_ui_text(label, comment.clone()),
    })
}

#[inline(never)]
pub(super) fn lower_navigation_link_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let screen_ids = cx.screen_ids;
    let functions = cx.functions;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let destination = take_required_arg(&mut args, &inv.name, "destination", span)?;
    let guard = args.remove("when");
    let (destination, arguments) = ast::split_navigation_target(destination);
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let destination = lower_screen_target(
        &destination,
        arguments,
        screen_ids,
        symbols,
        functions,
        cx.type_registries(),
        "NavigationLink destination",
    )
    .map_err(|error| {
        if screen_ids.is_empty() {
            CompileError::new(span, "NavigationLink requires declared app screens")
        } else {
            error
        }
    })?;
    let guard = guard
        .map(|guard| lower_expr(&guard, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let lowered_children = lower_nodes(children, &child_cx)?;
    Ok(Node::NavigationLink {
        destination: destination.0,
        arguments: destination.1,
        guard,
        children: lowered_children,
    })
}

#[inline(never)]
pub(super) fn lower_link_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let url = take_required_arg(&mut args, &inv.name, "url", span)?;
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let lowered_url = lower_expr(&url, Some(&Type::String), &cx.exprs(false))?;
    if let ast::Expr::String(value, _) = &url
        && !is_link_url(value)
    {
        return Err(CompileError::new(
            span,
            "Link URL must include a valid absolute scheme (for example `https://` or `mailto:`)",
        ));
    }
    let children = lower_nodes(children, &child_cx)?;
    Ok(Node::Link {
        url: lowered_url,
        children,
    })
}
