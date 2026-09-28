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
//! fields in declaration order. Sets and maps are written in a canonical
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
    out.push_str(&format!(
        "fun {name}(reader: {}): {}? {{\n",
        core_type("NexaValueReader"),
        kotlin_type(ty)
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
        _ => vec![format!(
            "val field{index} = {} ?: return null",
            read_expression(ty, reader)
        )],
    }
}

fn read_statements(ty: &Type, reader: &str) -> Vec<String> {
    match ty {
        Type::Bool | Type::Numeric(_) => {
            vec![format!("return {reader}.{}()", scalar_read_method(ty))]
        }
        Type::String => vec![format!("return {reader}.readString()")],
        Type::Bytes => vec![format!("return {reader}.readBuffer()")],
        Type::Enum(name) => vec![format!(
            "return {}.entries.getOrNull({reader}.readInt32() ?: return null)",
            enum_name(name)
        )],
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
            let element_codec = codec_name(element, Direction::Read);
            let mut lines = vec![
                format!("val count = {reader}.readCount() ?: return null"),
                "if (count < 0) {".to_owned(),
                "    return null".to_owned(),
                "}".to_owned(),
                format!("val values = ArrayList<{}>(count)", kotlin_type(element)),
                "repeat(count) {".to_owned(),
                format!("    values.add({element_codec}({reader}) ?: return null)"),
                "}".to_owned(),
            ];
            if matches!(ty, Type::Set(_)) {
                lines.push("return values.toSet()".to_owned());
            } else {
                lines.push("return values".to_owned());
            }
            lines
        }
        Type::Map(key, value_type) => {
            let key_codec = codec_name(key, Direction::Read);
            let value_codec = codec_name(value_type, Direction::Read);
            vec![
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
                format!("    val entryValue = {value_codec}({reader}) ?: return null"),
                "    values[entryKey] = entryValue".to_owned(),
                "}".to_owned(),
                "return values".to_owned(),
            ]
        }
        Type::Pair(first, second) => vec![
            format!(
                "val field0 = {}({reader}) ?: return null",
                codec_name(first, Direction::Read)
            ),
            format!(
                "val field1 = {}({reader}) ?: return null",
                codec_name(second, Direction::Read)
            ),
            "return Pair(field0, field1)".to_owned(),
        ],
        Type::Triple(first, second, third) => vec![
            format!(
                "val field0 = {}({reader}) ?: return null",
                codec_name(first, Direction::Read)
            ),
            format!(
                "val field1 = {}({reader}) ?: return null",
                codec_name(second, Direction::Read)
            ),
            format!(
                "val field2 = {}({reader}) ?: return null",
                codec_name(third, Direction::Read)
            ),
            "return Triple(field0, field1, field2)".to_owned(),
        ],
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
        Type::Bool | Type::Numeric(_) | Type::String | Type::Bytes | Type::Enum(_) => {
            format!("{reader}.{}()", scalar_read_method(ty))
        }
        other => format!("{}({reader})", codec_name(other, Direction::Read)),
    }
}

#[cfg(test)]
mod tests {
    use nexa_ir::{NumericType, Type};

    use super::{read_statements, write_statements};

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
}
