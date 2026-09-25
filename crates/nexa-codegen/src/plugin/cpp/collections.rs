//! C++ collection facades and Swift-facing collection bridges.
//!
//! Every `Array`/`Set`/`Map` the Swift adapter target can spell needs a facade
//! type plus two conversion functions. This module decides which facades a
//! contract needs, names them, renders their adapters, and bridges collection
//! arguments and results across the Swift boundary.

use super::abi::{
    bridge_strip_optional, bridge_type_arguments, bridge_type_is_generic, bridge_type_name,
    cpp_getter_name, cpp_identifier, cpp_method_return, cpp_setter_name,
    cpp_swift_class_method_adapter_name, cpp_swift_future_adapter_name,
    cpp_swift_property_adapter_name, cpp_swift_service_adapter_name, cpp_type,
    unique_cpp_type_name,
};
use super::swift::{
    SwiftAdapterContext, swift_cpp_argument_value, swift_cpp_base_type,
    swift_cpp_future_adapter_needed, swift_cpp_named_value_type, swift_cpp_value_base_type,
    swift_cpp_value_type,
};
use crate::SourceWriter;
use crate::plugin::bridge_plan::{
    BridgeInterface, BridgeMethod, BridgeParameter, BridgePlan, BridgeProperty, BridgeScalar,
    BridgeType,
};
use crate::plugin::type_visit::{Order, for_each_value_type, walk_type};

pub(crate) fn swift_cpp_set_element_type(ty: &BridgeType) -> Option<&'static str> {
    if ty.is_optional() {
        return None;
    }
    let BridgeType::Set(element) = ty else {
        return None;
    };
    if element.is_optional() || bridge_type_is_generic(element) {
        return None;
    }
    match bridge_type_name(element) {
        "Bool" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32"
        | "UInt64" | "Bytes" => swift_cpp_base_type(element),
        // C++ ordering for floating point and UTF-8 strings does not match
        // Swift Set equality for NaN, signed zero, or canonically equivalent text.
        _ => None,
    }
}

fn swift_cpp_supported_set(ty: &BridgeType) -> bool {
    matches!(ty, BridgeType::Set(_)) && swift_cpp_set_element_type(ty).is_some()
}

