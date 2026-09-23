//! C++ implementation contracts generated from the shared native IDL.
#![allow(clippy::all)]

use nexa_plugin_idl::{
    Event, Interface, InterfaceKind, Literal, Method, NamedType, NamedTypeKind, PluginIdl,
    Property, TypeRef,
};

pub fn render(idl: &PluginIdl, plugin_id: &str) -> String {
    let mut out = String::from(
        "#pragma once\n\n#include <atomic>\n#include <cstdint>\n#include <exception>\n#include <functional>\n#include <future>\n#include <map>\n#include <memory>\n#include <optional>\n#include <set>\n#include <string>\n#include <tuple>\n#include <utility>\n#include <variant>\n#include <vector>\n\n#if __has_include(<swift/bridging>)\n#include <swift/bridging>\n#define NEXA_CXX_SWIFT_SHARED_REFERENCE(...) SWIFT_SHARED_REFERENCE(__VA_ARGS__)\n#define NEXA_CXX_SWIFT_RETURNS_RETAINED SWIFT_RETURNS_RETAINED\n#define NEXA_CXX_SWIFT_NONNULL _Nonnull\n#define NEXA_CXX_SWIFT_NULLABLE _Nullable\n#else\n#define NEXA_CXX_SWIFT_SHARED_REFERENCE(...)\n#define NEXA_CXX_SWIFT_RETURNS_RETAINED\n#define NEXA_CXX_SWIFT_NONNULL\n#define NEXA_CXX_SWIFT_NULLABLE\n#endif\n\n",
    );
    out.push_str(&format!("namespace {} {{\n\n", cpp_namespace(plugin_id)));
    if idl.interfaces.iter().any(|interface| {
        interface.kind == InterfaceKind::NativeClass && !interface.events.is_empty()
    }) {
        out.push_str(
            "class NexaCppSwiftEventContextHandle final {\n    using Release = void (*)(void* NEXA_CXX_SWIFT_NONNULL);\n    struct State {\n        std::size_t references;\n        void* NEXA_CXX_SWIFT_NONNULL context;\n        Release release;\n        State(void* value, Release callback) : references(1), context(value), release(callback) {}\n    };\n    State* state_;\n    void retain() noexcept { if (state_ != nullptr) __atomic_add_fetch(&state_->references, std::size_t{1}, __ATOMIC_RELAXED); }\n    void relinquish() noexcept {\n        if (state_ != nullptr && __atomic_sub_fetch(&state_->references, std::size_t{1}, __ATOMIC_ACQ_REL) == 0) {\n            state_->release(state_->context);\n            delete state_;\n        }\n    }\npublic:\n    NexaCppSwiftEventContextHandle(void* NEXA_CXX_SWIFT_NONNULL context, Release release) : state_(new State(context, release)) {}\n    NexaCppSwiftEventContextHandle(const NexaCppSwiftEventContextHandle& other) noexcept : state_(other.state_) { retain(); }\n    NexaCppSwiftEventContextHandle(NexaCppSwiftEventContextHandle&& other) noexcept : state_(std::exchange(other.state_, nullptr)) {}\n    ~NexaCppSwiftEventContextHandle() { relinquish(); }\n    void* NEXA_CXX_SWIFT_NONNULL get() const noexcept { return state_->context; }\n};\n\n",
        );
    }
    out.push_str(&format!(
        "using {} = std::vector<std::uint8_t>;\n\n",
        cpp_byte_buffer_alias_name(idl)
    ));
    render_optional_bridge(&mut out, idl);
    render_swift_array_aliases(&mut out, idl);
    render_swift_map_adapters(&mut out, idl);
    render_result_type(&mut out);

    for ty in &idl.types {
        let declaration = match ty.kind {
            NamedTypeKind::Struct | NamedTypeKind::Error => {
                format!("struct {};\n", cpp_identifier(&ty.name))
            }
            NamedTypeKind::Enum => {
                format!("enum class {} : std::uint8_t;\n", cpp_identifier(&ty.name))
            }
        };
        out.push_str(&declaration);
    }
    for interface in &idl.interfaces {
        if matches!(
            interface.kind,
            InterfaceKind::Interface | InterfaceKind::NativeClass
        ) {
            out.push_str(&format!("class {}Spec;\n", cpp_identifier(&interface.name)));
        }
    }
    out.push('\n');

    for ty in &idl.types {
        render_named_type(&mut out, ty);
        out.push('\n');
    }
    render_cpp_swift_future_adapters(&mut out, idl);
    render_swift_cpp_error_bridges(&mut out, idl);
    for interface in &idl.interfaces {
        match interface.kind {
            InterfaceKind::Interface | InterfaceKind::NativeClass => {
                render_contract(&mut out, interface, idl);
                out.push('\n');
                if interface.kind == InterfaceKind::NativeClass {
                    render_factory(&mut out, interface);
                    out.push('\n');
                    render_swift_factory(&mut out, interface, idl);
                    out.push('\n');
                }
            }
            InterfaceKind::Service => {
                render_service(&mut out, interface);
                out.push('\n');
                for method in &interface.methods {
                    if swift_cpp_method_uses_collection_adapter(method) {
                        render_swift_service_collection_adapter(&mut out, idl, interface, method);
                    }
                }
                if interface
                    .methods
                    .iter()
                    .any(swift_cpp_method_uses_collection_adapter)
                {
                    out.push('\n');
                }
            }
            InterfaceKind::NativeComponent => {}
        }
    }
    out.push_str("} // namespace ");
    out.push_str(&cpp_namespace(plugin_id));
    out.push_str(
        "\n\n#undef NEXA_CXX_SWIFT_SHARED_REFERENCE\n#undef NEXA_CXX_SWIFT_RETURNS_RETAINED\n#undef NEXA_CXX_SWIFT_NONNULL\n#undef NEXA_CXX_SWIFT_NULLABLE\n",
    );
    out
}

