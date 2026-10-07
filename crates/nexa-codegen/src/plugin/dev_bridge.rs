//! Statically typed plugin adapters for the Dev hot-reload interpreter.
//!
//! The Dev protocol carries evaluated values dynamically, but the final hop
//! into a plugin remains a generated direct call with arguments decoded to
//! the IDL's concrete types. Keeping these adapters debug-only preserves the
//! production plugin path and avoids reflection or a runtime service locator.

use nexa_plugin_idl::{InterfaceKind, PluginIdl};

use super::bridge_plan::{
    BridgeConstructor, BridgeInterface, BridgeMethod, BridgeNamedKind, BridgeNamedType,
    BridgeParameter, BridgePlan, BridgeScalar, BridgeType, BridgeTypeKind, BridgeVariant,
};

pub(super) fn swift(plugins: &[(String, PluginIdl)]) -> Result<String, String> {
    let mut out = String::from(
        "import Foundation\nimport SwiftUI\n\ninternal struct NexaDevPluginFailure: Error, @unchecked Sendable {\n    let namespace: String\n    let errorType: String\n    let variant: String\n    let payload: [String: Any]\n}\n\n@MainActor\ninternal enum NexaDevPluginBridge {\n    static func construct(namespace: String, name: String, arguments: [Any]) -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            if let Some(constructor) = interface.constructors.first()
                && supported_parameters(&plan, &constructor.parameters)
            {
                render_swift_constructor(&mut out, namespace, &plan, interface, constructor);
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeSync(namespace: String, name: String, options: [String: Any], codecs: [Any], enumCases: [String: [String]]) -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method, &plan) || method.is_async {
                    continue;
                }
                render_swift_method(&mut out, namespace, &plan, method, false)?;
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeAsync(namespace: String, name: String, options: [String: Any], codecs: [Any], enumCases: [String: [String]]) async throws -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method, &plan) {
                    continue;
                }
                render_swift_method(&mut out, namespace, &plan, method, true)?;
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeInstanceSync(receiver: Any, namespace: String, name: String, options: [String: Any], codecs: [Any], enumCases: [String: [String]]) -> (Bool, Any) {\n",
    );
    render_swift_instance_methods(&mut out, plugins, false)?;
    out.push_str(
        "        return (false, NSNull())\n    }\n\n    static func invokeInstanceAsync(receiver: Any, namespace: String, name: String, options: [String: Any], codecs: [Any], enumCases: [String: [String]]) async throws -> (Bool, Any) {\n",
    );
    render_swift_instance_methods(&mut out, plugins, true)?;
    out.push_str("        return (false, NSNull())\n    }\n\n");
    render_swift_instance_properties(&mut out, plugins)?;
    render_swift_instance_events(&mut out, plugins)?;
    render_swift_components(&mut out, plugins)?;
    render_swift_plugin_errors(&mut out, plugins)?;
    out.push_str(
        "    private static func decodeString(_ raw: Any?) -> String? { raw as? String }\n    private static func decodeBool(_ raw: Any?) -> Bool? { (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeBytes(_ raw: Any?) -> Data? {\n        if let bytes = raw as? Data { return bytes }\n        guard let text = raw as? String else { return nil }\n        return Data(base64Encoded: text)\n    }\n    private static func decodeOptionalString(_ raw: Any?) -> String? { raw is NSNull ? nil : raw as? String }\n    private static func decodeOptionalBool(_ raw: Any?) -> Bool? { raw is NSNull ? nil : (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeOptionalBytes(_ raw: Any?) -> Data? { raw is NSNull ? nil : decodeBytes(raw) }\n    private static func decodeInt8(_ raw: Any?) -> Int8? { (raw as? NSNumber).map { Int8(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt16(_ raw: Any?) -> Int16? { (raw as? NSNumber).map { Int16(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt32(_ raw: Any?) -> Int32? { (raw as? NSNumber).map { Int32(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt64(_ raw: Any?) -> Int64? { (raw as? NSNumber).map { $0.int64Value } }\n    private static func decodeUInt8(_ raw: Any?) -> UInt8? { (raw as? NSNumber).map { UInt8(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt16(_ raw: Any?) -> UInt16? { (raw as? NSNumber).map { UInt16(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt32(_ raw: Any?) -> UInt32? { (raw as? NSNumber).map { UInt32(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt64(_ raw: Any?) -> UInt64? { (raw as? NSNumber).map { $0.uint64Value } }\n    private static func decodeFloat32(_ raw: Any?) -> Float? { (raw as? NSNumber).map { $0.floatValue } }\n    private static func decodeFloat64(_ raw: Any?) -> Double? { (raw as? NSNumber).map { $0.doubleValue } }\n    private static func decodeOptionalInt8(_ raw: Any?) -> Int8? { raw is NSNull ? nil : decodeInt8(raw) }\n    private static func decodeOptionalInt16(_ raw: Any?) -> Int16? { raw is NSNull ? nil : decodeInt16(raw) }\n    private static func decodeOptionalInt32(_ raw: Any?) -> Int32? { raw is NSNull ? nil : decodeInt32(raw) }\n    private static func decodeOptionalInt64(_ raw: Any?) -> Int64? { raw is NSNull ? nil : decodeInt64(raw) }\n    private static func decodeOptionalUInt8(_ raw: Any?) -> UInt8? { raw is NSNull ? nil : decodeUInt8(raw) }\n    private static func decodeOptionalUInt16(_ raw: Any?) -> UInt16? { raw is NSNull ? nil : decodeUInt16(raw) }\n    private static func decodeOptionalUInt32(_ raw: Any?) -> UInt32? { raw is NSNull ? nil : decodeUInt32(raw) }\n    private static func decodeOptionalUInt64(_ raw: Any?) -> UInt64? { raw is NSNull ? nil : decodeUInt64(raw) }\n    private static func decodeOptionalFloat32(_ raw: Any?) -> Float? { raw is NSNull ? nil : decodeFloat32(raw) }\n    private static func decodeOptionalFloat64(_ raw: Any?) -> Double? { raw is NSNull ? nil : decodeFloat64(raw) }\n}\n",
    );
    Ok(out)
}

pub(super) fn kotlin(plugins: &[(String, PluginIdl)]) -> Result<String, String> {
    let mut out = String::from(
        "import android.util.Base64\nimport org.json.JSONObject\n\ninternal class NexaDevPluginFailure(\n    val namespace: String,\n    val errorType: String,\n    val variant: String,\n    val payload: Map<String, Any>,\n    cause: Throwable? = null,\n) : Exception(\"$namespace.$errorType.$variant\", cause)\n\ninternal object NexaDevPluginBridge {\n    fun construct(namespace: String, name: String, arguments: List<Any>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            if let Some(constructor) = interface.constructors.first()
                && supported_parameters(&plan, &constructor.parameters)
            {
                render_kotlin_constructor(&mut out, namespace, &plan, interface, constructor);
            }
        }
    }
    out.push_str(
        "            else -> false to JSONObject.NULL\n        }\n\n    fun invokeSync(namespace: String, name: String, options: Map<String, Any>, codecs: List<Any>, enumCases: Map<String, List<String>>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method, &plan) || method.is_async {
                    continue;
                }
                render_kotlin_method(&mut out, namespace, &plan, method, false)?;
            }
        }
    }
    out.push_str(
        "            else -> false to JSONObject.NULL\n        }\n\n    suspend fun invokeAsync(namespace: String, name: String, options: Map<String, Any>, codecs: List<Any>, enumCases: Map<String, List<String>>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method, &plan) {
                    continue;
                }
                render_kotlin_method(&mut out, namespace, &plan, method, true)?;
            }
        }
    }
    out.push_str("            else -> false to JSONObject.NULL\n        }\n\n    fun invokeInstanceSync(receiver: Any, namespace: String, name: String, options: Map<String, Any>, codecs: List<Any>, enumCases: Map<String, List<String>>): Pair<Boolean, Any> {\n");
    render_kotlin_instance_methods(&mut out, plugins, false)?;
    out.push_str("        return false to JSONObject.NULL\n    }\n\n    suspend fun invokeInstanceAsync(receiver: Any, namespace: String, name: String, options: Map<String, Any>, codecs: List<Any>, enumCases: Map<String, List<String>>): Pair<Boolean, Any> {\n");
    render_kotlin_instance_methods(&mut out, plugins, true)?;
    out.push_str("        return false to JSONObject.NULL\n    }\n\n");
    render_kotlin_instance_properties(&mut out, plugins)?;
    render_kotlin_instance_events(&mut out, plugins)?;
    render_kotlin_components(&mut out, plugins)?;
    render_kotlin_plugin_errors(&mut out, plugins)?;
    out.push_str("    private fun decodeBytes(raw: Any?): ByteArray? = when (raw) {\n        is ByteArray -> raw\n        is String -> Base64.decode(raw, Base64.DEFAULT)\n        else -> null\n    }\n    private fun decodeOptionalString(raw: Any?): String? = if (raw == null || raw == JSONObject.NULL) null else raw as? String\n    private fun decodeOptionalBool(raw: Any?): Boolean? = if (raw == null || raw == JSONObject.NULL) null else raw as? Boolean\n    private fun decodeOptionalBytes(raw: Any?): ByteArray? = if (raw == null || raw == JSONObject.NULL) null else decodeBytes(raw)\n    private fun decodeOptionalNumber(raw: Any?, type: String): Any? {\n        if (raw == null || raw == JSONObject.NULL) return null\n        val number = raw as? Number ?: return null\n        return when (type) {\n            \"Int8\" -> number.toByte()\n            \"Int16\" -> number.toShort()\n            \"Int32\" -> number.toInt()\n            \"Int64\" -> number.toLong()\n            \"UInt8\" -> number.toByte().toUByte()\n            \"UInt16\" -> number.toShort().toUShort()\n            \"UInt32\" -> number.toInt().toUInt()\n            \"UInt64\" -> number.toLong().toULong()\n            \"Float32\" -> number.toFloat()\n            else -> number.toDouble()\n        }\n    }\n}\n");
    Ok(out)
}

fn supported(method: &BridgeMethod, plan: &BridgePlan) -> bool {
    if method.type_parameters.is_empty() {
        return supported_parameters(plan, &method.parameters)
            && supported_return(plan, method.success_type());
    }
    method.parameters.iter().all(|parameter| {
        if super::bridge_plan::contains_type_parameter(&parameter.ty) {
            supported_dynamic_shape(&parameter.ty)
        } else {
            supported_value_type(plan, &parameter.ty)
        }
    }) && if super::bridge_plan::contains_type_parameter(method.success_type()) {
        match method.success_type() {
            // Optional generic parameters and optional values inside a
            // compound return are supported by the typed codecs. The outer
            // optional on a generic return is represented by the Dev host's
            // null sentinel; the reader codec handles the non-optional inner
            // value.
            BridgeType::Optional(inner) => supported_dynamic_shape(inner),
            other => supported_dynamic_shape(other),
        }
    } else {
        supported_return(plan, method.success_type())
    }
}

fn supported_dynamic_shape(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::TypeParameter(_) => true,
        BridgeType::Array(inner) | BridgeType::Set(inner) => supported_dynamic_shape(inner),
        BridgeType::Map(key, value) => {
            supported_dynamic_shape(key) && supported_dynamic_shape(value)
        }
        BridgeType::Pair(first, second) => {
            supported_dynamic_shape(first) && supported_dynamic_shape(second)
        }
        BridgeType::Triple(first, second, third) => {
            supported_dynamic_shape(first)
                && supported_dynamic_shape(second)
                && supported_dynamic_shape(third)
        }
        BridgeType::Optional(inner) => supported_dynamic_shape(inner),
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Named {
            kind: BridgeNamedKind::Enum | BridgeNamedKind::Struct,
            ..
        } => true,
        BridgeType::Named { .. } | BridgeType::Result { .. } | BridgeType::Signal(_) => false,
    }
}

/// Codec closures follow the plugin contract: one entry for each generic
/// parameter in declaration order, then one for the generic return value.
fn method_codec_shapes(method: &BridgeMethod) -> Vec<(&BridgeType, bool)> {
    let mut codecs = method
        .parameters
        .iter()
        .map(|parameter| &parameter.ty)
        .filter(|ty| super::bridge_plan::contains_type_parameter(ty))
        .map(|ty| (ty, false))
        .collect::<Vec<_>>();
    let success = method.success_type();
    if method.row_type_parameters.is_empty() && super::bridge_plan::contains_type_parameter(success)
    {
        codecs.push((
            match success {
                BridgeType::Optional(inner) => inner.as_ref(),
                other => other,
            },
            true,
        ));
    }
    codecs
}

fn method_codec_count(method: &BridgeMethod) -> usize {
    method_codec_shapes(method).len() + method.row_type_parameters.len()
}

fn row_mapping_error_case<'a>(
    plan: &'a BridgePlan,
    method: &'a BridgeMethod,
) -> Result<(&'a str, &'a BridgeVariant), String> {
    let error_name = method.error_type().ok_or_else(|| {
        format!(
            "row-mapped method `{}` must declare a typed error",
            method.name
        )
    })?;
    let error_type = plan
        .declared_type(error_name)
        .or_else(|| {
            plan.referenced_errors
                .iter()
                .find(|ty| ty.name == error_name)
        })
        .ok_or_else(|| {
            format!(
                "row-mapped method `{}` has an unresolved error type",
                method.name
            )
        })?;
    let case_name = method.row_error_case.as_deref().ok_or_else(|| {
        format!(
            "row-mapped method `{}` must declare a row mapping error case",
            method.name
        )
    })?;
    let case = error_type
        .cases
        .iter()
        .find(|case| case.name == case_name)
        .ok_or_else(|| {
            format!("row mapping error case `{case_name}` is not declared by `{error_name}`")
        })?;
    Ok((error_name, case))
}

fn row_cell_enum<'a>(
    plan: &'a BridgePlan,
    method: &BridgeMethod,
) -> Result<&'a BridgeNamedType, String> {
    let Some(BridgeType::Named {
        name,
        kind: BridgeNamedKind::Enum,
    }) = method.row_value_type.as_ref()
    else {
        return Err(format!(
            "row-mapped method `{}` needs a declared enum cell type",
            method.name
        ));
    };
    let cell_type = plan
        .declared_type(name)
        .ok_or_else(|| format!("row cell enum `{name}` is not declared"))?;
    let null_cases = cell_type
        .cases
        .iter()
        .filter(|case| case.parameters.is_empty())
        .count();
    if null_cases != 1 || cell_type.cases.iter().any(|case| {
        !case.parameters.is_empty()
            && !matches!(case.parameters.as_slice(), [parameter] if row_cell_scalar(&parameter.ty))
    }) {
        return Err(format!(
            "row cell enum `{name}` must have one null case and scalar payloads on every other case"
        ));
    }
    Ok(cell_type)
}

fn row_cell_scalar(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
                | BridgeScalar::String
                | BridgeScalar::Bytes
        )
    )
}

