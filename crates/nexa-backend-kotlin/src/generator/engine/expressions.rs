use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{BinaryOp, CollectionTransform, Expr, InterpolatedPart, NumericType, Type};

use super::utils::{kotlin_string, kotlin_string_content};
use super::{features::Features, imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_size_class,
        "androidx.compose.ui.platform.LocalConfiguration",
    );
}

pub(crate) fn expression(expr: &Expr) -> String {
    expression_with_locals(expr, &[])
}

fn expression_with_locals(expr: &Expr, locals: &[String]) -> String {
    let render = |value: &Expr| expression_with_locals(value, locals);
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
                        value.push_str(&render(value_expr));
                        value.push('}');
                    }
                }
            }
            value.push('"');
            value
        }
        Expr::Bool(value) => value.to_string(),
        Expr::IsRegularWidth => "(LocalConfiguration.current.screenWidthDp >= 600)".to_owned(),
        Expr::IsCompactWidth => "(LocalConfiguration.current.screenWidthDp < 600)".to_owned(),
        Expr::IsRegularHeight => "(LocalConfiguration.current.screenHeightDp >= 600)".to_owned(),
        Expr::IsCompactHeight => "(LocalConfiguration.current.screenHeightDp < 600)".to_owned(),
        Expr::Number { raw, ty } => kotlin_number(raw, *ty),
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
        Expr::Null(_) => "null".to_owned(),
        Expr::Coalesce(left, right) => {
            format!("({} ?: {})", render(left), render(right))
        }
        Expr::Index {
            collection,
            index,
            optional,
            ..
        } => {
            if *optional {
                format!("{}?.get({})", render(collection), render(index))
            } else {
                format!("{}[{}]", render(collection), render(index))
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
                render(start),
                if *inclusive { ".." } else { " until " },
                render(end)
            );
            match step {
                Some(step) => format!("({range} step {})", render(step)),
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
            } else if matches!(member_name, Type::Plugin { .. }) {
                name.clone()
            } else if matches!(member_name, Type::NetworkResponse) && name == "body" {
                "text".to_owned()
            } else {
                name.clone()
            };
            format!(
                "{}{}{}",
                render(base),
                if *optional { "?." } else { "." },
                name
            )
        }
        Expr::Array(items) => format!(
            "listOf({})",
            items.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Expr::Set(items) => format!(
            "setOf({})",
            items.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Expr::Map(entries) => format!(
            "mapOf({})",
            entries
                .iter()
                .map(|(key, value)| format!("{} to {}", render(key), render(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Pair(first, second) => {
            format!("Pair({}, {})", render(first), render(second))
        }
        Expr::Triple(first, second, third) => format!(
            "Triple({}, {}, {})",
            render(first),
            render(second),
            render(third)
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
                arguments.iter().map(render).collect::<Vec<_>>().join(", ")
            )
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
                    "{collection}.fold({}, {closure})",
                    initial
                        .as_deref()
                        .map(render)
                        .unwrap_or_else(|| "0".to_owned())
                ),
            }
        }
        Expr::Closure { parameters, body } => {
            let body = expression_with_locals(body, parameters);
            format!("{{ {} -> {} }}", parameters.join(", "), body)
        }
        Expr::NativeCall {
            receiver,
            namespace,
            name,
            arguments,
            ..
        } => native_call(receiver.as_deref(), namespace, name, arguments, locals),
        Expr::Await(value) | Expr::TryAwait(value) => render(value),
        Expr::Add(left, right, ty) => {
            let sum = format!("({} + {})", render(left), render(right));
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
            render(left),
            binary_operator(*op),
            render(right)
        ),
        Expr::Contains {
            value, collection, ..
        } => format!("({} in {})", render(value), render(collection)),
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
                .map(|(_, value)| expression_with_locals(value, locals))
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
    let body = match arguments
        .iter()
        .find(|(candidate, _)| candidate == "body")
        .map(|(_, value)| value)
    {
        Some(Expr::Null(_)) => "null".to_owned(),
        Some(value) if is_optional_expression(value) => {
            format!("{}?.toByteArray()", expression_with_locals(value, locals))
        }
        Some(value) => format!("{}.toByteArray()", expression_with_locals(value, locals)),
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
                .map(|(_, value)| expression_with_locals(value, locals))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::expression;
    use nexa_ir::{Expr, Type};

    #[test]
    fn file_calls_use_the_generated_native_helper_names_and_argument_order() {
        let read = Expr::Await(Box::new(Expr::NativeCall {
            receiver: None,
            namespace: "File".to_owned(),
            name: "readText".to_owned(),
            arguments: vec![("path".to_owned(), Expr::String("notes.txt".to_owned()))],
            return_type: Type::String,
            is_async: true,
            is_throwing: false,
        }));
        let write = Expr::Await(Box::new(Expr::NativeCall {
            receiver: None,
            namespace: "File".to_owned(),
            name: "writeText".to_owned(),
            arguments: vec![
                ("contents".to_owned(), Expr::String("saved".to_owned())),
                ("path".to_owned(), Expr::String("notes.txt".to_owned())),
            ],
            return_type: Type::Bool,
            is_async: true,
            is_throwing: false,
        }));
        let delete = Expr::Await(Box::new(Expr::NativeCall {
            receiver: None,
            namespace: "File".to_owned(),
            name: "delete".to_owned(),
            arguments: vec![("path".to_owned(), Expr::String("notes.txt".to_owned()))],
            return_type: Type::Bool,
            is_async: true,
            is_throwing: false,
        }));

        assert_eq!(expression(&read), "NexaFile.readText(\"notes.txt\")");
        assert_eq!(
            expression(&write),
            "NexaFile.writeText(\"saved\", \"notes.txt\")"
        );
        assert_eq!(expression(&delete), "NexaFile.delete(\"notes.txt\")");
    }

    #[test]
    fn native_instance_calls_stay_direct_and_use_kotlin_positional_arguments() {
        let call = Expr::NativeCall {
            receiver: Some(Box::new(Expr::State(
                "player".to_owned(),
                Type::Plugin {
                    namespace: "Video".to_owned(),
                    name: "VideoPlayer".to_owned(),
                },
            ))),
            namespace: "Video".to_owned(),
            name: "prepare".to_owned(),
            arguments: vec![("url".to_owned(), Expr::String("clip.mp4".to_owned()))],
            return_type: Type::Void,
            is_async: true,
            is_throwing: false,
        };

        assert_eq!(
            expression(&Expr::Await(Box::new(call))),
            "nexa_player.prepare(\"clip.mp4\")"
        );
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

pub(crate) fn text_expression(expr: &Expr) -> String {
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
