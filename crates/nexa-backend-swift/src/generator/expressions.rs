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
        Expr::EnumValue {
            enum_name,
            case_name,
        } => format!(
            "{}.{}",
            nexa_codegen::names::enum_name(enum_name),
            case_name
        ),
        Expr::Not(value) => format!("(!{})", expression(value)),
        Expr::Null(_) => "nil".to_owned(),
        Expr::Coalesce(left, right) => {
            format!("({} ?? {})", expression(left), expression(right))
        }
        Expr::Index {
            collection,
            index,
            optional,
            collection_type,
            ..
        } => {
            let index = expression(index);
            let is_array = match collection_type {
                Type::Array(_) => true,
                Type::Optional(inner) => matches!(inner.as_ref(), Type::Array(_)),
                _ => false,
            };
            let index = if is_array {
                format!("Int({index})")
            } else {
                index
            };
            if *optional {
                format!("{}?[{index}]", expression(collection))
            } else {
                format!("{}[{index}]", expression(collection))
            }
        }
        Expr::Range {
            start,
            end,
            inclusive,
            step,
        } => match step {
            Some(step) => format!(
                "stride(from: {}, {}: {}, by: {})",
                range_bound(start),
                if *inclusive { "through" } else { "to" },
                range_bound(end),
                range_bound(step)
            ),
            None => format!(
                "{}{}{}",
                range_bound(start),
                if *inclusive { "..." } else { "..<" },
                range_bound(end)
            ),
        },
        Expr::Member {
            base,
            name,
            optional,
            base_type,
            ..
        } => {
            let tuple_type = match base_type {
                Type::Optional(inner) => inner.as_ref(),
                base_type => base_type,
            };
            let field = match (tuple_type, name.as_str()) {
                (Type::Pair(_, _), "first") | (Type::Triple(_, _, _), "first") => ".0",
                (Type::Pair(_, _), "second") | (Type::Triple(_, _, _), "second") => ".1",
                (Type::Triple(_, _, _), "third") => ".2",
                (Type::Struct { .. }, field) => {
                    return format!(
                        "{}{}.{}",
                        expression(base),
                        if *optional { "?" } else { "" },
                        nexa_codegen::names::struct_field_name(field)
                    );
                }
                _ => unreachable!("semantic analysis validates member access"),
            };
            format!(
                "{}{}{}",
                expression(base),
                if *optional { "?" } else { "" },
                field
            )
        }
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
            name,
            arguments,
            return_type,
            is_constructor,
            ..
        } => {
            let callee = if *is_constructor {
                return_type.swift()
            } else {
                function_name(name)
            };
            let arguments = if *is_constructor {
                match return_type {
                    Type::Struct { fields, .. } => arguments
                        .iter()
                        .zip(fields)
                        .map(|(argument, (field, _))| {
                            format!(
                                "{}: {}",
                                nexa_codegen::names::struct_field_name(field),
                                expression(argument)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => arguments
                        .iter()
                        .map(expression)
                        .collect::<Vec<_>>()
                        .join(", "),
                }
            } else {
                arguments
                    .iter()
                    .map(expression)
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!("{callee}({arguments})")
        }
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
        Expr::Contains {
            value,
            collection,
            collection_type,
        } => {
            let collection = expression(collection);
            let receiver = if matches!(collection_type, Type::Map(_, _)) {
                format!("{}.keys", collection)
            } else {
                collection
            };
            format!("{}.contains({})", receiver, expression(value))
        }
    }
}

fn range_bound(expr: &Expr) -> String {
    match expr {
        Expr::Number {
            raw,
            ty: NumericType::Int32,
        } => format!("Int32({raw})"),
        _ => expression(expr),
    }
}

fn binary_operator(operator: BinaryOp) -> &'static str {
    match operator {
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Contains => "contains",
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
        Expr::State(_, Type::String)
        | Expr::Member {
            field_type: Type::String,
            ..
        }
        | Expr::String(_)
        | Expr::Interpolation(_) => expression(expr),
        _ => format!("String(describing: {})", expression(expr)),
    }
}
