//! Statically typed plugin adapters for the Dev hot-reload interpreter.
//!
//! The Dev protocol carries evaluated values dynamically, but the final hop
//! into a plugin remains a generated direct call with arguments decoded to
//! the IDL's concrete types. Keeping these adapters debug-only preserves the
//! production plugin path and avoids reflection or a runtime service locator.

use nexa_plugin_idl::{InterfaceKind, PluginIdl};

use super::bridge_plan::{
    BridgeConstructor, BridgeInterface, BridgeMethod, BridgeParameter, BridgePlan, BridgeScalar,
    BridgeType,
};

pub(super) fn swift(plugins: &[(String, PluginIdl)]) -> Result<String, String> {
    let mut out = String::from(
        "import Foundation\n\n@MainActor\ninternal enum NexaDevPluginBridge {\n    static func construct(namespace: String, name: String, arguments: [Any]) -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            if let Some(constructor) = interface.constructors.first()
                && supported_parameters(&constructor.parameters)
            {
                render_swift_constructor(&mut out, namespace, interface, constructor);
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeSync(namespace: String, name: String, options: [String: Any]) -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method) || method.is_async || method.error_type().is_some() {
                    continue;
                }
                render_swift_method(&mut out, namespace, method, false);
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeAsync(namespace: String, name: String, options: [String: Any]) async throws -> (Bool, Any) {\n        switch (namespace, name) {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_swift_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method) {
                    continue;
                }
                render_swift_method(&mut out, namespace, method, true);
            }
        }
    }
    out.push_str(
        "        default: return (false, NSNull())\n        }\n    }\n\n    static func invokeInstanceSync(receiver: Any, namespace: String, name: String, options: [String: Any]) -> (Bool, Any) {\n",
    );
    render_swift_instance_methods(&mut out, plugins, false)?;
    out.push_str(
        "        return (false, NSNull())\n    }\n\n    static func invokeInstanceAsync(receiver: Any, namespace: String, name: String, options: [String: Any]) async throws -> (Bool, Any) {\n",
    );
    render_swift_instance_methods(&mut out, plugins, true)?;
    out.push_str(
        "        return (false, NSNull())\n    }\n\n    private static func decodeString(_ raw: Any?) -> String? { raw as? String }\n    private static func decodeBool(_ raw: Any?) -> Bool? { (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeBytes(_ raw: Any?) -> Data? {\n        guard let text = raw as? String else { return nil }\n        return Data(base64Encoded: text)\n    }\n    private static func decodeOptionalString(_ raw: Any?) -> String? { raw is NSNull ? nil : raw as? String }\n    private static func decodeOptionalBool(_ raw: Any?) -> Bool? { raw is NSNull ? nil : (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeOptionalBytes(_ raw: Any?) -> Data? { raw is NSNull ? nil : decodeBytes(raw) }\n    private static func decodeInt8(_ raw: Any?) -> Int8? { (raw as? NSNumber).map { Int8(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt16(_ raw: Any?) -> Int16? { (raw as? NSNumber).map { Int16(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt32(_ raw: Any?) -> Int32? { (raw as? NSNumber).map { Int32(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt64(_ raw: Any?) -> Int64? { (raw as? NSNumber).map { $0.int64Value } }\n    private static func decodeUInt8(_ raw: Any?) -> UInt8? { (raw as? NSNumber).map { UInt8(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt16(_ raw: Any?) -> UInt16? { (raw as? NSNumber).map { UInt16(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt32(_ raw: Any?) -> UInt32? { (raw as? NSNumber).map { UInt32(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt64(_ raw: Any?) -> UInt64? { (raw as? NSNumber).map { $0.uint64Value } }\n    private static func decodeFloat32(_ raw: Any?) -> Float? { (raw as? NSNumber).map { $0.floatValue } }\n    private static func decodeFloat64(_ raw: Any?) -> Double? { (raw as? NSNumber).map { $0.doubleValue } }\n    private static func decodeOptionalInt8(_ raw: Any?) -> Int8? { raw is NSNull ? nil : decodeInt8(raw) }\n    private static func decodeOptionalInt16(_ raw: Any?) -> Int16? { raw is NSNull ? nil : decodeInt16(raw) }\n    private static func decodeOptionalInt32(_ raw: Any?) -> Int32? { raw is NSNull ? nil : decodeInt32(raw) }\n    private static func decodeOptionalInt64(_ raw: Any?) -> Int64? { raw is NSNull ? nil : decodeInt64(raw) }\n    private static func decodeOptionalUInt8(_ raw: Any?) -> UInt8? { raw is NSNull ? nil : decodeUInt8(raw) }\n    private static func decodeOptionalUInt16(_ raw: Any?) -> UInt16? { raw is NSNull ? nil : decodeUInt16(raw) }\n    private static func decodeOptionalUInt32(_ raw: Any?) -> UInt32? { raw is NSNull ? nil : decodeUInt32(raw) }\n    private static func decodeOptionalUInt64(_ raw: Any?) -> UInt64? { raw is NSNull ? nil : decodeUInt64(raw) }\n    private static func decodeOptionalFloat32(_ raw: Any?) -> Float? { raw is NSNull ? nil : decodeFloat32(raw) }\n    private static func decodeOptionalFloat64(_ raw: Any?) -> Double? { raw is NSNull ? nil : decodeFloat64(raw) }\n}\n",
    );
    Ok(out)
}