fn swift_dev_row_cell_cases(cell_type: &BridgeNamedType) -> String {
    cell_type
        .cases
        .iter()
        .map(|case| match case.parameters.as_slice() {
            [] => format!("case .{}: return NSNull()", case.name),
            [parameter] if parameter.ty == BridgeType::Scalar(BridgeScalar::Bytes) => format!(
                "case .{}(let nexaPayload): return nexaPayload.base64EncodedString()",
                case.name
            ),
            [_] => format!("case .{}(let nexaPayload): return nexaPayload", case.name),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n                                    ")
}

fn kotlin_dev_row_cell_cases(cell_type: &BridgeNamedType) -> String {
    cell_type
        .cases
        .iter()
        .map(|case| match case.parameters.as_slice() {
            [] => format!("{0}.{1} -> JSONObject.NULL", cell_type.name, case.name),
            [parameter] if parameter.ty == BridgeType::Scalar(BridgeScalar::Bytes) => format!(
                "is {0}.{1} -> Base64.encodeToString(nexaCell.{2}, Base64.NO_WRAP)",
                cell_type.name, case.name, parameter.name
            ),
            [parameter] => format!(
                "is {0}.{1} -> nexaCell.{2}",
                cell_type.name, case.name, parameter.name
            ),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n                            ")
}

fn swift_dev_row_mapper(
    row_codec_index: usize,
    method: &BridgeMethod,
    plan: &BridgePlan,
) -> Result<String, String> {
    let cell_type = row_cell_enum(plan, method)?;
    let (error_type, error_case) = row_mapping_error_case(plan, method)?;
    let payload_name = &error_case.parameters[0].name;
    let error = format!(
        "throw {error_type}.{}({payload_name}: \"{{message}}\")",
        error_case.name
    );
    let cell_cases = swift_dev_row_cell_cases(cell_type);
    Ok(format!(
        "{{ columnNames in\n                guard let nexaRowCodec = nexaCodec{row_codec_index} as? [String: Any], let nexaRowType = nexaRowCodec[\"ty\"] as? [String: Any], let nexaStruct = nexaRowType[\"Struct\"] as? [String: Any], let nexaRawFields = nexaStruct[\"fields\"] as? [[Any]] else {{ {invalid_type} }}\n                let nexaExpectedFields = nexaRawFields.compactMap {{ $0.first as? String }}\n                guard nexaExpectedFields.count == nexaRawFields.count else {{ {invalid_fields} }}\n                var nexaIndexes: [String: Int] = [:]\n                nexaIndexes.reserveCapacity(nexaExpectedFields.count)\n                for nexaField in nexaExpectedFields {{ guard let nexaIndex = columnNames.firstIndex(of: nexaField), columnNames.lastIndex(of: nexaField) == nexaIndex else {{ {missing_column} }}; nexaIndexes[nexaField] = nexaIndex }}\n                return {{ row in var nexaValues: [String: Any] = [:]; nexaValues.reserveCapacity(nexaIndexes.count); for (nexaField, nexaIndex) in nexaIndexes {{ let nexaCell = try row.value(Int32(nexaIndex)); let nexaDynamicValue: Any = {{ switch nexaCell {{\n                                    {cell_cases}\n                                    }} }}(); nexaValues[nexaField] = nexaDynamicValue }}; return NexaDevDynamicRow(fields: nexaValues) }}\n            }}",
        invalid_type = error.replace("{message}", "Invalid hot-reload row type."),
        invalid_fields = error.replace("{message}", "Invalid hot-reload row fields."),
        missing_column = error.replace(
            "{message}",
            "Row mapper requires column `\\(nexaField)` exactly once."
        ),
    ))
}

fn kotlin_dev_row_mapper(
    row_codec_index: usize,
    method: &BridgeMethod,
    plan: &BridgePlan,
) -> Result<String, String> {
    let cell_type = row_cell_enum(plan, method)?;
    row_mapping_error_case(plan, method)?;
    let error = "throw rowFailure(\"{message}\")";
    let cell_cases = kotlin_dev_row_cell_cases(cell_type);
    Ok(format!(
        "{{ columnNames, rowFailure ->\n                val nexaRowCodec = nexaCodec{row_codec_index} as? JSONObject ?: run {{ {invalid_type} }}\n                val nexaRowType = nexaRowCodec.optJSONObject(\"ty\")?.optJSONObject(\"Struct\") ?: run {{ {invalid_type} }}\n                val nexaRawFields = nexaRowType.optJSONArray(\"fields\") ?: run {{ {invalid_fields} }}\n                val nexaExpectedFields = (0 until nexaRawFields.length()).mapNotNull {{ nexaRawFields.optJSONArray(it)?.optString(0)?.takeIf(String::isNotEmpty) }}\n                if (nexaExpectedFields.size != nexaRawFields.length()) {{ {invalid_fields} }}\n                val nexaIndexes = HashMap<String, Int>(nexaExpectedFields.size)\n                nexaExpectedFields.forEach {{ nexaField -> val nexaIndex = columnNames.indexOf(nexaField); if (nexaIndex < 0 || columnNames.lastIndexOf(nexaField) != nexaIndex) {{ {missing_column} }}; nexaIndexes[nexaField] = nexaIndex }}\n                {{ row -> val nexaValues = HashMap<String, Any>(nexaIndexes.size); nexaIndexes.forEach {{ (nexaField, nexaIndex) -> val nexaCell = row.value(nexaIndex); val nexaDynamicValue: Any = when (nexaCell) {{\n                            {cell_cases}\n                        }}; nexaValues[nexaField] = nexaDynamicValue }}; NexaDevDynamicRow(nexaValues) }}\n            }}",
        invalid_type = error.replace("{message}", "Invalid hot-reload row type."),
        invalid_fields = error.replace("{message}", "Invalid hot-reload row fields."),
        missing_column = error.replace(
            "{message}",
            "Row mapper requires column `$nexaField` exactly once."
        ),
    ))
}

fn kotlin_dynamic_type(ty: &BridgeType) -> Option<String> {
    Some(match ty {
        // A generic app type may be nullable even when the plugin IDL only
        // declares a type parameter. Keep the dynamic bridge's erased type
        // nullable so a valid null value can travel through its tagged result.
        BridgeType::TypeParameter(_) => "Any?".to_owned(),
        BridgeType::Scalar(BridgeScalar::Void) => return None,
        BridgeType::Scalar(_) => kotlin_type_for_dev(ty),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum | BridgeNamedKind::Struct,
        } => name.clone(),
        BridgeType::Array(inner) => format!("List<{}>", kotlin_dynamic_type(inner)?),
        BridgeType::Set(inner) => format!("Set<{}>", kotlin_dynamic_type(inner)?),
        BridgeType::Map(key, value) => format!(
            "Map<{}, {}>",
            kotlin_dynamic_type(key)?,
            kotlin_dynamic_type(value)?
        ),
        BridgeType::Pair(first, second) => format!(
            "Pair<{}, {}>",
            kotlin_dynamic_type(first)?,
            kotlin_dynamic_type(second)?
        ),
        BridgeType::Triple(first, second, third) => format!(
            "Triple<{}, {}, {}>",
            kotlin_dynamic_type(first)?,
            kotlin_dynamic_type(second)?,
            kotlin_dynamic_type(third)?
        ),
        BridgeType::Optional(inner) => {
            let inner = kotlin_dynamic_type(inner)?;
            if inner.ends_with('?') {
                inner
            } else {
                format!("{inner}?")
            }
        }
        BridgeType::Named { .. } | BridgeType::Result { .. } | BridgeType::Signal(_) => {
            return None;
        }
    })
}

fn render_swift_instance_properties(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str(
        "    static func readInstanceProperty(receiver: Any, property: String) -> (Bool, Any) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !supported_runtime_value_type(&plan, &property.ty) {
                    continue;
                }
                let encoded =
                    swift_encode_dev_value(&plan, &property.ty, "nexaValue", namespace, 0)
                        .ok_or_else(|| {
                            format!("unsupported Dev property type {:?}", property.ty)
                        })?;
                out.push_str(&format!(
                    "        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n            let nexaValue = nexaReceiver.{}\n            return (true, {})\n        }}\n",
                    swift_escape(&property.name), interface.name, property.name,
                    encoded
                ));
            }
        }
    }
    out.push_str("        return (false, NSNull())\n    }\n\n    static func writeInstanceProperty(receiver: Any, property: String, value: Any) -> Bool {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !property.mutable || !supported_runtime_value_type(&plan, &property.ty) {
                    continue;
                }
                out.push_str(&format!(
                    "        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n",
                    swift_escape(&property.name),
                    interface.name
                ));
                render_swift_value_binding(
                    out,
                    &plan,
                    &property.ty,
                    "value",
                    "nexaValue",
                    "return false",
                    namespace,
                    "            ",
                )?;
                out.push_str(&format!("            nexaReceiver.{} = nexaValue\n            return true\n        }}\n", property.name));
            }
        }
    }
    out.push_str("        return false\n    }\n\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        render_swift_enum_codecs(out, namespace, &plan)?;
        render_swift_struct_codecs(out, namespace, &plan)?;
    }
    Ok(())
}

fn render_swift_instance_events(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str("    static func subscribeInstanceEvent(receiver: Any, property: String, handler: @escaping ([Any]) -> Void) -> Bool {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for event in &interface.events {
                if !event
                    .parameters
                    .iter()
                    .all(|parameter| supported_runtime_value_type(&plan, &parameter.ty))
                {
                    continue;
                }
                let property = nexa_plugin_idl::event_callback_property(&event.name);
                out.push_str(&format!(
                    "        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n            nexaReceiver.{} = {{",
                    swift_escape(&property), interface.name, property
                ));
                for (index, _) in event.parameters.iter().enumerate() {
                    out.push_str(&format!(" nexaEvent{index}"));
                    if index + 1 < event.parameters.len() {
                        out.push(',');
                    }
                }
                if !event.parameters.is_empty() {
                    out.push_str(" in");
                }
                out.push_str("\n                handler([");
                let encoded = event
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, parameter)| {
                        swift_encode_dev_value(
                            &plan,
                            &parameter.ty,
                            &format!("nexaEvent{index}"),
                            namespace,
                            0,
                        )
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| "unsupported Dev event value type".to_owned())?;
                out.push_str(&encoded.join(", "));
                out.push_str("])\n            }\n            return true\n        }\n");
            }
        }
    }
    out.push_str("        return false\n    }\n\n    static func clearInstanceEvent(receiver: Any, property: String) -> Bool {\n");
    for (_namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for event in &interface.events {
                let property = nexa_plugin_idl::event_callback_property(&event.name);
                out.push_str(&format!("        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n            nexaReceiver.{} = nil\n            return true\n        }}\n", swift_escape(&property), interface.name, property));
            }
        }
    }
    out.push_str("        return false\n    }\n\n");
    Ok(())
}

fn render_swift_components(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str("    static func renderComponent(namespace: String, name: String, arguments: [String: Any], events: [String: ([Any]) -> Void], content: AnyView) -> AnyView? {\n        switch (namespace, name) {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeComponent)
        {
            if !interface
                .properties
                .iter()
                .all(|property| supported_runtime_value_type(&plan, &property.ty))
                || !interface.events.iter().all(|event| {
                    event
                        .parameters
                        .iter()
                        .all(|parameter| supported_runtime_value_type(&plan, &parameter.ty))
                })
            {
                continue;
            }
            out.push_str(&format!(
                "        case (\"{}\", \"{}\"):\n",
                swift_escape(namespace),
                swift_escape(&interface.name)
            ));
            for property in &interface.properties {
                let raw = format!("arguments[\"{}\"]", swift_escape(&property.name));
                if let Some(default) = &property.default {
                    out.push_str(&format!("            let nexaRaw_{} = {raw}\n            let nexaArg_{}: {}\n            if nexaRaw_{} == nil {{ nexaArg_{} = {} }} else {{\n", property.name, property.name, swift_type_for_dev(&property.ty), property.name, property.name, swift_literal_for_dev(default)));
                    render_swift_value_binding(
                        out,
                        &plan,
                        &property.ty,
                        &format!("nexaRaw_{}", property.name),
                        &format!("nexaDecoded_{}", property.name),
                        "return nil",
                        namespace,
                        "                ",
                    )?;
                    out.push_str(&format!(
                        "                nexaArg_{} = nexaDecoded_{}\n            }}\n",
                        property.name, property.name
                    ));
                } else {
                    render_swift_value_binding(
                        out,
                        &plan,
                        &property.ty,
                        &raw,
                        &format!("nexaArg_{}", property.name),
                        "return nil",
                        namespace,
                        "            ",
                    )?;
                }
            }
            let mut call_arguments = interface
                .properties
                .iter()
                .map(|property| format!("{}: nexaArg_{}", property.name, property.name))
                .collect::<Vec<_>>();
            for event in &interface.events {
                let callback_property = nexa_plugin_idl::event_callback_property(&event.name);
                let callback = if event.parameters.is_empty() {
                    format!(
                        "events[\"{callback_property}\"].map {{ handler in {{ handler([]) }} }}"
                    )
                } else {
                    let params = event
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, _)| format!("nexaEvent{index}"))
                        .collect::<Vec<_>>();
                    let encoded = event
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, parameter)| {
                            swift_encode_dev_value(
                                &plan,
                                &parameter.ty,
                                &format!("nexaEvent{index}"),
                                namespace,
                                0,
                            )
                        })
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| "unsupported Dev component event value type".to_owned())?;
                    let closure = format!(
                        "{{ {} in handler([{}]) }}",
                        params.join(", "),
                        encoded.join(", ")
                    );
                    format!("events[\"{callback_property}\"].map {{ handler in {closure} }}")
                };
                call_arguments.push(format!("{callback_property}: {callback}"));
            }
            if interface.has_content_slot {
                call_arguments.push("content: { content }".to_owned());
            }
            out.push_str(&format!(
                "            return AnyView({}({}))\n",
                interface.name,
                call_arguments.join(", ")
            ));
        }
    }
    out.push_str("        default: return nil\n        }\n    }\n\n");
    Ok(())
}

fn render_kotlin_instance_properties(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str(
        "    fun readInstanceProperty(receiver: Any, property: String): Pair<Boolean, Any> {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !supported_runtime_value_type(&plan, &property.ty) {
                    continue;
                }
                let encoded =
                    kotlin_encode_dev_value(&plan, &property.ty, "nexaValue", namespace, 0)
                        .ok_or_else(|| {
                            format!("unsupported Dev property type {:?}", property.ty)
                        })?;
                out.push_str(&format!("        if (property == \"{}\" && receiver is {}) {{\n            val nexaValue = receiver.{}\n            return true to {}\n        }}\n", kotlin_escape(&property.name), interface.name, property.name, encoded));
            }
        }
    }
    out.push_str("        return false to JSONObject.NULL\n    }\n\n    fun writeInstanceProperty(receiver: Any, property: String, value: Any): Boolean {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !property.mutable || !supported_runtime_value_type(&plan, &property.ty) {
                    continue;
                }
                out.push_str(&format!(
                    "        if (property == \"{}\" && receiver is {}) {{\n",
                    kotlin_escape(&property.name),
                    interface.name
                ));
                render_kotlin_value_binding(
                    out,
                    &plan,
                    namespace,
                    &property.ty,
                    "value",
                    "nexaValue",
                    "return false",
                    "            ",
                )?;
                out.push_str(&format!(
                    "            receiver.{} = nexaValue\n            return true\n        }}\n",
                    property.name
                ));
            }
        }
    }
    out.push_str("        return false\n    }\n\n    fun clearInstanceEvent(receiver: Any, property: String): Boolean {\n");
    for (_namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for event in &interface.events {
                let property = nexa_plugin_idl::event_callback_property(&event.name);
                out.push_str(&format!("        if (property == \"{}\" && receiver is {}) {{\n            receiver.{} = null\n            return true\n        }}\n", kotlin_escape(&property), interface.name, property));
            }
        }
    }
    out.push_str("        return false\n    }\n\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        render_kotlin_enum_codecs(out, namespace, &plan)?;
        render_kotlin_struct_codecs(out, namespace, &plan)?;
    }
    Ok(())
}

fn render_kotlin_instance_events(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str("    fun subscribeInstanceEvent(receiver: Any, property: String, handler: (List<Any>) -> Unit): Boolean {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for event in &interface.events {
                if !event
                    .parameters
                    .iter()
                    .all(|parameter| supported_runtime_value_type(&plan, &parameter.ty))
                {
                    continue;
                }
                let property = nexa_plugin_idl::event_callback_property(&event.name);
                out.push_str(&format!("        if (property == \"{}\" && receiver is {}) {{\n            receiver.{} = {{", kotlin_escape(&property), interface.name, property));
                for (index, _) in event.parameters.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("nexaEvent{index}"));
                }
                out.push_str(" -> handler(listOf(");
                let encoded = event
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, parameter)| {
                        kotlin_encode_dev_value(
                            &plan,
                            &parameter.ty,
                            &format!("nexaEvent{index}"),
                            namespace,
                            0,
                        )
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| "unsupported Dev event value type".to_owned())?;
                out.push_str(&encoded.join(", "));
                out.push_str(")) }\n            return true\n        }\n");
            }
        }
    }
    out.push_str("        return false\n    }\n\n");
    Ok(())
}

fn render_kotlin_components(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str("    @Composable fun renderComponent(namespace: String, name: String, arguments: Map<String, Any>, events: Map<String, (List<Any>) -> Unit>, content: @Composable () -> Unit): Boolean {\n        when (\"$namespace.$name\") {\n");
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeComponent)
        {
            if !interface
                .properties
                .iter()
                .all(|property| supported_runtime_value_type(&plan, &property.ty))
                || !interface.events.iter().all(|event| {
                    event
                        .parameters
                        .iter()
                        .all(|parameter| supported_runtime_value_type(&plan, &parameter.ty))
                })
            {
                continue;
            }
            out.push_str(&format!(
                "            \"{}.{}\" -> {{\n",
                kotlin_escape(namespace),
                kotlin_escape(&interface.name)
            ));
            for property in &interface.properties {
                let raw = format!("arguments[\"{}\"]", kotlin_escape(&property.name));
                if let Some(default) = &property.default {
                    out.push_str(&format!("                val nexaArg_{} = if (!arguments.containsKey(\"{}\")) {} else {{\n", property.name, kotlin_escape(&property.name), kotlin_literal_for_dev(default)));
                    render_kotlin_value_binding(
                        out,
                        &plan,
                        namespace,
                        &property.ty,
                        &raw,
                        &format!("nexaDecoded_{}", property.name),
                        "return false",
                        "                    ",
                    )?;
                    out.push_str(&format!(
                        "                    nexaDecoded_{}\n                }}\n",
                        property.name
                    ));
                } else {
                    render_kotlin_value_binding(
                        out,
                        &plan,
                        namespace,
                        &property.ty,
                        &raw,
                        &format!("nexaArg_{}", property.name),
                        "return false",
                        "                ",
                    )?;
                }
            }
            let mut args = interface
                .properties
                .iter()
                .map(|property| format!("{} = nexaArg_{}", property.name, property.name))
                .collect::<Vec<_>>();
            for event in &interface.events {
                let property = nexa_plugin_idl::event_callback_property(&event.name);
                let callback = if event.parameters.is_empty() {
                    format!("{{ events[\"{property}\"]?.invoke(emptyList()) }}")
                } else {
                    let params = event
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, parameter)| {
                            format!("nexaEvent{index}: {}", kotlin_type_for_dev(&parameter.ty))
                        })
                        .collect::<Vec<_>>();
                    let encoded = event
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, parameter)| {
                            kotlin_encode_dev_value(
                                &plan,
                                &parameter.ty,
                                &format!("nexaEvent{index}"),
                                namespace,
                                0,
                            )
                        })
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| "unsupported Dev component event value type".to_owned())?;
                    format!(
                        "{{ {} -> events[\"{property}\"]?.invoke(listOf({})) }}",
                        params.join(", "),
                        encoded.join(", ")
                    )
                };
                args.push(format!("{property} = {callback}"));
            }
            if interface.has_content_slot {
                args.push("content = content".to_owned());
            }
            out.push_str(&format!(
                "                {}({})\n                return true\n            }}\n",
                interface.name,
                args.join(", ")
            ));
        }
    }
    out.push_str("        }\n        return false\n    }\n\n");
    Ok(())
}

fn supported_runtime_value_type(plan: &BridgePlan, ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Named {
            kind: BridgeNamedKind::Enum | BridgeNamedKind::NativeClass,
            ..
        } => true,
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => supported_struct_value(plan, name, 0),
        BridgeType::Optional(inner) => supported_runtime_value_type(plan, inner),
        BridgeType::Array(inner) => supported_array_element(plan, inner, 0),
        BridgeType::Set(inner) => supported_hashable_collection_element(inner),
        BridgeType::Map(key, value) => {
            supported_map_key(key) && supported_static_collection_element(plan, value, 0)
        }
        BridgeType::Pair(first, second) => {
            supported_static_collection_element(plan, first, 0)
                && supported_static_collection_element(plan, second, 0)
        }
        BridgeType::Triple(first, second, third) => {
            supported_static_collection_element(plan, first, 0)
                && supported_static_collection_element(plan, second, 0)
                && supported_static_collection_element(plan, third, 0)
        }
        _ => false,
    }
}

fn supported_static_collection_element(plan: &BridgePlan, ty: &BridgeType, depth: usize) -> bool {
    if depth >= 64 {
        return false;
    }
    match ty {
        BridgeType::Optional(inner) => {
            !matches!(inner.as_ref(), BridgeType::Optional(_))
                && supported_runtime_value_type(plan, inner)
        }
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Named {
            kind: BridgeNamedKind::Enum | BridgeNamedKind::NativeClass,
            ..
        } => true,
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => supported_struct_value(plan, name, depth + 1),
        BridgeType::Array(inner) => supported_array_element(plan, inner, depth + 1),
        BridgeType::Set(inner) => supported_hashable_collection_element(inner),
        BridgeType::Map(key, value) => {
            supported_map_key(key) && supported_static_collection_element(plan, value, depth + 1)
        }
        BridgeType::Pair(first, second) => {
            supported_static_collection_element(plan, first, depth + 1)
                && supported_static_collection_element(plan, second, depth + 1)
        }
        BridgeType::Triple(first, second, third) => {
            supported_static_collection_element(plan, first, depth + 1)
                && supported_static_collection_element(plan, second, depth + 1)
                && supported_static_collection_element(plan, third, depth + 1)
        }
        _ => false,
    }
}

