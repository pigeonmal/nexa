use nexa_ir::{NumericType, Type};

use super::expressions::expression;

pub(super) fn kotlin_state_initializer(state: &nexa_ir::State) -> String {
    match state.ty {
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