pub(super) fn kotlin(plugins: &[(String, PluginIdl)]) -> Result<String, String> {
    let mut out = String::from(
        "import android.util.Base64\nimport org.json.JSONObject\n\ninternal object NexaDevPluginBridge {\n    fun construct(namespace: String, name: String, arguments: List<Any>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::NativeClass)
        {
            if let Some(constructor) = interface.constructors.first()
                && supported_parameters(&constructor.parameters)
            {
                render_kotlin_constructor(&mut out, namespace, interface, constructor);
            }
        }
    }
    out.push_str(
        "            else -> false to JSONObject.NULL\n        }\n\n    fun invokeSync(namespace: String, name: String, options: Map<String, Any>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method) || method.is_async || method.error_type().is_some() {
                    continue;
                }
                render_kotlin_method(&mut out, namespace, method, false);
            }
        }
    }
    out.push_str(
        "            else -> false to JSONObject.NULL\n        }\n\n    suspend fun invokeAsync(namespace: String, name: String, options: Map<String, Any>): Pair<Boolean, Any> =\n        when (\"$namespace.$name\") {\n",
    );
    for (namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                if !supported(method) {
                    continue;
                }
                render_kotlin_method(&mut out, namespace, method, true);
            }
        }
    }
    out.push_str("            else -> false to JSONObject.NULL\n        }\n\n    fun invokeInstanceSync(receiver: Any, namespace: String, name: String, options: Map<String, Any>): Pair<Boolean, Any> {\n");
    render_kotlin_instance_methods(&mut out, plugins, false)?;
    out.push_str("        return false to JSONObject.NULL\n    }\n\n    suspend fun invokeInstanceAsync(receiver: Any, namespace: String, name: String, options: Map<String, Any>): Pair<Boolean, Any> {\n");
    render_kotlin_instance_methods(&mut out, plugins, true)?;
    out.push_str("        return false to JSONObject.NULL\n    }\n\n    private fun decodeBytes(raw: Any?): ByteArray? =\n        (raw as? String)?.let { Base64.decode(it, Base64.DEFAULT) }\n    private fun decodeOptionalString(raw: Any?): String? = if (raw == null || raw == JSONObject.NULL) null else raw as? String\n    private fun decodeOptionalBool(raw: Any?): Boolean? = if (raw == null || raw == JSONObject.NULL) null else raw as? Boolean\n    private fun decodeOptionalBytes(raw: Any?): ByteArray? = if (raw == null || raw == JSONObject.NULL) null else decodeBytes(raw)\n    private fun decodeOptionalNumber(raw: Any?, type: String): Any? {\n        if (raw == null || raw == JSONObject.NULL) return null\n        val number = raw as? Number ?: return null\n        return when (type) {\n            \"Int8\" -> number.toByte()\n            \"Int16\" -> number.toShort()\n            \"Int32\" -> number.toInt()\n            \"Int64\" -> number.toLong()\n            \"UInt8\" -> number.toByte().toUByte()\n            \"UInt16\" -> number.toShort().toUShort()\n            \"UInt32\" -> number.toInt().toUInt()\n            \"UInt64\" -> number.toLong().toULong()\n            \"Float32\" -> number.toFloat()\n            else -> number.toDouble()\n        }\n    }\n}\n");
    Ok(out)
}

