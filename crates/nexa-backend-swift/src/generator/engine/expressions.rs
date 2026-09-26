use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{
    BinaryOp, CollectionTransform, Expr, InterpolatedPart, MemberKind, NetworkRequest, NumericType,
    PermissionOpKind, TuplePosition, Type,
};

use super::utils::{swift_string, swift_string_content};
use crate::generator::engine::types::swift_type;

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
                "stride(from: {}, {}: {}, by: Int({}))",
                range_bound(start, locals),
                if *inclusive { "through" } else { "to" },
                range_bound(end, locals),
                range_bound(step, locals)
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
            optional,
            kind,
            ..
        } => {
            let (field, wrap_status_code) = match kind {
                MemberKind::TupleIndex(TuplePosition::First) => (".0".to_owned(), false),
                MemberKind::TupleIndex(TuplePosition::Second) => (".1".to_owned(), false),
                MemberKind::TupleIndex(TuplePosition::Third) => (".2".to_owned(), false),
                MemberKind::StructField(field) => (
                    format!(".{}", nexa_codegen::names::struct_field_name(field)),
                    false,
                ),
                MemberKind::PluginField(field) => (format!(".{field}"), false),
                MemberKind::NetworkStatusCode => (".statusCode".to_owned(), true),
                MemberKind::NetworkHeaders => (".headers".to_owned(), false),
                MemberKind::NetworkBody => (".text".to_owned(), false),
            };
            let access = format!(
                "{}{}{}",
                render(base),
                if *optional { "?" } else { "" },
                field
            );
            if wrap_status_code {
                format!("Int32({access})")
            } else {
                access
            }
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
                swift_type(return_type)
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
        Expr::NetworkFetch(request) => render_network_request(request, None, locals),
        Expr::NetworkDownload {
            destination,
            request,
        } => render_network_request(request, Some(destination), locals),
        Expr::PathJoin { path, component } => {
            format!("NexaPath.join({}, {})", render(path), render(component))
        }
        Expr::FileExists { path } => format!("NexaFile.exists({})", render(path)),
        Expr::FileReadText { path } => format!("NexaFile.readText({})", render(path)),
        Expr::FileWriteText { path, contents } => format!(
            "NexaFile.writeText({}, to: {})",
            render(contents),
            render(path)
        ),
        Expr::FileDelete { path } => format!("NexaFile.delete({})", render(path)),
        Expr::PermissionOp { op, permission } => {
            let method = match op {
                PermissionOpKind::Status => "status",
                PermissionOpKind::Request => "request",
            };
            format!("NexaPermissions.{method}({})", render(permission))
        }
        Expr::Await(value) => {
            if value.is_throwing_call() {
                format!("try await {}", render(value))
            } else {
                format!("await {}", render(value))
            }
        }
        Expr::TryAwait(value) => format!("try await {}", render(value)),
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
        Expr::ResultOk { value, .. } => format!(".success({})", render(value)),
        Expr::ResultErr { error, .. } => format!(".failure({})", render(error)),
        Expr::Try { expr, .. } => format!("try {}.get()", render(expr)),
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
    match (namespace, name) {
        ("Path", path_name) => {
            format!("NexaPath.{path_name}()")
        }
        _ => format!(
            "{}Plugin.shared.{}({})",
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

/// Renders a validated network request. `destination` is present only for
/// `Network.download`; the format strings match the previous
/// argument-lookup rendering exactly.
fn render_network_request(
    request: &NetworkRequest,
    destination: Option<&Expr>,
    locals: &[String],
) -> String {
    let render = |value: &Expr| expression_with_locals(value, locals);
    let body = match request.body.as_deref() {
        None => "nil".to_owned(),
        Some(body) if is_optional_expression(body) => {
            format!("{}.map {{ Data($0.utf8) }}", render(body))
        }
        Some(body) => format!("Data({}.utf8)", render(body)),
    };
    let url = render(&request.url);
    let method = render(&request.method);
    let headers = render(&request.headers);
    let timeout = render(&request.timeout);
    let use_cache = render(&request.use_cache);
    let follow_redirects = render(&request.follow_redirects);
    let max_response_bytes = render(&request.max_response_bytes);
    let certificate_pins = render(&request.certificate_pins);
    match destination {
        Some(destination) => format!(
            "NexaNetwork.download(url: {url}, destinationPath: {}, method: {method}, body: {body}, headers: {headers}, timeout: {timeout}, useCache: {use_cache}, followRedirects: {follow_redirects}, maxResponseBytes: Int({max_response_bytes}), certificatePins: {certificate_pins})",
            render(destination),
        ),
        None => format!(
            "NexaNetwork.fetch(url: {url}, method: {method}, body: {body}, headers: {headers}, timeout: {timeout}, useCache: {use_cache}, followRedirects: {follow_redirects}, maxResponseBytes: Int({max_response_bytes}), certificatePins: {certificate_pins})"
        ),
    }
}

fn is_optional_expression(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::State(_, Type::Optional(_))
            | Expr::Member {
                field_type: Type::Optional(_),
                ..
            }
            | Expr::Index {
                element_type: Type::Optional(_),
                ..
            }
            | Expr::Null(_)
    )
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

#[cfg(test)]
mod tests {
    use super::expression;
    use nexa_ir::{Expr, Type};

    #[test]
    fn throwing_native_calls_preserve_errors_for_explicit_recovery() {
        let value = Expr::Await(Box::new(Expr::NativeCall {
            receiver: None,
            namespace: "Camera".to_owned(),
            name: "stop".to_owned(),
            arguments: Vec::new(),
            return_type: Type::Void,
            is_async: true,
            is_throwing: true,
        }));

        assert_eq!(expression(&value), "try await CameraPlugin.shared.stop()");
    }

    #[test]
    fn file_calls_use_the_generated_native_helper_names_and_labels() {
        let path = || Box::new(Expr::String("notes.txt".to_owned()));
        let read = Expr::Await(Box::new(Expr::FileReadText { path: path() }));
        let write = Expr::Await(Box::new(Expr::FileWriteText {
            path: path(),
            contents: Box::new(Expr::String("saved".to_owned())),
        }));
        let delete = Expr::Await(Box::new(Expr::FileDelete { path: path() }));
        let exists = Expr::Await(Box::new(Expr::FileExists { path: path() }));

        assert_eq!(
            expression(&read),
            "try await NexaFile.readText(\"notes.txt\")"
        );
        assert_eq!(
            expression(&write),
            "try await NexaFile.writeText(\"saved\", to: \"notes.txt\")"
        );
        assert_eq!(
            expression(&delete),
            "try await NexaFile.delete(\"notes.txt\")"
        );
        assert_eq!(expression(&exists), "await NexaFile.exists(\"notes.txt\")");
    }

    #[test]
    fn native_instance_calls_keep_the_receiver_and_use_positional_arguments() {
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
            "await nexa_player.prepare(\"clip.mp4\")"
        );
    }
}
