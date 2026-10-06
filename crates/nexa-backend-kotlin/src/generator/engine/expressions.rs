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
    imports.add(
        features.facts.capabilities.uses_localized_strings || features.uses_text_input,
        "androidx.compose.ui.res.stringResource",
    );
    imports.add(
        features.facts.capabilities.uses_localized_strings || features.uses_text_input,
        "androidx.compose.ui.res.pluralStringResource",
    );
}

pub(crate) fn expression(expr: &Expr) -> String {
    expression_with_locals(expr, &[])
}

fn expression_with_locals(expr: &Expr, locals: &[String]) -> String {
    let render = |value: &Expr| expression_with_locals(value, locals);
    match expr {
        Expr::This(_) => "this".to_owned(),
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
        Expr::LocalizedText { key, value, .. } => localized_text_expression(key, value, locals),
        Expr::Bool(value) => value.to_string(),
        Expr::IsRegularWidth => "(LocalConfiguration.current.screenWidthDp >= 600)".to_owned(),
        Expr::IsCompactWidth => "(LocalConfiguration.current.screenWidthDp < 600)".to_owned(),
        Expr::IsRegularHeight => "(LocalConfiguration.current.screenHeightDp >= 600)".to_owned(),
        Expr::IsCompactHeight => "(LocalConfiguration.current.screenHeightDp < 600)".to_owned(),
        Expr::Number { raw, ty } => kotlin_number(raw, *ty),
        Expr::State(name, _) if locals.iter().any(|local| local == name) => name.clone(),
        Expr::State(name, _) => match name.split_once("::") {
            Some((class, property)) => format!(
                "{}.{}",
                nexa_codegen::names::struct_name(class),
                state_name(property)
            ),
            None => state_name(name),
        },
        Expr::AnimatedState(name, Type::Numeric(NumericType::Float64)) => {
            format!("{}Animated.toDouble()", state_name(name))
        }
        Expr::AnimatedState(name, _) if locals.iter().any(|local| local == name) => name.clone(),
        Expr::AnimatedState(name, _) => format!("{}Animated", state_name(name)),
        Expr::EnumValue {
            enum_name,
            case_name,
        } => format!(
            "{}.{}",
            nexa_codegen::names::enum_name(enum_name),
            case_name
        ),
        Expr::PluginEnumValue {
            enum_name,
            case_name,
            ..
        } => format!("{enum_name}.{case_name}"),
        Expr::PluginEnumConstructor {
            enum_name,
            case_name,
            arguments,
            ..
        } => format!(
            "{enum_name}.{case_name}({})",
            arguments.iter().map(render).collect::<Vec<_>>().join(", ")
        ),
        Expr::PluginEnumOptionalConstructor {
            enum_name,
            case_name,
            null_case_name,
            value,
            ..
        } => format!(
            "({})?.let {{ {enum_name}.{case_name}(it) }} ?: {enum_name}.{null_case_name}",
            render(value)
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
            base_type,
            kind,
            ..
        } => {
            if let MemberKind::ClassStaticField(field) = kind {
                let owner = match base_type {
                    Type::Class { name, .. } => nexa_codegen::names::struct_name(name),
                    _ => render(base),
                };
                return format!("{}.{}", owner, nexa_codegen::names::state_name(field));
            }
            // Rendering follows the validated `kind`; tuple positions map to
            // Kotlin's named Pair/Triple accessors.
            let member_name = match kind {
                MemberKind::TupleIndex(TuplePosition::First) => "first".to_owned(),
                MemberKind::TupleIndex(TuplePosition::Second) => "second".to_owned(),
                MemberKind::TupleIndex(TuplePosition::Third) => "third".to_owned(),
                MemberKind::StructField(name) => nexa_codegen::names::struct_field_name(name),
                MemberKind::ClassField(name) => nexa_codegen::names::state_name(name),
                MemberKind::ClassStaticField(name) => nexa_codegen::names::state_name(name),
                MemberKind::PluginField(name) => name.clone(),
                MemberKind::NetworkStatusCode => "statusCode".to_owned(),
                MemberKind::NetworkHeaders => "headers".to_owned(),
                MemberKind::NetworkBody => "text".to_owned(),
                MemberKind::CollectionCount => "size".to_owned(),
                MemberKind::CollectionIsEmpty => "isEmpty()".to_owned(),
                MemberKind::StringTrimmed => "trim()".to_owned(),
                MemberKind::SignalValue => "value".to_owned(),
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
            } else if let Some((class_name, method_name)) = name.split_once('.') {
                format!(
                    "{}.{}",
                    nexa_codegen::names::struct_name(class_name),
                    nexa_codegen::names::function_name(method_name)
                )
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
                CollectionTransform::FlatMap => format!("{collection}.flatMap {closure}"),
                CollectionTransform::Filter => format!("{collection}.filter {closure}"),
                CollectionTransform::Reduce => format!(
                    "{collection}.fold({}, {closure})",
                    initial
                        .as_deref()
                        .map(render)
                        .unwrap_or_else(|| "0".to_owned())
                ),
                CollectionTransform::SortedBy => format!("{collection}.sortedBy {closure}"),
                CollectionTransform::GroupedBy => {
                    format!("{collection}.groupBy {closure}.values.toList()")
                }
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
                CollectionUtilityKind::First => format!("{collection}.firstOrNull()"),
                CollectionUtilityKind::Last => format!("{collection}.lastOrNull()"),
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
                CollectionUtilityKind::Take => {
                    let count = end.as_deref().map(render).unwrap_or_else(|| "0".to_owned());
                    format!("{collection}.take({count}.coerceAtLeast(0))")
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
            value,
            collection,
            collection_type,
        } => {
            if matches!(collection_type, Type::String) {
                format!(
                    "{}.contains({}, ignoreCase = true)",
                    render(collection),
                    render(value)
                )
            } else {
                format!("({} in {})", render(value), render(collection))
            }
        }
        Expr::ResultOk { value, .. } => format!("NexaResult.Success({})", render(value)),
        Expr::ResultErr { error, .. } => format!("NexaResult.Failure({})", render(error)),
        Expr::Try { expr, .. } => format!("{}.getOrThrow()", render(expr)),
    }
}

fn localized_text_expression(key: &str, value: &Expr, locals: &[String]) -> String {
    let arguments = nexa_ir::localization::interpolation_arguments(value);
    let format_arguments = arguments
        .into_iter()
        .map(|argument| expression_with_locals(argument, locals))
        .collect::<Vec<_>>();
    let resource = nexa_codegen::names::localization_resource_name(key);
    let count_argument = nexa_ir::localization::interpolation_arguments(value)
        .into_iter()
        .enumerate()
        .find(|(index, argument)| {
            nexa_ir::localization::argument_name(argument, *index) == "count"
                && nexa_ir::localization::is_integer_argument(argument)
        });
    if let Some((_, count)) = count_argument {
        let count = expression_with_locals(count, locals);
        format!(
            "NexaRuntime.context().resources.getQuantityString(R.plurals.{resource}, {count}{})",
            if format_arguments.is_empty() {
                String::new()
            } else {
                format!(", {}", format_arguments.join(", "))
            }
        )
    } else {
        let format_arguments = if format_arguments.is_empty() {
            String::new()
        } else {
            format!(", {}", format_arguments.join(", "))
        };
        format!("NexaRuntime.context().getString(R.string.{resource}{format_arguments})")
    }
}

fn localized_composable_text(key: &str, value: &Expr) -> String {
    let arguments = nexa_ir::localization::interpolation_arguments(value);
    let format_arguments = arguments
        .iter()
        .map(|argument| expression(argument))
        .collect::<Vec<_>>();
    let resource = nexa_codegen::names::localization_resource_name(key);
    let count_argument = arguments.into_iter().enumerate().find(|(index, argument)| {
        nexa_ir::localization::argument_name(argument, *index) == "count"
            && nexa_ir::localization::is_integer_argument(argument)
    });
    if let Some((_, count)) = count_argument {
        let count = expression(count);
        format!(
            "pluralStringResource(R.plurals.{resource}, {count}{})",
            if format_arguments.is_empty() {
                String::new()
            } else {
                format!(", {}", format_arguments.join(", "))
            }
        )
    } else {
        format!(
            "stringResource(R.string.{resource}{})",
            if format_arguments.is_empty() {
                String::new()
            } else {
                format!(", {}", format_arguments.join(", "))
            }
        )
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
        TimeMethod::StartOfDay => format!("nexaStartOfDay({first})"),
        TimeMethod::AddCalendarDays => {
            let days = rendered.get(1).cloned().unwrap_or_else(|| "0".to_owned());
            format!("nexaAddCalendarDays({first}, {days})")
        }
        TimeMethod::LocalizedDate => format!(
            "java.text.DateFormat.getDateInstance(java.text.DateFormat.SHORT).format(java.util.Date({first}))"
        ),
        TimeMethod::LocalizedTime => format!(
            "java.text.DateFormat.getTimeInstance(java.text.DateFormat.SHORT).format(java.util.Date({first}))"
        ),
        TimeMethod::LocalizedDateTime => format!(
            "java.text.DateFormat.getDateTimeInstance(java.text.DateFormat.SHORT, java.text.DateFormat.SHORT).format(java.util.Date({first}))"
        ),
        TimeMethod::Format => {
            let pattern = rendered
                .get(1)
                .cloned()
                .unwrap_or_else(|| "\"\"".to_owned());
            format!("nexaFormatDate({first}, {pattern})")
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
            let helper = if name == "parse" {
                nexa_codegen::value::json_parse_name(&codec.ty)
            } else {
                nexa_codegen::value::json_stringify_name(&codec.ty)
            };
            return format!("{helper}({raw_or_value})");
        }
    }
    // Codec closures are appended positionally. Plugin implementations may
    // choose their own parameter names, and positional calls keep those source
    // names out of the generated app code.
    for codec in codecs {
        if codec.row_mapper {
            if let Some(mapper) = plugin_row_mapper(&codec.ty) {
                rendered.push(mapper);
            }
            continue;
        }
        let function = nexa_codegen::value::codec_name(
            &codec.ty,
            if codec.decodes {
                nexa_codegen::value::Direction::Read
            } else {
                nexa_codegen::value::Direction::Write
            },
        );
        rendered.push(if codec.decodes {
            if matches!(&codec.ty, nexa_ir::Type::Optional(_)) {
                format!("{{ reader -> {function}(reader) }}")
            } else {
                let result = format!("{}.NexaValueReadResult", nexa_codegen::value::KOTLIN_CORE_PACKAGE);
                format!(
                    "{{ reader -> {function}(reader)?.let {{ {result}.Value(it) }} ?: {result}.Invalid }}"
                )
            }
        } else {
            format!("{{ item, writer -> {function}(item, writer) }}")
        });
    }
    if let Some(receiver) = receiver {
        let native_name = if namespace.starts_with("__NexaUserClass:") {
            nexa_codegen::names::function_name(name)
        } else {
            name.to_owned()
        };
        return format!(
            "{}.{}({})",
            expression_with_locals(receiver, locals),
            native_name,
            rendered.join(", ")
        );
    }
    match (namespace, name) {
        ("Screen", "lockOrientation") => format!(
            "dev.nexa.core.NexaRuntimeCore.lockOrientation({})",
            rendered.first().map(String::as_str).unwrap_or("\"All\"")
        ),
        ("AppIcon", "set") => format!(
            "NexaRuntime.setAlternateAppIcon({})",
            rendered.first().map(String::as_str).unwrap_or("null")
        ),
        ("Network", "isOnline") => "NexaNetwork.isOnline(NexaRuntime.context())".to_owned(),
        ("Network", "upload") => format!(
            "NexaNetwork.upload(NexaRuntime.context(), {}, {}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\""),
            rendered.get(2).map(String::as_str).unwrap_or("emptyMap()"),
        ),
        ("Haptics", "impact") => {
            let style = arguments
                .iter()
                .find(|(argument, _)| argument == "style")
                .and_then(|(_, value)| match value {
                    Expr::String(style) => Some(style.as_str()),
                    _ => None,
                });
            let constant = match style {
                Some("Medium") => "VIRTUAL_KEY",
                Some("Heavy") => "LONG_PRESS",
                _ => "KEYBOARD_TAP",
            };
            format!(
                "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.{constant})"
            )
        }
        ("Haptics", "notification") => {
            let kind = arguments
                .iter()
                .find(|(argument, _)| argument == "kind")
                .and_then(|(_, value)| match value {
                    Expr::String(kind) => Some(kind.as_str()),
                    _ => None,
                });
            let constant = if kind == Some("Error") {
                "REJECT"
            } else {
                "CONFIRM"
            };
            format!(
                "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.{constant})"
            )
        }
        ("Haptics", "selection") => "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.KEYBOARD_TAP)".to_owned(),
        ("Number", "formatCurrency") => format!(
            "nexaFormatCurrency({}, {})",
            rendered.first().map(String::as_str).unwrap_or("0.0"),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("Locale", "currentLanguageCode") => "java.util.Locale.getDefault().language".to_owned(),
        ("Locale", "preferredLanguageCodes") => "if (android.os.Build.VERSION.SDK_INT >= 24) android.os.LocaleList.getDefault().let { nexaLocales -> (0 until nexaLocales.size()).map { nexaLocales[it].toLanguageTag() } } else listOf(java.util.Locale.getDefault().toLanguageTag())".to_owned(),
        ("Locale", "displayName") => format!(
            "java.util.Locale.forLanguageTag({}).getDisplayLanguage(java.util.Locale.getDefault()).takeIf {{ it.isNotEmpty() }}",
            rendered.first().map(String::as_str).unwrap_or("\"und\"")
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
            "NexaSecureStorage.get(NexaRuntime.context(), {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "set") => format!(
            "NexaSecureStorage.set(NexaRuntime.context(), {}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\""),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "delete") => format!(
            "NexaSecureStorage.delete(NexaRuntime.context(), {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("SecureStorage", "clear") => "NexaSecureStorage.clear(NexaRuntime.context())".to_owned(),
        ("Storage", "getString") => format!(
            "NexaStorage.getString(NexaRuntime.context(), {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "setString") => format!(
            "NexaStorage.setString(NexaRuntime.context(), {}, {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"").to_owned(),
            rendered.get(1).map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "delete") => format!(
            "NexaStorage.delete(NexaRuntime.context(), {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Storage", "clear") => "NexaStorage.clear(NexaRuntime.context())".to_owned(),
        ("Clipboard", "setText") => format!(
            "NexaClipboard.setText(NexaRuntime.context(), {})",
            rendered.first().map(String::as_str).unwrap_or("\"\"")
        ),
        ("Clipboard", "getText") => "NexaClipboard.getText(NexaRuntime.context())".to_owned(),
        ("Clipboard", "hasText") => "NexaClipboard.hasText(NexaRuntime.context())".to_owned(),
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

/// Emits a typed native row mapper. Column names are resolved once before
/// iteration, and each row reads only the requested columns.
fn plugin_row_mapper(ty: &Type) -> Option<String> {
    let Type::Struct { name, fields } = ty else {
        return None;
    };
    let mut setup = Vec::with_capacity(fields.len() + 4);
    let mut values = Vec::with_capacity(fields.len());
    let expected_columns = fields
        .iter()
        .map(|(field, _)| kotlin_string(field))
        .collect::<Vec<_>>()
        .join(", ");
    setup.push(format!(
        "val nexaExpectedColumns = setOf({expected_columns})"
    ));
    setup.push("val nexaColumnIndices = HashMap<String, Int>(nexaExpectedColumns.size); val nexaDuplicateColumns = HashSet<String>()".to_owned());
    setup.push("columnNames.forEachIndexed { index, name -> if (name in nexaExpectedColumns && nexaColumnIndices.put(name, index) != null) nexaDuplicateColumns.add(name) }".to_owned());
    for (index, (field, field_type)) in fields.iter().enumerate() {
        let column = format!("nexaColumn{index}");
        setup.push(format!(
            "val {column} = nexaColumnIndices[{}] ?: throw rowFailure({}); if ({} in nexaDuplicateColumns) throw rowFailure({})",
            kotlin_string(field),
            kotlin_string(&format!("Query must select column `{field}` exactly once.")),
            kotlin_string(field),
            kotlin_string(&format!("Query must select column `{field}` exactly once.")),
        ));
        values.push(format!(
            "{} = {}",
            nexa_codegen::names::struct_field_name(field),
            plugin_kotlin_decode(field_type, &column, field),
        ));
    }
    Some(format!(
        "{{ columnNames, rowFailure -> {}; {{ row -> {}({}) }} }}",
        setup.join("; "),
        nexa_codegen::names::struct_name(name),
        values.join(", "),
    ))
}

fn plugin_kotlin_decode(ty: &Type, index: &str, column: &str) -> String {
    let (optional, value_type) = match ty {
        Type::Optional(inner) => (true, inner.as_ref()),
        other => (false, other),
    };
    let failure = kotlin_string(&format!("Row field `{column}` has an incompatible value."));
    let (getter, conversion) = match value_type {
        Type::Bool => ("boolean", None),
        Type::String => ("text", None),
        Type::Bytes => ("bytes", None),
        Type::Numeric(NumericType::Float32) => ("decimal", Some(("toFloat", None))),
        Type::Numeric(NumericType::Float64) => ("decimal", None),
        Type::Numeric(NumericType::Int8) => (
            "integer",
            Some((
                "toByte",
                Some("Byte.MIN_VALUE.toLong()..Byte.MAX_VALUE.toLong()"),
            )),
        ),
        Type::Numeric(NumericType::Int16) => (
            "integer",
            Some((
                "toShort",
                Some("Short.MIN_VALUE.toLong()..Short.MAX_VALUE.toLong()"),
            )),
        ),
        Type::Numeric(NumericType::Int32) => (
            "integer",
            Some((
                "toInt",
                Some("Int.MIN_VALUE.toLong()..Int.MAX_VALUE.toLong()"),
            )),
        ),
        Type::Numeric(NumericType::Int64) => ("integer", None),
        Type::Numeric(NumericType::UInt8) => (
            "integer",
            Some(("toUByte", Some("0L..UByte.MAX_VALUE.toLong()"))),
        ),
        Type::Numeric(NumericType::UInt16) => (
            "integer",
            Some(("toUShort", Some("0L..UShort.MAX_VALUE.toLong()"))),
        ),
        Type::Numeric(NumericType::UInt32) => (
            "integer",
            Some(("toUInt", Some("0L..UInt.MAX_VALUE.toLong()"))),
        ),
        Type::Numeric(NumericType::UInt64) => {
            ("integer", Some(("toULong", Some("0L..Long.MAX_VALUE"))))
        }
        _ => return format!("throw rowFailure({failure})"),
    };
    let reader_method = if optional {
        format!("optional{}", capitalize_ascii(getter))
    } else {
        getter.to_owned()
    };
    let read = format!("row.{reader_method}({index}, {})", kotlin_string(column));
    match (optional, conversion) {
        (false, Some((convert, Some(bounds)))) => format!(
            "run {{ val raw = {read}; if (raw in {bounds}) raw.{convert}() else throw rowFailure({failure}) }}"
        ),
        (true, Some((convert, Some(bounds)))) => format!(
            "{read}?.let {{ raw -> if (raw in {bounds}) raw.{convert}() else throw rowFailure({failure}) }}"
        ),
        (false, Some((convert, None))) => format!("{read}.{convert}()"),
        (true, Some((convert, None))) => format!("{read}?.{convert}()"),
        (_, None) => read,
    }
}

fn capitalize_ascii(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + characters.as_str(),
        None => String::new(),
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
    if let Expr::LocalizedText { key, value, .. } = expr {
        return localized_composable_text(key, value);
    }
    if is_non_optional_string_expression(expr) {
        expression(expr)
    } else {
        format!("{}.toString()", expression(expr))
    }
}

fn is_non_optional_string_expression(expr: &Expr) -> bool {
    match expr {
        Expr::String(_)
        | Expr::Interpolation(_)
        | Expr::Concat(_, _)
        | Expr::LocalizedText { .. } => true,
        Expr::State(_, Type::String) | Expr::AnimatedState(_, Type::String) => true,
        Expr::Member {
            optional: false,
            field_type: Type::String,
            ..
        }
        | Expr::Index {
            optional: false,
            element_type: Type::String,
            ..
        } => true,
        Expr::Call {
            return_type: Type::String,
            ..
        }
        | Expr::NativeCall {
            return_type: Type::String,
            ..
        }
        | Expr::TimeCall {
            return_type: Type::String,
            ..
        }
        | Expr::Conditional {
            value_type: Type::String,
            ..
        }
        | Expr::Try {
            value_type: Type::String,
            ..
        }
        | Expr::PathJoin { .. }
        | Expr::FileReadText { .. } => true,
        Expr::Coalesce(_, fallback) | Expr::Await(fallback) | Expr::TryAwait(fallback) => {
            is_non_optional_string_expression(fallback)
        }
        Expr::CollectionTransform {
            operation: CollectionTransform::Reduce,
            initial: Some(initial),
            ..
        } => is_non_optional_string_expression(initial),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{PluginCodec, expression, plugin_row_mapper, text_expression};
    use nexa_ir::{
        ArithmeticOp, CollectionTransform, CollectionUtilityKind, Expr, InterpolatedPart,
        NumericType, TimeMethod, Type,
    };

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
    fn optional_plugin_payload_is_mapped_once_and_uses_the_null_case() {
        let mapped = Expr::PluginEnumOptionalConstructor {
            namespace: "SQLite".to_owned(),
            enum_name: "Value".to_owned(),
            case_name: "int64".to_owned(),
            null_case_name: "nullValue".to_owned(),
            payload_name: "value".to_owned(),
            value: Box::new(Expr::State(
                "id".to_owned(),
                Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
            )),
            return_type: Type::Plugin {
                namespace: "SQLite".to_owned(),
                name: "Value".to_owned(),
            },
        };

        assert_eq!(
            expression(&mapped),
            "(nexa_id)?.let { Value.int64(it) } ?: Value.nullValue"
        );
    }

    #[test]
    fn plugin_row_mapper_resolves_columns_once_and_reads_only_typed_fields() {
        let ty = Type::Struct {
            name: "Note".to_owned(),
            fields: vec![
                (
                    "id".to_owned(),
                    Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
                ),
                ("title".to_owned(), Type::String),
            ],
        };

        let mapper = plugin_row_mapper(&ty).expect("structs should receive typed row mappers");
        assert!(mapper.contains("columnNames.forEachIndexed"));
        assert!(!mapper.contains("columnNames.indexOf"));
        assert!(mapper.contains("row.optionalInteger(nexaColumn0, \"id\")"));
        assert!(mapper.contains("row.text(nexaColumn1, \"title\")"));
        assert!(!mapper.contains("Value"));
        assert!(!mapper.contains("row["));
    }

    #[test]
    fn plugin_row_mapper_generates_direct_optional_and_range_checked_reads() {
        let ty = Type::Struct {
            name: "TypedNote".to_owned(),
            fields: vec![
                (
                    "count".to_owned(),
                    Type::Optional(Box::new(Type::Numeric(NumericType::Int32))),
                ),
                ("enabled".to_owned(), Type::Bool),
                ("ratio".to_owned(), Type::Numeric(NumericType::Float32)),
                ("payload".to_owned(), Type::Optional(Box::new(Type::Bytes))),
            ],
        };

        let mapper = plugin_row_mapper(&ty).expect("structs should receive typed row mappers");
        assert!(mapper.contains("row.optionalInteger(nexaColumn0, \"count\")"));
        assert!(mapper.contains("raw in Int.MIN_VALUE.toLong()..Int.MAX_VALUE.toLong()"));
        assert!(mapper.contains("row.boolean(nexaColumn1, \"enabled\")"));
        assert!(mapper.contains("row.decimal(nexaColumn2, \"ratio\").toFloat()"));
        assert!(mapper.contains("row.optionalBytes(nexaColumn3, \"payload\")"));
    }

    #[test]
    fn typed_row_mapping_works_for_any_plugin_namespace() {
        let item = Type::Struct {
            name: "Item".to_owned(),
            fields: vec![("title".to_owned(), Type::String)],
        };
        let call = Expr::NativeCall {
            receiver: None,
            namespace: "Records".to_owned(),
            name: "load".to_owned(),
            arguments: Vec::new(),
            codecs: vec![PluginCodec {
                ty: item.clone(),
                decodes: true,
                row_mapper: true,
            }],
            return_type: Type::Array(Box::new(item)),
            is_async: true,
            is_throwing: true,
        };

        let generated = expression(&call);
        assert!(generated.contains("RecordsPlugin.instance.load"));
        assert!(generated.contains("rowFailure"));
        assert!(generated.contains("row.text(nexaColumn0, \"title\")"));
        assert!(!generated.contains("SQLite"));
    }

    #[test]
    fn text_expression_avoids_string_conversion_for_typed_string_values() {
        let index = |optional| Expr::Index {
            collection: Box::new(Expr::State(
                "messages".to_owned(),
                Type::Array(Box::new(Type::String)),
            )),
            index: Box::new(Expr::Number {
                raw: "0".to_owned(),
                ty: NumericType::Int32,
            }),
            optional,
            collection_type: Type::Array(Box::new(Type::String)),
            element_type: Type::String,
        };
        let direct_index = index(false);
        assert_eq!(text_expression(&direct_index), "nexa_messages[0]");

        let call = Expr::Call {
            name: "displayName".to_owned(),
            arguments: Vec::new(),
            return_type: Type::String,
            is_async: false,
            is_throwing: false,
            is_constructor: false,
        };
        assert_eq!(text_expression(&call), "nexa_fn_displayName()");

        let optional_index = index(true);
        assert_eq!(
            text_expression(&optional_index),
            "nexa_messages?.get(0).toString()"
        );
    }

    #[test]
    fn localized_text_nodes_use_compose_resource_lookups() {
        let localized = Expr::LocalizedText {
            key: "Welcome, {name}!".to_owned(),
            value: Box::new(Expr::Interpolation(vec![
                InterpolatedPart::Literal("Welcome, ".to_owned()),
                InterpolatedPart::Value(Box::new(Expr::State("name".to_owned(), Type::String))),
                InterpolatedPart::Literal("!".to_owned()),
            ])),
            comment: None,
        };
        let resource = nexa_codegen::names::localization_resource_name("Welcome, {name}!");

        assert_eq!(
            text_expression(&localized),
            format!("stringResource(R.string.{resource}, nexa_name)")
        );
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
            "NexaClipboard.setText(NexaRuntime.context(), \"copy\")"
        );
        assert_eq!(
            expression(&call(
                "getText",
                Vec::new(),
                Type::Optional(Box::new(Type::String)),
            )),
            "NexaClipboard.getText(NexaRuntime.context())"
        );
        assert_eq!(
            expression(&call("hasText", Vec::new(), Type::Bool)),
            "NexaClipboard.hasText(NexaRuntime.context())"
        );
    }

    #[test]
    fn localized_strings_use_compile_time_resource_identifiers() {
        let literal = Expr::LocalizedText {
            key: "Save".to_owned(),
            value: Box::new(Expr::String("Save".to_owned())),
            comment: Some("Button title".to_owned()),
        };
        let resource = nexa_codegen::names::localization_resource_name("Save");
        assert_eq!(
            expression(&literal),
            format!("NexaRuntime.context().getString(R.string.{resource})")
        );
        assert!(!expression(&literal).contains("getIdentifier"));

        let formatted = Expr::LocalizedText {
            key: "Welcome {name}, score {value2}".to_owned(),
            value: Box::new(Expr::Interpolation(vec![
                InterpolatedPart::Literal("Welcome ".to_owned()),
                InterpolatedPart::Value(Box::new(Expr::State("name".to_owned(), Type::String))),
                InterpolatedPart::Literal(", score ".to_owned()),
                InterpolatedPart::Value(Box::new(Expr::Number {
                    raw: "7".to_owned(),
                    ty: NumericType::Int32,
                })),
            ])),
            comment: None,
        };
        let formatted_source = expression(&formatted);
        assert!(formatted_source.contains("NexaRuntime.context().getString(R.string."));
        assert!(formatted_source.contains("nexa_name, 7"));

        let plural = Expr::LocalizedText {
            key: "{count} task".to_owned(),
            value: Box::new(Expr::Interpolation(vec![
                InterpolatedPart::Value(Box::new(Expr::State(
                    "count".to_owned(),
                    Type::Numeric(NumericType::Int32),
                ))),
                InterpolatedPart::Literal(" task".to_owned()),
            ])),
            comment: None,
        };
        let plural_source = expression(&plural);
        assert!(plural_source.contains("R.plurals."));
        assert!(plural_source.contains("NexaRuntime.context().resources.getQuantityString("));
        assert!(plural_source.contains("nexa_count, nexa_count"));
        let plural_resource = nexa_codegen::names::localization_resource_name("{count} task");
        assert!(plural_source.contains(&format!("R.plurals.{plural_resource}")));
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

        assert_eq!(
            expression(&call),
            "NexaNetwork.isOnline(NexaRuntime.context())"
        );
    }

    #[test]
    fn network_upload_is_rendered_as_a_suspending_native_call() {
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
            "NexaNetwork.upload(NexaRuntime.context(), \"https://example.com\", \"receipt.pdf\", mapOf())"
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
            "NexaStorage.getString(NexaRuntime.context(), \"theme\")"
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
            "NexaStorage.setString(NexaRuntime.context(), \"theme\", \"dark\")"
        );
        assert_eq!(
            expression(&call(
                "delete",
                vec![("key".to_owned(), Expr::String("theme".to_owned()))],
                Type::Void,
            )),
            "NexaStorage.delete(NexaRuntime.context(), \"theme\")"
        );
        assert_eq!(
            expression(&call("clear", Vec::new(), Type::Void)),
            "NexaStorage.clear(NexaRuntime.context())"
        );
    }

    #[test]
    fn haptics_calls_emit_view_feedback_constants() {
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
            "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.LONG_PRESS)"
        );
        assert_eq!(
            expression(&call(
                "notification",
                vec![("kind".to_owned(), Expr::String("Error".to_owned()))],
            )),
            "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.REJECT)"
        );
        assert_eq!(
            expression(&call("selection", Vec::new())),
            "dev.nexa.core.NexaRuntimeCore.performHapticFeedback(android.view.HapticFeedbackConstants.KEYBOARD_TAP)"
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
            expression(&utility(CollectionUtilityKind::First, None, None, false)),
            "listOf(1, 2).firstOrNull()"
        );
        assert_eq!(
            expression(&utility(CollectionUtilityKind::Last, None, None, false)),
            "listOf(1, 2).lastOrNull()"
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
    fn trimmed_string_member_uses_native_trim() {
        let trimmed = Expr::Member {
            base: Box::new(Expr::String(" name ".to_owned())),
            name: "trimmed".to_owned(),
            optional: false,
            base_type: Type::String,
            field_type: Type::String,
            kind: nexa_ir::MemberKind::StringTrimmed,
        };
        assert_eq!(expression(&trimmed), "\" name \".trim()");
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
        assert_eq!(
            expression(&Expr::TimeCall {
                method: TimeMethod::AddCalendarDays,
                arguments: vec![
                    Expr::Number {
                        raw: "1700000000000".to_owned(),
                        ty: NumericType::Int64,
                    },
                    Expr::Number {
                        raw: "2".to_owned(),
                        ty: NumericType::Int32,
                    },
                ],
                return_type: Type::Numeric(NumericType::Int64),
                is_async: false,
            }),
            "nexaAddCalendarDays(1700000000000L, 2)"
        );
        for (method, expected) in [
            (
                TimeMethod::LocalizedDate,
                "java.text.DateFormat.getDateInstance(java.text.DateFormat.SHORT).format(java.util.Date(1700000000000L))",
            ),
            (
                TimeMethod::LocalizedTime,
                "java.text.DateFormat.getTimeInstance(java.text.DateFormat.SHORT).format(java.util.Date(1700000000000L))",
            ),
            (
                TimeMethod::LocalizedDateTime,
                "java.text.DateFormat.getDateTimeInstance(java.text.DateFormat.SHORT, java.text.DateFormat.SHORT).format(java.util.Date(1700000000000L))",
            ),
        ] {
            assert_eq!(
                expression(&Expr::TimeCall {
                    method,
                    arguments: vec![Expr::Number {
                        raw: "1700000000000".to_owned(),
                        ty: NumericType::Int64,
                    }],
                    return_type: Type::String,
                    is_async: false,
                }),
                expected
            );
        }
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

    #[test]
    fn string_membership_uses_native_case_insensitive_search() {
        let search = Expr::Contains {
            value: Box::new(Expr::String("meeting".to_owned())),
            collection: Box::new(Expr::String("Team Meeting".to_owned())),
            collection_type: Type::String,
        };

        assert_eq!(
            expression(&search),
            "\"Team Meeting\".contains(\"meeting\", ignoreCase = true)"
        );
    }

    #[test]
    fn sorted_by_uses_the_native_sorted_by_transform() {
        let sorted = Expr::CollectionTransform {
            operation: CollectionTransform::SortedBy,
            collection: Box::new(Expr::State(
                "values".to_owned(),
                Type::Array(Box::new(Type::Numeric(NumericType::Int32))),
            )),
            initial: None,
            closure: Box::new(Expr::Closure {
                parameters: vec!["value".to_owned()],
                body: Box::new(Expr::State(
                    "value".to_owned(),
                    Type::Numeric(NumericType::Int32),
                )),
            }),
        };
        assert_eq!(
            expression(&sorted),
            "nexa_values.sortedBy { value -> value }"
        );
    }
}
