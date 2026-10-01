use std::collections::HashSet;

use nexa_codegen::SourceWriter;
use nexa_ir::walk::{IrVisitor, walk_action_children};
use nexa_ir::{Action, Module, Node, NumericType, State, Type};

use super::expressions::expression;

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
use crate::generator::engine::types::kotlin_type;

pub(crate) fn animated_state_targets(nodes: &[Node]) -> HashSet<String> {
    struct Collector(HashSet<String>);
    impl IrVisitor for Collector {
        fn visit_action(&mut self, action: &Action) {
            if let Action::WithAnimation {
                animated_states, ..
            } = action
            {
                self.0.extend(animated_states.iter().cloned());
            }
            walk_action_children(action, self);
        }
    }
    let mut collector = Collector(HashSet::new());
    collector.visit_nodes(nodes);
    collector.0
}

pub(crate) fn add_animated_state_targets(actions: &[Action], targets: &mut HashSet<String>) {
    struct Collector<'a>(&'a mut HashSet<String>);
    impl IrVisitor for Collector<'_> {
        fn visit_action(&mut self, action: &Action) {
            if let Action::WithAnimation {
                animated_states, ..
            } = action
            {
                self.0.extend(animated_states.iter().cloned());
            }
            walk_action_children(action, self);
        }
    }
    Collector(targets).visit_actions(actions);
}

pub(crate) fn app_animated_state_targets(module: &Module) -> HashSet<String> {
    let app_state_names = module
        .states
        .iter()
        .map(|state| state.name.as_str())
        .collect::<HashSet<_>>();
    let mut targets = animated_state_targets(&module.body);
    for actions in [
        module.on_appear.as_deref(),
        module.on_disappear.as_deref(),
        module.on_active.as_deref(),
        module.on_inactive.as_deref(),
        module.on_background.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        add_animated_state_targets(actions, &mut targets);
    }
    for screen in &module.screens {
        let mut screen_targets = animated_state_targets(&screen.body);
        for actions in [screen.on_appear.as_deref(), screen.on_disappear.as_deref()]
            .into_iter()
            .flatten()
        {
            add_animated_state_targets(actions, &mut screen_targets);
        }
        targets.extend(
            screen_targets
                .into_iter()
                .filter(|name| app_state_names.contains(name.as_str())),
        );
    }
    targets
}

pub(crate) fn module_uses_scoped_animation(module: &Module) -> bool {
    struct Finder(bool);
    impl IrVisitor for Finder {
        fn visit_action(&mut self, action: &Action) {
            if let Action::WithAnimation {
                animated_states, ..
            } = action
            {
                self.0 |= !animated_states.is_empty();
            }
            walk_action_children(action, self);
        }
    }
    let mut finder = Finder(false);
    finder.visit_nodes(&module.body);
    for actions in [
        module.on_appear.as_deref(),
        module.on_disappear.as_deref(),
        module.on_active.as_deref(),
        module.on_inactive.as_deref(),
        module.on_background.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        finder.visit_actions(actions);
    }
    for screen in &module.screens {
        finder.visit_nodes(&screen.body);
        for actions in [screen.on_appear.as_deref(), screen.on_disappear.as_deref()]
            .into_iter()
            .flatten()
        {
            finder.visit_actions(actions);
        }
    }
    for component in &module.components {
        finder.visit_nodes(&component.body);
    }
    module.background_tasks.iter().any(|task| {
        let mut local_finder = Finder(false);
        local_finder.visit_actions(&task.actions);
        local_finder.0
    }) || finder.0
}

pub(crate) fn render_animated_state_aliases(
    states: &[State],
    targets: &HashSet<String>,
    depth: usize,
    out: &mut SourceWriter,
) {
    for state in states {
        if !targets.contains(&state.name)
            || !state.mutable
            || !matches!(
                &state.ty,
                Type::Numeric(NumericType::Float32 | NumericType::Float64)
            )
        {
            continue;
        }
        let name = nexa_codegen::names::state_name(&state.name);
        let animation_spec = format!("{name}AnimationSpec");
        let animated = format!("{name}Animated");
        let label = crate::generator::utils::kotlin_string(&state.name);
        let prefix = "    ".repeat(depth);
        out.push_str(&format!(
            "{prefix}var {animation_spec} by remember {{\n{prefix}    androidx.compose.runtime.mutableStateOf<androidx.compose.animation.core.AnimationSpec<Float>>(\n{prefix}        androidx.compose.animation.core.snap()\n{prefix}    )\n{prefix}}}\n"
        ));
        out.push_str(&format!(
            "{prefix}val {animated} by androidx.compose.animation.core.animateFloatAsState(\n{prefix}    targetValue = {name}.toFloat(),\n{prefix}    animationSpec = {animation_spec},\n{prefix}    finishedListener = {{ _ -> {animation_spec} = androidx.compose.animation.core.snap() }},\n{prefix}    label = {label}\n{prefix})\n"
        ));
    }
}

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
            kotlin_type(element),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateListOf<{}>().also {{ it.addAll({}) }}",
            kotlin_type(element),
            expression(initial)
        ),
    }
}

fn set_initializer(initial: &nexa_ir::Expr, element: &Type) -> String {
    match initial {
        nexa_ir::Expr::Set(items) | nexa_ir::Expr::Array(items) => format!(
            "mutableStateSetOf<{}>({})",
            kotlin_type(element),
            items.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
        _ => format!(
            "mutableStateSetOf<{}>().also {{ it.addAll({}) }}",
            kotlin_type(element),
            expression(initial)
        ),
    }
}

fn map_initializer(initial: &nexa_ir::Expr, key: &Type, value: &Type) -> String {
    match initial {
        nexa_ir::Expr::Map(entries) => format!(
            "mutableStateMapOf<{}, {}>({})",
            kotlin_type(key),
            kotlin_type(value),
            entries
                .iter()
                .map(|(key, value)| format!("{} to {}", expression(key), expression(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "mutableStateMapOf<{}, {}>().also {{ it.putAll({}) }}",
            kotlin_type(key),
            kotlin_type(value),
            expression(initial)
        ),
    }
}
