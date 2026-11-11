//! Typed scroll_view component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_scroll_view_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let hide_horizontal_scrollbar = optional_bool(
        args.remove("hideHorizontalScrollbar"),
        true,
        "ScrollView hideHorizontalScrollbar",
    )?;
    let hide_vertical_scrollbar = optional_bool(
        args.remove("hideVerticalScrollbar"),
        true,
        "ScrollView hideVerticalScrollbar",
    )?;
    let axis = match args.remove("axis") {
        None => nexa_ir::adaptive_layout::ScrollAxis::Vertical,
        Some(ast::Expr::Name(name, _)) if name == "Vertical" => {
            nexa_ir::adaptive_layout::ScrollAxis::Vertical
        }
        Some(ast::Expr::Name(name, _)) if name == "Horizontal" => {
            nexa_ir::adaptive_layout::ScrollAxis::Horizontal
        }
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "ScrollView `axis` must be `Vertical` or `Horizontal`",
            ));
        }
    };
    let component_ref = args
        .remove("ref")
        .map(|value| {
            lower_expr(
                &value,
                Some(&Type::ComponentRef {
                    namespace: "Nexa".to_owned(),
                    name: "ScrollView".to_owned(),
                }),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    let style = ast::LayoutStyle {
        alignment: None,
        horizontal_arrangement: None,
        width: args.remove("width"),
        height: args.remove("height"),
        min_width: args.remove("minWidth"),
        max_width: args.remove("maxWidth"),
        min_height: args.remove("minHeight"),
        max_height: args.remove("maxHeight"),
        background: args.remove("background"),
        corner_radius: args.remove("cornerRadius"),
        corner_radius_bottom: args.remove("cornerRadiusBottom"),
        border_color: args.remove("borderColor"),
        border_width: args.remove("borderWidth"),
        opacity: args.remove("opacity"),
        scale: args.remove("scale"),
        scale_y: args.remove("scaleY"),
        rotation: args.remove("rotation"),
        offset_y: args.remove("offsetY"),
        shadow: args.remove("shadow"),
        blur: args.remove("blur"),
        clip: args.remove("clip"),
        z_index: args.remove("zIndex"),
        glass: args.remove("glass"),
        animation: args.remove("animation"),
    };
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::ScrollView {
        axis,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        style: lower_style(style, themes, &cx.exprs(false))?,
        component_ref,
        children: lower_nodes(children, &child_cx)?,
    })
}
