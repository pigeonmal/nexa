//! Swift-to-C++ adapter rendering.
//!
//! Emits the C++ entry points the Swift `NexaCpp` overlay calls, along with
//! the value-type spellings, named-value helpers, async future adapters, and
//! event delivery machinery those entry points need.

use super::abi::{
    bridge_strip_optional, bridge_type_is_generic, bridge_type_name, cpp_byte_buffer_alias_name,
    cpp_getter_name, cpp_identifier, cpp_namespace, cpp_optional_bridge_name, cpp_setter_name,
    cpp_swift_class_method_adapter_name, cpp_swift_event_copy_adapter_name,
    cpp_swift_event_setter_name, cpp_swift_property_adapter_name, cpp_swift_service_adapter_name,
};
use super::collections::{
    cpp_swift_array_alias_name, cpp_swift_map_entries_name, cpp_swift_map_entry_name,
    swift_cpp_argument_expression, swift_cpp_array_argument_expression, swift_cpp_array_swift_type,
    swift_cpp_map_types, swift_cpp_map_value_argument, swift_cpp_method_uses_collection_adapter,
    swift_cpp_needs_vector_bridge, swift_cpp_nested_array_supported, swift_cpp_result_expression,
    swift_cpp_set_element_type,
};
use super::errors::{
    TypedErrorCall, render_swift_cpp_error_converters, render_swift_cpp_typed_error_call,
};
use crate::SourceWriter;
use crate::plugin::bridge_plan::{
    BridgeEvent, BridgeInterface, BridgeMethod, BridgeNamedType, BridgeParameter, BridgePlan,
    BridgeType, BridgeTypeKind,
};
use crate::plugin::type_visit::collect_value_types;
use nexa_plugin_idl::{self, InterfaceKind};

/// Values fixed for one Swift adapter render and derived from the plan once.
///
/// The C++ namespace, byte-buffer alias, and optional-bridge name are needed
/// by nearly every value-spelling helper. Binding them to the plan they came
/// from means a helper cannot be called with a namespace from a different
/// render, and keeps the conversion helpers to one context parameter instead of
/// four positional ones.
pub(crate) struct SwiftAdapterContext<'a> {
    pub plan: &'a BridgePlan,
    pub namespace: &'a str,
    pub byte_buffer_type: &'a str,
    pub optional_bridge: &'a str,
}

