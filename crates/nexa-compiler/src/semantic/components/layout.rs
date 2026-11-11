//! Typed layout component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_column_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let stacked = inv.name == "Stack";
    let kind = match inv.name.as_str() {
        "Column" => LayoutKind::Column,
        "Row" => LayoutKind::Row,
        _ => LayoutKind::Stack,
    };
    let mut args = inv.arguments;
    let gap = args.remove("gap");
    let marquee = match args.get("animation") {
        Some(value) if super::super::marquee::is_marquee(value) => {
            if stacked {
                return Err(CompileError::new(
                    span,
                    "Marquee requires Row or Column; wrap Stack inside a Row or Column",
                ));
            }
            let value = args
                .remove("animation")
                .ok_or_else(|| CompileError::new(span, "missing marquee animation"))?;
            Some(Box::new(super::super::marquee::lower(
                value,
                &cx.exprs(false),
            )?))
        }
        _ => None,
    };
    let style = ast::LayoutStyle {
        alignment: args.remove("alignment"),
        horizontal_arrangement: args.remove("horizontalArrangement"),
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
    let gap =
        optional_dimension(gap, "gap", Some(ast::ThemeTokenKind::Spacing), themes)?.unwrap_or(0.0);
    let mut style = lower_style(style, themes, &cx.exprs(false))?;
    style.marquee = marquee;
    let lowered = lower_nodes(children, &child_cx)?;
    if !stacked
        && lowered.is_empty()
        && (style.width.is_some() || style.height.is_some())
        && style.min_width.is_none()
        && style.max_width.is_none()
        && style.min_height.is_none()
        && style.max_height.is_none()
        && style.background.is_none()
        && style.corner_radius.is_none()
        && style.corner_radius_expression.is_none()
        && style.corner_radius_bottom.is_none()
        && style.border_color.is_none()
        && style.border_width.is_none()
        && style.opacity.is_none()
        && !style.effects.has_modifiers()
        && style.animation.is_none()
        && style.marquee.is_none()
    {
        return Ok(Node::Spacer {
            width: Some(style.width.unwrap_or(nexa_ir::Dimension::Fixed(0.0))),
            height: Some(style.height.unwrap_or(nexa_ir::Dimension::Fixed(0.0))),
        });
    }
    Ok(Node::Layout {
        kind,
        gap,
        style,
        children: lowered,
    })
}
