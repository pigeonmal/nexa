//! Type-specialized JSON readers and writers for Android.
//!
//! Generated codecs use `JsonReader` and direct typed constructors. Parsing
//! does not build an untyped object tree or use reflection; only the source
//! JSON buffer and values requested by the app are materialized.

use nexa_codegen::SourceWriter;
use nexa_ir::{EnumDecl, NumericType, Type};

use crate::generator::engine::{features::Features, imports::ImportSet, types::kotlin_type};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    if features.facts.capabilities.uses_json_api {
        imports.add(true, "android.util.JsonReader");
        imports.add(true, "android.util.JsonToken");
        imports.add(true, "java.io.StringReader");
    }
}

pub(crate) fn render(types: &[Type], enums: &[EnumDecl], out: &mut SourceWriter) {
    out.push_str(RUNTIME);
    out.push('\n');
    for ty in types {
        render_type_codec(ty, enums, out);
    }
}

fn render_type_codec(ty: &Type, enums: &[EnumDecl], out: &mut SourceWriter) {
    let decoder = nexa_codegen::value::json_decode_name(ty);
    let encoder = nexa_codegen::value::json_encode_name(ty);
    let native = kotlin_type(ty);
    out.push_str(&format!(
        "private fun {decoder}(reader: JsonReader): {native} {{\n"
    ));
    render_decode_body(ty, enums, out);
    out.push_str("}\n\n");
    out.push_str(&format!(
        "private fun {encoder}(value: {native}, output: StringBuilder) {{\n"
    ));
    render_encode_body(ty, out);
    out.push_str("}\n\n");
    render_api_wrappers(ty, out);
}

