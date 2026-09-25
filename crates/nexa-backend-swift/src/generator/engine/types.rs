//! Swift type spellings for the shared IR.
//!
//! These mappings live in the backend (not in `nexa-ir`) because they are
//! platform output, not semantic facts: the IR knows *what* a type is, and
//! each backend decides how to spell it.

use nexa_codegen::names::{enum_name, struct_name};
use nexa_ir::{NumericType, Type};

/// Swift spelling of a numeric type.
pub(crate) fn swift_numeric(ty: NumericType) -> &'static str {
    match ty {
        NumericType::Int8 => "Int8",
        NumericType::Int16 => "Int16",
        NumericType::Int32 => "Int32",
        NumericType::Int64 => "Int64",
        NumericType::UInt8 => "UInt8",
        NumericType::UInt16 => "UInt16",
        NumericType::UInt32 => "UInt32",
        NumericType::UInt64 => "UInt64",
        NumericType::Float32 => "Float",
        NumericType::Float64 => "Double",
    }
}

/// Swift spelling of a shared IR type.
pub(crate) fn swift_type(ty: &Type) -> String {
    match ty {
        Type::Void => "Void".to_owned(),
        Type::String => "String".to_owned(),
        Type::Bool => "Bool".to_owned(),
        Type::Numeric(numeric) => swift_numeric(*numeric).to_owned(),
        Type::Optional(inner) => format!("{}?", swift_type(inner)),
        Type::Result(value, error) => {
            format!("Result<{}, {}>", swift_type(value), swift_type(error))
        }
        Type::Array(element) => format!("[{}]", swift_type(element)),
        Type::Set(element) => format!("Set<{}>", swift_type(element)),
        Type::Map(key, value) => format!("[{}: {}]", swift_type(key), swift_type(value)),
        Type::Pair(first, second) => {
            format!("({}, {})", swift_type(first), swift_type(second))
        }
        Type::Triple(first, second, third) => format!(
            "({}, {}, {})",
            swift_type(first),
            swift_type(second),
            swift_type(third)
        ),
        Type::Enum(name) => enum_name(name),
        Type::Plugin { name, .. } => name.clone(),
        Type::NetworkResponse => "NexaNetworkResponse".to_owned(),
        Type::Struct { name, .. } => struct_name(name),
    }
}