pub(crate) fn swift_cpp_map_types(ty: &BridgeType) -> Option<(&BridgeType, &BridgeType)> {
    if !matches!(ty, BridgeType::Map(..)) || ty.is_optional() {
        return None;
    }
    let (key, value): (&BridgeType, &BridgeType) = match ty {
        BridgeType::Map(key, value) => (key, value),
        _ => return None,
    };
    if key.is_optional()
        || value.is_optional()
        || swift_cpp_base_type(key).is_none()
        || !swift_cpp_map_value_supported(value)
        || !matches!(
            bridge_type_name(key),
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

fn swift_cpp_map_value_supported(ty: &BridgeType) -> bool {
    if matches!(ty, BridgeType::Map(..)) {
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

pub(crate) fn swift_cpp_array_swift_type(ty: &BridgeType) -> Option<String> {
    if ty.is_optional() {
        return None;
    }
    let BridgeType::Array(element) = ty else {
        return None;
    };
    if element.is_optional() {
        return None;
    }
    if matches!(element.as_ref(), BridgeType::Array(_)) {
        swift_cpp_array_leaf(element)?;
        return Some(format!("[{}]", swift_cpp_array_swift_type(element)?));
    }
    if matches!(element.as_ref(), BridgeType::Set(_)) {
        return Some(format!("[{}]", swift_cpp_value_type(element)?));
    }
    let swift_type = swift_cpp_value_base_type(element)?;
    (swift_type != "Void").then(|| format!("[{swift_type}]"))
}

pub(crate) fn swift_cpp_array_leaf(ty: &BridgeType) -> Option<&BridgeType> {
    if ty.is_optional() {
        return None;
    }
    let BridgeType::Array(element) = ty else {
        return None;
    };
    if element.is_optional() {
        return None;
    }
    if matches!(element.as_ref(), BridgeType::Array(_)) {
        swift_cpp_array_leaf(element)
    } else if swift_cpp_value_base_type(element).is_some_and(|base| base != "Void") {
        Some(element)
    } else {
        None
    }
}

pub(crate) fn swift_cpp_nested_array_supported(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Array(element)
            if matches!(element.as_ref(), BridgeType::Array(_) | BridgeType::Set(_))
    ) && swift_cpp_array_swift_type(ty).is_some()
}

fn swift_cpp_array_depth(ty: &BridgeType) -> usize {
    let mut depth = 0;
    let mut current = ty;
    while matches!(current, BridgeType::Array(_)) {
        depth += 1;
        let next: &BridgeType = match current {
            BridgeType::Array(element) => element,
            _ => break,
        };
        current = next;
    }
    depth
}

pub(crate) fn swift_cpp_needs_vector_bridge(ty: &BridgeType) -> bool {
    swift_cpp_supported_set(ty)
        || swift_cpp_map_types(ty).is_some()
        || swift_cpp_nested_array_supported(ty)
        || matches!(
            ty,
            BridgeType::Array(element)
                if !ty.is_optional()
                    && matches!(
                        element.as_ref(),
                        BridgeType::Scalar(BridgeScalar::Bool)
                    )
        )
}

pub(crate) fn swift_cpp_method_uses_collection_adapter(method: &BridgeMethod) -> bool {
    let success_type = method.success_type();
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

pub(crate) fn swift_cpp_argument_expression(
    ty: &BridgeType,
    value: &str,
    context: &SwiftAdapterContext<'_>,
) -> String {
    let SwiftAdapterContext {
        optional_bridge, ..
    } = context;
    if !ty.is_optional() {
        return swift_cpp_argument_value(ty, value, context);
    }
    let some = format!(
        "{optional_bridge}.some{}",
        cpp_identifier(bridge_type_name(ty))
    );
    let none = format!(
        "{optional_bridge}.none{}()",
        cpp_identifier(bridge_type_name(ty))
    );
    let converted = swift_cpp_argument_value(ty, "$0", context);
    format!("{value}.map {{ {some}({converted}) }} ?? {none}")
}

pub(crate) fn swift_cpp_array_argument_expression(
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
    let BridgeType::Array(element) = bridge_strip_optional(ty) else {
        return format!(
            "{namespace}.{}({value})",
            cpp_swift_array_alias_name_for_type(plan, ty)
        );
    };
    let alias = format!(
        "{namespace}.{}",
        cpp_swift_array_alias_name_for_type(plan, ty)
    );
    let converted = if matches!(element.as_ref(), BridgeType::Array(_)) {
        let nested = swift_cpp_array_argument_expression(element, "nexaNestedArray", context);
        format!("{value}.map {{ nexaNestedArray in {nested} }}")
    } else if matches!(element.as_ref(), BridgeType::Set(_)) {
        let nested = swift_cpp_argument_value(element, "nexaNestedCollection", context);
        format!("{value}.map {{ nexaNestedCollection in {nested} }}")
    } else {
        match bridge_type_name(element) {
            "Bool" => format!("{value}.map {{ UInt8($0 ? 1 : 0) }}"),
            "String" => format!("{value}.map {{ std.string($0) }}"),
            "Bytes" => format!("{value}.map {{ {byte_buffer_type}($0) }}"),
            _ if swift_cpp_named_value_type(plan, element).is_some() => format!(
                "{value}.map {{ nexaSwiftToCpp{}($0) }}",
                cpp_identifier(bridge_type_name(element))
            ),
            _ => value.to_owned(),
        }
    };
    format!("{alias}({converted})")
}

pub(crate) fn swift_cpp_map_value_argument(
    ty: &BridgeType,
    value: &str,
    context: &SwiftAdapterContext<'_>,
) -> String {
    let SwiftAdapterContext {
        plan,
        byte_buffer_type,
        ..
    } = context;
    match bridge_type_name(ty) {
        "String" => format!("std.string({value})"),
        "Bytes" => format!("{byte_buffer_type}({value})"),
        _ if swift_cpp_named_value_type(plan, ty).is_some() => {
            format!(
                "nexaSwiftToCpp{}({value})",
                cpp_identifier(bridge_type_name(ty))
            )
        }
        _ => value.to_owned(),
    }
}

pub(crate) fn swift_cpp_result_expression(
    ty: &BridgeType,
    call: &str,
    context: &SwiftAdapterContext<'_>,
) -> String {
    let SwiftAdapterContext {
        plan, namespace, ..
    } = context;
    if ty.is_optional() {
        let converted = match bridge_type_name(ty) {
            "String" => "String($0)",
            "Bytes" => "Data($0)",
            _ if swift_cpp_named_value_type(plan, ty).is_some() => {
                return format!(
                    "Optional(fromCxx: {call}).map {{ nexaSwiftFromCpp{}($0) }}",
                    cpp_identifier(bridge_type_name(ty))
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
        match bridge_type_name(ty) {
            "Map" => {
                let (key, value_type) =
                    swift_cpp_map_types(ty).expect("validated Swift C++ map result");
                // Collection adapters already return the vector-backed entry
                // representation. Importing the std::map conversion helper
                // here would make Swift expect a std::map at the call site.
                let entries = format!("Array({call})");
                let key_value = swift_cpp_map_key_result(key, "$0.key");
                let mapped_value = swift_cpp_result_expression(value_type, "$0.value", context);
                format!(
                    "Dictionary(uniqueKeysWithValues: {entries}.map {{ ({key_value}, {mapped_value}) }})"
                )
            }
            "String" => format!("String({call})"),
            "Bytes" => format!("Data({call})"),
            "Array" if swift_cpp_nested_array_supported(ty) => {
                swift_cpp_array_result_expression(ty, call, context)
            }
            "Array" => {
                let BridgeType::Array(element) = bridge_strip_optional(ty) else {
                    return call.to_owned();
                };
                let copied = format!(
                    "Array({namespace}.{}({call}))",
                    cpp_swift_array_alias_name(plan, bridge_type_name(element))
                );
                match bridge_type_name(element) {
                    "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
                    "String" => format!("{copied}.map {{ String($0) }}"),
                    "Bytes" => format!("{copied}.map {{ Data($0) }}"),
                    _ => copied,
                }
            }
            "Set" => {
                let BridgeType::Set(element) = bridge_strip_optional(ty) else {
                    return call.to_owned();
                };
                let copied = format!(
                    "Array({namespace}.{}({call}))",
                    cpp_swift_array_alias_name(plan, bridge_type_name(element))
                );
                let converted = match bridge_type_name(element) {
                    "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
                    "Bytes" => format!("{copied}.map {{ Data($0) }}"),
                    _ => copied,
                };
                format!("Set({converted})")
            }
            _ if swift_cpp_named_value_type(plan, ty).is_some() => {
                format!(
                    "nexaSwiftFromCpp{}({call})",
                    cpp_identifier(bridge_type_name(ty))
                )
            }
            _ => call.to_owned(),
        }
    }
}

fn swift_cpp_array_result_expression(
    ty: &BridgeType,
    call: &str,
    context: &SwiftAdapterContext<'_>,
) -> String {
    let SwiftAdapterContext {
        plan, namespace, ..
    } = context;
    let alias = cpp_swift_array_alias_name_for_type(plan, ty);
    let copied = format!("Array({namespace}.{alias}({call}))");
    let BridgeType::Array(element) = bridge_strip_optional(ty) else {
        return copied;
    };
    if matches!(element.as_ref(), BridgeType::Array(_) | BridgeType::Set(_)) {
        let nested = swift_cpp_result_expression(element, "nexaNestedCollection", context);
        format!("{copied}.map {{ nexaNestedCollection in {nested} }}")
    } else {
        match bridge_type_name(element) {
            "Bool" => format!("{copied}.map {{ $0 != 0 }}"),
            "String" => format!("{copied}.map {{ String($0) }}"),
            "Bytes" => format!("{copied}.map {{ Data($0) }}"),
            _ if swift_cpp_named_value_type(plan, element).is_some() => format!(
                "{copied}.map {{ nexaSwiftFromCpp{}($0) }}",
                cpp_identifier(bridge_type_name(element))
            ),
            _ => copied,
        }
    }
}

fn swift_cpp_map_key_result(ty: &BridgeType, value: &str) -> String {
    match bridge_type_name(ty) {
        "String" => format!("String({value})"),
        "Bytes" => format!("Data({value})"),
        _ => value.to_owned(),
    }
}

pub(crate) fn render_swift_array_aliases(out: &mut SourceWriter, plan: &BridgePlan) {
    let mut arrays = Vec::new();
    let mut add = |ty: &BridgeType| {
        let array = match ty {
            BridgeType::Set(element) if swift_cpp_supported_set(ty) => {
                BridgeType::Array(element.clone())
            }
            BridgeType::Array(_) => ty.clone(),
            // Non-array shapes never enter the alias table.
            _ => return,
        };
        // Only adapter-spellable arrays enter the alias table; anything
        // else (such as optional-element arrays, which share the bare
        // element alias name) renders through direct `cpp_type` spellings.
        if swift_cpp_array_swift_type(&array).is_some()
            && !arrays.iter().any(|existing| existing == &array)
        {
            arrays.push(array.clone());
        }
    };
    for_each_array_shape(plan, &mut add);
    arrays.sort_by_key(|array| {
        let depends_on_set_facade = matches!(
            array,
            BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
        );
        (swift_cpp_array_depth(array), depends_on_set_facade)
    });
    for array in arrays {
        out.push_str(&format!(
            "using {} = {};\n",
            cpp_swift_array_alias_name_for_type(plan, &array),
            cpp_swift_array_facade_type(&array, plan)
        ));
    }
    let mut bool_facades = arrays_with_bool_facades(plan);
    bool_facades.sort_by_key(swift_cpp_array_depth);
    for array in bool_facades {
        render_swift_array_conversion_adapters(out, plan, &array);
    }
    let mut set_facades = arrays_with_nested_set_facades(plan);
    set_facades.sort_by_key(swift_cpp_array_depth);
    for array in set_facades {
        render_swift_array_conversion_adapters(out, plan, &array);
    }
    if !out.ends_with("\n\n") && out.ends_with('\n') {
        out.push('\n');
    }
}

fn cpp_swift_array_facade_type(ty: &BridgeType, plan: &BridgePlan) -> String {
    let BridgeType::Array(element) = bridge_strip_optional(ty) else {
        return cpp_type(ty);
    };
    if matches!(element.as_ref(), BridgeType::Array(_)) {
        format!(
            "std::vector<{}>",
            cpp_swift_array_facade_type(element, plan)
        )
    } else if let BridgeType::Set(inner) = element.as_ref() {
        format!(
            "std::vector<{}>",
            cpp_swift_array_alias_name(plan, bridge_type_name(inner))
        )
    } else if matches!(
        bridge_strip_optional(element),
        BridgeType::Scalar(BridgeScalar::Bool)
    ) {
        "std::vector<std::uint8_t>".to_owned()
    } else {
        format!("std::vector<{}>", cpp_type(element))
    }
}

fn arrays_with_bool_facades(plan: &BridgePlan) -> Vec<BridgeType> {
    let mut arrays = Vec::new();
    let mut add = |ty: &BridgeType| {
        if swift_cpp_array_swift_type(ty).is_some()
            && swift_cpp_array_leaf(ty).is_some_and(|leaf| {
                matches!(
                    bridge_strip_optional(leaf),
                    BridgeType::Scalar(BridgeScalar::Bool)
                )
            })
            && !arrays.iter().any(|existing| existing == ty)
        {
            arrays.push(ty.clone());
        }
    };
    for_each_array_shape(plan, &mut add);
    arrays
}

fn arrays_with_nested_set_facades(plan: &BridgePlan) -> Vec<BridgeType> {
    let mut arrays = Vec::new();
    let mut add = |ty: &BridgeType| {
        if swift_cpp_nested_array_supported(ty)
            && matches!(
                ty,
                BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
            )
            && !arrays.iter().any(|existing| existing == ty)
        {
            arrays.push(ty.clone());
        }
    };
    for_each_array_shape(plan, &mut add);
    arrays
}

/// Visits every array-shaped type reachable from a value position, outermost
/// first.
///
/// One traversal backs the alias table and both facade-converter tables. A
/// `Set` of supported scalars crosses Swift as an array too, so it is visited
/// as the equivalent `Array` shape; every other shape is skipped, and plan
/// validation has already proven each visited array is adapter-supported.
fn for_each_array_shape(plan: &BridgePlan, visit: &mut impl FnMut(&BridgeType)) {
    for_each_value_type(plan, &mut |root| {
        walk_type(root, Order::OuterFirst, &mut |ty| match ty {
            BridgeType::Set(element) if swift_cpp_supported_set(ty) => {
                visit(&BridgeType::Array(element.clone()))
            }
            BridgeType::Array(_) => visit(ty),
            _ => {}
        })
    });
}

fn render_swift_array_conversion_adapters(
    out: &mut SourceWriter,
    plan: &BridgePlan,
    ty: &BridgeType,
) {
    let native_type = cpp_type(ty);
    let facade_type = cpp_swift_array_alias_name_for_type(plan, ty);
    let to_native_name = cpp_swift_array_conversion_name(plan, ty, "ToNative");
    let from_native_name = cpp_swift_array_conversion_name(plan, ty, "FromNative");
    out.push_str(&format!(
        "inline {native_type} {to_native_name}({facade_type} value) noexcept {{\n    {native_type} result;\n    result.reserve(value.size());\n    for (auto&& element : value) result.push_back({});\n    return result;\n}}\ninline {facade_type} {from_native_name}({native_type} value) noexcept {{\n    {facade_type} result;\n    result.reserve(value.size());\n    for (auto&& element : value) result.push_back({});\n    return result;\n}}\n\n",
        cpp_swift_array_convert_element(plan, ty, "element", true),
        cpp_swift_array_convert_element(plan, ty, "element", false),
    ));
}

fn cpp_swift_array_convert_element(
    plan: &BridgePlan,
    ty: &BridgeType,
    element: &str,
    to_native: bool,
) -> String {
    let BridgeType::Array(nested) = bridge_strip_optional(ty) else {
        return format!("static_cast<bool>({element})");
    };
    if matches!(nested.as_ref(), BridgeType::Array(_)) {
        let direction = if to_native { "ToNative" } else { "FromNative" };
        format!(
            "{}(std::move({element}))",
            cpp_swift_array_conversion_name(plan, nested, direction)
        )
    } else if let BridgeType::Set(inner) = nested.as_ref() {
        let element_type = cpp_type(inner);
        if to_native {
            format!("std::set<{element_type}>({element}.begin(), {element}.end())")
        } else {
            format!(
                "{}({element}.begin(), {element}.end())",
                cpp_swift_array_alias_name(plan, bridge_type_name(inner))
            )
        }
    } else if to_native {
        format!("static_cast<bool>({element})")
    } else {
        format!("static_cast<std::uint8_t>({element})")
    }
}

pub(crate) fn cpp_swift_array_conversion_name(
    plan: &BridgePlan,
    ty: &BridgeType,
    direction: &str,
) -> String {
    let alias = cpp_swift_array_alias_name_for_type(plan, ty);
    let signature = alias.trim_start_matches("NexaCppArray");
    format!("nexaCppArray{direction}{signature}")
}

pub(crate) fn render_swift_map_adapters(out: &mut SourceWriter, plan: &BridgePlan) {
    // Children first: a `Map<K, Map<..>>` entry adapter reads the inner map's
    // entry type, so the inner adapter has to be declared before the outer one.
    let mut maps = Vec::new();
    for_each_value_type(plan, &mut |root| {
        walk_type(root, Order::ChildrenFirst, &mut |ty| {
            if swift_cpp_map_types(ty).is_some() && !maps.iter().any(|existing| existing == ty) {
                maps.push(ty.clone());
            }
        })
    });

    for ty in maps {
        let (key, value) = swift_cpp_map_types(&ty).expect("collected map type is supported");
        let entry = cpp_swift_map_entry_name(plan, &ty);
        let entries = cpp_swift_map_entries_name(plan, &ty);
        let map_type = cpp_type(&ty);
        let key_type = cpp_type(key);
        let value_type = cpp_swift_collection_bridge_type(value, plan);
        let from_entries = cpp_swift_map_conversion_name(plan, &ty, "FromEntries");
        let to_entries = cpp_swift_map_conversion_name(plan, &ty, "ToEntries");
        let native_value = cpp_swift_collection_argument(value, "entry.value", plan);
        let bridge_value = cpp_swift_map_value_for_entry(value, "mappedValue", plan);
        out.push_str(&format!(
            "struct {entry} {{\n    {key_type} key;\n    {value_type} value;\n    {entry}({key_type} keyValue, {value_type} mappedValue) : key(std::move(keyValue)), value(std::move(mappedValue)) {{}}\n}};\nusing {entries} = std::vector<{entry}>;\ninline {map_type} {from_entries}({entries} entries) noexcept {{\n    {map_type} result;\n    for (auto& entry : entries) result.emplace(std::move(entry.key), {native_value});\n    return result;\n}}\ninline {entries} {to_entries}({map_type} value) noexcept {{\n    {entries} entries;\n    entries.reserve(value.size());\n    for (auto& [key, mappedValue] : value) entries.emplace_back(std::move(key), {bridge_value});\n    return entries;\n}}\n\n"
        ));
    }
}

fn cpp_swift_map_value_for_entry(ty: &BridgeType, value: &str, plan: &BridgePlan) -> String {
    match bridge_type_name(ty) {
        "Map" => format!(
            "{}({value})",
            cpp_swift_map_conversion_name(plan, ty, "ToEntries")
        ),
        "Array"
            if swift_cpp_nested_array_supported(ty)
                && matches!(
                    bridge_strip_optional(ty),
                    BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
                ) =>
        {
            format!(
                "{}({value})",
                cpp_swift_array_conversion_name(plan, ty, "FromNative")
            )
        }
        "Set" => {
            let facade = cpp_swift_collection_bridge_type(ty, plan);
            format!("{facade}({value}.begin(), {value}.end())")
        }
        _ => format!("std::move({value})"),
    }
}

fn cpp_swift_map_signature(ty: &BridgeType) -> String {
    bridge_type_arguments(ty)
        .into_iter()
        .map(cpp_swift_type_signature)
        .collect::<String>()
}

fn cpp_swift_type_signature(ty: &BridgeType) -> String {
    let mut signature = cpp_identifier(bridge_type_name(ty));
    for argument in bridge_type_arguments(bridge_strip_optional(ty)) {
        signature.push_str(&cpp_swift_type_signature(argument));
    }
    if ty.is_optional() {
        signature.push_str("Optional");
    }
    signature
}

pub(crate) fn cpp_swift_map_entry_name(plan: &BridgePlan, ty: &BridgeType) -> String {
    unique_cpp_type_name(
        plan,
        &format!("NexaCppMapEntry{}", cpp_swift_map_signature(ty)),
    )
}

pub(crate) fn cpp_swift_map_entries_name(plan: &BridgePlan, ty: &BridgeType) -> String {
    unique_cpp_type_name(
        plan,
        &format!("NexaCppMapEntries{}", cpp_swift_map_signature(ty)),
    )
}

pub(crate) fn cpp_swift_map_conversion_name(
    plan: &BridgePlan,
    ty: &BridgeType,
    direction: &str,
) -> String {
    format!(
        "nexaCppMap{direction}{}",
        cpp_swift_map_entries_name(plan, ty).trim_start_matches("NexaCppMapEntries")
    )
}

pub(crate) fn render_swift_service_collection_adapter(
    out: &mut SourceWriter,
    plan: &BridgePlan,
    interface: &BridgeInterface,
    method: &BridgeMethod,
) {
    let success_type = method.success_type();
    let return_type = cpp_swift_collection_bridge_type(success_type, plan);
    let parameters = cpp_swift_collection_bridge_parameters(&method.parameters, plan);
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| {
            cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), plan)
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
            let future_adapter = cpp_swift_future_adapter_name(plan, interface, method);
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
    render_cpp_swift_collection_result(out, success_type, &call, plan, 1);
    out.push_str("}\n");
}

pub(crate) fn render_cpp_swift_collection_property_adapter(
    out: &mut SourceWriter,
    interface: &BridgeInterface,
    property: &BridgeProperty,
    plan: &BridgePlan,
) {
    let bridge_type = cpp_swift_collection_bridge_type(&property.ty, plan);
    let getter = cpp_getter_name(&property.name);
    let get_adapter = cpp_swift_property_adapter_name(interface, &property.name, false);
    if swift_cpp_map_types(&property.ty).is_some() {
        let to_entries = cpp_swift_map_conversion_name(plan, &property.ty, "ToEntries");
        out.push_str(&format!(
            "    {bridge_type} {get_adapter}() const noexcept {{ return {to_entries}({getter}()); }}\n"
        ));
    } else if matches!(bridge_strip_optional(&property.ty), BridgeType::Array(_))
        // `Bool` leaves and nested set facades both convert element-wise.
        && (swift_cpp_array_leaf(&property.ty).is_some_and(|leaf| {
            matches!(
                bridge_strip_optional(leaf),
                BridgeType::Scalar(BridgeScalar::Bool)
            )
        }) || (swift_cpp_nested_array_supported(&property.ty)
            && matches!(
                &property.ty,
                BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
            )))
    {
        let from_native = cpp_swift_array_conversion_name(plan, &property.ty, "FromNative");
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
        let converted = cpp_swift_collection_argument(&property.ty, "value", plan);
        out.push_str(&format!(
            "    void {set_adapter}({bridge_type} value) noexcept {{ {setter}({converted}); }}\n"
        ));
    }
}

pub(crate) fn render_cpp_swift_collection_method_adapter(
    out: &mut SourceWriter,
    interface: &BridgeInterface,
    method: &BridgeMethod,
    plan: &BridgePlan,
) {
    let success_type = method.success_type();
    let return_type = cpp_swift_collection_bridge_type(success_type, plan);
    let parameters = cpp_swift_collection_bridge_parameters(&method.parameters, plan);
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| {
            cpp_swift_collection_argument(&parameter.ty, &cpp_identifier(&parameter.name), plan)
        })
        .collect::<Vec<_>>()
        .join(", ");
    let call = format!("{}({arguments})", cpp_identifier(&method.name));
    let adapter_name = cpp_swift_class_method_adapter_name(interface, &method.name);
    if method.is_async {
        if swift_cpp_future_adapter_needed(method) {
            let future_adapter = cpp_swift_future_adapter_name(plan, interface, method);
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
    render_cpp_swift_collection_result(out, success_type, &call, plan, 2);
    out.push_str("    }\n");
}

pub(crate) fn cpp_swift_collection_bridge_type(ty: &BridgeType, plan: &BridgePlan) -> String {
    if swift_cpp_map_types(ty).is_some() {
        cpp_swift_map_entries_name(plan, ty)
    } else if swift_cpp_nested_array_supported(ty) {
        cpp_swift_array_alias_name_for_type(plan, ty)
    } else if swift_cpp_needs_vector_bridge(ty) {
        let element_name = match ty {
            BridgeType::Array(element) | BridgeType::Set(element) => {
                bridge_type_name(element).to_owned()
            }
            _ => {
                debug_assert!(
                    false,
                    "vector bridge requires a validated array or set type"
                );
                bridge_type_name(ty).to_owned()
            }
        };
        cpp_swift_array_alias_name(plan, &element_name)
    } else {
        cpp_type(ty)
    }
}

pub(crate) fn cpp_swift_collection_bridge_parameters(
    parameters: &[BridgeParameter],
    plan: &BridgePlan,
) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "{} {}",
                cpp_swift_collection_bridge_type(&parameter.ty, plan),
                cpp_identifier(&parameter.name)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn cpp_swift_collection_argument(
    ty: &BridgeType,
    argument: &str,
    plan: &BridgePlan,
) -> String {
    match bridge_type_name(ty) {
        "Map" if swift_cpp_map_types(ty).is_some() => format!(
            "{}({argument})",
            cpp_swift_map_conversion_name(plan, ty, "FromEntries")
        ),
        "Set" => {
            let element = match ty {
                BridgeType::Set(element) => element.as_ref(),
                _ => {
                    debug_assert!(
                        false,
                        "set collection argument requires a validated set type"
                    );
                    return argument.to_owned();
                }
            };
            format!(
                "std::set<{}>({argument}.begin(), {argument}.end())",
                cpp_type(element)
            )
        }
        "Array"
            if swift_cpp_array_leaf(ty).is_some_and(|leaf| {
                matches!(
                    bridge_strip_optional(leaf),
                    BridgeType::Scalar(BridgeScalar::Bool)
                )
            }) =>
        {
            format!(
                "{}({argument})",
                cpp_swift_array_conversion_name(plan, ty, "ToNative")
            )
        }
        "Array"
            if swift_cpp_nested_array_supported(ty)
                && matches!(
                    ty,
                    BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
                ) =>
        {
            format!(
                "{}({argument})",
                cpp_swift_array_conversion_name(plan, ty, "ToNative")
            )
        }
        "Array" if swift_cpp_nested_array_supported(ty) => argument.to_owned(),
        "Array" if swift_cpp_needs_vector_bridge(ty) => {
            format!("std::vector<bool>({argument}.begin(), {argument}.end())")
        }
        _ => argument.to_owned(),
    }
}

pub(crate) fn render_cpp_swift_collection_result(
    out: &mut SourceWriter,
    ty: &BridgeType,
    expression: &str,
    plan: &BridgePlan,
    indent: usize,
) {
    let prefix = "    ".repeat(indent);
    if ty.is_void() {
        out.push_str(&format!("{prefix}{expression};\n"));
    } else if swift_cpp_map_types(ty).is_some() {
        let to_entries = cpp_swift_map_conversion_name(plan, ty, "ToEntries");
        out.push_str(&format!("{prefix}return {to_entries}({expression});\n"));
    } else if swift_cpp_needs_vector_bridge(ty) {
        let alias = cpp_swift_collection_bridge_type(ty, plan);
        if matches!(bridge_strip_optional(ty), BridgeType::Array(_))
            // `Bool` leaves and nested set facades both convert element-wise
            // rather than through a plain vector range.
            && (swift_cpp_array_leaf(ty).is_some_and(|leaf| {
                matches!(
                    bridge_strip_optional(leaf),
                    BridgeType::Scalar(BridgeScalar::Bool)
                )
            }) || (swift_cpp_nested_array_supported(ty)
                && matches!(
                    ty,
                    BridgeType::Array(element) if matches!(element.as_ref(), BridgeType::Set(_))
                )))
        {
            let from_native = cpp_swift_array_conversion_name(plan, ty, "FromNative");
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

pub(crate) fn cpp_swift_array_alias_name(plan: &BridgePlan, element_type: &str) -> String {
    let base = format!("NexaCppArray{}", cpp_identifier(element_type));
    unique_cpp_type_name(plan, &base)
}

fn cpp_swift_array_alias_name_for_type(plan: &BridgePlan, ty: &BridgeType) -> String {
    let element: &BridgeType = match ty {
        BridgeType::Array(element) | BridgeType::Set(element) => element,
        _ => {
            debug_assert!(false, "Swift collection adapters require an array element");
            return cpp_swift_array_alias_name(plan, bridge_type_name(ty));
        }
    };
    if !matches!(element, BridgeType::Array(_)) {
        return cpp_swift_array_alias_name(plan, bridge_type_name(element));
    }
    fn signature(ty: &BridgeType) -> String {
        match ty {
            BridgeType::Array(inner) => {
                format!("Array{}", signature(inner))
            }
            other => cpp_identifier(bridge_type_name(other)),
        }
    }
    unique_cpp_type_name(plan, &format!("NexaCppArray{}", signature(element)))
}
