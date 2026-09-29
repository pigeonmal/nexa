use nexa_codegen::names::{function_name, state_name};
use nexa_ir::{
    ArithmeticOp, BinaryOp, CollectionTransform, CollectionUtilityKind, Expr, InterpolatedPart,
    LogMethod, MemberKind, NetworkRequest, NumericType, PermissionOpKind, PluginCodec, TimeMethod,
    TuplePosition, Type,
};

use super::utils::{kotlin_string, kotlin_string_content};
use super::{features::Features, imports::ImportSet};
use crate::generator::engine::types::kotlin_type;

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
            optional,
            kind,
            ..
        } => {
            // Rendering follows the validated `kind`; tuple positions map to
            // Kotlin's named Pair/Triple accessors.
            let member_name = match kind {
                MemberKind::TupleIndex(TuplePosition::First) => "first".to_owned(),
                MemberKind::TupleIndex(TuplePosition::Second) => "second".to_owned(),
                MemberKind::TupleIndex(TuplePosition::Third) => "third".to_owned(),
                MemberKind::StructField(name) => nexa_codegen::names::struct_field_name(name),
                MemberKind::PluginField(name) => name.clone(),
                MemberKind::NetworkStatusCode => "statusCode".to_owned(),
                MemberKind::NetworkHeaders => "headers".to_owned(),
                MemberKind::NetworkBody => "text".to_owned(),
                MemberKind::CollectionCount => "size".to_owned(),
                MemberKind::CollectionIsEmpty => "isEmpty()".to_owned(),
            };
            format!(
                "{}{}{}",
                render(base),
                if *optional { "?." } else { "." },
                member_name
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
                kotlin_type(return_type)
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
                CollectionUtilityKind::Random => format!("{collection}.randomOrNull()"),
                CollectionUtilityKind::Shuffled => format!("{collection}.shuffled()"),
                CollectionUtilityKind::Reverse => format!("{collection}.reversed()"),
                CollectionUtilityKind::Slice => {
                    let start = start
                        .as_deref()
                        .map(render)
                        .unwrap_or_else(|| "0".to_owned());
                    let end = end.as_deref().map(render).unwrap_or_else(|| "0".to_owned());
                    format!(
                        "{collection}.slice({start}{}{})",
                        if *inclusive { ".." } else { " until " },
                        end
                    )
                }
            }
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => format!(
            "(if ({}) {} else {})",
            render(condition),
            render(then_value),
            render(else_value)
        ),
        Expr::TimeCall {
            method, arguments, ..
        } => time_call(*method, arguments, locals),
        Expr::LogCall { method, message } => {
            let severity = match method {
                LogMethod::Info => "i",
                LogMethod::Warning => "w",
                LogMethod::Error => "e",
            };
            format!("android.util.Log.{severity}(\"Nexa\", {})", render(message))
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
        Expr::FileWriteText { path, contents } => {
            format!("NexaFile.writeText({}, {})", render(contents), render(path))
        }
        Expr::FileDelete { path } => format!("NexaFile.delete({})", render(path)),
        // `ByteArray` conversions stay in the standard library; `fromText`
        // encodes straight from the string instead of building a list.
        Expr::BytesFromText { text } => format!("{}.toByteArray()", render(text)),
        Expr::BytesFromArray { values } => {
            format!("{}.map {{ it.toByte() }}.toByteArray()", render(values))
        }
        Expr::BytesCount { bytes } => format!("{}.size", render(bytes)),
        Expr::PermissionOp { op, permission } => {
            let method = match op {
                PermissionOpKind::Status => "status",
                PermissionOpKind::Request => "request",
            };
            format!(
                "NexaPermissions.{method}(NexaRuntime.context(), {})",
                render(permission)
            )
        }
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
        Expr::Concat(left, right) => {
            format!("({} + {})", render(left), render(right))
        }
        Expr::Arithmetic {
            op,
            left,
            right,
            ty,
        } => {
            let operator = match op {
                ArithmeticOp::Subtract => "-",
                ArithmeticOp::Multiply => "*",
                ArithmeticOp::Divide => "/",
                ArithmeticOp::Remainder => "%",
            };
            let result = format!("({} {operator} {})", render(left), render(right));
            match ty {
                NumericType::Int8 => format!("{result}.toByte()"),
                NumericType::Int16 => format!("{result}.toShort()"),
                NumericType::UInt8 => format!("{result}.toUByte()"),
                NumericType::UInt16 => format!("{result}.toUShort()"),
                _ => result,
            }
        }
        Expr::Negate { value, ty } => match ty {
            NumericType::Float32 | NumericType::Float64 => format!("(-{})", render(value)),
            NumericType::UInt8 => format!("(0u.toUByte() - {}).toUByte()", render(value)),
            NumericType::UInt16 => format!("(0u.toUShort() - {}).toUShort()", render(value)),
            NumericType::UInt32 => format!("(0u - {})", render(value)),
            NumericType::UInt64 => format!("(0uL - {})", render(value)),
            NumericType::Int8 => format!("(0 - {}).toByte()", render(value)),
            NumericType::Int16 => format!("(0 - {}).toShort()", render(value)),
            NumericType::Int32 | NumericType::Int64 => format!("(-{})", render(value)),
        },
        Expr::Binary { op, left, right } => format!(
            "({} {} {})",
            render(left),
            binary_operator(*op),
            render(right)
        ),
        Expr::Contains {
            value, collection, ..
        } => format!("({} in {})", render(value), render(collection)),
        Expr::ResultOk { value, .. } => format!("NexaResult.Success({})", render(value)),
        Expr::ResultErr { error, .. } => format!("NexaResult.Failure({})", render(error)),
        Expr::Try { expr, .. } => format!("{}.getOrThrow()", render(expr)),
    }
}

