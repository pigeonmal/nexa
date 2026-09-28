//! Generated value codecs for generic plugin calls.
//!
//! A plugin method with value type parameters receives one codec closure per
//! bound type. The closures are generated here, once per type the module
//! actually uses, so a call site stays a direct native call plus a direct call
//! to a concrete function: no reflection, no type strings, and nothing left
//! generic in the emitted Swift.
//!
//! The layout is fixed: a scalar is its little-endian bytes, a string or
//! buffer is a `UInt32` byte length followed by its bytes, a collection is a
//! `UInt32` element count followed by its elements, and a struct is its
//! fields in declaration order. Sets and maps are written in a canonical
//! order - each element encoded, then sorted bytewise - so the same logical
//! value always produces the same bytes, which is what lets a store written
//! on one platform read back on the other.
//!
//! These declarations are emitted into the same file as the app's structs and
//! enums because those are file-private; the writer and reader stay internal
//! so plugin files in the same module can use them.

use nexa_codegen::{
    SourceWriter,
    names::{enum_name, struct_field_name, struct_name},
    value::{Codec, Direction, codec_name, numeric_mangled},
};
use nexa_ir::{NumericType, Type};

use super::types::swift_type;

/// Emits the codec runtime and one function per collected codec.
pub(crate) fn render(codecs: &[Codec], out: &mut SourceWriter) {
    if codecs.is_empty() {
        return;
    }
    runtime(out);
    render_codecs(codecs, out);
}

/// Emits the debug host's dynamic codec runtime even when the current module
/// has no generic plugin call. A later hot reload can add one without adding
/// new native source files or rebuilding the host.
pub(crate) fn render_for_dev(codecs: &[Codec], out: &mut SourceWriter) {
    render_with_runtime(codecs, out);
}

/// Emits the codec runtime for a native plugin contract and any call-site
/// codecs collected from the app.
pub(crate) fn render_with_runtime(codecs: &[Codec], out: &mut SourceWriter) {
    runtime(out);
    render_codecs(codecs, out);
}

fn render_codecs(codecs: &[Codec], out: &mut SourceWriter) {
    for codec in codecs {
        match codec.direction {
            Direction::Write => write_function(&codec.ty, out),
            Direction::Read => read_function(&codec.ty, out),
        }
    }
}