fn supported_array_element(plan: &BridgePlan, ty: &BridgeType, depth: usize) -> bool {
    if depth >= 64 {
        return false;
    }
    match ty {
        BridgeType::Optional(inner) => supported_runtime_value_type(plan, inner),
        other => supported_static_collection_element(plan, other, depth + 1),
    }
}

fn supported_hashable_collection_element(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Optional(inner) => {
            !matches!(inner.as_ref(), BridgeType::Optional(_))
                && supported_hashable_collection_element(inner)
        }
        BridgeType::Scalar(
            BridgeScalar::Bool
            | BridgeScalar::Int8
            | BridgeScalar::Int16
            | BridgeScalar::Int32
            | BridgeScalar::Int64
            | BridgeScalar::UInt8
            | BridgeScalar::UInt16
            | BridgeScalar::UInt32
            | BridgeScalar::UInt64
            | BridgeScalar::Float32
            | BridgeScalar::Float64
            | BridgeScalar::String
            | BridgeScalar::Bytes
            | BridgeScalar::BufferView,
        ) => true,
        BridgeType::Named {
            kind: BridgeNamedKind::Enum,
            ..
        } => true,
        _ => false,
    }
}

fn supported_map_key(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::String
                | BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
        ) | BridgeType::Named {
            kind: BridgeNamedKind::Enum,
            ..
        }
    )
}

fn supported_struct_value(plan: &BridgePlan, name: &str, depth: usize) -> bool {
    if depth >= 64 {
        return false;
    }
    let Some(structure) = plan
        .types
        .iter()
        .find(|ty| ty.name == name && ty.kind == BridgeTypeKind::Struct)
    else {
        return false;
    };
    structure.fields.iter().all(|field| {
        let field_depth = depth + 1;
        match &field.ty {
            BridgeType::Optional(inner) => {
                supported_runtime_value_type(plan, inner) && field_depth < 64
            }
            other => supported_static_collection_element(plan, other, field_depth),
        }
    })
}

fn swift_type_for_dev(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Void) => "Void".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bool) => "Bool".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int8) => "Int8".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int16) => "Int16".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int32) => "Int32".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int64) => "Int64".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt8) => "UInt8".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt16) => "UInt16".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt32) => "UInt32".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt64) => "UInt64".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float32) => "Float".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float64) => "Double".to_owned(),
        BridgeType::Scalar(BridgeScalar::String) => "String".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bytes) => "Data".to_owned(),
        BridgeType::Scalar(BridgeScalar::BufferView) => "Data".to_owned(),
        BridgeType::Named { name, .. } => name.clone(),
        BridgeType::Array(inner) => format!("[{}]", swift_type_for_dev(inner)),
        BridgeType::Set(inner) => format!("Set<{}>", swift_type_for_dev(inner)),
        BridgeType::Map(key, value) => format!(
            "[{}: {}]",
            swift_type_for_dev(key),
            swift_type_for_dev(value)
        ),
        BridgeType::Pair(first, second) => format!(
            "({}, {})",
            swift_type_for_dev(first),
            swift_type_for_dev(second)
        ),
        BridgeType::Triple(first, second, third) => format!(
            "({}, {}, {})",
            swift_type_for_dev(first),
            swift_type_for_dev(second),
            swift_type_for_dev(third)
        ),
        BridgeType::Optional(inner) => format!("{}?", swift_type_for_dev(inner)),
        _ => "Any".to_owned(),
    }
}

fn kotlin_type_for_dev(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Void) => "Unit".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bool) => "Boolean".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int8) => "Byte".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int16) => "Short".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int32) => "Int".to_owned(),
        BridgeType::Scalar(BridgeScalar::Int64) => "Long".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt8) => "UByte".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt16) => "UShort".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt32) => "UInt".to_owned(),
        BridgeType::Scalar(BridgeScalar::UInt64) => "ULong".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float32) => "Float".to_owned(),
        BridgeType::Scalar(BridgeScalar::Float64) => "Double".to_owned(),
        BridgeType::Scalar(BridgeScalar::String) => "String".to_owned(),
        BridgeType::Scalar(BridgeScalar::Bytes) => "ByteArray".to_owned(),
        BridgeType::Scalar(BridgeScalar::BufferView) => "ByteArray".to_owned(),
        BridgeType::Named { name, .. } => name.clone(),
        BridgeType::Array(inner) => format!("List<{}>", kotlin_type_for_dev(inner)),
        BridgeType::Set(inner) => format!("Set<{}>", kotlin_type_for_dev(inner)),
        BridgeType::Map(key, value) => format!(
            "Map<{}, {}>",
            kotlin_type_for_dev(key),
            kotlin_type_for_dev(value)
        ),
        BridgeType::Optional(inner) => format!("{}?", kotlin_type_for_dev(inner)),
        _ => "Any".to_owned(),
    }
}

fn swift_literal_for_dev(literal: &nexa_plugin_idl::Literal) -> String {
    match literal {
        nexa_plugin_idl::Literal::String(value) => format!("\"{}\"", swift_escape(value)),
        nexa_plugin_idl::Literal::Number(value) => value.clone(),
        nexa_plugin_idl::Literal::Bool(value) => value.to_string(),
        nexa_plugin_idl::Literal::Null => "nil".to_owned(),
    }
}

fn kotlin_literal_for_dev(literal: &nexa_plugin_idl::Literal) -> String {
    match literal {
        nexa_plugin_idl::Literal::String(value) => format!("\"{}\"", kotlin_escape(value)),
        nexa_plugin_idl::Literal::Number(value) => value.clone(),
        nexa_plugin_idl::Literal::Bool(value) => value.to_string(),
        nexa_plugin_idl::Literal::Null => "null".to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_swift_value_binding(
    out: &mut String,
    _plan: &BridgePlan,
    ty: &BridgeType,
    raw: &str,
    name: &str,
    failure: &str,
    namespace: &str,
    indent: &str,
) -> Result<(), String> {
    if let BridgeType::Optional(inner) = ty {
        let decoder = swift_decode_value(inner, raw, namespace, 0)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
        out.push_str(&format!("{indent}let {name}: {}\n{indent}if {raw} is NSNull {{ {name} = nil }} else {{\n{indent}    guard let nexaUnwrapped = {decoder} else {{ {failure} }}\n{indent}    {name} = nexaUnwrapped\n{indent}}}\n", swift_type_for_dev(ty)));
    } else {
        let decoder = swift_decode_value(ty, raw, namespace, 0)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
        out.push_str(&format!(
            "{indent}guard let {name} = {decoder} else {{ {failure} }}\n"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_kotlin_value_binding(
    out: &mut String,
    _plan: &BridgePlan,
    namespace: &str,
    ty: &BridgeType,
    raw: &str,
    name: &str,
    failure: &str,
    indent: &str,
) -> Result<(), String> {
    let decoder = match ty {
        BridgeType::Optional(inner) => {
            let decoder = kotlin_decode_value(inner, raw, namespace, 0)
                .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
            out.push_str(&format!("{indent}val {name}: {}\n{indent}if ({raw} == null || {raw} == JSONObject.NULL) {{ {name} = null }} else {{\n", kotlin_type_for_dev(ty)));
            out.push_str(&format!("{indent}    val nexaUnwrapped = {decoder} ?: {failure}\n{indent}    {name} = nexaUnwrapped\n{indent}}}\n"));
            return Ok(());
        }
        _ => kotlin_decode_value(ty, raw, namespace, 0)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?,
    };
    out.push_str(&format!("{indent}val {name} = {decoder} ?: {failure}\n"));
    Ok(())
}

fn swift_decode_value(ty: &BridgeType, raw: &str, namespace: &str, depth: usize) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(_) => format!("Self.{}({raw})", swift_decoder(ty)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::NativeClass,
        } => format!("{raw} as? {name}"),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("Self.{}({raw})", swift_enum_decoder(namespace, name)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => format!("Self.{}({raw})", swift_struct_decoder(namespace, name)),
        BridgeType::Array(inner) => {
            let item = format!("nexaDevArrayItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded = swift_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformOptionalArray({raw}, decode: {{ {item} in {decoded} }})"
                )
            } else {
                let decoded = swift_decode_value(inner, &item, namespace, depth + 1)?;
                format!(
                    "({raw} as? [Any]).flatMap({{ values in let decoded = values.compactMap({{ {item} in {decoded} }}); return decoded.count == values.count ? decoded : nil }})"
                )
            }
        }
        BridgeType::Set(inner) => {
            let item = format!("nexaDevSetItem{depth}");
            let decoded = swift_decode_value(inner, &item, namespace, depth + 1)?;
            format!("NexaDevValueCodec.transformSet({raw}, decode: {{ {item} in {decoded} }})")
        }
        BridgeType::Map(key, value) => {
            let raw_key = format!("nexaDevMapKey{depth}");
            let raw_value = format!("nexaDevMapValue{depth}");
            let decoded_key = swift_decode_map_key(key, &raw_key, namespace)?;
            let decoded_value = swift_decode_value(value, &raw_value, namespace, depth + 1)?;
            format!(
                "NexaDevValueCodec.transformMap({raw}, decodeKey: {{ {raw_key} in {decoded_key} }}, decodeValue: {{ {raw_value} in {decoded_value} }})"
            )
        }
        BridgeType::Pair(first, second) => {
            let raw_first = format!("nexaDevPairFirst{depth}");
            let raw_second = format!("nexaDevPairSecond{depth}");
            let decoded_first = swift_decode_value(first, &raw_first, namespace, depth + 1)?;
            let decoded_second = swift_decode_value(second, &raw_second, namespace, depth + 1)?;
            format!(
                "NexaDevValueCodec.transformPair({raw}, decodeFirst: {{ {raw_first} in {decoded_first} }}, decodeSecond: {{ {raw_second} in {decoded_second} }})"
            )
        }
        BridgeType::Triple(first, second, third) => {
            let raw_first = format!("nexaDevTripleFirst{depth}");
            let raw_second = format!("nexaDevTripleSecond{depth}");
            let raw_third = format!("nexaDevTripleThird{depth}");
            let decoded_first = swift_decode_value(first, &raw_first, namespace, depth + 1)?;
            let decoded_second = swift_decode_value(second, &raw_second, namespace, depth + 1)?;
            let decoded_third = swift_decode_value(third, &raw_third, namespace, depth + 1)?;
            format!(
                "NexaDevValueCodec.transformTriple({raw}, decodeFirst: {{ {raw_first} in {decoded_first} }}, decodeSecond: {{ {raw_second} in {decoded_second} }}, decodeThird: {{ {raw_third} in {decoded_third} }})"
            )
        }
        BridgeType::Optional(inner) => {
            let item = format!("nexaDevOptionalValue{depth}");
            let decoded = swift_decode_value(inner, &item, namespace, depth + 1)?;
            format!("NexaDevValueCodec.transformOptional({raw}, decode: {{ {item} in {decoded} }})")
        }
        _ => return None,
    })
}

fn swift_decode_map_key(ty: &BridgeType, raw: &str, namespace: &str) -> Option<String> {
    let parse = |name: &str| format!("({raw} as? String).flatMap {{ {name}($0) }}");
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::String) => format!("{raw} as? String"),
        BridgeType::Scalar(BridgeScalar::Bool) => format!(
            "({raw} as? String).flatMap {{ value in value == \"true\" ? true : value == \"false\" ? false : nil }}"
        ),
        BridgeType::Scalar(BridgeScalar::Int8) => parse("Int8"),
        BridgeType::Scalar(BridgeScalar::Int16) => parse("Int16"),
        BridgeType::Scalar(BridgeScalar::Int32) => parse("Int32"),
        BridgeType::Scalar(BridgeScalar::Int64) => parse("Int64"),
        BridgeType::Scalar(BridgeScalar::UInt8) => parse("UInt8"),
        BridgeType::Scalar(BridgeScalar::UInt16) => parse("UInt16"),
        BridgeType::Scalar(BridgeScalar::UInt32) => parse("UInt32"),
        BridgeType::Scalar(BridgeScalar::UInt64) => parse("UInt64"),
        BridgeType::Scalar(BridgeScalar::Float32) => parse("Float"),
        BridgeType::Scalar(BridgeScalar::Float64) => parse("Double"),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("Self.{}({raw})", swift_enum_decoder(namespace, name)),
        _ => return None,
    })
}

fn unwrap_optional(ty: &BridgeType) -> (bool, &BridgeType) {
    match ty {
        BridgeType::Optional(inner) => (true, inner.as_ref()),
        other => (false, other),
    }
}

fn swift_dynamic_decode_value(
    ty: &BridgeType,
    raw: &str,
    namespace: &str,
    depth: usize,
) -> Option<String> {
    Some(match ty {
        BridgeType::TypeParameter(_) => format!("NexaDevValueCodec.box({raw})"),
        BridgeType::Array(inner) => {
            let item = format!("nexaDevDynamicItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded =
                    swift_dynamic_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformOptionalArray({raw}, decode: {{ {item} in {decoded} }})"
                )
            } else {
                let decoded = swift_dynamic_decode_value(inner, &item, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformArray({raw}, decode: {{ {item} in {decoded} }})"
                )
            }
        }
        BridgeType::Set(inner) if super::bridge_plan::contains_type_parameter(inner) => {
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let item = format!("nexaDevDynamicSetItem{depth}");
                let decoded =
                    swift_dynamic_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformOptionalSet({raw}, decode: {{ {item} in {decoded} }})"
                )
            } else {
                format!("NexaDevValueCodec.asSet({raw})")
            }
        }
        BridgeType::Map(key, value) => {
            let raw_key = format!("nexaDevDynamicKey{depth}");
            let raw_value = format!("nexaDevDynamicValue{depth}");
            let decoded_key = swift_dynamic_decode_value(key, &raw_key, namespace, depth + 1)?;
            if let BridgeType::Optional(optional_value) = value.as_ref() {
                let decoded_value =
                    swift_dynamic_decode_value(optional_value, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformMap({raw}, decodeKey: {{ {raw_key} in {decoded_key} }}, decodeValue: {{ {raw_value} in {decoded_value} }})"
                )
            } else {
                let decoded_value =
                    swift_dynamic_decode_value(value, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformMap({raw}, decodeKey: {{ {raw_key} in {decoded_key} }}, decodeValue: {{ {raw_value} in {decoded_value} }})"
                )
            }
        }
        BridgeType::Pair(first, second) => {
            let raw_first = format!("nexaDevDynamicPairFirst{depth}");
            let raw_second = format!("nexaDevDynamicPairSecond{depth}");
            let decoded_first =
                swift_dynamic_decode_value(first, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                swift_dynamic_decode_value(second, &raw_second, namespace, depth + 1)?;
            format!(
                "NexaDevValueCodec.transformPair({raw}, decodeFirst: {{ {raw_first} in {decoded_first} }}, decodeSecond: {{ {raw_second} in {decoded_second} }})"
            )
        }
        BridgeType::Triple(first, second, third) => {
            let raw_first = format!("nexaDevDynamicTripleFirst{depth}");
            let raw_second = format!("nexaDevDynamicTripleSecond{depth}");
            let raw_third = format!("nexaDevDynamicTripleThird{depth}");
            let decoded_first =
                swift_dynamic_decode_value(first, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                swift_dynamic_decode_value(second, &raw_second, namespace, depth + 1)?;
            let decoded_third =
                swift_dynamic_decode_value(third, &raw_third, namespace, depth + 1)?;
            format!(
                "NexaDevValueCodec.transformTriple({raw}, decodeFirst: {{ {raw_first} in {decoded_first} }}, decodeSecond: {{ {raw_second} in {decoded_second} }}, decodeThird: {{ {raw_third} in {decoded_third} }})"
            )
        }
        BridgeType::Optional(inner) => {
            let item = format!("nexaDevDynamicOptional{depth}");
            let decoded = swift_dynamic_decode_value(inner, &item, namespace, depth + 1)?;
            format!("NexaDevValueCodec.transformOptional({raw}, decode: {{ {item} in {decoded} }})")
        }
        other => swift_decode_value(other, raw, namespace, depth)?,
    })
}