fn supported(method: &BridgeMethod) -> bool {
    method.type_parameters.is_empty()
        && supported_parameters(&method.parameters)
        && supported_return(method.success_type())
}

fn supported_parameters(parameters: &[BridgeParameter]) -> bool {
    parameters
        .iter()
        .all(|parameter| supported_value_type(&parameter.ty))
}

fn supported_return(ty: &BridgeType) -> bool {
    ty.is_void() || supported_value_type(ty)
}

fn supported_value_type(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Optional(inner) => matches!(
            inner.as_ref(),
            BridgeType::Scalar(scalar)
                if *scalar != BridgeScalar::Void && supported_scalar(*scalar)
        ),
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
    )
}

fn render_swift_constructor(
    out: &mut String,
    namespace: &str,
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
                if !supported(method)
                    || method.error_type().is_some()
                    || (!asynchronous && method.is_async)
                {
                    continue;
                }
                out.push_str(&format!(
                    "        if namespace == \"{}\", name == \"{}\", let nexaReceiver = receiver as? {} {{\n",
                    swift_escape(namespace),
                    swift_escape(&method.name),
                    interface.name
                ));
                for (index, parameter) in method.parameters.iter().enumerate() {
                    render_swift_decode_argument(
                        out,
                        index,
                        &parameter.ty,
                        &format!("options[\"{}\"]", swift_escape(&parameter.name)),
                        "            ",
                    );
                }
                let arguments = (0..method.parameters.len())
                    .map(|index| format!("nexaArg{index}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let prefix = if method.is_async { "await " } else { "" };
                if method.success_type().is_void() {
                    out.push_str(&format!(
                        "            {prefix}nexaReceiver.{}({arguments})\n            return (true, NSNull())\n        }}\n",
                        method.name
                    ));
                } else {
                    out.push_str(&format!(
                        "            let result = {prefix}nexaReceiver.{}({arguments})\n",
                        method.name
                    ));
                    render_swift_result(out, method.success_type(), "            ");
                    out.push_str("        }\n");
                }
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
                if !supported(method)
                    || method.error_type().is_some()
                    || (!asynchronous && method.is_async)
                {
                    continue;
                }
                out.push_str(&format!(
                    "        if (namespace == \"{}\" && name == \"{}\" && receiver is {}) {{\n            val nexaReceiver = receiver\n",
                    kotlin_escape(namespace),
                    kotlin_escape(&method.name),
                    interface.name
                ));
                for (index, parameter) in method.parameters.iter().enumerate() {
                    render_kotlin_decode_argument(
                        out,
                        index,
                        &parameter.ty,
                        &format!("options[\"{}\"]", kotlin_escape(&parameter.name)),
                        &format!(
                            "Invalid argument `{}` for {}.{}",
                            parameter.name, namespace, method.name
                        ),
                        "            ",
                    );
                }
                let arguments = (0..method.parameters.len())
                    .map(|index| format!("nexaArg{index}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if method.success_type().is_void() {
                    out.push_str(&format!(
                        "            nexaReceiver.{}({arguments})\n            return true to JSONObject.NULL\n        }}\n",
                        method.name
                    ));
                } else {
                    out.push_str(&format!(
                        "            val result = nexaReceiver.{}({arguments})\n",
                        method.name
                    ));
                    render_kotlin_result(out, method.success_type(), "            ");
                    out.push_str("        }\n");
                }
            }
        }
    }
    Ok(())
}