/// The writer and reader both the generated functions and the plugin use.
fn runtime(out: &mut SourceWriter) {
    out.push_str(
        r#"// MARK: - Plugin value codecs

/// Append-only little-endian writer for a plugin value codec.
///
/// A value is encoded into one contiguous buffer, so a store receives a
/// complete payload with no intermediate dictionaries.
public final class NexaValueWriter {
    public private(set) var data = Data()

    public func writeRaw(_ value: Data) {
        data.append(value)
    }

    public func writeCount(_ count: Int) {
        withUnsafeBytes(of: UInt32(count).littleEndian) { data.append(contentsOf: $0) }
    }

    public func writeBool(_ value: Bool) {
        data.append(value ? 1 : 0)
    }

    public func writeString(_ value: String) {
        let utf8 = Array(value.utf8)
        writeCount(utf8.count)
        data.append(contentsOf: utf8)
    }

    public func writeBuffer(_ value: Data) {
        writeCount(value.count)
        data.append(value)
    }
"#,
    );
    for numeric in NUMERICS {
        // A float's payload is its IEEE-754 bit pattern, so it goes through
        // the integer writer of the same width, reinterpreting the bits.
        let body = match swift_bits_writer(numeric) {
            Some(writer) => format!(
                "{writer}({}(bitPattern: value.bitPattern))",
                swift_signed_bits_type(numeric)
            ),
            None => "var little = value.littleEndian\n        withUnsafeBytes(of: &little) { data.append(contentsOf: $0) }"
                .to_owned(),
        };
        out.push_str(&format!(
            "\n    public func {method}(_ value: {value}) {{\n        {body}\n    }}\n",
            method = swift_write_method(numeric),
            value = swift_numeric_type(numeric),
        ));
    }
    out.push_str(
        r#"}
/// Bounds-checked reader over an encoded value.
///
/// Every read either advances past a complete value or reports `nil`, so a
/// truncated or foreign payload can never read out of bounds.
public final class NexaValueReader {
    private let data: Data
    private var offset = 0

    public init(_ data: Data) {
        self.data = data
    }

    /// Reads a fixed-width value straight out of the payload. The bytes are
    /// never copied: a decoded struct costs one allocation for its collections,
    /// not one per field.
    private func readFixedWidth<T: FixedWidthInteger>() -> T? {
        let size = MemoryLayout<T>.size
        guard offset + size <= data.count else { return nil }
        defer { offset += size }
        return data.withUnsafeBytes { raw in
            T(littleEndian: raw.loadUnaligned(fromByteOffset: offset, as: T.self))
        }
    }

    private func readSlice(_ count: Int) -> Data? {
        guard count >= 0, offset + count <= data.count else { return nil }
        defer { offset += count }
        return data.subdata(in: offset..<(offset + count))
    }

    public func readCount() -> Int? {
        guard let value: UInt32 = readFixedWidth() else { return nil }
        return Int(value)
    }

    public func readBool() -> Bool? {
        guard let raw: UInt8 = readFixedWidth(), raw <= 1 else { return nil }
        return raw == 1
    }

    public func readString() -> String? {
        guard let count = readCount(), let raw = readSlice(count) else { return nil }
        return String(decoding: raw, as: UTF8.self)
    }

    public func readBuffer() -> Data? {
        guard let count = readCount() else { return nil }
        return readSlice(count)
    }
"#,
    );
    for numeric in NUMERICS {
        out.push_str(&format!(
            r#"
    public func {}() -> {}? {{
        guard let bits: {} = readFixedWidth() else {{ return nil }}
        return {}
    }}
"#,
            swift_read_method(numeric),
            swift_numeric_type(numeric),
            swift_bits_type(numeric),
            swift_from_bits(numeric)
        ));
    }
    out.push_str("}\n\n");
}

const NUMERICS: [NumericType; 10] = [
    NumericType::Int8,
    NumericType::Int16,
    NumericType::Int32,
    NumericType::Int64,
    NumericType::UInt8,
    NumericType::UInt16,
    NumericType::UInt32,
    NumericType::UInt64,
    NumericType::Float32,
    NumericType::Float64,
];

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

fn swift_write_method(numeric: NumericType) -> String {
    format!("write{}", pascal(numeric_mangled(numeric)))
}

fn swift_read_method(numeric: NumericType) -> String {
    format!("read{}", pascal(numeric_mangled(numeric)))
}

fn swift_numeric_type(numeric: NumericType) -> String {
    swift_type(&Type::Numeric(numeric))
}

/// The unsigned integer whose bits a numeric is stored as. Float values use
/// their IEEE-754 bit pattern, which is the only difference from the value
/// type itself.
/// The integer writer a float delegates to, because a float's payload is its
/// IEEE-754 bit pattern.
fn swift_bits_writer(numeric: NumericType) -> Option<&'static str> {
    match numeric {
        NumericType::Float32 => Some("writeInt32"),
        NumericType::Float64 => Some("writeInt64"),
        _ => None,
    }
}

/// The signed integer type a float's bits are re-interpreted as when they are
/// written, so a float and an integer of the same width share one writer.
fn swift_signed_bits_type(numeric: NumericType) -> &'static str {
    match numeric {
        NumericType::Float32 => "Int32",
        _ => "Int64",
    }
}

fn swift_bits_type(numeric: NumericType) -> &'static str {
    match numeric {
        NumericType::Int8 | NumericType::UInt8 => "UInt8",
        NumericType::Int16 | NumericType::UInt16 => "UInt16",
        NumericType::Int32 | NumericType::UInt32 | NumericType::Float32 => "UInt32",
        _ => "UInt64",
    }
}

fn swift_from_bits(numeric: NumericType) -> String {
    match numeric {
        NumericType::Float32 => "Float(bitPattern: bits)".to_owned(),
        NumericType::Float64 => "Double(bitPattern: bits)".to_owned(),
        NumericType::UInt8 => "UInt8(bits)".to_owned(),
        NumericType::UInt16 => "UInt16(bits)".to_owned(),
        NumericType::UInt32 => "UInt32(bits)".to_owned(),
        NumericType::UInt64 => "UInt64(bits)".to_owned(),
        _ => format!("{}(bitPattern: bits)", swift_numeric_type(numeric)),
    }
}

