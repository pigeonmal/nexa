//! Generated value codecs for generic plugin calls.
//!
//! The Kotlin mirror of the Swift codec layer: same layout, same canonical
//! ordering, same one-function-per-bound-type mapping. A plugin method with
//! value type parameters receives one codec closure per bound type, so a call
//! site is a direct native call plus a direct call to a concrete function.
//!
//! The layout is fixed: a scalar is its little-endian bytes, a string or
//! buffer is a `UInt32` byte length followed by its bytes, a collection is a
//! `UInt32` element count followed by its elements, and a struct is its
//! fields in declaration order. Results have a one-byte case tag followed
//! by the selected payload. Sets and maps are written in a canonical
//! order - each element encoded, then sorted bytewise - so the same logical
//! value always produces the same bytes, which is what lets a store written
//! on one platform read back on the other.
//!
//! The declarations are emitted into the same file as the app's structs and
//! enums because those are file-private; the writer and reader stay internal
//! so plugin files in the same module can use them.

use nexa_codegen::{
    SourceWriter,
    names::{enum_name, struct_field_name, struct_name},
    value::{Codec, Direction, codec_name, numeric_mangled},
};
use nexa_ir::{NumericType, Type};

use super::types::kotlin_type;

/// Emits one codec function per collected codec. The reader and writer live
/// in the fixed core package, so this file only imports them.
pub(crate) fn render(codecs: &[Codec], out: &mut SourceWriter) {
    if codecs.is_empty() {
        return;
    }
    for codec in codecs {
        match codec.direction {
            Direction::Write => write_function(&codec.ty, out),
            Direction::Read => read_function(&codec.ty, out),
        }
    }
}

// The core runtime lives in its own package, and a generated file cannot add
// an import after its first declaration, so the codec functions name it
// through its package rather than importing it.
fn core_type(name: &str) -> String {
    format!("{}.{name}", nexa_codegen::value::KOTLIN_CORE_PACKAGE)
}

fn pascal(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for character in name.chars() {
        if character == '_' {
            upper = true;
        } else if upper {
            out.extend(character.to_uppercase());
            upper = false;
        } else {
            out.push(character);
        }
    }
    out
}

fn write_method(numeric: NumericType) -> String {
    format!("write{}", pascal(numeric_mangled(numeric)))
}

fn read_method(numeric: NumericType) -> String {
    format!("read{}", pascal(numeric_mangled(numeric)))
}

