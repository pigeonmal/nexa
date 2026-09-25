//! C++ specification-header rendering: the pure native ABI surface.
//!
//! Owns the shared native vocabulary every other C++ module speaks: C++ type
//! spellings, identifier and member naming, literals, and the result-free
//! contract shapes (structs, enums, interfaces, factories, services, events,
//! properties). Collection facades, typed errors, and the platform adapters
//! live in their own modules and are composed from here.

use super::collections::{
    cpp_swift_array_conversion_name, cpp_swift_collection_argument,
    cpp_swift_collection_bridge_parameters, cpp_swift_collection_bridge_type,
    cpp_swift_map_conversion_name, render_cpp_swift_collection_method_adapter,
    render_cpp_swift_collection_property_adapter, render_cpp_swift_collection_result,
    render_swift_array_aliases, render_swift_map_adapters, render_swift_service_collection_adapter,
    swift_cpp_array_leaf, swift_cpp_map_types, swift_cpp_method_uses_collection_adapter,
    swift_cpp_needs_vector_bridge, swift_cpp_nested_array_supported,
};
use super::errors::{render_result_type, render_swift_cpp_error_bridges};
use super::swift::swift_cpp_value_type;
use crate::SourceWriter;
use crate::plugin::bridge_plan::{
    self, BridgeEvent, BridgeInterface, BridgeMethod, BridgeNamedType, BridgeParameter, BridgePlan,
    BridgeProperty, BridgeScalar, BridgeType, BridgeTypeKind,
};
use crate::plugin::type_visit::{for_each_named_type, for_each_value_type};
use nexa_plugin_idl::{self, InterfaceKind, Literal};