fn write_function(ty: &Type, out: &mut SourceWriter) {
    let name = codec_name(ty, Direction::Write);
    out.push_str(&format!(
        "func {name}(_ value: {}, into writer: NexaValueWriter) {{\n",
        swift_type(ty)
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
        "func {name}(_ reader: NexaValueReader) -> {}? {{\n",
        swift_type(ty)
    ));
    for line in read_statements(ty, "reader") {
        out.push_str("    ");
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("}\n\n");
}

/// Statements that append `expression` to `writer`.
fn write_statements(ty: &Type, expression: &str, writer: &str) -> Vec<String> {
    match ty {
        Type::Bool => vec![format!("{writer}.writeBool({expression})")],
        Type::String => vec![format!("{writer}.writeString({expression})")],
        Type::Bytes => vec![format!("{writer}.writeBuffer({expression})")],
        Type::Numeric(numeric) => vec![format!(
            "{writer}.{}({expression})",
            swift_write_method(*numeric)
        )],
        Type::Enum(_) => vec![format!("{writer}.writeInt32(Int32({expression}.rawValue))")],
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
            let mut lines = vec![format!("{writer}.writeCount({expression}.count)")];
            lines.push(format!("for element in {expression} {{"));
            lines.push(format!("    {element_codec}(element, into: {writer})"));
            lines.push("}".to_owned());
            lines
        }
        Type::Set(element) => {
            // A set has no order of its own, so each element is encoded and
            // the encodings are sorted. The stored bytes are then canonical.
            let element_codec = codec_name(element, Direction::Write);
            vec![
                "var encoded: [Data] = []".to_owned(),
                format!("for element in {expression} {{"),
                "    let sink = NexaValueWriter()".to_owned(),
                format!("    {element_codec}(element, into: sink)"),
                "    encoded.append(sink.data)".to_owned(),
                "}".to_owned(),
                "encoded.sort { $0.lexicographicallyPrecedes($1) }".to_owned(),
                format!("{writer}.writeCount(encoded.count)"),
                format!("for entry in encoded {{ {writer}.writeRaw(entry) }}"),
            ]
        }
        Type::Map(key, value) => {
            let key_codec = codec_name(key, Direction::Write);
            let value_codec = codec_name(value, Direction::Write);
            vec![
                "var encoded: [(Data, Data)] = []".to_owned(),
                format!("for (entryKey, entryValue) in {expression} {{"),
                "    let keySink = NexaValueWriter()".to_owned(),
                "    let valueSink = NexaValueWriter()".to_owned(),
                format!("    {key_codec}(entryKey, into: keySink)"),
                format!("    {value_codec}(entryValue, into: valueSink)"),
                "    encoded.append((keySink.data, valueSink.data))".to_owned(),
                "}".to_owned(),
                "encoded.sort { $0.0.lexicographicallyPrecedes($1.0) }".to_owned(),
                format!("{writer}.writeCount(encoded.count)"),
                format!(
                    "for entry in encoded {{ {writer}.writeRaw(entry.0); {writer}.writeRaw(entry.1) }}"
                ),
            ]
        }
        other => vec![format!("// unsupported codec type {other:?}")],
    }
}

/// Statements that bind one value of `ty` to `binding_name(index)`.
///
/// An enum takes two: its ordinal is read first, because the case is rebuilt
/// from it rather than read directly.
fn field_read_statements(ty: &Type, index: usize, reader: &str) -> Vec<String> {
    let binding = binding_name(index);
    match ty {
        Type::Enum(name) => {
            // The ordinal is read into its own name, then the case is rebuilt
            // from it: a single `guard let` cannot produce the case directly.
            let ordinal = format!("ordinal{index}");
            vec![
                format!("guard let {ordinal} = {reader}.readInt32() else {{ return nil }}"),
                // An ordinal no case claims means a foreign payload, so the
                // whole read fails rather than inventing a case.
                format!(
                    "guard let {binding} = {}(rawValue: Int({ordinal})) else {{ return nil }}",
                    enum_name(name)
                ),
            ]
        }
        _ => vec![format!(
            "guard let {binding} = {} else {{ return nil }}",
            read_expression(ty, reader)
        )],
    }
}

