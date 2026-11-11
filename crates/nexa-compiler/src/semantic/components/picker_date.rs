//! Typed picker_date component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_picker_date_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let span = inv.span;
    let mut args = inv.arguments;
    let required = take_required_arg(&mut args, &inv.name, "type", span)?;
    let picker_type = match enum_argument(&required, "PickerDate type")?.as_str() {
        "Date" => PickerDateType::Date,
        "Time" => PickerDateType::Time,
        "DateTime" => PickerDateType::DateTime,
        value => {
            return Err(CompileError::new(
                required.span(),
                format!("unknown PickerDate type `{value}`; expected Date, Time, or DateTime"),
            ));
        }
    };
    let style = args
                .remove("style")
                .map(|value| match enum_argument(&value, "PickerDate style")?.as_str() {
                    "Automatic" => Ok(PickerDateStyle::Automatic),
                    "Graphical" => Ok(PickerDateStyle::Graphical),
                    "Compact" => Ok(PickerDateStyle::Compact),
                    "Input" => Ok(PickerDateStyle::Input),
                    "Wheel" => Ok(PickerDateStyle::Wheel),
                    other => Err(CompileError::new(value.span(), format!("unknown PickerDate style `{other}`; expected Automatic, Graphical, Compact, Input, or Wheel"))),
                })
                .transpose()?
                .unwrap_or(PickerDateStyle::Graphical);
    let selection = args
        .remove("selection")
        .map(
            |value| match enum_argument(&value, "PickerDate selection")?.as_str() {
                "Single" => Ok(PickerDateSelection::Single),
                "Range" => Ok(PickerDateSelection::Range),
                other => Err(CompileError::new(
                    value.span(),
                    format!("unknown PickerDate selection `{other}`; expected Single or Range"),
                )),
            },
        )
        .transpose()?
        .unwrap_or(PickerDateSelection::Single);

    let timestamp = args.remove("timestamp");
    let start_timestamp = args.remove("startTimestamp");
    let end_timestamp = args.remove("endTimestamp");
    match selection {
        PickerDateSelection::Single if start_timestamp.is_some() || end_timestamp.is_some() => {
            return Err(CompileError::new(
                span,
                "PickerDate `startTimestamp` and `endTimestamp` require `selection: Range`",
            ));
        }
        PickerDateSelection::Range if picker_type != PickerDateType::Date => {
            return Err(CompileError::new(
                span,
                "PickerDate `selection: Range` is only supported with `type: Date`",
            ));
        }
        PickerDateSelection::Range if timestamp.is_some() => {
            return Err(CompileError::new(
                span,
                "PickerDate `timestamp` cannot be combined with `selection: Range`; use `startTimestamp` and `endTimestamp`",
            ));
        }
        _ => {}
    }
    if start_timestamp.is_some() != end_timestamp.is_some() {
        return Err(CompileError::new(
            span,
            "PickerDate range binding requires both `startTimestamp` and `endTimestamp`, or neither",
        ));
    }
    let timestamp_state = timestamp
        .as_ref()
        .map(|value| {
            require_mutable_binding(
                value,
                &Type::Numeric(NumericType::Int64),
                symbols,
                span,
                "PickerDate `timestamp`",
            )
        })
        .transpose()?;
    let start_timestamp_state = start_timestamp
        .as_ref()
        .map(|value| {
            require_mutable_binding(
                value,
                &Type::Numeric(NumericType::Int64),
                symbols,
                span,
                "PickerDate `startTimestamp`",
            )
        })
        .transpose()?;
    let end_timestamp_state = end_timestamp
        .as_ref()
        .map(|value| {
            require_mutable_binding(
                value,
                &Type::Numeric(NumericType::Int64),
                symbols,
                span,
                "PickerDate `endTimestamp`",
            )
        })
        .transpose()?;
    let minimum = args
        .remove("minimum")
        .map(|value| {
            lower_expr(
                &value,
                Some(&Type::Numeric(NumericType::Int64)),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    let maximum = args
        .remove("maximum")
        .map(|value| {
            lower_expr(
                &value,
                Some(&Type::Numeric(NumericType::Int64)),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "PickerDate tint", cx))
        .transpose()?;
    let text_color = args
        .remove("textColor")
        .map(|value| lower_color_expression(value, "PickerDate textColor", cx))
        .transpose()?;
    let background_color = args
        .remove("backgroundColor")
        .map(|value| lower_color_expression(value, "PickerDate backgroundColor", cx))
        .transpose()?;
    let picker_background_color = args
        .remove("pickerBackgroundColor")
        .map(|value| lower_color_expression(value, "PickerDate pickerBackgroundColor", cx))
        .transpose()?;
    let border_color = args
        .remove("borderColor")
        .map(|value| lower_color_expression(value, "PickerDate borderColor", cx))
        .transpose()?;
    let border_width = args
        .remove("borderWidth")
        .map(|value| number_value(&value, "PickerDate borderWidth"))
        .transpose()?;
    let corner_radius = args
        .remove("cornerRadius")
        .map(|value| number_value(&value, "PickerDate cornerRadius"))
        .transpose()?;
    let locale = args
        .remove("locale")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?;
    let time_zone = args
        .remove("timeZone")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?;
    let is_24_hour = args
        .remove("is24Hour")
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let enabled = args
        .remove("enabled")
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let accessibility_label = args
        .remove("accessibilityLabel")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .map(|value| localize_ui_text(value, None));

    Ok(Node::PickerDate {
        props: Box::new(nexa_ir::PickerDateProps {
            picker_type,
            style,
            selection,
            timestamp_state,
            start_timestamp_state,
            end_timestamp_state,
            minimum,
            maximum,
            tint,
            text_color,
            background_color,
            picker_background_color,
            border_color,
            border_width,
            corner_radius,
            locale,
            time_zone,
            is_24_hour,
            enabled,
            accessibility_label,
        }),
    })
}
