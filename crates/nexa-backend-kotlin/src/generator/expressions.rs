use nexa_codegen::names::state_name;
use nexa_ir::{BinaryOp, Expr, NumericType, Type};

use super::utils::kotlin_string;

pub(super) fn expression(expr: &Expr) -> String {
    match expr {
        Expr::String(value) => kotlin_string(value),
        Expr::Bool(value) => value.to_string(),
        Expr::Number { raw, ty } => kotlin_number(raw, *ty),
        Expr::State(name, _) => state_name(name),
        Expr::Not(value) => format!("(!{})", expression(value)),
        Expr::Array(items) => format!(
            "listOf({})",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
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
        Expr::String(_) | Expr::State(_, Type::String) => expression(expr),
        _ => format!("{}.toString()", expression(expr)),
    }
}