/// Emits direct Swift adapters for supported C++ plugin values.
/// Unsupported signatures fail generation instead of producing a wrapper that
/// cannot honor the IDL.
pub fn render_swift_adapters(plan: &BridgePlan, plugin_id: &str) -> Result<String, String> {
    let namespace = cpp_namespace(plugin_id).replace("::", ".");
    let byte_buffer_type = format!("{namespace}.{}", cpp_byte_buffer_alias_name(plan));
    let optional_bridge = format!("{namespace}.{}", cpp_optional_bridge_name(plan));
    // Named `ctx` because `context` is already the Swift event context
    // handle name inside the per-interface loops below.
    let ctx = SwiftAdapterContext {
        plan,
        namespace: &namespace,
        byte_buffer_type: &byte_buffer_type,
        optional_bridge: &optional_bridge,
    };
    let mut out = SourceWriter::new();
    out.push_str("import CxxStdlib\nimport Foundation\n\n");
    render_swift_cpp_named_value_helpers(&mut out, &ctx);
    if plan
        .interfaces
        .iter()
        .flat_map(|interface| &interface.methods)
        .any(|method| method.is_async)
    {
        out.push_str(
            "private final class NexaCppFutureWork<Output>: @unchecked Sendable {\n    private let operation: () -> Output\n    init(_ operation: @escaping () -> Output) { self.operation = operation }\n    func run() -> Output { operation() }\n}\n\n",
        );
    }
    render_swift_cpp_error_converters(&mut out, &ctx);
    for interface in plan
        .interfaces
        .iter()
        .filter(|interface| interface.kind == InterfaceKind::NativeClass)
    {
        for event in &interface.events {
            render_swift_cpp_event_adapters(&mut out, &ctx, plugin_id, interface, event);
        }
    }
    let mut generated = false;

    for interface in &plan.interfaces {
        match interface.kind {
            InterfaceKind::Service => {
                generated = true;
                out.push_str(&format!(
                    "public final class {}Plugin: {}, @unchecked Sendable {{\n    public static let shared = {}Plugin()\n    private init() {{}}\n",
                    interface.name, interface.name, interface.name
                ));
                for method in &interface.methods {
                    let uses_collection_adapter = swift_cpp_method_uses_collection_adapter(method);
                    let receiver = if uses_collection_adapter {
                        namespace.clone()
                    } else {
                        // Direct service calls are nested under their IDL interface namespace.
                        // Collection adapters are emitted into the plugin namespace because
                        // they convert Swift-friendly vector types to the C++ collection contract.
                        format!("{namespace}.{}", cpp_identifier(&interface.name))
                    };
                    let adapter = uses_collection_adapter
                        .then(|| cpp_swift_service_adapter_name(&interface.name, &method.name));
                    render_swift_cpp_method(
                        &mut out,
                        &receiver,
                        adapter.as_deref(),
                        &ctx,
                        method,
                        1,
                    );
                }
                out.push_str("}\n\n");
            }
            InterfaceKind::NativeClass => {
                generated = true;
                let cpp_type = format!("{namespace}.{}Spec", cpp_identifier(&interface.name));
                out.push_str(&format!(
                    "@MainActor\npublic final class {0}Impl: {0}Spec {{\n    private let nexaCppObject: {cpp_type}\n",
                    interface.name
                ));
                for event in &interface.events {
                    let context = swift_cpp_event_context_name(plugin_id, interface, event);
                    let backing = swift_cpp_event_backing_name(interface, event);
                    out.push_str(&format!("    private var {backing}: {context}?\n"));
                }
                let constructor = interface.constructors.first();
                let parameters = constructor
                    .map(|constructor| swift_cpp_parameters(&constructor.parameters))
                    .unwrap_or_default();
                let factory_arguments = constructor
                    .map(|constructor| swift_cpp_arguments(&constructor.parameters, &ctx))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "\n    required public init({parameters}) {{\n        nexaCppObject = {namespace}.nexaMake{}ForSwift({factory_arguments})\n    }}\n",
                    cpp_identifier(&interface.name)
                ));
                for property in &interface.properties {
                    let ty = swift_cpp_value_type(&property.ty)
                        .expect("property type was validated above");
                    let getter = if swift_cpp_needs_vector_bridge(&property.ty) {
                        format!(
                            "nexaCppObject.{}()",
                            cpp_swift_property_adapter_name(interface, &property.name, false)
                        )
                    } else {
                        format!("nexaCppObject.{}()", cpp_getter_name(&property.name))
                    };
                    let getter = swift_cpp_result_expression(&property.ty, &getter, &ctx);
                    out.push_str(&format!(
                        "\n    public var {}: {ty} {{\n        get {{ {getter} }}",
                        property.name
                    ));
                    if property.mutable {
                        let setter_value =
                            swift_cpp_argument_expression(&property.ty, "newValue", &ctx);
                        let setter = if swift_cpp_needs_vector_bridge(&property.ty) {
                            cpp_swift_property_adapter_name(interface, &property.name, true)
                        } else {
                            cpp_setter_name(&property.name)
                        };
                        out.push_str(&format!(
                            "\n        set {{ nexaCppObject.{setter}({setter_value}) }}"
                        ));
                    }
                    out.push_str("\n    }\n");
                }
                for event in &interface.events {
                    render_swift_cpp_event_property(&mut out, &ctx, plugin_id, interface, event);
                }
                for method in &interface.methods {
                    if method.name == "dispose"
                        && !interface.events.is_empty()
                        && method.parameters.is_empty()
                        && !method.is_async
                        && method.error_type().is_none()
                        && swift_cpp_value_type(method.success_type())
                            .is_some_and(|ty| ty == "Void")
                    {
                        render_swift_cpp_dispose(&mut out, interface, method);
                    } else {
                        render_swift_cpp_method(
                            &mut out,
                            "nexaCppObject",
                            swift_cpp_method_uses_collection_adapter(method)
                                .then(|| {
                                    cpp_swift_class_method_adapter_name(interface, &method.name)
                                })
                                .as_deref(),
                            &ctx,
                            method,
                            1,
                        );
                    }
                }
                out.push_str("}\n\n");
            }
            InterfaceKind::Interface | InterfaceKind::NativeComponent => {}
        }
    }

    if generated {
        Ok(out.finish())
    } else {
        Ok(String::new())
    }
}