fn render_decode_body(ty: &Type, enums: &[EnumDecl], out: &mut SourceWriter) {
    match ty {
        Type::String => out.push_str(
            "    if (reader.peek() != JsonToken.STRING) nexaJsonTypeMismatch()\n    return reader.nextString()\n",
        ),
        Type::Bool => out.push_str(
            "    if (reader.peek() != JsonToken.BOOLEAN) nexaJsonTypeMismatch()\n    return reader.nextBoolean()\n",
        ),
        Type::Numeric(numeric) => render_numeric_decode(*numeric, out),
        Type::Bytes => out.push_str(
            "    if (reader.peek() != JsonToken.STRING) nexaJsonTypeMismatch()\n    val encoded = reader.nextString()\n    return try { android.util.Base64.decode(encoded, android.util.Base64.DEFAULT) } catch (_: IllegalArgumentException) { nexaJsonInvalidValue() }\n",
        ),
        Type::Optional(inner) => {
            out.push_str(
                "    if (reader.peek() == JsonToken.NULL) { reader.nextNull(); return null }\n",
            );
            out.push_str(&format!(
                "    return {}(reader)\n",
                nexa_codegen::value::json_decode_name(inner)
            ));
        }
        Type::Array(element) | Type::Set(element) => {
            out.push_str(
                "    if (reader.peek() != JsonToken.BEGIN_ARRAY) nexaJsonTypeMismatch()\n    reader.beginArray()\n",
            );
            let is_set = matches!(ty, Type::Set(_));
            if is_set {
                out.push_str(&format!(
                    "    val values = LinkedHashSet<{}>()\n",
                    kotlin_type(element)
                ));
            } else {
                out.push_str(&format!(
                    "    val values = ArrayList<{}>()\n",
                    kotlin_type(element)
                ));
            }
            out.push_str("    while (reader.hasNext()) {\n");
            out.push_str(&format!(
                "        values.add({}(reader))\n",
                nexa_codegen::value::json_decode_name(element)
            ));
            out.push_str("    }\n    reader.endArray()\n    return values\n");
        }
        Type::Map(key, value) if matches!(key.as_ref(), Type::String) => {
            out.push_str(
                "    if (reader.peek() != JsonToken.BEGIN_OBJECT) nexaJsonTypeMismatch()\n    reader.beginObject()\n",
            );
            out.push_str(&format!(
                "    val values = LinkedHashMap<String, {}>()\n",
                kotlin_type(value)
            ));
            out.push_str("    while (reader.hasNext()) {\n        val key = reader.nextName()\n");
            out.push_str(&format!(
                "        values[key] = {}(reader)\n",
                nexa_codegen::value::json_decode_name(value)
            ));
            out.push_str("    }\n    reader.endObject()\n    return values\n");
        }
        Type::Pair(first, second) => {
            out.push_str("    if (reader.peek() != JsonToken.BEGIN_ARRAY) nexaJsonTypeMismatch()\n    reader.beginArray()\n");
            out.push_str(&format!(
                "    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val first = {}(reader)\n    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val second = {}(reader)\n    if (reader.hasNext()) nexaJsonInvalidValue()\n    reader.endArray()\n    return Pair(first, second)\n",
                nexa_codegen::value::json_decode_name(first),
                nexa_codegen::value::json_decode_name(second)
            ));
        }
        Type::Triple(first, second, third) => {
            out.push_str("    if (reader.peek() != JsonToken.BEGIN_ARRAY) nexaJsonTypeMismatch()\n    reader.beginArray()\n");
            out.push_str(&format!(
                "    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val first = {}(reader)\n    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val second = {}(reader)\n    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val third = {}(reader)\n    if (reader.hasNext()) nexaJsonInvalidValue()\n    reader.endArray()\n    return Triple(first, second, third)\n",
                nexa_codegen::value::json_decode_name(first),
                nexa_codegen::value::json_decode_name(second),
                nexa_codegen::value::json_decode_name(third)
            ));
        }
        Type::Result(success, failure) => {
            out.push_str("    if (reader.peek() != JsonToken.BEGIN_OBJECT) nexaJsonTypeMismatch()\n    reader.beginObject()\n    if (!reader.hasNext()) nexaJsonInvalidValue()\n    val caseName = reader.nextName()\n    val result = when (caseName) {\n");
            out.push_str(&format!(
                "        \"success\" -> NexaResult.Success({}(reader))\n        \"failure\" -> NexaResult.Failure({}(reader))\n",
                nexa_codegen::value::json_decode_name(success),
                nexa_codegen::value::json_decode_name(failure)
            ));
            out.push_str("        else -> nexaJsonInvalidValue()\n    }\n    if (reader.hasNext()) nexaJsonInvalidValue()\n    reader.endObject()\n    return result\n");
        }
        Type::Enum(name) => {
            let native_name = nexa_codegen::names::enum_name(name);
            out.push_str("    if (reader.peek() != JsonToken.STRING) nexaJsonTypeMismatch()\n    return when (val caseName = reader.nextString()) {\n");
            if let Some(declaration) = enums.iter().find(|declaration| declaration.name == *name) {
                for case in &declaration.cases {
                    out.push_str(&format!("        \"{case}\" -> {native_name}.{case}\n"));
                }
            }
            out.push_str("        else -> nexaJsonInvalidValue()\n    }\n");
        }
        Type::Struct { name, fields } => render_struct_decode(name, fields, out),
        Type::Void
        | Type::TypeParam(_)
        | Type::Plugin { .. }
        | Type::Class { .. }
        | Type::TaskHandle
        | Type::NetworkResponse
        | Type::Signal(_) => {
            debug_assert!(false, "unsupported JSON value type reached Kotlin codegen: {ty:?}");
            out.push_str("    nexaJsonTypeMismatch()\n");
        }
        Type::Map(_, _) => {
            debug_assert!(false, "non-string JSON map key reached Kotlin codegen");
            out.push_str("    nexaJsonTypeMismatch()\n");
        }
    }
}

fn render_numeric_decode(numeric: NumericType, out: &mut SourceWriter) {
    let is_float = matches!(numeric, NumericType::Float32 | NumericType::Float64);
    if is_float {
        out.push_str("    val token = reader.peek()\n    if (token != JsonToken.NUMBER && token != JsonToken.STRING) nexaJsonTypeMismatch()\n    val raw = reader.nextString()\n");
        match numeric {
            NumericType::Float32 => out.push_str(
                "    return when (raw) { \"Infinity\" -> Float.POSITIVE_INFINITY; \"-Infinity\" -> Float.NEGATIVE_INFINITY; \"NaN\" -> Float.NaN; else -> raw.toFloatOrNull() ?: nexaJsonInvalidValue() }\n",
            ),
            NumericType::Float64 => out.push_str(
                "    return when (raw) { \"Infinity\" -> Double.POSITIVE_INFINITY; \"-Infinity\" -> Double.NEGATIVE_INFINITY; \"NaN\" -> Double.NaN; else -> raw.toDoubleOrNull() ?: nexaJsonInvalidValue() }\n",
            ),
            _ => {}
        }
        return;
    }
    out.push_str("    if (reader.peek() != JsonToken.NUMBER) nexaJsonTypeMismatch()\n    val raw = reader.nextString()\n");
    let parser = match numeric {
        NumericType::Int8 => "toByteOrNull",
        NumericType::Int16 => "toShortOrNull",
        NumericType::Int32 => "toIntOrNull",
        NumericType::Int64 => "toLongOrNull",
        NumericType::UInt8 => "toUByteOrNull",
        NumericType::UInt16 => "toUShortOrNull",
        NumericType::UInt32 => "toUIntOrNull",
        NumericType::UInt64 => "toULongOrNull",
        NumericType::Float32 | NumericType::Float64 => return,
    };
    out.push_str(&format!(
        "    return raw.{parser}() ?: nexaJsonInvalidValue()\n"
    ));
}

