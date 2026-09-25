//! Kotlin type spellings for the shared IR.
//!
//! These mappings live in the backend (not in `nexa-ir`) because they are
//! platform output, not semantic facts: the IR knows *what* a type is, and
//! each backend decides how to spell it.

use nexa_codegen::names::{enum_name, struct_name};
use nexa_ir::{NumericType, Type};

/// Kotlin spelling of a numeric type.
pub(crate) fn kotlin_numeric(ty: NumericType) -> &'static str {
    match ty {
        NumericType::Int8 => "Byte",
        NumericType::Int16 => "Short",
        NumericType::Int32 => "Int",
        NumericType::Int64 => "Long",
        NumericType::UInt8 => "UByte",
        NumericType::UInt16 => "UShort",
        NumericType::UInt32 => "UInt",
        NumericType::UInt64 => "ULong",
        NumericType::Float32 => "Float",
        NumericType::Float64 => "Double",
    }
}

/// Kotlin spelling of a shared IR type.
pub(crate) fn kotlin_type(ty: &Type) -> String {
    match ty {
        Type::Void => "Unit".to_owned(),
        Type::String => "String".to_owned(),
        Type::Bool => "Boolean".to_owned(),
        Type::Numeric(numeric) => kotlin_numeric(*numeric).to_owned(),
        Type::Optional(inner) => format!("{}?", kotlin_type(inner)),
        Type::Result(value, error) => {
            format!("NexaResult<{}, {}>", kotlin_type(value), kotlin_type(error))
        }
        Type::Array(element) => format!("List<{}>", kotlin_type(element)),
        Type::Set(element) => format!("Set<{}>", kotlin_type(element)),
        Type::Map(key, value) => format!("Map<{}, {}>", kotlin_type(key), kotlin_type(value)),
        Type::Pair(first, second) => {
            format!("Pair<{}, {}>", kotlin_type(first), kotlin_type(second))
        }
        Type::Triple(first, second, third) => format!(
            "Triple<{}, {}, {}>",
            kotlin_type(first),
            kotlin_type(second),
            kotlin_type(third)
        ),
        Type::Enum(name) => enum_name(name),
        Type::Plugin { name, .. } => name.clone(),
        Type::NetworkResponse => "NexaNetworkResponse".to_owned(),
        Type::Struct { name, .. } => struct_name(name),
    }
}