/// Emits direct Swift adapters for supported C++ plugin values.
/// Unsupported signatures fail generation instead of producing a wrapper that
/// cannot honor the IDL.
pub fn render_swift_adapters(idl: &PluginIdl, plugin_id: &str) -> Result<String, String> {
    let namespace = cpp_namespace(plugin_id).replace("::", ".");
    let byte_buffer_type = format!("{namespace}.{}", cpp_byte_buffer_alias_name(idl));
    let optional_bridge = format!("{namespace}.{}", cpp_optional_bridge_name(idl));
    let mut out = String::from("import CxxStdlib\nimport Foundation\n\n");
    render_swift_cpp_named_value_helpers(&mut out, idl, &namespace, &byte_buffer_type);
    if idl
        .interfaces
        .iter()
        .flat_map(|interface| &interface.methods)
        .any(|method| method.is_async)
    {
        out.push_str(
            "private final class NexaCppFutureWork<Output>: @unchecked Sendable {\n    private let operation: () -> Output\n    init(_ operation: @escaping () -> Output) { self.operation = operation }\n    func run() -> Output { operation() }\n}\n\n",
        );
    }
    render_swift_cpp_error_converters(&mut out, idl, &namespace);
    for interface in idl
        .interfaces
        .iter()
        .filter(|interface| interface.kind == InterfaceKind::NativeClass)
    {
        for event in &interface.events {
            for parameter in &event.parameters {
                ensure_swift_cpp_value(&interface.name, &parameter.name, &parameter.ty, false)?;
            }
            render_swift_cpp_event_adapters(&mut out, idl, plugin_id, &namespace, interface, event);
        }
    }
    let mut generated = false;

    for interface in &idl.interfaces {
        match interface.kind {
            InterfaceKind::Service => {
                for method in &interface.methods {
                    validate_swift_cpp_method(idl, &interface.name, method)?;
                }
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
                        idl,
                        &namespace,
                        &byte_buffer_type,
                        &optional_bridge,
                        method,
                        1,
                    );
                }
                out.push_str("}\n\n");
            }
            InterfaceKind::NativeClass => {
                for constructor in &interface.constructors {
                    for parameter in &constructor.parameters {
                        ensure_swift_cpp_value(
                            &interface.name,
                            &parameter.name,
                            &parameter.ty,
                            false,
                        )?;
                    }
                }
                for property in &interface.properties {
                    ensure_swift_cpp_value(&interface.name, &property.name, &property.ty, false)?;
                }
                for method in &interface.methods {
                    validate_swift_cpp_method(idl, &interface.name, method)?;
                }
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
                    .map(|constructor| {
                        swift_cpp_arguments(
                            &constructor.parameters,
                            idl,
                            &namespace,
                            &byte_buffer_type,
                            &optional_bridge,
                        )
                    })
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
                    let getter =
                        swift_cpp_result_expression(&property.ty, &getter, idl, &namespace);
                    out.push_str(&format!(
                        "\n    public var {}: {ty} {{\n        get {{ {getter} }}",
                        property.name
                    ));
                    if property.mutable {
                        let setter_value = swift_cpp_argument_expression(
                            &property.ty,
                            "newValue",
                            idl,
                            &namespace,
                            &byte_buffer_type,
                            &optional_bridge,
                        );
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
                    render_swift_cpp_event_property(
                        &mut out, plugin_id, interface, event, &namespace,
                    );
                }
                for method in &interface.methods {
                    if method.name == "dispose"
                        && !interface.events.is_empty()
                        && method.parameters.is_empty()
                        && !method.is_async
                        && swift_cpp_method_error_type(method).is_none()
                        && swift_cpp_value_type(swift_cpp_method_success_type(method))
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
                            idl,
                            &namespace,
                            &byte_buffer_type,
                            &optional_bridge,
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
        Ok(out)
    } else {
        Ok(String::new())
    }
}

fn render_swift_cpp_event_adapters(
    out: &mut String,
    idl: &PluginIdl,
    plugin_id: &str,
    namespace: &str,
    interface: &Interface,
    event: &Event,
) {
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
            cpp_swift_event_copy_adapter_name(idl, interface, event, index)
        ));
        let swift_value = swift_cpp_result_expression(
            &parameter.ty,
            &format!("nexaCxxEventValue{index}"),
            idl,
            namespace,
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
    out: &mut String,
    plugin_id: &str,
    interface: &Interface,
    event: &Event,
    _namespace: &str,
) {
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

fn render_swift_cpp_dispose(out: &mut String, interface: &Interface, method: &Method) {
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

fn swift_cpp_event_callback_type(event: &Event) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| swift_cpp_value_type(&parameter.ty).expect("validated event type"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({parameters}) -> Void")
}

fn swift_cpp_event_token(plugin_id: &str, interface: &Interface, event: &Event) -> String {
    format!(
        "{}_{}_{}",
        cpp_identifier(plugin_id),
        cpp_identifier(&interface.name),
        cpp_identifier(&event.name)
    )
}

fn swift_cpp_event_context_name(plugin_id: &str, interface: &Interface, event: &Event) -> String {
    format!(
        "NexaCppEventContext_{}",
        swift_cpp_event_token(plugin_id, interface, event)
    )
}

fn swift_cpp_event_delivery_name(plugin_id: &str, interface: &Interface, event: &Event) -> String {
    format!(
        "NexaCppEventDelivery_{}",
        swift_cpp_event_token(plugin_id, interface, event)
    )
}

fn swift_cpp_event_backing_name(interface: &Interface, event: &Event) -> String {
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

/// Emits Kotlin wrappers and JNI declarations for the synchronous Android value
/// subset of a C++ plugin contract, including flat primitive arrays. Unsupported
/// IDL shapes fail generation instead of leaving declarations without a safe
/// native implementation.
pub fn render_android_adapters(
    idl: &PluginIdl,
    plugin_id: &str,
    plugin_namespace: &str,
    package: &str,
    plugin_index: usize,
) -> Result<(String, String), String> {
    let mut native_declarations = String::new();
    let mut kotlin_implementations = String::new();
    let mut kotlin_event_bridges = String::new();
    let mut jni = String::new();
    let class_name = format!("NexaPlugin{plugin_index}_CppBindings");
    let mut service_interfaces = Vec::new();
    let mut service_methods = std::collections::BTreeSet::new();
    let mut has_adapters = false;

    for interface in &idl.interfaces {
        match interface.kind {
            InterfaceKind::Service => {
                for method in &interface.methods {
                    validate_android_cpp_method(idl, &interface.name, method, false)?;
                    if !service_methods.insert(method.name.clone()) {
                        return Err(format!(
                            "Android C++ adapters do not support duplicate service method name `{}`",
                            method.name
                        ));
                    }
                    let native_name = format!("service_{}_{}", interface.name, method.name);
                    render_kotlin_native_declaration(
                        &mut native_declarations,
                        method,
                        &native_name,
                    );
                    render_jni_service_method(
                        &mut jni,
                        package,
                        &class_name,
                        &interface.name,
                        method,
                        &native_name,
                    );
                }
                service_interfaces.push(interface.name.clone());
                has_adapters = true;
            }
            InterfaceKind::NativeClass => {
                if interface.constructors.len() > 1 {
                    return Err(format!(
                        "Android C++ adapters currently support one constructor for `{}`",
                        interface.name
                    ));
                }
                let constructor = interface.constructors.first();
                if let Some(constructor) = constructor {
                    for parameter in &constructor.parameters {
                        ensure_android_cpp_value(
                            idl,
                            &interface.name,
                            &parameter.name,
                            &parameter.ty,
                            false,
                        )?;
                    }
                }
                for event in &interface.events {
                    for parameter in &event.parameters {
                        ensure_android_cpp_value(
                            idl,
                            &interface.name,
                            &parameter.name,
                            &parameter.ty,
                            false,
                        )?;
                    }
                    let setter = format!(
                        "set_{}_{}",
                        interface.name,
                        nexa_plugin_idl::event_callback_property(&event.name)
                    );
                    render_kotlin_native_event_declaration(&mut native_declarations, &setter);
                    render_jni_event_setter(
                        &mut jni,
                        plugin_id,
                        package,
                        &class_name,
                        interface,
                        event,
                        &setter,
                    );
                    render_kotlin_event_bridge(
                        &mut kotlin_event_bridges,
                        interface,
                        event,
                        plugin_index,
                    );
                }
                for property in &interface.properties {
                    ensure_android_cpp_value(
                        idl,
                        &interface.name,
                        &property.name,
                        &property.ty,
                        false,
                    )?;
                }
                let dispose = interface
                    .methods
                    .iter()
                    .find(|method| method.name == "dispose");
                let Some(dispose) = dispose else {
                    return Err(format!(
                        "Android C++ native class `{}` must declare `fn dispose()` for deterministic native ownership",
                        interface.name
                    ));
                };
                validate_android_cpp_method(idl, &interface.name, dispose, true)?;
                for method in &interface.methods {
                    validate_android_cpp_method(
                        idl,
                        &interface.name,
                        method,
                        method.name == "dispose",
                    )?;
                }

                let create_name = format!("create_{}", interface.name);
                render_kotlin_native_declaration(
                    &mut native_declarations,
                    &Method {
                        name: create_name.clone(),
                        parameters: constructor
                            .map(|constructor| constructor.parameters.clone())
                            .unwrap_or_default(),
                        return_type: TypeRef {
                            name: "Int64".to_owned(),
                            optional: false,
                            arguments: Vec::new(),
                        },
                        is_async: false,
                        throws: None,
                    },
                    &create_name,
                );
                render_jni_constructor(
                    &mut jni,
                    plugin_id,
                    package,
                    &class_name,
                    interface,
                    &create_name,
                );

                for property in &interface.properties {
                    let getter_name = format!("get_{}_{}", interface.name, property.name);
                    render_kotlin_property_native_declarations(
                        &mut native_declarations,
                        property,
                        &interface.name,
                        &getter_name,
                    );
                    render_jni_property(
                        &mut jni,
                        plugin_id,
                        package,
                        &class_name,
                        interface,
                        property,
                        &getter_name,
                    );
                }
                for method in &interface.methods {
                    if method.name == "dispose" {
                        let native_name = format!("dispose_{}", interface.name);
                        render_kotlin_native_dispose_declaration(
                            &mut native_declarations,
                            &native_name,
                        );
                        render_jni_dispose(
                            &mut jni,
                            plugin_id,
                            package,
                            &class_name,
                            interface,
                            &native_name,
                        );
                    } else {
                        let native_name = format!("call_{}_{}", interface.name, method.name);
                        render_kotlin_class_native_declaration(
                            &mut native_declarations,
                            method,
                            &native_name,
                        );
                        render_jni_class_method(
                            &mut jni,
                            plugin_id,
                            package,
                            &class_name,
                            interface,
                            method,
                            &native_name,
                        );
                    }
                }
                render_kotlin_cpp_class(
                    &mut kotlin_implementations,
                    interface,
                    plugin_index,
                    constructor,
                );
                has_adapters = true;
            }
            InterfaceKind::Interface | InterfaceKind::NativeComponent => {}
        }
    }

    if !has_adapters {
        return Ok((String::new(), String::new()));
    }

    let mut kotlin_output = format!(
        "\ninternal object {class_name} {{\n    init {{ System.loadLibrary(\"nexa_plugins\") }}\n"
    );
    kotlin_output.push_str(&native_declarations);
    kotlin_output.push_str("}\n\n");

    render_android_error_factory(&mut kotlin_output, idl, plugin_index);

    if !service_interfaces.is_empty() {
        let contracts = service_interfaces
            .iter()
            .map(|name| name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        kotlin_output.push_str(&format!(
            "public object {plugin_namespace}Plugin : {contracts} {{\n    public val instance: {plugin_namespace}Plugin get() = this\n"
        ));
        for interface in idl
            .interfaces
            .iter()
            .filter(|interface| interface.kind == InterfaceKind::Service)
        {
            for method in &interface.methods {
                render_kotlin_service_adapter(
                    &mut kotlin_output,
                    method,
                    &format!("service_{}_{}", interface.name, method.name),
                    &class_name,
                );
            }
        }
        kotlin_output.push_str("}\n\n");
    }

    if !kotlin_event_bridges.is_empty() {
        kotlin_output.push_str(&format!(
            "private object NexaPlugin{plugin_index}_CppEventDispatcher {{\n    private val mainHandler = android.os.Handler(android.os.Looper.getMainLooper())\n    fun post(operation: () -> Unit) {{ mainHandler.post(operation) }}\n}}\n\n"
        ));
        kotlin_output.push_str(&kotlin_event_bridges);
    }
    kotlin_output.push_str(&kotlin_implementations);
    let mut jni_output = render_android_jni_prelude(plugin_id);
    render_android_jni_error_converters(&mut jni_output, idl, package, plugin_index);
    render_android_jni_named_value_helpers(&mut jni_output, idl, package);
    jni_output.push_str(&jni);
    Ok((kotlin_output, jni_output))
}

fn render_android_jni_prelude(plugin_id: &str) -> String {
    const PRELUDE: &str = r#"#include <jni.h>
#include <algorithm>
#include <array>
#include <bit>
#include <cstdint>
#include <exception>
#include <limits>
#include <memory>
#include <new>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include "NexaPluginBindings.hpp"

namespace {
using namespace __NEXA_CPP_NAMESPACE__;

void throwIllegalState(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) return;
    jclass exception = env->FindClass("java/lang/IllegalStateException");
    if (exception != nullptr) env->ThrowNew(exception, message);
}

void throwRuntime(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) return;
    jclass exception = env->FindClass("java/lang/RuntimeException");
    if (exception != nullptr) env->ThrowNew(exception, message);
}

template <class Function>
auto withJniExceptions(JNIEnv* env, Function&& function) noexcept -> decltype(function()) {
    using Return = decltype(function());
    try {
        if constexpr (std::is_void_v<Return>) {
            function();
            return;
        } else {
            return function();
        }
    } catch (const std::exception& error) {
        throwRuntime(env, error.what());
    } catch (...) {
        throwRuntime(env, "native plugin operation failed");
    }
    if constexpr (std::is_void_v<Return>) return;
    else return Return{};
}

void appendUtf8(std::string& output, std::uint32_t codepoint) {
    if (codepoint <= 0x7f) output.push_back(static_cast<char>(codepoint));
    else if (codepoint <= 0x7ff) {
        output.push_back(static_cast<char>(0xc0 | (codepoint >> 6)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    } else if (codepoint <= 0xffff) {
        output.push_back(static_cast<char>(0xe0 | (codepoint >> 12)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    } else {
        output.push_back(static_cast<char>(0xf0 | (codepoint >> 18)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 12) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3f)));
        output.push_back(static_cast<char>(0x80 | (codepoint & 0x3f)));
    }
}

std::string fromJniString(JNIEnv* env, jstring value) {
    if (value == nullptr) {
        throwIllegalState(env, "non-null Nexa string was null at the JNI boundary");
        return {};
    }
    const jsize length = env->GetStringLength(value);
    const jchar* characters = env->GetStringChars(value, nullptr);
    if (characters == nullptr) return {};
    struct ReleaseStringChars {
        JNIEnv* env;
        jstring value;
        const jchar* characters;
        ~ReleaseStringChars() { env->ReleaseStringChars(value, characters); }
    } release{env, value, characters};

    std::string output;
    for (jsize index = 0; index < length; ++index) {
        std::uint32_t codepoint = characters[index];
        if (codepoint >= 0xd800 && codepoint <= 0xdbff) {
            if (index + 1 < length && characters[index + 1] >= 0xdc00 && characters[index + 1] <= 0xdfff) {
                codepoint = 0x10000 + ((codepoint - 0xd800) << 10) + (characters[++index] - 0xdc00);
            } else {
                codepoint = 0xfffd;
            }
        } else if (codepoint >= 0xdc00 && codepoint <= 0xdfff) {
            codepoint = 0xfffd;
        }
        appendUtf8(output, codepoint);
    }
    return output;
}

std::vector<std::uint8_t> fromJniBytes(JNIEnv* env, jbyteArray value) {
    if (value == nullptr) {
        throwIllegalState(env, "non-null Nexa byte array was null at the JNI boundary");
        return {};
    }
    const jsize length = env->GetArrayLength(value);
    std::vector<std::uint8_t> output(static_cast<std::size_t>(length));
    if (length > 0) {
        env->GetByteArrayRegion(value, 0, length, reinterpret_cast<jbyte*>(output.data()));
    }
    return output;
}

jbyteArray toJniBytes(JNIEnv* env, const std::vector<std::uint8_t>& value) {
    if (value.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) {
        throw std::length_error("native plugin byte array is too large for JNI");
    }
    const auto length = static_cast<jsize>(value.size());
    jbyteArray output = env->NewByteArray(length);
    if (output == nullptr || length == 0) return output;
    env->SetByteArrayRegion(output, 0, length, reinterpret_cast<const jbyte*>(value.data()));
    return output;
}

std::uint32_t decodeUtf8(const std::string& value, std::size_t& index) {
    const auto first = static_cast<std::uint8_t>(value[index]);
    if (first < 0x80) {
        ++index;
        return first;
    }
    const std::size_t count = first >= 0xf0 ? 4 : first >= 0xe0 ? 3 : first >= 0xc2 ? 2 : 0;
    if (count == 0 || index + count > value.size()) {
        ++index;
        return 0xfffd;
    }
    std::uint32_t codepoint = first & (count == 2 ? 0x1f : count == 3 ? 0x0f : 0x07);
    for (std::size_t offset = 1; offset < count; ++offset) {
        const auto byte = static_cast<std::uint8_t>(value[index + offset]);
        if ((byte & 0xc0) != 0x80) {
            ++index;
            return 0xfffd;
        }
        codepoint = (codepoint << 6) | (byte & 0x3f);
    }
    const bool overlong = (count == 2 && codepoint < 0x80) ||
        (count == 3 && codepoint < 0x800) ||
        (count == 4 && codepoint < 0x10000);
    if (overlong || codepoint > 0x10ffff || (codepoint >= 0xd800 && codepoint <= 0xdfff)) {
        ++index;
        return 0xfffd;
    }
    index += count;
    return codepoint;
}

jstring toJniString(JNIEnv* env, const std::string& value) {
    std::vector<jchar> output;
    output.reserve(value.size());
    for (std::size_t index = 0; index < value.size();) {
        const auto codepoint = decodeUtf8(value, index);
        if (codepoint <= 0xffff) {
            output.push_back(static_cast<jchar>(codepoint));
        } else {
            const auto adjusted = codepoint - 0x10000;
            output.push_back(static_cast<jchar>(0xd800 + (adjusted >> 10)));
            output.push_back(static_cast<jchar>(0xdc00 + (adjusted & 0x3ff)));
        }
    }
    if (output.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) {
        throw std::length_error("native plugin string is too large for JNI");
    }
    const jchar empty = 0;
    return env->NewString(output.empty() ? &empty : output.data(), static_cast<jsize>(output.size()));
}

template <class T>
std::unique_ptr<T>* requireHandle(JNIEnv* env, jlong rawHandle) {
    if (rawHandle == 0) {
        throwIllegalState(env, "native plugin object is disposed");
        return nullptr;
    }
    auto* handle = reinterpret_cast<std::unique_ptr<T>*>(static_cast<std::uintptr_t>(rawHandle));
    if (handle == nullptr || !*handle) {
        throwIllegalState(env, "native plugin object handle is invalid");
        return nullptr;
    }
    return handle;
}

template <class T>
class ScopedLocalRef {
public:
    ScopedLocalRef(JNIEnv* env, T value) : env_(env), value_(value) {}
    ScopedLocalRef(const ScopedLocalRef&) = delete;
    ScopedLocalRef& operator=(const ScopedLocalRef&) = delete;
    ~ScopedLocalRef() { if (value_ != nullptr) env_->DeleteLocalRef(value_); }
    T get() const noexcept { return value_; }
    T release() noexcept { T value = value_; value_ = nullptr; return value; }
private:
    JNIEnv* env_;
    T value_;
};

class ScopedJniEnvironment {
public:
    explicit ScopedJniEnvironment(JavaVM* vm) noexcept : vm_(vm) {
        if (vm_ == nullptr) return;
        void* rawEnvironment = nullptr;
        const jint status = vm_->GetEnv(&rawEnvironment, JNI_VERSION_1_6);
        if (status == JNI_OK) {
            environment_ = static_cast<JNIEnv*>(rawEnvironment);
        } else if (status == JNI_EDETACHED && attachCurrentThread() == JNI_OK) {
            attached_ = true;
        }
    }
    ScopedJniEnvironment(const ScopedJniEnvironment&) = delete;
    ScopedJniEnvironment& operator=(const ScopedJniEnvironment&) = delete;
    ~ScopedJniEnvironment() {
        if (attached_) vm_->DetachCurrentThread();
    }
    JNIEnv* get() const noexcept { return environment_; }
private:
    jint attachCurrentThread() noexcept {
#if defined(__ANDROID__)
        return vm_->AttachCurrentThread(&environment_, nullptr);
#else
        return vm_->AttachCurrentThread(reinterpret_cast<void**>(&environment_), nullptr);
#endif
    }
    JavaVM* vm_{};
    JNIEnv* environment_{};
    bool attached_{};
};

class JniGlobalReference {
public:
    JniGlobalReference(JNIEnv* env, jobject value) {
        if (env == nullptr || value == nullptr || env->GetJavaVM(&vm_) != JNI_OK) return;
        value_ = env->NewGlobalRef(value);
    }
    JniGlobalReference(const JniGlobalReference&) = delete;
    JniGlobalReference& operator=(const JniGlobalReference&) = delete;
    ~JniGlobalReference() {
        if (value_ == nullptr) return;
        ScopedJniEnvironment scope(vm_);
        if (auto* env = scope.get()) env->DeleteGlobalRef(value_);
    }
    JavaVM* vm() const noexcept { return vm_; }
    jobject get() const noexcept { return value_; }
private:
    JavaVM* vm_{};
    jobject value_{};
};

class ScopedJniLocalFrame {
public:
    ScopedJniLocalFrame(JNIEnv* env, jint capacity) noexcept
        : env_(env), active_(env_ != nullptr && env_->PushLocalFrame(capacity) == JNI_OK) {}
    ScopedJniLocalFrame(const ScopedJniLocalFrame&) = delete;
    ScopedJniLocalFrame& operator=(const ScopedJniLocalFrame&) = delete;
    ~ScopedJniLocalFrame() { if (active_) env_->PopLocalFrame(nullptr); }
    bool active() const noexcept { return active_; }
private:
    JNIEnv* env_{};
    bool active_{};
};

void requireJniMapOperation(JNIEnv* env, const char* message) {
    if (env->ExceptionCheck()) throw std::runtime_error(message);
}

template <class T>
inline constexpr bool isOptionalMapValue = false;

template <class T>
inline constexpr bool isOptionalMapValue<std::optional<T>> = true;

template <class Map, class KeyConverter, class ValueConverter>
Map fromJniMap(JNIEnv* env, jobject input, KeyConverter&& convertKey, ValueConverter&& convertValue) {
    if (input == nullptr) {
        throwIllegalState(env, "non-null Nexa map was null at the JNI boundary");
        throw std::runtime_error("null JNI map");
    }
    ScopedLocalRef<jclass> mapClass(env, env->FindClass("java/util/Map"));
    requireJniMapOperation(env, "unable to resolve java.util.Map");
    jmethodID entrySetMethod = env->GetMethodID(mapClass.get(), "entrySet", "()Ljava/util/Set;");
    requireJniMapOperation(env, "unable to resolve Map.entrySet");
    ScopedLocalRef<jobject> entries(env, env->CallObjectMethod(input, entrySetMethod));
    requireJniMapOperation(env, "unable to read JNI map entries");
    ScopedLocalRef<jclass> setClass(env, env->FindClass("java/util/Set"));
    requireJniMapOperation(env, "unable to resolve java.util.Set");
    jmethodID iteratorMethod = env->GetMethodID(setClass.get(), "iterator", "()Ljava/util/Iterator;");
    requireJniMapOperation(env, "unable to resolve Set.iterator");
    ScopedLocalRef<jobject> iterator(env, env->CallObjectMethod(entries.get(), iteratorMethod));
    requireJniMapOperation(env, "unable to iterate JNI map entries");
    ScopedLocalRef<jclass> iteratorClass(env, env->FindClass("java/util/Iterator"));
    requireJniMapOperation(env, "unable to resolve java.util.Iterator");
    jmethodID hasNextMethod = env->GetMethodID(iteratorClass.get(), "hasNext", "()Z");
    jmethodID nextMethod = env->GetMethodID(iteratorClass.get(), "next", "()Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve Iterator methods");
    ScopedLocalRef<jclass> entryClass(env, env->FindClass("java/util/Map$Entry"));
    requireJniMapOperation(env, "unable to resolve java.util.Map.Entry");
    jmethodID keyMethod = env->GetMethodID(entryClass.get(), "getKey", "()Ljava/lang/Object;");
    jmethodID valueMethod = env->GetMethodID(entryClass.get(), "getValue", "()Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve Map.Entry methods");

    Map output;
    while (env->CallBooleanMethod(iterator.get(), hasNextMethod) == JNI_TRUE) {
        requireJniMapOperation(env, "unable to advance JNI map iterator");
        ScopedLocalRef<jobject> entry(env, env->CallObjectMethod(iterator.get(), nextMethod));
        requireJniMapOperation(env, "unable to read JNI map entry");
        ScopedLocalRef<jobject> key(env, env->CallObjectMethod(entry.get(), keyMethod));
        requireJniMapOperation(env, "unable to read JNI map key");
        ScopedLocalRef<jobject> value(env, env->CallObjectMethod(entry.get(), valueMethod));
        requireJniMapOperation(env, "unable to read JNI map value");
        if (key.get() == nullptr) {
            throwIllegalState(env, "non-null Nexa map key was null at the JNI boundary");
            throw std::runtime_error("null JNI map key");
        }
        auto convertedKey = convertKey(key.get());
        requireJniMapOperation(env, "unable to convert JNI map key");
        if (value.get() == nullptr) {
            if constexpr (isOptionalMapValue<std::invoke_result_t<ValueConverter, jobject>>) {
                auto convertedValue = convertValue(nullptr);
                requireJniMapOperation(env, "unable to convert nullable JNI map value");
                output.insert_or_assign(std::move(convertedKey), std::move(convertedValue));
                continue;
            } else {
                throwIllegalState(env, "non-null Nexa map value was null at the JNI boundary");
                throw std::runtime_error("null JNI map value");
            }
        }
        auto convertedValue = convertValue(value.get());
        requireJniMapOperation(env, "unable to convert JNI map value");
        output.insert_or_assign(std::move(convertedKey), std::move(convertedValue));
    }
    requireJniMapOperation(env, "unable to finish reading JNI map");
    return output;
}

template <class Map, class KeyConverter, class ValueConverter>
jobject toJniMap(JNIEnv* env, const Map& input, KeyConverter&& convertKey, ValueConverter&& convertValue) {
    const auto maxCapacity = static_cast<std::size_t>(std::numeric_limits<jint>::max());
    if (input.size() > maxCapacity) {
        throw std::length_error("native plugin map is too large for JNI");
    }
    const auto expectedSize = input.size();
    const auto extraCapacity = (expectedSize + 2) / 3;
    const auto initialCapacity = std::min(maxCapacity, expectedSize + extraCapacity);
    ScopedLocalRef<jclass> mapClass(env, env->FindClass("java/util/HashMap"));
    requireJniMapOperation(env, "unable to resolve java.util.HashMap");
    jmethodID constructor = env->GetMethodID(mapClass.get(), "<init>", "(I)V");
    jmethodID putMethod = env->GetMethodID(mapClass.get(), "put", "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;");
    requireJniMapOperation(env, "unable to resolve HashMap methods");
    ScopedLocalRef<jobject> output(env, env->NewObject(mapClass.get(), constructor, static_cast<jint>(initialCapacity)));
    requireJniMapOperation(env, "unable to create JNI map");
    for (const auto& item : input) {
        ScopedLocalRef<jobject> key(env, convertKey(item.first));
        requireJniMapOperation(env, "unable to convert native map key");
        ScopedLocalRef<jobject> value(env, convertValue(item.second));
        requireJniMapOperation(env, "unable to convert native map value");
        ScopedLocalRef<jobject> previous(env, env->CallObjectMethod(output.get(), putMethod, key.get(), value.get()));
        requireJniMapOperation(env, "unable to populate JNI map");
    }
    return output.release();
}
} // namespace
"#;
    PRELUDE.replace("__NEXA_CPP_NAMESPACE__", &cpp_namespace(plugin_id))
}

fn validate_android_cpp_method(
    idl: &PluginIdl,
    interface: &str,
    method: &Method,
    is_dispose: bool,
) -> Result<(), String> {
    if let Some(error_type) = android_cpp_method_error_type(method) {
        let Some(error) = idl
            .types
            .iter()
            .find(|ty| ty.kind == NamedTypeKind::Error && ty.name == error_type.name)
        else {
            return Err(format!(
                "Android C++ adapters cannot resolve typed error `{}` for `{interface}.{}`",
                error_type.name, method.name
            ));
        };
        for case in &error.cases {
            for parameter in &case.parameters {
                if parameter.ty.optional
                    || !parameter.ty.arguments.is_empty()
                    || parameter.ty.name == "Void"
                    || android_cpp_value(&parameter.ty).is_none()
                {
                    return Err(format!(
                        "Android C++ typed errors support non-optional primitive, `String`, and `Bytes` payloads; `{interface}.{}` error case `{}.{}` uses `{}`",
                        method.name, error.name, case.name, parameter.ty.name
                    ));
                }
            }
        }
    }
    let success_type = android_cpp_method_success_type(method);
    for parameter in &method.parameters {
        ensure_android_cpp_value(idl, interface, &parameter.name, &parameter.ty, false)?;
    }
    if is_dispose {
        if !method.parameters.is_empty()
            || method.is_async
            || method.return_type.name != "Void"
            || method.return_type.optional
        {
            return Err(format!(
                "Android C++ native class disposal must be a synchronous parameterless `fn dispose()` on `{interface}`"
            ));
        }
    } else {
        ensure_android_cpp_value(idl, interface, &method.name, success_type, true)?;
    }
    Ok(())
}

fn android_cpp_method_error_type(method: &Method) -> Option<&TypeRef> {
    method.throws.as_ref().or_else(|| {
        (method.return_type.name == "Result")
            .then(|| method.return_type.arguments.get(1))
            .flatten()
    })
}

fn android_cpp_method_success_type(method: &Method) -> &TypeRef {
    if method.return_type.name == "Result" {
        method
            .return_type
            .arguments
            .first()
            .expect("validated Result type has a success type")
    } else {
        &method.return_type
    }
}

fn ensure_android_cpp_value(
    idl: &PluginIdl,
    interface: &str,
    member: &str,
    ty: &TypeRef,
    allow_void: bool,
) -> Result<(), String> {
    let named_scalar = android_cpp_named_value_type(idl, ty).is_some();
    let collections_with_named_values = !named_scalar && android_cpp_contains_named_value(idl, ty);
    let requires_named_declaration = ty.arguments.is_empty()
        && android_cpp_value(ty).is_none()
        && !matches!(
            ty.name.as_str(),
            "Array" | "Set" | "Map" | "Pair" | "Triple" | "Result"
        );
    let named_type_is_supported = !requires_named_declaration
        || android_cpp_named_value_type(idl, ty)
            .is_some_and(|named| !ty.optional && android_cpp_named_value_supported(idl, named));
    if android_cpp_type(ty).is_some()
        && named_type_is_supported
        && !collections_with_named_values
        && (allow_void || !android_cpp_is_void(ty))
    {
        return Ok(());
    }
    Err(format!(
        "Android C++ adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, flat primitive/string `Map` values, and maps with array or compatible set values; `{interface}.{member}` uses unsupported type `{}`",
        ty.name
    ))
}

fn android_cpp_named_value_type<'a>(idl: &'a PluginIdl, ty: &TypeRef) -> Option<&'a NamedType> {
    if !ty.arguments.is_empty() {
        return None;
    }
    idl.types.iter().find(|named| {
        named.name == ty.name && matches!(named.kind, NamedTypeKind::Struct | NamedTypeKind::Enum)
    })
}

fn android_cpp_named_value_supported(idl: &PluginIdl, ty: &NamedType) -> bool {
    match ty.kind {
        NamedTypeKind::Enum => !ty.cases.is_empty() && ty.cases.len() <= 256,
        NamedTypeKind::Struct => ty.fields.iter().all(|field| {
            let field_type = &field.ty;
            if android_cpp_value(field_type).is_some_and(|value| !value.optional) {
                return true;
            }
            !field_type.optional
                && android_cpp_named_value_type(idl, field_type)
                    .is_some_and(|nested| android_cpp_named_value_supported(idl, nested))
        }),
        NamedTypeKind::Error => false,
    }
}

fn android_cpp_contains_named_value(idl: &PluginIdl, ty: &TypeRef) -> bool {
    if android_cpp_named_value_type(idl, ty).is_some() {
        return true;
    }
    ty.arguments
        .iter()
        .any(|argument| android_cpp_contains_named_value(idl, argument))
}

#[derive(Clone, Copy)]
struct AndroidValue {
    kotlin: &'static str,
    jni_kotlin: &'static str,
    jni: &'static str,
    cpp: &'static str,
    kotlin_to_jni: Option<&'static str>,
    kotlin_from_jni: Option<&'static str>,
    unsigned: bool,
    optional: bool,
}

fn android_cpp_value(ty: &TypeRef) -> Option<AndroidValue> {
    if !ty.arguments.is_empty() {
        return None;
    }
    let (kotlin, jni_kotlin, jni, cpp, kotlin_to_jni, kotlin_from_jni, unsigned) =
        match ty.name.as_str() {
            "Void" => ("Unit", "Unit", "void", "void", None, None, false),
            "Bool" => ("Boolean", "Boolean", "jboolean", "bool", None, None, false),
            "Int8" => ("Byte", "Byte", "jbyte", "std::int8_t", None, None, false),
            "Int16" => (
                "Short",
                "Short",
                "jshort",
                "std::int16_t",
                None,
                None,
                false,
            ),
            "Int32" => ("Int", "Int", "jint", "std::int32_t", None, None, false),
            "Int64" => ("Long", "Long", "jlong", "std::int64_t", None, None, false),
            "UInt8" => (
                "UByte",
                "Byte",
                "jbyte",
                "std::uint8_t",
                Some("toByte"),
                Some("toUByte"),
                true,
            ),
            "UInt16" => (
                "UShort",
                "Short",
                "jshort",
                "std::uint16_t",
                Some("toShort"),
                Some("toUShort"),
                true,
            ),
            "UInt32" => (
                "UInt",
                "Int",
                "jint",
                "std::uint32_t",
                Some("toInt"),
                Some("toUInt"),
                true,
            ),
            "UInt64" => (
                "ULong",
                "Long",
                "jlong",
                "std::uint64_t",
                Some("toLong"),
                Some("toULong"),
                true,
            ),
            "Float32" => ("Float", "Float", "jfloat", "float", None, None, false),
            "Float64" => ("Double", "Double", "jdouble", "double", None, None, false),
            "String" => (
                "String",
                "String",
                "jstring",
                "std::string",
                None,
                None,
                false,
            ),
            "Bytes" => (
                "ByteArray",
                "ByteArray",
                "jbyteArray",
                "std::vector<std::uint8_t>",
                None,
                None,
                false,
            ),
            _ => return None,
        };
    if ty.optional && kotlin == "Unit" {
        return None;
    }
    let jni_kotlin = if ty.optional && unsigned {
        "Long"
    } else {
        jni_kotlin
    };
    Some(AndroidValue {
        kotlin,
        jni_kotlin,
        jni: if ty.optional { "jobject" } else { jni },
        cpp,
        kotlin_to_jni,
        kotlin_from_jni,
        unsigned,
        optional: ty.optional,
    })
}

fn android_cpp_type(ty: &TypeRef) -> Option<String> {
    if ty.name == "Map" {
        let [key, value] = ty.arguments.as_slice() else {
            return None;
        };
        if !android_map_is_supported(ty)
            || !android_map_element_is_supported(key, true)
            || !android_map_value_is_supported(value)
        {
            return None;
        }
        let map_type = format!(
            "std::map<{}, {}>",
            android_cpp_type(key)?,
            android_cpp_type(value)?
        );
        return Some(if ty.optional {
            format!("std::optional<{map_type}>")
        } else {
            map_type
        });
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let [element] = ty.arguments.as_slice() else {
            return None;
        };
        if ty.name == "Set" && !android_set_element_is_supported(element) {
            return None;
        }
        if ty.optional {
            return None;
        }
        if ty.name == "Set" {
            if android_primitive_array(element).is_none()
                && android_reference_array_element(element).is_none()
                && !element.optional
            {
                return None;
            }
            return Some(format!("std::set<{}>", android_cpp_type(element)?));
        }
        return Some(format!("std::vector<{}>", android_cpp_type(element)?));
    }
    if let Some(value) = android_cpp_value(ty) {
        return Some(if ty.optional {
            format!("std::optional<{}>", value.cpp)
        } else {
            value.cpp.to_owned()
        });
    }
    (ty.arguments.is_empty() && !matches!(ty.name.as_str(), "Pair" | "Triple" | "Result"))
        .then(|| cpp_type(ty))
}

fn android_set_element_is_supported(ty: &TypeRef) -> bool {
    matches!(
        ty.name.as_str(),
        "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "String"
    ) && ty.arguments.is_empty()
}

fn android_map_element_is_supported(ty: &TypeRef, is_key: bool) -> bool {
    (!is_key || !ty.optional)
        && ty.arguments.is_empty()
        && matches!(
            ty.name.as_str(),
            "Bool"
                | "Int8"
                | "Int16"
                | "Int32"
                | "Int64"
                | "UInt8"
                | "UInt16"
                | "UInt32"
                | "UInt64"
                | "Float32"
                | "Float64"
                | "String"
        )
        && (!is_key || !matches!(ty.name.as_str(), "Float32" | "Float64"))
}

fn android_map_value_is_supported(ty: &TypeRef) -> bool {
    if ty.name == "Map" {
        return android_map_is_supported(ty);
    }
    if android_map_element_is_supported(ty, false)
        || (ty.name == "Bytes" && ty.arguments.is_empty())
    {
        return true;
    }
    if ty.optional || !matches!(ty.name.as_str(), "Array" | "Set") {
        return false;
    }
    let [element] = ty.arguments.as_slice() else {
        return false;
    };
    if ty.name == "Set" {
        return android_set_element_is_supported(element);
    }
    android_map_value_is_supported(element)
}

fn android_cpp_map_type(ty: &TypeRef) -> Option<(&TypeRef, &TypeRef)> {
    android_map_is_supported(ty).then(|| (&ty.arguments[0], &ty.arguments[1]))
}

fn android_map_is_supported(ty: &TypeRef) -> bool {
    if ty.name != "Map" {
        return false;
    }
    let [key, value] = ty.arguments.as_slice() else {
        return false;
    };
    android_map_element_is_supported(key, true) && android_map_value_is_supported(value)
}

fn android_cpp_is_void(ty: &TypeRef) -> bool {
    ty.name == "Void" && !ty.optional && ty.arguments.is_empty()
}

fn android_jni_type(ty: &TypeRef) -> Option<String> {
    if android_cpp_map_type(ty).is_some() {
        return Some("jobject".to_owned());
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let [element] = ty.arguments.as_slice() else {
            return None;
        };
        if ty.optional {
            return None;
        }
        if let Some(array) = android_primitive_array(element) {
            return Some(array.jni_array.to_owned());
        }
        if !android_jni_reference_class_available(element) {
            return None;
        }
        return Some("jobjectArray".to_owned());
    }
    android_cpp_value(ty)
        .map(|value| value.jni.to_owned())
        .or_else(|| {
            (ty.arguments.is_empty() && !matches!(ty.name.as_str(), "Pair" | "Triple" | "Result"))
                .then(|| "jobject".to_owned())
        })
}

fn android_reference_array_element(ty: &TypeRef) -> Option<AndroidValue> {
    if ty.optional || !ty.arguments.is_empty() || !matches!(ty.name.as_str(), "String" | "Bytes") {
        return None;
    }
    android_cpp_value(ty)
}

struct AndroidPrimitiveArray {
    kotlin_array: &'static str,
    jni_array: &'static str,
    element_jni: &'static str,
    get_region: &'static str,
    set_region: &'static str,
}

fn android_primitive_array(ty: &TypeRef) -> Option<AndroidPrimitiveArray> {
    if ty.optional || !ty.arguments.is_empty() {
        return None;
    }
    let scalar = android_cpp_value(ty)?;
    let (kotlin_array, jni_array, get_region, set_region) = match ty.name.as_str() {
        "Bool" => (
            "BooleanArray",
            "jbooleanArray",
            "GetBooleanArrayRegion",
            "SetBooleanArrayRegion",
        ),
        "Int8" | "UInt8" => (
            "ByteArray",
            "jbyteArray",
            "GetByteArrayRegion",
            "SetByteArrayRegion",
        ),
        "Int16" | "UInt16" => (
            "ShortArray",
            "jshortArray",
            "GetShortArrayRegion",
            "SetShortArrayRegion",
        ),
        "Int32" | "UInt32" => (
            "IntArray",
            "jintArray",
            "GetIntArrayRegion",
            "SetIntArrayRegion",
        ),
        "Int64" | "UInt64" => (
            "LongArray",
            "jlongArray",
            "GetLongArrayRegion",
            "SetLongArrayRegion",
        ),
        "Float32" => (
            "FloatArray",
            "jfloatArray",
            "GetFloatArrayRegion",
            "SetFloatArrayRegion",
        ),
        "Float64" => (
            "DoubleArray",
            "jdoubleArray",
            "GetDoubleArrayRegion",
            "SetDoubleArrayRegion",
        ),
        _ => return None,
    };
    Some(AndroidPrimitiveArray {
        kotlin_array,
        jni_array,
        element_jni: scalar.jni,
        get_region,
        set_region,
    })
}

fn android_kotlin_type(ty: &TypeRef, jni_carrier: bool) -> String {
    if let Some((key, value)) = android_cpp_map_type(ty) {
        let base = format!(
            "Map<{}, {}>",
            android_kotlin_type(key, jni_carrier),
            android_kotlin_type(value, jni_carrier)
        );
        return if ty.optional {
            format!("{base}?")
        } else {
            base
        };
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        if let Some(element) = ty.arguments.first() {
            if jni_carrier {
                if let Some(array) = android_primitive_array(element) {
                    return array.kotlin_array.to_owned();
                }
                if android_jni_reference_class_available(element) {
                    return format!("Array<{}>", android_kotlin_type(element, true));
                }
            }
            if ty.name == "Set" {
                return format!("Set<{}>", android_kotlin_type(element, false));
            }
            return format!("List<{}>", android_kotlin_type(&ty.arguments[0], false));
        }
    }
    let Some(scalar) = android_cpp_value(ty) else {
        let base = ty.name.clone();
        return if ty.optional {
            format!("{base}?")
        } else {
            base
        };
    };
    let base = if jni_carrier {
        scalar.jni_kotlin
    } else {
        scalar.kotlin
    };
    if ty.optional {
        format!("{base}?")
    } else {
        base.to_owned()
    }
}

fn render_kotlin_native_declaration(out: &mut String, method: &Method, native_name: &str) {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                android_kotlin_type(&parameter.ty, true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = android_kotlin_type(android_cpp_method_success_type(method), true);
    out.push_str(&format!(
        "    external fun {native_name}({parameters}): {return_type}\n"
    ));
}

fn render_kotlin_class_native_declaration(out: &mut String, method: &Method, native_name: &str) {
    let mut parameters = vec!["handle: Long".to_owned()];
    parameters.extend(method.parameters.iter().map(|parameter| {
        format!(
            "{}: {}",
            parameter.name,
            android_kotlin_type(&parameter.ty, true)
        )
    }));
    let return_type = android_kotlin_type(android_cpp_method_success_type(method), true);
    out.push_str(&format!(
        "    external fun {native_name}({}): {return_type}\n",
        parameters.join(", ")
    ));
}

fn render_kotlin_property_native_declarations(
    out: &mut String,
    property: &Property,
    interface: &str,
    getter_name: &str,
) {
    let value_type = android_kotlin_type(&property.ty, true);
    out.push_str(&format!(
        "    external fun {getter_name}(handle: Long): {value_type}\n"
    ));
    if property.mutable {
        out.push_str(&format!(
            "    external fun set_{interface}_{}(handle: Long, value: {value_type})\n",
            property.name
        ));
    }
}

fn render_kotlin_native_dispose_declaration(out: &mut String, native_name: &str) {
    out.push_str(&format!("    external fun {native_name}(handle: Long)\n"));
}

fn render_kotlin_native_event_declaration(out: &mut String, native_name: &str) {
    out.push_str(&format!(
        "    external fun {native_name}(handle: Long, callback: Any?)\n"
    ));
}

fn render_kotlin_service_adapter(
    out: &mut String,
    method: &Method,
    native_name: &str,
    bindings_class: &str,
) {
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                android_kotlin_type(&parameter.ty, false)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let success_type = android_cpp_method_success_type(method);
    let return_type = android_kotlin_type(success_type, false);
    if method.is_async {
        let native_call = format!("{bindings_class}.{native_name}({arguments})");
        let expression = if return_type == "Unit" {
            native_call
        } else {
            kotlin_from_jni_expression(native_call, success_type)
        };
        out.push_str(&format!(
            "    override suspend fun {}({parameters}): {return_type} = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {{ {expression} }}\n",
            method.name
        ));
        return;
    }
    if return_type == "Unit" {
        out.push_str(&format!(
            "    override fun {}({parameters}) {{ {bindings_class}.{native_name}({arguments}) }}\n",
            method.name
        ));
    } else {
        let native_call = format!("{bindings_class}.{native_name}({arguments})");
        let result = kotlin_from_jni_expression(native_call, success_type);
        out.push_str(&format!(
            "    override fun {}({parameters}): {return_type} = {result}\n",
            method.name
        ));
    }
}

fn kotlin_to_jni_expression(value: &str, ty: &TypeRef) -> String {
    if let Some((key, map_value)) = android_cpp_map_type(ty) {
        let mut expression = if ty.optional {
            "nexaOptionalMap".to_owned()
        } else {
            value.to_owned()
        };
        if let Some(conversion) = android_cpp_value(key).and_then(|scalar| scalar.kotlin_to_jni) {
            expression = format!("{expression}.mapKeys {{ it.key.{conversion}() }}");
        }
        let converted_value = kotlin_to_jni_expression("nexaMapValue", map_value);
        if converted_value != "nexaMapValue" {
            expression =
                format!("{expression}.mapValues {{ (_, nexaMapValue) -> {converted_value} }}");
        }
        if (!ty.optional && expression == value) || (ty.optional && expression == "nexaOptionalMap")
        {
            return value.to_owned();
        }
        return if ty.optional {
            format!("{value}?.let {{ nexaOptionalMap -> {expression} }}")
        } else {
            expression
        };
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let element = &ty.arguments[0];
        if ty.name == "Array" && matches!(element.name.as_str(), "Array" | "Set")
            || (ty.name == "Array" && android_cpp_map_type(element).is_some())
            || element.optional
        {
            return format!(
                "{value}.map {{ nexaNestedCollection -> {} }}.toTypedArray()",
                kotlin_to_jni_expression("nexaNestedCollection", element)
            );
        }
        let Some(array) = android_primitive_array(element) else {
            debug_assert!(android_reference_array_element(element).is_some());
            return format!("{value}.toTypedArray()");
        };
        if ty.name == "Set" {
            let scalar = android_cpp_value(element).expect("validated array element");
            let source = if let Some(conversion) = scalar.kotlin_to_jni {
                format!("{value}.map {{ it.{conversion}() }}")
            } else {
                value.to_owned()
            };
            return format!(
                "{source}.to{}Array()",
                array.kotlin_array.trim_end_matches("Array")
            );
        }
        if let Some(conversion) = android_cpp_value(element)
            .expect("validated array element")
            .kotlin_to_jni
        {
            return format!(
                "{}({value}.size) {{ {value}[it].{conversion}() }}",
                array.kotlin_array
            );
        }
        return format!(
            "{value}.to{}Array()",
            array.kotlin_array.trim_end_matches("Array")
        );
    }
    if android_cpp_value(ty).is_none() {
        return value.to_owned();
    }
    let scalar = android_cpp_value(ty).expect("validated Android value");
    if ty.optional && scalar.unsigned {
        // A nullable signed Long carrier preserves every UInt64 bit pattern and null.
        return format!("{value}?.toLong()");
    }
    scalar.kotlin_to_jni.map_or_else(
        || value.to_owned(),
        |conversion| format!("{value}.{conversion}()"),
    )
}

fn kotlin_from_jni_expression(expression: String, ty: &TypeRef) -> String {
    if let Some((key, map_value)) = android_cpp_map_type(ty) {
        let mut result = if ty.optional {
            "nexaOptionalMap".to_owned()
        } else {
            expression.clone()
        };
        if let Some(conversion) = android_cpp_value(key).and_then(|scalar| scalar.kotlin_from_jni) {
            result = format!("{result}.mapKeys {{ it.key.{conversion}() }}");
        }
        let converted_value = kotlin_from_jni_expression("nexaMapValue".to_owned(), map_value);
        if converted_value != "nexaMapValue" {
            result = format!("{result}.mapValues {{ (_, nexaMapValue) -> {converted_value} }}");
        }
        if (!ty.optional && result == expression) || (ty.optional && result == "nexaOptionalMap") {
            return expression;
        }
        return if ty.optional {
            format!("{expression}?.let {{ nexaOptionalMap -> {result} }}")
        } else {
            result
        };
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let element = &ty.arguments[0];
        if ty.name == "Array" && matches!(element.name.as_str(), "Array" | "Set")
            || (ty.name == "Array" && android_cpp_map_type(element).is_some())
            || element.optional
        {
            let converted = format!(
                "{expression}.map {{ nexaNestedCollection -> {} }}",
                kotlin_from_jni_expression("nexaNestedCollection".to_owned(), element)
            );
            return if ty.name == "Set" {
                format!("{converted}.toSet()")
            } else {
                converted
            };
        }
        let Some(_array) = android_primitive_array(element) else {
            debug_assert!(android_reference_array_element(element).is_some());
            return if ty.name == "Set" {
                format!("{expression}.toSet()")
            } else {
                format!("{expression}.asList()")
            };
        };
        if ty.name == "Set" {
            if let Some(conversion) = android_cpp_value(element)
                .expect("validated array element")
                .kotlin_from_jni
            {
                return format!("{expression}.map {{ it.{conversion}() }}.toSet()");
            }
            return format!("{expression}.toSet()");
        }
        if let Some(conversion) = android_cpp_value(element)
            .expect("validated array element")
            .kotlin_from_jni
        {
            return format!("{expression}.map {{ it.{conversion}() }}");
        }
        return format!("{expression}.asList()");
    }
    if android_cpp_value(ty).is_none() {
        return expression;
    }
    let scalar = android_cpp_value(ty).expect("validated Android value");
    if ty.optional && scalar.unsigned {
        let conversion = scalar
            .kotlin_from_jni
            .expect("unsigned Kotlin values have a carrier conversion");
        return format!("{expression}?.{conversion}()");
    }
    match scalar.kotlin_from_jni {
        Some(conversion) => format!("{expression}.{conversion}()"),
        None => expression,
    }
}

fn render_kotlin_cpp_class(
    out: &mut String,
    interface: &Interface,
    plugin_index: usize,
    constructor: Option<&nexa_plugin_idl::Constructor>,
) {
    let parameters = constructor
        .map(|constructor| {
            constructor
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}: {}",
                        parameter.name,
                        android_kotlin_type(&parameter.ty, false)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let arguments = constructor
        .map(|constructor| {
            constructor
                .parameters
                .iter()
                .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    out.push_str(&format!("\npublic class {}Impl private constructor(private var nativeHandle: Long) : {}Spec, java.lang.AutoCloseable {{\n    public constructor({parameters}) : this(NexaPlugin{plugin_index}_CppBindings.create_{}({arguments}))\n    private fun requireNativeHandle(): Long = checkNotNull(nativeHandle.takeIf {{ it != 0L }}) {{ \"{} is disposed\" }}\n", interface.name, interface.name, interface.name, interface.name));
    for property in &interface.properties {
        let value_type = android_kotlin_type(&property.ty, false);
        let getter = format!("get_{}_{}", interface.name, property.name);
        let getter_call =
            format!("NexaPlugin{plugin_index}_CppBindings.{getter}(requireNativeHandle())");
        let getter_value = kotlin_from_jni_expression(getter_call, &property.ty);
        out.push_str(&format!(
            "    override {} {}: {value_type}\n        @Synchronized\n        get() = {getter_value}",
            if property.mutable { "var" } else { "val" },
            property.name
        ));
        if property.mutable {
            let value = kotlin_to_jni_expression("value", &property.ty);
            out.push_str(&format!(
                "\n        @Synchronized\n        set(value) = NexaPlugin{plugin_index}_CppBindings.set_{}_{}(requireNativeHandle(), {value})",
                interface.name,
                property.name
            ));
        }
        out.push('\n');
    }
    for event in &interface.events {
        let property = nexa_plugin_idl::event_callback_property(&event.name);
        let bridge = android_cpp_event_bridge_name(plugin_index, interface, event);
        let setter = format!("set_{}_{}", interface.name, property);
        let callback_type = android_kotlin_event_callback_type(event);
        let backing = android_cpp_event_backing_name(interface, event);
        out.push_str(&format!(
            "    private var {backing}: {bridge}? = null\n    override var {property}: {callback_type}?\n        @Synchronized get() = {backing}?.callback\n        @Synchronized set(value) {{\n            val previous = {backing}\n            previous?.deactivate()\n            val next = value?.let {{ {bridge}(it) }}\n            NexaPlugin{plugin_index}_CppBindings.{setter}(requireNativeHandle(), next)\n            {backing} = next\n        }}\n"
        ));
    }
    for method in &interface.methods {
        if method.name == "dispose" {
            continue;
        }
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    parameter.name,
                    android_kotlin_type(&parameter.ty, false)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let arguments = std::iter::once("requireNativeHandle()".to_owned())
            .chain(
                method
                    .parameters
                    .iter()
                    .map(|parameter| kotlin_to_jni_expression(&parameter.name, &parameter.ty)),
            )
            .collect::<Vec<_>>()
            .join(", ");
        let success_type = android_cpp_method_success_type(method);
        let return_type = android_kotlin_type(success_type, false);
        let native_name = format!("call_{}_{}", interface.name, method.name);
        if method.is_async {
            let call = if return_type == "Unit" {
                format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})")
            } else {
                let native_call =
                    format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})");
                kotlin_from_jni_expression(native_call, success_type)
            };
            out.push_str(&format!(
                "    override suspend fun {}({parameters}): {return_type} = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {{ synchronized(this) {{ {call} }} }}\n",
                method.name
            ));
        } else if return_type == "Unit" {
            out.push_str("    @Synchronized\n");
            out.push_str(&format!("    override fun {}({parameters}) {{ NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments}) }}\n", method.name));
        } else {
            out.push_str("    @Synchronized\n");
            let native_call =
                format!("NexaPlugin{plugin_index}_CppBindings.{native_name}({arguments})");
            let result = kotlin_from_jni_expression(native_call, success_type);
            out.push_str(&format!(
                "    override fun {}({parameters}): {return_type} = {result}\n",
                method.name
            ));
        }
    }
    out.push_str(&format!("    @Synchronized\n    override fun dispose() {{\n        val handle = nativeHandle\n        if (handle == 0L) return\n        nativeHandle = 0L\n"));
    for event in &interface.events {
        let backing = android_cpp_event_backing_name(interface, event);
        out.push_str(&format!(
            "        {backing}?.deactivate()\n        {backing} = null\n"
        ));
    }
    out.push_str(&format!(
        "        NexaPlugin{plugin_index}_CppBindings.dispose_{}(handle)\n    }}\n    override fun close() = dispose()\n    @Suppress(\"DEPRECATION\")\n    protected fun finalize() {{\n        if (nativeHandle != 0L) dispose()\n    }}\n}}\n",
        interface.name
    ));
}

fn android_cpp_event_backing_name(interface: &Interface, event: &Event) -> String {
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
    let base = format!("nexaCppEvent{}Bridge", cpp_identifier(&event.name));
    let mut candidate = base.clone();
    let mut suffix = 1usize;
    while occupied.contains(&candidate) {
        candidate = format!("{base}{suffix}");
        suffix += 1;
    }
    candidate
}

fn render_kotlin_event_bridge(
    out: &mut String,
    interface: &Interface,
    event: &Event,
    plugin_index: usize,
) {
    let name = android_cpp_event_bridge_name(plugin_index, interface, event);
    let callback_type = android_kotlin_event_callback_type(event);
    out.push_str(&format!("private class {name}(val callback: {callback_type}) {{\n    @Volatile private var active = true\n    fun deactivate() {{ active = false }}\n"));
    let parameters = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!(
                "nexaArg{index}: {}",
                android_kotlin_type(&parameter.ty, true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let conversions = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!(
                "        val nexaEventValue{index} = {}\n",
                kotlin_from_jni_expression(format!("nexaArg{index}"), &parameter.ty)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let arguments = (0..event.parameters.len())
        .map(|index| format!("nexaEventValue{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "    @JvmName(\"nexaDispatch\")\n    fun dispatchFromNative({parameters}) {{\n{}        if (!active) return\n        NexaPlugin{plugin_index}_CppEventDispatcher.post {{ if (active) callback({arguments}) }}\n    }}\n}}\n\n",
        conversions
    ));
}

fn android_cpp_event_bridge_name(
    plugin_index: usize,
    interface: &Interface,
    event: &Event,
) -> String {
    format!(
        "NexaPlugin{plugin_index}_CppEvent{}_{}",
        cpp_identifier(&interface.name),
        cpp_identifier(&event.name)
    )
}

fn android_kotlin_event_callback_type(event: &Event) -> String {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| android_kotlin_type(&parameter.ty, false))
        .collect::<Vec<_>>()
        .join(", ");
    format!("(({parameters}) -> Unit)")
}

fn render_jni_service_method(
    out: &mut String,
    package: &str,
    class_name: &str,
    interface_name: &str,
    method: &Method,
    native_name: &str,
) {
    let symbol = jni_symbol(package, class_name, native_name);
    let parameters = jni_parameter_declarations(&method.parameters);
    let success_type = android_cpp_method_success_type(method);
    let return_type = android_jni_type(success_type).expect("validated Android type");
    let method_name = cpp_identifier(&method.name);
    let prefix = if parameters.is_empty() {
        String::new()
    } else {
        format!(", {parameters}")
    };
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {symbol}(JNIEnv* env, jobject thiz{prefix}) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    let arguments =
        render_jni_argument_conversions(out, &method.parameters, jni_failure_return(&return_type));
    let call = format!(
        "{}::{}({})",
        cpp_identifier(interface_name),
        method_name,
        arguments.join(", ")
    );
    let call = if method.is_async {
        out.push_str(&format!("        auto nexaCppFuture = {call};\n"));
        "nexaCppFuture.get()".to_owned()
    } else {
        call
    };
    if let Some(error_type) = android_cpp_method_error_type(method) {
        render_jni_typed_error_result(
            out,
            success_type,
            error_type,
            &call,
            jni_failure_return(&return_type),
        );
    } else if android_cpp_is_void(success_type) {
        out.push_str(&format!("        {call};\n"));
    } else {
        render_jni_return(out, success_type, &call);
    }
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_constructor(
    out: &mut String,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &Interface,
    native_name: &str,
) {
    let symbol = jni_symbol(package, class_name, native_name);
    let constructor = interface.constructors.first();
    let parameters = constructor
        .map(|constructor| jni_parameter_declarations(&constructor.parameters))
        .unwrap_or_default();
    let prefix = if parameters.is_empty() {
        String::new()
    } else {
        format!(", {parameters}")
    };
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT jlong JNICALL {symbol}(JNIEnv* env, jobject thiz{prefix}) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "jlong");
    let arguments = constructor
        .map(|constructor| {
            render_jni_argument_conversions(out, &constructor.parameters, "return 0;")
        })
        .unwrap_or_default();
    out.push_str(&format!(
        "        auto instance = {}::{}({});\n        if (!instance) throw std::runtime_error(\"native plugin factory returned null\");\n        auto* handle = new std::unique_ptr<{cpp_type_name}>(std::move(instance));\n        return static_cast<jlong>(reinterpret_cast<std::uintptr_t>(handle));\n",
        cpp_namespace(plugin_id),
        cpp_factory_name(&interface.name),
        arguments.join(", ")
    ));
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_property(
    out: &mut String,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &Interface,
    property: &Property,
    getter_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let return_type = android_jni_type(&property.ty).expect("validated property type");
    let getter_symbol = jni_symbol(package, class_name, getter_name);
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {getter_symbol}(JNIEnv* env, jobject thiz, jlong rawHandle) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) {}\n",
        jni_failure_return(&return_type)
    ));
    render_jni_return(
        out,
        &property.ty,
        &format!("(*handle)->{}()", cpp_getter_name(&property.name)),
    );
    render_jni_boundary_end(out);
    out.push_str("}\n\n");

    if property.mutable {
        let setter_name = format!("set_{}_{}", interface.name, property.name);
        let setter_symbol = jni_symbol(package, class_name, &setter_name);
        out.push_str(&format!(
            "extern \"C\" JNIEXPORT void JNICALL {setter_symbol}(JNIEnv* env, jobject thiz, jlong rawHandle, {return_type} value) {{\n    (void)thiz;\n",
        ));
        render_jni_boundary_start(out, "void");
        out.push_str(&format!(
            "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n"
        ));
        let value = if android_cpp_map_type(&property.ty).is_some() {
            render_jni_map_argument_conversion(
                out,
                &property.ty,
                "value",
                "return;",
                "nexaJniMapProperty",
            )
        } else if matches!(property.ty.name.as_str(), "Array" | "Set") {
            render_jni_array_argument_conversion(
                out,
                &property.ty,
                "value",
                "return;",
                "nexaJniArrayProperty",
            )
        } else if android_cpp_value(&property.ty)
            .expect("validated property type")
            .optional
        {
            render_jni_optional_argument_conversion(
                out,
                &property.ty,
                "value",
                "return;",
                "nexaJniOptionalProperty",
            )
        } else if return_type == "jstring" {
            out.push_str(
                "        auto nexaJniStringValue = fromJniString(env, value);\n        if (env->ExceptionCheck()) return;\n",
            );
            "std::move(nexaJniStringValue)".to_owned()
        } else if return_type == "jbyteArray" {
            out.push_str(
                "        auto nexaJniBytesValue = fromJniBytes(env, value);\n        if (env->ExceptionCheck()) return;\n",
            );
            "std::move(nexaJniBytesValue)".to_owned()
        } else {
            format!(
                "static_cast<{}>(value)",
                android_cpp_type(&property.ty).expect("validated property type")
            )
        };
        out.push_str(&format!(
            "        (*handle)->{}({value});\n",
            cpp_setter_name(&property.name)
        ));
        render_jni_boundary_end(out);
        out.push_str("}\n\n");
    }
}

fn render_jni_class_method(
    out: &mut String,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &Interface,
    method: &Method,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let success_type = android_cpp_method_success_type(method);
    let return_type = android_jni_type(success_type).expect("validated method type");
    let failure_return = jni_failure_return(&return_type);
    let symbol = jni_symbol(package, class_name, native_name);
    let mut parameters = String::from("jlong rawHandle");
    for parameter in &method.parameters {
        parameters.push_str(&format!(
            ", {} {}",
            android_jni_type(&parameter.ty).expect("validated parameter type"),
            parameter.name
        ));
    }
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT {return_type} JNICALL {symbol}(JNIEnv* env, jobject thiz, {parameters}) {{\n    (void)thiz;\n",
    ));
    render_jni_boundary_start(out, &return_type);
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) {failure_return}\n"
    ));
    let arguments = render_jni_argument_conversions(out, &method.parameters, failure_return);
    let call = format!(
        "(*handle)->{}({})",
        cpp_identifier(&method.name),
        arguments.join(", ")
    );
    let call = if method.is_async {
        out.push_str(&format!("        auto nexaCppFuture = {call};\n"));
        "nexaCppFuture.get()".to_owned()
    } else {
        call
    };
    if let Some(error_type) = android_cpp_method_error_type(method) {
        render_jni_typed_error_result(out, success_type, error_type, &call, failure_return);
    } else if android_cpp_is_void(success_type) {
        out.push_str(&format!("        {call};\n"));
    } else {
        render_jni_return(out, success_type, &call);
    }
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_dispose(
    out: &mut String,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &Interface,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let symbol = jni_symbol(package, class_name, native_name);
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT void JNICALL {symbol}(JNIEnv* env, jobject thiz, jlong rawHandle) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "void");
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n        std::unique_ptr<{cpp_type_name}> instance = std::move(*handle);\n        delete handle;\n"
    ));
    for event in &interface.events {
        out.push_str(&format!(
            "        instance->{}({{}});\n",
            cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name))
        ));
    }
    out.push_str("        instance->dispose();\n");
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn render_jni_event_setter(
    out: &mut String,
    plugin_id: &str,
    package: &str,
    class_name: &str,
    interface: &Interface,
    event: &Event,
    native_name: &str,
) {
    let cpp_type_name = format!(
        "{}::{}Spec",
        cpp_namespace(plugin_id),
        cpp_identifier(&interface.name)
    );
    let symbol = jni_symbol(package, class_name, native_name);
    let descriptor = event
        .parameters
        .iter()
        .map(|parameter| android_jni_callback_descriptor(&parameter.ty, package))
        .collect::<String>();
    let setter = cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name));
    out.push_str(&format!(
        "extern \"C\" JNIEXPORT void JNICALL {symbol}(JNIEnv* env, jobject thiz, jlong rawHandle, jobject callback) {{\n    (void)thiz;\n"
    ));
    render_jni_boundary_start(out, "void");
    out.push_str(&format!(
        "        auto* handle = requireHandle<{cpp_type_name}>(env, rawHandle);\n        if (handle == nullptr) return;\n"
    ));
    out.push_str(
        "        jmethodID nexaDispatch = nullptr;\n        std::shared_ptr<JniGlobalReference> nexaCallback;\n",
    );
    out.push_str(
        "        if (callback != nullptr) {\n            ScopedLocalRef<jclass> nexaCallbackClass(env, env->GetObjectClass(callback));\n            if (nexaCallbackClass.get() == nullptr) return;\n",
    );
    out.push_str(&format!(
        "            nexaDispatch = env->GetMethodID(nexaCallbackClass.get(), \"nexaDispatch\", \"({descriptor})V\");\n            if (nexaDispatch == nullptr) return;\n            nexaCallback = std::make_shared<JniGlobalReference>(env, callback);\n            if (nexaCallback->get() == nullptr) return;\n        }}\n        if (callback == nullptr) {{ (*handle)->{setter}({{}}); return; }}\n"
    ));
    let lambda_parameters = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| format!("const {}& nexaValue{index}", cpp_type(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "        (*handle)->{setter}([nexaCallback, nexaDispatch]({lambda_parameters}) noexcept {{\n            try {{\n                ScopedJniEnvironment nexaEnvironment(nexaCallback->vm());\n                JNIEnv* nexaEnv = nexaEnvironment.get();\n                if (nexaEnv == nullptr) return;\n                JNIEnv* env = nexaEnv;\n                ScopedJniLocalFrame nexaLocalFrame(nexaEnv, {});\n                if (!nexaLocalFrame.active()) {{ if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear(); return; }}\n                ScopedLocalRef<jobject> nexaCallbackLocal(nexaEnv, nexaEnv->NewLocalRef(nexaCallback->get()));\n                if (nexaCallbackLocal.get() == nullptr || nexaEnv->ExceptionCheck()) {{ if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear(); return; }}\n                std::array<jvalue, {}> nexaArguments{{}};\n",
        (event.parameters.len() * 4 + 8).max(16),
        event.parameters.len(),
    ));
    for (index, parameter) in event.parameters.iter().enumerate() {
        let jni_type =
            android_jni_type(&parameter.ty).expect("validated event parameter has a JNI carrier");
        out.push_str(&format!(
            "                auto nexaArgument{index} = [&]() -> {jni_type} {{\n"
        ));
        render_jni_return(out, &parameter.ty, &format!("nexaValue{index}"));
        out.push_str("                }();\n                if (nexaEnv->ExceptionCheck()) { nexaEnv->ExceptionClear(); return; }\n");
        let field = match jni_type.as_str() {
            "jboolean" => "z",
            "jbyte" => "b",
            "jshort" => "s",
            "jint" => "i",
            "jlong" => "j",
            "jfloat" => "f",
            "jdouble" => "d",
            _ => "l",
        };
        let value = if field == "l" {
            format!("static_cast<jobject>(nexaArgument{index})")
        } else {
            format!("nexaArgument{index}")
        };
        out.push_str(&format!(
            "                nexaArguments[{index}].{field} = {value};\n"
        ));
    }
    out.push_str(
        "                nexaEnv->CallVoidMethodA(nexaCallbackLocal.get(), nexaDispatch, nexaArguments.data());\n                if (nexaEnv->ExceptionCheck()) nexaEnv->ExceptionClear();\n            } catch (...) { }\n        });\n",
    );
    render_jni_boundary_end(out);
    out.push_str("}\n\n");
}