pub fn render(plan: &BridgePlan, plugin_id: &str) -> String {
    let mut out = SourceWriter::new();
    out.push_str(
        "#pragma once\n\n#include <atomic>\n#include <cstdint>\n#include <exception>\n#include <functional>\n#include <future>\n#include <map>\n#include <memory>\n#include <optional>\n#include <set>\n#include <string>\n#include <tuple>\n#include <utility>\n#include <variant>\n#include <vector>\n\n#if __has_include(<swift/bridging>)\n#include <swift/bridging>\n#define NEXA_CXX_SWIFT_SHARED_REFERENCE(...) SWIFT_SHARED_REFERENCE(__VA_ARGS__)\n#define NEXA_CXX_SWIFT_RETURNS_RETAINED SWIFT_RETURNS_RETAINED\n#define NEXA_CXX_SWIFT_NONNULL _Nonnull\n#define NEXA_CXX_SWIFT_NULLABLE _Nullable\n#else\n#define NEXA_CXX_SWIFT_SHARED_REFERENCE(...)\n#define NEXA_CXX_SWIFT_RETURNS_RETAINED\n#define NEXA_CXX_SWIFT_NONNULL\n#define NEXA_CXX_SWIFT_NULLABLE\n#endif\n\n",
    );
    out.push_str(&format!("namespace {} {{\n\n", cpp_namespace(plugin_id)));
    if plan.interfaces.iter().any(|interface| {
        interface.kind == InterfaceKind::NativeClass && !interface.events.is_empty()
    }) {
        out.push_str(
            "class NexaCppSwiftEventContextHandle final {\n    using Release = void (*)(void* NEXA_CXX_SWIFT_NONNULL);\n    struct State {\n        std::size_t references;\n        void* NEXA_CXX_SWIFT_NONNULL context;\n        Release release;\n        State(void* value, Release callback) : references(1), context(value), release(callback) {}\n    };\n    State* state_;\n    void retain() noexcept { if (state_ != nullptr) __atomic_add_fetch(&state_->references, std::size_t{1}, __ATOMIC_RELAXED); }\n    void relinquish() noexcept {\n        if (state_ != nullptr && __atomic_sub_fetch(&state_->references, std::size_t{1}, __ATOMIC_ACQ_REL) == 0) {\n            state_->release(state_->context);\n            delete state_;\n        }\n    }\npublic:\n    NexaCppSwiftEventContextHandle(void* NEXA_CXX_SWIFT_NONNULL context, Release release) : state_(new State(context, release)) {}\n    NexaCppSwiftEventContextHandle(const NexaCppSwiftEventContextHandle& other) noexcept : state_(other.state_) { retain(); }\n    NexaCppSwiftEventContextHandle(NexaCppSwiftEventContextHandle&& other) noexcept : state_(std::exchange(other.state_, nullptr)) {}\n    ~NexaCppSwiftEventContextHandle() { relinquish(); }\n    void* NEXA_CXX_SWIFT_NONNULL get() const noexcept { return state_->context; }\n};\n\n",
        );
    }
    out.push_str(&format!(
        "using {} = std::vector<std::uint8_t>;\n\n",
        cpp_byte_buffer_alias_name(plan)
    ));
    render_optional_bridge(&mut out, plan);
    render_swift_array_aliases(&mut out, plan);
    render_swift_map_adapters(&mut out, plan);
    render_result_type(&mut out);

    for_each_named_type(plan, &mut |ty| {
        let declaration = match ty.kind {
            BridgeTypeKind::Struct | BridgeTypeKind::Error => {
                format!("struct {};\n", cpp_identifier(&ty.name))
            }
            BridgeTypeKind::Enum => {
                format!("enum class {} : std::uint8_t;\n", cpp_identifier(&ty.name))
            }
        };
        out.push_str(&declaration);
    });
    for interface in &plan.interfaces {
        if matches!(
            interface.kind,
            InterfaceKind::Interface | InterfaceKind::NativeClass
        ) {
            out.push_str(&format!("class {}Spec;\n", cpp_identifier(&interface.name)));
        }
    }
    out.push('\n');

    for_each_named_type(plan, &mut |ty| {
        render_named_type(&mut out, ty);
        out.push('\n');
    });
    render_cpp_swift_future_adapters(&mut out, plan);
    render_swift_cpp_error_bridges(&mut out, plan);
    for interface in &plan.interfaces {
        match interface.kind {
            InterfaceKind::Interface | InterfaceKind::NativeClass => {
                render_contract(&mut out, interface, plan);
                out.push('\n');
                if interface.kind == InterfaceKind::NativeClass {
                    render_factory(&mut out, interface);
                    out.push('\n');
                    render_swift_factory(&mut out, interface, plan);
                    out.push('\n');
                }
            }
            InterfaceKind::Service => {
                render_service(&mut out, interface);
                out.push('\n');
                for method in &interface.methods {
                    if swift_cpp_method_uses_collection_adapter(method) {
                        render_swift_service_collection_adapter(&mut out, plan, interface, method);
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
    out.finish()
}

/// Strips one optional layer, mirroring how the historical renderers
/// matched on `TypeRef::name` while the `optional` flag lived separately.
/// Only use where the original code ignored the flag in the same check.
pub(crate) fn bridge_strip_optional(ty: &BridgeType) -> &BridgeType {
    match ty {
        BridgeType::Optional(inner) => inner,
        other => other,
    }
}

/// Source-level name of a resolved type, mirroring the IDL spelling the
/// renderers historically matched on (`optional` is transparent, like the
/// old `TypeRef::name` field).
pub(crate) fn bridge_type_name(ty: &BridgeType) -> &str {
    match ty {
        BridgeType::Scalar(scalar) => match scalar {
            BridgeScalar::Void => "Void",
            BridgeScalar::Bool => "Bool",
            BridgeScalar::Int8 => "Int8",
            BridgeScalar::Int16 => "Int16",
            BridgeScalar::Int32 => "Int32",
            BridgeScalar::Int64 => "Int64",
            BridgeScalar::UInt8 => "UInt8",
            BridgeScalar::UInt16 => "UInt16",
            BridgeScalar::UInt32 => "UInt32",
            BridgeScalar::UInt64 => "UInt64",
            BridgeScalar::Float32 => "Float32",
            BridgeScalar::Float64 => "Float64",
            BridgeScalar::String => "String",
            BridgeScalar::Bytes => "Bytes",
        },
        BridgeType::Named { name, .. } => name,
        BridgeType::Array(_) => "Array",
        BridgeType::Set(_) => "Set",
        BridgeType::Map(..) => "Map",
        BridgeType::Pair(..) => "Pair",
        BridgeType::Triple(..) => "Triple",
        BridgeType::Optional(inner) => bridge_type_name(inner),
        BridgeType::Result { .. } => "Result",
    }
}

/// Generic arguments of a compound bridge type, mirroring the old
/// `TypeRef::arguments` vector (`optional` was a separate flag there, so
/// `Optional` contributes no arguments here either).
pub(crate) fn bridge_type_arguments(ty: &BridgeType) -> Vec<&BridgeType> {
    match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => vec![element],
        BridgeType::Map(key, value) | BridgeType::Pair(key, value) => vec![key, value],
        BridgeType::Triple(first, second, third) => vec![first, second, third],
        BridgeType::Result { success, .. } => vec![success],
        BridgeType::Scalar(_) | BridgeType::Named { .. } | BridgeType::Optional(_) => Vec::new(),
    }
}

/// Whether the type is a generic container, i.e. whether it has type
/// arguments. Total and allocation-free, for the many spellings that only need
/// to know *that* a type is generic rather than which arguments it carries.
pub(crate) fn bridge_type_is_generic(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Array(_)
            | BridgeType::Set(_)
            | BridgeType::Map(..)
            | BridgeType::Pair(..)
            | BridgeType::Triple(..)
            | BridgeType::Result { .. }
    )
}

fn render_optional_bridge(out: &mut SourceWriter, plan: &BridgePlan) {
    let optional_types = optional_swift_cpp_types(plan);
    if optional_types.is_empty() {
        return;
    }

    for ty in &optional_types {
        let name = bridge_value_base_name(ty);
        out.push_str(&format!(
            "using {} = std::optional<{}>;\n",
            cpp_optional_alias_name(plan, name),
            cpp_type(ty)
        ));
    }
    out.push_str(&format!("\nstruct {} {{\n", cpp_optional_bridge_name(plan)));
    for ty in optional_types {
        let name = bridge_value_base_name(&ty);
        let alias = cpp_optional_alias_name(plan, name);
        let cpp_type = cpp_type(&ty);
        out.push_str(&format!(
            "    static {alias} some{name}({cpp_type} value) noexcept {{ return value; }}\n    static {alias} none{name}() noexcept {{ return std::nullopt; }}\n"
        ));
    }
    out.push_str("};\n\n");
}

/// Base name of an optional element type: the scalar spelling or the
/// referenced declaration name. Validation guarantees only scalars and
/// named values reach the optional bridge.
fn bridge_value_base_name(ty: &BridgeType) -> &str {
    match ty {
        BridgeType::Scalar(scalar) => bridge_plan::bridge_scalar_name(*scalar),
        BridgeType::Named { name, .. } => name,
        _ => {
            debug_assert!(
                false,
                "optional bridge collection should not contain compound element types"
            );
            "NexaValue"
        }
    }
}

fn render_named_type(out: &mut SourceWriter, ty: &BridgeNamedType) {
    let name = cpp_identifier(&ty.name);
    match ty.kind {
        BridgeTypeKind::Struct => {
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
        BridgeTypeKind::Enum => {
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
        BridgeTypeKind::Error => {
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
                    .map(|case| cpp_case_type_name(&case.name))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str(">;\n    Value value;\n};\n");
        }
    }
}

fn render_contract(out: &mut SourceWriter, interface: &BridgeInterface, plan: &BridgePlan) {
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
            render_cpp_swift_collection_property_adapter(out, interface, property, plan);
        }
    }
    for method in &interface.methods {
        if swift_cpp_method_uses_collection_adapter(method) {
            render_cpp_swift_collection_method_adapter(out, interface, method, plan);
        }
    }
    for event in &interface.events {
        render_event_setter(out, event);
        render_cpp_swift_event_setter(out, plan, interface, event);
    }
    out.push_str(
        "private:\n    mutable std::atomic<std::size_t> swift_references_{1};\n} NEXA_CXX_SWIFT_SHARED_REFERENCE(.nexaRetainForSwift, .nexaReleaseFromSwift);\n",
    );
    for event in &interface.events {
        for (index, parameter) in event.parameters.iter().enumerate() {
            render_cpp_swift_event_copy_adapter(out, plan, interface, event, index, &parameter.ty);
        }
    }
}

fn render_cpp_swift_event_setter(
    out: &mut SourceWriter,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) {
    let callback_name = cpp_swift_event_callback_type_name(plan, interface, event);
    let release_name = cpp_swift_event_release_type_name(plan, interface, event);
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
    out: &mut SourceWriter,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    event: &BridgeEvent,
    index: usize,
    ty: &BridgeType,
) {
    let name = cpp_swift_event_copy_adapter_name(plan, interface, event, index);
    let return_type = cpp_swift_collection_bridge_type(ty, plan);
    let value = format!("*static_cast<const {}*>(rawValue)", cpp_type(ty));
    let converted = if swift_cpp_map_types(ty).is_some() {
        format!(
            "{}({value})",
            cpp_swift_map_conversion_name(plan, ty, "ToEntries")
        )
    } else if matches!(bridge_strip_optional(ty), BridgeType::Array(_))
        // `Bool` leaves and nested set facades both need the element-wise
        // array converter rather than a plain vector range.
        && (swift_cpp_array_leaf(ty).is_some_and(|leaf| {
            matches!(bridge_strip_optional(leaf), BridgeType::Scalar(BridgeScalar::Bool))
        }) || (swift_cpp_nested_array_supported(ty)
            && matches!(
                ty,
                BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
            )))
    {
        format!(
            "{}({value})",
            cpp_swift_array_conversion_name(plan, ty, "FromNative")
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
    plan: &BridgePlan,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    unique_cpp_type_name(
        plan,
        &format!(
            "NexaCppSwiftEventCallback{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

fn cpp_swift_event_release_type_name(
    plan: &BridgePlan,
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
    unique_cpp_type_name(
        plan,
        &format!(
            "NexaCppSwiftEventRelease{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

pub(crate) fn cpp_swift_event_setter_name(
    interface: &BridgeInterface,
    event: &BridgeEvent,
) -> String {
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

pub(crate) fn cpp_swift_event_copy_adapter_name(
    plan: &BridgePlan,
    interface: &BridgeInterface,
    event: &BridgeEvent,
    index: usize,
) -> String {
    unique_cpp_type_name(
        plan,
        &format!(
            "nexaSwiftCopyEvent{}_{}_{index}",
            cpp_identifier(&interface.name),
            cpp_identifier(&event.name)
        ),
    )
}

fn render_getter(out: &mut SourceWriter, property: &BridgeProperty) {
    out.push_str(&format!(
        "    virtual {} {}() const noexcept = 0;\n",
        cpp_type(&property.ty),
        cpp_getter_name(&property.name)
    ));
}

fn render_setter(out: &mut SourceWriter, property: &BridgeProperty) {
    out.push_str(&format!(
        "    virtual void {}({} value) noexcept = 0;\n",
        cpp_setter_name(&property.name),
        cpp_type(&property.ty)
    ));
}

fn render_event_setter(out: &mut SourceWriter, event: &BridgeEvent) {
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

fn render_factory(out: &mut SourceWriter, interface: &BridgeInterface) {
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

fn render_swift_factory(out: &mut SourceWriter, interface: &BridgeInterface, plan: &BridgePlan) {
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
            cpp_swift_collection_bridge_parameters(&constructor.parameters, plan);
        let bridge_arguments = constructor
            .parameters
            .iter()
            .map(|parameter| {
                cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), plan)
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "\ninline NEXA_CXX_SWIFT_RETURNS_RETAINED {name}Spec* NEXA_CXX_SWIFT_NONNULL nexaMake{name}ForSwift({bridge_parameters}) noexcept {{\n    try {{\n        auto instance = {}({bridge_arguments});\n        if (!instance) std::terminate();\n        return instance.release();\n    }} catch (...) {{\n        std::terminate();\n    }}\n}}\n",
            cpp_factory_name(&interface.name)
        ));
    }
}

fn render_service(out: &mut SourceWriter, interface: &BridgeInterface) {
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
    out.push('\n');
}

fn render_cpp_swift_future_adapters(out: &mut SourceWriter, plan: &BridgePlan) {
    for interface in &plan.interfaces {
        if !matches!(
            interface.kind,
            InterfaceKind::Service | InterfaceKind::NativeClass
        ) {
            continue;
        }
        for method in &interface.methods {
            let success_type = method.success_type();
            if method.is_async
                && swift_cpp_needs_vector_bridge(success_type)
                && swift_cpp_value_type(success_type).is_some()
                && method
                    .parameters
                    .iter()
                    .all(|parameter| swift_cpp_value_type(&parameter.ty).is_some())
            {
                render_cpp_swift_future_adapter(out, plan, interface, method);
            }
        }
    }
}

fn render_cpp_swift_future_adapter(
    out: &mut SourceWriter,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    method: &BridgeMethod,
) {
    let adapter_name = cpp_swift_future_adapter_name(plan, interface, method);
    let success_type = method.success_type();
    let bridge_success = cpp_swift_collection_bridge_type(success_type, plan);
    let future_type = cpp_method_return(method);
    let error_name = method.error_type();
    let bridge_result = error_name
        .map(|error| format!("NexaResult<{bridge_success}, {}>", cpp_identifier(error)))
        .unwrap_or_else(|| bridge_success.clone());

    out.push_str(&format!(
        "class {adapter_name} {{\npublic:\n    explicit {adapter_name}({future_type} future) noexcept : future_(std::move(future)) {{}}\n    {bridge_result} get() noexcept {{\n"
    ));
    if let Some(error_name) = error_name {
        let cpp_error = cpp_identifier(error_name);
        out.push_str(&format!(
            "        auto result = future_.get();\n        if (!result.has_value()) return NexaResult<{bridge_success}, {cpp_error}>::failure(std::move(result.error()));\n        return NexaResult<{bridge_success}, {cpp_error}>::success([&]() -> {bridge_success} {{\n"
        ));
        render_cpp_swift_collection_result(out, success_type, "result.value()", plan, 3);
        out.push_str("        }());\n");
    } else {
        out.push_str("        auto result = future_.get();\n");
        out.push_str(&format!("        return [&]() -> {bridge_success} {{\n"));
        render_cpp_swift_collection_result(out, success_type, "result", plan, 3);
        out.push_str("        }();\n");
    }
    out.push_str(&format!(
        "    }}\nprivate:\n    {future_type} future_;\n}};\n\n"
    ));
}

pub(crate) fn cpp_swift_service_adapter_name(interface: &str, method: &str) -> String {
    format!(
        "nexaSwiftAdapter_{}_{}",
        cpp_identifier(interface),
        cpp_identifier(method)
    )
}

pub(crate) fn cpp_swift_future_adapter_name(
    plan: &BridgePlan,
    interface: &BridgeInterface,
    method: &BridgeMethod,
) -> String {
    unique_cpp_type_name(
        plan,
        &format!(
            "NexaCppSwiftFuture{}_{}",
            cpp_identifier(&interface.name),
            cpp_identifier(&method.name)
        ),
    )
}

pub(crate) fn cpp_swift_class_method_adapter_name(
    interface: &BridgeInterface,
    method: &str,
) -> String {
    let key = format!("method:{method}");
    cpp_swift_class_adapter_member_name(interface, &key)
}

pub(crate) fn cpp_swift_property_adapter_name(
    interface: &BridgeInterface,
    property: &str,
    setter: bool,
) -> String {
    let key = format!("property:{}:{property}", if setter { "set" } else { "get" });
    cpp_swift_class_adapter_member_name(interface, &key)
}

fn cpp_swift_class_adapter_member_name(interface: &BridgeInterface, target_key: &str) -> String {
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

pub(crate) fn cpp_method_return(method: &BridgeMethod) -> String {
    let base = if let Some(throws) = &method.throws {
        format!(
            "NexaResult<{}, {}>",
            cpp_type(&method.return_type),
            cpp_identifier(throws)
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

fn cpp_parameters(parameters: &[BridgeParameter]) -> String {
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

pub(crate) fn cpp_type(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Optional(inner) => format!("std::optional<{}>", cpp_type(inner)),
        other => cpp_type_base(other),
    }
}

fn cpp_type_base(ty: &BridgeType) -> String {
    match ty {
        BridgeType::Scalar(scalar) => match scalar {
            BridgeScalar::Void => "void",
            BridgeScalar::String => "std::string",
            BridgeScalar::Bool => "bool",
            BridgeScalar::Int8 => "std::int8_t",
            BridgeScalar::Int16 => "std::int16_t",
            BridgeScalar::Int32 => "std::int32_t",
            BridgeScalar::Int64 => "std::int64_t",
            BridgeScalar::UInt8 => "std::uint8_t",
            BridgeScalar::UInt16 => "std::uint16_t",
            BridgeScalar::UInt32 => "std::uint32_t",
            BridgeScalar::UInt64 => "std::uint64_t",
            BridgeScalar::Float32 => "float",
            BridgeScalar::Float64 => "double",
            BridgeScalar::Bytes => "std::vector<std::uint8_t>",
        }
        .to_owned(),
        BridgeType::Array(element) => format!("std::vector<{}>", cpp_type(element)),
        BridgeType::Set(element) => format!("std::set<{}>", cpp_type(element)),
        BridgeType::Map(key, value) => format!("std::map<{}, {}>", cpp_type(key), cpp_type(value)),
        BridgeType::Pair(first, second) => {
            format!("std::pair<{}, {}>", cpp_type(first), cpp_type(second))
        }
        BridgeType::Triple(first, second, third) => format!(
            "std::tuple<{}, {}, {}>",
            cpp_type(first),
            cpp_type(second),
            cpp_type(third)
        ),
        BridgeType::Result { success, failure } => format!(
            "NexaResult<{}, {}>",
            cpp_type(success),
            cpp_identifier(failure)
        ),
        BridgeType::Named { name, .. } => cpp_identifier(name),
        BridgeType::Optional(inner) => cpp_type_base(inner),
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

pub(crate) fn cpp_identifier(value: &str) -> String {
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

pub(crate) fn cpp_case_type_name(value: &str) -> String {
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

pub(crate) fn cpp_getter_name(value: &str) -> String {
    format!("get{}", cpp_case_type_name(value).trim_end_matches("Case"))
}

pub(crate) fn cpp_setter_name(value: &str) -> String {
    format!("set{}", cpp_case_type_name(value).trim_end_matches("Case"))
}

pub(crate) fn cpp_factory_name(value: &str) -> String {
    format!(
        "make{}Impl",
        cpp_case_type_name(value).trim_end_matches("Case")
    )
}

pub(crate) fn cpp_namespace(plugin_id: &str) -> String {
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

pub(crate) fn cpp_byte_buffer_alias_name(plan: &BridgePlan) -> String {
    let base = "NexaCppByteBuffer";
    if !cpp_name_is_declared(plan, base) {
        return base.to_owned();
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}{suffix}");
        if !cpp_name_is_declared(plan, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn optional_swift_cpp_types(plan: &BridgePlan) -> Vec<BridgeType> {
    let mut collected = Vec::new();
    let mut add = |ty: &BridgeType| {
        // Mirrors the renderer's old collection rule: optional scalars
        // (other than `Void`) and optional named values get bridge aliases.
        let BridgeType::Optional(inner) = ty else {
            return;
        };
        if !matches!(
            inner.as_ref(),
            BridgeType::Scalar(scalar) if !matches!(scalar, BridgeScalar::Void)
        ) && !matches!(inner.as_ref(), BridgeType::Named { .. })
        {
            return;
        }
        if !collected.iter().any(|existing| existing == inner.as_ref()) {
            collected.push((**inner).clone());
        }
    };
    for_each_value_type(plan, &mut add);
    collected
}

fn cpp_name_is_declared(plan: &BridgePlan, name: &str) -> bool {
    plan.types.iter().any(|ty| ty.name == name)
        || plan
            .interfaces
            .iter()
            .any(|interface| interface.name == name)
}

fn cpp_optional_alias_name(plan: &BridgePlan, value_type: &str) -> String {
    let base = format!("NexaCppOptional{}", cpp_identifier(value_type));
    unique_cpp_type_name(plan, &base)
}

pub(crate) fn cpp_optional_bridge_name(plan: &BridgePlan) -> String {
    unique_cpp_type_name(plan, "NexaCppOptionalBridge")
}

pub(crate) fn unique_cpp_type_name(plan: &BridgePlan, base: &str) -> String {
    if !cpp_name_is_declared(plan, base) {
        return base.to_owned();
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}{suffix}");
        if !cpp_name_is_declared(plan, &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}