fn render_swift_cpp_event_adapters(
    out: &mut SourceWriter,
    // `context` is the generated Swift event context class name below.
    adapter: &SwiftAdapterContext<'_>,
    plugin_id: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) {
    let SwiftAdapterContext {
        plan, namespace, ..
    } = adapter;
    let context = swift_cpp_event_context_name(plugin_id, interface, event);
    let delivery = swift_cpp_event_delivery_name(plugin_id, interface, event);
    let callback_type = swift_cpp_event_callback_type(event);
    let token = swift_cpp_event_token(plugin_id, interface, event);
    let invoke_symbol = format!("nexaCppEventInvoke{token}");
    let release_symbol = format!("nexaCppEventRelease{token}");

    out.push_str(&format!(
        "private final class {context}: @unchecked Sendable {{\n    let callback: {callback_type}\n    private let lock = NSLock()\n    private var active = true\n    init(_ callback: @escaping {callback_type}) {{ self.callback = callback }}\n    func deactivate() {{ lock.lock(); active = false; lock.unlock() }}\n    func isActive() -> Bool {{ lock.lock(); defer {{ lock.unlock() }}; return active }}\n}}\n\n"
    ));
    out.push_str(&format!(
        "private final class {delivery}: @unchecked Sendable {{\n    let context: {context}\n"
    ));
    for (index, parameter) in event.parameters.iter().enumerate() {
        let ty = swift_cpp_value_type(&parameter.ty).expect("event type was validated");
        out.push_str(&format!("    let value{index}: {ty}\n"));
    }
    let initializers = std::iter::once(format!("context: {context}"))
        .chain(
            event
                .parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    format!(
                        "value{index}: {}",
                        swift_cpp_value_type(&parameter.ty).expect("validated event type")
                    )
                }),
        )
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "    init({initializers}) {{ self.context = context"
    ));
    for (index, _) in event.parameters.iter().enumerate() {
        out.push_str(&format!("; self.value{index} = value{index}"));
    }
    out.push_str(" }\n    @MainActor func deliver() {\n        guard context.isActive() else { return }\n        context.callback(");
    out.push_str(
        &(0..event.parameters.len())
            .map(|index| format!("value{index}"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str(")\n    }\n}\n\n");

    let raw_parameters = std::iter::once("_ rawContext: UnsafeMutableRawPointer".to_owned())
        .chain(
            event
                .parameters
                .iter()
                .enumerate()
                .map(|(index, _)| format!("_ rawValue{index}: UnsafeRawPointer")),
        )
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "@_cdecl(\"{invoke_symbol}\")\nprivate func {invoke_symbol}({raw_parameters}) {{\n    let context = Unmanaged<{context}>.fromOpaque(rawContext).takeUnretainedValue()\n    guard context.isActive() else {{ return }}\n"
    ));
    let mut delivery_arguments = vec!["context: context".to_owned()];
    for (index, parameter) in event.parameters.iter().enumerate() {
        out.push_str(&format!(
            "    let nexaCxxEventValue{index} = {namespace}.{}(rawValue{index})\n",
            cpp_swift_event_copy_adapter_name(plan, interface, event, index)
        ));
        let swift_value = swift_cpp_result_expression(
            &parameter.ty,
            &format!("nexaCxxEventValue{index}"),
            adapter,
        );
        out.push_str(&format!("    let value{index} = {swift_value}\n"));
        delivery_arguments.push(format!("value{index}: value{index}"));
    }
    out.push_str(&format!(
        "    let delivery = {delivery}({})\n    Task {{ @MainActor in delivery.deliver() }}\n}}\n\n",
        delivery_arguments.join(", ")
    ));
    out.push_str(&format!(
        "@_cdecl(\"{release_symbol}\")\nprivate func {release_symbol}(_ rawContext: UnsafeMutableRawPointer) {{\n    let context = Unmanaged<{context}>.fromOpaque(rawContext).takeRetainedValue()\n    Task {{ @MainActor in withExtendedLifetime(context) {{ }} }}\n}}\n\n"
    ));
}