fn render_swift_decode_argument(
    out: &mut String,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    indent: &str,
) {
    let decoder = swift_decoder(ty);
    if matches!(ty, BridgeType::Optional(_)) {
        out.push_str(&format!(
            "{indent}let nexaArg{index} = Self.{decoder}({raw})\n"
        ));
    } else {
        out.push_str(&format!(
            "{indent}guard let nexaArg{index} = Self.{decoder}({raw}) else {{ return (true, NSNull()) }}\n"
        ));
    }
}

fn render_kotlin_decode_argument(
    out: &mut String,
    index: usize,
    ty: &BridgeType,
    raw: &str,
    error: &str,
    indent: &str,
) {
    let decoder = kotlin_value_decoder(ty, raw);
    if matches!(ty, BridgeType::Optional(_)) {
        out.push_str(&format!("{indent}val nexaArg{index} = {decoder}\n"));
    } else {
        out.push_str(&format!(
            "{indent}val nexaArg{index} = {decoder} ?: error(\"{}\")\n",
            kotlin_escape(error)
        ));
    }
}

fn render_swift_result(out: &mut String, ty: &BridgeType, indent: &str) {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bytes) => out.push_str(&format!(
            "{indent}return (true, result.base64EncodedString())\n"
        )),
        BridgeType::Optional(inner)
            if matches!(inner.as_ref(), BridgeType::Scalar(BridgeScalar::Bytes)) =>
        {
            out.push_str(&format!(
                "{indent}if let result {{ return (true, result.base64EncodedString()) }}\n{indent}return (true, NSNull())\n"
            ));
        }
        BridgeType::Optional(_) => out.push_str(&format!(
            "{indent}if let result {{ return (true, result) }}\n{indent}return (true, NSNull())\n"
        )),
        _ => out.push_str(&format!("{indent}return (true, result)\n")),
    }
}

fn render_kotlin_result(out: &mut String, ty: &BridgeType, indent: &str) {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bytes) => out.push_str(&format!(
            "{indent}return true to Base64.encodeToString(result, Base64.NO_WRAP)\n"
        )),
        BridgeType::Optional(inner)
            if matches!(inner.as_ref(), BridgeType::Scalar(BridgeScalar::Bytes)) =>
        {
            out.push_str(&format!(
                "{indent}return true to (result?.let {{ Base64.encodeToString(it, Base64.NO_WRAP) }} ?: JSONObject.NULL)\n"
            ));
        }
        BridgeType::Optional(_) => out.push_str(&format!(
            "{indent}return true to (result ?: JSONObject.NULL)\n"
        )),
        _ => out.push_str(&format!("{indent}return true to result\n")),
    }
}

fn kotlin_value_decoder(ty: &BridgeType, raw: &str) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::String) => format!("{raw} as? String"),
        BridgeType::Scalar(BridgeScalar::Bool) => format!("{raw} as? Boolean"),
        BridgeType::Scalar(scalar) if kotlin_number_conversion(*scalar).is_some() => {
            format!(
                "({raw} as? Number)?.{}()",
                kotlin_number_conversion(*scalar).unwrap()
            )
        }
        BridgeType::Scalar(BridgeScalar::Bytes) => format!("decodeBytes({raw})"),
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Scalar(BridgeScalar::String) => format!("decodeOptionalString({raw})"),
            BridgeType::Scalar(BridgeScalar::Bool) => format!("decodeOptionalBool({raw})"),
            BridgeType::Scalar(BridgeScalar::Bytes) => format!("decodeOptionalBytes({raw})"),
            BridgeType::Scalar(scalar) if kotlin_number_conversion(*scalar).is_some() => {
                format!(
                    "({raw} as? Number)?.{}()",
                    kotlin_number_conversion(*scalar).unwrap()
                )
            }
            _ => unreachable!("unsupported optional plugin argument passed generation"),
        },
        _ => unreachable!("unsupported plugin argument passed generation"),
    }
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

