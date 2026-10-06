use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Alignment, AnimationSpec, Color, ColorValue, ViewEffects, ViewShadow, ViewStyle};
use nexa_syntax::ast;

use super::themes::ThemeSymbols;

pub(super) fn lower_style(
    style: ast::LayoutStyle,
    themes: &ThemeSymbols,
) -> Result<ViewStyle, CompileError> {
    let min_width_span = style.min_width.as_ref().map(ast::Expr::span);
    let max_width_span = style.max_width.as_ref().map(ast::Expr::span);
    let min_height_span = style.min_height.as_ref().map(ast::Expr::span);
    let max_height_span = style.max_height.as_ref().map(ast::Expr::span);
    let padding = optional_dimension(
        style.padding,
        "padding",
        Some(ast::ThemeTokenKind::Spacing),
        themes,
    )?;
    let width = optional_dimension(style.width, "width", None, themes)?;
    let height = optional_dimension(style.height, "height", None, themes)?;
    let min_width = optional_dimension(style.min_width, "minWidth", None, themes)?;
    let max_width = optional_dimension(style.max_width, "maxWidth", None, themes)?;
    let min_height = optional_dimension(style.min_height, "minHeight", None, themes)?;
    let max_height = optional_dimension(style.max_height, "maxHeight", None, themes)?;
    validate_bounds(
        min_width,
        max_width,
        min_width_span.or(max_width_span).unwrap_or_default(),
        "minWidth",
        "maxWidth",
    )?;
    validate_bounds(
        min_height,
        max_height,
        min_height_span.or(max_height_span).unwrap_or_default(),
        "minHeight",
        "maxHeight",
    )?;
    let corner_radius = optional_dimension(
        style.corner_radius,
        "cornerRadius",
        Some(ast::ThemeTokenKind::Radius),
        themes,
    )?;
    let border_color_span = style.border_color.as_ref().map(ast::Expr::span);
    let border_width_span = style.border_width.as_ref().map(ast::Expr::span);
    let border_color = style
        .border_color
        .map(|color| parse_color(color, themes, "borderColor"))
        .transpose()?;
    let border_width = optional_dimension(style.border_width, "borderWidth", None, themes)?;
    if border_color.is_some() != border_width.is_some() {
        return Err(CompileError::new(
            border_color_span.or(border_width_span).unwrap_or_default(),
            "borderColor and borderWidth must be provided together",
        ));
    }
    let opacity = lower_opacity(style.opacity)?;
    let effects = lower_view_effects(
        style.scale,
        style.rotation,
        style.shadow,
        style.blur,
        style.clip,
        style.z_index,
        style.glass,
        themes,
    )?;
    let alignment = style.alignment.map(parse_alignment).transpose()?;
    let animation = style.animation.map(parse_animation).transpose()?;
    let background = style
        .background
        .map(|color| parse_color(color, themes, "background"))
        .transpose()?;
    Ok(ViewStyle {
        alignment,
        padding,
        width,
        height,
        min_width,
        max_width,
        min_height,
        max_height,
        background,
        corner_radius,
        border_color,
        border_width,
        opacity,
        effects,
        animation,
    })
}