fn render_swift_cpp_event_property(
    out: &mut SourceWriter,
    context: &SwiftAdapterContext<'_>,
    plugin_id: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) {
    let _ = context;
    let property = nexa_plugin_idl::event_callback_property(&event.name);
    let callback_type = swift_cpp_event_callback_type(event);
    let backing = swift_cpp_event_backing_name(interface, event);
    let token = swift_cpp_event_token(plugin_id, interface, event);
    let invoke_symbol = format!("nexaCppEventInvoke{token}");
    let release_symbol = format!("nexaCppEventRelease{token}");
    let setter = cpp_swift_event_setter_name(interface, event);
    out.push_str(&format!(
        "\n    public var {property}: ({callback_type})? {{\n        get {{ {backing}?.callback }}\n        set {{\n            {backing}?.deactivate()\n            guard let callback = newValue else {{\n                {backing} = nil\n                nexaCppObject.{setter}(nil, nil, nil)\n                return\n            }}\n            let context = {context}(callback)\n            {backing} = context\n            nexaCppObject.{setter}({invoke_symbol}, Unmanaged.passRetained(context).toOpaque(), {release_symbol})\n        }}\n    }}\n",
        context = swift_cpp_event_context_name(plugin_id, interface, event)
    ));
}

fn render_swift_cpp_dispose(
    out: &mut SourceWriter,
    interface: &BridgeInterface,
    method: &BridgeMethod,
) {
    out.push_str(&format!("\n    public func {}() -> Void {{\n", method.name));
    for event in &interface.events {
        let backing = swift_cpp_event_backing_name(interface, event);
        let setter = cpp_swift_event_setter_name(interface, event);
        out.push_str(&format!(
            "        {backing}?.deactivate()\n        {backing} = nil\n        nexaCppObject.{setter}(nil, nil, nil)\n"
        ));
    }
    out.push_str("        nexaCppObject.dispose()\n    }\n");
}

fn swift_cpp_event_callback_type(event: &BridgeEvent) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| swift_cpp_value_type(&parameter.ty).expect("validated event type"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({parameters}) -> Void")
}

fn swift_cpp_event_token(
    plugin_id: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    format!(
        "{}_{}_{}",
        cpp_identifier(plugin_id),
        cpp_identifier(&interface.name),
        cpp_identifier(&event.name)
    )
}

fn swift_cpp_event_context_name(
    plugin_id: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    format!(
        "NexaCppEventContext_{}",
        swift_cpp_event_token(plugin_id, interface, event)
    )
}

fn swift_cpp_event_delivery_name(
    plugin_id: &str,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    format!(
        "NexaCppEventDelivery_{}",
        swift_cpp_event_token(plugin_id, interface, event)
    )
}

fn swift_cpp_event_backing_name(interface: &BridgeInterface, event: &BridgeEvent) -> String {
    let mut occupied = std::collections::BTreeSet::new();
    occupied.extend(
        interface
            .properties
            .iter()
            .map(|property| property.name.clone()),
    );
    occupied.extend(interface.methods.iter().map(|method| method.name.clone()));
    occupied.extend(
        interface
            .events
            .iter()
            .map(|event| nexa_plugin_idl::event_callback_property(&event.name)),
    );
    let base = format!("_nexaCppEventContext{}", cpp_identifier(&event.name));
    let mut candidate = base.clone();
    let mut suffix = 1usize;
    while occupied.contains(&candidate) {
        candidate = format!("{base}{suffix}");
        suffix += 1;
    }
    candidate
}

