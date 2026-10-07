use nexa_ir::{Color, ColorExpression, ColorValue};

pub(crate) fn expression(color: ColorValue) -> String {
    match color {
        ColorValue::Static(color) => static_expression(color),
        ColorValue::Adaptive { light, dark } => format!(
            "(nexaColorScheme == .dark ? {} : {})",
            static_expression(dark),
            static_expression(light)
        ),
    }
}

pub(crate) fn expression_for_color(color: &ColorExpression) -> String {
    match color {
        ColorExpression::Static(color) => expression(*color),
        ColorExpression::Dynamic(value) => format!(
            "nexaColor(hex: {})",
            crate::generator::expressions::expression(value)
        ),
    }
}

fn static_expression(color: Color) -> String {
    format!(
        "Color(red: {:.6}, green: {:.6}, blue: {:.6}, opacity: {:.6})",
        f64::from(color.red) / 255.0,
        f64::from(color.green) / 255.0,
        f64::from(color.blue) / 255.0,
        f64::from(color.alpha) / 255.0
    )
}

pub(crate) fn expression_from_argb(argb: u32) -> String {
    static_expression(Color {
        red: ((argb >> 16) & 0xFF) as u8,
        green: ((argb >> 8) & 0xFF) as u8,
        blue: (argb & 0xFF) as u8,
        alpha: ((argb >> 24) & 0xFF) as u8,
    })
}
