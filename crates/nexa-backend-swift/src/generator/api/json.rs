//! Statically typed JSON codecs backed by Swift's native Codable decoder.
//!
//! Each concrete Nexa value type receives a compile-time codec alias. Struct
//! fields and enum cases are emitted as direct typed code; the generated app
//! never uses reflection or an untyped model dictionary.

use nexa_codegen::SourceWriter;
use nexa_ir::{EnumDecl, Type};

use crate::generator::engine::{features::Features, imports::ImportSet, types::swift_type};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.facts.capabilities.uses_json_api, "Foundation");
}

pub(crate) fn render(types: &[Type], enums: &[EnumDecl], out: &mut SourceWriter) {
    out.push_str(RUNTIME);
    out.push('\n');
    for ty in types {
        render_type_codec(ty, enums, out);
    }
    render_api(out);
}

fn render_api(out: &mut SourceWriter) {
    out.push_str(
        r#"func nexaJsonParse<Codec: NexaJsonValueCodec>(
    _ raw: String,
    using codec: Codec.Type
) -> Result<Codec.Value, NexaJsonError> {
    let decoder = JSONDecoder()
    decoder.nonConformingFloatDecodingStrategy = .convertFromString(
        positiveInfinity: "Infinity",
        negativeInfinity: "-Infinity",
        nan: "NaN"
    )
    do {
        let value = try decoder.decode(NexaJsonDecodedValue<Codec>.self, from: Data(raw.utf8))
        return .success(value.value)
    } catch is NexaJsonInvalidValue {
        return .failure(.invalidValue)
    } catch let error as DecodingError {
        switch error {
        case .keyNotFound:
            return .failure(.missingField)
        case .typeMismatch, .valueNotFound:
            return .failure(.typeMismatch)
        case .dataCorrupted:
            return .failure(.invalidJson)
        @unknown default:
            return .failure(.invalidJson)
        }
    } catch {
        return .failure(.invalidJson)
    }
}

func nexaJsonStringify<Codec: NexaJsonValueCodec>(
    _ value: Codec.Value,
    using codec: Codec.Type
) -> String {
    var writer = NexaJsonWriter()
    Codec.encodeJSON(value, into: &writer)
    return writer.output
}
"#,
    );
}

fn render_type_codec(ty: &Type, enums: &[EnumDecl], out: &mut SourceWriter) {
    let alias = nexa_codegen::value::json_codec_name(ty);
    let implementation = match ty {
        Type::String => "NexaJsonStringCodec".to_owned(),
        Type::Bool => "NexaJsonBoolCodec".to_owned(),
        Type::Numeric(numeric) if is_float(*numeric) => {
            format!(
                "NexaJsonFloat{}Codec",
                if is_float32(*numeric) { "32" } else { "64" }
            )
        }
        Type::Numeric(numeric) => format!(
            "NexaJsonIntegerCodec<{}>",
            swift_type(&Type::Numeric(*numeric))
        ),
        Type::Bytes => "NexaJsonDataCodec".to_owned(),
        Type::Optional(inner) => format!(
            "NexaJsonOptionalCodec<{}>",
            nexa_codegen::value::json_codec_name(inner)
        ),
        Type::Array(inner) => format!(
            "NexaJsonArrayCodec<{}>",
            nexa_codegen::value::json_codec_name(inner)
        ),
        Type::Set(inner) => format!(
            "NexaJsonSetCodec<{}>",
            nexa_codegen::value::json_codec_name(inner)
        ),
        Type::Map(_, value) => format!(
            "NexaJsonMapCodec<{}>",
            nexa_codegen::value::json_codec_name(value)
        ),
        Type::Pair(first, second) => format!(
            "NexaJsonPairCodec<{}, {}>",
            nexa_codegen::value::json_codec_name(first),
            nexa_codegen::value::json_codec_name(second)
        ),
        Type::Triple(first, second, third) => format!(
            "NexaJsonTripleCodec<{}, {}, {}>",
            nexa_codegen::value::json_codec_name(first),
            nexa_codegen::value::json_codec_name(second),
            nexa_codegen::value::json_codec_name(third)
        ),
        Type::Result(success, failure) => format!(
            "NexaJsonResultCodec<{}, {}>",
            nexa_codegen::value::json_codec_name(success),
            nexa_codegen::value::json_codec_name(failure)
        ),
        Type::Enum(name) => {
            render_enum_codec(name, enums, out);
            named_codec_name(ty)
        }
        Type::Struct { name, fields } => {
            render_struct_codec(name, fields, out);
            named_codec_name(ty)
        }
        other => {
            debug_assert!(
                false,
                "unsupported JSON value type reached Swift codegen: {other:?}"
            );
            "NexaJsonStringCodec".to_owned()
        }
    };
    out.push_str(&format!("typealias {alias} = {implementation}\n\n"));
}