pub(crate) fn swift_cpp_value_type(ty: &BridgeType) -> Option<String> {
    if let Some((key, value)) = swift_cpp_map_types(ty) {
        return Some(format!(
            "[{}: {}]",
            swift_cpp_base_type(key)?,
            swift_cpp_value_type(value)?
        ));
    }
    if matches!(ty, BridgeType::Set(_)) {
        let element = swift_cpp_set_element_type(ty)?;
        return Some(format!("Set<{element}>"));
    }
    if matches!(ty, BridgeType::Array(_)) {
        return swift_cpp_array_swift_type(ty);
    }
    let base = swift_cpp_value_base_type(ty)?;
    if ty.is_optional() {
        if base == "Void" {
            return None;
        }
        Some(format!("{base}?"))
    } else {
        Some(base.to_owned())
    }
}

pub(crate) fn swift_cpp_base_type(ty: &BridgeType) -> Option<&'static str> {
    if bridge_type_is_generic(ty) {
        return None;
    }
    match bridge_type_name(ty) {
        "Void" => Some("Void"),
        "Bool" => Some("Bool"),
        "Int8" => Some("Int8"),
        "Int16" => Some("Int16"),
        "Int32" => Some("Int32"),
        "Int64" => Some("Int64"),
        "UInt8" => Some("UInt8"),
        "UInt16" => Some("UInt16"),
        "UInt32" => Some("UInt32"),
        "UInt64" => Some("UInt64"),
        "Float32" => Some("Float"),
        "Float64" => Some("Double"),
        "String" => Some("String"),
        "Bytes" => Some("Data"),
        _ => None,
    }
}

pub(crate) fn swift_cpp_value_base_type(ty: &BridgeType) -> Option<String> {
    if let Some(base) = swift_cpp_base_type(ty) {
        return Some(base.to_owned());
    }
    if !bridge_type_is_generic(ty) {
        return Some(bridge_type_name(ty).to_owned());
    }
    None
}

pub(crate) fn swift_cpp_named_value_type<'a>(
    plan: &'a BridgePlan,
    ty: &BridgeType,
) -> Option<&'a BridgeNamedType> {
    if ty.is_optional() || bridge_type_is_generic(ty) {
        return None;
    }
    plan.types.iter().find(|named| {
        named.name == bridge_type_name(ty)
            && matches!(named.kind, BridgeTypeKind::Struct | BridgeTypeKind::Enum)
    })
}

pub(crate) fn swift_cpp_future_adapter_needed(method: &BridgeMethod) -> bool {
    method.is_async && swift_cpp_needs_vector_bridge(method.success_type())
}