fn kotlin_decode_value(
    ty: &BridgeType,
    raw: &str,
    namespace: &str,
    depth: usize,
) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::String) => format!("{raw} as? String"),
        BridgeType::Scalar(BridgeScalar::Bool) => format!("{raw} as? Boolean"),
        BridgeType::Scalar(BridgeScalar::Bytes | BridgeScalar::BufferView) => {
            format!("decodeBytes({raw})")
        }
        BridgeType::Scalar(scalar) if kotlin_number_conversion(*scalar).is_some() => format!(
            "({raw} as? Number)?.{}()",
            kotlin_number_conversion(*scalar)?
        ),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::NativeClass,
        } => format!("{raw} as? {name}"),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("{}({raw})", kotlin_enum_decoder(namespace, name)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => format!(
            "decode{}{}({raw})",
            dev_struct_suffix(namespace, name),
            "FromValue"
        ),
        BridgeType::Array(inner) => {
            let item = format!("nexaDevArrayItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded = kotlin_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformOptionalArray({raw}) {{ {item} -> {decoded} }}")
            } else {
                let decoded = kotlin_decode_value(inner, &item, namespace, depth + 1)?;
                format!(
                    "({raw} as? List<*>)?.let {{ values -> val decoded = values.mapNotNull {{ {item} -> {decoded} }}; decoded.takeIf {{ it.size == values.size }} }}"
                )
            }
        }
        BridgeType::Set(inner) => {
            let item = format!("nexaDevSetItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded = kotlin_decode_value(optional_inner, &item, namespace, depth + 1)?;
                if matches!(
                    optional_inner.as_ref(),
                    BridgeType::Scalar(BridgeScalar::Bytes)
                ) {
                    format!(
                        "NexaDevValueCodec.transformOptionalByteArraySet({raw}) {{ {item} -> {decoded} }}"
                    )
                } else {
                    format!(
                        "NexaDevValueCodec.transformOptionalSet({raw}) {{ {item} -> {decoded} }}"
                    )
                }
            } else if matches!(inner.as_ref(), BridgeType::Scalar(BridgeScalar::Bytes)) {
                let decoded = kotlin_decode_value(inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformByteArraySet({raw}) {{ {item} -> {decoded} }}")
            } else {
                let decoded = kotlin_decode_value(inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformSet({raw}) {{ {item} -> {decoded} }}")
            }
        }
        BridgeType::Map(key, value) => {
            let raw_key = format!("nexaDevMapKey{depth}");
            let raw_value = format!("nexaDevMapValue{depth}");
            let decoded_key = kotlin_decode_map_key(key, &raw_key)?;
            if let BridgeType::Optional(inner) = value.as_ref() {
                let decoded_value = kotlin_decode_value(inner, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformOptionalValueMap({raw}, decodeKey = {{ {raw_key} -> {decoded_key} }}, decodeValue = {{ {raw_value} -> {decoded_value} }})"
                )
            } else {
                let decoded_value = kotlin_decode_value(value, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformMap({raw}, decodeKey = {{ {raw_key} -> {decoded_key} }}, decodeValue = {{ {raw_value} -> {decoded_value} }})"
                )
            }
        }
        BridgeType::Pair(first, second) => {
            let raw_first = format!("nexaDevPairFirst{depth}");
            let raw_second = format!("nexaDevPairSecond{depth}");
            let (optional_first, first_value) = unwrap_optional(first);
            let (optional_second, second_value) = unwrap_optional(second);
            let decoded_first = kotlin_decode_value(first_value, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                kotlin_decode_value(second_value, &raw_second, namespace, depth + 1)?;
            if optional_first || optional_second {
                format!(
                    "(NexaDevValueCodec.transformPairNullable({raw}, optionalFirst = {optional_first}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, optionalSecond = {optional_second}, decodeSecond = {{ {raw_second} -> {decoded_second} }}) as? {})",
                    kotlin_type_for_dev(ty)
                )
            } else {
                format!(
                    "NexaDevValueCodec.transformPair({raw}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, decodeSecond = {{ {raw_second} -> {decoded_second} }})"
                )
            }
        }
        BridgeType::Triple(first, second, third) => {
            let raw_first = format!("nexaDevTripleFirst{depth}");
            let raw_second = format!("nexaDevTripleSecond{depth}");
            let raw_third = format!("nexaDevTripleThird{depth}");
            let (optional_first, first_value) = unwrap_optional(first);
            let (optional_second, second_value) = unwrap_optional(second);
            let (optional_third, third_value) = unwrap_optional(third);
            let decoded_first = kotlin_decode_value(first_value, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                kotlin_decode_value(second_value, &raw_second, namespace, depth + 1)?;
            let decoded_third = kotlin_decode_value(third_value, &raw_third, namespace, depth + 1)?;
            if optional_first || optional_second || optional_third {
                format!(
                    "(NexaDevValueCodec.transformTripleNullable({raw}, optionalFirst = {optional_first}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, optionalSecond = {optional_second}, decodeSecond = {{ {raw_second} -> {decoded_second} }}, optionalThird = {optional_third}, decodeThird = {{ {raw_third} -> {decoded_third} }}) as? {})",
                    kotlin_type_for_dev(ty)
                )
            } else {
                format!(
                    "NexaDevValueCodec.transformTriple({raw}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, decodeSecond = {{ {raw_second} -> {decoded_second} }}, decodeThird = {{ {raw_third} -> {decoded_third} }})"
                )
            }
        }
        _ => return None,
    })
}

fn kotlin_dynamic_decode_value(
    ty: &BridgeType,
    raw: &str,
    namespace: &str,
    depth: usize,
) -> Option<String> {
    Some(match ty {
        BridgeType::TypeParameter(_) => raw.to_owned(),
        BridgeType::Array(inner) => {
            let item = format!("nexaDevDynamicItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded =
                    kotlin_dynamic_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformOptionalArray({raw}) {{ {item} -> {decoded} }}")
            } else {
                let decoded = kotlin_dynamic_decode_value(inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformArray({raw}) {{ {item} -> {decoded} }}")
            }
        }
        BridgeType::Set(inner) if super::bridge_plan::contains_type_parameter(inner) => {
            let item = format!("nexaDevDynamicSetItem{depth}");
            if let BridgeType::Optional(optional_inner) = inner.as_ref() {
                let decoded =
                    kotlin_dynamic_decode_value(optional_inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformOptionalSet({raw}) {{ {item} -> {decoded} }}")
            } else {
                let decoded = kotlin_dynamic_decode_value(inner, &item, namespace, depth + 1)?;
                format!("NexaDevValueCodec.transformSet({raw}) {{ {item} -> {decoded} }}")
            }
        }
        BridgeType::Map(key, value) => {
            let raw_key = format!("nexaDevDynamicKey{depth}");
            let raw_value = format!("nexaDevDynamicValue{depth}");
            let decoded_key = kotlin_dynamic_decode_value(key, &raw_key, namespace, depth + 1)?;
            if let BridgeType::Optional(inner) = value.as_ref() {
                let decoded_value =
                    kotlin_dynamic_decode_value(inner, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformOptionalValueMap({raw}, decodeKey = {{ {raw_key} -> {decoded_key} }}, decodeValue = {{ {raw_value} -> {decoded_value} }})"
                )
            } else {
                let decoded_value =
                    kotlin_dynamic_decode_value(value, &raw_value, namespace, depth + 1)?;
                format!(
                    "NexaDevValueCodec.transformMap({raw}, decodeKey = {{ {raw_key} -> {decoded_key} }}, decodeValue = {{ {raw_value} -> {decoded_value} }})"
                )
            }
        }
        BridgeType::Pair(first, second) => {
            let raw_first = format!("nexaDevDynamicPairFirst{depth}");
            let raw_second = format!("nexaDevDynamicPairSecond{depth}");
            let (optional_first, first_value) = unwrap_optional(first);
            let (optional_second, second_value) = unwrap_optional(second);
            let decoded_first =
                kotlin_dynamic_decode_value(first_value, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                kotlin_dynamic_decode_value(second_value, &raw_second, namespace, depth + 1)?;
            if optional_first || optional_second {
                format!(
                    "(NexaDevValueCodec.transformPairNullable({raw}, optionalFirst = {optional_first}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, optionalSecond = {optional_second}, decodeSecond = {{ {raw_second} -> {decoded_second} }}) as? {})",
                    kotlin_dynamic_type(ty)?
                )
            } else {
                format!(
                    "NexaDevValueCodec.transformPair({raw}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, decodeSecond = {{ {raw_second} -> {decoded_second} }})"
                )
            }
        }
        BridgeType::Triple(first, second, third) => {
            let raw_first = format!("nexaDevDynamicTripleFirst{depth}");
            let raw_second = format!("nexaDevDynamicTripleSecond{depth}");
            let raw_third = format!("nexaDevDynamicTripleThird{depth}");
            let (optional_first, first_value) = unwrap_optional(first);
            let (optional_second, second_value) = unwrap_optional(second);
            let (optional_third, third_value) = unwrap_optional(third);
            let decoded_first =
                kotlin_dynamic_decode_value(first_value, &raw_first, namespace, depth + 1)?;
            let decoded_second =
                kotlin_dynamic_decode_value(second_value, &raw_second, namespace, depth + 1)?;
            let decoded_third =
                kotlin_dynamic_decode_value(third_value, &raw_third, namespace, depth + 1)?;
            if optional_first || optional_second || optional_third {
                format!(
                    "(NexaDevValueCodec.transformTripleNullable({raw}, optionalFirst = {optional_first}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, optionalSecond = {optional_second}, decodeSecond = {{ {raw_second} -> {decoded_second} }}, optionalThird = {optional_third}, decodeThird = {{ {raw_third} -> {decoded_third} }}) as? {})",
                    kotlin_dynamic_type(ty)?
                )
            } else {
                format!(
                    "NexaDevValueCodec.transformTriple({raw}, decodeFirst = {{ {raw_first} -> {decoded_first} }}, decodeSecond = {{ {raw_second} -> {decoded_second} }}, decodeThird = {{ {raw_third} -> {decoded_third} }})"
                )
            }
        }
        other => kotlin_decode_value(other, raw, namespace, depth)?,
    })
}

fn kotlin_decode_map_key(ty: &BridgeType, raw: &str) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::String) => format!("{raw} as? String"),
        BridgeType::Scalar(BridgeScalar::Bool) => format!(
            "({raw} as? String)?.let {{ value -> when (value) {{ \"true\" -> true; \"false\" -> false; else -> null }} }}"
        ),
        BridgeType::Scalar(BridgeScalar::Int8) => format!("({raw} as? String)?.toByteOrNull()"),
        BridgeType::Scalar(BridgeScalar::Int16) => format!("({raw} as? String)?.toShortOrNull()"),
        BridgeType::Scalar(BridgeScalar::Int32) => format!("({raw} as? String)?.toIntOrNull()"),
        BridgeType::Scalar(BridgeScalar::Int64) => format!("({raw} as? String)?.toLongOrNull()"),
        BridgeType::Scalar(BridgeScalar::UInt8) => format!("({raw} as? String)?.toUByteOrNull()"),
        BridgeType::Scalar(BridgeScalar::UInt16) => format!("({raw} as? String)?.toUShortOrNull()"),
        BridgeType::Scalar(BridgeScalar::UInt32) => format!("({raw} as? String)?.toUIntOrNull()"),
        BridgeType::Scalar(BridgeScalar::UInt64) => format!("({raw} as? String)?.toULongOrNull()"),
        BridgeType::Scalar(BridgeScalar::Float32) => format!("({raw} as? String)?.toFloatOrNull()"),
        BridgeType::Scalar(BridgeScalar::Float64) => {
            format!("({raw} as? String)?.toDoubleOrNull()")
        }
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!(
            "({raw} as? {name}) ?: ({raw} as? String)?.let {{ requested -> {name}.values().firstOrNull {{ it.name == requested }} }}"
        ),
        _ => return None,
    })
}

fn swift_struct_decoder(namespace: &str, name: &str) -> String {
    format!(
        "nexaDevDecode{}FromValue",
        dev_struct_suffix(namespace, name)
    )
}

fn dev_struct_suffix(namespace: &str, name: &str) -> String {
    namespace
        .chars()
        .chain(name.chars())
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn swift_encode_value(ty: &BridgeType, value: &str, namespace: &str) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bytes | BridgeScalar::BufferView) => {
            format!("{value}.base64EncodedString()")
        }
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("Self.{}({value})", swift_enum_encoder(namespace, name)),
        BridgeType::Optional(inner) => format!(
            "{value}.map {{ {} as Any }} ?? NSNull()",
            swift_encode_value(inner, "$0", namespace)
        ),
        _ => value.to_owned(),
    }
}