pub(super) fn lower_opacity(expr: Option<ast::Expr>) -> Result<Option<f32>, CompileError> {
    expr.map(|expr| {
        let value = number_value(&expr, "opacity")?;
        if !(0.0..=1.0).contains(&value) {
            return Err(CompileError::new(
                expr.span(),
                "opacity must be between 0 and 1",
            ));
        }
        Ok(value)
    })
    .transpose()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_view_effects(
    scale: Option<ast::Expr>,
    rotation: Option<ast::Expr>,
    shadow: Option<ast::Expr>,
    blur: Option<ast::Expr>,
    clip: Option<ast::Expr>,
    z_index: Option<ast::Expr>,
    glass: Option<ast::Expr>,
    themes: &ThemeSymbols,
) -> Result<ViewEffects, CompileError> {
    let scale = scale
        .map(|expr| finite_number_value(&expr, "scale"))
        .transpose()?;
    let rotation = rotation
        .map(|expr| finite_number_value(&expr, "rotation"))
        .transpose()?;
    let blur = blur
        .map(|expr| number_value(&expr, "blur radius"))
        .transpose()?;
    let shadow = shadow.map(|expr| lower_shadow(expr, themes)).transpose()?;
    let clip_rounded = clip.map(lower_rounded_clip).transpose()?;
    let z_index = z_index.map(lower_z_index).transpose()?;
    let glass = glass.map(|expr| lower_glass(expr, themes)).transpose()?;
    Ok(ViewEffects {
        scale,
        rotation,
        shadow,
        blur,
        clip_rounded,
        z_index,
        glass,
    })
}

pub(super) fn lower_glass(
    expr: ast::Expr,
    themes: &ThemeSymbols,
) -> Result<nexa_ir::ViewGlass, CompileError> {
    match expr {
        ast::Expr::String(ref s, span) => {
            let shape = parse_glass_shape(s, span)?;
            Ok(nexa_ir::ViewGlass { tint: None, shape })
        }
        ast::Expr::Bool(true, _) => Ok(nexa_ir::ViewGlass {
            tint: None,
            shape: nexa_ir::GlassShape::Capsule,
        }),
        ast::Expr::Call(ref name, _, ref args, _span) if name == "Glass" => {
            let mut shape = nexa_ir::GlassShape::Circle;
            let mut tint = None;
            for arg in args {
                match arg {
                    ast::Expr::String(s, span) => {
                        shape = parse_glass_shape(s, *span)?;
                    }
                    _ => {
                        if let Ok(c) = parse_color(arg.clone(), themes, "glass tint") {
                            tint = Some(c);
                        }
                    }
                }
            }
            Ok(nexa_ir::ViewGlass { tint, shape })
        }
        _ => Err(CompileError::new(
            expr.span(),
            "glass expects a shape string (\"circle\", \"capsule\", \"rounded\") or Glass(shape, tint: ...)",
        )),
    }
}

pub(super) fn parse_glass_shape(s: &str, span: Span) -> Result<nexa_ir::GlassShape, CompileError> {
    match s.to_ascii_lowercase().as_str() {
        "circle" => Ok(nexa_ir::GlassShape::Circle),
        "capsule" => Ok(nexa_ir::GlassShape::Capsule),
        "rounded" => Ok(nexa_ir::GlassShape::Rounded(16.0)),
        other => {
            if let Some(r) = other
                .strip_prefix("rounded(")
                .and_then(|r| r.strip_suffix(")"))
                .and_then(|n| n.parse::<f32>().ok())
            {
                return Ok(nexa_ir::GlassShape::Rounded(r));
            }
            Err(CompileError::new(
                span,
                format!("unknown glass shape `{s}`; expected circle, capsule, or rounded"),
            ))
        }
    }
}

fn lower_shadow(expr: ast::Expr, themes: &ThemeSymbols) -> Result<ViewShadow, CompileError> {
    let ast::Expr::Call(name, type_arguments, mut arguments, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "shadow must specify radius, x, y, and color",
        ));
    };
    if name != "Shadow" || !type_arguments.is_empty() || arguments.len() != 4 {
        return Err(CompileError::new(
            span,
            "shadow must use `Shadow(radius, x, y, color)`",
        ));
    }
    let color_expr = arguments
        .pop()
        .ok_or_else(|| CompileError::new(span, "shadow must use `Shadow(radius, x, y, color)`"))?;
    let y_expr = arguments
        .pop()
        .ok_or_else(|| CompileError::new(span, "shadow must use `Shadow(radius, x, y, color)`"))?;
    let x_expr = arguments
        .pop()
        .ok_or_else(|| CompileError::new(span, "shadow must use `Shadow(radius, x, y, color)`"))?;
    let radius_expr = arguments
        .pop()
        .ok_or_else(|| CompileError::new(span, "shadow must use `Shadow(radius, x, y, color)`"))?;
    Ok(ViewShadow {
        radius: number_value(&radius_expr, "shadow radius")?,
        x: finite_number_value(&x_expr, "shadow x offset")?,
        y: finite_number_value(&y_expr, "shadow y offset")?,
        color: parse_color(color_expr, themes, "shadow color")?,
    })
}

fn lower_rounded_clip(expr: ast::Expr) -> Result<f32, CompileError> {
    let ast::Expr::Call(name, type_arguments, arguments, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "clip shape must be `Rounded(radius)`",
        ));
    };
    if name != "Rounded" || !type_arguments.is_empty() || arguments.len() != 1 {
        return Err(CompileError::new(
            span,
            "clip shape must be `Rounded(radius)`",
        ));
    }
    number_value(&arguments[0], "clip radius")
}

