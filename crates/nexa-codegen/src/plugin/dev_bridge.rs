//! Statically typed plugin adapters for the Dev hot-reload interpreter.
//!
//! The Dev protocol carries evaluated values dynamically, but the final hop
//! into a plugin remains a generated direct call with arguments decoded to
//! the IDL's concrete types. Keeping these adapters debug-only preserves the
//! production plugin path and avoids reflection or a runtime service locator.

use nexa_plugin_idl::{InterfaceKind, PluginIdl};

use super::bridge_plan::{
    BridgeConstructor, BridgeInterface, BridgeMethod, BridgeNamedKind, BridgeParameter, BridgePlan,
    BridgeScalar, BridgeType, BridgeTypeKind,
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
                if !supported(method) || method.is_async {
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
    out.push_str("        return (false, NSNull())\n    }\n\n");
    render_swift_instance_properties(&mut out, plugins)?;
    render_swift_instance_events(&mut out, plugins)?;
    render_swift_components(&mut out, plugins)?;
    render_swift_plugin_errors(&mut out, plugins)?;
    out.push_str(
        "    private static func decodeString(_ raw: Any?) -> String? { raw as? String }\n    private static func decodeBool(_ raw: Any?) -> Bool? { (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeBytes(_ raw: Any?) -> Data? {\n        guard let text = raw as? String else { return nil }\n        return Data(base64Encoded: text)\n    }\n    private static func decodeOptionalString(_ raw: Any?) -> String? { raw is NSNull ? nil : raw as? String }\n    private static func decodeOptionalBool(_ raw: Any?) -> Bool? { raw is NSNull ? nil : (raw as? NSNumber).map { $0.boolValue } }\n    private static func decodeOptionalBytes(_ raw: Any?) -> Data? { raw is NSNull ? nil : decodeBytes(raw) }\n    private static func decodeInt8(_ raw: Any?) -> Int8? { (raw as? NSNumber).map { Int8(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt16(_ raw: Any?) -> Int16? { (raw as? NSNumber).map { Int16(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt32(_ raw: Any?) -> Int32? { (raw as? NSNumber).map { Int32(truncatingIfNeeded: $0.int64Value) } }\n    private static func decodeInt64(_ raw: Any?) -> Int64? { (raw as? NSNumber).map { $0.int64Value } }\n    private static func decodeUInt8(_ raw: Any?) -> UInt8? { (raw as? NSNumber).map { UInt8(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt16(_ raw: Any?) -> UInt16? { (raw as? NSNumber).map { UInt16(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt32(_ raw: Any?) -> UInt32? { (raw as? NSNumber).map { UInt32(truncatingIfNeeded: $0.uint64Value) } }\n    private static func decodeUInt64(_ raw: Any?) -> UInt64? { (raw as? NSNumber).map { $0.uint64Value } }\n    private static func decodeFloat32(_ raw: Any?) -> Float? { (raw as? NSNumber).map { $0.floatValue } }\n    private static func decodeFloat64(_ raw: Any?) -> Double? { (raw as? NSNumber).map { $0.doubleValue } }\n    private static func decodeOptionalInt8(_ raw: Any?) -> Int8? { raw is NSNull ? nil : decodeInt8(raw) }\n    private static func decodeOptionalInt16(_ raw: Any?) -> Int16? { raw is NSNull ? nil : decodeInt16(raw) }\n    private static func decodeOptionalInt32(_ raw: Any?) -> Int32? { raw is NSNull ? nil : decodeInt32(raw) }\n    private static func decodeOptionalInt64(_ raw: Any?) -> Int64? { raw is NSNull ? nil : decodeInt64(raw) }\n    private static func decodeOptionalUInt8(_ raw: Any?) -> UInt8? { raw is NSNull ? nil : decodeUInt8(raw) }\n    private static func decodeOptionalUInt16(_ raw: Any?) -> UInt16? { raw is NSNull ? nil : decodeUInt16(raw) }\n    private static func decodeOptionalUInt32(_ raw: Any?) -> UInt32? { raw is NSNull ? nil : decodeUInt32(raw) }\n    private static func decodeOptionalUInt64(_ raw: Any?) -> UInt64? { raw is NSNull ? nil : decodeUInt64(raw) }\n    private static func decodeOptionalFloat32(_ raw: Any?) -> Float? { raw is NSNull ? nil : decodeFloat32(raw) }\n    private static func decodeOptionalFloat64(_ raw: Any?) -> Double? { raw is NSNull ? nil : decodeFloat64(raw) }\n}\n",
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
                if !supported(method) || method.is_async {
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
    out.push_str("        return false to JSONObject.NULL\n    }\n\n");
    render_kotlin_instance_properties(&mut out, plugins)?;
    render_kotlin_instance_events(&mut out, plugins)?;
    render_kotlin_components(&mut out, plugins)?;
    render_kotlin_plugin_errors(&mut out, plugins)?;
    out.push_str("    private fun decodeBytes(raw: Any?): ByteArray? =\n        (raw as? String)?.let { Base64.decode(it, Base64.DEFAULT) }\n    private fun decodeOptionalString(raw: Any?): String? = if (raw == null || raw == JSONObject.NULL) null else raw as? String\n    private fun decodeOptionalBool(raw: Any?): Boolean? = if (raw == null || raw == JSONObject.NULL) null else raw as? Boolean\n    private fun decodeOptionalBytes(raw: Any?): ByteArray? = if (raw == null || raw == JSONObject.NULL) null else decodeBytes(raw)\n    private fun decodeOptionalNumber(raw: Any?, type: String): Any? {\n        if (raw == null || raw == JSONObject.NULL) return null\n        val number = raw as? Number ?: return null\n        return when (type) {\n            \"Int8\" -> number.toByte()\n            \"Int16\" -> number.toShort()\n            \"Int32\" -> number.toInt()\n            \"Int64\" -> number.toLong()\n            \"UInt8\" -> number.toByte().toUByte()\n            \"UInt16\" -> number.toShort().toUShort()\n            \"UInt32\" -> number.toInt().toUInt()\n            \"UInt64\" -> number.toLong().toULong()\n            \"Float32\" -> number.toFloat()\n            else -> number.toDouble()\n        }\n    }\n}\n");
    Ok(out)
}

fn supported(method: &BridgeMethod) -> bool {
    method.type_parameters.is_empty()
        && supported_parameters(&method.parameters)
        && supported_return(method.success_type())
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
                if !supported_runtime_value_type(&property.ty) {
                    continue;
                }
                out.push_str(&format!(
                    "        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n            let nexaValue = nexaReceiver.{}\n            return (true, {})\n        }}\n",
                    swift_escape(&property.name), interface.name, property.name,
                    swift_encode_value(&property.ty, "nexaValue", namespace)
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
                if !property.mutable || !supported_runtime_value_type(&property.ty) {
                    continue;
                }
                out.push_str(&format!(
                    "        if property == \"{}\", let nexaReceiver = receiver as? {} {{\n",
                    swift_escape(&property.name),
                    interface.name
                ));
                render_swift_value_binding(
                    out,
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
        render_swift_enum_codecs(out, namespace, &plan);
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
                    .all(|parameter| supported_runtime_value_type(&parameter.ty))
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
                out.push_str(
                    &event
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(index, parameter)| {
                            swift_encode_value(
                                &parameter.ty,
                                &format!("nexaEvent{index}"),
                                namespace,
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
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
                .all(|property| supported_runtime_value_type(&property.ty))
                || !interface.events.iter().all(|event| {
                    event
                        .parameters
                        .iter()
                        .all(|parameter| supported_runtime_value_type(&parameter.ty))
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
                    let closure = format!(
                        "{{ {} in handler([{}]) }}",
                        params.join(", "),
                        params.join(", ")
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
    for (_namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !supported_runtime_value_type(&property.ty) {
                    continue;
                }
                out.push_str(&format!("        if (property == \"{}\" && receiver is {}) {{\n            val nexaValue = receiver.{}\n            return true to {}\n        }}\n", kotlin_escape(&property.name), interface.name, property.name, kotlin_encode_value(&property.ty, "nexaValue")));
            }
        }
    }
    out.push_str("        return false to JSONObject.NULL\n    }\n\n    fun writeInstanceProperty(receiver: Any, property: String, value: Any): Boolean {\n");
    for (contract_namespace, contract) in plugins {
        let plan = BridgePlan::validate_kotlin_contract(contract)?;
        for interface in plan
            .interfaces
            .iter()
            .filter(|i| i.kind == InterfaceKind::NativeClass)
        {
            for property in &interface.properties {
                if !property.mutable || !supported_runtime_value_type(&property.ty) {
                    continue;
                }
                out.push_str(&format!(
                    "        if (property == \"{}\" && receiver is {}) {{\n",
                    kotlin_escape(&property.name),
                    interface.name
                ));
                render_kotlin_value_binding(
                    out,
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
        let _ = contract_namespace;
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
    Ok(())
}

fn render_kotlin_instance_events(
    out: &mut String,
    plugins: &[(String, PluginIdl)],
) -> Result<(), String> {
    out.push_str("    fun subscribeInstanceEvent(receiver: Any, property: String, handler: (List<Any>) -> Unit): Boolean {\n");
    for (_namespace, contract) in plugins {
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
                    .all(|parameter| supported_runtime_value_type(&parameter.ty))
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
                out.push_str(
                    &(0..event.parameters.len())
                        .map(|index| format!("nexaEvent{index}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
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
                .all(|property| supported_runtime_value_type(&property.ty))
                || !interface.events.iter().all(|event| {
                    event
                        .parameters
                        .iter()
                        .all(|parameter| supported_runtime_value_type(&parameter.ty))
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
                    let values = (0..event.parameters.len())
                        .map(|index| format!("nexaEvent{index}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "{{ {} -> events[\"{property}\"]?.invoke(listOf({values})) }}",
                        params.join(", ")
                    )
                };
                args.push(format!("{property} = {callback}"));
            }
            if interface.has_content_slot {
                args.push("content = content".to_owned());
            }
            out.push_str(&format!(
                "                {}({})\n                true\n            }}\n",
                interface.name,
                args.join(", ")
            ));
        }
    }
    out.push_str("        }\n        return false\n    }\n\n");
    Ok(())
}

fn supported_runtime_value_type(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Scalar(scalar) => *scalar != BridgeScalar::Void && supported_scalar(*scalar),
        BridgeType::Named {
            kind: BridgeNamedKind::Enum | BridgeNamedKind::NativeClass,
            ..
        } => true,
        BridgeType::Optional(inner) => supported_runtime_value_type(inner),
        _ => false,
    }
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
        BridgeType::Named { name, .. } => name.clone(),
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
        BridgeType::Named { name, .. } => name.clone(),
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

fn render_swift_value_binding(
    out: &mut String,
    ty: &BridgeType,
    raw: &str,
    name: &str,
    failure: &str,
    namespace: &str,
    indent: &str,
) -> Result<(), String> {
    if let BridgeType::Optional(inner) = ty {
        let decoder = swift_decode_value(inner, raw, namespace)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
        out.push_str(&format!("{indent}let {name}: {}\n{indent}if {raw} is NSNull {{ {name} = nil }} else {{\n{indent}    guard let nexaUnwrapped = {decoder} else {{ {failure} }}\n{indent}    {name} = nexaUnwrapped\n{indent}}}\n", swift_type_for_dev(ty)));
    } else {
        let decoder = swift_decode_value(ty, raw, namespace)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
        out.push_str(&format!(
            "{indent}guard let {name} = {decoder} else {{ {failure} }}\n"
        ));
    }
    Ok(())
}

fn render_kotlin_value_binding(
    out: &mut String,
    ty: &BridgeType,
    raw: &str,
    name: &str,
    failure: &str,
    indent: &str,
) -> Result<(), String> {
    let decoder = match ty {
        BridgeType::Optional(inner) => {
            let decoder = kotlin_decode_value(inner, raw)
                .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?;
            out.push_str(&format!("{indent}val {name}: {}\n{indent}if ({raw} == null || {raw} == JSONObject.NULL) {{ {name} = null }} else {{\n", kotlin_type_for_dev(ty)));
            out.push_str(&format!("{indent}    val nexaUnwrapped = {decoder} ?: {failure}\n{indent}    {name} = nexaUnwrapped\n{indent}}}\n"));
            return Ok(());
        }
        _ => kotlin_decode_value(ty, raw)
            .ok_or_else(|| format!("unsupported Dev value type {ty:?}"))?,
    };
    out.push_str(&format!("{indent}val {name} = {decoder} ?: {failure}\n"));
    Ok(())
}

fn swift_decode_value(ty: &BridgeType, raw: &str, namespace: &str) -> Option<String> {
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
        _ => return None,
    })
}

fn kotlin_decode_value(ty: &BridgeType, raw: &str) -> Option<String> {
    Some(match ty {
        BridgeType::Scalar(BridgeScalar::String) => format!("{raw} as? String"),
        BridgeType::Scalar(BridgeScalar::Bool) => format!("{raw} as? Boolean"),
        BridgeType::Scalar(BridgeScalar::Bytes) => format!("decodeBytes({raw})"),
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
        } => format!(
            "({raw} as? {name}) ?: ({raw} as? String)?.let {{ requested -> {name}.values().firstOrNull {{ it.name == requested }} }}"
        ),
        _ => return None,
    })
}

fn swift_encode_value(ty: &BridgeType, value: &str, namespace: &str) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bytes) => format!("{value}.base64EncodedString()"),
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

fn kotlin_encode_value(ty: &BridgeType, value: &str) -> String {
    match ty {
        BridgeType::Scalar(BridgeScalar::Bytes) => {
            format!("Base64.encodeToString({value}, Base64.NO_WRAP)")
        }
        BridgeType::Named {
            kind: BridgeNamedKind::Enum,
            ..
        } => format!("{value}.name"),
        BridgeType::Optional(inner) => format!(
            "{value}?.let {{ {} }} ?: JSONObject.NULL",
            kotlin_encode_value(inner, "it")
        ),
        _ => value.to_owned(),
    }
}

fn render_swift_enum_codecs(out: &mut String, namespace: &str, plan: &BridgePlan) {
    for ty in plan
        .types
        .iter()
        .filter(|ty| ty.kind == BridgeTypeKind::Enum)
    {
        let helper = swift_enum_encoder(namespace, &ty.name);
        out.push_str(&format!(
            "    private static func {helper}(_ value: {}) -> String {{\n        switch value {{\n",
            ty.name
        ));
        for case in &ty.cases {
            out.push_str(&format!(
                "        case .{}: return \"{}\"\n",
                case.name,
                swift_escape(&case.name)
            ));
        }
        out.push_str("        }\n    }\n");
        let helper = swift_enum_decoder(namespace, &ty.name);
        out.push_str(&format!("    private static func {helper}(_ raw: Any?) -> {}? {{\n        switch raw as? String {{\n", ty.name));
        for case in &ty.cases {
            out.push_str(&format!(
                "        case \"{}\": return .{}\n",
                swift_escape(&case.name),
                case.name
            ));
        }
        out.push_str("        default: return nil\n        }\n    }\n\n");
    }
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
                        format!(
                            "\"{}\": {} as Any",
                            swift_escape(&parameter.name),
                            swift_encode_value(&parameter.ty, &binding, namespace)
                        )
                    })
                    .collect::<Vec<_>>()
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
                        format!(
                            "\"{}\" to {}",
                            kotlin_escape(&parameter.name),
                            kotlin_encode_value(
                                &parameter.ty,
                                &format!("error.{}", parameter.name)
                            )
                        )
                    })
                    .collect::<Vec<_>>()
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
                if !supported(method) || (!asynchronous && method.is_async) {
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
                    render_swift_result(out, method.success_type(), indent);
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
                if !supported(method) || (!asynchronous && method.is_async) {
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
                    render_kotlin_result(out, method.success_type(), indent);
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
    if !asynchronous && method.is_async {
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
        render_swift_result(out, method.success_type(), indent);
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
}

fn render_kotlin_method(
    out: &mut String,
    namespace: &str,
    method: &BridgeMethod,
    asynchronous: bool,
) {
    if !asynchronous && method.is_async {
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
        render_kotlin_result(out, method.success_type(), indent);
    }
    if let Some(error_type) = method.error_type() {
        let helper = dev_error_helper(namespace, error_type);
        out.push_str(&format!(
            "                }} catch (error: {error_type}) {{\n                    throw {helper}(error)\n                }}\n",
        ));
    }
    out.push_str("            }\n");
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

    fn video_player() -> Vec<(String, PluginIdl)> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/plugins/video-player/native.nxid");
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
            generated.contains("VideoView(player = nexaArg_player, controls = nexaArg_controls")
        );
        assert!(generated.contains("nexaReceiver.prepare(nexaArg0)"));
        assert!(generated.contains("throw nexaDevFailureVideoPlayerPlayerError(error)"));
        assert!(generated.contains("\"message\" to error.message"));
    }
}