/// A core clock call. Every one is a direct platform call: a system clock, a
/// monotonic counter, a coroutine delay, or the generated ISO 8601 helpers.
fn time_call(method: TimeMethod, arguments: &[Expr], locals: &[String]) -> String {
    let rendered = arguments
        .iter()
        .map(|argument| expression_with_locals(argument, locals))
        .collect::<Vec<_>>();
    let first = rendered.first().cloned().unwrap_or_else(|| "0L".to_owned());
    match method {
        TimeMethod::Now => "System.currentTimeMillis()".to_owned(),
        TimeMethod::Monotonic => "System.nanoTime()".to_owned(),
        TimeMethod::Elapsed => format!("(System.nanoTime() - {first})"),
        TimeMethod::Sleep => format!("kotlinx.coroutines.delay({first})"),
        TimeMethod::Iso8601 => format!("nexaIso8601({first})"),
        TimeMethod::Iso8601ToMillis => format!("nexaIso8601ToMillis({first})"),
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
    // Codec closures are appended positionally. Plugin implementations may
    // choose their own parameter names, and positional calls keep those source
    // names out of the generated app code.
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
            format!("{{ reader -> {function}(reader) }}")
        } else {
            format!("{{ item, writer -> {function}(item, writer) }}")
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
        ("Path", path_name) => {
            format!("NexaPath.{path_name}(NexaRuntime.context())")
        }
        ("Keyboard", "dismiss") => "dev.nexa.core.NexaRuntimeCore.dismissKeyboard()".to_owned(),
        _ => format!(
            "{}Plugin.instance.{}({})",
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
        None => "null".to_owned(),
        Some(body) if is_optional_expression(body) => {
            format!("{}?.toByteArray()", render(body))
        }
        Some(body) => format!("{}.toByteArray()", render(body)),
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
            "NexaNetwork.download(NexaRuntime.context(), {url}, {}, {method}, {body}, {headers}, ({timeout} * 1000.0).toLong(), {use_cache}, {follow_redirects}, {max_response_bytes}, {certificate_pins})",
            render(destination),
        ),
        None => format!(
            "NexaNetwork.fetch(NexaRuntime.context(), {url}, {method}, {body}, {headers}, ({timeout} * 1000.0).toLong(), {use_cache}, {follow_redirects}, {max_response_bytes}, {certificate_pins})"
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

#[cfg(test)]
mod tests {
    use super::expression;
    use nexa_ir::{ArithmeticOp, CollectionUtilityKind, Expr, NumericType, TimeMethod, Type};

    #[test]
    fn conditional_expressions_use_a_native_if_expression() {
        let conditional = Expr::Conditional {
            condition: Box::new(Expr::Bool(true)),
            then_value: Box::new(Expr::String("ready".to_owned())),
            else_value: Box::new(Expr::String("waiting".to_owned())),
            value_type: Type::String,
        };

        assert_eq!(
            expression(&conditional),
            "(if (true) \"ready\" else \"waiting\")"
        );
    }

    #[test]
    fn array_utilities_use_direct_kotlin_collection_operations() {
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
            "listOf(1, 2).randomOrNull()"
        );
        assert_eq!(
            expression(&utility(CollectionUtilityKind::Shuffled, None, None, false)),
            "listOf(1, 2).shuffled()"
        );
        assert_eq!(
            expression(&utility(CollectionUtilityKind::Reverse, None, None, false)),
            "listOf(1, 2).reversed()"
        );
        assert_eq!(
            expression(&utility(
                CollectionUtilityKind::Slice,
                Some("0"),
                Some("1"),
                false
            )),
            "listOf(1, 2).slice(0 until 1)"
        );
        assert_eq!(
            expression(&utility(
                CollectionUtilityKind::Slice,
                Some("0"),
                Some("1"),
                true
            )),
            "listOf(1, 2).slice(0..1)"
        );
    }

    #[test]
    fn arithmetic_and_negation_render_for_kotlin_numeric_types() {
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

        assert_eq!(expression(&subtract), "(8 - 3)");
        assert_eq!(expression(&negate), "(-8)");
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
    fn file_calls_use_the_generated_native_helper_names_and_argument_order() {
        let path = || Box::new(Expr::String("notes.txt".to_owned()));
        let read = Expr::Await(Box::new(Expr::FileReadText { path: path() }));
        let write = Expr::Await(Box::new(Expr::FileWriteText {
            contents: Box::new(Expr::String("saved".to_owned())),
            path: path(),
        }));
        let delete = Expr::Await(Box::new(Expr::FileDelete { path: path() }));

        assert_eq!(expression(&read), "NexaFile.readText(\"notes.txt\")");
        assert_eq!(
            expression(&write),
            "NexaFile.writeText(\"saved\", \"notes.txt\")"
        );
        assert_eq!(expression(&delete), "NexaFile.delete(\"notes.txt\")");
    }

    #[test]
    fn clock_calls_render_direct_platform_calls() {
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::Now,
                arguments: Vec::new(),
                return_type: Type::Numeric(NumericType::Int64),
                is_async: false,
            }),
            "System.currentTimeMillis()"
        );
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::Monotonic,
                arguments: Vec::new(),
                return_type: Type::Numeric(NumericType::Int64),
                is_async: false,
            }),
            "System.nanoTime()"
        );
        // Kotlin's `await` is transparent, so a sleep is the bare call.
        assert_eq!(
            expression(&Expr::Await(Box::new(Expr::TimeCall {
                method: TimeMethod::Sleep,
                arguments: vec![Expr::Number {
                    raw: "5L".to_owned(),
                    ty: NumericType::Int64,
                }],
                return_type: Type::Void,
                is_async: true,
            }))),
            "kotlinx.coroutines.delay(5LL)"
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
            "nexaIso8601(1700000000000L)"
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
            codecs: Vec::new(),
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