fn lower_z_index(expr: ast::Expr) -> Result<i32, CompileError> {
    let span = expr.span();
    let value = match expr {
        ast::Expr::Number(raw, _) => raw.parse::<i64>().ok(),
        ast::Expr::Negate(inner, _) => match *inner {
            ast::Expr::Number(raw, _) => raw.parse::<i64>().ok().and_then(i64::checked_neg),
            _ => None,
        },
        _ => None,
    }
    .ok_or_else(|| CompileError::new(span, "zIndex must be an Int32 literal"))?;
    i32::try_from(value).map_err(|_| CompileError::new(span, "zIndex is outside the Int32 range"))
}

fn finite_number_value(expr: &ast::Expr, field: &str) -> Result<f32, CompileError> {
    let (raw, span, negative) = match expr {
        ast::Expr::Number(raw, span) => (raw.as_str(), *span, false),
        ast::Expr::Negate(inner, span) => match inner.as_ref() {
            ast::Expr::Number(raw, _) => (raw.as_str(), *span, true),
            _ => {
                return Err(CompileError::new(
                    expr.span(),
                    format!("`{field}` must be a finite numeric literal"),
                ));
            }
        },
        _ => {
            return Err(CompileError::new(
                expr.span(),
                format!("`{field}` must be a finite numeric literal"),
            ));
        }
    };
    let value = raw.parse::<f32>().map_err(|_| {
        CompileError::new(
            span,
            format!("`{field}` is outside the supported numeric range"),
        )
    })?;
    let value = if negative { -value } else { value };
    if !value.is_finite() {
        return Err(CompileError::new(
            span,
            format!("`{field}` must be a finite numeric literal"),
        ));
    }
    Ok(value)
}

fn validate_bounds(
    minimum: Option<f32>,
    maximum: Option<f32>,
    span: Span,
    minimum_name: &str,
    maximum_name: &str,
) -> Result<(), CompileError> {
    if let (Some(minimum), Some(maximum)) = (minimum, maximum)
        && minimum > maximum
    {
        return Err(CompileError::new(
            span,
            format!("`{minimum_name}` cannot be greater than `{maximum_name}`"),
        ));
    }
    Ok(())
}

pub(super) fn parse_animation(expr: ast::Expr) -> Result<AnimationSpec, CompileError> {
    match expr {
        ast::Expr::Name(name, span) => match name.as_str() {
            "Spring" => Ok(AnimationSpec::Spring {
                response: 0.5,
                damping: 0.825,
            }),
            "EaseIn" => Ok(AnimationSpec::EaseIn),
            "EaseOut" => Ok(AnimationSpec::EaseOut),
            "EaseInOut" => Ok(AnimationSpec::EaseInOut),
            "Linear" => Ok(AnimationSpec::Linear),
            _ => Err(CompileError::new(
                span,
                "animation must be `Spring`, `Spring(response: ..., damping: ...)`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`",
            )),
        },
        ast::Expr::Call(name, type_arguments, arguments, _span)
            if name == "Spring" && type_arguments.is_empty() && arguments.len() == 2 =>
        {
            let response = finite_number_value(&arguments[0], "Spring response")?;
            let damping = finite_number_value(&arguments[1], "Spring damping")?;
            if response <= 0.0 {
                return Err(CompileError::new(
                    arguments[0].span(),
                    "Spring response must be greater than zero",
                ));
            }
            if damping <= 0.0 {
                return Err(CompileError::new(
                    arguments[1].span(),
                    "Spring damping must be greater than zero",
                ));
            }
            let stiffness = 100.0 / f64::from(response).powi(2);
            if !stiffness.is_finite()
                || stiffness < f64::from(f32::MIN_POSITIVE)
                || stiffness > f64::from(f32::MAX)
            {
                return Err(CompileError::new(
                    arguments[0].span(),
                    "Spring response is outside the supported native animation range",
                ));
            }
            Ok(AnimationSpec::Spring { response, damping })
        }
        other => Err(CompileError::new(
            other.span(),
            "animation must be `Spring`, `Spring(response: ..., damping: ...)`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`",
        )),
    }
}

fn parse_alignment(expr: ast::Expr) -> Result<Alignment, CompileError> {
    match expr {
        ast::Expr::Name(name, span) => match name.as_str() {
            "Start" => Ok(Alignment::Start),
            "Center" => Ok(Alignment::Center),
            "End" => Ok(Alignment::End),
            _ => Err(CompileError::new(
                span,
                "alignment must be `Start`, `Center`, or `End`",
            )),
        },
        expr => Err(CompileError::new(
            expr.span(),
            "alignment must be `Start`, `Center`, or `End`",
        )),
    }
}