fn write_function(ty: &Type, out: &mut SourceWriter) {
    let name = codec_name(ty, Direction::Write);
    out.push_str(&format!(
        "fun {name}(value: {}, writer: {}) {{\n",
        kotlin_type(ty),
        core_type("NexaValueWriter")
    ));
    for line in write_statements(ty, "value", "writer") {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("}\n\n");
}

fn read_function(ty: &Type, out: &mut SourceWriter) {
    let name = codec_name(ty, Direction::Read);
    let uses_nullable_result = matches!(ty, Type::Optional(_));
    let visibility = if uses_nullable_result { "private " } else { "" };
    let result_type = if uses_nullable_result {
        format!("{}<{}>", core_type("NexaValueReadResult"), kotlin_type(ty))
    } else {
        format!("{}?", kotlin_type(ty))
    };
    out.push_str(&format!(
        "{visibility}fun {name}(reader: {}): {result_type} {{\n",
        core_type("NexaValueReader"),
    ));
    for line in read_statements(ty, "reader") {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("}\n\n");
}

fn write_statements(ty: &Type, expression: &str, writer: &str) -> Vec<String> {
    match ty {
        Type::Bool => vec![format!("{writer}.writeBool({expression})")],
        Type::String => vec![format!("{writer}.writeString({expression})")],
        Type::Bytes => vec![format!("{writer}.writeBuffer({expression})")],
        Type::Numeric(numeric) => {
            vec![format!("{writer}.{}({expression})", write_method(*numeric))]
        }
        Type::Enum(_) => vec![format!("{writer}.writeInt32({expression}.ordinal)")],
        Type::Result(value_type, error_type) => {
            let value_codec = codec_name(value_type, Direction::Write);
            let error_codec = codec_name(error_type, Direction::Write);
            let value_type = kotlin_type(value_type);
            let error_type = kotlin_type(error_type);
            vec![
                format!("when ({expression}) {{"),
                "    is NexaResult.Success<*> -> {".to_owned(),
                format!("        {writer}.writeBool(true)"),
                format!("        val payload = ({expression}.value as {value_type})"),
                format!("        {value_codec}(payload, {writer})"),
                "    }".to_owned(),
                "    is NexaResult.Failure<*> -> {".to_owned(),
                format!("        {writer}.writeBool(false)"),
                format!("        val failure = ({expression}.error as {error_type})"),
                format!("        {error_codec}(failure, {writer})"),
                "    }".to_owned(),
                "}".to_owned(),
            ]
        }
        Type::Optional(inner) => {
            let inner_codec = codec_name(inner, Direction::Write);
            vec![
                format!("{writer}.writeBool({expression} != null)"),
                format!(
                    "{expression}?.let {{ nexaOptionalValue -> {inner_codec}(nexaOptionalValue, {writer}) }}"
                ),
            ]
        }
        Type::Struct { fields, .. } => {
            let mut lines = Vec::new();
            for (field, field_type) in fields {
                lines.extend(write_statements(
                    field_type,
                    &format!("{expression}.{}", struct_field_name(field)),
                    writer,
                ));
            }
            lines
        }
        Type::Array(element) => {
            let element_codec = codec_name(element, Direction::Write);
            vec![
                format!("{writer}.writeCount({expression}.size)"),
                format!("for (element in {expression}) {{"),
                format!("    {element_codec}(element, writer)"),
                "}".to_owned(),
            ]
        }
        Type::Set(element) => {
            let element_codec = codec_name(element, Direction::Write);
            vec![
                format!("val encoded = ArrayList<ByteArray>({expression}.size)"),
                format!("for (element in {expression}) {{"),
                format!("    val sink = {}()", core_type("NexaValueWriter")),
                format!("    {element_codec}(element, sink)"),
                "    encoded.add(sink.toByteArray())".to_owned(),
                "}".to_owned(),
                format!(
                    "encoded.sortWith {{ left, right -> {}(left, right) }}",
                    core_type("nexaCompareBytes")
                ),
                format!("{writer}.writeCount(encoded.size)"),
                format!("for (entry in encoded) {{ {writer}.writeRaw(entry) }}"),
            ]
        }
        Type::Map(key, value_type) => {
            let key_codec = codec_name(key, Direction::Write);
            let value_codec = codec_name(value_type, Direction::Write);
            vec![
                format!("val encoded = ArrayList<Pair<ByteArray, ByteArray>>({expression}.size)"),
                format!("for ((entryKey, entryValue) in {expression}) {{"),
                format!("    val keySink = {}()", core_type("NexaValueWriter")),
                format!("    val valueSink = {}()", core_type("NexaValueWriter")),
                format!("    {key_codec}(entryKey, keySink)"),
                format!("    {value_codec}(entryValue, valueSink)"),
                "    encoded.add(Pair(keySink.toByteArray(), valueSink.toByteArray()))".to_owned(),
                "}".to_owned(),
                format!(
                    "encoded.sortWith {{ left, right -> {}(left.first, right.first) }}",
                    core_type("nexaCompareBytes")
                ),
                format!("{writer}.writeCount(encoded.size)"),
                format!(
                    "for (entry in encoded) {{ {writer}.writeRaw(entry.first); {writer}.writeRaw(entry.second) }}"
                ),
            ]
        }
        Type::Pair(first, second) => vec![
            format!(
                "{}({expression}.first, writer)",
                codec_name(first, Direction::Write)
            ),
            format!(
                "{}({expression}.second, writer)",
                codec_name(second, Direction::Write)
            ),
        ],
        Type::Triple(first, second, third) => vec![
            format!(
                "{}({expression}.first, writer)",
                codec_name(first, Direction::Write)
            ),
            format!(
                "{}({expression}.second, writer)",
                codec_name(second, Direction::Write)
            ),
            format!(
                "{}({expression}.third, writer)",
                codec_name(third, Direction::Write)
            ),
        ],
        other => vec![format!("// unsupported codec type {other:?}")],
    }
}

/// Statements that bind one value of `ty` to `field{index}`.
///
/// An enum takes two: its ordinal is read first, because the case is looked up
/// from it rather than read directly. An ordinal no case claims means a foreign
/// payload, so the read fails instead of inventing a case.
fn field_read_statements(ty: &Type, index: usize, reader: &str) -> Vec<String> {
    match ty {
        Type::Enum(name) => vec![
            format!("val ordinal{index} = {reader}.readInt32() ?: return null"),
            format!(
                "val field{index} = {}.entries.getOrNull(ordinal{index}) ?: return null",
                enum_name(name)
            ),
        ],
        _ => read_binding_statements(ty, reader, &format!("field{index}"), ""),
    }
}

/// Binds one nested value, keeping an optional value separate from a failed
/// decode. Kotlin otherwise flattens both cases to the same nullable value.
fn read_binding_statements(ty: &Type, reader: &str, binding: &str, indent: &str) -> Vec<String> {
    let expression = read_expression(ty, reader);
    if matches!(ty, Type::Optional(_)) {
        let decoded = format!("nexaDecoded_{binding}");
        vec![
            format!("{indent}val {decoded} = {expression}"),
            format!(
                "{indent}if ({decoded} !is {}.Value<*>) return null",
                core_type("NexaValueReadResult")
            ),
            format!(
                "{indent}val {binding} = {decoded}.value as {}",
                kotlin_type(ty)
            ),
        ]
    } else {
        vec![format!(
            "{indent}val {binding} = {expression} ?: return null"
        )]
    }
}

fn read_statements(ty: &Type, reader: &str) -> Vec<String> {
    match ty {
        Type::Bool | Type::Numeric(_) => {
            vec![format!("return {reader}.{}()", scalar_read_method(ty))]
        }
        Type::String => vec![format!("return {reader}.readString()")],
        Type::Bytes => vec![format!("return {reader}.readBuffer()")],
        Type::Optional(inner) => {
            let inner = read_expression(inner, reader);
            let result = core_type("NexaValueReadResult");
            vec![
                format!("val present = {reader}.readBool() ?: return {result}.Invalid"),
                format!("if (!present) return {result}.Value(null)"),
                format!("val value = {inner} ?: return {result}.Invalid"),
                format!("return {result}.Value(value)"),
            ]
        }
        Type::Enum(name) => vec![format!(
            "return {}.entries.getOrNull({reader}.readInt32() ?: return null)",
            enum_name(name)
        )],
        Type::Result(value_type, error_type) => {
            let mut lines = vec![
                format!("val isSuccess = {reader}.readBool() ?: return null"),
                "if (isSuccess) {".to_owned(),
            ];
            lines.extend(read_binding_statements(
                value_type, reader, "payload", "    ",
            ));
            lines.push("    return NexaResult.Success(payload)".to_owned());
            lines.push("}".to_owned());
            lines.push(format!(
                "val failure = {}({reader}) ?: return null",
                codec_name(error_type, Direction::Read)
            ));
            lines.push("return NexaResult.Failure(failure)".to_owned());
            lines
        }
        Type::Struct { name, fields } => {
            let mut lines = Vec::new();
            for (index, (_, field_type)) in fields.iter().enumerate() {
                lines.extend(field_read_statements(field_type, index, reader));
            }
            let bindings = (0..fields.len())
                .map(|index| format!("field{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("return {}({bindings})", struct_name(name)));
            lines
        }
        Type::Array(element) | Type::Set(element) => {
            let mut lines = vec![
                format!("val count = {reader}.readCount() ?: return null"),
                "if (count < 0) {".to_owned(),
                "    return null".to_owned(),
                "}".to_owned(),
                format!("val values = ArrayList<{}>(count)", kotlin_type(element)),
                "repeat(count) {".to_owned(),
            ];
            lines.extend(read_binding_statements(element, reader, "element", "    "));
            lines.push("    values.add(element)".to_owned());
            lines.push("}".to_owned());
            if matches!(ty, Type::Set(_)) {
                lines.push("return values.toSet()".to_owned());
            } else {
                lines.push("return values".to_owned());
            }
            lines
        }
        Type::Map(key, value_type) => {
            let key_codec = codec_name(key, Direction::Read);
            let mut lines = vec![
                format!("val count = {reader}.readCount() ?: return null"),
                "if (count < 0) {".to_owned(),
                "    return null".to_owned(),
                "}".to_owned(),
                format!(
                    "val values = LinkedHashMap<{}, {}>(count)",
                    kotlin_type(key),
                    kotlin_type(value_type)
                ),
                "repeat(count) {".to_owned(),
                format!("    val entryKey = {key_codec}({reader}) ?: return null"),
            ];
            lines.extend(read_binding_statements(
                value_type,
                reader,
                "entryValue",
                "    ",
            ));
            lines.push("    values[entryKey] = entryValue".to_owned());
            lines.push("}".to_owned());
            lines.push("return values".to_owned());
            lines
        }
        Type::Pair(first, second) => {
            let mut lines = Vec::new();
            lines.extend(read_binding_statements(first, reader, "field0", ""));
            lines.extend(read_binding_statements(second, reader, "field1", ""));
            lines.push("return Pair(field0, field1)".to_owned());
            lines
        }
        Type::Triple(first, second, third) => {
            let mut lines = Vec::new();
            lines.extend(read_binding_statements(first, reader, "field0", ""));
            lines.extend(read_binding_statements(second, reader, "field1", ""));
            lines.extend(read_binding_statements(third, reader, "field2", ""));
            lines.push("return Triple(field0, field1, field2)".to_owned());
            lines
        }
        other => vec![format!("// unsupported codec type {other:?}")],
    }
}

fn scalar_read_method(ty: &Type) -> String {
    match ty {
        Type::Bool => "readBool".to_owned(),
        Type::Numeric(numeric) => read_method(*numeric),
        Type::String => "readString".to_owned(),
        Type::Bytes => "readBuffer".to_owned(),
        other => format!("// unsupported scalar {other:?}"),
    }
}

fn read_expression(ty: &Type, reader: &str) -> String {
    match ty {
        Type::Bool | Type::Numeric(_) | Type::String | Type::Bytes => {
            format!("{reader}.{}()", scalar_read_method(ty))
        }
        other => format!("{}({reader})", codec_name(other, Direction::Read)),
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::{
        SourceWriter,
        value::{Codec, Direction, codec_name},
    };
    use nexa_ir::{NumericType, Type};

    use super::{read_statements, render, write_statements};

    #[test]
    fn pair_and_triple_codecs_write_and_read_members_in_order() {
        let pair = Type::Pair(
            Box::new(Type::String),
            Box::new(Type::Numeric(NumericType::Int32)),
        );
        let pair_write = write_statements(&pair, "value", "writer").join("\n");
        let pair_read = read_statements(&pair, "reader").join("\n");
        assert!(pair_write.contains("nexaWritestring(value.first, writer)"));
        assert!(pair_write.contains("nexaWriteint32(value.second, writer)"));
        assert!(pair_read.contains("return Pair(field0, field1)"));

        let triple = Type::Triple(
            Box::new(Type::String),
            Box::new(Type::Numeric(NumericType::Int32)),
            Box::new(Type::Bool),
        );
        let triple_write = write_statements(&triple, "value", "writer").join("\n");
        let triple_read = read_statements(&triple, "reader").join("\n");
        assert!(triple_write.contains("nexaWritestring(value.first, writer)"));
        assert!(triple_write.contains("nexaWriteint32(value.second, writer)"));
        assert!(triple_write.contains("nexaWritebool(value.third, writer)"));
        assert!(triple_read.contains("return Triple(field0, field1, field2)"));
    }

    #[test]
    fn result_codec_tags_success_and_failure_payloads() {
        let result = Type::Result(
            Box::new(Type::Numeric(NumericType::Float64)),
            Box::new(Type::Enum("StoreError".to_owned())),
        );
        let write = write_statements(&result, "value", "writer").join("\n");
        let read = read_statements(&result, "reader").join("\n");

        assert!(write.contains("is NexaResult.Success<*>"));
        assert!(write.contains("writer.writeBool(true)"));
        assert!(write.contains("is NexaResult.Failure<*>"));
        assert!(write.contains("writer.writeBool(false)"));
        assert!(read.contains("return NexaResult.Success(payload)"));
        assert!(read.contains("return NexaResult.Failure(failure)"));
    }

    #[test]
    fn optional_writer_emits_a_presence_tag_before_its_value() {
        let optional = Type::Optional(Box::new(Type::String));
        let write = write_statements(&optional, "value", "writer").join("\n");

        assert!(write.contains("writer.writeBool(value != null)"));
        assert!(write.contains(
            "value?.let { nexaOptionalValue -> nexaWritestring(nexaOptionalValue, writer) }"
        ));
    }

    #[test]
    fn optional_compound_reads_keep_null_values_separate_from_decode_failure() {
        let optional = Type::Optional(Box::new(Type::String));
        let optional_read = read_statements(&optional, "reader").join("\n");
        assert!(optional_read.contains("return dev.nexa.core.NexaValueReadResult.Value(null)"));
        assert!(optional_read.contains("return dev.nexa.core.NexaValueReadResult.Invalid"));

        let array = Type::Array(Box::new(optional));
        let array_read = read_statements(&array, "reader").join("\n");
        assert!(array_read.contains("is dev.nexa.core.NexaValueReadResult.Value<*>"));
        assert!(array_read.contains("values.add(element)"));

        let array = Type::Array(Box::new(Type::Optional(Box::new(Type::String))));
        let optional = match &array {
            Type::Array(element) => element.as_ref().clone(),
            _ => unreachable!(),
        };
        let codecs = [
            Codec {
                ty: optional.clone(),
                direction: Direction::Read,
            },
            Codec {
                ty: array,
                direction: Direction::Read,
            },
        ];
        let mut output = SourceWriter::new();
        render(&codecs, &mut output);
        let generated = output.as_str();
        assert!(!generated.contains("private sealed class NexaValueReadResult"));
        assert!(generated.contains(&format!(
            "private fun {}(reader: dev.nexa.core.NexaValueReader): dev.nexa.core.NexaValueReadResult<String?>",
            codec_name(&optional, Direction::Read)
        )));
        assert!(generated.contains(&format!(
            "fun {}(reader: dev.nexa.core.NexaValueReader): List<String?>?",
            codec_name(&codecs[1].ty, Direction::Read)
        )));

        let result = Type::Result(
            Box::new(Type::Optional(Box::new(Type::String))),
            Box::new(Type::Enum("StoreError".to_owned())),
        );
        let result_read = read_statements(&result, "reader").join("\n");
        assert!(result_read.contains("val nexaDecoded_payload = nexaReadoptional_string(reader)"));
        assert!(result_read.contains("val payload = nexaDecoded_payload.value as String?"));
        assert!(result_read.contains("return NexaResult.Success(payload)"));
    }
}