fn render_enum_codec(name: &str, enums: &[EnumDecl], out: &mut SourceWriter) {
    let native_name = nexa_codegen::names::enum_name(name);
    let codec = named_codec_name(&Type::Enum(name.to_owned()));
    let cases = enums
        .iter()
        .find(|declaration| declaration.name == name)
        .map(|declaration| declaration.cases.as_slice())
        .unwrap_or(&[]);
    out.push_str(&format!(
        "struct {codec}: NexaJsonValueCodec {{\n    typealias Value = {native_name}\n\n    static func decode(from decoder: Decoder) throws -> Value {{\n        let container = try decoder.singleValueContainer()\n        let raw = try container.decode(String.self)\n        switch raw {{\n"
    ));
    for case in cases {
        out.push_str(&format!("        case \"{case}\": return .{case}\n"));
    }
    out.push_str(
        "        default: throw NexaJsonInvalidValue()\n        }\n    }\n\n    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {\n        switch value {\n",
    );
    for case in cases {
        out.push_str(&format!(
            "        case .{case}: writer.appendQuoted(\"{case}\")\n"
        ));
    }
    out.push_str("        }\n    }\n}\n\n");
}

fn render_struct_codec(name: &str, fields: &[(String, Type)], out: &mut SourceWriter) {
    let native_name = nexa_codegen::names::struct_name(name);
    let codec = named_codec_name(&Type::Struct {
        name: name.to_owned(),
        fields: fields.to_vec(),
    });
    out.push_str(&format!(
        "struct {codec}: NexaJsonValueCodec {{\n    typealias Value = {native_name}\n\n    private enum Keys: String, CodingKey {{\n"
    ));
    for (field, _) in fields {
        let field_name = nexa_codegen::names::struct_field_name(field);
        out.push_str(&format!("        case {field_name} = \"{field}\"\n"));
    }
    out.push_str("    }\n\n    static func decode(from decoder: Decoder) throws -> Value {\n        let container = try decoder.container(keyedBy: Keys.self)\n        return Value(\n");
    for (index, (field, ty)) in fields.iter().enumerate() {
        let field_name = nexa_codegen::names::struct_field_name(field);
        let json_codec = nexa_codegen::value::json_codec_name(ty);
        let decoder = if matches!(ty, Type::Optional(_)) {
            format!(
                "try container.decodeIfPresent(NexaJsonDecodedValue<{json_codec}>.self, forKey: .{field_name})?.value"
            )
        } else {
            format!(
                "try container.decode(NexaJsonDecodedValue<{json_codec}>.self, forKey: .{field_name}).value"
            )
        };
        out.push_str(&format!(
            "            {field_name}: {decoder}{}\n",
            if index + 1 == fields.len() { "" } else { "," }
        ));
    }
    out.push_str("        )\n    }\n\n    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {\n        writer.appendRaw(\"{\")\n");
    for (index, (field, ty)) in fields.iter().enumerate() {
        let field_name = nexa_codegen::names::struct_field_name(field);
        let json_codec = nexa_codegen::value::json_codec_name(ty);
        if index > 0 {
            out.push_str("        writer.appendRaw(\",\")\n");
        }
        out.push_str(&format!(
            "        writer.appendQuoted(\"{field}\")\n        writer.appendRaw(\":\")\n        {json_codec}.encodeJSON(value.{field_name}, into: &writer)\n"
        ));
    }
    out.push_str("        writer.appendRaw(\"}\")\n    }\n}\n\n");
}

fn named_codec_name(ty: &Type) -> String {
    nexa_codegen::value::json_codec_name(ty).replacen("NexaJsonCodec", "NexaJsonNamedCodec", 1)
}