fn swift_cpp_parameters(parameters: &[BridgeParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "_ {}: {}",
                parameter.name,
                swift_cpp_value_type(&parameter.ty).expect("validated parameter value")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_cpp_arguments(
    parameters: &[BridgeParameter],
    context: &SwiftAdapterContext<'_>,
) -> String {
    parameters
        .iter()
        .map(|parameter| swift_cpp_argument_expression(&parameter.ty, &parameter.name, context))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn swift_cpp_argument_value(
    ty: &BridgeType,
    value: &str,
    context: &SwiftAdapterContext<'_>,
) -> String {
    let SwiftAdapterContext {
        plan,
        namespace,
        byte_buffer_type,
        ..
    } = context;
    match bridge_type_name(ty) {
        "Map" if swift_cpp_map_types(ty).is_some() => {
            let (key, value_type) = swift_cpp_map_types(ty).expect("validated Swift C++ map value");
            let entry = format!("{namespace}.{}", cpp_swift_map_entry_name(plan, ty));
            let entries = format!("{namespace}.{}", cpp_swift_map_entries_name(plan, ty));
            let key_value = swift_cpp_map_value_argument(key, "$0.key", context);
            let mapped_value = swift_cpp_argument_value(value_type, "$0.value", context);
            format!("{entries}({value}.map {{ {entry}({key_value}, {mapped_value}) }})")
        }
        "String" => format!("std.string({value})"),
        "Bytes" => format!("{byte_buffer_type}({value})"),
        "Array" if swift_cpp_nested_array_supported(ty) => {
            swift_cpp_array_argument_expression(ty, value, context)
        }
        "Array" => {
            let BridgeType::Array(element) = bridge_strip_optional(ty) else {
                return value.to_owned();
            };
            let converted = match bridge_type_name(element) {
                "Bool" => format!("{value}.map {{ UInt8($0 ? 1 : 0) }}"),
                "String" => format!("{value}.map {{ std.string($0) }}"),
                "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
                _ if swift_cpp_named_value_type(plan, element).is_some() => format!(
                    "{value}.map {{ nexaSwiftToCpp{}($0) }}",
                    cpp_identifier(bridge_type_name(element))
                ),
                _ => value.to_owned(),
            };
            format!(
                "{namespace}.{}({converted})",
                cpp_swift_array_alias_name(plan, bridge_type_name(element))
            )
        }
        "Set" => {
            let BridgeType::Set(element) = bridge_strip_optional(ty) else {
                return value.to_owned();
            };
            let converted = match bridge_type_name(element) {
                "Bool" => format!("Array({value}).map {{ UInt8($0 ? 1 : 0) }}"),
                "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
                _ => format!("Array({value})"),
            };
            format!(
                "{namespace}.{}({converted})",
                cpp_swift_array_alias_name(plan, bridge_type_name(element))
            )
        }
        _ if swift_cpp_named_value_type(plan, ty).is_some() => {
            format!(
                "nexaSwiftToCpp{}({value})",
                cpp_identifier(bridge_type_name(ty))
            )
        }
        _ => value.to_owned(),
    }
}

fn render_swift_cpp_method(
    out: &mut SourceWriter,
    receiver: &str,
    adapter_method: Option<&str>,
    context: &SwiftAdapterContext<'_>,
    method: &BridgeMethod,
    depth: usize,
) {
    let indent = "    ".repeat(depth);
    let parameters = swift_cpp_parameters(&method.parameters);
    let arguments = swift_cpp_arguments(&method.parameters, context);
    let success_type = method.success_type();
    let return_type = swift_cpp_value_type(success_type).expect("validated method return type");
    let call_on_worker = method.is_async && adapter_method.is_some();
    let async_modifier = if method.is_async { " async" } else { "" };
    let throws_modifier = method
        .error_type()
        .map(|error| format!(" throws({error})"))
        .unwrap_or_default();
    out.push_str(&format!(
        "\n{indent}public func {}({parameters}){async_modifier}{throws_modifier} -> {return_type} {{\n",
        method.name
    ));
    let method_name = adapter_method.unwrap_or(&method.name);
    let mut call = format!("{receiver}.{}({arguments})", cpp_identifier(method_name));
    if call_on_worker && receiver == "nexaCppObject" {
        out.push_str(&format!(
            "{indent}    let nexaCppObjectForAsync = nexaCppObject\n"
        ));
        call = call.replacen("nexaCppObject", "nexaCppObjectForAsync", 1);
    }
    if let Some(error_name) = method.error_type() {
        render_swift_cpp_typed_error_call(
            out,
            &TypedErrorCall {
                method,
                success_type,
                return_type: &return_type,
                error_name,
                call: &call,
                call_on_worker,
            },
            context,
            depth,
        );
        out.push_str(&format!("{indent}}}\n"));
        return;
    }
    if method.is_async {
        if !call_on_worker {
            out.push_str(&format!("{indent}    var nexaCppFuture = {call}\n"));
        }
        let work_type = if return_type == "Void" {
            "Void"
        } else {
            return_type.as_str()
        };
        let work_body = if return_type == "Void" {
            if call_on_worker {
                format!("var nexaCppFuture = {call}; nexaCppFuture.get()")
            } else {
                "nexaCppFuture.get()".to_owned()
            }
        } else {
            let converted =
                swift_cpp_result_expression(success_type, "nexaCppFuture.get()", context);
            if call_on_worker {
                format!("var nexaCppFuture = {call}; {converted}")
            } else {
                converted
            }
        };
        out.push_str(&format!(
            "{indent}    let nexaCppFutureWork = NexaCppFutureWork<{work_type}> {{ {work_body} }}\n"
        ));
        if return_type == "Void" {
            out.push_str(&format!(
                "{indent}    await withCheckedContinuation {{ (continuation: CheckedContinuation<Void, Never>) in\n{indent}        DispatchQueue.global(qos: .userInitiated).async {{\n{indent}            nexaCppFutureWork.run()\n{indent}            continuation.resume()\n{indent}        }}\n{indent}    }}\n"
            ));
        } else {
            out.push_str(&format!(
                "{indent}    return await withCheckedContinuation {{ (continuation: CheckedContinuation<{return_type}, Never>) in\n{indent}        DispatchQueue.global(qos: .userInitiated).async {{\n{indent}            continuation.resume(returning: nexaCppFutureWork.run())\n{indent}        }}\n{indent}    }}\n"
            ));
        }
        out.push_str(&format!("{indent}}}\n"));
        return;
    }
    if return_type == "Void" {
        out.push_str(&format!("{indent}    {call}\n"));
    } else {
        let converted = swift_cpp_result_expression(success_type, &call, context);
        out.push_str(&format!("{indent}    return {converted}\n"));
    }
    out.push_str(&format!("{indent}}}\n"));
}

fn render_swift_cpp_named_value_helpers(out: &mut SourceWriter, context: &SwiftAdapterContext<'_>) {
    let SwiftAdapterContext {
        plan, namespace, ..
    } = context;
    for ty in collect_value_types(plan) {
        let name = cpp_identifier(&ty.name);
        let cxx_type = format!("{namespace}.{name}");
        match ty.kind {
            BridgeTypeKind::Enum => {
                out.push_str(&format!(
                    "private func nexaSwiftToCpp{name}(_ value: {name}) -> {cxx_type} {{\n    switch value {{\n"
                ));
                for case in &ty.cases {
                    let case_name = cpp_identifier(&case.name);
                    out.push_str(&format!(
                        "    case .{case_name}: return {cxx_type}.{case_name}\n"
                    ));
                }
                out.push_str("    }\n}\n");
                out.push_str(&format!(
                    "private func nexaSwiftFromCpp{name}(_ value: {cxx_type}) -> {name} {{\n    switch value {{\n"
                ));
                for case in &ty.cases {
                    let case_name = cpp_identifier(&case.name);
                    out.push_str(&format!("    case .{case_name}: return .{case_name}\n"));
                }
                out.push_str(&format!(
                    "    @unknown default: preconditionFailure(\"invalid C++ {name} case\")\n    }}\n}}\n\n"
                ));
            }
            BridgeTypeKind::Struct => {
                let swift_fields = ty
                    .fields
                    .iter()
                    .map(|field| {
                        let cpp_field = cpp_identifier(&field.name);
                        let converted = swift_cpp_result_expression(
                            &field.ty,
                            &format!("value.{cpp_field}"),
                            context,
                        );
                        format!("{}: {converted}", field.name)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!(
                    "private func nexaSwiftToCpp{name}(_ value: {name}) -> {cxx_type} {{\n    var result = {cxx_type}()\n"
                ));
                for field in &ty.fields {
                    let converted = swift_cpp_argument_expression(
                        &field.ty,
                        &format!("value.{}", field.name),
                        context,
                    );
                    out.push_str(&format!(
                        "    result.{} = {converted}\n",
                        cpp_identifier(&field.name)
                    ));
                }
                out.push_str("    return result\n}\n");
                out.push_str(&format!(
                    "private func nexaSwiftFromCpp{name}(_ value: {cxx_type}) -> {name} {{\n    {name}({swift_fields})\n}}\n\n"
                ));
            }
            BridgeTypeKind::Error => {
                // Excluded by the struct/enum filter above; error converters
                // render through the referenced-error lists instead.
            }
        }
    }
}
