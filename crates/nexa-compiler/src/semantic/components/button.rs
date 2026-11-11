//! Typed button component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_button_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let mut positional = inv.positional;
    let label = take_positional(&mut positional, span, "Button")?;
    let mut options = inv.arguments;
    let comment = take_translator_comment(&mut options, "Button")?;
    let icon = options.get("icon").cloned();
    let loading = options.get("loading").cloned();
    let disabled = options.get("disabled").cloned();
    let accessibility_label = options
        .remove("accessibilityLabel")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .map(|value| localize_ui_text(value, None));
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    let label = localize_ui_text(
        lower_expr(&label, Some(&Type::String), &cx.exprs(false))?,
        comment.clone(),
    );
    if !matches!(
        label,
        Expr::String(_)
            | Expr::Interpolation(_)
            | Expr::LocalizedText { .. }
            | Expr::State(_, Type::String)
            | Expr::NativeCall {
                return_type: Type::String,
                ..
            }
            | Expr::Call {
                return_type: Type::String,
                ..
            }
            | Expr::Conditional {
                value_type: Type::String,
                ..
            }
    ) {
        return Err(CompileError::new(span, "Button label must be a String"));
    }
    let icon = icon
                .map(|value| {
                    let name = require_string_literal(&value, "Button icon")?;
                    let resolved = cx.resolve_icon(&name);
                    resolved.ok_or_else(|| {
                        CompileError::new(
                            value.span(),
                            format!("unknown icon `{name}` in Button `icon`; use a Nexa shared icon, an alias from `assets.icons`, or the `Icon` component for platform-specific symbols"),
                        )
                    })
                })
                .transpose()?;
    let loading = loading
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let disabled = disabled
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let style = options
        .get("style")
        .cloned()
        .map(|v| match v {
            ast::Expr::String(s, span) => match s.as_str() {
                "prominent" | "borderedProminent" => Ok(ButtonStyle::BorderedProminent),
                "bordered" => Ok(ButtonStyle::Bordered),
                "borderless" => Ok(ButtonStyle::Borderless),
                "plain" => Ok(ButtonStyle::Plain),
                other => Err(CompileError::new(
                    span,
                    format!("unknown button style `{other}`"),
                )),
            },
            ast::Expr::Name(s, span) => match s.as_str() {
                "BorderedProminent" | "Prominent" => Ok(ButtonStyle::BorderedProminent),
                "Bordered" => Ok(ButtonStyle::Bordered),
                "Borderless" => Ok(ButtonStyle::Borderless),
                "Plain" => Ok(ButtonStyle::Plain),
                other => Err(CompileError::new(
                    span,
                    format!("unknown button style `{other}`"),
                )),
            },
            _ => Err(CompileError::new(
                v.span(),
                "button style must be a String or enum",
            )),
        })
        .transpose()?;
    let size = options
        .get("size")
        .cloned()
        .map(|v| match v {
            ast::Expr::String(s, span) => match s.as_str() {
                "small" => Ok(ButtonSize::Small),
                "regular" => Ok(ButtonSize::Regular),
                "large" => Ok(ButtonSize::Large),
                other => Err(CompileError::new(
                    span,
                    format!("unknown button size `{other}`"),
                )),
            },
            ast::Expr::Name(s, span) => match s.as_str() {
                "Small" => Ok(ButtonSize::Small),
                "Regular" => Ok(ButtonSize::Regular),
                "Large" => Ok(ButtonSize::Large),
                other => Err(CompileError::new(
                    span,
                    format!("unknown button size `{other}`"),
                )),
            },
            _ => Err(CompileError::new(
                v.span(),
                "button size must be a String or enum",
            )),
        })
        .transpose()?;
    let shape = options
        .get("shape")
        .cloned()
        .map(|value| parse_shape(value, "button shape"))
        .transpose()?;
    let tint = options
        .get("tint")
        .cloned()
        .map(|value| lower_color_expression(value, "button tint", cx))
        .transpose()?;
    let glass = optional_bool(options.remove("glass"), false, "Button glass")?;
    let lowered = lower_actions_with_aliases(
        actions,
        symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    Ok(Node::Button {
        label,
        icon,
        loading,
        disabled,
        style,
        size,
        shape,
        tint,
        glass,
        accessibility_label: accessibility_label.map(Box::new),
        actions: lowered,
    })
}
