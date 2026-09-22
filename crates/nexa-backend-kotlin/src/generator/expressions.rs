use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{BinaryOp, Expr, InterpolatedPart, NumericType, Type};

use super::utils::{kotlin_string, kotlin_string_content};

pub(super) fn expression(expr: &Expr) -> String {
    match expr {
        Expr::String(value) => kotlin_string(value),
        Expr::Interpolation(parts) => {
            let mut value = String::from("\"");
            for part in parts {
                match part {
                    InterpolatedPart::Literal(literal) => {
                        value.push_str(&kotlin_string_content(literal));
                    }
                    InterpolatedPart::Value(value_expr) => {
                        value.push_str("${");
                        value.push_str(&expression(value_expr));
                        value.push('}');
                    }
                }
            }
            value.push('"');
            value
        }
        Expr::Bool(value) => value.to_string(),
        Expr::IsRegularWidth => "(LocalConfiguration.current.screenWidthDp >= 600)".to_owned(),
        Expr::Number { raw, ty } => kotlin_number(raw, *ty),
        Expr::State(name, _) => state_name(name),
        Expr::Not(value) => format!("(!{})", expression(value)),
        Expr::Null(_) => "null".to_owned(),
        Expr::Coalesce(left, right) => {
            format!("({} ?: {})", expression(left), expression(right))
        }
        Expr::Index {
            collection, index, ..
        } => format!("{}[{}]", expression(collection), expression(index)),
        Expr::Range {
            start,
            end,
            inclusive,
        } => format!(
            "{}{}{}",
            expression(start),
            if *inclusive { ".." } else { " until " },
            expression(end)
        ),
        Expr::Array(items) => format!(
            "listOf({})",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        Expr::Set(items) => format!(
            "setOf({})",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        Expr::Map(entries) => format!(
            "mapOf({})",
            entries
                .iter()
                .map(|(key, value)| format!("{} to {}", expression(key), expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Pair(first, second) => {
            format!("Pair({}, {})", expression(first), expression(second))
        }
        Expr::Triple(first, second, third) => format!(
            "Triple({}, {}, {})",
            expression(first),
            expression(second),
            expression(third)
        ),
        Expr::Call {
            name, arguments, ..
        } => format!(
            "{}({})",
            function_name(name),
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Await(value) => expression(value),
        Expr::Add(left, right, ty) => {
            let sum = format!("({} + {})", expression(left), expression(right));
            match ty {
                NumericType::Int8 => format!("{sum}.toByte()"),
                NumericType::Int16 => format!("{sum}.toShort()"),
                NumericType::UInt8 => format!("{sum}.toUByte()"),
                NumericType::UInt16 => format!("{sum}.toUShort()"),
                _ => sum,
            }
        }
        Expr::Binary { op, left, right } => format!(
            "({} {} {})",
            expression(left),
            binary_operator(*op),
            expression(right)
        ),
    }
}

fn binary_operator(operator: BinaryOp) -> &'static str {
    match operator {
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}

fn kotlin_number(raw: &str, ty: NumericType) -> String {
    match ty {
        NumericType::Int8 => {
            if raw.starts_with('-') {
                format!("({raw}).toByte()")
            } else {
                format!("{raw}.toByte()")
            }
        }
        NumericType::Int16 => {
            if raw.starts_with('-') {
                format!("({raw}).toShort()")
            } else {
                format!("{raw}.toShort()")
            }
        }
        NumericType::Int32 => raw.to_owned(),
        NumericType::Int64 => format!("{raw}L"),
        NumericType::UInt8 => format!("{raw}u.toUByte()"),
        NumericType::UInt16 => format!("{raw}u.toUShort()"),
        NumericType::UInt32 => format!("{raw}u"),
        NumericType::UInt64 => format!("{raw}uL"),
        NumericType::Float32 => {
            if raw.contains('.') {
                format!("{raw}f")
            } else {
                format!("{raw}.0f")
            }
        }
        NumericType::Float64 => {
            if raw.contains('.') {
                raw.to_owned()
            } else {
                format!("{raw}.0")
            }
        }
    }
}

pub(super) fn text_expression(expr: &Expr) -> String {
    match expr {
        Expr::String(_) | Expr::State(_, Type::String) | Expr::Interpolation(_) => expression(expr),
        _ => format!("{}.toString()", expression(expr)),
    }
}
