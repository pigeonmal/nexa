use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{BinaryOp, Expr, InterpolatedPart, NumericType, Type};

use super::utils::{swift_string, swift_string_content};

pub(super) fn expression(expr: &Expr) -> String {
    match expr {
        Expr::String(value) => swift_string(value),
        Expr::Interpolation(parts) => {
            let mut value = String::from("\"");
            for part in parts {
                match part {
                    InterpolatedPart::Literal(literal) => {
                        value.push_str(&swift_string_content(literal));
                    }
                    InterpolatedPart::Value(value_expr) => {
                        value.push_str("\\(");
                        value.push_str(&expression(value_expr));
                        value.push(')');
                    }
                }
            }
            value.push('"');
            value
        }
        Expr::Bool(value) => value.to_string(),
        Expr::IsRegularWidth => "(nexaHorizontalSizeClass == .regular)".to_owned(),
        Expr::Number { raw, ty } => match ty {
            NumericType::Float32 => format!("Float({raw})"),
            NumericType::Float64 => format!("Double({raw})"),
            _ => raw.clone(),
        },
        Expr::State(name, _) => state_name(name),
        Expr::Not(value) => format!("(!{})", expression(value)),
        Expr::Array(items) => format!(
            "[{}]",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        Expr::Set(items) => format!(
            "[{}]",
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        Expr::Map(entries) if entries.is_empty() => "[:]".to_owned(),
        Expr::Map(entries) => format!(
            "Dictionary([{}], uniquingKeysWith: {{ _, newValue in newValue }})",
            entries
                .iter()
                .map(|(key, value)| format!("({}, {})", expression(key), expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Pair(first, second) => {
            format!("({}, {})", expression(first), expression(second))
        }
        Expr::Triple(first, second, third) => format!(
            "({}, {}, {})",
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
        Expr::Await(value) => format!("await {}", expression(value)),
        Expr::Add(left, right, ty) => {
            let operator = if matches!(ty, NumericType::Float32 | NumericType::Float64) {
                "+"
            } else {
                "&+"
            };
            format!("({} {operator} {})", expression(left), expression(right))
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

pub(super) fn text_expression(expr: &Expr) -> String {
    match expr {
        Expr::State(_, Type::String) | Expr::String(_) | Expr::Interpolation(_) => expression(expr),
        _ => format!("String(describing: {})", expression(expr)),
    }
}