fn android_jni_callback_descriptor(ty: &TypeRef, package: &str) -> String {
    if android_cpp_map_type(ty).is_some() {
        return "Ljava/util/Map;".to_owned();
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let element = &ty.arguments[0];
        if android_primitive_array(element).is_some() {
            let code = android_primitive_descriptor(element);
            return format!("[{code}");
        }
        return format!("[{}", android_jni_callback_descriptor(element, package));
    }
    let Some(scalar) = android_cpp_value(ty) else {
        return format!("L{}/{};", package.replace('.', "/"), ty.name);
    };
    if ty.optional {
        return match ty.name.as_str() {
            "String" => "Ljava/lang/String;".to_owned(),
            "Bytes" => "[B".to_owned(),
            _ => {
                let wrapper = match scalar.jni_kotlin {
                    "Boolean" => "java/lang/Boolean",
                    "Byte" => "java/lang/Byte",
                    "Short" => "java/lang/Short",
                    "Int" => "java/lang/Integer",
                    "Long" => "java/lang/Long",
                    "Float" => "java/lang/Float",
                    "Double" => "java/lang/Double",
                    _ => unreachable!("validated optional scalar carrier has a wrapper"),
                };
                format!("L{wrapper};")
            }
        };
    }
    match scalar.jni {
        "jboolean" => "Z".to_owned(),
        "jbyte" => "B".to_owned(),
        "jshort" => "S".to_owned(),
        "jint" => "I".to_owned(),
        "jlong" => "J".to_owned(),
        "jfloat" => "F".to_owned(),
        "jdouble" => "D".to_owned(),
        "jstring" => "Ljava/lang/String;".to_owned(),
        "jbyteArray" => "[B".to_owned(),
        _ => unreachable!("validated event callback type has a JVM descriptor"),
    }
}

fn android_primitive_descriptor(ty: &TypeRef) -> &'static str {
    match ty.name.as_str() {
        "Bool" => "Z",
        "Int8" | "UInt8" => "B",
        "Int16" | "UInt16" => "S",
        "Int32" | "UInt32" => "I",
        "Int64" | "UInt64" => "J",
        "Float32" => "F",
        "Float64" => "D",
        _ => unreachable!("validated primitive array element has a descriptor"),
    }
}

