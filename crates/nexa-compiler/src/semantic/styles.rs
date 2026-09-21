use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Color, ViewStyle};
use nexa_syntax::ast;

pub(super) fn lower_style(style: ast::LayoutStyle) -> Result<ViewStyle, CompileError> {
    let padding = optional_dimension(style.padding, "padding")?;
    let width = optional_dimension(style.width, "width")?;
    let height = optional_dimension(style.height, "height")?;
    let corner_radius = optional_dimension(style.corner_radius, "cornerRadius")?;
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
    let background = style.background.map(parse_color).transpose()?;
    Ok(ViewStyle {
        padding,
        width,
        height,
        background,
        corner_radius,
        opacity,
    })
}

pub(super) fn optional_dimension(
    expr: Option<ast::Expr>,
    field: &str,
) -> Result<Option<f32>, CompileError> {
    expr.map(|expr| number_value(&expr, field)).transpose()
}

fn number_value(expr: &ast::Expr, field: &str) -> Result<f32, CompileError> {
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

fn parse_color(expr: ast::Expr) -> Result<Color, CompileError> {
    let ast::Expr::String(value, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "`background` must be a hexadecimal color string",
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