fn is_float(numeric: nexa_ir::NumericType) -> bool {
    is_float32(numeric) || matches!(numeric, nexa_ir::NumericType::Float64)
}

fn is_float32(numeric: nexa_ir::NumericType) -> bool {
    matches!(numeric, nexa_ir::NumericType::Float32)
}

const RUNTIME: &str = r#"// Statically typed JSON codec primitives.

protocol NexaJsonValueCodec {
    associatedtype Value
    static func decode(from decoder: Decoder) throws -> Value
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter)
}

struct NexaJsonDecodedValue<Codec: NexaJsonValueCodec>: Decodable {
    let value: Codec.Value

    init(from decoder: Decoder) throws {
        value = try Codec.decode(from: decoder)
    }
}

struct NexaJsonInvalidValue: Error {}

struct NexaJsonWriter {
    private(set) var output = ""

    mutating func appendRaw(_ value: String) {
        output.append(contentsOf: value)
    }

    mutating func appendQuoted(_ value: String) {
        output.append("\"")
        for scalar in value.unicodeScalars {
            switch scalar.value {
            case 0x22: output.append("\\\"")
            case 0x5c: output.append("\\\\")
            case 0x08: output.append("\\b")
            case 0x09: output.append("\\t")
            case 0x0a: output.append("\\n")
            case 0x0c: output.append("\\f")
            case 0x0d: output.append("\\r")
            case 0x00...0x1f:
                output.append("\\u")
                let hex = String(scalar.value, radix: 16)
                if hex.count < 4 {
                    output.append(String(repeating: "0", count: 4 - hex.count))
                }
                output.append(contentsOf: hex)
            default: output.unicodeScalars.append(scalar)
            }
        }
        output.append("\"")
    }
}

struct NexaJsonStringCodec: NexaJsonValueCodec {
    typealias Value = String
    static func decode(from decoder: Decoder) throws -> String {
        try decoder.singleValueContainer().decode(String.self)
    }
    static func encodeJSON(_ value: String, into writer: inout NexaJsonWriter) {
        writer.appendQuoted(value)
    }
}

struct NexaJsonBoolCodec: NexaJsonValueCodec {
    typealias Value = Bool
    static func decode(from decoder: Decoder) throws -> Bool {
        try decoder.singleValueContainer().decode(Bool.self)
    }
    static func encodeJSON(_ value: Bool, into writer: inout NexaJsonWriter) {
        writer.appendRaw(value ? "true" : "false")
    }
}

struct NexaJsonIntegerCodec<IntegerValue: FixedWidthInteger & Decodable>: NexaJsonValueCodec {
    typealias Value = IntegerValue
    static func decode(from decoder: Decoder) throws -> IntegerValue {
        try decoder.singleValueContainer().decode(IntegerValue.self)
    }
    static func encodeJSON(_ value: IntegerValue, into writer: inout NexaJsonWriter) {
        writer.appendRaw(String(value))
    }
}

struct NexaJsonFloat32Codec: NexaJsonValueCodec {
    typealias Value = Float
    static func decode(from decoder: Decoder) throws -> Float {
        try decoder.singleValueContainer().decode(Float.self)
    }
    static func encodeJSON(_ value: Float, into writer: inout NexaJsonWriter) {
        if value.isFinite { writer.appendRaw(String(value)) }
        else if value.isNaN { writer.appendQuoted("NaN") }
        else if value.sign == .minus { writer.appendQuoted("-Infinity") }
        else { writer.appendQuoted("Infinity") }
    }
}

struct NexaJsonFloat64Codec: NexaJsonValueCodec {
    typealias Value = Double
    static func decode(from decoder: Decoder) throws -> Double {
        try decoder.singleValueContainer().decode(Double.self)
    }
    static func encodeJSON(_ value: Double, into writer: inout NexaJsonWriter) {
        if value.isFinite { writer.appendRaw(String(value)) }
        else if value.isNaN { writer.appendQuoted("NaN") }
        else if value.sign == .minus { writer.appendQuoted("-Infinity") }
        else { writer.appendQuoted("Infinity") }
    }
}