fn render_jni_boundary_start(out: &mut String, return_type: &str) {
    out.push_str(&format!(
        "    return withJniExceptions(env, [&]() -> {return_type} {{\n"
    ));
}

fn render_jni_boundary_end(out: &mut String) {
    out.push_str("    });\n");
}

fn jni_failure_return(return_type: &str) -> &'static str {
    if return_type == "void" {
        "return;"
    } else {
        "return {};"
    }
}

fn render_jni_typed_error_result(
    out: &mut String,
    success_type: &TypeRef,
    error_type: &TypeRef,
    expression: &str,
    failure_return: &str,
) {
    let error_name = cpp_identifier(&error_type.name);
    out.push_str(&format!(
        "        auto nexaCppTypedResult = {expression};\n        if (!nexaCppTypedResult.has_value()) {{\n            nexaCppThrowError{error_name}(env, nexaCppTypedResult.error());\n            {failure_return}\n        }}\n"
    ));
    if android_cpp_is_void(success_type) {
        return;
    }
    render_jni_return(out, success_type, "nexaCppTypedResult.value()");
}

fn render_android_error_factory(out: &mut String, idl: &PluginIdl, plugin_index: usize) {
    let errors = android_cpp_referenced_errors(idl);
    if errors.is_empty() {
        return;
    }
    out.push_str(&format!(
        "internal object NexaPlugin{plugin_index}_CppErrorFactory {{\n"
    ));
    for error in errors {
        for (case_index, case) in error.cases.iter().enumerate() {
            let parameters = case
                .parameters
                .iter()
                .map(|parameter| {
                    format!(
                        "{}: {}",
                        cpp_identifier(&parameter.name),
                        android_kotlin_type(&parameter.ty, true)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let arguments = case
                .parameters
                .iter()
                .map(|parameter| {
                    let name = cpp_identifier(&parameter.name);
                    match parameter.ty.name.as_str() {
                        "UInt8" => format!("{name}.toUByte()"),
                        "UInt16" => format!("{name}.toUShort()"),
                        "UInt32" => format!("{name}.toUInt()"),
                        "UInt64" => format!("{name}.toULong()"),
                        _ => name,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let method = android_cpp_error_factory_method(&error.name, case_index);
            let case_name = cpp_identifier(&case.name);
            let value = if case.parameters.is_empty() {
                format!("{}.{}", error.name, case_name)
            } else {
                format!("{}.{}({arguments})", error.name, case_name)
            };
            out.push_str(&format!(
                "    @JvmStatic public fun {method}({parameters}): {} = {value}\n",
                error.name
            ));
        }
    }
    out.push_str("}\n\n");
}

fn render_android_jni_error_converters(
    out: &mut String,
    idl: &PluginIdl,
    package: &str,
    plugin_index: usize,
) {
    let errors = android_cpp_referenced_errors(idl);
    if errors.is_empty() {
        return;
    }
    let factory_class = format!(
        "{}/NexaPlugin{plugin_index}_CppErrorFactory",
        package.replace('.', "/")
    );
    out.push_str("\nnamespace {\n");
    for error in errors {
        let error_name = cpp_identifier(&error.name);
        let return_signature = format!("L{}/{};", package.replace('.', "/"), error.name);
        out.push_str(&format!(
            "void nexaCppThrowError{error_name}(JNIEnv* env, const {error_name}& error) {{\n    ScopedLocalRef<jclass> factoryClass(env, env->FindClass(\"{factory_class}\"));\n    if (factoryClass.get() == nullptr) return;\n    switch (error.value.index()) {{\n"
        ));
        for (case_index, case) in error.cases.iter().enumerate() {
            let method_name = android_cpp_error_factory_method(&error.name, case_index);
            let parameters_signature = case
                .parameters
                .iter()
                .map(|parameter| android_jni_signature(&parameter.ty))
                .collect::<String>();
            let method_signature = format!("({parameters_signature}){return_signature}");
            out.push_str(&format!(
                "    case {case_index}: {{\n        jmethodID factoryMethod = env->GetStaticMethodID(factoryClass.get(), \"{method_name}\", \"{method_signature}\");\n        if (factoryMethod == nullptr) return;\n        std::array<jvalue, {}> arguments{{}};\n",
                case.parameters.len().max(1)
            ));
            for (parameter_index, parameter) in case.parameters.iter().enumerate() {
                let case_type = cpp_case_type_name(&case.name);
                let value = format!(
                    "std::get<{error_name}::{case_type}>(error.value).{}",
                    cpp_identifier(&parameter.name)
                );
                let scalar = android_cpp_value(&parameter.ty)
                    .expect("validated Android typed error payload");
                match parameter.ty.name.as_str() {
                    "String" | "Bytes" => {
                        let converter = if parameter.ty.name == "String" {
                            "toJniString"
                        } else {
                            "toJniBytes"
                        };
                        out.push_str(&format!(
                            "        ScopedLocalRef<jobject> argument{parameter_index}(env, {converter}(env, {value}));\n        if (env->ExceptionCheck()) return;\n        arguments[{parameter_index}].l = argument{parameter_index}.get();\n"
                        ));
                    }
                    _ => {
                        let carrier = if scalar.unsigned {
                            match parameter.ty.name.as_str() {
                                "UInt8" => format!(
                                    "std::bit_cast<jbyte>(static_cast<std::uint8_t>({value}))"
                                ),
                                "UInt16" => format!(
                                    "std::bit_cast<jshort>(static_cast<std::uint16_t>({value}))"
                                ),
                                "UInt32" => format!(
                                    "std::bit_cast<jint>(static_cast<std::uint32_t>({value}))"
                                ),
                                "UInt64" => format!(
                                    "std::bit_cast<jlong>(static_cast<std::uint64_t>({value}))"
                                ),
                                _ => unreachable!("only unsigned integer payloads are unsigned"),
                            }
                        } else {
                            format!("static_cast<{}>({value})", scalar.jni)
                        };
                        let field = match scalar.jni {
                            "jboolean" => "z",
                            "jbyte" => "b",
                            "jshort" => "s",
                            "jint" => "i",
                            "jlong" => "j",
                            "jfloat" => "f",
                            "jdouble" => "d",
                            _ => unreachable!("validated error scalar has a JNI primitive"),
                        };
                        out.push_str(&format!(
                            "        arguments[{parameter_index}].{field} = {carrier};\n"
                        ));
                    }
                }
            }
            out.push_str(&format!(
                "        ScopedLocalRef<jobject> typedError(env, env->CallStaticObjectMethodA(factoryClass.get(), factoryMethod, arguments.data()));\n        if (env->ExceptionCheck()) return;\n        if (typedError.get() == nullptr) {{ throwIllegalState(env, \"Kotlin typed-error factory returned null\"); return; }}\n        env->Throw(static_cast<jthrowable>(typedError.get()));\n        return;\n    }}\n"
            ));
        }
        out.push_str(&format!(
            "    default: throwIllegalState(env, \"native plugin returned an unknown {error_name} variant\"); return;\n    }}\n}}\n\n"
        ));
    }
    out.push_str("} // namespace\n");
}

fn render_android_jni_named_value_helpers(out: &mut String, idl: &PluginIdl, package: &str) {
    let types = idl
        .types
        .iter()
        .filter(|ty| {
            matches!(ty.kind, NamedTypeKind::Struct | NamedTypeKind::Enum)
                && android_cpp_named_value_supported(idl, ty)
        })
        .collect::<Vec<_>>();
    if types.is_empty() {
        return;
    }
    out.push_str("\nnamespace {\n");
    for ty in &types {
        let name = cpp_identifier(&ty.name);
        out.push_str(&format!(
            "{name} nexaFromJni{name}(JNIEnv* env, jobject raw);\njobject nexaToJni{name}(JNIEnv* env, const {name}& value);\n"
        ));
    }
    for ty in types {
        let name = cpp_identifier(&ty.name);
        let class_name = format!("{}/{}", package.replace('.', "/"), ty.name);
        match ty.kind {
            NamedTypeKind::Enum => {
                let values_descriptor = format!("()[L{};", class_name);
                out.push_str(&format!(
                    "{name} nexaFromJni{name}(JNIEnv* env, jobject raw) {{\n    if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa enum was null at the JNI boundary\"); throw std::runtime_error(\"null JNI enum\"); }}\n    ScopedLocalRef<jclass> enumClass(env, env->FindClass(\"{class_name}\"));\n    if (enumClass.get() == nullptr) throw std::runtime_error(\"unable to resolve Kotlin enum class\");\n    jmethodID ordinalMethod = env->GetMethodID(enumClass.get(), \"ordinal\", \"()I\");\n    if (ordinalMethod == nullptr) throw std::runtime_error(\"unable to resolve Kotlin enum ordinal\");\n    const jint ordinal = env->CallIntMethod(raw, ordinalMethod);\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin enum ordinal\");\n    if (ordinal < 0 || ordinal >= {}) {{ throwIllegalState(env, \"Kotlin enum ordinal is outside the IDL definition\"); throw std::runtime_error(\"invalid JNI enum ordinal\"); }}\n    return static_cast<{name}>(ordinal);\n}}\n\njobject nexaToJni{name}(JNIEnv* env, const {name}& value) {{\n    const auto ordinal = static_cast<std::uint8_t>(value);\n    if (ordinal >= {}) {{ throwIllegalState(env, \"native enum value is outside the IDL definition\"); return nullptr; }}\n    ScopedLocalRef<jclass> enumClass(env, env->FindClass(\"{class_name}\"));\n    if (enumClass.get() == nullptr) return nullptr;\n    jmethodID valuesMethod = env->GetStaticMethodID(enumClass.get(), \"values\", \"{values_descriptor}\");\n    if (valuesMethod == nullptr) return nullptr;\n    ScopedLocalRef<jobjectArray> values(env, static_cast<jobjectArray>(env->CallStaticObjectMethod(enumClass.get(), valuesMethod)));\n    if (values.get() == nullptr || env->ExceptionCheck()) return nullptr;\n    return env->GetObjectArrayElement(values.get(), static_cast<jsize>(ordinal));\n}}\n\n",
                    ty.cases.len(),
                    ty.cases.len()
                ));
            }
            NamedTypeKind::Struct => {
                let descriptor = ty
                    .fields
                    .iter()
                    .map(|field| android_jni_callback_descriptor(&field.ty, package))
                    .collect::<String>();
                let mut constructor_descriptor = format!("({descriptor})V");
                if ty.fields.is_empty() {
                    constructor_descriptor = "()V".to_owned();
                }
                out.push_str(&format!(
                    "{name} nexaFromJni{name}(JNIEnv* env, jobject raw) {{\n    if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa struct was null at the JNI boundary\"); throw std::runtime_error(\"null JNI struct\"); }}\n    ScopedLocalRef<jclass> valueClass(env, env->FindClass(\"{class_name}\"));\n    if (valueClass.get() == nullptr) throw std::runtime_error(\"unable to resolve Kotlin data class\");\n    {name} result{{}};\n"
                ));
                for (index, field) in ty.fields.iter().enumerate() {
                    let field_name = cpp_identifier(&field.name);
                    let getter = android_kotlin_getter_name(&field.name);
                    let signature = android_jni_callback_descriptor(&field.ty, package);
                    let jni_type =
                        android_jni_type(&field.ty).expect("supported struct field has a JNI type");
                    let method = android_jni_call_method(&jni_type);
                    out.push_str(&format!(
                        "    jmethodID getter{index} = env->GetMethodID(valueClass.get(), \"{getter}\", \"(){signature}\");\n    if (getter{index} == nullptr) throw std::runtime_error(\"unable to resolve Kotlin data-class getter\");\n"
                    ));
                    if android_jni_type_is_reference(&jni_type) {
                        out.push_str(&format!(
                            "    ScopedLocalRef<jobject> field{index}(env, env->{method}(raw, getter{index}));\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin data-class field\");\n"
                        ));
                    } else {
                        out.push_str(&format!(
                            "    const {jni_type} field{index} = env->{method}(raw, getter{index});\n    if (env->ExceptionCheck()) throw std::runtime_error(\"unable to read Kotlin data-class field\");\n"
                        ));
                    }
                    let conversion = android_jni_to_cpp_field(&field.ty, index);
                    out.push_str(&format!("    result.{field_name} = {conversion};\n"));
                }
                out.push_str("    return result;\n}\n\n");
                out.push_str(&format!(
                    "jobject nexaToJni{name}(JNIEnv* env, const {name}& value) {{\n    ScopedLocalRef<jclass> valueClass(env, env->FindClass(\"{class_name}\"));\n    if (valueClass.get() == nullptr) return nullptr;\n    jmethodID constructor = env->GetMethodID(valueClass.get(), \"<init>\", \"{constructor_descriptor}\");\n    if (constructor == nullptr) return nullptr;\n    std::array<jvalue, {}> arguments{{}};\n",
                    ty.fields.len()
                ));
                for (index, field) in ty.fields.iter().enumerate() {
                    let field_name = cpp_identifier(&field.name);
                    let jni_type =
                        android_jni_type(&field.ty).expect("supported struct field has a JNI type");
                    let conversion =
                        android_cpp_to_jni_field(&field.ty, &format!("value.{field_name}"));
                    if android_jni_type_is_reference(&jni_type) {
                        out.push_str(&format!(
                            "    ScopedLocalRef<jobject> argument{index}(env, {conversion});\n    if (env->ExceptionCheck()) return nullptr;\n    arguments[{index}].l = argument{index}.get();\n"
                        ));
                    } else {
                        let field = android_jvalue_field(&jni_type);
                        out.push_str(&format!("    arguments[{index}].{field} = {conversion};\n"));
                    }
                }
                out.push_str("    return env->NewObjectA(valueClass.get(), constructor, arguments.data());\n}\n\n");
            }
            NamedTypeKind::Error => unreachable!("errors are not Android value types"),
        }
    }
    out.push_str("} // namespace\n");
}

fn android_jni_to_cpp_field(ty: &TypeRef, index: usize) -> String {
    if let Some(value) = android_cpp_value(ty) {
        return match ty.name.as_str() {
            "String" => format!("fromJniString(env, static_cast<jstring>(field{index}.get()))"),
            "Bytes" => format!("fromJniBytes(env, static_cast<jbyteArray>(field{index}.get()))"),
            _ if value.unsigned => format!("std::bit_cast<{}>(field{index})", value.cpp),
            _ => format!("static_cast<{}>(field{index})", value.cpp),
        };
    }
    format!(
        "nexaFromJni{}(env, field{index}.get())",
        cpp_identifier(&ty.name)
    )
}

fn android_cpp_to_jni_field(ty: &TypeRef, expression: &str) -> String {
    if let Some(value) = android_cpp_value(ty) {
        return match ty.name.as_str() {
            "String" => format!("toJniString(env, {expression})"),
            "Bytes" => format!("toJniBytes(env, {expression})"),
            _ if value.unsigned => format!("std::bit_cast<{}>({expression})", value.jni),
            _ => format!("static_cast<{}>({expression})", value.jni),
        };
    }
    format!("nexaToJni{}(env, {expression})", cpp_identifier(&ty.name))
}

fn android_jni_call_method(jni_type: &str) -> &'static str {
    match jni_type {
        "jboolean" => "CallBooleanMethod",
        "jbyte" => "CallByteMethod",
        "jshort" => "CallShortMethod",
        "jint" => "CallIntMethod",
        "jlong" => "CallLongMethod",
        "jfloat" => "CallFloatMethod",
        "jdouble" => "CallDoubleMethod",
        _ => "CallObjectMethod",
    }
}

fn android_jni_type_is_reference(jni_type: &str) -> bool {
    matches!(jni_type, "jobject" | "jstring" | "jbyteArray")
}

fn android_jvalue_field(jni_type: &str) -> &'static str {
    match jni_type {
        "jboolean" => "z",
        "jbyte" => "b",
        "jshort" => "s",
        "jint" => "i",
        "jlong" => "j",
        "jfloat" => "f",
        "jdouble" => "d",
        _ => "l",
    }
}

fn android_kotlin_getter_name(field_name: &str) -> String {
    let mut characters = field_name.chars();
    let is_property_getter =
        field_name.starts_with("is") && field_name.chars().nth(2).is_some_and(char::is_uppercase);
    match characters.next() {
        Some(_) if is_property_getter => field_name.to_owned(),
        Some(first) => format!("get{}{}", first.to_uppercase(), characters.as_str()),
        None => "get".to_owned(),
    }
}

fn android_cpp_referenced_errors(idl: &PluginIdl) -> Vec<&nexa_plugin_idl::NamedType> {
    let names = idl
        .interfaces
        .iter()
        .filter(|interface| {
            matches!(
                interface.kind,
                InterfaceKind::Service | InterfaceKind::NativeClass
            )
        })
        .flat_map(|interface| &interface.methods)
        .filter_map(android_cpp_method_error_type)
        .map(|ty| ty.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    names
        .into_iter()
        .filter_map(|name| {
            idl.types
                .iter()
                .find(|ty| ty.kind == NamedTypeKind::Error && ty.name == name)
        })
        .collect()
}

fn android_cpp_error_factory_method(error_name: &str, case_index: usize) -> String {
    format!("create{}Case{case_index}", cpp_identifier(error_name))
}

fn android_jni_signature(ty: &TypeRef) -> String {
    match android_jni_type(ty)
        .expect("validated Android typed error payload")
        .as_str()
    {
        "jboolean" => "Z".to_owned(),
        "jbyte" => "B".to_owned(),
        "jshort" => "S".to_owned(),
        "jint" => "I".to_owned(),
        "jlong" => "J".to_owned(),
        "jfloat" => "F".to_owned(),
        "jdouble" => "D".to_owned(),
        "jstring" => "Ljava/lang/String;".to_owned(),
        "jbyteArray" => "[B".to_owned(),
        _ => unreachable!("validated Android typed error payload has a JNI type"),
    }
}

fn render_jni_argument_conversions(
    out: &mut String,
    parameters: &[nexa_plugin_idl::Parameter],
    failure_return: &str,
) -> Vec<String> {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            if android_cpp_map_type(&parameter.ty).is_some() {
                let mut local = format!("nexaJniMapArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                return render_jni_map_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &local,
                );
            }
            if matches!(parameter.ty.name.as_str(), "Array" | "Set") {
                let mut local = format!("nexaJniArrayArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                return render_jni_array_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &local,
                );
            }
            let Some(scalar) = android_cpp_value(&parameter.ty) else {
                let name = cpp_identifier(&parameter.ty.name);
                let local = format!("nexaJniNamedArgument{index}");
                out.push_str(&format!(
                    "        auto {local} = nexaFromJni{name}(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                return format!("std::move({local})");
            };
            if scalar.optional {
                render_jni_optional_argument_conversion(
                    out,
                    &parameter.ty,
                    &parameter.name,
                    failure_return,
                    &format!("nexaJniOptionalArgument{index}"),
                )
            } else if scalar.jni == "jstring" {
                let mut local = format!("nexaJniStringArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                out.push_str(&format!(
                    "        auto {local} = fromJniString(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                format!("std::move({local})")
            } else if scalar.jni == "jbyteArray" {
                let mut local = format!("nexaJniBytesArgument{index}");
                while parameters.iter().any(|parameter| parameter.name == local) {
                    local.push('_');
                }
                out.push_str(&format!(
                    "        auto {local} = fromJniBytes(env, {});\n        if (env->ExceptionCheck()) {failure_return}\n",
                    parameter.name
                ));
                format!("std::move({local})")
            } else {
                format!("static_cast<{}>({})", scalar.cpp, parameter.name)
            }
        })
        .collect()
}

fn render_jni_array_argument_conversion(
    out: &mut String,
    ty: &TypeRef,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let element = &ty.arguments[0];
    if (ty.name == "Array"
        && (matches!(element.name.as_str(), "Array" | "Set")
            || android_cpp_map_type(element).is_some()
            || element.optional))
        || (ty.name == "Set" && element.optional)
    {
        return render_jni_nested_array_argument_conversion(out, ty, value, failure_return, local);
    }
    render_jni_array_argument_conversion_flat(out, ty, value, failure_return, local)
}

fn render_jni_nested_array_argument_conversion(
    out: &mut String,
    ty: &TypeRef,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let element = &ty.arguments[0];
    let cpp_type = android_cpp_type(ty).expect("validated nested array type");
    let is_set = ty.name == "Set";
    let reserve = if is_set {
        String::new()
    } else {
        format!("{local}.reserve(static_cast<std::size_t>({local}Length));\n        ")
    };
    let null_element_check = if element.optional {
        String::new()
    } else {
        format!(
            "            if ({local}Element == nullptr) {{\n                throwIllegalState(env, \"non-null nested Nexa collection contained null\");\n                {failure_return}\n            }}\n"
        )
    };
    out.push_str(&format!(
        "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa collection was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        {cpp_type} {local};\n        {reserve}for (jsize {local}Index = 0; {local}Index < {local}Length; ++{local}Index) {{\n            auto {local}Element = env->GetObjectArrayElement(static_cast<jobjectArray>({value}), {local}Index);\n            if (env->ExceptionCheck()) {failure_return}\n{null_element_check}"
    ));
    let element_local = format!("{local}ElementValue");
    let element_value = if element.name == "Map" {
        render_jni_map_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    } else if matches!(element.name.as_str(), "Array" | "Set") {
        render_jni_array_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    } else {
        render_jni_optional_argument_conversion(
            out,
            element,
            &format!("{local}Element"),
            failure_return,
            &element_local,
        )
    };
    let append = if is_set {
        format!("{local}.insert(std::move({element_value}));")
    } else {
        format!("{local}.push_back(std::move({element_value}));")
    };
    out.push_str(&format!(
        "            {append}\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
    ));
    format!("std::move({local})")
}

fn render_jni_array_argument_conversion_flat(
    out: &mut String,
    ty: &TypeRef,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let element = &ty.arguments[0];
    let is_set = ty.name == "Set";
    if android_primitive_array(element).is_none() {
        let scalar =
            android_reference_array_element(element).expect("validated reference array element");
        let (jni_type, conversion) = if element.name == "String" {
            ("jstring", "fromJniString")
        } else {
            ("jbyteArray", "fromJniBytes")
        };
        if is_set {
            out.push_str(&format!(
                "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa set was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::set<{}> {local};\n        for (jsize index = 0; index < {local}Length; ++index) {{\n            auto {local}Element = static_cast<{jni_type}>(env->GetObjectArrayElement(static_cast<jobjectArray>({value}), index));\n            if (env->ExceptionCheck()) {failure_return}\n            auto {local}Value = {conversion}(env, {local}Element);\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n            {local}.insert(std::move({local}Value));\n        }}\n",
                scalar.cpp,
            ));
        } else {
            out.push_str(&format!(
                "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa array was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<jobjectArray>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::vector<{}> {local};\n        {local}.reserve(static_cast<std::size_t>({local}Length));\n        for (jsize index = 0; index < {local}Length; ++index) {{\n            auto {local}Element = static_cast<{jni_type}>(env->GetObjectArrayElement(static_cast<jobjectArray>({value}), index));\n            if (env->ExceptionCheck()) {failure_return}\n            auto {local}Value = {conversion}(env, {local}Element);\n            if ({local}Element != nullptr) env->DeleteLocalRef({local}Element);\n            if (env->ExceptionCheck()) {failure_return}\n            {local}.push_back(std::move({local}Value));\n        }}\n",
                scalar.cpp,
            ));
        }
        return format!("std::move({local})");
    }
    let array = android_primitive_array(element).expect("validated primitive array");
    let cpp_type = android_cpp_value(element)
        .expect("validated array element")
        .cpp;
    if is_set {
        out.push_str(&format!(
            "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa set was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<{}>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::set<{cpp_type}> {local};\n",
            array.jni_array,
        ));
        out.push_str(&format!(
            "        if ({local}Length > 0) {{\n            std::vector<{}> {local}Carrier({local}Length);\n            env->{}(static_cast<{}>({value}), 0, {local}Length, {local}Carrier.data());\n            if (env->ExceptionCheck()) {failure_return}\n            for (jsize index = 0; index < {local}Length; ++index) {local}.insert(static_cast<{cpp_type}>({local}Carrier[index]));\n        }}\n",
            array.element_jni,
            array.get_region,
            array.jni_array,
        ));
    } else {
        out.push_str(&format!(
            "        if ({value} == nullptr) {{\n            throwIllegalState(env, \"non-null Nexa array was null at the JNI boundary\");\n            {failure_return}\n        }}\n        const jsize {local}Length = env->GetArrayLength(static_cast<{}>({value}));\n        if (env->ExceptionCheck()) {failure_return}\n        std::vector<{cpp_type}> {local}({local}Length);\n",
            array.jni_array,
        ));
        out.push_str(&format!(
            "        if ({local}Length > 0) {{\n            std::vector<{}> {local}Carrier({local}Length);\n            env->{}(static_cast<{}>({value}), 0, {local}Length, {local}Carrier.data());\n            if (env->ExceptionCheck()) {failure_return}\n            for (jsize index = 0; index < {local}Length; ++index) {local}[index] = static_cast<{cpp_type}>({local}Carrier[index]);\n        }}\n",
            array.element_jni,
            array.get_region,
            array.jni_array,
        ));
    }
    format!("std::move({local})")
}

struct AndroidBoxedPrimitive {
    class_name: &'static str,
    unbox_method: &'static str,
    unbox_signature: &'static str,
    unbox_call: &'static str,
    value_of_signature: &'static str,
    jni_primitive: &'static str,
}

fn android_boxed_primitive(name: &str) -> Option<AndroidBoxedPrimitive> {
    let (class_name, unbox_method, unbox_signature, unbox_call, value_of_signature, jni_primitive) =
        match name {
            "Bool" => (
                "java/lang/Boolean",
                "booleanValue",
                "()Z",
                "CallBooleanMethod",
                "(Z)Ljava/lang/Boolean;",
                "jboolean",
            ),
            "Int8" => (
                "java/lang/Byte",
                "byteValue",
                "()B",
                "CallByteMethod",
                "(B)Ljava/lang/Byte;",
                "jbyte",
            ),
            "Int16" => (
                "java/lang/Short",
                "shortValue",
                "()S",
                "CallShortMethod",
                "(S)Ljava/lang/Short;",
                "jshort",
            ),
            "Int32" => (
                "java/lang/Integer",
                "intValue",
                "()I",
                "CallIntMethod",
                "(I)Ljava/lang/Integer;",
                "jint",
            ),
            "Int64" => (
                "java/lang/Long",
                "longValue",
                "()J",
                "CallLongMethod",
                "(J)Ljava/lang/Long;",
                "jlong",
            ),
            "UInt8" | "UInt16" | "UInt32" | "UInt64" => (
                "java/lang/Long",
                "longValue",
                "()J",
                "CallLongMethod",
                "(J)Ljava/lang/Long;",
                "jlong",
            ),
            "Float32" => (
                "java/lang/Float",
                "floatValue",
                "()F",
                "CallFloatMethod",
                "(F)Ljava/lang/Float;",
                "jfloat",
            ),
            "Float64" => (
                "java/lang/Double",
                "doubleValue",
                "()D",
                "CallDoubleMethod",
                "(D)Ljava/lang/Double;",
                "jdouble",
            ),
            _ => return None,
        };
    Some(AndroidBoxedPrimitive {
        class_name,
        unbox_method,
        unbox_signature,
        unbox_call,
        value_of_signature,
        jni_primitive,
    })
}

fn android_jni_map_boxed_primitive(ty: &TypeRef) -> Option<AndroidBoxedPrimitive> {
    let carrier_name = match ty.name.as_str() {
        "UInt8" => "Int8",
        "UInt16" => "Int16",
        "UInt32" => "Int32",
        "UInt64" => "Int64",
        name => name,
    };
    android_boxed_primitive(carrier_name)
}

fn render_android_jni_map_converter(
    out: &mut String,
    ty: &TypeRef,
    name: &str,
    to_java: bool,
    failure_return: &str,
) -> String {
    if let Some((key, value)) = android_cpp_map_type(ty) {
        let cpp_type = android_cpp_type(ty).expect("validated nested Android map type");
        let key_converter = render_android_jni_map_converter(
            out,
            key,
            &format!("{name}Key"),
            to_java,
            failure_return,
        );
        let value_converter = render_android_jni_map_converter(
            out,
            value,
            &format!("{name}Value"),
            to_java,
            failure_return,
        );
        return match (to_java, ty.optional) {
            (true, true) => format!(
                "[&](const {cpp_type}& value) -> jobject {{ if (!value.has_value()) return nullptr; return toJniMap(env, *value, {key_converter}, {value_converter}); }}"
            ),
            (false, true) => {
                let map_type = format!(
                    "std::map<{}, {}>",
                    android_cpp_type(key).expect("validated nested Android map key"),
                    android_cpp_type(value).expect("validated nested Android map value")
                );
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; return fromJniMap<{map_type}>(env, raw, {key_converter}, {value_converter}); }}"
                )
            }
            (true, false) => format!(
                "[&](const {cpp_type}& value) -> jobject {{ return toJniMap(env, value, {key_converter}, {value_converter}); }}"
            ),
            (false, false) => format!(
                "[&](jobject raw) -> {cpp_type} {{ return fromJniMap<{cpp_type}>(env, raw, {key_converter}, {value_converter}); }}"
            ),
        };
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let [element] = ty.arguments.as_slice() else {
            unreachable!("validated Android map collection has one element type")
        };
        let collection_type = android_cpp_type(ty)
            .expect("validated Android map collection has a C++ representation");
        if matches!(ty.name.as_str(), "Array" | "Set")
            && (matches!(element.name.as_str(), "Array" | "Set")
                || android_cpp_map_type(element).is_some()
                || element.optional)
        {
            let element_class = android_jni_class_descriptor(element)
                .expect("validated nested Android collection has a JNI descriptor");
            let converter = render_android_jni_map_converter(
                out,
                element,
                &format!("{name}Element"),
                to_java,
                failure_return,
            );
            let set_reserve = ty.name != "Set";
            let reserve = if set_reserve {
                "            output.reserve(static_cast<std::size_t>(length));\n"
            } else {
                ""
            };
            let null_check = if element.optional {
                ""
            } else {
                "                if (element.get() == nullptr) { throwIllegalState(env, \"non-null Nexa nested collection contained null\"); throw std::runtime_error(\"null JNI nested collection element\"); }\n"
            };
            let append = if ty.name == "Set" {
                "output.insert(std::move(value));"
            } else {
                "output.push_back(std::move(value));"
            };
            return if to_java {
                format!(
                    "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            ScopedLocalRef<jclass> elementClass(env, env->FindClass(\"{element_class}\"));\n            requireJniMapOperation(env, \"unable to resolve JNI nested collection element type\");\n            ScopedLocalRef<jobjectArray> output(env, env->NewObjectArray(length, elementClass.get(), nullptr));\n            requireJniMapOperation(env, \"unable to allocate JNI nested collection\");\n            jsize index = 0;\n            for (const auto& value : values) {{\n                ScopedLocalRef<jobject> element(env, {converter}(value));\n                requireJniMapOperation(env, \"unable to convert native nested collection element\");\n                env->SetObjectArrayElement(output.get(), index++, element.get());\n                requireJniMapOperation(env, \"unable to populate JNI nested collection\");\n            }}\n            return output.release();\n        }}"
                )
            } else {
                format!(
                    "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa nested collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI nested collection\"); }}\n            auto input = static_cast<jobjectArray>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI nested collection length\");\n            {collection_type} output;\n{reserve}            for (jsize index = 0; index < length; ++index) {{\n                ScopedLocalRef<jobject> element(env, env->GetObjectArrayElement(input, index));\n                requireJniMapOperation(env, \"unable to read JNI nested collection element\");\n{null_check}                auto value = {converter}(element.get());\n                requireJniMapOperation(env, \"unable to convert JNI nested collection element\");\n                {append}\n            }}\n            return output;\n        }}"
                )
            };
        }
        let Some(array) = android_primitive_array(element) else {
            android_reference_array_element(element)
                .expect("validated Android map collection has a reference element");
            let element_class = if element.name == "String" {
                "java/lang/String"
            } else {
                "[B"
            };
            if to_java {
                let to_jni = if element.name == "String" {
                    "toJniString"
                } else {
                    "toJniBytes"
                };
                return format!(
                    "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            ScopedLocalRef<jclass> elementClass(env, env->FindClass(\"{element_class}\"));\n            requireJniMapOperation(env, \"unable to resolve JNI map collection element type\");\n            ScopedLocalRef<jobjectArray> output(env, env->NewObjectArray(length, elementClass.get(), nullptr));\n            requireJniMapOperation(env, \"unable to allocate JNI map collection\");\n            jsize index = 0;\n            for (const auto& value : values) {{\n                ScopedLocalRef<jobject> element(env, {to_jni}(env, value));\n                requireJniMapOperation(env, \"unable to convert native map collection element\");\n                env->SetObjectArrayElement(output.get(), index++, element.get());\n                requireJniMapOperation(env, \"unable to populate JNI map collection\");\n            }}\n            return output.release();\n        }}"
                );
            }
            let from_jni = if element.name == "String" {
                "fromJniString"
            } else {
                "fromJniBytes"
            };
            let reference_type = if element.name == "String" {
                "jstring"
            } else {
                "jbyteArray"
            };
            let append = if ty.name == "Set" {
                "output.insert(std::move(value));"
            } else {
                "output.push_back(std::move(value));"
            };
            return format!(
                "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI map collection\"); }}\n            auto input = static_cast<jobjectArray>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI map collection length\");\n            {collection_type} output;\n            for (jsize index = 0; index < length; ++index) {{\n                ScopedLocalRef<jobject> element(env, env->GetObjectArrayElement(input, index));\n                requireJniMapOperation(env, \"unable to read JNI map collection element\");\n                if (element.get() == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection contained null\"); throw std::runtime_error(\"null JNI map collection element\"); }}\n                auto value = {from_jni}(env, static_cast<{reference_type}>(element.get()));\n                requireJniMapOperation(env, \"unable to convert JNI map collection element\");\n                {append}\n            }}\n            return output;\n        }}"
            );
        };
        let scalar = android_cpp_value(element)
            .expect("validated Android map collection has a native scalar element");
        if to_java {
            let carrier = if scalar.unsigned {
                format!(
                    "std::bit_cast<{}>(static_cast<{}>(element))",
                    array.element_jni, scalar.cpp
                )
            } else {
                format!("static_cast<{}>(element)", array.element_jni)
            };
            return format!(
                "[&](const {collection_type}& values) -> jobject {{\n            if (values.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin collection is too large for JNI\");\n            const auto length = static_cast<jsize>(values.size());\n            auto output = env->New{}Array(length);\n            if (output == nullptr) return nullptr;\n            std::vector<{}> carrier;\n            carrier.reserve(values.size());\n            for (const auto& element : values) carrier.push_back({carrier});\n            if (length > 0) env->{}(output, 0, length, carrier.data());\n            if (env->ExceptionCheck()) return nullptr;\n            return output;\n        }}",
                array.kotlin_array.trim_end_matches("Array"),
                array.element_jni,
                array.set_region,
            );
        }

        let converted = if scalar.unsigned {
            format!("std::bit_cast<{}>(carrier[index])", scalar.cpp)
        } else {
            format!("static_cast<{}>(carrier[index])", scalar.cpp)
        };
        let insertion = if ty.name == "Set" {
            format!("output.insert({converted});")
        } else {
            format!("output.push_back({converted});")
        };
        return format!(
            "[&](jobject raw) -> {collection_type} {{\n            if (raw == nullptr) {{ throwIllegalState(env, \"non-null Nexa map collection was null at the JNI boundary\"); throw std::runtime_error(\"null JNI map collection\"); }}\n            auto input = static_cast<{}>(raw);\n            const jsize length = env->GetArrayLength(input);\n            requireJniMapOperation(env, \"unable to read JNI map collection length\");\n            std::vector<{}> carrier(static_cast<std::size_t>(length));\n            if (length > 0) env->{}(input, 0, length, carrier.data());\n            requireJniMapOperation(env, \"unable to read JNI map collection values\");\n            {collection_type} output;\n            for (jsize index = 0; index < length; ++index) {{ {insertion} }}\n            return output;\n        }}",
            array.jni_array, array.element_jni, array.get_region,
        );
    }
    let scalar = android_cpp_value(ty).expect("validated Android map scalar");
    if ty.optional {
        let cpp_type = android_cpp_type(ty).expect("validated nullable map scalar");
        if matches!(ty.name.as_str(), "String" | "Bytes") {
            let (class, converter) = if ty.name == "String" {
                ("jstring", "String")
            } else {
                ("jbyteArray", "Bytes")
            };
            return if to_java {
                format!(
                    "[&](const {cpp_type}& value) -> jobject {{ if (!value) return nullptr; return toJni{converter}(env, *value); }}"
                )
            } else {
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; return fromJni{converter}(env, static_cast<{class}>(raw)); }}"
                )
            };
        }
        let boxed = android_boxed_primitive(&ty.name)
            .expect("validated nullable map primitive has a boxed JNI type");
        let class_name = format!("{name}Class");
        let method_name = format!("{name}Method");
        let (lookup, lambda) = if to_java {
            let carrier = if scalar.unsigned && ty.name == "UInt64" {
                "std::bit_cast<jlong>(static_cast<std::uint64_t>(*value))".to_owned()
            } else if scalar.unsigned {
                "static_cast<jlong>(*value)".to_owned()
            } else {
                format!("static_cast<{}>(*value)", boxed.jni_primitive)
            };
            (
                format!(
                    "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetStaticMethodID({class_name}, \"valueOf\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                    boxed.class_name, boxed.value_of_signature
                ),
                format!(
                    "[&](const {cpp_type}& value) -> jobject {{ if (!value) return nullptr; return env->CallStaticObjectMethod({class_name}, {method_name}, {carrier}); }}"
                ),
            )
        } else {
            (
                format!(
                    "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetMethodID({class_name}, \"{}\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                    boxed.class_name, boxed.unbox_method, boxed.unbox_signature
                ),
                format!(
                    "[&](jobject raw) -> {cpp_type} {{ if (raw == nullptr) return std::nullopt; auto carrier = env->{call}(raw, {method_name}); return static_cast<{cpp}>(carrier); }}",
                    call = boxed.unbox_call,
                    cpp = scalar.cpp
                ),
            )
        };
        out.push_str(&lookup);
        return lambda;
    }
    if ty.name == "String" {
        return if to_java {
            "[&](const std::string& value) -> jobject { return toJniString(env, value); }"
                .to_owned()
        } else {
            "[&](jobject raw) -> std::string { return fromJniString(env, static_cast<jstring>(raw)); }".to_owned()
        };
    }
    if ty.name == "Bytes" {
        return if to_java {
            "[&](const std::vector<std::uint8_t>& value) -> jobject { return toJniBytes(env, value); }"
                .to_owned()
        } else {
            "[&](jobject raw) -> std::vector<std::uint8_t> { return fromJniBytes(env, static_cast<jbyteArray>(raw)); }"
                .to_owned()
        };
    }
    let boxed = android_jni_map_boxed_primitive(ty).expect("validated boxed map scalar");
    let class_name = format!("{name}Class");
    let method_name = format!("{name}Method");
    let (lookup, lambda) = if to_java {
        let carrier = if scalar.unsigned {
            format!(
                "std::bit_cast<{}>(static_cast<{}>(value))",
                boxed.jni_primitive, scalar.cpp
            )
        } else {
            format!("static_cast<{}>(value)", boxed.jni_primitive)
        };
        (
            format!(
                "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetStaticMethodID({class_name}, \"valueOf\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                boxed.class_name, boxed.value_of_signature
            ),
            format!(
                "[&](const {cpp}& value) -> jobject {{ return env->CallStaticObjectMethod({class_name}, {method_name}, {carrier}); }}",
                cpp = scalar.cpp
            ),
        )
    } else {
        (
            format!(
                "        jclass {class_name} = env->FindClass(\"{}\");\n        if ({class_name} == nullptr) {failure_return}\n        jmethodID {method_name} = env->GetMethodID({class_name}, \"{}\", \"{}\");\n        if ({method_name} == nullptr) {failure_return}\n",
                boxed.class_name, boxed.unbox_method, boxed.unbox_signature
            ),
            format!(
                "[&](jobject raw) -> {cpp} {{ auto carrier = env->{call}(raw, {method_name}); return static_cast<{cpp}>(carrier); }}",
                cpp = scalar.cpp,
                call = boxed.unbox_call
            ),
        )
    };
    out.push_str(&lookup);
    lambda
}

