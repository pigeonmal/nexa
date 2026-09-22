use nexa_ir::{NumericType, Type};

use super::expressions::expression;

pub(super) fn is_mutable_collection(state: &nexa_ir::State) -> bool {
    state.mutable && matches!(&state.ty, Type::Array(_) | Type::Set(_) | Type::Map(_, _))
}

pub(super) fn kotlin_state_initializer(state: &nexa_ir::State) -> String {
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
        _ => format!(
            "mutableStateOf<{}>({})",
            state.ty.kotlin(),
            expression(&state.initial)
        ),
    }
}

fn array_initializer(initial: &nexa_ir::Expr, element: &Type) -> String {
    match initial {
        nexa_ir::Expr::Array(items) => format!(
            "mutableStateListOf<{}>({})",
            element.kotlin(),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateListOf<{}>().also {{ it.addAll({}) }}",
            element.kotlin(),
            expression(initial)
        ),
    }
}

fn set_initializer(initial: &nexa_ir::Expr, element: &Type) -> String {
    match initial {
        nexa_ir::Expr::Set(items) | nexa_ir::Expr::Array(items) => format!(
            "mutableStateSetOf<{}>({})",
            element.kotlin(),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateSetOf<{}>().also {{ it.addAll({}) }}",
            element.kotlin(),
            expression(initial)
        ),
    }
}

fn map_initializer(initial: &nexa_ir::Expr, key: &Type, value: &Type) -> String {
    match initial {
        nexa_ir::Expr::Map(entries) => format!(
            "mutableStateMapOf<{}, {}>({})",
            key.kotlin(),
            value.kotlin(),
            entries
                .iter()
                .map(|(key, value)| format!("{} to {}", expression(key), expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "mutableStateMapOf<{}, {}>().also {{ it.putAll({}) }}",
            key.kotlin(),
            value.kotlin(),
            expression(initial)
        ),
    }
}
