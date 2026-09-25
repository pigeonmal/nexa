//! One generic type visitor over a validated [`BridgePlan`].
//!
//! Renderers need the same thing over and over: "every type in a value
//! position in this contract", and "every type nested inside this type". The
//! collection tables behind array facades, map adapters, optional aliases, and
//! error converters each re-implemented that walk, so the member traversal
//! (constructors, properties, method parameters and returns, event
//! parameters) was copied five times and had to be kept in sync by hand.
//!
//! This module is the single traversal. Because a [`BridgePlan`] is already
//! validated, "value position" is unambiguous: every constructor parameter,
//! property, method parameter, method return, and event parameter, in
//! declaration order. Named-type fields and error payloads are reached
//! through [`walk_named_types`].
//!
//! ```
//! use nexa_codegen::plugin::bridge_plan::BridgeType;
//! use nexa_codegen::plugin::type_visit::for_each_value_type;
//! # fn example(plan: &nexa_codegen::plugin::bridge_plan::BridgePlan) {
//! let mut arrays = Vec::new();
//! for_each_value_type(plan, &mut |ty| {
//!     if matches!(ty, BridgeType::Array(_)) {
//!         arrays.push(ty.clone());
//!     }
//! });
//! # }
//! ```

use super::bridge_plan::{
    BridgeInterface, BridgeNamedType, BridgePlan, BridgeType, BridgeTypeKind,
};

/// Visits every type in a value position of every interface, in declaration
/// order: constructors, properties, method parameters, method returns, then
/// event parameters.
///
/// This is the traversal every C++ collection table and optional-alias table
/// needs; renderers filter the stream instead of re-walking members.
pub fn for_each_value_type(plan: &BridgePlan, visit: &mut impl FnMut(&BridgeType)) {
    for interface in &plan.interfaces {
        for_each_interface_value_type(interface, visit);
    }
}

/// The shared member traversal behind [`for_each_value_type`].
fn for_each_interface_value_type(interface: &BridgeInterface, visit: &mut impl FnMut(&BridgeType)) {
    for constructor in &interface.constructors {
        for parameter in &constructor.parameters {
            visit(&parameter.ty);
        }
    }
    for property in &interface.properties {
        visit(&property.ty);
    }
    for method in &interface.methods {
        for parameter in &method.parameters {
            visit(&parameter.ty);
        }
        visit(&method.return_type);
    }
    for event in &interface.events {
        for parameter in &event.parameters {
            visit(&parameter.ty);
        }
    }
}

/// Visits every declared struct, enum, and error type, in declaration order.
///
/// The yielded reference borrows from `plan`, so callers can look types up or
/// hold them for the rest of the render.
pub fn for_each_named_type<'p>(plan: &'p BridgePlan, visit: &mut impl FnMut(&'p BridgeNamedType)) {
    for ty in &plan.types {
        visit(ty);
    }
}

/// Depth-first traversal order over a type tree.
///
/// Order is observable in generated C++: a facade or adapter for a nested
/// shape must be declared before the shape that uses it, so most tables
/// collect [`Order::OuterFirst`]. Tables that emit `using` aliases for a
/// compound value and its element independently need the opposite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Visit a node before its children.
    OuterFirst,
    /// Visit a node after its children.
    ChildrenFirst,
}

/// Walks a type tree depth-first in the requested order.
///
/// `Optional` wrappers are transparent, matching how the renderers treat the
/// optional flag as a separate concern from the type shape. `Result` is
/// transparent over its success payload; the error type is a name, not a
/// type, and is reached through [`referenced_errors`].
pub fn walk_type(ty: &BridgeType, order: Order, visit: &mut impl FnMut(&BridgeType)) {
    let unwrapped = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        BridgeType::Result { success, .. } => success.as_ref(),
        other => other,
    };
    if order == Order::OuterFirst {
        visit(unwrapped);
    }
    match unwrapped {
        BridgeType::Array(element) | BridgeType::Set(element) => walk_type(element, order, visit),
        BridgeType::Map(key, value) | BridgeType::Pair(key, value) => {
            walk_type(key, order, visit);
            walk_type(value, order, visit);
        }
        BridgeType::Triple(first, second, third) => {
            walk_type(first, order, visit);
            walk_type(second, order, visit);
            walk_type(third, order, visit);
        }
        BridgeType::Scalar(_)
        | BridgeType::Named { .. }
        | BridgeType::Optional(_)
        | BridgeType::Result { .. } => {}
    }
    if order == Order::ChildrenFirst {
        visit(unwrapped);
    }
}

/// Collects the distinct types in a value position for which `select` holds,
/// preserving first-seen order.
///
/// Renderers that need a deduplicated collection table filter this stream
/// rather than re-walking members themselves.
pub fn collect_selected_value_types(
    plan: &BridgePlan,
    select: &mut impl FnMut(&BridgeType) -> bool,
) -> Vec<BridgeType> {
    let mut collected: Vec<BridgeType> = Vec::new();
    for_each_value_type(plan, &mut |ty| {
        if select(ty) && !collected.contains(ty) {
            collected.push(ty.clone());
        }
    });
    collected
}

/// Every error type a contract's service or native-class methods can
/// produce, sorted by name.
///
/// The plan precomputes this with the service/native-class scoping applied, so
/// renderers do not re-derive it and cannot disagree about which error types a
/// contract needs converters for.
pub fn referenced_errors(plan: &BridgePlan) -> &[BridgeNamedType] {
    &plan.referenced_errors
}