fn render_jni_map_argument_conversion(
    out: &mut String,
    ty: &TypeRef,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let (key, map_value) = android_cpp_map_type(ty).expect("validated Android map type");
    let map_cpp_type = format!(
        "std::map<{}, {}>",
        android_cpp_type(key).expect("validated Android map key type"),
        android_cpp_type(map_value).expect("validated Android map value type")
    );
    let key_converter =
        render_android_jni_map_converter(out, key, &format!("{local}Key"), false, failure_return);
    let value_converter = render_android_jni_map_converter(
        out,
        map_value,
        &format!("{local}Value"),
        false,
        failure_return,
    );
    if ty.optional {
        out.push_str(&format!(
            "        std::optional<{map_cpp_type}> {local};\n        if ({value} != nullptr) {{\n            {local}.emplace(fromJniMap<{map_cpp_type}>(env, {value}, {key_converter}, {value_converter}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n",
        ));
    } else {
        out.push_str(&format!(
            "        auto {local} = fromJniMap<{map_cpp_type}>(env, {value}, {key_converter}, {value_converter});\n        if (env->ExceptionCheck()) {failure_return}\n",
        ));
    }
    format!("std::move({local})")
}

fn render_jni_optional_argument_conversion(
    out: &mut String,
    ty: &TypeRef,
    value: &str,
    failure_return: &str,
    local: &str,
) -> String {
    let scalar = android_cpp_value(ty).expect("validated optional JNI value");
    out.push_str(&format!("        std::optional<{}> {local};\n", scalar.cpp));
    match ty.name.as_str() {
        "String" => out.push_str(&format!(
            "        if ({value} != nullptr) {{\n            {local} = fromJniString(env, static_cast<jstring>({value}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
        )),
        "Bytes" => out.push_str(&format!(
            "        if ({value} != nullptr) {{\n            {local} = fromJniBytes(env, static_cast<jbyteArray>({value}));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n"
        )),
        _ => {
            let boxed = android_boxed_primitive(&ty.name)
                .expect("validated optional boxed primitive");
            out.push_str(&format!(
                "        if ({value} != nullptr) {{\n            jclass {local}Class = env->FindClass(\"{}\");\n            if ({local}Class == nullptr) {failure_return}\n            jmethodID {local}Unbox = env->GetMethodID({local}Class, \"{}\", \"{}\");\n            if ({local}Unbox == nullptr) {failure_return}\n            {local} = static_cast<{}>(env->{}({value}, {local}Unbox));\n            if (env->ExceptionCheck()) {failure_return}\n        }}\n",
                boxed.class_name,
                boxed.unbox_method,
                boxed.unbox_signature,
                scalar.cpp,
                boxed.unbox_call,
            ));
        }
    }
    format!("std::move({local})")
}

fn render_jni_return(out: &mut String, ty: &TypeRef, expression: &str) {
    if let Some((key, map_value)) = android_cpp_map_type(ty) {
        out.push_str("        return [&]() -> jobject {\n");
        let key_converter = render_android_jni_map_converter(
            out,
            key,
            "nexaJniMapReturnKey",
            true,
            "return nullptr;",
        );
        let value_converter = render_android_jni_map_converter(
            out,
            map_value,
            "nexaJniMapReturnValue",
            true,
            "return nullptr;",
        );
        if ty.optional {
            out.push_str(&format!(
                "            auto nexaJniOptionalMapReturn = {expression};\n            if (!nexaJniOptionalMapReturn.has_value()) return nullptr;\n            return toJniMap(env, *nexaJniOptionalMapReturn, {key_converter}, {value_converter});\n        }}();\n",
            ));
        } else {
            out.push_str(&format!(
                "            return toJniMap(env, {expression}, {key_converter}, {value_converter});\n        }}();\n",
            ));
        }
        return;
    }
    if matches!(ty.name.as_str(), "Array" | "Set") {
        let element = &ty.arguments[0];
        if (ty.name == "Array"
            && (matches!(element.name.as_str(), "Array" | "Set")
                || android_cpp_map_type(element).is_some()
                || element.optional))
            || (ty.name == "Set" && element.optional)
        {
            render_jni_nested_array_return(out, ty, expression);
            return;
        }
        let cpp_element = android_cpp_type(element).expect("validated collection element");
        if ty.name == "Set" {
            out.push_str(&format!(
                "        auto nexaArraySetValues = {expression};\n        std::vector<{cpp_element}> nexaArrayValues(nexaArraySetValues.begin(), nexaArraySetValues.end());\n"
            ));
        } else {
            out.push_str(&format!("        auto nexaArrayValues = {expression};\n"));
        }
        if android_primitive_array(element).is_none() {
            android_reference_array_element(element).expect("validated reference array element");
            let (element_class, conversion) = if element.name == "String" {
                ("java/lang/String", "toJniString")
            } else {
                ("[B", "toJniBytes")
            };
            out.push_str(&format!(
                "        if (nexaArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaArrayLength = static_cast<jsize>(nexaArrayValues.size());\n        jclass nexaArrayElementClass = env->FindClass(\"{element_class}\");\n        if (nexaArrayElementClass == nullptr) return nullptr;\n        auto nexaArrayOutput = env->NewObjectArray(nexaArrayLength, nexaArrayElementClass, nullptr);\n        if (nexaArrayOutput == nullptr) return nullptr;\n        for (jsize index = 0; index < nexaArrayLength; ++index) {{\n            auto nexaArrayElement = {conversion}(env, nexaArrayValues[index]);\n            if (env->ExceptionCheck()) return nullptr;\n            env->SetObjectArrayElement(nexaArrayOutput, index, nexaArrayElement);\n            if (nexaArrayElement != nullptr) env->DeleteLocalRef(nexaArrayElement);\n            if (env->ExceptionCheck()) return nullptr;\n        }}\n        env->DeleteLocalRef(nexaArrayElementClass);\n        return nexaArrayOutput;\n"
            ));
            return;
        }
        let array = android_primitive_array(element).expect("validated primitive array");
        out.push_str(&format!(
            "        if (nexaArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaArrayLength = static_cast<jsize>(nexaArrayValues.size());\n        auto nexaArrayOutput = env->New{}Array(nexaArrayLength);\n        if (nexaArrayOutput == nullptr) return nullptr;\n",
            array.kotlin_array.trim_end_matches("Array"),
        ));
        out.push_str(&format!(
            "        if (nexaArrayLength > 0) {{\n            std::vector<{}> nexaArrayCarrier(nexaArrayValues.size());\n            for (jsize index = 0; index < nexaArrayLength; ++index) nexaArrayCarrier[index] = static_cast<{}>(nexaArrayValues[index]);\n            env->{}(nexaArrayOutput, 0, nexaArrayLength, nexaArrayCarrier.data());\n            if (env->ExceptionCheck()) return nullptr;\n        }}\n        return nexaArrayOutput;\n",
            array.element_jni,
            array.element_jni,
            array.set_region,
        ));
        return;
    }
    let Some(scalar) = android_cpp_value(ty) else {
        let name = cpp_identifier(&ty.name);
        out.push_str(&format!(
            "        return nexaToJni{name}(env, {expression});\n"
        ));
        return;
    };
    if scalar.optional {
        out.push_str("        return [&]() -> jobject {\n");
        out.push_str(&format!(
            "            auto nexaOptionalValue = {expression};\n"
        ));
        out.push_str("            if (!nexaOptionalValue) return nullptr;\n");
        match ty.name.as_str() {
            "String" => out.push_str("            return toJniString(env, *nexaOptionalValue);\n"),
            "Bytes" => out.push_str("            return toJniBytes(env, *nexaOptionalValue);\n"),
            _ => {
                let boxed =
                    android_boxed_primitive(&ty.name).expect("validated optional boxed primitive");
                let value_expression = if scalar.unsigned && ty.name == "UInt64" {
                    "std::bit_cast<jlong>(static_cast<std::uint64_t>(*nexaOptionalValue))"
                        .to_owned()
                } else if scalar.unsigned {
                    "static_cast<jlong>(*nexaOptionalValue)".to_owned()
                } else {
                    format!("static_cast<{}>(*nexaOptionalValue)", boxed.jni_primitive)
                };
                out.push_str(&format!(
                    "            jclass nexaBoxClass = env->FindClass(\"{}\");\n            if (nexaBoxClass == nullptr) return nullptr;\n            jmethodID nexaValueOf = env->GetStaticMethodID(nexaBoxClass, \"valueOf\", \"{}\");\n            if (nexaValueOf == nullptr) return nullptr;\n            return env->CallStaticObjectMethod(nexaBoxClass, nexaValueOf, {value_expression});\n",
                    boxed.class_name,
                    boxed.value_of_signature,
                ));
            }
        }
        out.push_str("        }();\n");
    } else if scalar.jni == "jstring" {
        out.push_str(&format!("        return toJniString(env, {expression});\n"));
    } else if scalar.jni == "jbyteArray" {
        out.push_str(&format!("        return toJniBytes(env, {expression});\n"));
    } else if scalar.unsigned {
        out.push_str(&format!(
            "        return std::bit_cast<{}>(static_cast<{}>({expression}));\n",
            scalar.jni, scalar.cpp
        ));
    } else {
        out.push_str(&format!(
            "        return static_cast<{}>({expression});\n",
            scalar.jni
        ));
    }
}

fn render_jni_nested_array_return(out: &mut String, ty: &TypeRef, expression: &str) {
    let element = &ty.arguments[0];
    let element_class = android_jni_class_descriptor(element)
        .expect("validated nested array element has a JNI array class");
    if ty.name == "Set" {
        let element_type = android_cpp_type(element).expect("validated set element type");
        out.push_str(&format!(
            "        auto nexaNestedArraySetValues = {expression};\n        std::vector<{element_type}> nexaNestedArrayValues(nexaNestedArraySetValues.begin(), nexaNestedArraySetValues.end());\n"
        ));
    } else {
        out.push_str(&format!(
            "        auto nexaNestedArrayValues = {expression};\n"
        ));
    }
    out.push_str(&format!(
        "        if (nexaNestedArrayValues.size() > static_cast<std::size_t>(std::numeric_limits<jsize>::max())) throw std::length_error(\"native plugin array is too large for JNI\");\n        const auto nexaNestedArrayLength = static_cast<jsize>(nexaNestedArrayValues.size());\n        jclass nexaNestedArrayElementClass = env->FindClass(\"{element_class}\");\n        if (nexaNestedArrayElementClass == nullptr) return nullptr;\n        auto nexaNestedArrayOutput = env->NewObjectArray(nexaNestedArrayLength, nexaNestedArrayElementClass, nullptr);\n        env->DeleteLocalRef(nexaNestedArrayElementClass);\n        if (nexaNestedArrayOutput == nullptr) return nullptr;\n        for (jsize nexaNestedArrayIndex = 0; nexaNestedArrayIndex < nexaNestedArrayLength; ++nexaNestedArrayIndex) {{\n            auto nexaNestedArrayElement = [&]() -> jobject {{\n"
    ));
    render_jni_return(
        out,
        element,
        "nexaNestedArrayValues[static_cast<std::size_t>(nexaNestedArrayIndex)]",
    );
    out.push_str(
        "            }();\n            if (env->ExceptionCheck()) return nullptr;\n            env->SetObjectArrayElement(nexaNestedArrayOutput, nexaNestedArrayIndex, nexaNestedArrayElement);\n            if (nexaNestedArrayElement != nullptr) env->DeleteLocalRef(nexaNestedArrayElement);\n            if (env->ExceptionCheck()) return nullptr;\n        }\n        return nexaNestedArrayOutput;\n",
    );
}

