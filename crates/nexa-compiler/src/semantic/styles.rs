use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Alignment, AnimationSpec, Color, ColorValue, ViewStyle};
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
    let opacity = match style.opacity {
        Some(expr) => {
            let value = number_value(&expr, "opacity")?;
            if !(0.0..=1.0).contains(&value) {
                return Err(CompileError::new(
                    expr.span(),
                    "opacity must be between 0 and 1",
                ));
            }
            Some(value)
        }
        None => None,
    };
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
        animation,
    })
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

fn parse_animation(expr: ast::Expr) -> Result<AnimationSpec, CompileError> {
    let ast::Expr::Name(name, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "animation must be `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`",
        ));
    };
    match name.as_str() {
        "Spring" => Ok(AnimationSpec::Spring),
        "EaseIn" => Ok(AnimationSpec::EaseIn),
        "EaseOut" => Ok(AnimationSpec::EaseOut),
        "EaseInOut" => Ok(AnimationSpec::EaseInOut),
        "Linear" => Ok(AnimationSpec::Linear),
        _ => Err(CompileError::new(
            span,
            "animation must be `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`",
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

fn parse_color(
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
