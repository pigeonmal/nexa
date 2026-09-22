use nexa_ir::{Color, ColorValue};

pub(crate) fn expression(color: ColorValue) -> String {
    match color {
        ColorValue::Static(color) => static_expression(color),
        ColorValue::Adaptive { light, dark } => format!(
            "if (nexaIsDarkTheme) {} else {}",
            static_expression(dark),
            static_expression(light)
        ),
    }
}

fn static_expression(color: Color) -> String {
    let argb = (u32::from(color.alpha) << 24)
        | (u32::from(color.red) << 16)
        | (u32::from(color.green) << 8)
        | u32::from(color.blue);
    format!("Color(0x{argb:08X})")
}
