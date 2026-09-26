//! Swift type spellings for the shared IR.
//!
//! These mappings live in the backend (not in `nexa-ir`) because they are
//! platform output, not semantic facts: the IR knows *what* a type is, and
//! each backend decides how to spell it.

use nexa_codegen::{
    names::{enum_name, struct_name},
    SourceWriter,
};
use nexa_ir::{Module, NumericType, Type};

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

/// Renders the module's enums as Swift `String`-backed error types.
///
/// Every `.nx` enum is lowered to a `String`-backed `Error` conformance
/// because a failed `Result` must carry the case name across the native
/// boundary without any runtime type information.
pub(crate) fn render_enums(module: &Module, out: &mut SourceWriter) {
    for declaration in &module.enums {
        out.push_str(&format!(
            "private enum {}: String, Error {{\n",
            enum_name(&declaration.name)
        ));
        for case in &declaration.cases {
            out.push_str(&format!("    case {case}\n"));
        }
        out.push_str("}\n\n");
    }
}

/// Renders the `NexaNavigationRoute` enum that types the navigation path.
///
/// Each case carries a `UUID` route identity ahead of the screen's parameters.
/// SwiftUI needs that identity to distinguish two pushes of the same screen
/// with different arguments, which a payload-only enum cannot express.
pub(crate) fn render_navigation_routes(module: &Module, out: &mut SourceWriter) {
    if module.screens.is_empty() {
        return;
    }
    out.push_str("private enum NexaNavigationRoute: Hashable {\n");
    for screen in &module.screens {
        let case_name = nexa_codegen::names::navigation_case_name(screen.id);
        let mut payload_types = vec!["UUID".to_owned()];
        payload_types.extend(
            screen
                .parameters
                .iter()
                .map(|parameter| swift_type(&parameter.ty)),
        );
        out.push_str(&format!(
            "    case {case_name}({})\n",
            payload_types.join(", ")
        ));
    }
    out.push_str("}\n\n");
}