/// Every declared struct and enum, in declaration order: the types that
/// cross a bridge as values. Errors are excluded because they travel as
/// failure payloads, not values.
pub fn collect_value_types(plan: &BridgePlan) -> Vec<&BridgeNamedType> {
    let mut types = Vec::new();
    for_each_named_type(plan, &mut |ty| {
        if is_value_type(ty) {
            types.push(ty);
        }
    });
    types
}

/// Whether a declared type is a struct or enum, i.e. a value type rather than
/// a typed error.
pub fn is_value_type(ty: &BridgeNamedType) -> bool {
    matches!(ty.kind, BridgeTypeKind::Struct | BridgeTypeKind::Enum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::bridge_plan::{BridgeScalar, BridgeType};

    const CONTRACT: &str = r#"
        struct Point { x: Float64  y: Float64 }
        enum Grade { low  high }
        error Fault { empty  detail(code: Int32) }
        service Lookup {
            fn resolve(key: String) -> Map<Int32, Int32>
            async fn load() -> Array<Point>
        }
        native class Recorder {
            init(label: String)
            property samples: Array<Int32>
            property point: Point
            fn dispose()
        }
    "#;

    fn plan() -> BridgePlan {
        let idl = nexa_plugin_idl::parse(CONTRACT).expect("contract should parse");
        BridgePlan::validate_swift_cpp(&idl).expect("contract should validate")
    }

    fn names(types: &[BridgeType]) -> Vec<String> {
        types.iter().map(type_name).collect()
    }

    fn type_name(ty: &BridgeType) -> String {
        crate::plugin::cpp::abi::bridge_type_name(ty).to_owned()
    }

    #[test]
    fn value_positions_cover_every_interface_member_kind() {
        let plan = plan();
        let mut seen = Vec::new();
        for_each_value_type(&plan, &mut |ty| seen.push(ty.clone()));

        // Declaration order, interface by interface: the `Lookup` service
        // first (`resolve(key: String)`, its `Map` return, `load()`'s
        // `Array<Point>` return), then the `Recorder` native class
        // (`init(label: String)`, `samples`, `point`, `dispose()`).
        let kinds: Vec<&str> = seen
            .iter()
            .map(|ty| match ty {
                BridgeType::Scalar(BridgeScalar::String) => "String",
                BridgeType::Array(_) => "Array",
                BridgeType::Map(..) => "Map",
                BridgeType::Named { name, .. } => name.as_str(),
                BridgeType::Scalar(BridgeScalar::Void) => "Void",
                other => panic!("unexpected value position: {other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            ["String", "Map", "Array", "String", "Array", "Point", "Void"]
        );
    }

    #[test]
    fn walk_type_covers_every_nested_node_in_both_orders() {
        let plan = plan();
        let mut map = None;
        for_each_value_type(&plan, &mut |ty| {
            if matches!(ty, BridgeType::Map(..)) {
                map = Some(ty.clone());
            }
        });
        let map = map.expect("contract should contain a map value position");

        let mut outer_first: Vec<String> = Vec::new();
        walk_type(&map, Order::OuterFirst, &mut |ty| {
            outer_first.push(type_name(ty))
        });
        let mut children_first: Vec<String> = Vec::new();
        walk_type(&map, Order::ChildrenFirst, &mut |ty| {
            children_first.push(type_name(ty))
        });

        assert_eq!(outer_first, ["Map", "Int32", "Int32"]);
        assert_eq!(children_first, ["Int32", "Int32", "Map"]);
    }

    #[test]
    fn walk_type_treats_optional_and_result_as_transparent() {
        let optional = BridgeType::Optional(Box::new(BridgeType::Array(Box::new(
            BridgeType::Scalar(BridgeScalar::Int32),
        ))));
        let mut visited: Vec<String> = Vec::new();
        walk_type(&optional, Order::OuterFirst, &mut |ty| {
            visited.push(type_name(ty))
        });
        // The `Optional` wrapper itself is not a node renderers name.
        assert_eq!(visited, ["Array", "Int32"]);

        let result = BridgeType::Result {
            success: Box::new(BridgeType::Scalar(BridgeScalar::Bool)),
            failure: "Fault".to_owned(),
        };
        let mut visited: Vec<String> = Vec::new();
        walk_type(&result, Order::OuterFirst, &mut |ty| {
            visited.push(type_name(ty))
        });
        assert_eq!(visited, ["Bool"]);
    }

    #[test]
    fn selected_collection_deduplicates_in_first_seen_order() {
        let plan = plan();
        let arrays =
            collect_selected_value_types(&plan, &mut |ty| matches!(ty, BridgeType::Array(_)));
        assert_eq!(names(&arrays), ["Array", "Array"]);

        let distinct =
            collect_selected_value_types(&plan, &mut |ty| matches!(ty, BridgeType::Array(_)));
        assert_eq!(distinct, arrays);
    }

    #[test]
    fn value_types_exclude_declared_errors() {
        let plan = plan();
        let value_types: Vec<&str> = collect_value_types(&plan)
            .into_iter()
            .map(|ty| ty.name.as_str())
            .collect();
        assert_eq!(value_types, ["Point", "Grade"]);

        let declared: Vec<&str> = {
            let mut names = Vec::new();
            for_each_named_type(&plan, &mut |ty| names.push(ty.name.as_str()));
            names
        };
        assert_eq!(declared, ["Point", "Grade", "Fault"]);
    }

    #[test]
    fn referenced_errors_are_scoped_to_service_and_native_class_methods() {
        let plan = plan();
        // `Fault` is declared but never referenced by a throwing method, so the
        // plan keeps it out of the converter set.
        assert!(referenced_errors(&plan).is_empty());
    }
}