fn android_jni_reference_class_available(ty: &TypeRef) -> bool {
    if android_boxed_primitive(&ty.name).is_some() && ty.optional {
        return true;
    }
    matches!(
        ty.name.as_str(),
        "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
            | "String"
            | "Bytes"
            | "Map"
            | "Array"
            | "Set"
    ) || (ty.arguments.is_empty() && android_cpp_value(ty).is_none())
}

fn android_jni_class_descriptor(ty: &TypeRef) -> Option<String> {
    if ty.optional {
        if let Some(boxed) = android_boxed_primitive(&ty.name) {
            return Some(format!("L{};", boxed.class_name));
        }
    }
    match ty.name.as_str() {
        "Bool" => Some("Z".to_owned()),
        "Int8" | "UInt8" => Some("B".to_owned()),
        "Int16" | "UInt16" => Some("S".to_owned()),
        "Int32" | "UInt32" => Some("I".to_owned()),
        "Int64" | "UInt64" => Some("J".to_owned()),
        "Float32" => Some("F".to_owned()),
        "Float64" => Some("D".to_owned()),
        "String" => Some("Ljava/lang/String;".to_owned()),
        "Bytes" => Some("[B".to_owned()),
        "Map" => Some("Ljava/util/Map;".to_owned()),
        "Array" => {
            let [element] = ty.arguments.as_slice() else {
                return None;
            };
            if android_primitive_array(element).is_some() {
                let descriptor = match element.name.as_str() {
                    "Bool" => "Z",
                    "Int8" | "UInt8" => "B",
                    "Int16" | "UInt16" => "S",
                    "Int32" | "UInt32" => "I",
                    "Int64" | "UInt64" => "J",
                    "Float32" => "F",
                    "Float64" => "D",
                    _ => return None,
                };
                Some(format!("[{descriptor}"))
            } else {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            }
        }
        "Set" => {
            let [element] = ty.arguments.as_slice() else {
                return None;
            };
            if android_primitive_array(element).is_some() {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            } else if android_reference_array_element(element).is_some() {
                Some(format!("[{}", android_jni_class_descriptor(element)?))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn jni_parameter_declarations(parameters: &[nexa_plugin_idl::Parameter]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} {}",
                android_jni_type(&parameter.ty).expect("validated parameter type"),
                parameter.name
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn jni_symbol(package: &str, class_name: &str, method_name: &str) -> String {
    let package = package.replace('.', "/");
    format!(
        "Java_{}_{}_{}",
        jni_mangle(&package),
        jni_mangle(class_name),
        jni_mangle(method_name)
    )
}

fn jni_mangle(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '/' => out.push('_'),
            '_' => out.push_str("_1"),
            ';' => out.push_str("_2"),
            '[' => out.push_str("_3"),
            character if character.is_ascii_alphanumeric() => out.push(character),
            character => out.push_str(&format!("_0{:04x}", character as u32)),
        }
    }
    out
}

fn validate_swift_cpp_method(
    idl: &PluginIdl,
    interface: &str,
    method: &Method,
) -> Result<(), String> {
    if let Some(error_type) = swift_cpp_method_error_type(method) {
        let Some(error) = idl
            .types
            .iter()
            .find(|ty| ty.kind == NamedTypeKind::Error && ty.name == error_type.name)
        else {
            return Err(format!(
                "C++ Swift adapters cannot resolve typed error `{}` for `{interface}.{}`",
                error_type.name, method.name
            ));
        };
        for case in &error.cases {
            for parameter in &case.parameters {
                if !swift_cpp_error_payload_supported(&parameter.ty) {
                    return Err(format!(
                        "C++ Swift typed errors support primitive, `String`, and `Bytes` payloads; `{interface}.{}` error case `{}.{}` uses `{}`",
                        method.name, error.name, case.name, parameter.ty.name
                    ));
                }
            }
        }
        for parameter in &method.parameters {
            ensure_swift_cpp_value(interface, &parameter.name, &parameter.ty, false)?;
        }
        return ensure_swift_cpp_value(
            interface,
            &method.name,
            swift_cpp_method_success_type(method),
            true,
        );
    }
    for parameter in &method.parameters {
        ensure_swift_cpp_value(interface, &parameter.name, &parameter.ty, false)?;
    }
    ensure_swift_cpp_value(
        interface,
        &method.name,
        swift_cpp_method_success_type(method),
        true,
    )
}

fn swift_cpp_method_error_type(method: &Method) -> Option<&TypeRef> {
    method.throws.as_ref().or_else(|| {
        (method.return_type.name == "Result")
            .then(|| method.return_type.arguments.get(1))
            .flatten()
    })
}

fn swift_cpp_method_success_type(method: &Method) -> &TypeRef {
    if method.return_type.name == "Result" {
        method
            .return_type
            .arguments
            .first()
            .expect("validated Result type has a success type")
    } else {
        &method.return_type
    }
}

fn swift_cpp_error_payload_supported(ty: &TypeRef) -> bool {
    matches!(
        ty.name.as_str(),
        "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
            | "String"
            | "Bytes"
    ) && ty.arguments.is_empty()
}

fn ensure_swift_cpp_value(
    interface: &str,
    member: &str,
    ty: &TypeRef,
    allow_void: bool,
) -> Result<(), String> {
    if swift_cpp_value_type(ty).is_some_and(|swift_type| allow_void || swift_type != "Void") {
        return Ok(());
    }
    Err(format!(
        "C++ Swift adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, and supported-key `Map` values; `{interface}.{member}` uses `{}`",
        ty.name
    ))
}

fn swift_cpp_value_type(ty: &TypeRef) -> Option<String> {
    if let Some((key, value)) = swift_cpp_map_types(ty) {
        return Some(format!(
            "[{}: {}]",
            swift_cpp_base_type(key)?,
            swift_cpp_value_type(value)?
        ));
    }
    if ty.name == "Set" {
        let element = swift_cpp_set_element_type(ty)?;
        return Some(format!("Set<{element}>"));
    }
    if ty.name == "Array" {
        return swift_cpp_array_swift_type(ty);
    }
    let base = swift_cpp_value_base_type(ty)?;
    if ty.optional {
        if base == "Void" {
            return None;
        }
        Some(format!("{base}?"))
    } else {
        Some(base.to_owned())
    }
}

fn swift_cpp_base_type(ty: &TypeRef) -> Option<&'static str> {
    if !ty.arguments.is_empty() {
        return None;
    }
    match ty.name.as_str() {
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

fn swift_cpp_value_base_type(ty: &TypeRef) -> Option<String> {
    if let Some(base) = swift_cpp_base_type(ty) {
        return Some(base.to_owned());
    }
    if ty.arguments.is_empty()
        && !matches!(
            ty.name.as_str(),
            "Array" | "Set" | "Map" | "Pair" | "Triple" | "Result"
        )
    {
        return Some(ty.name.clone());
    }
    None
}

fn swift_cpp_named_value_type<'a>(idl: &'a PluginIdl, ty: &TypeRef) -> Option<&'a NamedType> {
    if ty.optional || !ty.arguments.is_empty() {
        return None;
    }
    idl.types.iter().find(|named| {
        named.name == ty.name && matches!(named.kind, NamedTypeKind::Struct | NamedTypeKind::Enum)
    })
}

fn swift_cpp_set_element_type(ty: &TypeRef) -> Option<&'static str> {
    if ty.optional || ty.name != "Set" || ty.arguments.len() != 1 {
        return None;
    }
    let element = &ty.arguments[0];
    if element.optional || !element.arguments.is_empty() {
        return None;
    }
    match element.name.as_str() {
        "Bool" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32"
        | "UInt64" | "Bytes" => swift_cpp_base_type(element),
        // C++ ordering for floating point and UTF-8 strings does not match
        // Swift Set equality for NaN, signed zero, or canonically equivalent text.
        _ => None,
    }
}

fn swift_cpp_supported_set(ty: &TypeRef) -> bool {
    ty.name == "Set" && swift_cpp_set_element_type(ty).is_some()
}

fn swift_cpp_map_types(ty: &TypeRef) -> Option<(&TypeRef, &TypeRef)> {
    if ty.name != "Map" || ty.optional {
        return None;
    }
    let [key, value] = ty.arguments.as_slice() else {
        return None;
    };
    if key.optional
        || value.optional
        || swift_cpp_base_type(key).is_none()
        || !swift_cpp_map_value_supported(value)
        || !matches!(
            key.name.as_str(),
            "Bool"
                | "Int8"
                | "Int16"
                | "Int32"
                | "Int64"
                | "UInt8"
                | "UInt16"
                | "UInt32"
                | "UInt64"
                | "Bytes"
        )
    {
        return None;
    }
    Some((key, value))
}

fn swift_cpp_map_value_supported(ty: &TypeRef) -> bool {
    if ty.name == "Map" {
        return swift_cpp_map_types(ty).is_some();
    }
    if swift_cpp_supported_set(ty) {
        return true;
    }
    if let Some(base) = swift_cpp_value_base_type(ty) {
        return base != "Void";
    }
    swift_cpp_array_swift_type(ty).is_some()
}

fn swift_cpp_array_swift_type(ty: &TypeRef) -> Option<String> {
    if ty.optional || ty.name != "Array" || ty.arguments.len() != 1 {
        return None;
    }
    let element = &ty.arguments[0];
    if element.optional {
        return None;
    }
    if element.name == "Array" {
        swift_cpp_array_leaf(element)?;
        return Some(format!("[{}]", swift_cpp_array_swift_type(element)?));
    }
    if element.name == "Set" {
        return Some(format!("[{}]", swift_cpp_value_type(element)?));
    }
    let swift_type = swift_cpp_value_base_type(element)?;
    (swift_type != "Void").then(|| format!("[{swift_type}]"))
}

fn swift_cpp_array_leaf(ty: &TypeRef) -> Option<&TypeRef> {
    if ty.name != "Array" || ty.optional || ty.arguments.len() != 1 {
        return None;
    }
    let element = &ty.arguments[0];
    if element.optional {
        return None;
    }
    if element.name == "Array" {
        swift_cpp_array_leaf(element)
    } else if swift_cpp_value_base_type(element).is_some_and(|base| base != "Void") {
        Some(element)
    } else {
        None
    }
}

fn swift_cpp_nested_array_supported(ty: &TypeRef) -> bool {
    ty.name == "Array"
        && ty
            .arguments
            .first()
            .is_some_and(|element| matches!(element.name.as_str(), "Array" | "Set"))
        && swift_cpp_array_swift_type(ty).is_some()
}

fn swift_cpp_array_depth(ty: &TypeRef) -> usize {
    let mut depth = 0;
    let mut current = ty;
    while current.name == "Array" {
        depth += 1;
        let Some(element) = current.arguments.first() else {
            break;
        };
        current = element;
    }
    depth
}

fn swift_cpp_needs_vector_bridge(ty: &TypeRef) -> bool {
    swift_cpp_supported_set(ty)
        || swift_cpp_map_types(ty).is_some()
        || swift_cpp_nested_array_supported(ty)
        || (ty.name == "Array"
            && !ty.optional
            && ty.arguments.len() == 1
            && !ty.arguments[0].optional
            && ty.arguments[0].name == "Bool")
}

fn swift_cpp_method_uses_collection_adapter(method: &Method) -> bool {
    let success_type = swift_cpp_method_success_type(method);
    let values_are_supported = method
        .parameters
        .iter()
        .all(|parameter| swift_cpp_value_type(&parameter.ty).is_some())
        && swift_cpp_value_type(success_type).is_some();
    values_are_supported
        && (method
            .parameters
            .iter()
            .any(|parameter| swift_cpp_needs_vector_bridge(&parameter.ty))
            || swift_cpp_needs_vector_bridge(success_type))
}

fn swift_cpp_future_adapter_needed(method: &Method) -> bool {
    method.is_async && swift_cpp_needs_vector_bridge(swift_cpp_method_success_type(method))
}

fn swift_cpp_parameters(parameters: &[nexa_plugin_idl::Parameter]) -> String {
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
    parameters: &[nexa_plugin_idl::Parameter],
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
    optional_bridge: &str,
) -> String {
    parameters
        .iter()
        .map(|parameter| {
            swift_cpp_argument_expression(
                &parameter.ty,
                &parameter.name,
                idl,
                namespace,
                byte_buffer_type,
                optional_bridge,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn swift_cpp_argument_expression(
    ty: &TypeRef,
    value: &str,
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
    optional_bridge: &str,
) -> String {
    if !ty.optional {
        return swift_cpp_argument_value(ty, value, idl, namespace, byte_buffer_type);
    }
    let some = format!("{optional_bridge}.some{}", cpp_identifier(&ty.name));
    let none = format!("{optional_bridge}.none{}()", cpp_identifier(&ty.name));
    let converted = swift_cpp_argument_value(ty, "$0", idl, namespace, byte_buffer_type);
    format!("{value}.map {{ {some}({converted}) }} ?? {none}")
}

fn swift_cpp_argument_value(
    ty: &TypeRef,
    value: &str,
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
) -> String {
    match ty.name.as_str() {
        "Map" if swift_cpp_map_types(ty).is_some() => {
            let (key, value_type) = swift_cpp_map_types(ty).expect("validated Swift C++ map value");
            let entry = format!("{namespace}.{}", cpp_swift_map_entry_name(idl, ty));
            let entries = format!("{namespace}.{}", cpp_swift_map_entries_name(idl, ty));
            let key_value = swift_cpp_map_value_argument(key, "$0.key", idl, byte_buffer_type);
            let mapped_value =
                swift_cpp_argument_value(value_type, "$0.value", idl, namespace, byte_buffer_type);
            format!("{entries}({value}.map {{ {entry}({key_value}, {mapped_value}) }})")
        }
        "String" => format!("std.string({value})"),
        "Bytes" => format!("{byte_buffer_type}({value})"),
        "Array" if swift_cpp_nested_array_supported(ty) => {
            swift_cpp_array_argument_expression(ty, value, idl, namespace, byte_buffer_type)
        }
        "Array" => {
            let element = &ty.arguments[0];
            let converted = match element.name.as_str() {
                "Bool" => format!("{value}.map {{ UInt8($0 ? 1 : 0) }}"),
                "String" => format!("{value}.map {{ std.string($0) }}"),
                "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
                _ if swift_cpp_named_value_type(idl, element).is_some() => format!(
                    "{value}.map {{ nexaSwiftToCpp{}($0) }}",
                    cpp_identifier(&element.name)
                ),
                _ => value.to_owned(),
            };
            format!(
                "{namespace}.{}({converted})",
                cpp_swift_array_alias_name(idl, &element.name)
            )
        }
        "Set" => {
            let element = &ty.arguments[0];
            let converted = match element.name.as_str() {
                "Bool" => format!("Array({value}).map {{ UInt8($0 ? 1 : 0) }}"),
                "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
                _ => format!("Array({value})"),
            };
            format!(
                "{namespace}.{}({converted})",
                cpp_swift_array_alias_name(idl, &element.name)
            )
        }
        _ if swift_cpp_named_value_type(idl, ty).is_some() => {
            format!("nexaSwiftToCpp{}({value})", cpp_identifier(&ty.name))
        }
        _ => value.to_owned(),
    }
}

fn swift_cpp_array_argument_expression(
    ty: &TypeRef,
    value: &str,
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
) -> String {
    let element = &ty.arguments[0];
    let alias = format!(
        "{namespace}.{}",
        cpp_swift_array_alias_name_for_type(idl, ty)
    );
    let converted = if element.name == "Array" {
        let nested = swift_cpp_array_argument_expression(
            element,
            "nexaNestedArray",
            idl,
            namespace,
            byte_buffer_type,
        );
        format!("{value}.map {{ nexaNestedArray in {nested} }}")
    } else if element.name == "Set" {
        let nested = swift_cpp_argument_value(
            element,
            "nexaNestedCollection",
            idl,
            namespace,
            byte_buffer_type,
        );
        format!("{value}.map {{ nexaNestedCollection in {nested} }}")
    } else {
        match element.name.as_str() {
            "Bool" => format!("{value}.map {{ UInt8($0 ? 1 : 0) }}"),
            "String" => format!("{value}.map {{ std.string($0) }}"),
            "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
            _ if swift_cpp_named_value_type(idl, element).is_some() => format!(
                "{value}.map {{ nexaSwiftToCpp{}($0) }}",
                cpp_identifier(&element.name)
            ),
            _ => value.to_owned(),
        }
    };
    format!("{alias}({converted})")
}

fn swift_cpp_map_value_argument(
    ty: &TypeRef,
    value: &str,
    idl: &PluginIdl,
    byte_buffer_type: &str,
) -> String {
    match ty.name.as_str() {
        "String" => format!("std.string({value})"),
        "Bytes" => format!("{byte_buffer_type}({value})"),
        _ if swift_cpp_named_value_type(idl, ty).is_some() => {
            format!("nexaSwiftToCpp{}({value})", cpp_identifier(&ty.name))
        }
        _ => value.to_owned(),
    }
}

fn swift_cpp_result_expression(
    ty: &TypeRef,
    call: &str,
    idl: &PluginIdl,
    namespace: &str,
) -> String {
    if ty.optional {
        let converted = match ty.name.as_str() {
            "String" => "String($0)",
            "Bytes" => "Data($0)",
            _ if swift_cpp_named_value_type(idl, ty).is_some() => {
                return format!(
                    "Optional(fromCxx: {call}).map {{ nexaSwiftFromCpp{}($0) }}",
                    cpp_identifier(&ty.name)
                );
            }
            _ => "$0",
        };
        if converted == "$0" {
            format!("Optional(fromCxx: {call})")
        } else {
            format!("Optional(fromCxx: {call}).map {{ {converted} }}")
        }
    } else {
        match ty.name.as_str() {
            "Map" => {
                let (key, value_type) =
                    swift_cpp_map_types(ty).expect("validated Swift C++ map result");
                // Collection adapters already return the vector-backed entry
                // representation. Importing the std::map conversion helper
                // here would make Swift expect a std::map at the call site.
                let entries = format!("Array({call})");
                let key_value = swift_cpp_map_key_result(key, "$0.key");
                let mapped_value =
                    swift_cpp_result_expression(value_type, "$0.value", idl, namespace);
                format!(
                    "Dictionary(uniqueKeysWithValues: {entries}.map {{ ({key_value}, {mapped_value}) }})"
                )
            }
            "String" => format!("String({call})"),
            "Bytes" => format!("Data({call})"),
            "Array" if swift_cpp_nested_array_supported(ty) => {
                swift_cpp_array_result_expression(ty, call, idl, namespace)
            }
            "Array" => {
                let element = &ty.arguments[0];
                let copied = format!(
                    "Array({namespace}.{}({call}))",
                    cpp_swift_array_alias_name(idl, &element.name)
                );
                match element.name.as_str() {
                    "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
                    "String" => format!("{copied}.map {{ String($0) }}"),
                    "Bytes" => format!("{copied}.map {{ Data($0) }}"),
                    _ => copied,
                }
            }
            "Set" => {
                let element = &ty.arguments[0];
                let copied = format!(
                    "Array({namespace}.{}({call}))",
                    cpp_swift_array_alias_name(idl, &element.name)
                );
                let converted = match element.name.as_str() {
                    "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
                    "Bytes" => format!("{copied}.map {{ Data($0) }}"),
                    _ => copied,
                };
                format!("Set({converted})")
            }
            _ if swift_cpp_named_value_type(idl, ty).is_some() => {
                format!("nexaSwiftFromCpp{}({call})", cpp_identifier(&ty.name))
            }
            _ => call.to_owned(),
        }
    }
}

fn swift_cpp_array_result_expression(
    ty: &TypeRef,
    call: &str,
    idl: &PluginIdl,
    namespace: &str,
) -> String {
    let alias = cpp_swift_array_alias_name_for_type(idl, ty);
    let copied = format!("Array({namespace}.{alias}({call}))");
    let element = &ty.arguments[0];
    if matches!(element.name.as_str(), "Array" | "Set") {
        let nested = swift_cpp_result_expression(element, "nexaNestedCollection", idl, namespace);
        format!("{copied}.map {{ nexaNestedCollection in {nested} }}")
    } else {
        match element.name.as_str() {
            "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
            "String" => format!("{copied}.map {{ String($0) }}"),
            "Bytes" => format!("{copied}.map {{ Data($0) }}"),
            _ if swift_cpp_named_value_type(idl, element).is_some() => format!(
                "{copied}.map {{ nexaSwiftFromCpp{}($0) }}",
                cpp_identifier(&element.name)
            ),
            _ => copied,
        }
    }
}

fn swift_cpp_map_key_result(ty: &TypeRef, value: &str) -> String {
    match ty.name.as_str() {
        "String" => format!("String({value})"),
        "Bytes" => format!("Data({value})"),
        _ => value.to_owned(),
    }
}

fn render_swift_cpp_method(
    out: &mut String,
    receiver: &str,
    adapter_method: Option<&str>,
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
    optional_bridge: &str,
    method: &Method,
    depth: usize,
) {
    let indent = "    ".repeat(depth);
    let parameters = swift_cpp_parameters(&method.parameters);
    let arguments = swift_cpp_arguments(
        &method.parameters,
        idl,
        namespace,
        byte_buffer_type,
        optional_bridge,
    );
    let success_type = swift_cpp_method_success_type(method);
    let return_type = swift_cpp_value_type(success_type).expect("validated method return type");
    let call_on_worker = method.is_async && adapter_method.is_some();
    let async_modifier = if method.is_async { " async" } else { "" };
    let throws_modifier = swift_cpp_method_error_type(method)
        .map(|error| format!(" throws({})", error.name))
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
    if let Some(error_type) = swift_cpp_method_error_type(method) {
        render_swift_cpp_typed_error_call(
            out,
            method,
            success_type,
            &return_type,
            error_type,
            &call,
            call_on_worker,
            idl,
            namespace,
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
                swift_cpp_result_expression(success_type, "nexaCppFuture.get()", idl, namespace);
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
        let converted = swift_cpp_result_expression(success_type, &call, idl, namespace);
        out.push_str(&format!("{indent}    return {converted}\n"));
    }
    out.push_str(&format!("{indent}}}\n"));
}

#[allow(clippy::too_many_arguments)]
fn render_swift_cpp_typed_error_call(
    out: &mut String,
    method: &Method,
    success_type: &TypeRef,
    return_type: &str,
    error_type: &TypeRef,
    call: &str,
    call_on_worker: bool,
    idl: &PluginIdl,
    namespace: &str,
    depth: usize,
) {
    let indent = "    ".repeat(depth);
    let converter = swift_cpp_error_converter_name(&error_type.name);
    if method.is_async {
        let result_type = format!("Result<{return_type}, {}>", error_type.name);
        if !call_on_worker {
            out.push_str(&format!("{indent}    var nexaCppFuture = {call}\n"));
        }
        let work_body = if return_type == "Void" {
            format!(
                "{}var nexaCppTypedResult = nexaCppFuture.get(); if nexaCppTypedResult.has_value() {{ return .success(()) }}; return .failure({converter}(nexaCppTypedResult.nexaSwiftError()))",
                if call_on_worker {
                    format!("var nexaCppFuture = {call}; ")
                } else {
                    String::new()
                }
            )
        } else {
            let converted = swift_cpp_result_expression(
                success_type,
                "nexaCppTypedResult.nexaSwiftValue()",
                idl,
                namespace,
            );
            format!(
                "{}var nexaCppTypedResult = nexaCppFuture.get(); if nexaCppTypedResult.has_value() {{ return .success({converted}) }}; return .failure({converter}(nexaCppTypedResult.nexaSwiftError()))",
                if call_on_worker {
                    format!("var nexaCppFuture = {call}; ")
                } else {
                    String::new()
                }
            )
        };
        out.push_str(&format!(
            "{indent}    let nexaCppFutureWork = NexaCppFutureWork<{result_type}> {{ {work_body} }}\n{indent}    let nexaCppTypedResult = await withCheckedContinuation {{ (continuation: CheckedContinuation<{result_type}, Never>) in\n{indent}        DispatchQueue.global(qos: .userInitiated).async {{\n{indent}            continuation.resume(returning: nexaCppFutureWork.run())\n{indent}        }}\n{indent}    }}\n{indent}    return try nexaCppTypedResult.get()\n"
        ));
        return;
    }

    out.push_str(&format!("{indent}    var nexaCppTypedResult = {call}\n"));
    out.push_str(&format!(
        "{indent}    guard nexaCppTypedResult.has_value() else {{ throw {converter}(nexaCppTypedResult.nexaSwiftError()) }}\n"
    ));
    if return_type == "Void" {
        return;
    }
    let converted = swift_cpp_result_expression(
        success_type,
        "nexaCppTypedResult.nexaSwiftValue()",
        idl,
        namespace,
    );
    out.push_str(&format!("{indent}    return {converted}\n"));
}

fn render_result_type(out: &mut String) {
    out.push_str(
        "template <class Success, class Failure>\nclass NexaResult {\npublic:\n    static NexaResult success(Success value) {\n        return NexaResult(Storage{std::in_place_index<0>, std::move(value)});\n    }\n    static NexaResult failure(Failure error) {\n        return NexaResult(Storage{std::in_place_index<1>, std::move(error)});\n    }\n    bool has_value() const noexcept { return storage_.index() == 0; }\n    Success& value() & { return std::get<0>(storage_); }\n    const Success& value() const & { return std::get<0>(storage_); }\n    Failure& error() & { return std::get<1>(storage_); }\n    const Failure& error() const & { return std::get<1>(storage_); }\n    Success nexaSwiftValue() { return std::move(std::get<0>(storage_)); }\n    Failure nexaSwiftError() { return std::move(std::get<1>(storage_)); }\nprivate:\n    using Storage = std::variant<Success, Failure>;\n    explicit NexaResult(Storage storage) : storage_(std::move(storage)) {}\n    Storage storage_;\n};\n\ntemplate <class Failure>\nclass NexaResult<void, Failure> {\npublic:\n    static NexaResult success() { return NexaResult(std::nullopt); }\n    static NexaResult failure(Failure error) {\n        return NexaResult(std::optional<Failure>(std::move(error)));\n    }\n    bool has_value() const noexcept { return !error_.has_value(); }\n    Failure& error() & { return *error_; }\n    const Failure& error() const & { return *error_; }\n    Failure nexaSwiftError() { return std::move(*error_); }\nprivate:\n    explicit NexaResult(std::optional<Failure> error) : error_(std::move(error)) {}\n    std::optional<Failure> error_;\n};\n\n",
    );
}

fn render_optional_bridge(out: &mut String, idl: &PluginIdl) {
    let optional_types = optional_swift_cpp_types(idl);
    if optional_types.is_empty() {
        return;
    }

    for name in &optional_types {
        let ty = TypeRef {
            name: name.clone(),
            arguments: Vec::new(),
            optional: false,
        };
        out.push_str(&format!(
            "using {} = std::optional<{}>;\n",
            cpp_optional_alias_name(idl, name),
            cpp_type(&ty)
        ));
    }
    out.push_str(&format!("\nstruct {} {{\n", cpp_optional_bridge_name(idl)));
    for name in optional_types {
        let ty = TypeRef {
            name: name.clone(),
            arguments: Vec::new(),
            optional: false,
        };
        let alias = cpp_optional_alias_name(idl, &name);
        let cpp_type = cpp_type(&ty);
        out.push_str(&format!(
            "    static {alias} some{name}({cpp_type} value) noexcept {{ return value; }}\n    static {alias} none{name}() noexcept {{ return std::nullopt; }}\n"
        ));
    }
    out.push_str("};\n\n");
}

fn render_swift_array_aliases(out: &mut String, idl: &PluginIdl) {
    let mut arrays = Vec::new();
    let mut add = |ty: &TypeRef| {
        let array = if ty.name == "Set" && swift_cpp_supported_set(ty) {
            TypeRef {
                name: "Array".to_owned(),
                arguments: ty.arguments.clone(),
                optional: false,
            }
        } else {
            ty.clone()
        };
        if swift_cpp_array_swift_type(&array).is_some()
            && !arrays.iter().any(|existing| existing == &array)
        {
            arrays.push(array.clone());
        }
        if array.name == "Array"
            && array
                .arguments
                .first()
                .is_some_and(|element| element.name == "Array")
        {
            let mut nested = array.arguments[0].clone();
            while nested.name == "Array" {
                if swift_cpp_array_swift_type(&nested).is_some()
                    && !arrays.iter().any(|existing| existing == &nested)
                {
                    arrays.push(nested.clone());
                }
                if nested
                    .arguments
                    .first()
                    .is_some_and(|element| element.name == "Array")
                {
                    nested = nested.arguments[0].clone();
                } else {
                    break;
                }
            }
        }
    };
    for interface in &idl.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
        for property in &interface.properties {
            collect_swift_array_types(&property.ty, &mut add);
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
            collect_swift_array_types(&method.return_type, &mut add);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
    }
    arrays.sort_by_key(|array| {
        let depends_on_set_facade = array
            .arguments
            .first()
            .is_some_and(|element| element.name == "Set");
        (swift_cpp_array_depth(array), depends_on_set_facade)
    });
    for array in arrays {
        out.push_str(&format!(
            "using {} = {};\n",
            cpp_swift_array_alias_name_for_type(idl, &array),
            cpp_swift_array_facade_type(&array, idl)
        ));
    }
    let mut bool_facades = arrays_with_bool_facades(idl);
    bool_facades.sort_by_key(swift_cpp_array_depth);
    for array in bool_facades {
        render_swift_array_conversion_adapters(out, idl, &array);
    }
    let mut set_facades = arrays_with_nested_set_facades(idl);
    set_facades.sort_by_key(swift_cpp_array_depth);
    for array in set_facades {
        render_swift_array_conversion_adapters(out, idl, &array);
    }
    if !out.ends_with("\n\n") && out.ends_with('\n') {
        out.push('\n');
    }
}

fn cpp_swift_array_facade_type(ty: &TypeRef, idl: &PluginIdl) -> String {
    let element = &ty.arguments[0];
    if element.name == "Array" {
        format!("std::vector<{}>", cpp_swift_array_facade_type(element, idl))
    } else if element.name == "Set" {
        format!(
            "std::vector<{}>",
            cpp_swift_array_alias_name(idl, &element.arguments[0].name)
        )
    } else if element.name == "Bool" {
        "std::vector<std::uint8_t>".to_owned()
    } else {
        format!("std::vector<{}>", cpp_type(element))
    }
}

fn arrays_with_bool_facades(idl: &PluginIdl) -> Vec<TypeRef> {
    let mut arrays = Vec::new();
    let mut add = |ty: &TypeRef| {
        if swift_cpp_array_swift_type(ty).is_some()
            && swift_cpp_array_leaf(ty).is_some_and(|leaf| leaf.name == "Bool")
            && !arrays.iter().any(|existing| existing == ty)
        {
            arrays.push(ty.clone());
        }
    };
    for interface in &idl.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
        for property in &interface.properties {
            collect_swift_array_types(&property.ty, &mut add);
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
            collect_swift_array_types(&method.return_type, &mut add);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
    }
    arrays
}

fn arrays_with_nested_set_facades(idl: &PluginIdl) -> Vec<TypeRef> {
    let mut arrays = Vec::new();
    let mut add = |ty: &TypeRef| {
        if swift_cpp_nested_array_supported(ty)
            && ty.arguments[0].name == "Set"
            && !arrays.iter().any(|existing| existing == ty)
        {
            arrays.push(ty.clone());
        }
    };
    for interface in &idl.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
        for property in &interface.properties {
            collect_swift_array_types(&property.ty, &mut add);
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
            collect_swift_array_types(&method.return_type, &mut add);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                collect_swift_array_types(&parameter.ty, &mut add);
            }
        }
    }
    arrays
}

fn collect_swift_array_types(ty: &TypeRef, add: &mut impl FnMut(&TypeRef)) {
    if ty.name == "Set" && swift_cpp_supported_set(ty) {
        let array = TypeRef {
            name: "Array".to_owned(),
            arguments: ty.arguments.clone(),
            optional: false,
        };
        add(&array);
    } else if ty.name == "Array" && swift_cpp_array_swift_type(ty).is_some() {
        add(ty);
    }
    if ty.name == "Array" {
        if let Some(element) = ty
            .arguments
            .first()
            .filter(|element| matches!(element.name.as_str(), "Array" | "Set"))
        {
            collect_swift_array_types(element, add);
        }
    } else if ty.name == "Map" {
        for argument in &ty.arguments {
            collect_swift_array_types(argument, add);
        }
    }
}

fn render_swift_array_conversion_adapters(out: &mut String, idl: &PluginIdl, ty: &TypeRef) {
    let native_type = cpp_type(ty);
    let facade_type = cpp_swift_array_alias_name_for_type(idl, ty);
    let to_native_name = cpp_swift_array_conversion_name(idl, ty, "ToNative");
    let from_native_name = cpp_swift_array_conversion_name(idl, ty, "FromNative");
    out.push_str(&format!(
        "inline {native_type} {to_native_name}({facade_type} value) noexcept {{\n    {native_type} result;\n    result.reserve(value.size());\n    for (auto&& element : value) result.push_back({});\n    return result;\n}}\ninline {facade_type} {from_native_name}({native_type} value) noexcept {{\n    {facade_type} result;\n    result.reserve(value.size());\n    for (auto&& element : value) result.push_back({});\n    return result;\n}}\n\n",
        cpp_swift_array_convert_element(idl, ty, "element", true),
        cpp_swift_array_convert_element(idl, ty, "element", false),
    ));
}

fn cpp_swift_array_convert_element(
    idl: &PluginIdl,
    ty: &TypeRef,
    element: &str,
    to_native: bool,
) -> String {
    let nested = &ty.arguments[0];
    if nested.name == "Array" {
        let direction = if to_native { "ToNative" } else { "FromNative" };
        format!(
            "{}(std::move({element}))",
            cpp_swift_array_conversion_name(idl, nested, direction)
        )
    } else if nested.name == "Set" {
        let element_type = cpp_type(&nested.arguments[0]);
        if to_native {
            format!("std::set<{element_type}>({element}.begin(), {element}.end())")
        } else {
            format!(
                "{}({element}.begin(), {element}.end())",
                cpp_swift_array_alias_name(idl, &nested.arguments[0].name)
            )
        }
    } else if to_native {
        format!("static_cast<bool>({element})")
    } else {
        format!("static_cast<std::uint8_t>({element})")
    }
}

fn cpp_swift_array_conversion_name(idl: &PluginIdl, ty: &TypeRef, direction: &str) -> String {
    let alias = cpp_swift_array_alias_name_for_type(idl, ty);
    let signature = alias.trim_start_matches("NexaCppArray");
    format!("nexaCppArray{direction}{signature}")
}

fn render_swift_map_adapters(out: &mut String, idl: &PluginIdl) {
    let mut maps = Vec::new();
    for interface in &idl.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                collect_swift_map_types(&parameter.ty, &mut maps);
            }
        }
        for property in &interface.properties {
            collect_swift_map_types(&property.ty, &mut maps);
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                collect_swift_map_types(&parameter.ty, &mut maps);
            }
            collect_swift_map_types(&method.return_type, &mut maps);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                collect_swift_map_types(&parameter.ty, &mut maps);
            }
        }
    }

    for ty in maps {
        let (key, value) = swift_cpp_map_types(&ty).expect("collected map type is supported");
        let entry = cpp_swift_map_entry_name(idl, &ty);
        let entries = cpp_swift_map_entries_name(idl, &ty);
        let map_type = cpp_type(&ty);
        let key_type = cpp_type(key);
        let value_type = cpp_swift_collection_bridge_type(value, idl);
        let from_entries = cpp_swift_map_conversion_name(idl, &ty, "FromEntries");
        let to_entries = cpp_swift_map_conversion_name(idl, &ty, "ToEntries");
        let native_value = cpp_swift_collection_argument(value, "entry.value", idl);
        let bridge_value = cpp_swift_map_value_for_entry(value, "mappedValue", idl);
        out.push_str(&format!(
            "struct {entry} {{\n    {key_type} key;\n    {value_type} value;\n    {entry}({key_type} keyValue, {value_type} mappedValue) : key(std::move(keyValue)), value(std::move(mappedValue)) {{}}\n}};\nusing {entries} = std::vector<{entry}>;\ninline {map_type} {from_entries}({entries} entries) noexcept {{\n    {map_type} result;\n    for (auto& entry : entries) result.emplace(std::move(entry.key), {native_value});\n    return result;\n}}\ninline {entries} {to_entries}({map_type} value) noexcept {{\n    {entries} entries;\n    entries.reserve(value.size());\n    for (auto& [key, mappedValue] : value) entries.emplace_back(std::move(key), {bridge_value});\n    return entries;\n}}\n\n"
        ));
    }
}