fn render_swift_method(
    out: &mut String,
    namespace: &str,
    method: &BridgeMethod,
    asynchronous: bool,
) {
    if !asynchronous && (method.is_async || method.error_type().is_some()) {
        return;
    }
    out.push_str(&format!(
        "        case (\"{}\", \"{}\"):\n",
        swift_escape(namespace),
        swift_escape(&method.name)
    ));
    for (index, parameter) in method.parameters.iter().enumerate() {
        render_swift_decode_argument(
            out,
            index,
            &parameter.ty,
            &format!("options[\"{}\"]", swift_escape(&parameter.name)),
            "            ",
        );
    }
    let args = (0..method.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let invoke = format!("{}Plugin.shared.{}({args})", namespace, method.name);
    let prefix = match (method.is_async, method.error_type().is_some()) {
        (true, true) => "try await ",
        (true, false) => "await ",
        (false, true) => "try ",
        (false, false) => "",
    };
    if method.success_type().is_void() {
        out.push_str(&format!(
            "            {prefix}{invoke}\n            return (true, NSNull())\n"
        ));
    } else {
        out.push_str(&format!("            let result = {prefix}{invoke}\n"));
        render_swift_result(out, method.success_type(), "            ");
    }
}

fn render_kotlin_method(
    out: &mut String,
    namespace: &str,
    method: &BridgeMethod,
    asynchronous: bool,
) {
    if !asynchronous && (method.is_async || method.error_type().is_some()) {
        return;
    }
    out.push_str(&format!(
        "            \"{}.{}\" -> {{\n",
        kotlin_escape(namespace),
        kotlin_escape(&method.name)
    ));
    for (index, parameter) in method.parameters.iter().enumerate() {
        render_kotlin_decode_argument(
            out,
            index,
            &parameter.ty,
            &format!("options[\"{}\"]", kotlin_escape(&parameter.name)),
            &format!(
                "Invalid argument `{}` for {namespace}.{}",
                parameter.name, method.name
            ),
            "                ",
        );
    }
    let args = (0..method.parameters.len())
        .map(|index| format!("nexaArg{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let call = format!("{}Plugin.instance.{}({args})", namespace, method.name);
    let invoke = if method.is_async {
        format!("val result = {call}")
    } else if method.success_type().is_void() {
        call.to_owned()
    } else {
        format!("val result = {call}")
    };
    out.push_str(&format!("                {invoke}\n"));
    if method.success_type().is_void() {
        out.push_str("                true to JSONObject.NULL\n            }\n");
    } else {
        render_kotlin_result(out, method.success_type(), "                ");
        out.push_str("            }\n");
    }
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
        BridgeType::Scalar(BridgeScalar::Bytes) => "decodeBytes",
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
        assert!(generated.contains("let nexaArg1 = Self.decodeOptionalString(arguments[1])"));
        assert!(generated.contains("DevStore(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.setString(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.getString(nexaArg0)"));
        assert!(generated.contains("if let result { return (true, result) }"));
    }

    #[test]
    fn kotlin_bridge_emits_direct_construction_and_scalar_native_class_calls() {
        let generated = kotlin(&scalar_store()).expect("render Kotlin Dev bridge");
        assert!(generated.contains("\"Storage.DevStore\" -> {"));
        assert!(generated.contains("decodeOptionalString(arguments[1])"));
        assert!(generated.contains("DevStore(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.setString(nexaArg0, nexaArg1)"));
        assert!(generated.contains("nexaReceiver.getString(nexaArg0)"));
        assert!(generated.contains("?.toInt()?.toUInt()"));
        assert!(generated.contains("return true to (result ?: JSONObject.NULL)"));
    }
}