pub(super) fn optional_color(
    expr: Option<ast::Expr>,
    role: &str,
    themes: &ThemeSymbols,
) -> Result<Option<ColorValue>, CompileError> {
    expr.map(|expr| parse_color(expr, themes, role)).transpose()
}

pub(super) fn optional_dimension(
    expr: Option<ast::Expr>,
    field: &str,
    theme_kind: Option<ast::ThemeTokenKind>,
    themes: &ThemeSymbols,
) -> Result<Option<f32>, CompileError> {
    expr.map(|expr| dimension_value(&expr, field, theme_kind, themes))
        .transpose()
}

pub(super) fn number_value(expr: &ast::Expr, field: &str) -> Result<f32, CompileError> {
    let ast::Expr::Number(raw, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            format!("`{field}` must be a non-negative numeric literal"),
        ));
    };
    let value = raw.parse::<f32>().map_err(|_| {
        CompileError::new(
            *span,
            format!("`{field}` is outside the supported dimension range"),
        )
    })?;
    if !value.is_finite() || value < 0.0 {
        return Err(CompileError::new(
            *span,
            format!("`{field}` must be finite and non-negative"),
        ));
    }
    Ok(value)
}

fn dimension_value(
    expr: &ast::Expr,
    field: &str,
    theme_kind: Option<ast::ThemeTokenKind>,
    themes: &ThemeSymbols,
) -> Result<f32, CompileError> {
    match expr {
        ast::Expr::Number(_, _) => number_value(expr, field),
        ast::Expr::ThemeToken(name, span) => match theme_kind {
            Some(kind) => themes.metric(name, *span, kind, field),
            None => Err(CompileError::new(
                *span,
                format!("`{field}` does not accept a theme token"),
            )),
        },
        _ => Err(CompileError::new(
            expr.span(),
            format!("`{field}` must be a non-negative numeric literal or matching theme token"),
        )),
    }
}

pub(super) fn parse_color(
    expr: ast::Expr,
    themes: &ThemeSymbols,
    role: &str,
) -> Result<ColorValue, CompileError> {
    match expr {
        ast::Expr::ThemeToken(name, span) => themes.color(&name, span, role),
        literal => Ok(ColorValue::Static(parse_color_literal(literal, role)?)),
    }
}

pub(super) fn parse_color_literal(expr: ast::Expr, role: &str) -> Result<Color, CompileError> {
    let ast::Expr::String(value, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            format!("`{role}` must be a hexadecimal color string"),
        ));
    };
    let Some(hex) = value.strip_prefix('#') else {
        return Err(CompileError::new(
            span,
            "color must use `#RGB`, `#RRGGBB`, or `#RRGGBBAA` notation",
        ));
    };
    if !hex.is_ascii() {
        return Err(invalid_color(span));
    }
    let (red, green, blue, alpha) = match hex.len() {
        3 => {
            let mut values = hex.chars().map(|character| character.to_digit(16));
            let red = values.next().flatten();
            let green = values.next().flatten();
            let blue = values.next().flatten();
            let (Some(red), Some(green), Some(blue)) = (red, green, blue) else {
                return Err(invalid_color(span));
            };
            (red * 17, green * 17, blue * 17, 255)
        }
        6 | 8 => {
            let red = color_channel(&hex[0..2], span)?;
            let green = color_channel(&hex[2..4], span)?;
            let blue = color_channel(&hex[4..6], span)?;
            let alpha = if hex.len() == 8 {
                color_channel(&hex[6..8], span)?
            } else {
                255
            };
            (
                u32::from(red),
                u32::from(green),
                u32::from(blue),
                u32::from(alpha),
            )
        }
        _ => return Err(invalid_color(span)),
    };
    Ok(Color {
        red: red as u8,
        green: green as u8,
        blue: blue as u8,
        alpha: alpha as u8,
    })
}

fn color_channel(value: &str, span: Span) -> Result<u8, CompileError> {
    u8::from_str_radix(value, 16).map_err(|_| invalid_color(span))
}

fn invalid_color(span: Span) -> CompileError {
    CompileError::new(
        span,
        "color must use `#RGB`, `#RRGGBB`, or `#RRGGBBAA` hexadecimal notation",
    )
}