struct NexaJsonDataCodec: NexaJsonValueCodec {
    typealias Value = Data
    static func decode(from decoder: Decoder) throws -> Data {
        let encoded = try decoder.singleValueContainer().decode(String.self)
        guard let data = Data(base64Encoded: encoded) else {
            throw NexaJsonInvalidValue()
        }
        return data
    }
    static func encodeJSON(_ value: Data, into writer: inout NexaJsonWriter) {
        writer.appendQuoted(value.base64EncodedString())
    }
}

struct NexaJsonOptionalCodec<Wrapped: NexaJsonValueCodec>: NexaJsonValueCodec {
    typealias Value = Wrapped.Value?
    static func decode(from decoder: Decoder) throws -> Value {
        if try decoder.singleValueContainer().decodeNil() { return nil }
        return try Wrapped.decode(from: decoder)
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        guard let value else { writer.appendRaw("null"); return }
        Wrapped.encodeJSON(value, into: &writer)
    }
}

struct NexaJsonArrayCodec<Element: NexaJsonValueCodec>: NexaJsonValueCodec {
    typealias Value = [Element.Value]
    static func decode(from decoder: Decoder) throws -> Value {
        var container = try decoder.unkeyedContainer()
        var values: Value = []
        while !container.isAtEnd {
            values.append(try Element.decode(from: container.superDecoder()))
        }
        return values
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        writer.appendRaw("[")
        for index in value.indices {
            if index > value.startIndex { writer.appendRaw(",") }
            Element.encodeJSON(value[index], into: &writer)
        }
        writer.appendRaw("]")
    }
}

struct NexaJsonSetCodec<Element: NexaJsonValueCodec>: NexaJsonValueCodec where Element.Value: Hashable {
    typealias Value = Set<Element.Value>
    static func decode(from decoder: Decoder) throws -> Value {
        var container = try decoder.unkeyedContainer()
        var values = Value()
        while !container.isAtEnd {
            values.insert(try Element.decode(from: container.superDecoder()))
        }
        return values
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        let encoded = value.map { item -> (String, Element.Value) in
            var itemWriter = NexaJsonWriter()
            Element.encodeJSON(item, into: &itemWriter)
            return (itemWriter.output, item)
        }.sorted { $0.0 < $1.0 }
        writer.appendRaw("[")
        for index in encoded.indices {
            if index > encoded.startIndex { writer.appendRaw(",") }
            writer.appendRaw(encoded[index].0)
        }
        writer.appendRaw("]")
    }
}

struct NexaJsonCodingKey: CodingKey, Hashable {
    let stringValue: String
    let intValue: Int?
    init?(stringValue: String) { self.stringValue = stringValue; self.intValue = nil }
    init?(intValue: Int) { self.stringValue = String(intValue); self.intValue = intValue }
    init(_ value: String) { self.stringValue = value; self.intValue = nil }
}

struct NexaJsonMapCodec<ValueCodec: NexaJsonValueCodec>: NexaJsonValueCodec {
    typealias Value = [String: ValueCodec.Value]
    static func decode(from decoder: Decoder) throws -> Value {
        let container = try decoder.container(keyedBy: NexaJsonCodingKey.self)
        var values: Value = [:]
        for key in container.allKeys {
            values[key.stringValue] = try ValueCodec.decode(from: container.superDecoder(forKey: key))
        }
        return values
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        writer.appendRaw("{")
        let keys = value.keys.sorted()
        for (index, key) in keys.enumerated() {
            if index > 0 { writer.appendRaw(",") }
            writer.appendQuoted(key)
            writer.appendRaw(":")
            if let item = value[key] { ValueCodec.encodeJSON(item, into: &writer) }
            else { writer.appendRaw("null") }
        }
        writer.appendRaw("}")
    }
}

struct NexaJsonPairCodec<First: NexaJsonValueCodec, Second: NexaJsonValueCodec>: NexaJsonValueCodec {
    typealias Value = (First.Value, Second.Value)
    static func decode(from decoder: Decoder) throws -> Value {
        var container = try decoder.unkeyedContainer()
        let first = try First.decode(from: container.superDecoder())
        let second = try Second.decode(from: container.superDecoder())
        guard container.isAtEnd else { throw NexaJsonInvalidValue() }
        return (first, second)
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        writer.appendRaw("[")
        First.encodeJSON(value.0, into: &writer)
        writer.appendRaw(",")
        Second.encodeJSON(value.1, into: &writer)
        writer.appendRaw("]")
    }
}

