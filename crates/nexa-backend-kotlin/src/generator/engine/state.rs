use nexa_ir::{NumericType, Type};

use super::expressions::expression;

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
use crate::generator::engine::types::kotlin_type;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_mutable_state || features.uses_list_end_reached,
        "androidx.compose.runtime.getValue",
    );
    imports.add(
        features.uses_mutable_state || features.uses_list_end_reached,
        "androidx.compose.runtime.setValue",
    );
    imports.add(
        features.uses_mutable_state
            || features.uses_mutable_collection
            || features.uses_native_class_instance
            || features.uses_list_end_reached,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_mutable_list,
        "androidx.compose.runtime.mutableStateListOf",
    );
    imports.add(
        features.uses_mutable_set,
        "androidx.compose.runtime.mutableStateSetOf",
    );
    imports.add(
        features.uses_mutable_map,
        "androidx.compose.runtime.mutableStateMapOf",
    );
    imports.add(
        features.uses_mutable_int_state,
        "androidx.compose.runtime.mutableIntStateOf",
    );
    imports.add(
        features.uses_mutable_long_state,
        "androidx.compose.runtime.mutableLongStateOf",
    );
    imports.add(
        features.uses_mutable_float_state,
        "androidx.compose.runtime.mutableFloatStateOf",
    );
    imports.add(
        features.uses_mutable_double_state,
        "androidx.compose.runtime.mutableDoubleStateOf",
    );
    imports.add(
        features.uses_mutable_generic_state,
        "androidx.compose.runtime.mutableStateOf",
    );
}

pub(crate) fn is_mutable_collection(state: &nexa_ir::State) -> bool {
    state.mutable && matches!(&state.ty, Type::Array(_) | Type::Set(_) | Type::Map(_, _))
}

pub(crate) fn kotlin_state_initializer(state: &nexa_ir::State) -> String {
    match &state.ty {
        Type::Array(element) => array_initializer(&state.initial, element),
        Type::Set(element) => set_initializer(&state.initial, element),
        Type::Map(key, value) => map_initializer(&state.initial, key, value),
        Type::Numeric(NumericType::Int32) => {
            format!("mutableIntStateOf({})", expression(&state.initial))
        }
        Type::Numeric(NumericType::Int64) => {
            format!("mutableLongStateOf({})", expression(&state.initial))
        }
        Type::Numeric(NumericType::Float32) => {
            format!("mutableFloatStateOf({})", expression(&state.initial))
        }
        Type::Numeric(NumericType::Float64) => {
            format!("mutableDoubleStateOf({})", expression(&state.initial))
        }
        _ => format!(
            "mutableStateOf<{}>({})",
            kotlin_type(&state.ty),
            expression(&state.initial)
        ),
    }
}

fn array_initializer(initial: &nexa_ir::Expr, element: &Type) -> String {
    match initial {
        nexa_ir::Expr::Array(items) => format!(
            "mutableStateListOf<{}>({})",
            kotlin_type(&element),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateListOf<{}>().also {{ it.addAll({}) }}",
            kotlin_type(&element),
            expression(initial)
        ),
    }
}

fn set_initializer(initial: &nexa_ir::Expr, element: &Type) -> String {
    match initial {
        nexa_ir::Expr::Set(items) | nexa_ir::Expr::Array(items) => format!(
            "mutableStateSetOf<{}>({})",
            kotlin_type(&element),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateSetOf<{}>().also {{ it.addAll({}) }}",
            kotlin_type(&element),
            expression(initial)
        ),
    }
}

fn map_initializer(initial: &nexa_ir::Expr, key: &Type, value: &Type) -> String {
    match initial {
        nexa_ir::Expr::Map(entries) => format!(
            "mutableStateMapOf<{}, {}>({})",
            kotlin_type(&key),
            kotlin_type(&value),
            entries
                .iter()
                .map(|(key, value)| format!("{} to {}", expression(key), expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "mutableStateMapOf<{}, {}>().also {{ it.putAll({}) }}",
            kotlin_type(&key),
            kotlin_type(&value),
            expression(initial)
        ),
    }
}
