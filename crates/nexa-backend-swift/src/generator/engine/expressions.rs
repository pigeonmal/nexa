use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{BinaryOp, CollectionTransform, Expr, InterpolatedPart, NumericType, Type};

use super::utils::{swift_string, swift_string_content};

pub(crate) fn expression(expr: &Expr) -> String {
    expression_with_locals(expr, &[])
}

fn expression_with_locals(expr: &Expr, locals: &[String]) -> String {
    let render = |value: &Expr| expression_with_locals(value, locals);
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
                        value.push_str(&render(value_expr));
                        value.push(')');
                    }
                }
            }
            value.push('"');
            value
        }
        Expr::Bool(value) => value.to_string(),
        Expr::IsRegularWidth => "(nexaHorizontalSizeClass == .regular)".to_owned(),
        Expr::IsCompactWidth => "(nexaHorizontalSizeClass == .compact)".to_owned(),
        Expr::IsRegularHeight => "(nexaVerticalSizeClass == .regular)".to_owned(),
        Expr::IsCompactHeight => "(nexaVerticalSizeClass == .compact)".to_owned(),
        Expr::Number { raw, ty } => match ty {
            NumericType::Float32 => format!("Float({raw})"),
            NumericType::Float64 => format!("Double({raw})"),
            _ => raw.clone(),
        },
        Expr::State(name, _) if locals.iter().any(|local| local == name) => name.clone(),
        Expr::State(name, _) => state_name(name),
        Expr::EnumValue {
            enum_name,
            case_name,
        } => format!(
            "{}.{}",
            nexa_codegen::names::enum_name(enum_name),
            case_name
        ),
        Expr::Not(value) => format!("(!{})", render(value)),
        Expr::Null(_) => "nil".to_owned(),
        Expr::Coalesce(left, right) => {
            format!("({} ?? {})", render(left), render(right))
        }
        Expr::Index {
            collection,
            index,
            optional,
            collection_type,
            ..
        } => {
            let index = render(index);
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
                format!("{}?[{index}]", render(collection))
            } else {
                format!("{}[{index}]", render(collection))
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
                range_bound(start, locals),
                if *inclusive { "through" } else { "to" },
                range_bound(end, locals),
                format!("Int({})", range_bound(step, locals))
            ),
            None => format!(
                "{}{}{}",
                range_bound(start, locals),
                if *inclusive { "..." } else { "..<" },
                range_bound(end, locals)
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
                        render(base),
                        if *optional { "?" } else { "" },
                        nexa_codegen::names::struct_field_name(field)
                    );
                }
                (Type::Plugin { .. }, field) => {
                    return format!(
                        "{}{}.{}",
                        render(base),
                        if *optional { "?" } else { "" },
                        field
                    );
                }
                (Type::NetworkResponse, "statusCode") => ".statusCode",
                (Type::NetworkResponse, "headers") => ".headers",
                (Type::NetworkResponse, "body") => ".text",
                _ => unreachable!("semantic analysis validates member access"),
            };
            if matches!(tuple_type, Type::NetworkResponse) && name == "statusCode" {
                return format!(
                    "Int32({}{}{})",
                    render(base),
                    if *optional { "?" } else { "" },
                    field
                );
            }
            format!(
                "{}{}{}",
                render(base),
                if *optional { "?" } else { "" },
                field
            )
        }
        Expr::Array(items) => format!(
            "[{}]",
            items.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Expr::Set(items) => format!(
            "[{}]",
            items.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Expr::Map(entries) if entries.is_empty() => "[:]".to_owned(),
        Expr::Map(entries) => format!(
            "Dictionary([{}], uniquingKeysWith: {{ _, newValue in newValue }})",
            entries
                .iter()
                .map(|(key, value)| format!("({}, {})", render(key), render(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Pair(first, second) => {
            format!("({}, {})", render(first), render(second))
        }
        Expr::Triple(first, second, third) => {
            format!("({}, {}, {})", render(first), render(second), render(third))
        }
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
                                render(argument)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => arguments.iter().map(render).collect::<Vec<_>>().join(", "),
                }
            } else {
                arguments.iter().map(render).collect::<Vec<_>>().join(", ")
            };
            format!("{callee}({arguments})")
        }
        Expr::CollectionTransform {
            operation,
            collection,
            initial,
            closure,
        } => {
            let collection = render(collection);
            let closure = render(closure);
            match operation {
                CollectionTransform::Map => format!("{collection}.map {closure}"),
                CollectionTransform::Filter => format!("{collection}.filter {closure}"),
                CollectionTransform::Reduce => format!(
                    "{collection}.reduce({}, {closure})",
                    initial
                        .as_deref()
                        .map(render)
                        .unwrap_or_else(|| "0".to_owned())
                ),
            }
        }
        Expr::Closure { parameters, body } => {
            let body = expression_with_locals(body, parameters);
            format!("{{ {} in {} }}", parameters.join(", "), body)
        }
        Expr::NativeCall {
            receiver,
            namespace,
            name,
            arguments,
            ..
        } => native_call(receiver.as_deref(), namespace, name, arguments, locals),
        Expr::Await(value) => match value.as_ref() {
            Expr::NativeCall {
                return_type,
                is_throwing,
                ..
            } if *is_throwing => format!(
                "(try? await {}) ?? {}",
                render(value),
                native_failure_default(return_type)
            ),
            Expr::NativeCall { .. } => format!("await {}", render(value)),
            _ => format!("await {}", render(value)),
        },
        Expr::Add(left, right, ty) => {
            let operator = if matches!(ty, NumericType::Float32 | NumericType::Float64) {
                "+"
            } else {
                "&+"
            };
            format!("({} {operator} {})", render(left), render(right))
        }
        Expr::Binary { op, left, right } => format!(
            "({} {} {})",
            render(left),
            binary_operator(*op),
            render(right)
        ),
        Expr::Contains {
            value,
            collection,
            collection_type,
        } => {
            let collection = render(collection);
            let receiver = if matches!(collection_type, Type::Map(_, _)) {
                format!("{}.keys", collection)
            } else {
                collection
            };
            format!("{}.contains({})", receiver, render(value))
        }
    }
}

fn native_call(
    receiver: Option<&Expr>,
    namespace: &str,
    name: &str,
    arguments: &[(String, Expr)],
    locals: &[String],
) -> String {
    if let Some(receiver) = receiver {
        return format!(
            "{}.{}({})",
            expression_with_locals(receiver, locals),
            name,
            arguments
                .iter()
                .map(|(argument_name, value)| {
                    format!(
                        "{}: {}",
                        argument_name,
                        expression_with_locals(value, locals)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let argument = |name: &str| {
        arguments
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| expression_with_locals(value, locals))
            .expect("native argument validated by semantic analysis")
    };
    let body = arguments
        .iter()
        .find(|(candidate, _)| candidate == "body")
        .map(|(_, value)| match value {
            Expr::Null(_) => "nil".to_owned(),
            _ if is_optional_expression(value) => {
                format!(
                    "{}.map {{ Data($0.utf8) }}",
                    expression_with_locals(value, locals)
                )
            }
            _ => format!("Data({}.utf8)", expression_with_locals(value, locals)),
        })
        .unwrap_or_else(|| "nil".to_owned());
    match (namespace, name) {
        ("Network", "fetch") => format!(
            "NexaNetwork.fetch(url: {}, method: {}, body: {}, headers: {}, timeout: {}, useCache: {}, followRedirects: {}, maxResponseBytes: Int({}), certificatePins: {})",
            argument("url"),
            argument("method"),
            body,
            argument("headers"),
            argument("timeout"),
            argument("useCache"),
            argument("followRedirects"),
            argument("maxResponseBytes"),
            argument("certificatePins"),
        ),
        ("Network", "download") => format!(
            "NexaNetwork.download(url: {}, destinationPath: {}, method: {}, body: {}, headers: {}, timeout: {}, useCache: {}, followRedirects: {}, maxResponseBytes: Int({}), certificatePins: {})",
            argument("url"),
            argument("destinationPath"),
            argument("method"),
            body,
            argument("headers"),
            argument("timeout"),
            argument("useCache"),
            argument("followRedirects"),
            argument("maxResponseBytes"),
            argument("certificatePins"),
        ),
        ("Path", path_name) => {
            if path_name == "join" {
                format!(
                    "NexaPath.join({}, {})",
                    argument("path"),
                    argument("component")
                )
            } else {
                format!("NexaPath.{}()", path_name)
            }
        }
        ("File", "exists") => format!("NexaFile.exists({})", argument("path")),
        ("File", "readText") => format!("NexaFile.readText({})", argument("path")),
        ("File", "writeText") => format!(
            "NexaFile.writeText({}, to: {})",
            argument("contents"),
            argument("path")
        ),
        ("File", "delete") => format!("NexaFile.delete({})", argument("path")),
        ("Permissions", "status") => {
            format!("NexaPermissions.status({})", argument("permission"))
        }
        ("Permissions", "request") => {
            format!("NexaPermissions.request({})", argument("permission"))
        }
        _ => format!(
            "{}Plugin.shared.{}({})",
            namespace,
            name,
            arguments
                .iter()
                .map(|(argument_name, value)| {
                    format!("{argument_name}: {}", expression_with_locals(value, locals))
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn is_optional_expression(expr: &Expr) -> bool {
    match expr {
        Expr::State(_, Type::Optional(_))
        | Expr::Member {
            field_type: Type::Optional(_),
            ..
        }
        | Expr::Index {
            element_type: Type::Optional(_),
            ..
        }
        | Expr::Null(_) => true,
        _ => false,
    }
}

fn native_failure_default(ty: &Type) -> String {
    match ty {
        Type::NetworkResponse => {
            "NexaNetworkResponse(statusCode: -1, headers: [:], body: Data())".to_owned()
        }
        Type::String => "\"\"".to_owned(),
        Type::Bool => "false".to_owned(),
        _ => "fatalError(\"native operation failed\")".to_owned(),
    }
}

fn range_bound(expr: &Expr, locals: &[String]) -> String {
    match expr {
        Expr::Number {
            raw,
            ty: NumericType::Int32,
        } => format!("Int32({raw})"),
        _ => expression_with_locals(expr, locals),
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

pub(crate) fn text_expression(expr: &Expr) -> String {
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