struct NexaJsonTripleCodec<First: NexaJsonValueCodec, Second: NexaJsonValueCodec, Third: NexaJsonValueCodec>: NexaJsonValueCodec {
    typealias Value = (First.Value, Second.Value, Third.Value)
    static func decode(from decoder: Decoder) throws -> Value {
        var container = try decoder.unkeyedContainer()
        let first = try First.decode(from: container.superDecoder())
        let second = try Second.decode(from: container.superDecoder())
        let third = try Third.decode(from: container.superDecoder())
        guard container.isAtEnd else { throw NexaJsonInvalidValue() }
        return (first, second, third)
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        writer.appendRaw("[")
        First.encodeJSON(value.0, into: &writer)
        writer.appendRaw(",")
        Second.encodeJSON(value.1, into: &writer)
        writer.appendRaw(",")
        Third.encodeJSON(value.2, into: &writer)
        writer.appendRaw("]")
    }
}

struct NexaJsonResultCodec<Success: NexaJsonValueCodec, Failure: NexaJsonValueCodec>: NexaJsonValueCodec where Failure.Value: Error {
    typealias Value = Result<Success.Value, Failure.Value>
    private enum Keys: String, CodingKey { case success, failure }
    static func decode(from decoder: Decoder) throws -> Value {
        let container = try decoder.container(keyedBy: Keys.self)
        guard container.allKeys.count == 1 else {
            throw NexaJsonInvalidValue()
        }
        if container.contains(.success) {
            return .success(try Success.decode(from: container.superDecoder(forKey: .success)))
        }
        return .failure(try Failure.decode(from: container.superDecoder(forKey: .failure)))
    }
    static func encodeJSON(_ value: Value, into writer: inout NexaJsonWriter) {
        writer.appendRaw("{")
        switch value {
        case .success(let success):
            writer.appendQuoted("success")
            writer.appendRaw(":")
            Success.encodeJSON(success, into: &writer)
        case .failure(let failure):
            writer.appendQuoted("failure")
            writer.appendRaw(":")
            Failure.encodeJSON(failure, into: &writer)
        }
        writer.appendRaw("}")
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::{render, render_type_codec};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{EnumDecl, NumericType, Type};

    #[test]
    fn renders_static_codecs_for_nested_models_and_compound_values() {
        let profile = Type::Struct {
            name: "Profile".to_owned(),
            fields: vec![
                ("display_name".to_owned(), Type::String),
                (
                    "scores".to_owned(),
                    Type::Array(Box::new(Type::Numeric(NumericType::Int32))),
                ),
            ],
        };
        let mut output = SourceWriter::new();
        let enumerations = vec![EnumDecl {
            name: "LoadState".to_owned(),
            cases: vec!["idle".to_owned(), "ready".to_owned()],
        }];
        render_type_codec(
            &Type::Numeric(NumericType::Int32),
            &enumerations,
            &mut output,
        );
        render_type_codec(
            &Type::Array(Box::new(Type::Numeric(NumericType::Int32))),
            &enumerations,
            &mut output,
        );
        render_type_codec(&profile, &enumerations, &mut output);
        render_type_codec(
            &Type::Enum("LoadState".to_owned()),
            &enumerations,
            &mut output,
        );
        let generated = output.finish();
        assert!(generated.contains("NexaJsonIntegerCodec<Int32>"));
        assert!(generated.contains("NexaJsonArrayCodec<NexaJsonCodecint32>"));
        assert!(generated.contains("case nexa_field_display_name = \"display_name\""));
        assert!(generated.contains("case \"idle\": return .idle"));
    }

    #[test]
    fn runtime_uses_typed_results_and_handles_non_finite_float_values() {
        let mut output = SourceWriter::new();
        render(&[], &[], &mut output);
        let generated = output.finish();
        assert!(generated.contains("Result<Codec.Value, NexaJsonError>"));
        assert!(generated.contains("positiveInfinity: \"Infinity\""));
        assert!(generated.contains("static func encodeJSON(_ value: Double"));
        assert!(generated.contains("NexaJsonWriter"));
    }
}