fn render_struct_decode(name: &str, fields: &[(String, Type)], out: &mut SourceWriter) {
    let native_name = nexa_codegen::names::struct_name(name);
    out.push_str("    if (reader.peek() != JsonToken.BEGIN_OBJECT) nexaJsonTypeMismatch()\n    reader.beginObject()\n");
    for (index, (_, ty)) in fields.iter().enumerate() {
        out.push_str(&format!(
            "    var field{index}: NexaJsonFieldSlot<{}> = NexaJsonFieldSlot.Missing\n",
            kotlin_type(ty)
        ));
    }
    out.push_str("    while (reader.hasNext()) {\n        when (reader.nextName()) {\n");
    for (index, (field, ty)) in fields.iter().enumerate() {
        out.push_str(&format!(
            "            \"{field}\" -> field{index} = NexaJsonFieldSlot.Present({}(reader))\n",
            nexa_codegen::value::json_decode_name(ty)
        ));
    }
    out.push_str(
        "            else -> reader.skipValue()\n        }\n    }\n    reader.endObject()\n",
    );
    for (index, (field, ty)) in fields.iter().enumerate() {
        let value = if matches!(ty, Type::Optional(_)) {
            format!(
                "when (val slot = field{index}) {{ is NexaJsonFieldSlot.Present -> slot.value; NexaJsonFieldSlot.Missing -> null }}"
            )
        } else {
            format!(
                "when (val slot = field{index}) {{ is NexaJsonFieldSlot.Present -> slot.value; NexaJsonFieldSlot.Missing -> nexaJsonMissingField(\"{field}\") }}"
            )
        };
        out.push_str(&format!(
            "    val value{index}: {} = {value}\n",
            kotlin_type(ty)
        ));
    }
    let values = (0..fields.len())
        .map(|index| format!("value{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("    return {native_name}({values})\n"));
}

fn render_encode_body(ty: &Type, out: &mut SourceWriter) {
    match ty {
        Type::String | Type::Enum(_) => {
            let raw = if matches!(ty, Type::Enum(_)) {
                "value.name"
            } else {
                "value"
            };
            out.push_str(&format!("    nexaJsonAppendQuoted({raw}, output)\n"));
        }
        Type::Bool => out.push_str("    output.append(if (value) \"true\" else \"false\")\n"),
        Type::Numeric(NumericType::Float32) => out.push_str("    if (value.isFinite()) output.append(value.toString()) else nexaJsonAppendQuoted(if (value.isNaN()) \"NaN\" else if (value < 0) \"-Infinity\" else \"Infinity\", output)\n"),
        Type::Numeric(NumericType::Float64) => out.push_str("    if (value.isFinite()) output.append(value.toString()) else nexaJsonAppendQuoted(if (value.isNaN()) \"NaN\" else if (value < 0) \"-Infinity\" else \"Infinity\", output)\n"),
        Type::Numeric(_) => out.push_str("    output.append(value.toString())\n"),
        Type::Bytes => out.push_str("    nexaJsonAppendQuoted(android.util.Base64.encodeToString(value, android.util.Base64.NO_WRAP), output)\n"),
        Type::Optional(inner) => {
            out.push_str("    if (value == null) { output.append(\"null\"); return }\n");
            out.push_str(&format!(
                "    {}(value, output)\n",
                nexa_codegen::value::json_encode_name(inner)
            ));
        }
        Type::Array(element) | Type::Set(element) => {
            let encoder = nexa_codegen::value::json_encode_name(element);
            out.push_str("    output.append('[')\n    var first = true\n");
            if matches!(ty, Type::Set(_)) {
                out.push_str(&format!(
                    "    val encoded = value.map {{ item -> val itemOutput = StringBuilder(); {encoder}(item, itemOutput); itemOutput.toString() }}.sorted()\n    for (item in encoded) {{ if (!first) output.append(','); first = false; output.append(item) }}\n"
                ));
            } else {
                out.push_str(&format!(
                    "    for (item in value) {{ if (!first) output.append(','); first = false; {encoder}(item, output) }}\n"
                ));
            }
            out.push_str("    output.append(']')\n");
        }
        Type::Map(_, value_type) => {
            out.push_str(&format!("    output.append('{{')\n    var first = true\n    for (key in value.keys.sorted()) {{\n        if (!first) output.append(',')\n        first = false\n        nexaJsonAppendQuoted(key, output)\n        output.append(':')\n        {}(value.getValue(key), output)\n    }}\n    output.append('}}')\n", nexa_codegen::value::json_encode_name(value_type)));
        }
        Type::Pair(first, second) => {
            out.push_str("    output.append('[')\n");
            out.push_str(&format!(
                "    {}(value.first, output)\n    output.append(',')\n    {}(value.second, output)\n    output.append(']')\n",
                nexa_codegen::value::json_encode_name(first),
                nexa_codegen::value::json_encode_name(second)
            ));
        }
        Type::Triple(first, second, third) => {
            out.push_str("    output.append('[')\n");
            out.push_str(&format!(
                "    {}(value.first, output)\n    output.append(',')\n    {}(value.second, output)\n    output.append(',')\n    {}(value.third, output)\n    output.append(']')\n",
                nexa_codegen::value::json_encode_name(first),
                nexa_codegen::value::json_encode_name(second),
                nexa_codegen::value::json_encode_name(third)
            ));
        }
        Type::Result(success, failure) => {
            let success_code = nexa_codegen::value::json_encode_name(success);
            let failure_code = nexa_codegen::value::json_encode_name(failure);
            out.push_str(&format!(
                "    output.append('{{')\n    when (value) {{\n        is NexaResult.Success<*> -> {{\n            nexaJsonAppendQuoted(\"success\", output)\n            output.append(':')\n            @Suppress(\"UNCHECKED_CAST\")\n            val item = value.value as {}\n            {success_code}(item, output)\n        }}\n        is NexaResult.Failure<*> -> {{\n            nexaJsonAppendQuoted(\"failure\", output)\n            output.append(':')\n            @Suppress(\"UNCHECKED_CAST\")\n            val item = value.error as {}\n            {failure_code}(item, output)\n        }}\n    }}\n    output.append('}}')\n",
                kotlin_type(success),
                kotlin_type(failure)
            ));
        }
        Type::Struct { name: _, fields } => {
            out.push_str("    output.append('{')\n");
            for (index, (field, ty)) in fields.iter().enumerate() {
                if index > 0 {
                    out.push_str("    output.append(',')\n");
                }
                out.push_str(&format!(
                    "    nexaJsonAppendQuoted(\"{field}\", output)\n    output.append(':')\n    {}(value.{}, output)\n",
                    nexa_codegen::value::json_encode_name(ty),
                    nexa_codegen::names::struct_field_name(field)
                ));
            }
            out.push_str("    output.append('}')\n");
        }
        Type::Void
        | Type::TypeParam(_)
        | Type::Plugin { .. }
        | Type::Class { .. }
        | Type::TaskHandle
        | Type::NetworkResponse
        | Type::Signal(_) => {
            debug_assert!(false, "unsupported JSON value type reached Kotlin codegen: {ty:?}");
            out.push_str("    output.append(\"null\")\n");
        }
    }
}

fn render_api_wrappers(ty: &Type, out: &mut SourceWriter) {
    let suffix = nexa_codegen::value::json_parse_name(ty)
        .strip_prefix("nexaJsonParse_")
        .unwrap_or("unsupported")
        .to_owned();
    let native = kotlin_type(ty);
    out.push_str(&format!(
        "internal fun nexaJsonParse_{suffix}(raw: String): NexaResult<{native}, NexaJsonError> {{\n    return try {{\n        val reader = JsonReader(StringReader(raw))\n        reader.isLenient = false\n        val value = {}(reader)\n        if (reader.peek() != JsonToken.END_DOCUMENT) nexaJsonInvalidValue()\n        NexaResult.Success(value)\n    }} catch (failure: NexaJsonFailure) {{\n        NexaResult.Failure(failure.error)\n    }} catch (_: java.io.IOException) {{\n        NexaResult.Failure(NexaJsonError.invalidJson)\n    }} catch (_: IllegalStateException) {{\n        NexaResult.Failure(NexaJsonError.invalidJson)\n    }}\n}}\n\n",
        nexa_codegen::value::json_decode_name(ty)
    ));
    out.push_str(&format!(
        "internal fun nexaJsonStringify_{suffix}(value: {native}): String {{\n    val output = StringBuilder()\n    {}(value, output)\n    return output.toString()\n}}\n\n",
        nexa_codegen::value::json_encode_name(ty)
    ));
}

const RUNTIME: &str = r#"// Statically typed JSON helpers.

private class NexaJsonFailure(val error: NexaJsonError) : RuntimeException(null, null, false, false)

private fun nexaJsonTypeMismatch(): Nothing = throw NexaJsonFailure(NexaJsonError.typeMismatch)
private fun nexaJsonInvalidValue(): Nothing = throw NexaJsonFailure(NexaJsonError.invalidValue)
private fun nexaJsonMissingField(name: String): Nothing = throw NexaJsonFailure(NexaJsonError.missingField)

private sealed class NexaJsonFieldSlot<out Value> {
    object Missing : NexaJsonFieldSlot<Nothing>()
    data class Present<out Value>(val value: Value) : NexaJsonFieldSlot<Value>()
}

private fun nexaJsonAppendQuoted(value: String, output: StringBuilder) {
    output.append('"')
    for (character in value) {
        when (character) {
            '"' -> output.append("\\\"")
            '\\' -> output.append("\\\\")
            '\b' -> output.append("\\b")
            '\u000C' -> output.append("\\f")
            '\n' -> output.append("\\n")
            '\r' -> output.append("\\r")
            '\t' -> output.append("\\t")
            else -> if (character.code < 0x20 || character in '\uD800'..'\uDFFF') {
                output.append("\\u")
                output.append(character.code.toString(16).padStart(4, '0'))
            } else output.append(character)
        }
    }
    output.append('"')
}
"#;

#[cfg(test)]
mod tests {
    use super::{render, render_type_codec};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{EnumDecl, NumericType, Type};

    #[test]
    fn emits_typed_streaming_codecs_for_models_and_collections() {
        let profile = Type::Struct {
            name: "Profile".to_owned(),
            fields: vec![
                ("name".to_owned(), Type::String),
                (
                    "scores".to_owned(),
                    Type::Array(Box::new(Type::Numeric(NumericType::Int32))),
                ),
            ],
        };
        let enums = vec![EnumDecl {
            name: "LoadState".to_owned(),
            cases: vec!["idle".to_owned(), "ready".to_owned()],
        }];
        let mut output = SourceWriter::new();
        render_type_codec(&Type::Numeric(NumericType::Int32), &enums, &mut output);
        render_type_codec(
            &Type::Array(Box::new(Type::Numeric(NumericType::Int32))),
            &enums,
            &mut output,
        );
        render_type_codec(&profile, &enums, &mut output);
        render_type_codec(&Type::Enum("LoadState".to_owned()), &enums, &mut output);
        let generated = output.finish();
        assert!(generated.contains("raw.toIntOrNull()"));
        assert!(generated.contains("values.add(nexaJsonDecode_int32(reader))"));
        assert!(generated.contains("NexaJsonFieldSlot.Present"));
        assert!(generated.contains("JsonToken.BEGIN_OBJECT"));
        assert!(generated.contains("LoadState.idle"));
    }

    #[test]
    fn emits_json_parse_error_results_and_stringify_helpers() {
        let mut output = SourceWriter::new();
        render(&[Type::String], &[], &mut output);
        let generated = output.finish();
        assert!(generated.contains("NexaJsonError.invalidJson"));
        assert!(generated.contains("NexaResult<String, NexaJsonError>"));
        assert!(generated.contains("reader.isLenient = false"));
        assert!(generated.contains("private fun nexaJsonAppendQuoted"));
    }
}
