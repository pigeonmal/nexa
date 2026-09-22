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
        Expr::EnumValue {
            enum_name,
            case_name,
        } => format!(
            "{}.{}",
            nexa_codegen::names::enum_name(enum_name),
            case_name
        ),
        Expr::Not(value) => format!("(!{})", expression(value)),
        Expr::Null(_) => "null".to_owned(),
        Expr::Coalesce(left, right) => {
            format!("({} ?: {})", expression(left), expression(right))
        }
        Expr::Index {
            collection,
            index,
            optional,
            ..
        } => {
            if *optional {
                format!("{}?.get({})", expression(collection), expression(index))
            } else {
                format!("{}[{}]", expression(collection), expression(index))
            }
        }
        Expr::Range {
            start,
            end,
            inclusive,
            step,
        } => {
            let range = format!(
                "{}{}{}",
                expression(start),
                if *inclusive { ".." } else { " until " },
                expression(end)
            );
            match step {
                Some(step) => format!("({range} step {})", expression(step)),
                None => range,
            }
        }
        Expr::Member {
            base,
            name,
            optional,
            base_type,
            ..
        } => {
            let member_name = match base_type {
                Type::Optional(inner) => inner.as_ref(),
                base_type => base_type,
            };
            let name = if matches!(member_name, Type::Struct { .. }) {
                nexa_codegen::names::struct_field_name(name)
            } else if matches!(member_name, Type::NetworkResponse) && name == "body" {
                "text".to_owned()
            } else {
                name.clone()
            };
            format!(
                "{}{}{}",
                expression(base),
                if *optional { "?." } else { "." },
                name
            )
        }
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
            name,
            arguments,
            return_type,
            is_constructor,
            ..
        } => {
            let callee = if *is_constructor {
                return_type.kotlin()
            } else {
                function_name(name)
            };
            format!(
                "{callee}({})",
                arguments
                    .iter()
                    .map(expression)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Expr::NativeCall {
            namespace,
            name,
            arguments,
            ..
        } => native_call(namespace, name, arguments),
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
        Expr::Contains {
            value, collection, ..
        } => format!("({} in {})", expression(value), expression(collection)),
    }
}

fn native_call(namespace: &str, name: &str, arguments: &[(String, Expr)]) -> String {
    let argument = |name: &str| {
        arguments
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| expression(value))
            .expect("native argument validated by semantic analysis")
    };
    let body = match arguments
        .iter()
        .find(|(candidate, _)| candidate == "body")
        .map(|(_, value)| value)
    {
        Some(Expr::Null(_)) => "null".to_owned(),
        Some(value) if is_optional_expression(value) => {
            format!("{}?.toByteArray()", expression(value))
        }
        Some(value) => format!("{}.toByteArray()", expression(value)),
        None => "null".to_owned(),
    };
    match (namespace, name) {
        ("Network", "fetch") => format!(
            "NexaNetwork.fetch(NexaRuntime.context(), {}, {}, {}, {}, ({} * 1000.0).toLong(), {}, {}, {}, {})",
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
            "NexaNetwork.download(NexaRuntime.context(), {}, {}, {}, {}, {}, ({} * 1000.0).toLong(), {}, {}, {}, {})",
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
                format!("NexaPath.{}(NexaRuntime.context())", path_name)
            }
        }
        ("File", "exists") => format!("NexaFile.exists({})", argument("path")),
        ("File", "readText") => format!("NexaFile.readText({})", argument("path")),
        ("File", "writeText") => format!(
            "NexaFile.writeText({}, {})",
            argument("contents"),
            argument("path")
        ),
        ("File", "delete") => format!("NexaFile.delete({})", argument("path")),
        ("Permissions", "status") => format!(
            "NexaPermissions.status(NexaRuntime.context(), {})",
            argument("permission")
        ),
        ("Permissions", "request") => format!(
            "NexaPermissions.request(NexaRuntime.context(), {})",
            argument("permission")
        ),
        _ => format!(
            "{}Plugin.instance.{}({})",
            namespace,
            name,
            arguments
                .iter()
                .map(|(_, value)| expression(value))
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

fn binary_operator(operator: BinaryOp) -> &'static str {
    match operator {
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Contains => "in",
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
        Expr::String(_)
        | Expr::State(_, Type::String)
        | Expr::Member {
            field_type: Type::String,
            ..
        }
        | Expr::Interpolation(_) => expression(expr),
        _ => format!("{}.toString()", expression(expr)),
    }
}