fn collect_swift_map_types(ty: &TypeRef, maps: &mut Vec<TypeRef>) {
    for argument in &ty.arguments {
        collect_swift_map_types(argument, maps);
    }
    if swift_cpp_map_types(ty).is_some() && !maps.iter().any(|existing| existing == ty) {
        maps.push(ty.clone());
    }
}

fn cpp_swift_map_value_for_entry(ty: &TypeRef, value: &str, idl: &PluginIdl) -> String {
    match ty.name.as_str() {
        "Map" => format!(
            "{}({value})",
            cpp_swift_map_conversion_name(idl, ty, "ToEntries")
        ),
        "Array" if swift_cpp_nested_array_supported(ty) && ty.arguments[0].name == "Set" => {
            format!(
                "{}({value})",
                cpp_swift_array_conversion_name(idl, ty, "FromNative")
            )
        }
        "Set" => {
            let facade = cpp_swift_collection_bridge_type(ty, idl);
            format!("{facade}({value}.begin(), {value}.end())")
        }
        _ => format!("std::move({value})"),
    }
}

fn cpp_swift_map_signature(ty: &TypeRef) -> String {
    ty.arguments
        .iter()
        .map(cpp_swift_type_signature)
        .collect::<String>()
}

fn cpp_swift_type_signature(ty: &TypeRef) -> String {
    let mut signature = cpp_identifier(&ty.name);
    for argument in &ty.arguments {
        signature.push_str(&cpp_swift_type_signature(argument));
    }
    if ty.optional {
        signature.push_str("Optional");
    }
    signature
}

fn cpp_swift_map_entry_name(idl: &PluginIdl, ty: &TypeRef) -> String {
    unique_cpp_type_name(
        idl,
        &format!("NexaCppMapEntry{}", cpp_swift_map_signature(ty)),
    )
}

fn cpp_swift_map_entries_name(idl: &PluginIdl, ty: &TypeRef) -> String {
    unique_cpp_type_name(
        idl,
        &format!("NexaCppMapEntries{}", cpp_swift_map_signature(ty)),
    )
}

fn cpp_swift_map_conversion_name(idl: &PluginIdl, ty: &TypeRef, direction: &str) -> String {
    format!(
        "nexaCppMap{direction}{}",
        cpp_swift_map_entries_name(idl, ty).trim_start_matches("NexaCppMapEntries")
    )
}

