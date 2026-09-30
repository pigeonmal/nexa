use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{
    ArithmeticOp, BinaryOp, CollectionTransform, CollectionUtilityKind, Expr, InterpolatedPart,
    LogMethod, MemberKind, NetworkRequest, NumericType, PermissionOpKind, PluginCodec, TimeMethod,
    TuplePosition, Type,
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
                MemberKind::CollectionCount => (".count".to_owned(), false),
                MemberKind::CollectionIsEmpty => (".isEmpty".to_owned(), false),
            };
            let access = format!(
                "{}{}{}",
                render(base),
                if *optional { "?" } else { "" },
                field
            );
            if matches!(kind, MemberKind::CollectionCount) {
                if *optional {
                    format!("{}.map {{ Int32($0.count) }}", render(base))
                } else {
                    format!("Int32({access})")
                }
            } else if wrap_status_code {
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
        Expr::CollectionUtility {
            operation,
            collection,
            start,
            end,
            inclusive,
            ..
        } => {
            let collection = render(collection);
            match operation {
                CollectionUtilityKind::Random => format!("{collection}.randomElement()"),
                CollectionUtilityKind::Shuffled => format!("{collection}.shuffled()"),
                CollectionUtilityKind::Reverse => format!("Array({collection}.reversed())"),
                CollectionUtilityKind::Slice => {
                    let start = start
                        .as_deref()
                        .map(render)
                        .unwrap_or_else(|| "0".to_owned());
                    let end = end.as_deref().map(render).unwrap_or_else(|| "0".to_owned());
                    let range = if *inclusive {
                        format!("Int({start})...Int({end})")
                    } else {
                        format!("Int({start})..<Int({end})")
                    };
                    format!("Array({collection}[{range}])")
                }
            }
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => format!(
            "({} ? {} : {})",
            render(condition),
            render(then_value),
            render(else_value)
        ),
        Expr::TimeCall {
            method, arguments, ..
        } => time_call(*method, arguments, locals),
        Expr::LogCall { method, message } => {
            let severity = match method {
                LogMethod::Info => "INFO",
                LogMethod::Warning => "WARNING",
                LogMethod::Error => "ERROR",
            };
            format!("NSLog(\"[Nexa][{severity}] %@\", {})", render(message))
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
            codecs,
            ..
        } => native_call(
            receiver.as_deref(),
            namespace,
            name,
            arguments,
            codecs,
            locals,
        ),
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
        // `Data` conversions stay in the standard library: no generated
        // runtime support, and no intermediate array for `fromText`.
        Expr::BytesFromText { text } => format!("Data({}.utf8)", render(text)),
        Expr::BytesFromArray { values } => format!("Data({})", render(values)),
        Expr::BytesCount { bytes } => format!("Int32({}.count)", render(bytes)),
        Expr::PermissionOp { op, permission } => {
            let method = match op {
                PermissionOpKind::Status => "status",
                PermissionOpKind::Request => "request",
            };
            format!("NexaPermissions.{method}({})", render(permission))
        }
        Expr::Await(value) => {
            if value.is_throwing_call() {
                format!("(try await {})", render(value))
            } else {
                format!("(await {})", render(value))
            }
        }
        Expr::TryAwait(value) => format!("(try await {})", render(value)),
        Expr::Add(left, right, ty) => {
            let operator = if matches!(ty, NumericType::Float32 | NumericType::Float64) {
                "+"
            } else {
                "&+"
            };
            format!("({} {operator} {})", render(left), render(right))
        }
        Expr::Concat(left, right) => {
            format!("({} + {})", render(left), render(right))
        }
        Expr::Arithmetic {
            op,
            left,
            right,
            ty,
        } => {
            let floating = matches!(ty, NumericType::Float32 | NumericType::Float64);
            let operator = match op {
                ArithmeticOp::Subtract if !floating => "&-",
                ArithmeticOp::Subtract => "-",
                ArithmeticOp::Multiply if !floating => "&*",
                ArithmeticOp::Multiply => "*",
                ArithmeticOp::Divide => "/",
                ArithmeticOp::Remainder => "%",
            };
            format!("({} {operator} {})", render(left), render(right))
        }
        Expr::Negate { value, ty } => {
            let operator = if matches!(ty, NumericType::Float32 | NumericType::Float64) {
                "-"
            } else {
                "0 &- "
            };
            if operator == "-" {
                format!("(-{})", render(value))
            } else {
                format!("({operator}{})", render(value))
            }
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

/// A core clock call. Every one is a direct platform call: a system clock, a
/// monotonic counter, or the generated ISO 8601 helpers.
fn time_call(method: TimeMethod, arguments: &[Expr], locals: &[String]) -> String {
    let rendered = arguments
        .iter()
        .map(|argument| expression_with_locals(argument, locals))
        .collect::<Vec<_>>();
    match method {
        TimeMethod::Now => "Int64((Date().timeIntervalSince1970 * 1000).rounded())".to_owned(),
        // `uptimeNanoseconds` is a `UInt64` that only overflows after ~584
        // years of uptime, so the reinterpretation is the direct spelling.
        TimeMethod::Monotonic => {
            "Int64(bitPattern: DispatchTime.now().uptimeNanoseconds)".to_owned()
        }
        TimeMethod::Elapsed => format!(
            "(Int64(bitPattern: DispatchTime.now().uptimeNanoseconds) - {})",
            rendered.first().cloned().unwrap_or_else(|| "0".to_owned())
        ),
        // The surrounding `Await` writes `try await`; a sleep is the one method
        // that needs it, because cancelling the task throws out of `Task.sleep`.
        TimeMethod::Sleep => {
            let milliseconds = rendered.first().cloned().unwrap_or_else(|| "0".to_owned());
            format!("Task.sleep(nanoseconds: {milliseconds} &* 1_000_000)")
        }
        TimeMethod::Iso8601 => {
            format!(
                "nexaIso8601({})",
                rendered.first().cloned().unwrap_or_default()
            )
        }
        TimeMethod::Iso8601ToMillis => {
            format!(
                "nexaIso8601ToMillis({})",
                rendered.first().cloned().unwrap_or_default()
            )
        }
    }
}

fn native_call(
    receiver: Option<&Expr>,
    namespace: &str,
    name: &str,
    arguments: &[(String, Expr)],
    codecs: &[PluginCodec],
    locals: &[String],
) -> String {
    let mut rendered = arguments
        .iter()
        .map(|(_, value)| expression_with_locals(value, locals))
        .collect::<Vec<_>>();
    if namespace == "Json" {
        let raw_or_value = rendered.first().map(String::as_str).unwrap_or("\"\"");
        if let Some(codec) = codecs.first() {
            let codec_name = nexa_codegen::value::json_codec_name(&codec.ty);
            return match name {
                "parse" => format!("nexaJsonParse({raw_or_value}, using: {codec_name}.self)"),
                "stringify" => {
                    format!("nexaJsonStringify({raw_or_value}, using: {codec_name}.self)")
                }
                _ => format!("nexaInvalidJsonCall_{name}()"),
            };
        }
    }
    // A generic plugin call carries its value codecs as trailing closures, so
    // the bound type never appears as a string in the generated code.
    for codec in codecs {
        let function = nexa_codegen::value::codec_name(
            &codec.ty,
            if codec.decodes {
                nexa_codegen::value::Direction::Read
            } else {
                nexa_codegen::value::Direction::Write
            },
        );
        rendered.push(if codec.decodes {
            format!("{{ reader in {function}(reader) }}")
        } else {
            format!("{{ item, writer in {function}(item, into: writer) }}")
        });
    }
    if let Some(receiver) = receiver {
        return format!(
            "{}.{}({})",
            expression_with_locals(receiver, locals),
            name,
            rendered.join(", ")
        );
    }
    match (namespace, name) {
        ("Screen", "lockOrientation") => format!(
            "NexaScreen.lockOrientation({})",
            rendered.first().map(String::as_str).unwrap_or("\"All\"")
        ),
        ("Network", "isOnline") => "NexaNetwork.isOnline()".to_owned(),
        ("Network", "upload") => format!(
            "NexaNetwork.upload(url: {}, file: {}, fields: {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\""),
            rendered.get(2).map(String::as_str).unwrap_or("[:]"),
        ),
        ("Haptics", "impact") => {
            let style = arguments
                .iter()
                .find(|(argument, _)| argument == "style")
                .and_then(|(_, value)| match value {
                    Expr::String(style) => Some(style.as_str()),
                    _ => None,
                });
            let style = match style {
                Some("Medium") => "medium",
                Some("Heavy") => "heavy",
                _ => "light",
            };
            format!("UIImpactFeedbackGenerator(style: .{style}).impactOccurred()")
        }
        ("Haptics", "notification") => {
            let kind = arguments
                .iter()
                .find(|(argument, _)| argument == "kind")
                .and_then(|(_, value)| match value {
                    Expr::String(kind) => Some(kind.as_str()),
                    _ => None,
                });
            let kind = if kind == Some("Error") { "error" } else { "success" };
            format!("UINotificationFeedbackGenerator().notificationOccurred(.{kind})")
        }
        ("Haptics", "selection") => "UISelectionFeedbackGenerator().selectionChanged()".to_owned(),
        ("Number", "formatCurrency") => format!(
            "nexaFormatCurrency({}, {})",
            rendered.first().map(String::as_str).unwrap_or("0.0"),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("Crypto", "sha256") => format!(
            "nexaCryptoSha256({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Crypto", "sha512") => format!(
            "nexaCryptoSha512({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Crypto", "hmacSha256") => format!(
            "nexaCryptoHmacSha256({}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("Crypto", "randomBytes") => format!(
            "nexaCryptoRandomBytes({})",
            rendered.first().map(String::as_str).unwrap_or("0")
        ),
        ("SecureStorage", "get") => format!(
            "NexaSecureStorage.get({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "set") => format!(
            "NexaSecureStorage.set({}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "delete") => format!(
            "NexaSecureStorage.delete({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "clear") => "NexaSecureStorage.clear()".to_owned(),
        ("Storage", "getString") => format!(
            "NexaStorage.getString({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "setString") => format!(
            "NexaStorage.setString({}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "delete") => format!(
            "NexaStorage.delete({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "clear") => "NexaStorage.clear()".to_owned(),
        ("Clipboard", "setText") => format!(
            "NexaClipboard.setText({})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Clipboard", "getText") => "NexaClipboard.getText()".to_owned(),
        ("Clipboard", "hasText") => "NexaClipboard.hasText()".to_owned(),
        ("Path", path_name) => {
            format!("NexaPath.{path_name}()")
        }
        ("Keyboard", "dismiss") => {
            "UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)".to_owned()
        }
        _ => format!(
            "{}Plugin.shared.{}({})",
            namespace,
            name,
            rendered.join(", ")
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
    use nexa_ir::{ArithmeticOp, CollectionUtilityKind, Expr, NumericType, TimeMethod, Type};

    #[test]
    fn conditional_expressions_use_the_native_ternary_operator() {
        let conditional = Expr::Conditional {
            condition: Box::new(Expr::Bool(true)),
            then_value: Box::new(Expr::String("ready".to_owned())),
            else_value: Box::new(Expr::String("waiting".to_owned())),
            value_type: Type::String,
        };

        assert_eq!(expression(&conditional), "(true ? \"ready\" : \"waiting\")");
    }

    #[test]
    fn clipboard_calls_use_direct_native_helpers() {
        let call =
            |name: &str, arguments: Vec<(String, Expr)>, return_type: Type| Expr::NativeCall {
                receiver: None,
                namespace: "Clipboard".to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type,
                is_async: false,
                is_throwing: false,
            };

        assert_eq!(
            expression(&call(
                "setText",
                vec![("text".to_owned(), Expr::String("copy".to_owned()))],
                Type::Void,
            )),
            "NexaClipboard.setText(\"copy\")"
        );
        assert_eq!(
            expression(&call(
                "getText",
                Vec::new(),
                Type::Optional(Box::new(Type::String)),
            )),
            "NexaClipboard.getText()"
        );
        assert_eq!(
            expression(&call("hasText", Vec::new(), Type::Bool)),
            "NexaClipboard.hasText()"
        );
    }

    #[test]
    fn network_status_calls_use_the_direct_native_helper() {
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Network".to_owned(),
            name: "isOnline".to_owned(),
            arguments: Vec::new(),
            codecs: Vec::new(),
            return_type: Type::Bool,
            is_async: false,
            is_throwing: false,
        };

        assert_eq!(expression(&call), "NexaNetwork.isOnline()");
    }

    #[test]
    fn network_upload_is_rendered_as_a_throwing_async_native_call() {
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Network".to_owned(),
            name: "upload".to_owned(),
            arguments: vec![
                (
                    "url".to_owned(),
                    Expr::String("https://example.com".to_owned()),
                ),
                ("file".to_owned(), Expr::String("receipt.pdf".to_owned())),
                ("fields".to_owned(), Expr::Map(Vec::new())),
            ],
            codecs: Vec::new(),
            return_type: Type::NetworkResponse,
            is_async: true,
            is_throwing: true,
        };

        assert_eq!(
            expression(&Expr::TryAwait(Box::new(call))),
            "(try await NexaNetwork.upload(url: \"https://example.com\", file: \"receipt.pdf\", fields: [:]))"
        );
    }

    #[test]
    fn app_storage_calls_use_direct_native_helpers() {
        let call =
            |name: &str, arguments: Vec<(String, Expr)>, return_type: Type| Expr::NativeCall {
                receiver: None,
                namespace: "Storage".to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type,
                is_async: false,
                is_throwing: false,
            };

        assert_eq!(
            expression(&call(
                "getString",
                vec![("key".to_owned(), Expr::String("theme".to_owned()))],
                Type::Optional(Box::new(Type::String)),
            )),
            "NexaStorage.getString(\"theme\")"
        );
        assert_eq!(
            expression(&call(
                "setString",
                vec![
                    ("key".to_owned(), Expr::String("theme".to_owned())),
                    ("value".to_owned(), Expr::String("dark".to_owned())),
                ],
                Type::Void,
            )),
            "NexaStorage.setString(\"theme\", \"dark\")"
        );
        assert_eq!(
            expression(&call(
                "delete",
                vec![("key".to_owned(), Expr::String("theme".to_owned()))],
                Type::Void,
            )),
            "NexaStorage.delete(\"theme\")"
        );
        assert_eq!(
            expression(&call("clear", Vec::new(), Type::Void)),
            "NexaStorage.clear()"
        );
    }

    #[test]
    fn haptics_calls_emit_direct_uikit_feedback_generators() {
        let call = |name: &str, arguments: Vec<(String, Expr)>| Expr::NativeCall {
            receiver: None,
            namespace: "Haptics".to_owned(),
            name: name.to_owned(),
            arguments,
            codecs: Vec::new(),
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
        };

        assert_eq!(
            expression(&call(
                "impact",
                vec![("style".to_owned(), Expr::String("Heavy".to_owned()))],
            )),
            "UIImpactFeedbackGenerator(style: .heavy).impactOccurred()"
        );
        assert_eq!(
            expression(&call(
                "notification",
                vec![("kind".to_owned(), Expr::String("Error".to_owned()))],
            )),
            "UINotificationFeedbackGenerator().notificationOccurred(.error)"
        );
        assert_eq!(
            expression(&call("selection", Vec::new())),
            "UISelectionFeedbackGenerator().selectionChanged()"
        );
    }

    #[test]
    fn array_utilities_use_direct_swift_collection_operations() {
        let int = |raw: &str| Expr::Number {
            raw: raw.to_owned(),
            ty: NumericType::Int32,
        };
        let collection = || Expr::Array(vec![int("1"), int("2")]);
        let utility = |operation: CollectionUtilityKind,
                       start: Option<&str>,
                       end: Option<&str>,
                       inclusive: bool| Expr::CollectionUtility {
            operation,
            collection: Box::new(collection()),
            start: start.map(|value| Box::new(int(value))),
            end: end.map(|value| Box::new(int(value))),
            inclusive,
            element_type: Type::Numeric(NumericType::Int32),
        };

        assert_eq!(
            expression(&utility(CollectionUtilityKind::Random, None, None, false)),
            "[1, 2].randomElement()"
        );
        assert_eq!(
            expression(&utility(CollectionUtilityKind::Shuffled, None, None, false)),
            "[1, 2].shuffled()"
        );
        assert_eq!(
            expression(&utility(CollectionUtilityKind::Reverse, None, None, false)),
            "Array([1, 2].reversed())"
        );
        assert_eq!(
            expression(&utility(
                CollectionUtilityKind::Slice,
                Some("0"),
                Some("1"),
                false
            )),
            "Array([1, 2][Int(0)..<Int(1)])"
        );
        assert_eq!(
            expression(&utility(
                CollectionUtilityKind::Slice,
                Some("0"),
                Some("1"),
                true
            )),
            "Array([1, 2][Int(0)...Int(1)])"
        );
    }

    #[test]
    fn arithmetic_and_negation_use_wrapping_integer_operators() {
        let int = |raw: &str| Expr::Number {
            raw: raw.to_owned(),
            ty: NumericType::Int32,
        };
        let subtract = Expr::Arithmetic {
            op: ArithmeticOp::Subtract,
            left: Box::new(int("8")),
            right: Box::new(int("3")),
            ty: NumericType::Int32,
        };
        let negate = Expr::Negate {
            value: Box::new(int("8")),
            ty: NumericType::Int32,
        };

        assert_eq!(expression(&subtract), "(8 &- 3)");
        assert_eq!(expression(&negate), "(0 &- 8)");
    }

    #[test]
    fn awaited_values_are_parenthesized_inside_binary_expressions() {
        let awaited = |name: &str| {
            Expr::Await(Box::new(Expr::Call {
                name: name.to_owned(),
                arguments: Vec::new(),
                return_type: Type::Numeric(NumericType::Int32),
                is_async: true,
                is_constructor: false,
            }))
        };
        let sum = Expr::Add(
            Box::new(awaited("first")),
            Box::new(awaited("second")),
            NumericType::Int32,
        );

        assert_eq!(
            expression(&sum),
            "((await nexa_fn_first()) &+ (await nexa_fn_second()))"
        );
    }

    #[test]
    fn string_concatenation_uses_the_native_operator() {
        let concat = Expr::Concat(
            Box::new(Expr::String("Nexa".to_owned())),
            Box::new(Expr::String(" 1.0".to_owned())),
        );

        assert_eq!(expression(&concat), "(\"Nexa\" + \" 1.0\")");
    }

    #[test]
    fn throwing_native_calls_preserve_errors_for_explicit_recovery() {
        let value = Expr::Await(Box::new(Expr::NativeCall {
            receiver: None,
            namespace: "Camera".to_owned(),
            name: "stop".to_owned(),
            arguments: Vec::new(),
            codecs: Vec::new(),
            return_type: Type::Void,
            is_async: true,
            is_throwing: true,
        }));

        assert_eq!(expression(&value), "(try await CameraPlugin.shared.stop())");
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
            "(try await NexaFile.readText(\"notes.txt\"))"
        );
        assert_eq!(
            expression(&write),
            "(try await NexaFile.writeText(\"saved\", to: \"notes.txt\"))"
        );
        assert_eq!(
            expression(&delete),
            "(try await NexaFile.delete(\"notes.txt\"))"
        );
        assert_eq!(
            expression(&exists),
            "(await NexaFile.exists(\"notes.txt\"))"
        );
    }

    #[test]
    fn clock_calls_render_direct_platform_calls() {
        let now = Expr::TimeCall {
            method: TimeMethod::Now,
            arguments: Vec::new(),
            return_type: Type::Numeric(NumericType::Int64),
            is_async: false,
        };
        assert_eq!(
            expression(&now),
            "Int64((Date().timeIntervalSince1970 * 1000).rounded())"
        );
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::Monotonic,
                arguments: Vec::new(),
                return_type: Type::Numeric(NumericType::Int64),
                is_async: false,
            }),
            "Int64(bitPattern: DispatchTime.now().uptimeNanoseconds)"
        );
        // A sleep is the one clock call that can throw, so the `Await` node is
        // what writes `try`.
        assert_eq!(
            expression(&Expr::Await(Box::new(Expr::TimeCall {
                method: TimeMethod::Sleep,
                arguments: vec![Expr::Number {
                    raw: "5".to_owned(),
                    ty: NumericType::Int64,
                }],
                return_type: Type::Void,
                is_async: true,
            }))),
            "(try await Task.sleep(nanoseconds: 5 &* 1_000_000))"
        );
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::Iso8601,
                arguments: vec![Expr::Number {
                    raw: "1700000000000".to_owned(),
                    ty: NumericType::Int64,
                }],
                return_type: Type::String,
                is_async: false,
            }),
            "nexaIso8601(1700000000000)"
        );
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::Iso8601ToMillis,
                arguments: vec![Expr::String("2026-09-27T09:41:02.123Z".to_owned())],
                return_type: Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
                is_async: false,
            }),
            "nexaIso8601ToMillis(\"2026-09-27T09:41:02.123Z\")"
        );
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
            codecs: Vec::new(),
            return_type: Type::Void,
            is_async: true,
            is_throwing: false,
        };

        assert_eq!(
            expression(&Expr::Await(Box::new(call))),
            "(await nexa_player.prepare(\"clip.mp4\"))"
        );
    }
}