/// Statements that read one value of `ty` from `reader` and return it.
fn read_statements(ty: &Type, reader: &str) -> Vec<String> {
    match ty {
        Type::Bool | Type::Numeric(_) => {
            vec![format!("return {}.{}()", reader, scalar_read_method(ty))]
        }
        Type::String => vec![format!("return {reader}.readString()")],
        Type::Bytes => vec![format!("return {reader}.readBuffer()")],
        Type::Enum(name) => {
            vec![
                format!("guard let ordinal = {reader}.readInt32() else {{ return nil }}"),
                format!("return {}(rawValue: Int(ordinal))", enum_name(name)),
            ]
        }
        Type::Struct { name, fields } => {
            let mut lines = Vec::new();
            for (index, (_, field_type)) in fields.iter().enumerate() {
                lines.extend(field_read_statements(field_type, index, reader));
            }
            // The memberwise initializer is label-based, so a field is written
            // `nexa_field_name: value`.
            let bindings = fields
                .iter()
                .enumerate()
                .map(|(index, (field, _))| {
                    format!("{}: {}", struct_field_name(field), binding_name(index))
                })
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("return {}({bindings})", struct_name(name)));
            lines
        }
        Type::Array(element) | Type::Set(element) => {
            let element_codec = codec_name(element, Direction::Read);
            let element_type = swift_type(element);
            let lines = vec![
                if matches!(ty, Type::Set(_)) {
                    format!("var values: Set<{element_type}> = []")
                } else {
                    format!("var values: [{element_type}] = []")
                },
                format!("guard let count = {reader}.readCount() else {{ return nil }}"),
                "values.reserveCapacity(count)".to_owned(),
                "for _ in 0..<count {".to_owned(),
                format!("    let element = {element_codec}({reader})"),
                "    guard let element else { return nil }".to_owned(),
                // A set is collected as a set all along, so the return is the
                // same collection.
                if matches!(ty, Type::Set(_)) {
                    "    values.insert(element)".to_owned()
                } else {
                    "    values.append(element)".to_owned()
                },
                "}".to_owned(),
                "return values".to_owned(),
            ];
            lines
        }
        Type::Map(key, value_type) => {
            let key_codec = codec_name(key, Direction::Read);
            let value_codec = codec_name(value_type, Direction::Read);
            vec![
                format!(
                    "var values: [{}: {}] = [:]",
                    swift_type(key),
                    swift_type(value_type)
                ),
                format!("guard let count = {reader}.readCount() else {{ return nil }}"),
                "values.reserveCapacity(count)".to_owned(),
                "for _ in 0..<count {".to_owned(),
                format!("    guard let entryKey = {key_codec}({reader}) else {{ return nil }}"),
                format!("    guard let entryValue = {value_codec}({reader}) else {{ return nil }}"),
                "    values[entryKey] = entryValue".to_owned(),
                "}".to_owned(),
                "return values".to_owned(),
            ]
        }
        other => vec![format!("// unsupported codec type {other:?}")],
    }
}

fn binding_name(index: usize) -> String {
    format!("field{index}")
}

/// The reader method for a scalar type.
fn scalar_read_method(ty: &Type) -> String {
    match ty {
        Type::Bool => "readBool".to_owned(),
        Type::Numeric(numeric) => swift_read_method(*numeric),
        Type::String => "readString".to_owned(),
        Type::Bytes => "readBuffer".to_owned(),
        other => format!("// unsupported scalar {other:?}"),
    }
}

/// The expression that reads one value of `ty`, or the nested codec call for
/// a struct or collection.
fn read_expression(ty: &Type, reader: &str) -> String {
    match ty {
        // An enum is bound by `field_read_statements`, which reads its ordinal
        // first; the case itself never comes straight from the reader.
        Type::Bool | Type::Numeric(_) | Type::String | Type::Bytes => {
            format!("{reader}.{}()", scalar_read_method(ty))
        }
        other => format!("{}({reader})", codec_name(other, Direction::Read)),
    }
}