fn render_named_type(out: &mut String, ty: &NamedType) {
    let name = cpp_identifier(&ty.name);
    match ty.kind {
        NamedTypeKind::Struct => {
            out.push_str(&format!("struct {name} {{\n"));
            for field in &ty.fields {
                out.push_str(&format!(
                    "    {} {}{};\n",
                    cpp_type(&field.ty),
                    cpp_identifier(&field.name),
                    field
                        .default
                        .as_ref()
                        .map(|value| format!(" = {}", cpp_literal(value)))
                        .unwrap_or_default()
                ));
            }
            out.push_str("};\n");
        }
        NamedTypeKind::Enum => {
            out.push_str(&format!("enum class {name} : std::uint8_t {{\n    "));
            out.push_str(
                &ty.cases
                    .iter()
                    .map(|case| cpp_identifier(&case.name))
                    .collect::<Vec<_>>()
                    .join(",\n    "),
            );
            out.push_str("\n};\n");
        }
        NamedTypeKind::Error => {
            out.push_str(&format!("struct {name} {{\n"));
            for case in &ty.cases {
                let case_name = cpp_case_type_name(&case.name);
                out.push_str(&format!("    struct {case_name} {{"));
                if case.parameters.is_empty() {
                    out.push_str("};\n");
                } else {
                    out.push('\n');
                    for parameter in &case.parameters {
                        out.push_str(&format!(
                            "        {} {};\n",
                            cpp_type(&parameter.ty),
                            cpp_identifier(&parameter.name)
                        ));
                    }
                    out.push_str("    };\n");
                }
            }
            out.push_str("    using Value = std::variant<");
            out.push_str(
                &ty.cases
                    .iter()
                    .map(|case| format!("{}", cpp_case_type_name(&case.name)))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str(">;\n    Value value;\n};\n");
        }
    }
}

fn render_swift_cpp_error_bridges(out: &mut String, idl: &PluginIdl) {
    let error_names = idl
        .interfaces
        .iter()
        .filter(|interface| {
            matches!(
                interface.kind,
                InterfaceKind::Service | InterfaceKind::NativeClass
            )
        })
        .flat_map(|interface| &interface.methods)
        .filter_map(swift_cpp_method_error_type)
        .map(|ty| ty.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    for error_name in error_names {
        let error = idl
            .types
            .iter()
            .find(|ty| ty.kind == NamedTypeKind::Error && ty.name == error_name)
            .expect("typed errors are validated by the plugin IDL parser");
        let bridge = cpp_swift_error_bridge_name(idl, error_name);
        let name = cpp_identifier(error_name);
        out.push_str(&format!("struct {bridge} {{\n    static std::uint8_t caseIndex(const {name}& error) noexcept {{ return static_cast<std::uint8_t>(error.value.index()); }}\n"));
        for (case_index, case) in error.cases.iter().enumerate() {
            let case_type = cpp_case_type_name(&case.name);
            for (parameter_index, parameter) in case.parameters.iter().enumerate() {
                out.push_str(&format!(
                    "    static {} payload_{case_index}_{parameter_index}(const {name}& error) {{ return std::get<{name}::{case_type}>(error.value).{}; }}\n",
                    cpp_type(&parameter.ty),
                    cpp_identifier(&parameter.name)
                ));
            }
        }
        out.push_str("};\n\n");
    }
}

fn render_swift_cpp_named_value_helpers(
    out: &mut String,
    idl: &PluginIdl,
    namespace: &str,
    byte_buffer_type: &str,
) {
    let optional_bridge = format!("{namespace}.{}", cpp_optional_bridge_name(idl));
    for ty in idl
        .types
        .iter()
        .filter(|ty| matches!(ty.kind, NamedTypeKind::Struct | NamedTypeKind::Enum))
    {
        let name = cpp_identifier(&ty.name);
        let cxx_type = format!("{namespace}.{name}");
        match ty.kind {
            NamedTypeKind::Enum => {
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
            NamedTypeKind::Struct => {
                let swift_fields = ty
                    .fields
                    .iter()
                    .map(|field| {
                        let cpp_field = cpp_identifier(&field.name);
                        let converted = swift_cpp_result_expression(
                            &field.ty,
                            &format!("value.{cpp_field}"),
                            idl,
                            namespace,
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
                        idl,
                        namespace,
                        byte_buffer_type,
                        &optional_bridge,
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
            NamedTypeKind::Error => unreachable!(),
        }
    }
}

fn render_swift_cpp_error_converters(out: &mut String, idl: &PluginIdl, namespace: &str) {
    let error_names = idl
        .interfaces
        .iter()
        .filter(|interface| {
            matches!(
                interface.kind,
                InterfaceKind::Service | InterfaceKind::NativeClass
            )
        })
        .flat_map(|interface| &interface.methods)
        .filter_map(swift_cpp_method_error_type)
        .map(|ty| ty.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    for error_name in error_names {
        let Some(error) = idl
            .types
            .iter()
            .find(|ty| ty.kind == NamedTypeKind::Error && ty.name == error_name)
        else {
            continue;
        };
        let bridge = format!(
            "{namespace}.{}",
            cpp_swift_error_bridge_name(idl, error_name)
        );
        out.push_str(&format!(
            "private func {}(_ error: {namespace}.{error_name}) -> {error_name} {{\n    switch {bridge}.caseIndex(error) {{\n",
            swift_cpp_error_converter_name(error_name)
        ));
        for (case_index, case) in error.cases.iter().enumerate() {
            if case.parameters.is_empty() {
                out.push_str(&format!(
                    "    case {case_index}: return {error_name}.{}\n",
                    case.name
                ));
                continue;
            }
            let payload = case
                .parameters
                .iter()
                .enumerate()
                .map(|(parameter_index, parameter)| {
                    let bridge_call =
                        format!("{bridge}.payload_{case_index}_{parameter_index}(error)");
                    let value =
                        swift_cpp_result_expression(&parameter.ty, &bridge_call, idl, namespace);
                    format!("{}: {value}", parameter.name)
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "    case {case_index}: return {error_name}.{}({payload})\n",
                case.name
            ));
        }
        out.push_str(
            "    default: preconditionFailure(\"C++ error variant is outside the declared contract\")\n    }\n}\n\n",
        );
    }
}

fn render_contract(out: &mut String, interface: &Interface, idl: &PluginIdl) {
    let name = cpp_identifier(&interface.name);
    out.push_str(&format!(
        "class {name}Spec {{\npublic:\n    void nexaRetainForSwift() const noexcept {{ swift_references_.fetch_add(1, std::memory_order_relaxed); }}\n    void nexaReleaseFromSwift() const noexcept {{\n        if (swift_references_.fetch_sub(1, std::memory_order_acq_rel) == 1) delete this;\n    }}\n    virtual ~{name}Spec() = default;\n"
    ));
    for property in &interface.properties {
        render_getter(out, property);
        if property.mutable {
            render_setter(out, property);
        }
    }
    for method in &interface.methods {
        out.push_str("    virtual ");
        out.push_str(&cpp_method_return(method));
        out.push(' ');
        out.push_str(&cpp_identifier(&method.name));
        out.push('(');
        out.push_str(&cpp_parameters(&method.parameters));
        out.push(')');
        // C++ implementations report declared failures as values so native
        // exceptions cannot escape across Swift or JNI boundaries.
        out.push_str(" noexcept = 0;\n");
    }
    for property in &interface.properties {
        if swift_cpp_needs_vector_bridge(&property.ty) {
            render_cpp_swift_collection_property_adapter(out, interface, property, idl);
        }
    }
    for method in &interface.methods {
        if swift_cpp_method_uses_collection_adapter(method) {
            render_cpp_swift_collection_method_adapter(out, interface, method, idl);
        }
    }
    for event in &interface.events {
        render_event_setter(out, event);
        render_cpp_swift_event_setter(out, idl, interface, event);
    }
    out.push_str(
        "private:\n    mutable std::atomic<std::size_t> swift_references_{1};\n} NEXA_CXX_SWIFT_SHARED_REFERENCE(.nexaRetainForSwift, .nexaReleaseFromSwift);\n",
    );
    for event in &interface.events {
        for (index, parameter) in event.parameters.iter().enumerate() {
            render_cpp_swift_event_copy_adapter(out, idl, interface, event, index, &parameter.ty);
        }
    }
}

fn render_cpp_swift_event_setter(
    out: &mut String,
    idl: &PluginIdl,
    interface: &Interface,
    event: &Event,
) {
    let callback_name = cpp_swift_event_callback_type_name(idl, interface, event);
    let release_name = cpp_swift_event_release_type_name(idl, interface, event);
    let setter = cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name));
    let method = cpp_swift_event_setter_name(interface, event);
    let parameters = event
        .parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            format!("const {}& nexaEventValue{index}", cpp_type(&parameter.ty))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let callback_parameters = std::iter::once("void* NEXA_CXX_SWIFT_NONNULL".to_owned())
        .chain((0..event.parameters.len()).map(|_| "const void* NEXA_CXX_SWIFT_NONNULL".to_owned()))
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "    using {callback_name} = void (*)({callback_parameters});\n    using {release_name} = void (*)(void* NEXA_CXX_SWIFT_NONNULL);\n    void {method}({callback_name} NEXA_CXX_SWIFT_NULLABLE callback, void* NEXA_CXX_SWIFT_NULLABLE context, {release_name} NEXA_CXX_SWIFT_NULLABLE release) noexcept {{\n        if (callback == nullptr) {{\n            {setter}({{}});\n            if (context != nullptr && release != nullptr) release(context);\n            return;\n        }}\n        if (context == nullptr || release == nullptr) {{\n            {setter}({{}});\n            if (context != nullptr && release != nullptr) release(context);\n            return;\n        }}\n        NexaCppSwiftEventContextHandle retainedContext(context, release);\n        {setter}([retainedContext = std::move(retainedContext), callback]({parameters}) noexcept {{\n            callback(retainedContext.get(){});\n        }});\n    }}\n",
        if event.parameters.is_empty() {
            String::new()
        } else {
            (0..event.parameters.len())
                .map(|index| format!(", static_cast<const void*>(&nexaEventValue{index})"))
                .collect::<String>()
        }
    ));
}

fn render_cpp_swift_event_copy_adapter(
    out: &mut String,
    idl: &PluginIdl,
    interface: &Interface,
    event: &Event,
    index: usize,
    ty: &TypeRef,
) {
    let name = cpp_swift_event_copy_adapter_name(idl, interface, event, index);
    let return_type = cpp_swift_collection_bridge_type(ty, idl);
    let value = format!("*static_cast<const {}*>(rawValue)", cpp_type(ty));
    let converted = if swift_cpp_map_types(ty).is_some() {
        format!(
            "{}({value})",
            cpp_swift_map_conversion_name(idl, ty, "ToEntries")
        )
    } else if ty.name == "Array" && swift_cpp_array_leaf(ty).is_some_and(|leaf| leaf.name == "Bool")
    {
        format!(
            "{}({value})",
            cpp_swift_array_conversion_name(idl, ty, "FromNative")
        )
    } else if ty.name == "Array"
        && swift_cpp_nested_array_supported(ty)
        && ty.arguments[0].name == "Set"
    {
        format!(
            "{}({value})",
            cpp_swift_array_conversion_name(idl, ty, "FromNative")
        )
    } else if swift_cpp_needs_vector_bridge(ty) {
        format!("{return_type}({value}.begin(), {value}.end())")
    } else {
        value
    };
    out.push_str(&format!(
        "inline {return_type} {name}(const void* NEXA_CXX_SWIFT_NONNULL rawValue) noexcept {{ return {converted}; }}\n"
    ));
}

fn cpp_swift_event_callback_type_name(
    idl: &PluginIdl,
    interface: &Interface,
    event: &Event,
) -> String {
    unique_cpp_type_name(
        idl,
        &format!(
            "NexaCppSwiftEventCallback{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

fn cpp_swift_event_release_type_name(
    idl: &PluginIdl,
    interface: &Interface,
    event: &Event,
) -> String {
    unique_cpp_type_name(
        idl,
        &format!(
            "NexaCppSwiftEventRelease{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

fn cpp_swift_event_setter_name(interface: &Interface, event: &Event) -> String {
    let base = format!("nexaSwiftSetEvent{}", cpp_identifier(&event.name));
    let mut occupied = std::collections::BTreeSet::new();
    occupied.extend(
        interface
            .methods
            .iter()
            .map(|method| cpp_identifier(&method.name)),
    );
    occupied.extend(
        interface
            .events
            .iter()
            .map(|event| cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name))),
    );
    let mut candidate = base.clone();
    let mut suffix = 1usize;
    while occupied.contains(&candidate) {
        candidate = format!("{base}{suffix}");
        suffix += 1;
    }
    candidate
}

fn cpp_swift_event_copy_adapter_name(
    idl: &PluginIdl,
    interface: &Interface,
    event: &Event,
    index: usize,
) -> String {
    unique_cpp_type_name(
        idl,
        &format!(
            "nexaSwiftCopyEvent{}_{}_{index}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

fn render_getter(out: &mut String, property: &Property) {
    out.push_str(&format!(
        "    virtual {} {}() const noexcept = 0;\n",
        cpp_type(&property.ty),
        cpp_getter_name(&property.name)
    ));
}

fn render_setter(out: &mut String, property: &Property) {
    out.push_str(&format!(
        "    virtual void {}({} value) noexcept = 0;\n",
        cpp_setter_name(&property.name),
        cpp_type(&property.ty)
    ));
}

fn render_event_setter(out: &mut String, event: &Event) {
    let parameters = event
        .parameters
        .iter()
        .map(|parameter| cpp_type(&parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let callback = if parameters.is_empty() {
        "std::function<void()>".to_owned()
    } else {
        format!("std::function<void({parameters})>")
    };
    out.push_str(&format!(
        "    virtual void {}({callback} handler) noexcept = 0;\n",
        cpp_setter_name(&nexa_plugin_idl::event_callback_property(&event.name))
    ));
}

fn render_factory(out: &mut String, interface: &Interface) {
    let constructor = interface.constructors.first();
    let parameters = constructor
        .map(|constructor| cpp_parameters(&constructor.parameters))
        .unwrap_or_default();
    out.push_str(&format!(
        "std::unique_ptr<{}Spec> {}({parameters});\n",
        cpp_identifier(&interface.name),
        cpp_factory_name(&interface.name)
    ));
}

fn render_swift_factory(out: &mut String, interface: &Interface, idl: &PluginIdl) {
    let name = cpp_identifier(&interface.name);
    let constructor = interface.constructors.first();
    let parameters = constructor
        .map(|constructor| cpp_parameters(&constructor.parameters))
        .unwrap_or_default();
    let arguments = constructor
        .map(|constructor| {
            constructor
                .parameters
                .iter()
                .map(|parameter| cpp_identifier(&parameter.name))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    out.push_str(&format!(
        "inline NEXA_CXX_SWIFT_RETURNS_RETAINED {name}Spec* NEXA_CXX_SWIFT_NONNULL nexaMake{name}ForSwift({parameters}) noexcept {{\n    try {{\n        auto instance = {}({arguments});\n        if (!instance) std::terminate();\n        return instance.release();\n    }} catch (...) {{\n        std::terminate();\n    }}\n}}\n",
        cpp_factory_name(&interface.name)
    ));
    if let Some(constructor) = constructor.filter(|constructor| {
        constructor
            .parameters
            .iter()
            .all(|parameter| swift_cpp_value_type(&parameter.ty).is_some())
            && constructor
                .parameters
                .iter()
                .any(|parameter| swift_cpp_needs_vector_bridge(&parameter.ty))
    }) {
        let bridge_parameters =
            cpp_swift_collection_bridge_parameters(&constructor.parameters, idl);
        let bridge_arguments = constructor
            .parameters
            .iter()
            .map(|parameter| {
                cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), idl)
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "\ninline NEXA_CXX_SWIFT_RETURNS_RETAINED {name}Spec* NEXA_CXX_SWIFT_NONNULL nexaMake{name}ForSwift({bridge_parameters}) noexcept {{\n    try {{\n        auto instance = {}({bridge_arguments});\n        if (!instance) std::terminate();\n        return instance.release();\n    }} catch (...) {{\n        std::terminate();\n    }}\n}}\n",
            cpp_factory_name(&interface.name)
        ));
    }
}

fn render_service(out: &mut String, interface: &Interface) {
    let name = cpp_identifier(&interface.name);
    out.push_str(&format!("namespace {name} {{\n"));
    for method in &interface.methods {
        out.push_str("    ");
        out.push_str(&cpp_method_return(method));
        out.push(' ');
        out.push_str(&cpp_identifier(&method.name));
        out.push('(');
        out.push_str(&cpp_parameters(&method.parameters));
        out.push_str(") noexcept;\n");
    }
    out.push_str("} // namespace ");
    out.push_str(&name);
    out.push_str("\n");
}

fn render_swift_service_collection_adapter(
    out: &mut String,
    idl: &PluginIdl,
    interface: &Interface,
    method: &Method,
) {
    let success_type = swift_cpp_method_success_type(method);
    let return_type = cpp_swift_collection_bridge_type(success_type, idl);
    let parameters = cpp_swift_collection_bridge_parameters(&method.parameters, idl);
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| {
            cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), idl)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let call = format!(
        "{}::{}({arguments})",
        cpp_identifier(&interface.name),
        cpp_identifier(&method.name)
    );
    let adapter_name = cpp_swift_service_adapter_name(&interface.name, &method.name);
    if method.is_async {
        if swift_cpp_future_adapter_needed(method) {
            let future_adapter = cpp_swift_future_adapter_name(idl, interface, method);
            out.push_str(&format!(
                "inline {future_adapter} {adapter_name}({parameters}) noexcept {{\n    return {future_adapter}({}::{}({arguments}));\n}}\n",
                cpp_identifier(&interface.name),
                cpp_identifier(&method.name)
            ));
        } else {
            out.push_str(&format!(
                "inline {} {adapter_name}({parameters}) noexcept {{\n    return {}::{}({arguments});\n}}\n",
                cpp_method_return(method),
                cpp_identifier(&interface.name),
                cpp_identifier(&method.name)
            ));
        }
        return;
    }
    out.push_str(&format!(
        "inline {return_type} {adapter_name}({parameters}) noexcept {{\n"
    ));
    render_cpp_swift_collection_result(out, success_type, &call, idl, 1);
    out.push_str("}\n");
}

fn render_cpp_swift_collection_property_adapter(
    out: &mut String,
    interface: &Interface,
    property: &Property,
    idl: &PluginIdl,
) {
    let bridge_type = cpp_swift_collection_bridge_type(&property.ty, idl);
    let getter = cpp_getter_name(&property.name);
    let get_adapter = cpp_swift_property_adapter_name(interface, &property.name, false);
    if swift_cpp_map_types(&property.ty).is_some() {
        let to_entries = cpp_swift_map_conversion_name(idl, &property.ty, "ToEntries");
        out.push_str(&format!(
            "    {bridge_type} {get_adapter}() const noexcept {{ return {to_entries}({getter}()); }}\n"
        ));
    } else if property.ty.name == "Array"
        && swift_cpp_array_leaf(&property.ty).is_some_and(|leaf| leaf.name == "Bool")
    {
        let from_native = cpp_swift_array_conversion_name(idl, &property.ty, "FromNative");
        out.push_str(&format!(
            "    {bridge_type} {get_adapter}() const noexcept {{ return {from_native}({getter}()); }}\n"
        ));
    } else if property.ty.name == "Array"
        && swift_cpp_nested_array_supported(&property.ty)
        && property.ty.arguments[0].name == "Set"
    {
        let from_native = cpp_swift_array_conversion_name(idl, &property.ty, "FromNative");
        out.push_str(&format!(
            "    {bridge_type} {get_adapter}() const noexcept {{ return {from_native}({getter}()); }}\n"
        ));
    } else {
        out.push_str(&format!(
            "    {bridge_type} {get_adapter}() const noexcept {{ auto value = {getter}(); return {bridge_type}(value.begin(), value.end()); }}\n"
        ));
    }
    if property.mutable {
        let setter = cpp_setter_name(&property.name);
        let set_adapter = cpp_swift_property_adapter_name(interface, &property.name, true);
        let converted = cpp_swift_collection_argument(&property.ty, "value", idl);
        out.push_str(&format!(
            "    void {set_adapter}({bridge_type} value) noexcept {{ {setter}({converted}); }}\n"
        ));
    }
}

fn render_cpp_swift_collection_method_adapter(
    out: &mut String,
    interface: &Interface,
    method: &Method,
    idl: &PluginIdl,
) {
    let success_type = swift_cpp_method_success_type(method);
    let return_type = cpp_swift_collection_bridge_type(success_type, idl);
    let parameters = cpp_swift_collection_bridge_parameters(&method.parameters, idl);
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| {
            cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), idl)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let call = format!("{}({arguments})", cpp_identifier(&method.name));
    let adapter_name = cpp_swift_class_method_adapter_name(interface, &method.name);
    if method.is_async {
        if swift_cpp_future_adapter_needed(method) {
            let future_adapter = cpp_swift_future_adapter_name(idl, interface, method);
            out.push_str(&format!(
                "    {future_adapter} {adapter_name}({parameters}) noexcept {{ return {future_adapter}({call}); }}\n"
            ));
        } else {
            out.push_str(&format!(
                "    {} {adapter_name}({parameters}) noexcept {{ return {call}; }}\n",
                cpp_method_return(method)
            ));
        }
        return;
    }
    out.push_str(&format!(
        "    {return_type} {adapter_name}({parameters}) noexcept {{\n"
    ));
    render_cpp_swift_collection_result(out, success_type, &call, idl, 2);
    out.push_str("    }\n");
}

fn cpp_swift_collection_bridge_type(ty: &TypeRef, idl: &PluginIdl) -> String {
    if swift_cpp_map_types(ty).is_some() {
        cpp_swift_map_entries_name(idl, ty)
    } else if swift_cpp_nested_array_supported(ty) {
        cpp_swift_array_alias_name_for_type(idl, ty)
    } else if swift_cpp_needs_vector_bridge(ty) {
        cpp_swift_array_alias_name(idl, &ty.arguments[0].name)
    } else {
        cpp_type(ty)
    }
}

fn cpp_swift_collection_bridge_parameters(
    parameters: &[nexa_plugin_idl::Parameter],
    idl: &PluginIdl,
) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} {}",
                cpp_swift_collection_bridge_type(&parameter.ty, idl),
                cpp_identifier(&parameter.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn cpp_swift_collection_argument(ty: &TypeRef, argument: &str, idl: &PluginIdl) -> String {
    match ty.name.as_str() {
        "Map" if swift_cpp_map_types(ty).is_some() => format!(
            "{}({argument})",
            cpp_swift_map_conversion_name(idl, ty, "FromEntries")
        ),
        "Set" => format!(
            "std::set<{}>({argument}.begin(), {argument}.end())",
            cpp_type(&ty.arguments[0])
        ),
        "Array" if swift_cpp_array_leaf(ty).is_some_and(|leaf| leaf.name == "Bool") => {
            format!(
                "{}({argument})",
                cpp_swift_array_conversion_name(idl, ty, "ToNative")
            )
        }
        "Array" if swift_cpp_nested_array_supported(ty) && ty.arguments[0].name == "Set" => {
            format!(
                "{}({argument})",
                cpp_swift_array_conversion_name(idl, ty, "ToNative")
            )
        }
        "Array" if swift_cpp_nested_array_supported(ty) => argument.to_owned(),
        "Array" if swift_cpp_needs_vector_bridge(ty) => {
            format!("std::vector<bool>({argument}.begin(), {argument}.end())")
        }
        _ => argument.to_owned(),
    }
}

fn render_cpp_swift_collection_result(
    out: &mut String,
    ty: &TypeRef,
    expression: &str,
    idl: &PluginIdl,
    indent: usize,
) {
    let prefix = "    ".repeat(indent);
    if ty.name == "Void" {
        out.push_str(&format!("{prefix}{expression};\n"));
    } else if swift_cpp_map_types(ty).is_some() {
        let to_entries = cpp_swift_map_conversion_name(idl, ty, "ToEntries");
        out.push_str(&format!("{prefix}return {to_entries}({expression});\n"));
    } else if swift_cpp_needs_vector_bridge(ty) {
        let alias = cpp_swift_collection_bridge_type(ty, idl);
        if ty.name == "Array" && swift_cpp_array_leaf(ty).is_some_and(|leaf| leaf.name == "Bool") {
            let from_native = cpp_swift_array_conversion_name(idl, ty, "FromNative");
            out.push_str(&format!("{prefix}return {from_native}({expression});\n"));
        } else if ty.name == "Array"
            && swift_cpp_nested_array_supported(ty)
            && ty.arguments[0].name == "Set"
        {
            let from_native = cpp_swift_array_conversion_name(idl, ty, "FromNative");
            out.push_str(&format!("{prefix}return {from_native}({expression});\n"));
        } else {
            out.push_str(&format!(
                "{prefix}auto value = {expression};\n{prefix}return {alias}(value.begin(), value.end());\n"
            ));
        }
    } else {
        out.push_str(&format!("{prefix}return {expression};\n"));
    }
}

fn render_cpp_swift_future_adapters(out: &mut String, idl: &PluginIdl) {
    for interface in &idl.interfaces {
        if !matches!(
            interface.kind,
            InterfaceKind::Service | InterfaceKind::NativeClass
        ) {
            continue;
        }
        for method in &interface.methods {
            let success_type = swift_cpp_method_success_type(method);
            if method.is_async
                && swift_cpp_needs_vector_bridge(success_type)
                && swift_cpp_value_type(success_type).is_some()
                && method
                    .parameters
                    .iter()
                    .all(|parameter| swift_cpp_value_type(&parameter.ty).is_some())
            {
                render_cpp_swift_future_adapter(out, idl, interface, method);
            }
        }
    }
}

fn render_cpp_swift_future_adapter(
    out: &mut String,
    idl: &PluginIdl,
    interface: &Interface,
    method: &Method,
) {
    let adapter_name = cpp_swift_future_adapter_name(idl, interface, method);
    let success_type = swift_cpp_method_success_type(method);
    let bridge_success = cpp_swift_collection_bridge_type(success_type, idl);
    let future_type = cpp_method_return(method);
    let error_type = swift_cpp_method_error_type(method);
    let bridge_result = error_type
        .map(|error| format!("NexaResult<{bridge_success}, {}>", cpp_type(error)))
        .unwrap_or_else(|| bridge_success.clone());

    out.push_str(&format!(
        "class {adapter_name} {{\npublic:\n    explicit {adapter_name}({future_type} future) noexcept : future_(std::move(future)) {{}}\n    {bridge_result} get() noexcept {{\n"
    ));
    if let Some(error_type) = error_type {
        let cpp_error = cpp_type(error_type);
        out.push_str(&format!(
            "        auto result = future_.get();\n        if (!result.has_value()) return NexaResult<{bridge_success}, {cpp_error}>::failure(std::move(result.error()));\n        return NexaResult<{bridge_success}, {cpp_error}>::success([&]() -> {bridge_success} {{\n"
        ));
        render_cpp_swift_collection_result(out, success_type, "result.value()", idl, 3);
        out.push_str("        }());\n");
    } else {
        out.push_str("        auto result = future_.get();\n");
        out.push_str(&format!("        return [&]() -> {bridge_success} {{\n"));
        render_cpp_swift_collection_result(out, success_type, "result", idl, 3);
        out.push_str("        }();\n");
    }
    out.push_str(&format!(
        "    }}\nprivate:\n    {future_type} future_;\n}};\n\n"
    ));
}

fn cpp_swift_service_adapter_name(interface: &str, method: &str) -> String {
    format!(
        "nexaSwiftAdapter_{}_{}",
        cpp_identifier(interface),
        cpp_identifier(method)
    )
}

fn cpp_swift_future_adapter_name(
    idl: &PluginIdl,
    interface: &Interface,
    method: &Method,
) -> String {
    unique_cpp_type_name(
        idl,
        &format!(
            "NexaCppSwiftFuture{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&method.name)
        ),
    )
}

fn cpp_swift_class_method_adapter_name(interface: &Interface, method: &str) -> String {
    let key = format!("method:{method}");
    cpp_swift_class_adapter_member_name(interface, &key)
}

fn cpp_swift_property_adapter_name(interface: &Interface, property: &str, setter: bool) -> String {
    let key = format!("property:{}:{property}", if setter { "set" } else { "get" });
    cpp_swift_class_adapter_member_name(interface, &key)
}

fn cpp_swift_class_adapter_member_name(interface: &Interface, target_key: &str) -> String {
    let mut occupied = std::collections::BTreeSet::from([
        "nexaRetainForSwift".to_owned(),
        "nexaReleaseFromSwift".to_owned(),
        "swift_references_".to_owned(),
    ]);
    for method in &interface.methods {
        occupied.insert(cpp_identifier(&method.name));
    }
    for property in &interface.properties {
        occupied.insert(cpp_getter_name(&property.name));
        if property.mutable {
            occupied.insert(cpp_setter_name(&property.name));
        }
    }
    for event in &interface.events {
        occupied.insert(cpp_setter_name(&nexa_plugin_idl::event_callback_property(
            &event.name,
        )));
    }

    let mut requests = Vec::new();
    for property in &interface.properties {
        if swift_cpp_needs_vector_bridge(&property.ty) {
            requests.push((
                format!("property:get:{}", property.name),
                format!("nexaSwiftAdapterGet{}", cpp_identifier(&property.name)),
            ));
            if property.mutable {
                requests.push((
                    format!("property:set:{}", property.name),
                    format!("nexaSwiftAdapterSet{}", cpp_identifier(&property.name)),
                ));
            }
        }
    }
    for method in &interface.methods {
        if swift_cpp_method_uses_collection_adapter(method) {
            requests.push((
                format!("method:{}", method.name),
                format!("nexaSwiftAdapter_{}", cpp_identifier(&method.name)),
            ));
        }
    }

    for (key, base) in requests {
        let mut candidate = base.clone();
        let mut suffix = 1usize;
        while occupied.contains(&candidate) {
            candidate = format!("{base}{suffix}");
            suffix += 1;
        }
        if key == target_key {
            return candidate;
        }
        occupied.insert(candidate);
    }
    unreachable!("requested C++ Swift adapter member should be generated")
}

fn cpp_method_return(method: &Method) -> String {
    let base = if let Some(throws) = &method.throws {
        format!(
            "NexaResult<{}, {}>",
            cpp_type(&method.return_type),
            cpp_type(throws)
        )
    } else {
        cpp_type(&method.return_type)
    };
    if method.is_async {
        format!("std::future<{base}>")
    } else {
        base
    }
}

fn cpp_parameters(parameters: &[nexa_plugin_idl::Parameter]) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} {}",
                cpp_type(&parameter.ty),
                cpp_identifier(&parameter.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn cpp_type(ty: &TypeRef) -> String {
    let base = cpp_type_base(ty);
    if ty.optional {
        format!("std::optional<{base}>")
    } else {
        base
    }
}

fn cpp_type_base(ty: &TypeRef) -> String {
    match ty.name.as_str() {
        "Void" => "void".to_owned(),
        "String" => "std::string".to_owned(),
        "Bool" => "bool".to_owned(),
        "Int8" => "std::int8_t".to_owned(),
        "Int16" => "std::int16_t".to_owned(),
        "Int32" => "std::int32_t".to_owned(),
        "Int64" => "std::int64_t".to_owned(),
        "UInt8" => "std::uint8_t".to_owned(),
        "UInt16" => "std::uint16_t".to_owned(),
        "UInt32" => "std::uint32_t".to_owned(),
        "UInt64" => "std::uint64_t".to_owned(),
        "Float32" => "float".to_owned(),
        "Float64" => "double".to_owned(),
        "Bytes" => "std::vector<std::uint8_t>".to_owned(),
        "Array" => format!("std::vector<{}>", cpp_type(&ty.arguments[0])),
        "Set" => format!("std::set<{}>", cpp_type(&ty.arguments[0])),
        "Map" => format!(
            "std::map<{}, {}>",
            cpp_type(&ty.arguments[0]),
            cpp_type(&ty.arguments[1])
        ),
        "Pair" => format!(
            "std::pair<{}, {}>",
            cpp_type(&ty.arguments[0]),
            cpp_type(&ty.arguments[1])
        ),
        "Triple" => format!(
            "std::tuple<{}, {}, {}>",
            cpp_type(&ty.arguments[0]),
            cpp_type(&ty.arguments[1]),
            cpp_type(&ty.arguments[2])
        ),
        "Result" => format!(
            "NexaResult<{}, {}>",
            cpp_type(&ty.arguments[0]),
            cpp_type(&ty.arguments[1])
        ),
        name => cpp_identifier(name),
    }
}

fn cpp_literal(literal: &Literal) -> String {
    match literal {
        Literal::String(value) => format!("\"{}\"", cpp_string(value)),
        Literal::Number(value) => value.clone(),
        Literal::Bool(value) => value.to_string(),
        Literal::Null => "std::nullopt".to_owned(),
    }
}

fn cpp_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character => result.push(character),
        }
    }
    result
}

fn cpp_identifier(value: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "alignas",
        "alignof",
        "and",
        "asm",
        "auto",
        "bool",
        "break",
        "case",
        "catch",
        "char",
        "class",
        "concept",
        "const",
        "constexpr",
        "continue",
        "co_await",
        "co_return",
        "co_yield",
        "decltype",
        "default",
        "delete",
        "do",
        "double",
        "else",
        "enum",
        "explicit",
        "export",
        "extern",
        "false",
        "float",
        "for",
        "friend",
        "goto",
        "if",
        "inline",
        "int",
        "long",
        "namespace",
        "new",
        "noexcept",
        "not",
        "nullptr",
        "operator",
        "or",
        "private",
        "protected",
        "public",
        "register",
        "reinterpret_cast",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "template",
        "this",
        "thread_local",
        "throw",
        "true",
        "try",
        "typedef",
        "typeid",
        "typename",
        "union",
        "unsigned",
        "using",
        "virtual",
        "void",
        "volatile",
        "while",
        "xor",
    ];
    let mut identifier = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if identifier.is_empty() || identifier.as_bytes()[0].is_ascii_digit() {
        identifier.insert(0, '_');
    }
    if KEYWORDS.contains(&identifier.as_str()) {
        identifier.push('_');
    }
    identifier
}

fn cpp_case_type_name(value: &str) -> String {
    let mut name = String::new();
    let mut uppercase = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase {
                name.extend(character.to_uppercase());
                uppercase = false;
            } else {
                name.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    if name.is_empty() || name.as_bytes()[0].is_ascii_digit() {
        name.insert(0, '_');
    }
    format!("{}Case", cpp_identifier(&name))
}

fn cpp_getter_name(value: &str) -> String {
    format!("get{}", cpp_case_type_name(value).trim_end_matches("Case"))
}

fn cpp_setter_name(value: &str) -> String {
    format!("set{}", cpp_case_type_name(value).trim_end_matches("Case"))
}

fn cpp_factory_name(value: &str) -> String {
    format!(
        "make{}Impl",
        cpp_case_type_name(value).trim_end_matches("Case")
    )
}

fn cpp_namespace(plugin_id: &str) -> String {
    plugin_id
        .split('.')
        .map(|part| {
            let mut segment = String::from("plugin_");
            for character in part.chars() {
                match character {
                    '-' => segment.push_str("_dash_"),
                    '_' => segment.push_str("_under_"),
                    character if character.is_ascii_alphanumeric() => segment.push(character),
                    character => segment.push_str(&format!("x{:x}_", character as u32)),
                }
            }
            segment
        })
        .collect::<Vec<_>>()
        .join("::")
}

fn cpp_byte_buffer_alias_name(idl: &PluginIdl) -> String {
    let base = "NexaCppByteBuffer";
    if !cpp_name_is_declared(idl, base) {
        return base.to_owned();
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}{suffix}");
        if !cpp_name_is_declared(idl, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn optional_swift_cpp_types(idl: &PluginIdl) -> Vec<String> {
    let mut names = Vec::new();
    let mut add = |ty: &TypeRef| {
        if !ty.optional
            || !ty.arguments.is_empty()
            || swift_cpp_base_type(ty).is_none()
            || ty.name == "Void"
        {
            return;
        }
        if !names.iter().any(|name| name == &ty.name) {
            names.push(ty.name.clone());
        }
    };
    for interface in &idl.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                add(&parameter.ty);
            }
        }
        for property in &interface.properties {
            add(&property.ty);
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                add(&parameter.ty);
            }
            add(&method.return_type);
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                add(&parameter.ty);
            }
        }
    }
    names
}

fn cpp_name_is_declared(idl: &PluginIdl, name: &str) -> bool {
    idl.types.iter().any(|ty| ty.name == name)
        || idl
            .interfaces
            .iter()
            .any(|interface| interface.name == name)
}

fn cpp_optional_alias_name(idl: &PluginIdl, value_type: &str) -> String {
    let base = format!("NexaCppOptional{}", cpp_identifier(value_type));
    unique_cpp_type_name(idl, &base)
}

fn cpp_swift_array_alias_name(idl: &PluginIdl, element_type: &str) -> String {
    let base = format!("NexaCppArray{}", cpp_identifier(element_type));
    unique_cpp_type_name(idl, &base)
}

fn cpp_swift_array_alias_name_for_type(idl: &PluginIdl, ty: &TypeRef) -> String {
    let element = ty
        .arguments
        .first()
        .expect("Swift collection adapters require an array element");
    if element.name != "Array" {
        return cpp_swift_array_alias_name(idl, &element.name);
    }
    fn signature(ty: &TypeRef) -> String {
        if ty.name == "Array" {
            format!(
                "Array{}",
                signature(ty.arguments.first().expect("array type has an element"))
            )
        } else {
            cpp_identifier(&ty.name)
        }
    }
    unique_cpp_type_name(idl, &format!("NexaCppArray{}", signature(element)))
}

fn cpp_optional_bridge_name(idl: &PluginIdl) -> String {
    unique_cpp_type_name(idl, "NexaCppOptionalBridge")
}

fn cpp_swift_error_bridge_name(idl: &PluginIdl, error_name: &str) -> String {
    unique_cpp_type_name(
        idl,
        &format!("NexaCppErrorBridge{}", cpp_identifier(error_name)),
    )
}

fn swift_cpp_error_converter_name(error_name: &str) -> String {
    format!("nexaCppErrorTo{}", cpp_identifier(error_name))
}

fn unique_cpp_type_name(idl: &PluginIdl, base: &str) -> String {
    if !cpp_name_is_declared(idl, base) {
        return base.to_owned();
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}{suffix}");
        if !cpp_name_is_declared(idl, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}
