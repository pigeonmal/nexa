use nexa_codegen::names::state_name;
use nexa_ir::{Expr, NumericType, Type};

use super::utils::swift_string;

pub(super) fn expression(expr: &Expr) -> String {
    match expr {
        Expr::String(value) => swift_string(value),
        Expr::Bool(value) => value.to_string(),
        Expr::Number { raw, ty } => match ty {
            NumericType::Float32 => format!("Float({raw})"),
            NumericType::Float64 => format!("Double({raw})"),
            _ => raw.clone(),
        },
        Expr::State(name, _) => state_name(name),
        Expr::Array(items) => format!(
            "[{}]",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        Expr::Add(left, right, ty) => {
            let operator = if matches!(ty, NumericType::Float32 | NumericType::Float64) {
                "+"
            } else {
                "&+"
            };
            format!("({} {operator} {})", expression(left), expression(right))
        }
    }
}

pub(super) fn text_expression(expr: &Expr) -> String {
    match expr {
        Expr::State(_, Type::String) | Expr::String(_) => expression(expr),
        _ => format!("String(describing: {})", expression(expr)),
    }
}