fn swift_encode_dev_value(
    plan: &BridgePlan,
    ty: &BridgeType,
    value: &str,
    namespace: &str,
    depth: usize,
) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::Bytes | BridgeScalar::BufferView) => {
            format!("{value}.base64EncodedString()")
        }
        BridgeType::Scalar(_) => value.to_owned(),
        BridgeType::TypeParameter(_) => {
            format!("({value} as? NexaDevHashableValue)?.value ?? {value}")
        }
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("Self.{}({value})", swift_enum_encoder(namespace, name)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => {
            let structure = plan
                .types
                .iter()
                .find(|item| item.name == *name && item.kind == BridgeTypeKind::Struct)?;
            let fields = structure
                .fields
                .iter()
                .map(|field| {
                    Some(format!(
                        "\"{}\": {} as Any",
                        swift_escape(&field.name),
                        swift_encode_dev_value(
                            plan,
                            &field.ty,
                            &format!("{value}.{}", field.name),
                            namespace,
                            depth + 1,
                        )?
                    ))
                })
                .collect::<Option<Vec<_>>>()?
                .join(", ");
            format!("[{fields}]")
        }
        BridgeType::Named {
            kind: BridgeNamedKind::NativeClass,
            ..
        } => value.to_owned(),
        BridgeType::Optional(inner) => {
            let local = format!("nexaEncodeOptional{depth}");
            format!(
                "{value}.map {{ {local} in {} as Any }} ?? NSNull()",
                swift_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Array(inner) => {
            let local = format!("nexaEncodeArray{depth}");
            format!(
                "{value}.map {{ {local} in {} }}",
                swift_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Set(inner) => {
            let local = format!("nexaEncodeSet{depth}");
            format!(
                "Set({value}.map {{ {local} in NexaDevHashableValue({}) }})",
                swift_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Map(key_type, inner) => {
            let entry = format!("nexaEncodeMapEntry{depth}");
            let key = if super::bridge_plan::contains_type_parameter(key_type) {
                format!("({entry}.key as? NexaDevHashableValue)?.value ?? {entry}.key")
            } else {
                swift_encode_dev_value(
                    plan,
                    key_type,
                    &format!("{entry}.key"),
                    namespace,
                    depth + 1,
                )?
            };
            let encoded = swift_encode_dev_value(
                plan,
                inner,
                &format!("{entry}.value"),
                namespace,
                depth + 1,
            )?;
            format!(
                "Dictionary(uniqueKeysWithValues: {value}.map {{ {entry} in (String(describing: {key}), {encoded} as Any) }})"
            )
        }
        BridgeType::Pair(first, second) => format!(
            "[{} as Any, {} as Any]",
            swift_encode_dev_value(plan, first, &format!("{value}.0"), namespace, depth + 1)?,
            swift_encode_dev_value(plan, second, &format!("{value}.1"), namespace, depth + 1)?
        ),
        BridgeType::Triple(first, second, third) => format!(
            "[{} as Any, {} as Any, {} as Any]",
            swift_encode_dev_value(plan, first, &format!("{value}.0"), namespace, depth + 1)?,
            swift_encode_dev_value(plan, second, &format!("{value}.1"), namespace, depth + 1)?,
            swift_encode_dev_value(plan, third, &format!("{value}.2"), namespace, depth + 1)?
        ),
        _ => return None,
    })
}

fn kotlin_encode_dev_value(
    plan: &BridgePlan,
    ty: &BridgeType,
    value: &str,
    namespace: &str,
    depth: usize,
) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::Bytes | BridgeScalar::BufferView) => {
            format!("Base64.encodeToString({value}, Base64.NO_WRAP)")
        }
        BridgeType::Scalar(_) | BridgeType::TypeParameter(_) => value.to_owned(),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("{}({value})", kotlin_enum_encoder(namespace, name)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => {
            let structure = plan
                .types
                .iter()
                .find(|item| item.name == *name && item.kind == BridgeTypeKind::Struct)?;
            let fields = structure
                .fields
                .iter()
                .map(|field| {
                    Some(format!(
                        "\"{}\" to {}",
                        kotlin_escape(&field.name),
                        kotlin_encode_dev_value(
                            plan,
                            &field.ty,
                            &format!("{value}.{}", field.name),
                            namespace,
                            depth + 1,
                        )?
                    ))
                })
                .collect::<Option<Vec<_>>>()?
                .join(", ");
            format!("mapOf<String, Any>({fields})")
        }
        BridgeType::Named {
            kind: BridgeNamedKind::NativeClass,
            ..
        } => value.to_owned(),
        BridgeType::Optional(inner) => {
            let local = format!("nexaEncodeOptional{depth}");
            format!(
                "{value}?.let {{ {local} -> {} }} ?: JSONObject.NULL",
                kotlin_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Array(inner) => {
            let local = format!("nexaEncodeArray{depth}");
            format!(
                "{value}.map {{ {local} -> {} }}",
                kotlin_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Set(inner) => {
            let local = format!("nexaEncodeSet{depth}");
            format!(
                "{value}.map {{ {local} -> {} }}.toSet()",
                kotlin_encode_dev_value(plan, inner, &local, namespace, depth + 1)?
            )
        }
        BridgeType::Map(key_type, inner) => {
            let entry = format!("nexaEncodeMapEntry{depth}");
            let key = kotlin_encode_dev_value(
                plan,
                key_type,
                &format!("{entry}.key"),
                namespace,
                depth + 1,
            )?;
            let encoded = kotlin_encode_dev_value(
                plan,
                inner,
                &format!("{entry}.value"),
                namespace,
                depth + 1,
            )?;
            format!(
                "{value}.entries.associate {{ {entry} -> ({key}).toString() to ({encoded} as Any) }}"
            )
        }
        BridgeType::Pair(first, second) => format!(
            "listOf<Any>({} as Any, {} as Any)",
            kotlin_encode_dev_value(plan, first, &format!("{value}.first"), namespace, depth + 1)?,
            kotlin_encode_dev_value(
                plan,
                second,
                &format!("{value}.second"),
                namespace,
                depth + 1
            )?
        ),
        BridgeType::Triple(first, second, third) => format!(
            "listOf<Any>({} as Any, {} as Any, {} as Any)",
            kotlin_encode_dev_value(plan, first, &format!("{value}.first"), namespace, depth + 1)?,
            kotlin_encode_dev_value(
                plan,
                second,
                &format!("{value}.second"),
                namespace,
                depth + 1
            )?,
            kotlin_encode_dev_value(plan, third, &format!("{value}.third"), namespace, depth + 1)?
        ),
        _ => return None,
    })
}

fn render_swift_enum_codecs(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
) -> Result<(), String> {
    for ty in plan
        .types
        .iter()
        .filter(|ty| ty.kind == BridgeTypeKind::Enum)
    {
        let helper = swift_enum_encoder(namespace, &ty.name);
        out.push_str(&format!(
            "    private static func {helper}(_ value: {}) -> Any {{\n        switch value {{\n",
            ty.name
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                out.push_str(&format!(
                    "        case .{}: return \"{}\"\n",
                    case.name,
                    swift_escape(&case.name)
                ));
            } else {
                let names = (0..case.parameters.len())
                    .map(|index| format!("nexaPayload{index}"))
                    .collect::<Vec<_>>();
                let pattern = case
                    .parameters
                    .iter()
                    .zip(&names)
                    .map(|(parameter, local)| format!("{}: let {local}", parameter.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let values = case
                    .parameters
                    .iter()
                    .zip(&names)
                    .map(|(parameter, local)| {
                        swift_encode_dev_value(plan, &parameter.ty, local, namespace, 0)
                            .map(|value| format!("{value} as Any"))
                            .ok_or_else(|| {
                                format!(
                                    "unsupported Dev enum payload type {:?} for {}.{}",
                                    parameter.ty, ty.name, case.name
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                out.push_str(&format!(
                    "        case .{}({pattern}): return [\"__nexaEnum\": \"{}.{}\", \"case_name\": \"{}\", \"arguments\": [{values}]]\n",
                    case.name,
                    swift_escape(namespace),
                    swift_escape(&ty.name),
                    swift_escape(&case.name),
                ));
            }
        }
        out.push_str("        }\n    }\n");
        let helper = swift_enum_decoder(namespace, &ty.name);
        out.push_str(&format!(
            "    private static func {helper}(_ raw: Any?) -> {}? {{\n        if let name = raw as? String {{\n            switch name {{\n",
            ty.name
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                out.push_str(&format!(
                    "            case \"{}\": return .{}\n",
                    swift_escape(&case.name),
                    case.name
                ));
            }
        }
        out.push_str(&format!(
            "            default: break\n            }}\n        }}\n        guard let nexaValue = raw as? [String: Any], nexaValue[\"__nexaEnum\"] as? String == \"{}.{}\", let nexaCase = nexaValue[\"case_name\"] as? String, let nexaArguments = nexaValue[\"arguments\"] as? [Any] else {{ return nil }}\n        switch nexaCase {{\n",
            swift_escape(namespace),
            swift_escape(&ty.name),
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "        case \"{}\":\n            guard nexaArguments.count == {} else {{ return nil }}\n",
                swift_escape(&case.name),
                case.parameters.len()
            ));
            let mut arguments = Vec::with_capacity(case.parameters.len());
            for (index, parameter) in case.parameters.iter().enumerate() {
                let local = format!("nexaPayload{index}");
                render_swift_value_binding(
                    out,
                    plan,
                    &parameter.ty,
                    &format!("nexaArguments[{index}]"),
                    &local,
                    "return nil",
                    namespace,
                    "            ",
                )?;
                arguments.push(format!("{}: {local}", parameter.name));
            }
            out.push_str(&format!(
                "            return .{}({})\n",
                case.name,
                arguments.join(", ")
            ));
        }
        out.push_str("        default: return nil\n        }\n    }\n\n");
    }
    Ok(())
}

fn render_swift_struct_codecs(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
) -> Result<(), String> {
    for structure in plan
        .types
        .iter()
        .filter(|ty| ty.kind == BridgeTypeKind::Struct)
        .filter(|ty| supported_struct_value(plan, &ty.name, 0))
    {
        let helper = swift_struct_decoder(namespace, &structure.name);
        out.push_str(&format!(
            "    private static func {helper}(_ raw: Any?) -> {}? {{\n        guard let values = raw as? [String: Any] else {{ return nil }}\n",
            structure.name
        ));
        for field in &structure.fields {
            render_swift_value_binding(
                out,
                plan,
                &field.ty,
                &format!("values[\"{}\"]", swift_escape(&field.name)),
                &format!("nexaField_{}", field.name),
                "return nil",
                namespace,
                "        ",
            )?;
        }
        let arguments = structure
            .fields
            .iter()
            .map(|field| format!("{}: nexaField_{}", field.name, field.name))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "        return {}({arguments})\n    }}\n\n",
            structure.name
        ));
    }
    Ok(())
}

fn render_kotlin_enum_codecs(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
) -> Result<(), String> {
    for ty in plan
        .types
        .iter()
        .filter(|ty| ty.kind == BridgeTypeKind::Enum)
    {
        let encoder = kotlin_enum_encoder(namespace, &ty.name);
        out.push_str(&format!(
            "    private fun {encoder}(value: {}): Any = when (value) {{\n",
            ty.name
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                let pattern = format!("{}.{}", ty.name, case.name);
                out.push_str(&format!(
                    "        {pattern} -> \"{}\"\n",
                    kotlin_escape(&case.name)
                ));
            } else {
                let values = case
                    .parameters
                    .iter()
                    .map(|parameter| {
                        kotlin_encode_dev_value(
                            plan,
                            &parameter.ty,
                            &format!("value.{}", parameter.name),
                            namespace,
                            0,
                        )
                        .ok_or_else(|| {
                            format!(
                                "unsupported Dev enum payload type {:?} for {}.{}",
                                parameter.ty, ty.name, case.name
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                out.push_str(&format!(
                    "        is {}.{} -> mapOf<String, Any>(\"__nexaEnum\" to \"{}.{}\", \"case_name\" to \"{}\", \"arguments\" to listOf({values}))\n",
                    ty.name,
                    case.name,
                    kotlin_escape(namespace),
                    kotlin_escape(&ty.name),
                    kotlin_escape(&case.name),
                ));
            }
        }
        out.push_str("    }\n");

        let decoder = kotlin_enum_decoder(namespace, &ty.name);
        out.push_str(&format!(
            "    private fun {decoder}(raw: Any?): {}? {{\n        if (raw is String) return when (raw) {{\n",
            ty.name
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                out.push_str(&format!(
                    "            \"{}\" -> {}.{}\n",
                    kotlin_escape(&case.name),
                    ty.name,
                    case.name
                ));
            }
        }
        out.push_str(&format!(
            "            else -> null\n        }}\n        val nexaValue = raw as? Map<*, *> ?: return null\n        if (nexaValue[\"__nexaEnum\"] != \"{}.{}\") return null\n        val nexaCase = nexaValue[\"case_name\"] as? String ?: return null\n        val nexaArguments = nexaValue[\"arguments\"] as? List<*> ?: return null\n        return when (nexaCase) {{\n",
            kotlin_escape(namespace),
            kotlin_escape(&ty.name),
        ));
        for case in &ty.cases {
            if case.parameters.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "            \"{}\" -> {{\n                if (nexaArguments.size != {}) return null\n",
                kotlin_escape(&case.name),
                case.parameters.len()
            ));
            let mut arguments = Vec::with_capacity(case.parameters.len());
            for (index, parameter) in case.parameters.iter().enumerate() {
                let local = format!("nexaPayload{index}");
                render_kotlin_value_binding(
                    out,
                    plan,
                    namespace,
                    &parameter.ty,
                    &format!("nexaArguments[{index}]"),
                    &local,
                    "return null",
                    "                ",
                )?;
                arguments.push(local);
            }
            out.push_str(&format!(
                "                {}.{}({})\n            }}\n",
                ty.name,
                case.name,
                arguments.join(", ")
            ));
        }
        out.push_str("            else -> null\n        }\n    }\n\n");
    }
    Ok(())
}

fn render_kotlin_struct_codecs(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
) -> Result<(), String> {
    for structure in plan
        .types
        .iter()
        .filter(|ty| ty.kind == BridgeTypeKind::Struct)
        .filter(|ty| supported_struct_value(plan, &ty.name, 0))
    {
        let helper = format!(
            "decode{}FromValue",
            dev_struct_suffix(namespace, &structure.name)
        );
        out.push_str(&format!(
            "    private fun {helper}(raw: Any?): {}? {{\n        val values = raw as? Map<*, *> ?: return null\n",
            structure.name
        ));
        for field in &structure.fields {
            render_kotlin_value_binding(
                out,
                plan,
                namespace,
                &field.ty,
                &format!("values[\"{}\"]", kotlin_escape(&field.name)),
                &format!("nexaField_{}", field.name),
                "return null",
                "        ",
            )?;
        }
        let arguments = structure
            .fields
            .iter()
            .map(|field| format!("nexaField_{}", field.name))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "        return {}({arguments})\n    }}\n\n",
            structure.name
        ));
    }
    Ok(())
}

fn dev_error_helper(namespace: &str, error_type: &str) -> String {
    let suffix = namespace
        .chars()
        .chain(error_type.chars())
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    format!("nexaDevFailure{suffix}")
}

fn render_swift_plugin_errors(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for error in &plan.referenced_errors {
            let helper = dev_error_helper(namespace, &error.name);
            out.push_str(&format!(
                "    private static func {helper}(_ error: {}) -> NexaDevPluginFailure {{\n        switch error {{\n",
                error.name
            ));
            for variant in &error.cases {
                let bindings = variant
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, _)| format!("nexaPayload{index}"))
                    .collect::<Vec<_>>();
                let pattern = if bindings.is_empty() {
                    format!("case .{}:", variant.name)
                } else {
                    let labeled_bindings = variant
                        .parameters
                        .iter()
                        .zip(&bindings)
                        .map(|(parameter, binding)| format!("{}: let {}", parameter.name, binding))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("case .{}({}):", variant.name, labeled_bindings)
                };
                out.push_str(&format!("        {pattern}\n"));
                let fields = variant
                    .parameters
                    .iter()
                    .zip(bindings)
                    .map(|(parameter, binding)| {
                        Ok(format!(
                            "\"{}\": {} as Any",
                            swift_escape(&parameter.name),
                            swift_error_payload_value(
                                &plan,
                                &parameter.ty,
                                &binding,
                                namespace,
                                0,
                            )?
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .join(", ");
                let payload = if fields.is_empty() {
                    "[:]".to_owned()
                } else {
                    format!("[{fields}]")
                };
                out.push_str(&format!(
                    "            return NexaDevPluginFailure(namespace: \"{}\", errorType: \"{}\", variant: \"{}\", payload: {})\n",
                    swift_escape(namespace),
                    swift_escape(&error.name),
                    swift_escape(&variant.name),
                    payload
                ));
            }
            out.push_str("        }\n    }\n\n");
        }
    }
    Ok(())
}

fn render_kotlin_plugin_errors(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for error in &plan.referenced_errors {
            let helper = dev_error_helper(namespace, &error.name);
            out.push_str(&format!(
                "    private fun {helper}(error: {}): NexaDevPluginFailure = when (error) {{\n",
                error.name
            ));
            for variant in &error.cases {
                let fields = variant
                    .parameters
                    .iter()
                    .map(|parameter| {
                        Ok(format!(
                            "\"{}\" to {}",
                            kotlin_escape(&parameter.name),
                            kotlin_error_payload_value(
                                &plan,
                                &parameter.ty,
                                &format!("error.{}", parameter.name),
                                namespace,
                                0,
                            )?
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?
                    .join(", ");
                let pattern = if variant.parameters.is_empty() {
                    format!("{}.{}", error.name, variant.name)
                } else {
                    format!("is {}.{}", error.name, variant.name)
                };
                out.push_str(&format!(
                    "        {pattern} -> NexaDevPluginFailure(\"{}\", \"{}\", \"{}\", mapOf<String, Any>({fields}))\n",
                    kotlin_escape(namespace),
                    kotlin_escape(&error.name),
                    kotlin_escape(&variant.name),
                ));
            }
            out.push_str("    }\n\n");
        }
    }
    Ok(())
}

fn swift_error_payload_value(
    plan: &BridgePlan,
    ty: &BridgeType,
    value: &str,
    namespace: &str,
    depth: usize,
) -> Result<String, String> {
    let nested = |ty: &BridgeType, expression: &str| {
        swift_error_payload_value(plan, ty, expression, namespace, depth + 1)
    };
    Ok(match ty {
        BridgeType::Scalar(_) => swift_encode_value(ty, value, namespace),
        BridgeType::Named {
            kind: BridgeNamedKind::Enum,
            ..
        } => swift_encode_value(ty, value, namespace),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => {
            let structure = plan
                .types
                .iter()
                .find(|candidate| {
                    candidate.name == *name && candidate.kind == BridgeTypeKind::Struct
                })
                .ok_or_else(|| {
                    format!("missing plugin struct `{name}` while encoding Dev errors")
                })?;
            let fields = structure
                .fields
                .iter()
                .map(|field| {
                    Ok(format!(
                        "\"{}\": {} as Any",
                        swift_escape(&field.name),
                        nested(&field.ty, &format!("{value}.{}", field.name))?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?
                .join(", ");
            format!("[{fields}]")
        }
        BridgeType::Named { .. } => value.to_owned(),
        BridgeType::Optional(inner) => {
            let local = format!("nexaErrorOptional{depth}");
            format!(
                "({value}.map {{ {local} in {} as Any }} ?? NSNull())",
                nested(inner, &local)?
            )
        }
        BridgeType::Array(inner) => {
            let local = format!("nexaErrorArray{depth}");
            format!("{value}.map {{ {local} in {} }}", nested(inner, &local)?)
        }
        BridgeType::Set(inner) => {
            let local = format!("nexaErrorSet{depth}");
            format!(
                "{value}.sorted {{ String(describing: $0) < String(describing: $1) }}.map {{ {local} in {} }}",
                nested(inner, &local)?
            )
        }
        BridgeType::Map(_, element) => {
            let entry_local = format!("nexaErrorMapEntry{depth}");
            let value_local = format!("{entry_local}.value");
            format!(
                "Dictionary(uniqueKeysWithValues: {value}.map {{ {entry_local} in (String(describing: {entry_local}.key), {} as Any) }})",
                nested(element, &value_local)?
            )
        }
        BridgeType::Pair(first, second) => format!(
            "[{} as Any, {} as Any]",
            nested(first, &format!("{value}.0"))?,
            nested(second, &format!("{value}.1"))?
        ),
        BridgeType::Triple(first, second, third) => format!(
            "[{} as Any, {} as Any, {} as Any]",
            nested(first, &format!("{value}.0"))?,
            nested(second, &format!("{value}.1"))?,
            nested(third, &format!("{value}.2"))?
        ),
        BridgeType::Result { success, .. } => {
            let local = format!("nexaErrorResult{depth}");
            format!(
                "{{ () -> [String: Any] in switch {value} {{ case .success(let {local}): return [\"Ok\": {} as Any]; case .failure(let nexaErrorFailure{depth}): return [\"Err\": String(describing: nexaErrorFailure{depth})] }} }}()",
                nested(success, &local)?
            )
        }
        BridgeType::TypeParameter(_) => value.to_owned(),
        BridgeType::Signal(_) => {
            return Err("Signal cannot be used as an error payload".to_owned());
        }
    })
}

fn kotlin_error_payload_value(
    plan: &BridgePlan,
    ty: &BridgeType,
    value: &str,
    namespace: &str,
    depth: usize,
) -> Result<String, String> {
    let nested = |ty: &BridgeType, expression: &str| {
        kotlin_error_payload_value(plan, ty, expression, namespace, depth + 1)
    };
    Ok(match ty {
        BridgeType::Scalar(BridgeScalar::Bytes) => {
            format!("Base64.encodeToString({value}, Base64.NO_WRAP)")
        }
        BridgeType::Scalar(BridgeScalar::Void) => "JSONObject.NULL".to_owned(),
        BridgeType::Scalar(_) => value.to_owned(),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Enum,
        } => format!("{}({value})", kotlin_enum_encoder(namespace, name)),
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => {
            let structure = plan
                .types
                .iter()
                .find(|candidate| {
                    candidate.name == *name && candidate.kind == BridgeTypeKind::Struct
                })
                .ok_or_else(|| {
                    format!("missing plugin struct `{name}` while encoding Dev errors")
                })?;
            let fields = structure
                .fields
                .iter()
                .map(|field| {
                    Ok(format!(
                        "\"{}\" to {}",
                        kotlin_escape(&field.name),
                        nested(&field.ty, &format!("{value}.{}", field.name))?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?
                .join(", ");
            format!("mapOf<String, Any>({fields})")
        }
        BridgeType::Named { .. } => value.to_owned(),
        BridgeType::Optional(inner) => {
            let local = format!("nexaErrorOptional{depth}");
            format!(
                "{value}?.let {{ {local} -> {} }} ?: JSONObject.NULL",
                nested(inner, &local)?
            )
        }
        BridgeType::Array(inner) => {
            let local = format!("nexaErrorArray{depth}");
            format!("{value}.map {{ {local} -> {} }}", nested(inner, &local)?)
        }
        BridgeType::Set(inner) => {
            let local = format!("nexaErrorSet{depth}");
            format!(
                "{value}.sortedBy {{ it.toString() }}.map {{ {local} -> {} }}",
                nested(inner, &local)?
            )
        }
        BridgeType::Map(_, element) => {
            let local = format!("nexaErrorEntry{depth}");
            format!(
                "{value}.entries.associate {{ {local} -> {local}.key.toString() to {} }}",
                nested(element, &format!("{local}.value"))?
            )
        }
        BridgeType::Pair(first, second) => format!(
            "listOf<Any>({} as Any, {} as Any)",
            nested(first, &format!("{value}.first"))?,
            nested(second, &format!("{value}.second"))?
        ),
        BridgeType::Triple(first, second, third) => format!(
            "listOf<Any>({} as Any, {} as Any, {} as Any)",
            nested(first, &format!("{value}.first"))?,
            nested(second, &format!("{value}.second"))?,
            nested(third, &format!("{value}.third"))?
        ),
        BridgeType::Result { success, .. } => {
            let local = format!("nexaErrorResult{depth}");
            format!(
                "{value}.fold(onSuccess = {{ {local} -> mapOf<String, Any>(\"Ok\" to ({} as Any)) }}, onFailure = {{ failure -> mapOf<String, Any>(\"Err\" to failure.toString()) }})",
                nested(success, &local)?
            )
        }
        BridgeType::TypeParameter(_) => value.to_owned(),
        BridgeType::Signal(_) => {
            return Err("Signal cannot be used as an error payload".to_owned());
        }
    })
}

fn swift_enum_base(namespace: &str, name: &str) -> String {
    let prefix = namespace
        .chars()
        .chain(name.chars())
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    format!("nexaDevEnum{prefix}")
}

fn swift_enum_encoder(namespace: &str, name: &str) -> String {
    format!("{}ToString", swift_enum_base(namespace, name))
}
fn swift_enum_decoder(namespace: &str, name: &str) -> String {
    format!("{}FromString", swift_enum_base(namespace, name))
}

fn kotlin_enum_base(namespace: &str, name: &str) -> String {
    format!("nexaDevEnum{}", dev_struct_suffix(namespace, name))
}

fn kotlin_enum_encoder(namespace: &str, name: &str) -> String {
    format!("{}ToValue", kotlin_enum_base(namespace, name))
}

fn kotlin_enum_decoder(namespace: &str, name: &str) -> String {
    format!("{}FromValue", kotlin_enum_base(namespace, name))
}

fn supported_parameters(plan: &BridgePlan, parameters: &[BridgeParameter]) -> bool {
    parameters
        .iter()
        .all(|parameter| supported_value_type(plan, &parameter.ty))
}

fn supported_return(plan: &BridgePlan, ty: &BridgeType) -> bool {
    ty.is_void() || supported_value_type(plan, ty)
}

fn supported_value_type(plan: &BridgePlan, ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Optional(inner) => supported_runtime_value_type(plan, inner),
        BridgeType::Array(inner) => supported_array_element(plan, inner, 0),
        BridgeType::Set(inner) => supported_hashable_collection_element(inner),
        BridgeType::Map(key, value) => {
            supported_map_key(key) && supported_static_collection_element(plan, value, 0)
        }
        BridgeType::Pair(first, second) => {
            supported_static_collection_element(plan, first, 1)
                && supported_static_collection_element(plan, second, 1)
        }
        BridgeType::Triple(first, second, third) => {
            supported_static_collection_element(plan, first, 1)
                && supported_static_collection_element(plan, second, 1)
                && supported_static_collection_element(plan, third, 1)
        }
        BridgeType::Named {
            kind: BridgeNamedKind::Enum | BridgeNamedKind::NativeClass,
            ..
        } => true,
        BridgeType::Named {
            name,
            kind: BridgeNamedKind::Struct,
        } => supported_struct_value(plan, name, 0),
        _ => false,
    }
}

fn supported_scalar(scalar: BridgeScalar) -> bool {
    matches!(
        scalar,
        BridgeScalar::String
            | BridgeScalar::Bool
            | BridgeScalar::Int8
            | BridgeScalar::Int16
            | BridgeScalar::Int32
            | BridgeScalar::Int64
            | BridgeScalar::UInt8
            | BridgeScalar::UInt16
            | BridgeScalar::UInt32
            | BridgeScalar::UInt64
            | BridgeScalar::Float32
            | BridgeScalar::Float64
            | BridgeScalar::Bytes
            | BridgeScalar::BufferView
    )
}

fn render_swift_constructor(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    constructor: &BridgeConstructor,
) {
    out.push_str(&format!(
        "        case (\"{}\", \"{}\"):\n            guard arguments.count == {} else {{ return (true, NSNull()) }}\n",
        swift_escape(namespace),
        swift_escape(&interface.name),
        constructor.parameters.len()
    ));
    for (index, parameter) in constructor.parameters.iter().enumerate() {
        render_swift_decode_argument(
            out,
            plan,
            namespace,
            index,
            &parameter.ty,
            &format!("arguments[{index}]"),
            "            ",
        );
    }
    let arguments = (0..constructor.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "            return (true, {}({arguments}))\n",
        interface.name
    ));
}

fn render_kotlin_constructor(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    constructor: &BridgeConstructor,
) {
    out.push_str(&format!(
        "            \"{}.{}\" -> {{\n                if (arguments.size != {}) return true to JSONObject.NULL\n",
        kotlin_escape(namespace),
        kotlin_escape(&interface.name),
        constructor.parameters.len()
    ));
    for (index, parameter) in constructor.parameters.iter().enumerate() {
        render_kotlin_decode_argument(
            out,
            plan,
            namespace,
            index,
            &parameter.ty,
            &format!("arguments[{index}]"),
            &format!(
                "Invalid constructor argument `{}` for {}.{}",
                parameter.name, namespace, interface.name
            ),
            "                ",
        );
    }
    let arguments = (0..constructor.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "                true to {}({arguments})\n            }}\n",
        interface.name
    ));
}

fn render_swift_instance_methods(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
    asynchronous: bool,
) -> Result<(), String> {
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            for method in &interface.methods {
                if !supported(method, &plan) || (!asynchronous && method.is_async) {
                    continue;
                }
                out.push_str(&format!(
                    "        if namespace == \"{}\", name == \"{}\", let nexaReceiver = receiver as? any {}Spec {{\n",
                    swift_escape(namespace),
                    swift_escape(&method.name),
                    interface.name
                ));
                let generic = !method.type_parameters.is_empty();
                let codec_shapes = method_codec_shapes(method);
                if generic {
                    out.push_str(&format!(
                        "            guard codecs.count == {} else {{ return (false, NSNull()) }}\n",
                        method_codec_count(method)
                    ));
                    for index in 0..method_codec_count(method) {
                        out.push_str(&format!(
                            "            let nexaCodec{index} = codecs[{index}]\n"
                        ));
                    }
                }
                for (index, parameter) in method.parameters.iter().enumerate() {
                    let raw = format!("options[\"{}\"]", swift_escape(&parameter.name));
                    if generic && super::bridge_plan::contains_type_parameter(&parameter.ty) {
                        render_swift_dynamic_argument(
                            out,
                            index,
                            &parameter.ty,
                            &raw,
                            namespace,
                            "            ",
                        )?;
                    } else {
                        render_swift_decode_argument(
                            out,
                            &plan,
                            namespace,
                            index,
                            &parameter.ty,
                            &raw,
                            "            ",
                        );
                    }
                }
                let mut call_arguments = (0..method.parameters.len())
                    .map(|index| format!("nexaArg{index}"))
                    .collect::<Vec<_>>();
                if generic {
                    for (index, (codec_ty, decodes)) in codec_shapes.iter().enumerate() {
                        if *decodes {
                            let raw = format!(
                                "NexaDevValueCodec.read(type: nexaCodec{index}, from: nexaReader{index}, enumCases: enumCases)"
                            );
                            let decoded = swift_dynamic_decode_value(codec_ty, &raw, namespace, 0)
                                .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?;
                            call_arguments.push(format!("{{ nexaReader{index} in {decoded} }}"));
                        } else {
                            call_arguments.push(format!(
                                "{{ nexaItem{index}, nexaWriter{index} in _ = NexaDevValueCodec.write(nexaItem{index}, type: nexaCodec{index}, into: nexaWriter{index}, enumCases: enumCases) }}"
                            ));
                        }
                    }
                    if !method.row_type_parameters.is_empty() {
                        call_arguments.push(swift_dev_row_mapper(
                            codec_shapes.len(),
                            method,
                            &plan,
                        )?);
                    }
                }
                let arguments = call_arguments.join(", ");
                let prefix = match (method.is_async, method.error_type().is_some()) {
                    (true, true) => "try await ",
                    (true, false) => "await ",
                    (false, true) => "try ",
                    (false, false) => "",
                };
                let error_type = method.error_type();
                if error_type.is_some() {
                    out.push_str("            do {\n");
                }
                let indent = if error_type.is_some() {
                    "                "
                } else {
                    "            "
                };
                if method.success_type().is_void() {
                    out.push_str(&format!(
                        "{indent}{prefix}nexaReceiver.{}({arguments})\n{indent}return (true, NSNull())\n",
                        method.name,
                    ));
                } else {
                    out.push_str(&format!(
                        "{indent}let result = {prefix}nexaReceiver.{}({arguments})\n",
                        method.name,
                    ));
                    render_swift_result(out, &plan, namespace, method.success_type(), indent)?;
                }
                if let Some(error_type) = error_type {
                    let helper = dev_error_helper(namespace, error_type);
                    out.push_str(&format!(
                        "            }} catch let error as {error_type} {{\n"
                    ));
                    if asynchronous {
                        out.push_str(&format!("                throw Self.{helper}(error)\n",));
                    } else {
                        out.push_str(&format!(
                            "                return (true, Self.{helper}(error))\n",
                        ));
                    }
                    out.push_str("            }\n");
                }
                out.push_str("        }\n");
            }
        }
    }
    Ok(())
}

fn render_kotlin_instance_methods(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
    asynchronous: bool,
) -> Result<(), String> {
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            for method in &interface.methods {
                if !supported(method, &plan) || (!asynchronous && method.is_async) {
                    continue;
                }
                out.push_str(&format!(
                    "        if (namespace == \"{}\" && name == \"{}\" && receiver is {}) {{\n            val nexaReceiver = receiver\n",
                    kotlin_escape(namespace),
                    kotlin_escape(&method.name),
                    interface.name
                ));
                let generic = !method.type_parameters.is_empty();
                let codec_shapes = method_codec_shapes(method);
                if generic {
                    out.push_str(&format!(
                        "            if (codecs.size != {}) return false to JSONObject.NULL\n",
                        method_codec_count(method)
                    ));
                    for index in 0..method_codec_count(method) {
                        out.push_str(&format!(
                            "            val nexaCodec{index} = codecs[{index}]\n"
                        ));
                    }
                }
                for (index, parameter) in method.parameters.iter().enumerate() {
                    let raw = format!("options[\"{}\"]", kotlin_escape(&parameter.name));
                    let error = format!(
                        "Invalid argument `{}` for {}.{}",
                        parameter.name, namespace, method.name
                    );
                    if generic && super::bridge_plan::contains_type_parameter(&parameter.ty) {
                        render_kotlin_dynamic_argument(
                            out,
                            index,
                            &parameter.ty,
                            &raw,
                            namespace,
                            &error,
                            "            ",
                        )?;
                    } else {
                        render_kotlin_decode_argument(
                            out,
                            &plan,
                            namespace,
                            index,
                            &parameter.ty,
                            &raw,
                            &error,
                            "            ",
                        );
                    }
                }
                let mut call_arguments = (0..method.parameters.len())
                    .map(|index| format!("nexaArg{index}"))
                    .collect::<Vec<_>>();
                if generic {
                    for (index, (codec_ty, decodes)) in codec_shapes.iter().enumerate() {
                        if *decodes {
                            let raw = format!(
                                "NexaDevValueCodec.read(nexaCodec{index}, nexaReader{index}, enumCases)"
                            );
                            let decoded = kotlin_dynamic_decode_value(codec_ty, &raw, namespace, 0)
                                .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?;
                            call_arguments.push(format!(
                                "{{ nexaReader{index} -> NexaDevValueCodec.readResult<{shape}>({decoded}) }}",
                                shape = kotlin_dynamic_type(codec_ty)
                                    .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?
                            ));
                        } else {
                            call_arguments.push(format!(
                                "{{ nexaItem{index}, nexaWriter{index} -> check(NexaDevValueCodec.write(nexaItem{index}, nexaCodec{index}, nexaWriter{index}, enumCases)) }}"
                            ));
                        }
                    }
                    if !method.row_type_parameters.is_empty() {
                        call_arguments.push(kotlin_dev_row_mapper(
                            codec_shapes.len(),
                            method,
                            &plan,
                        )?);
                    }
                }
                let arguments = call_arguments.join(", ");
                if method.error_type().is_some() {
                    out.push_str("            try {\n");
                }
                let indent = if method.error_type().is_some() {
                    "                "
                } else {
                    "            "
                };
                if method.success_type().is_void() {
                    out.push_str(&format!(
                        "{indent}nexaReceiver.{}({arguments})\n{indent}return true to JSONObject.NULL\n",
                        method.name
                    ));
                } else {
                    out.push_str(&format!(
                        "{indent}val result = nexaReceiver.{}({arguments})\n",
                        method.name
                    ));
                    render_kotlin_result(out, &plan, namespace, method.success_type(), indent)?;
                }
                if let Some(error_type) = method.error_type() {
                    let helper = dev_error_helper(namespace, error_type);
                    out.push_str(&format!(
                        "            }} catch (error: {error_type}) {{\n                throw {helper}(error)\n            }}\n",
                    ));
                }
                out.push_str("        }\n");
            }
        }
    }
    Ok(())
}

fn render_swift_decode_argument(
    out: &mut String,
    _plan: &BridgePlan,
    namespace: &str,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    indent: &str,
) {
    let decode_type = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        other => other,
    };
    let Some(decoder) = swift_decode_value(decode_type, raw, namespace, 0) else {
        return;
    };
    if matches!(ty, BridgeType::Optional(_)) {
        out.push_str(&format!(
            "{indent}let nexaArg{index} = {raw} is NSNull ? nil : {decoder}\n"
        ));
    } else {
        out.push_str(&format!(
            "{indent}guard let nexaArg{index} = {decoder} else {{ return (true, NSNull()) }}\n"
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn render_kotlin_decode_argument(
    out: &mut String,
    _plan: &BridgePlan,
    namespace: &str,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    error: &str,
    indent: &str,
) {
    let decode_type = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        other => other,
    };
    let Some(decoder) = kotlin_decode_value(decode_type, raw, namespace, 0) else {
        return;
    };
    if matches!(ty, BridgeType::Optional(_)) {
        out.push_str(&format!(
            "{indent}val nexaArg{index} = if ({raw} == null || {raw} == JSONObject.NULL) null else {decoder}\n"
        ));
    } else {
        out.push_str(&format!(
            "{indent}val nexaArg{index} = {decoder} ?: error(\"{}\")\n",
            kotlin_escape(error)
        ));
    }
}

fn render_swift_result(
    out: &mut String,
    plan: &BridgePlan,
    namespace: &str,
    ty: &BridgeType,
    indent: &str,
) -> Result<(), String> {
    let encoded = swift_encode_dev_value(plan, ty, "result", namespace, 0)
        .ok_or_else(|| format!("unsupported Dev result type {ty:?}"))?;
    out.push_str(&format!("{indent}return (true, {encoded})\n"));
    Ok(())
}

fn render_kotlin_result(
    out: &mut String,
    plan: &BridgePlan,
    namespace: &str,
    ty: &BridgeType,
    indent: &str,
) -> Result<(), String> {
    let encoded = kotlin_encode_dev_value(plan, ty, "result", namespace, 0)
        .ok_or_else(|| format!("unsupported Dev result type {ty:?}"))?;
    out.push_str(&format!("{indent}return true to ({encoded} as Any)\n"));
    Ok(())
}

fn kotlin_number_conversion(scalar: BridgeScalar) -> Option<&'static str> {
    Some(match scalar {
        BridgeScalar::Int8 => "toByte",
        BridgeScalar::Int16 => "toShort",
        BridgeScalar::Int32 => "toInt",
        BridgeScalar::Int64 => "toLong",
        BridgeScalar::UInt8 => "toByte()?.toUByte",
        BridgeScalar::UInt16 => "toShort()?.toUShort",
        BridgeScalar::UInt32 => "toInt()?.toUInt",
        BridgeScalar::UInt64 => "toLong()?.toULong",
        BridgeScalar::Float32 => "toFloat",
        BridgeScalar::Float64 => "toDouble",
        _ => return None,
    })
}

fn render_swift_dynamic_argument(
    out: &mut String,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    namespace: &str,
    indent: &str,
) -> Result<(), String> {
    out.push_str(&format!(
        "{indent}guard let nexaRaw{index} = {raw} else {{ return (false, NSNull()) }}\n"
    ));
    let decoder = swift_dynamic_decode_value(ty, &format!("nexaRaw{index}"), namespace, 0)
        .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
    out.push_str(&format!(
        "{indent}guard let nexaArg{index} = {decoder} else {{ return (false, NSNull()) }}\n"
    ));
    Ok(())
}

fn render_kotlin_dynamic_argument(
    out: &mut String,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    namespace: &str,
    error: &str,
    indent: &str,
) -> Result<(), String> {
    let shape = kotlin_dynamic_type(ty)
        .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
    if let BridgeType::Optional(inner) = ty {
        let inner_shape = kotlin_dynamic_type(inner)
            .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
        let decoded = kotlin_dynamic_decode_value(inner, &format!("nexaRaw{index}"), namespace, 0)
            .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
        out.push_str(&format!(
            "{indent}val nexaRaw{index} = {raw} ?: JSONObject.NULL\n{indent}val nexaArg{index}: {shape} = if (nexaRaw{index} == JSONObject.NULL) null else ({decoded}) as? {inner_shape} ?: error(\"{}\")\n",
            kotlin_escape(error),
        ));
        return Ok(());
    }
    out.push_str(&format!(
        "{indent}val nexaRaw{index} = {raw} ?: error(\"{}\")\n",
        kotlin_escape(error)
    ));
    match ty {
        BridgeType::TypeParameter(_) => {
            out.push_str(&format!(
                "{indent}val nexaArg{index}: {shape} = nexaRaw{index}\n"
            ));
        }
        BridgeType::Pair(..) | BridgeType::Triple(..) => {
            let decoded = kotlin_dynamic_decode_value(ty, &format!("nexaRaw{index}"), namespace, 0)
                .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
            out.push_str(&format!(
                "{indent}val nexaArg{index}: {shape} = {decoded} ?: error(\"{}\")\n",
                kotlin_escape(error)
            ));
        }
        _ => {
            let decoded = kotlin_dynamic_decode_value(ty, &format!("nexaRaw{index}"), namespace, 0)
                .ok_or_else(|| format!("unsupported Dev generic argument type {ty:?}"))?;
            out.push_str(&format!(
                "{indent}val nexaArg{index}: {shape} = ({decoded}) as? {shape} ?: error(\"{}\")\n",
                kotlin_escape(error)
            ));
        }
    }
    Ok(())
}

fn render_swift_method(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
    method: &BridgeMethod,
    asynchronous: bool,
) -> Result<(), String> {
    if !asynchronous && method.is_async {
        return Ok(());
    }
    out.push_str(&format!(
        "        case (\"{}\", \"{}\"):\n",
        swift_escape(namespace),
        swift_escape(&method.name)
    ));
    let generic = !method.type_parameters.is_empty();
    let codec_shapes = method_codec_shapes(method);
    if generic {
        out.push_str(&format!(
            "            guard codecs.count == {} else {{ return (false, NSNull()) }}\n",
            method_codec_count(method)
        ));
        for index in 0..method_codec_count(method) {
            out.push_str(&format!(
                "            let nexaCodec{index} = codecs[{index}]\n"
            ));
        }
    }
    for (index, parameter) in method.parameters.iter().enumerate() {
        let raw = format!("options[\"{}\"]", swift_escape(&parameter.name));
        if generic && super::bridge_plan::contains_type_parameter(&parameter.ty) {
            render_swift_dynamic_argument(
                out,
                index,
                &parameter.ty,
                &raw,
                namespace,
                "            ",
            )?;
        } else {
            render_swift_decode_argument(
                out,
                plan,
                namespace,
                index,
                &parameter.ty,
                &raw,
                "            ",
            );
        }
    }
    let mut call_arguments = (0..method.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>();
    if generic {
        for (index, (codec_ty, decodes)) in codec_shapes.iter().enumerate() {
            if *decodes {
                let raw = format!(
                    "NexaDevValueCodec.read(type: nexaCodec{index}, from: nexaReader{index}, enumCases: enumCases)"
                );
                let decoded = swift_dynamic_decode_value(codec_ty, &raw, namespace, 0)
                    .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?;
                call_arguments.push(format!("{{ nexaReader{index} in {decoded} }}"));
            } else {
                call_arguments.push(format!(
                    "{{ nexaItem{index}, nexaWriter{index} in _ = NexaDevValueCodec.write(nexaItem{index}, type: nexaCodec{index}, into: nexaWriter{index}, enumCases: enumCases) }}"
                ));
            }
        }
        if !method.row_type_parameters.is_empty() {
            call_arguments.push(swift_dev_row_mapper(codec_shapes.len(), method, plan)?);
        }
    }
    let args = call_arguments.join(", ");
    let invoke = format!("{}Plugin.shared.{}({args})", namespace, method.name);
    let prefix = match (method.is_async, method.error_type().is_some()) {
        (true, true) => "try await ",
        (true, false) => "await ",
        (false, true) => "try ",
        (false, false) => "",
    };
    let error_type = method.error_type();
    if error_type.is_some() {
        out.push_str("            do {\n");
    }
    let indent = if error_type.is_some() {
        "                "
    } else {
        "            "
    };
    if method.success_type().is_void() {
        out.push_str(&format!(
            "{indent}{prefix}{invoke}\n{indent}return (true, NSNull())\n"
        ));
    } else {
        out.push_str(&format!("{indent}let result = {prefix}{invoke}\n"));
        render_swift_result(out, plan, namespace, method.success_type(), indent)?;
    }
    if let Some(error_type) = error_type {
        let helper = dev_error_helper(namespace, error_type);
        out.push_str(&format!(
            "            }} catch let error as {error_type} {{\n"
        ));
        if asynchronous {
            out.push_str(&format!("                throw Self.{helper}(error)\n",));
        } else {
            out.push_str(&format!(
                "                return (true, Self.{helper}(error))\n",
            ));
        }
        out.push_str("            }\n");
    }
    Ok(())
}

fn render_kotlin_method(
    out: &mut String,
    namespace: &str,
    plan: &BridgePlan,
    method: &BridgeMethod,
    asynchronous: bool,
) -> Result<(), String> {
    if !asynchronous && method.is_async {
        return Ok(());
    }
    out.push_str(&format!(
        "            \"{}.{}\" -> {{\n",
        kotlin_escape(namespace),
        kotlin_escape(&method.name)
    ));
    let generic = !method.type_parameters.is_empty();
    let codec_shapes = method_codec_shapes(method);
    if generic {
        out.push_str(&format!(
            "                if (codecs.size != {}) return false to JSONObject.NULL\n",
            method_codec_count(method)
        ));
        for index in 0..method_codec_count(method) {
            out.push_str(&format!(
                "                val nexaCodec{index} = codecs[{index}]\n"
            ));
        }
    }
    for (index, parameter) in method.parameters.iter().enumerate() {
        let raw = format!("options[\"{}\"]", kotlin_escape(&parameter.name));
        let error = format!(
            "Invalid argument `{}` for {namespace}.{}",
            parameter.name, method.name
        );
        if generic && super::bridge_plan::contains_type_parameter(&parameter.ty) {
            render_kotlin_dynamic_argument(
                out,
                index,
                &parameter.ty,
                &raw,
                namespace,
                &error,
                "                ",
            )?;
        } else {
            render_kotlin_decode_argument(
                out,
                plan,
                namespace,
                index,
                &parameter.ty,
                &raw,
                &error,
                "                ",
            );
        }
    }
    let mut call_arguments = (0..method.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>();
    if generic {
        for (index, (codec_ty, decodes)) in codec_shapes.iter().enumerate() {
            if *decodes {
                let raw = format!(
                    "NexaDevValueCodec.read(nexaCodec{index}, nexaReader{index}, enumCases)"
                );
                let decoded = kotlin_dynamic_decode_value(codec_ty, &raw, namespace, 0)
                    .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?;
                call_arguments.push(format!(
                    "{{ nexaReader{index} -> NexaDevValueCodec.readResult<{shape}>({decoded}) }}",
                    shape = kotlin_dynamic_type(codec_ty)
                        .ok_or_else(|| "unsupported Dev generic return codec".to_owned())?
                ));
            } else {
                call_arguments.push(format!(
                    "{{ nexaItem{index}, nexaWriter{index} -> check(NexaDevValueCodec.write(nexaItem{index}, nexaCodec{index}, nexaWriter{index}, enumCases)) }}"
                ));
            }
        }
        if !method.row_type_parameters.is_empty() {
            call_arguments.push(kotlin_dev_row_mapper(codec_shapes.len(), method, plan)?);
        }
    }
    let args = call_arguments.join(", ");
    let call = format!("{}Plugin.instance.{}({args})", namespace, method.name);
    if method.error_type().is_some() {
        out.push_str("                try {\n");
    }
    let indent = if method.error_type().is_some() {
        "                    "
    } else {
        "                "
    };
    if method.success_type().is_void() {
        out.push_str(&format!(
            "{indent}{call}\n{indent}true to JSONObject.NULL\n"
        ));
    } else {
        out.push_str(&format!("{indent}val result = {call}\n"));
        render_kotlin_result(out, plan, namespace, method.success_type(), indent)?;
    }
    if let Some(error_type) = method.error_type() {
        let helper = dev_error_helper(namespace, error_type);
        out.push_str(&format!(
            "                }} catch (error: {error_type}) {{\n                    throw {helper}(error)\n                }}\n",
        ));
    }
    out.push_str("            }\n");
    Ok(())
}

fn swift_decoder(ty: &BridgeType) -> &'static str {
    match ty {
        BridgeType::Scalar(BridgeScalar::String) => "decodeString",
        BridgeType::Scalar(BridgeScalar::Bool) => "decodeBool",
        BridgeType::Scalar(BridgeScalar::Int8) => "decodeInt8",
        BridgeType::Scalar(BridgeScalar::Int16) => "decodeInt16",
        BridgeType::Scalar(BridgeScalar::Int32) => "decodeInt32",
        BridgeType::Scalar(BridgeScalar::Int64) => "decodeInt64",
        BridgeType::Scalar(BridgeScalar::UInt8) => "decodeUInt8",
        BridgeType::Scalar(BridgeScalar::UInt16) => "decodeUInt16",
        BridgeType::Scalar(BridgeScalar::UInt32) => "decodeUInt32",
        BridgeType::Scalar(BridgeScalar::UInt64) => "decodeUInt64",
        BridgeType::Scalar(BridgeScalar::Float32) => "decodeFloat32",
        BridgeType::Scalar(BridgeScalar::Float64) => "decodeFloat64",
        BridgeType::Scalar(BridgeScalar::Bytes | BridgeScalar::BufferView) => "decodeBytes",
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Scalar(BridgeScalar::String) => "decodeOptionalString",
            BridgeType::Scalar(BridgeScalar::Bool) => "decodeOptionalBool",
            BridgeType::Scalar(BridgeScalar::Int8) => "decodeOptionalInt8",
            BridgeType::Scalar(BridgeScalar::Int16) => "decodeOptionalInt16",
            BridgeType::Scalar(BridgeScalar::Int32) => "decodeOptionalInt32",
            BridgeType::Scalar(BridgeScalar::Int64) => "decodeOptionalInt64",
            BridgeType::Scalar(BridgeScalar::UInt8) => "decodeOptionalUInt8",
            BridgeType::Scalar(BridgeScalar::UInt16) => "decodeOptionalUInt16",
            BridgeType::Scalar(BridgeScalar::UInt32) => "decodeOptionalUInt32",
            BridgeType::Scalar(BridgeScalar::UInt64) => "decodeOptionalUInt64",
            BridgeType::Scalar(BridgeScalar::Float32) => "decodeOptionalFloat32",
            BridgeType::Scalar(BridgeScalar::Float64) => "decodeOptionalFloat64",
            BridgeType::Scalar(BridgeScalar::Bytes) => "decodeOptionalBytes",
            _ => unreachable!("unsupported optional plugin parameter passed generation"),
        },
        _ => unreachable!("unsupported service method type passed generation"),
    }
}

fn swift_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn kotlin_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{kotlin, swift};
    use nexa_plugin_idl::PluginIdl;

    fn fast_math() -> Vec<(String, PluginIdl)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/plugins/fast-math/native.nxid");
        vec![(
            "FastMath".to_owned(),
            nexa_plugin_idl::parse_file(&path).expect("parse FastMath contract"),
        )]
    }

    fn scalar_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                native class DevStore {
                    init(instanceID: String, cryptKey: String?)
                    fn setString(key: String, value: String) -> Bool
                    fn getString(key: String) -> String?
                    fn setUInt32(value: UInt32) -> Bool
                    fn getUInt32() -> UInt32?
                }
                "#,
            )
            .expect("parse scalar native-class contract"),
        )]
    }

    fn generic_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                native class DevStore {
                    init()
                    fn setObject<T>(key: String, value: T) -> Bool
                    fn getObject<T>(key: String) -> T?
                    fn setList<T>(key: String, values: Array<T>) -> Bool
                    fn setSet<T>(key: String, values: Set<T>) -> Bool
                    fn getSet<T>(key: String) -> Set<T>?
                    fn setMap<K, V>(key: String, values: Map<K, V>) -> Bool
                    fn getMap<K, V>(key: String) -> Map<K, V>?
                    fn setPair<K, V>(value: Pair<K, V>) -> Bool
                    fn getPair<K, V>() -> Pair<K, V>?
                    fn setTriple<A, B, C>(value: Triple<A, B, C>) -> Bool
                }
                "#,
            )
            .expect("parse generic native-class contract"),
        )]
    }

    fn optional_array_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                struct OptionalLabels {
                    values: Array<String?>
                }
                native class OptionalStore {
                    init()
                    property values: Array<String?>
                    fn replace(values: Array<String?>) -> Array<String?>
                    event changed(values: Array<String?>)
                }
                native component OptionalBadge {
                    prop labels: OptionalLabels
                    event selected(values: Array<String?>)
                }
                "#,
            )
            .expect("parse array of optional plugin values"),
        )]
    }

    fn optional_compound_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                struct OptionalRecord {
                    values: Map<String, Int32?>
                    pair: Pair<String?, Int32>
                    tags: Set<String?>
                }
                native class OptionalCompoundStore {
                    init()
                    property values: Map<String, Int32?>
                    property pair: Pair<String?, Int32>
                    property nullableTags: Set<String?>
                    event changed(values: Map<String, Int32?>, pair: Pair<String?, Int32>, triple: Triple<String, Int32?, Bool>, tags: Set<String?>)
                    fn update(values: Map<String, Int32?>, pair: Pair<String?, Int32>, triple: Triple<String, Int32?, Bool>, tags: Set<String?>) -> Triple<String, Int32?, Bool>
                    fn genericPair<A, B>(value: Pair<A?, B>) -> Pair<A?, B>?
                    fn genericMap<K, V>(value: Map<K, V?>) -> Map<K, V?>?
                    fn genericSet<T>(value: Set<T?>) -> Set<T?>?
                    fn genericOptionalValue<T>(value: T?) -> Bool
                    fn genericOptionalPair<T, U>(value: Pair<T?, U>?) -> Bool
                }
                native component OptionalCompoundBadge {
                    prop record: OptionalRecord
                    event selected(pair: Pair<String?, Int32>, tags: Set<String?>)
                }
                "#,
            )
            .expect("parse nullable compound plugin values"),
        )]
    }

    fn compound_errors() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                struct StoreSnapshot {
                    values: Array<String>
                }
                error StoreError {
                    invalid(snapshot: StoreSnapshot, ids: Array<Int32>, counts: Map<String, UInt32>, note: String?)
                }
                service Cache {
                    async fn open() throws StoreError
                }
                native class Session {
                    init()
                    async fn reload() throws StoreError
                    async fn refresh() throws StoreError
                }
                "#,
            )
            .expect("parse compound Dev error contract"),
        )]
    }

    fn compound_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                struct StoreStats {
                    count: Int64
                    totalSize: Int64
                }
                native class DevStore {
                    init(instanceID: String)
                    readonly property stats: StoreStats
                    fn getAllKeys() -> Array<String>
                    fn removeMany(keys: Array<String>) -> Int32
                    fn statsSnapshot() -> StoreStats
                    fn update(stats: StoreStats) -> Bool
                    fn merge(stats: Array<StoreStats>) -> Bool
                    fn statsHistory(limit: Int32) -> Array<StoreStats>
                    fn tupleSnapshot() -> Pair<String, Int32>
                    fn tripleSnapshot() -> Triple<String, Int32, Bool>
                    event updated(stats: StoreStats, keys: Array<String>)
                }
                native component StatsBadge {
                    content
                    prop stats: StoreStats
                    event selected(keys: Array<String>)
                }
                "#,
            )
            .expect("parse compound native-class contract"),
        )]
    }

    fn collection_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                enum CacheBucket {
                    Images
                    Avatars
                }
                service StoreApi {
                    fn normalize(index: Map<Int32, Set<Int32>>) -> Map<Int32, Set<Int32>>
                }
                native class DevStore {
                    init(index: Map<Int32, Set<Int32>>)
                    property index: Map<Int32, Set<Int32>>
                    property buckets: Set<CacheBucket>
                    property indexByBucket: Map<CacheBucket, Set<Int32>>
                    fn replace(index: Map<Int32, Set<Int32>>) -> Map<Int32, Set<Int32>>
                    fn replaceIndex(index: Map<CacheBucket, Set<Int32>>) -> Map<CacheBucket, Set<Int32>>
                    event changed(index: Map<Int32, Set<Int32>>)
                    event bucketsChanged(buckets: Set<CacheBucket>, index: Map<CacheBucket, Set<Int32>>)
                }
                native component IndexBadge {
                    prop index: Map<Int32, Set<Int32>>
                    prop buckets: Set<CacheBucket>
                    prop indexByBucket: Map<CacheBucket, Set<Int32>>
                    event selected(buckets: Set<CacheBucket>, index: Map<CacheBucket, Set<Int32>>)
                }
                "#,
            )
            .expect("parse native collection contract"),
        )]
    }

    fn byte_set_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Storage".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                native class ByteSetStore {
                    init()
                    property values: Set<Bytes>
                    fn replace(values: Set<Bytes>) -> Bool
                    event changed(values: Set<Bytes>)
                }
                native component ByteSetBadge {
                    prop values: Set<Bytes>
                    event selected(values: Set<Bytes>)
                }
                "#,
            )
            .expect("parse plugin byte-set boundary contract"),
        )]
    }

    fn enum_payload_store() -> Vec<(String, PluginIdl)> {
        vec![(
            "Payloads".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                enum StoredValue {
                    empty
                    binary(data: Bytes)
                    optionalText(text: String?)
                }
                service StoredValueApi {
                    fn roundTrip(value: StoredValue) -> StoredValue
                }
                "#,
            )
            .expect("parse enum payload contract"),
        )]
    }

    fn sqlite() -> Vec<(String, PluginIdl)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/sqlite/native.nxid");
        vec![(
            "SQLite".to_owned(),
            nexa_plugin_idl::parse_file(&path).expect("parse SQLite contract"),
        )]
    }

    fn row_reader_plugin() -> Vec<(String, PluginIdl)> {
        vec![(
            "Records".to_owned(),
            nexa_plugin_idl::parse(
                r#"
                enum Cell {
                    absent
                    truth(value: Bool)
                    amount(value: Int64)
                    text(value: String)
                    bytes(value: Bytes)
                }
                error ReadFailure {
                    malformedRow(details: String)
                }
                service RecordReader {
                    async fn read<T: Row>(cells: Array<Cell>) -> Array<T> throws ReadFailure rowFailure malformedRow
                }
                "#,
            )
            .expect("parse generic row reader plugin contract"),
        )]
    }

    fn video_player() -> Vec<(String, PluginIdl)> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/video-player/native.nxid");
        vec![(
            "VideoPlayer".to_owned(),
            nexa_plugin_idl::parse_file(&path).expect("parse VideoPlayer contract"),
        )]
    }

    #[test]
    fn swift_bridge_emits_direct_typed_calls_for_service_scalars_and_bytes() {
        let generated = swift(&fast_math()).expect("render Swift dev bridge");
        assert!(generated.contains("FastMathPlugin.shared.add(nexaArg0, nexaArg1)"));
        assert!(generated.contains("Self.decodeBytes(options[\"data\"])"));
        assert!(generated.contains("FastMathPlugin.shared.computeHash(nexaArg0)"));
        assert!(generated.contains("await FastMathPlugin.shared.heavyCalculation(nexaArg0)"));
    }

    #[test]
    fn kotlin_bridge_emits_direct_typed_calls_for_service_scalars_and_bytes() {
        let generated = kotlin(&fast_math()).expect("render Kotlin dev bridge");
        assert!(generated.contains("FastMathPlugin.instance.add(nexaArg0, nexaArg1)"));
        assert!(generated.contains("FastMathPlugin.instance.computeHash(nexaArg0)"));
        assert!(generated.contains("FastMathPlugin.instance.heavyCalculation(nexaArg0)"));
        assert!(generated.contains("decodeBytes(options[\"data\"])"));
    }

    #[test]
    fn swift_bridge_emits_direct_construction_and_scalar_native_class_calls() {
        let generated = swift(&scalar_store()).expect("render Swift Dev bridge");
        assert!(generated.contains("case (\"Storage\", \"DevStore\")"));
        assert!(generated.contains(
            "let nexaArg1 = arguments[1] is NSNull ? nil : Self.decodeString(arguments[1])"
        ));
        assert!(generated.contains("DevStore(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.setString(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.getString(nexaArg0)"));
        assert!(generated.contains(
            "result.map { nexaEncodeOptional0 in nexaEncodeOptional0 as Any } ?? NSNull()"
        ));
    }

    #[test]
    fn kotlin_bridge_emits_direct_construction_and_scalar_native_class_calls() {
        let generated = kotlin(&scalar_store()).expect("render Kotlin Dev bridge");
        assert!(generated.contains("\"Storage.DevStore\" -> {"));
        assert!(generated.contains("if (arguments[1] == null || arguments[1] == JSONObject.NULL) null else arguments[1] as? String"));
        assert!(generated.contains("DevStore(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.setString(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.getString(nexaArg0)"));
        assert!(generated.contains("?.toInt()?.toUInt()"));
        assert!(generated.contains(
            "result?.let { nexaEncodeOptional0 -> nexaEncodeOptional0 } ?: JSONObject.NULL"
        ));
    }

    #[test]
    fn generic_plugin_calls_receive_hot_reload_value_codecs() {
        let contracts = generic_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("codecs: [Any], enumCases: [String: [String]]"));
        assert!(swift.contains("receiver as? any DevStoreSpec"));
        assert!(swift.contains("NexaDevValueCodec.write(nexaItem0, type: nexaCodec0"));
        assert!(swift.contains("NexaDevValueCodec.read(type: nexaCodec0"));
        assert!(swift.contains("nexaReceiver.setObject(nexaArg0, nexaArg1"));
        assert!(swift.contains("NexaDevValueCodec.transformMap(nexaRaw1"));
        assert!(swift.contains("NexaDevHashableValue((nexaEncodeSet1 as? NexaDevHashableValue)?.value ?? nexaEncodeSet1)"));
        assert!(swift.contains(
            "String(describing: (nexaEncodeMapEntry1.key as? NexaDevHashableValue)?.value ?? nexaEncodeMapEntry1.key)"
        ));
        assert!(swift.contains("result.map { nexaEncodeOptional0 in"));
        assert!(swift.contains("?? NSNull()"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("codecs: List<Any>, enumCases: Map<String, List<String>>"));
        assert!(kotlin.contains("NexaDevValueCodec.write(nexaItem0, nexaCodec0"));
        assert!(kotlin.contains("NexaDevValueCodec.read(nexaCodec0, nexaReader0, enumCases)"));
        assert!(kotlin.contains(
            "NexaDevValueCodec.readResult<Any?>(NexaDevValueCodec.read(nexaCodec0, nexaReader0, enumCases))"
        ));
        assert!(kotlin.contains("nexaReceiver.setObject(nexaArg0, nexaArg1"));
        assert!(kotlin.contains("as? Map<Any?, Any?>"));
        assert!(kotlin.contains(".map { nexaEncodeSet1 -> nexaEncodeSet1 }.toSet()"));
        assert!(kotlin.contains("(nexaEncodeMapEntry1.key).toString()"));
        assert!(kotlin.contains(
            "result?.let { nexaEncodeOptional0 -> nexaEncodeOptional0 } ?: JSONObject.NULL"
        ));
        assert!(swift.contains("NexaDevValueCodec.transformPair(nexaRaw0"));
        assert!(kotlin.contains("NexaDevValueCodec.transformPair(nexaRaw0"));
        assert!(kotlin.contains("NexaDevValueCodec.transformTriple(nexaRaw0"));
    }

    #[test]
    fn plugin_hot_reload_maps_typed_rows_without_actor_hops_or_repeated_name_lookups() {
        let generated_swift = swift(&sqlite()).expect("render SQLite Swift bridge");
        assert!(!generated_swift.contains("sqliteDevValue"));
        assert!(!generated_swift.contains("devRowCellValue"));
        assert!(generated_swift.contains("case .nullValue: return NSNull()"));
        assert!(
            generated_swift
                .contains("case .blob(let nexaPayload): return nexaPayload.base64EncodedString()")
        );
        assert!(generated_swift.contains("nexaIndexes.reserveCapacity(nexaExpectedFields.count)"));
        assert!(generated_swift.contains("columnNames.firstIndex(of: nexaField)"));
        assert!(generated_swift.contains("for (nexaField, nexaIndex) in nexaIndexes"));
        assert!(generated_swift.contains("try row.value(Int32(nexaIndex))"));
        assert!(generated_swift.contains("Row mapper requires column"));
        assert!(!generated_swift.contains("hot-reload SQL row"));

        let generated_kotlin = kotlin(&sqlite()).expect("render SQLite Kotlin bridge");
        assert!(!generated_kotlin.contains("sqliteDevValue"));
        assert!(!generated_kotlin.contains("devRowCellValue"));
        assert!(generated_kotlin.contains("Value.nullValue -> JSONObject.NULL"));
        assert!(
            generated_kotlin
                .contains("is Value.blob -> Base64.encodeToString(nexaCell.value, Base64.NO_WRAP)")
        );
        assert!(
            generated_kotlin
                .contains("val nexaIndexes = HashMap<String, Int>(nexaExpectedFields.size)")
        );
        assert!(generated_kotlin.contains("columnNames.indexOf(nexaField)"));
        assert!(generated_kotlin.contains("nexaIndexes.forEach"));
        assert!(generated_kotlin.contains("row.value(nexaIndex)"));
        assert!(generated_kotlin.contains("Row mapper requires column"));
        assert!(!generated_kotlin.contains("hot-reload SQL row"));
    }

    #[test]
    fn typed_row_dev_bridge_uses_the_plugin_cell_and_error_contract() {
        let contract = row_reader_plugin();
        let swift = swift(&contract).expect("render Swift row reader bridge");
        assert!(swift.contains("case .absent: return NSNull()"));
        assert!(
            swift
                .contains("case .bytes(let nexaPayload): return nexaPayload.base64EncodedString()")
        );
        assert!(swift.contains("ReadFailure.malformedRow(details:"));
        assert!(!swift.contains("SQLite"));
        assert!(!swift.contains("queryFailed"));

        let kotlin = kotlin(&contract).expect("render Kotlin row reader bridge");
        assert!(kotlin.contains("Cell.absent -> JSONObject.NULL"));
        assert!(
            kotlin
                .contains("is Cell.bytes -> Base64.encodeToString(nexaCell.value, Base64.NO_WRAP)")
        );
        assert!(kotlin.contains("throw rowFailure(\""));
        assert!(kotlin.contains("{ columnNames, rowFailure ->"));
        assert!(!kotlin.contains("SQLite"));
        assert!(!kotlin.contains("queryFailed"));
    }

    #[test]
    fn hot_reload_enum_payloads_encode_bytes_and_preserve_optional_nulls() {
        let contracts = enum_payload_store();
        let generated_swift = swift(&contracts).expect("render Swift enum payload bridge");
        assert!(generated_swift.contains("nexaPayload0.base64EncodedString() as Any"));
        assert!(generated_swift.contains("let nexaPayload0: String?"));
        assert!(generated_swift.contains("if nexaArguments[0] is NSNull"));

        let generated_kotlin = kotlin(&contracts).expect("render Kotlin enum payload bridge");
        assert!(generated_kotlin.contains("Base64.encodeToString(value.data, Base64.NO_WRAP)"));
        assert!(generated_kotlin.contains("val nexaPayload0: String?"));
        assert!(generated_kotlin.contains("nexaArguments[0] == JSONObject.NULL"));
    }

    #[test]
    fn optional_array_values_are_encoded_and_decoded_in_dev_runtime() {
        let contracts = optional_array_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("NexaDevValueCodec.transformOptionalArray(options[\"values\"]"));
        assert!(swift.contains("NexaDevValueCodec.transformOptionalArray(values[\"values\"]"));
        assert!(swift.contains("nexaReceiver.values = nexaValue"));
        assert!(swift.contains("nexaEvent0.map {"));
        assert!(swift.contains("case (\"Storage\", \"OptionalBadge\")"));
        assert!(swift.contains("OptionalBadge(labels: nexaArg_labels"));
        assert!(swift.contains("events[\"onSelected\"].map { handler in"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalArray(options[\"values\"]"));
        assert!(kotlin.contains("decodeStorageOptionalLabelsFromValue"));
        assert!(kotlin.contains("receiver.values = nexaValue"));
        assert!(kotlin.contains("nexaEvent0.map {"));
        assert!(kotlin.contains("\"Storage.OptionalBadge\" -> {"));
        assert!(kotlin.contains("OptionalBadge(labels = nexaArg_labels"));
        assert!(kotlin.contains("events[\"onSelected\"]?.invoke"));
    }

    #[test]
    fn optional_map_and_tuple_plugin_values_are_decoded_by_both_dev_bridges() {
        let contracts = optional_compound_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("NexaDevValueCodec.transformOptional("));
        assert!(swift.contains("NexaDevValueCodec.transformOptional(nexaRaw0"));
        assert!(swift.contains("NexaDevValueCodec.transformMap(options[\"values\"]"));
        assert!(swift.contains("NexaDevValueCodec.transformPair(options[\"pair\"]"));
        assert!(swift.contains("NexaDevValueCodec.transformSet(options[\"tags\"]"));
        assert!(swift.contains("case (\"Storage\", \"OptionalCompoundBadge\")"));
        assert!(swift.contains("OptionalCompoundBadge(record: nexaArg_record"));
        assert!(swift.contains("nexaReceiver.genericMap"));
        assert!(swift.contains("NexaDevValueCodec.transformOptionalSet(NexaDevValueCodec.read("));
        assert!(swift.contains("NexaDevValueCodec.transformMap(NexaDevValueCodec.read("));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("val nexaArg0: Any? = if (nexaRaw0 == JSONObject.NULL) null else (nexaRaw0) as? Any? ?: error("));
        assert!(kotlin.contains("if (nexaRaw0 == JSONObject.NULL) null else ((NexaDevValueCodec.transformPairNullable(nexaRaw0"));
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalValueMap(options[\"values\"]"));
        assert!(kotlin.contains("NexaDevValueCodec.transformPairNullable(options[\"pair\"]"));
        assert!(kotlin.contains("NexaDevValueCodec.transformTripleNullable"));
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalSet(options[\"tags\"]"));
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalSet(NexaDevValueCodec.read("));
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalSet(nexaRaw0)"));
        assert!(kotlin.contains("NexaDevValueCodec.transformOptionalValueMap(nexaRaw0"));
        assert!(kotlin.contains("\"Storage.OptionalCompoundBadge\" -> {"));
        assert!(kotlin.contains("OptionalCompoundBadge(record = nexaArg_record"));
        assert!(kotlin.contains("nexaReceiver.genericMap"));
        assert!(
            kotlin.contains("NexaDevValueCodec.transformOptionalValueMap(NexaDevValueCodec.read(")
        );
    }

    #[test]
    fn plugin_methods_route_compound_typed_errors_in_both_dev_bridges() {
        let contracts = compound_errors();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("nexaReceiver.refresh()"));
        assert!(swift.contains("try await nexaReceiver.reload()"));
        assert!(swift.contains("try await nexaReceiver.refresh()"));
        assert!(swift.contains("throw Self.nexaDevFailureStorageStoreError(error)"));
        assert!(swift.contains("try await StoragePlugin.shared.open()"));
        assert!(swift.contains("\"snapshot\": [\"values\": nexaPayload0.values.map"));
        assert!(swift.contains("\"ids\": nexaPayload1.map"));
        assert!(swift.contains("\"counts\": Dictionary(uniqueKeysWithValues:"));
        assert!(swift.contains("\"note\": (nexaPayload3.map"));
        assert!(swift.contains("throw Self.nexaDevFailureStorageStoreError(error)"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("nexaReceiver.refresh()"));
        assert!(kotlin.contains("nexaReceiver.reload()"));
        assert!(kotlin.contains("throw nexaDevFailureStorageStoreError(error)"));
        assert!(kotlin.contains("StoragePlugin.instance.open()"));
        assert!(
            kotlin
                .contains("\"snapshot\" to mapOf<String, Any>(\"values\" to error.snapshot.values")
        );
        assert!(kotlin.contains("\"ids\" to error.ids.map"));
        assert!(kotlin.contains("\"counts\" to error.counts.entries.associate"));
        assert!(kotlin.contains("\"note\" to error.note?.let"));
        assert!(kotlin.contains("throw nexaDevFailureStorageStoreError(error)"));
    }

    #[test]
    fn native_class_methods_route_array_and_struct_values_in_both_dev_bridges() {
        let contracts = compound_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("nexaReceiver.getAllKeys()"));
        assert!(swift.contains(
            "values.compactMap({ nexaDevArrayItem0 in Self.decodeString(nexaDevArrayItem0) })"
        ));
        assert!(swift.contains("nexaReceiver.removeMany(nexaArg0)"));
        assert!(swift.contains("return (true, [\"count\": result.count as Any, \"totalSize\": result.totalSize as Any])"));
        assert!(swift.contains("Self.nexaDevDecodeStorageStoreStatsFromValue(options[\"stats\"])"));
        assert!(swift.contains("Self.nexaDevDecodeStorageStoreStatsFromValue(nexaDevArrayItem0)"));
        assert!(swift.contains("nexaReceiver.onUpdated = { nexaEvent0, nexaEvent1 in"));
        assert!(swift.contains("handler([[\"count\": nexaEvent0.count as Any"));
        assert!(swift.contains("case (\"Storage\", \"StatsBadge\")"));
        assert!(swift.contains("StatsBadge(stats: nexaArg_stats"));
        assert!(swift.contains("onSelected: events[\"onSelected\"].map { handler in"));
        assert!(swift.contains("[result.0 as Any, result.1 as Any]"));
        assert!(swift.contains("[result.0 as Any, result.1 as Any, result.2 as Any]"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("nexaReceiver.getAllKeys()"));
        assert!(
            kotlin.contains(
                "values.mapNotNull { nexaDevArrayItem0 -> nexaDevArrayItem0 as? String }"
            )
        );
        assert!(kotlin.contains("nexaReceiver.removeMany(nexaArg0)"));
        assert!(kotlin.contains("return true to (mapOf<String, Any>(\"count\" to result.count, \"totalSize\" to result.totalSize) as Any)"));
        assert!(kotlin.contains("decodeStorageStoreStatsFromValue(options[\"stats\"])"));
        assert!(kotlin.contains("decodeStorageStoreStatsFromValue(nexaDevArrayItem0)"));
        assert!(kotlin.contains("receiver.onUpdated = {nexaEvent0, nexaEvent1 ->"));
        assert!(kotlin.contains("handler(listOf(mapOf<String, Any>(\"count\" to nexaEvent0.count"));
        assert!(kotlin.contains("\"Storage.StatsBadge\" -> {"));
        assert!(kotlin.contains("StatsBadge(stats = nexaArg_stats"));
        assert!(kotlin.contains("listOf<Any>(result.first as Any, result.second as Any)"));
        assert!(kotlin.contains(
            "listOf<Any>(result.first as Any, result.second as Any, result.third as Any)"
        ));
    }

    #[test]
    fn static_collection_plugin_values_are_supported_in_both_dev_bridges() {
        let contracts = collection_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("StoragePlugin.shared.normalize(nexaArg0)"));
        assert!(swift.contains("NexaDevValueCodec.transformMap(options[\"index\"]"));
        assert!(swift.contains(
            "decodeKey: { nexaDevMapKey0 in (nexaDevMapKey0 as? String).flatMap { Int32($0) } }"
        ));
        assert!(swift.contains("NexaDevValueCodec.transformSet(nexaDevMapValue0"));
        assert!(swift.contains("nexaReceiver.replace(nexaArg0)"));
        assert!(swift.contains("nexaReceiver.index = nexaValue"));
        assert!(swift.contains("handler([Dictionary(uniqueKeysWithValues:"));
        assert!(swift.contains("nexaReceiver.buckets = nexaValue"));
        assert!(swift.contains("NexaDevValueCodec.transformSet(value"));
        assert!(swift.contains(
            "decodeKey: { nexaDevMapKey0 in Self.nexaDevEnumStorageCacheBucketFromString(nexaDevMapKey0) }"
        ));
        assert!(
            swift.contains("Self.nexaDevEnumStorageCacheBucketToString(nexaEncodeMapEntry0.key)")
        );
        assert!(swift.contains("nexaReceiver.replaceIndex(nexaArg0)"));
        assert!(swift.contains("IndexBadge(index: nexaArg_index"));
        assert!(swift.contains("buckets: nexaArg_buckets"));
        assert!(swift.contains("onSelected: events[\"onSelected\"].map { handler in"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("StoragePlugin.instance.normalize(nexaArg0)"));
        assert!(kotlin.contains("NexaDevValueCodec.transformMap(options[\"index\"]"));
        assert!(kotlin.contains(
            "decodeKey = { nexaDevMapKey0 -> (nexaDevMapKey0 as? String)?.toIntOrNull() }"
        ));
        assert!(kotlin.contains("NexaDevValueCodec.transformSet(nexaDevMapValue0)"));
        assert!(kotlin.contains("nexaReceiver.replace(nexaArg0)"));
        assert!(kotlin.contains("receiver.index = nexaValue"));
        assert!(kotlin.contains("receiver.onChanged = {nexaEvent0 -> handler(listOf("));
        assert!(kotlin.contains("IndexBadge(index = nexaArg_index"));
        assert!(kotlin.contains("receiver.buckets = nexaValue"));
        assert!(kotlin.contains("NexaDevValueCodec.transformSet(value"));
        assert!(kotlin.contains("CacheBucket.values().firstOrNull { it.name == requested }"));
        assert!(kotlin.contains("nexaReceiver.replaceIndex(nexaArg0)"));
        assert!(kotlin.contains(
            "(nexaDevEnumStorageCacheBucketToValue(nexaEncodeMapEntry0.key)).toString()"
        ));
        assert!(kotlin.contains("buckets = nexaArg_buckets"));
        assert!(kotlin.contains("events[\"onSelected\"]?.invoke"));
    }

    #[test]
    fn byte_sets_are_supported_by_dev_plugin_bridges() {
        let contracts = byte_set_store();
        let swift = swift(&contracts).expect("render Swift Dev bridge");
        assert!(swift.contains("nexaReceiver.replace(nexaArg0)"));
        assert!(swift.contains("property == \"values\""));
        assert!(swift.contains("nexaReceiver.onChanged = {"));
        assert!(swift.contains("case (\"Storage\", \"ByteSetBadge\")"));

        let kotlin = kotlin(&contracts).expect("render Kotlin Dev bridge");
        assert!(kotlin.contains("nexaReceiver.replace(nexaArg0)"));
        assert!(kotlin.contains("property == \"values\""));
        assert!(kotlin.contains("receiver.onChanged = {"));
        assert!(kotlin.contains("\"Storage.ByteSetBadge\" -> {"));
        assert!(kotlin.contains("transformByteArraySet(options[\"values\"])"));
        assert!(kotlin.matches("transformByteArraySet(").count() >= 3);
    }

    #[test]
    fn swift_bridge_dispatches_plugin_properties_events_and_visual_components() {
        let generated = swift(&video_player()).expect("render Swift Dev bridge");
        assert!(generated.contains("nexaReceiver.volume = nexaValue"));
        assert!(generated.contains("nexaReceiver.state"));
        assert!(generated.contains("nexaReceiver.onEnded = {"));
        assert!(generated.contains("static func clearInstanceEvent"));
        assert!(generated.contains("case (\"VideoPlayer\", \"VideoView\")"));
        assert!(generated.contains("VideoView(player: nexaArg_player, controls: nexaArg_controls"));
        assert!(generated.contains("try await nexaReceiver.prepare(nexaArg0)"));
        assert!(generated.contains("throw Self.nexaDevFailureVideoPlayerPlayerError(error)"));
        assert!(generated.contains("case .decodingFailed(message: let nexaPayload0):"));
        assert!(generated.contains("variant: \"invalidUrl\", payload: [:]"));
        assert!(!generated.contains("onTapped"));
    }

    #[test]
    fn kotlin_bridge_dispatches_plugin_properties_events_and_visual_components() {
        let generated = kotlin(&video_player()).expect("render Kotlin Dev bridge");
        assert!(generated.contains("receiver.volume = nexaValue"));
        assert!(generated.contains("receiver.state"));
        assert!(generated.contains("receiver.onEnded = { -> handler(listOf()) }"));
        assert!(generated.contains("fun clearInstanceEvent"));
        assert!(generated.contains("\"VideoPlayer.VideoView\" -> {"));
        assert!(
            generated.contains(
                "VideoView(player = nexaArg_player, controls = nexaArg_controls, softwareDecodingEnabled = nexaArg_softwareDecodingEnabled"
            )
        );
        assert!(!generated.contains("onTapped"));
        assert!(generated.contains("nexaReceiver.prepare(nexaArg0)"));
        assert!(generated.contains("throw nexaDevFailureVideoPlayerPlayerError(error)"));
        assert!(generated.contains("\"message\" to error.message"));
    }
}
