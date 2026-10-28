use nexa_codegen::SourceWriter;
use nexa_ir::walk::walk_ir;
use nexa_ir::{Component, LayoutKind, Module, Node, Type, ViewStyle};

use crate::generator::engine::types::kotlin_type;
use crate::generator::{
    components::render_node, features::Features, layout, render_opt_in_annotation, state,
    utils::indent,
};

pub(crate) fn render(module: &Module, features: &Features, out: &mut SourceWriter) {
    for component in &module.components {
        render_component_ref_type(component, out);
        render_component(component, module, features, out);
    }
}

fn render_component_ref_type(component: &Component, out: &mut SourceWriter) {
    if component.ref_methods.is_empty() {
        return;
    }
    let name = component_ref_type_name(&component.name);
    out.push_str(&format!("\npublic class {name} {{\n"));
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| kotlin_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ");
        let suspend = if method.is_async { "suspend " } else { "" };
        out.push_str(&format!(
            "    private var {method_name}Action: ({suspend}({arguments}) -> {})? = null\n",
            kotlin_type(&method.return_type)
        ));
    }
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let attach_name = format!("attach{}", capitalize(&method_name));
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| kotlin_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ");
        let params = method
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    kotlin_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let call_arguments = method
            .parameters
            .iter()
            .map(|parameter| nexa_codegen::names::state_name(&parameter.name))
            .collect::<Vec<_>>()
            .join(", ");
        let action_result = kotlin_type(&method.return_type);
        let (result, failure, returns_result) = match &method.return_type {
            Type::Result(success, failure) if !method.is_throwing => {
                (kotlin_type(success), kotlin_type(failure), true)
            }
            return_type => (
                kotlin_type(return_type),
                method
                    .failure_type
                    .as_ref()
                    .map(kotlin_type)
                    .unwrap_or_else(|| "Unit".to_owned()),
                false,
            ),
        };
        let suspend = if method.is_async { "suspend " } else { "" };
        out.push_str(&format!(
            "\n    internal fun {attach_name}(action: {suspend}({arguments}) -> {action_result}) {{ {method_name}Action = action }}\n"
        ));
        out.push_str(&format!(
            "    public {suspend}fun {method_name}({params}): NexaResult<{result}, NexaComponentRefError<{failure}>> {{\n"
        ));
        out.push_str(&format!(
            "        val action = {method_name}Action ?: return dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.NotAttached)\n"
        ));
        let call = format!("action({call_arguments})");
        if method.is_throwing {
            out.push_str(&format!(
                "        return try {{ dev.nexa.core.NexaResult.Success({call}) }} catch (failure: {failure}) {{ dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.Failure(failure)) }}\n    }}\n"
            ));
        } else if returns_result {
            out.push_str(&format!(
                "        return when (val outcome = {call}) {{\n            is dev.nexa.core.NexaResult.Success<*> -> dev.nexa.core.NexaResult.Success(outcome.value as {result})\n            is dev.nexa.core.NexaResult.Failure<*> -> dev.nexa.core.NexaResult.Failure(dev.nexa.core.NexaComponentRefError.Failure(outcome.error as {failure}))\n        }}\n    }}\n"
            ));
        } else {
            out.push_str(&format!(
                "        return dev.nexa.core.NexaResult.Success({call})\n    }}\n"
            ));
        }
    }
    out.push_str("\n    internal fun detach() {\n");
    for method in &component.ref_methods {
        out.push_str(&format!(
            "        {}Action = null\n",
            component_ref_method_name(method)
        ));
    }
    out.push_str("    }\n}\n");
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

fn component_ref_type_name(component_name: &str) -> String {
    format!("{}Ref", nexa_codegen::names::struct_name(component_name))
}

fn component_ref_method_name(method: &nexa_ir::Function) -> String {
    nexa_codegen::names::function_name(method.name.rsplit('.').next().unwrap_or(&method.name))
}

fn render_component(
    component: &Component,
    module: &Module,
    features: &Features,
    out: &mut SourceWriter,
) {
    let mut opt_in_annotations = Vec::with_capacity(3);
    if component_uses_bottom_sheet(component) {
        opt_in_annotations.push("androidx.compose.material3.ExperimentalMaterial3Api::class");
    }
    if features.component_uses_keyboard_interactive(&component.name) {
        opt_in_annotations.push("androidx.compose.foundation.layout.ExperimentalLayoutApi::class");
    }
    if component_uses_sticky_header(component) {
        opt_in_annotations.push("androidx.compose.foundation.ExperimentalFoundationApi::class");
    }
    out.push('\n');
    render_opt_in_annotation(out, &opt_in_annotations);
    out.push_str(&format!(
        "@Composable\nprivate fun {}(",
        nexa_codegen::names::component_name(&component.name)
    ));
    out.push_str(
        &component
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    kotlin_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    let needs_system_theme = features.component_requires_system_theme(&component.name);
    let needs_navigation = features.component_requires_navigation(&component.name);
    if needs_system_theme && !component.parameters.is_empty() {
        out.push_str(", ");
    }
    if needs_system_theme {
        out.push_str("nexaIsDarkTheme: Boolean");
    }
    if needs_navigation {
        if needs_system_theme || !component.parameters.is_empty() {
            out.push_str(", ");
        }
        out.push_str("navController: NavHostController");
    }
    if !component.ref_methods.is_empty() {
        if needs_system_theme || needs_navigation || !component.parameters.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&format!(
            "nexaComponentRef: {}? = null",
            component_ref_type_name(&component.name)
        ));
    }
    if component_has_content_slot(component) {
        if needs_system_theme
            || needs_navigation
            || !component.parameters.is_empty()
            || !component.ref_methods.is_empty()
        {
            out.push_str(", ");
        }
        out.push_str("nexaContent: @Composable () -> Unit");
    }
    out.push_str(") {\n");

    if features.component_uses_task_scope(&component.name) {
        out.push_str("    val nexaTaskScope = rememberCoroutineScope()\n");
    }
    if features.component_uses_link(&component.name) {
        out.push_str("    val nexaLinkContext = LocalContext.current\n");
    }
    if features.component_uses_haptic(&component.name) {
        out.push_str("    val nexaHapticView = LocalView.current\n");
    }
    render_component_states(&component.states, 1, out);
    render_component_ref_methods(component, out);
    let animated_targets = state::animated_state_targets(&component.body);
    state::render_animated_state_aliases(&component.states, &animated_targets, 1, out);
    crate::generator::components::lifecycle::render_task_cancellation_on_dispose(
        &component.states,
        1,
        out,
    );
    let focus_bindings = features
        .facts
        .focus_bindings
        .components
        .get(&component.name)
        .cloned()
        .unwrap_or_default();
    for binding in &focus_bindings {
        out.push_str(&format!(
            "    val {} = remember {{ FocusRequester() }}\n",
            crate::generator::components::input::focus_requester_name(binding)
        ));
    }
    for binding in &focus_bindings {
        let state_name = nexa_codegen::names::state_name(binding);
        let requester_name = crate::generator::components::input::focus_requester_name(binding);
        out.push_str(&format!(
            "    LaunchedEffect({state_name}) {{\n        if ({state_name}) {requester_name}.requestFocus() else {requester_name}.freeFocus()\n    }}\n"
        ));
    }
    if !focus_bindings.is_empty() {
        out.push('\n');
    }
    render_body(&component.body, module, features, 1, out);
    if component.on_appear.is_some() || component.on_disappear.is_some() {
        out.push('\n');
    }
    crate::generator::components::lifecycle::render_on_appear(
        component.on_appear.as_deref(),
        1,
        out,
    );
    crate::generator::components::lifecycle::render_on_disappear(
        component.on_disappear.as_deref(),
        1,
        out,
    );
    out.push_str("\n}\n");
}

fn component_has_content_slot(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::Content),
        &mut |_| {},
    );
    found
}

fn render_component_ref_methods(component: &Component, out: &mut SourceWriter) {
    if component.ref_methods.is_empty() {
        return;
    }
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let helper = format!("nexaRefMethod{}", capitalize(&method_name));
        let suspend = if method.is_async { "suspend " } else { "" };
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    kotlin_type(&parameter.ty)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "    {suspend}fun {helper}({parameters}): {} {{\n",
            kotlin_type(&method.return_type)
        ));
        crate::generator::components::controls::render_actions(
            method.body_actions.as_deref().unwrap_or_default(),
            2,
            out,
        );
        if method.return_type == nexa_ir::Type::Void
            && !method
                .body_actions
                .as_deref()
                .unwrap_or_default()
                .iter()
                .any(|action| matches!(action, nexa_ir::Action::Return { .. }))
        {
            out.push_str("        Unit\n");
        }
        out.push_str("    }\n");
    }
    out.push_str("    androidx.compose.runtime.SideEffect {\n");
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let attach_name = format!("attach{}", capitalize(&method_name));
        let helper = format!("nexaRefMethod{}", capitalize(&method_name));
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| nexa_codegen::names::state_name(&parameter.name))
            .collect::<Vec<_>>();
        out.push_str(&format!("        nexaComponentRef?.{attach_name} {{"));
        if !parameters.is_empty() {
            out.push(' ');
            out.push_str(&parameters.join(", "));
            out.push_str(" ->");
        }
        out.push_str(&format!(" {helper}({}) }}\n", parameters.join(", ")));
    }
    out.push_str("    }\n");
    out.push_str("    androidx.compose.runtime.DisposableEffect(nexaComponentRef) {\n");
    out.push_str("        onDispose { nexaComponentRef?.detach() }\n    }\n\n");
}

fn component_uses_bottom_sheet(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::BottomSheet { .. }),
        &mut |_| {},
    );
    found
}

fn component_uses_sticky_header(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| {
            found |= matches!(
                node,
                Node::FastList { plan } if plan.sticky_header().is_some()
            )
        },
        &mut |_| {},
    );
    found
}

fn render_component_states(states: &[nexa_ir::State], depth: usize, out: &mut SourceWriter) {
    for state in states {
        indent(out, depth);
        let name = nexa_codegen::names::state_name(&state.name);
        if state.mutable {
            if state::is_mutable_collection(state) {
                out.push_str(&format!(
                    "val {name} = remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            } else {
                out.push_str(&format!(
                    "var {name} by remember {{ {} }}\n",
                    state::kotlin_state_initializer(state)
                ));
            }
        } else if state.is_native_class_instance_binding()
            || matches!(state.ty, nexa_ir::Type::Signal(_))
            || matches!(state.ty, nexa_ir::Type::ComponentRef { .. })
        {
            out.push_str(&format!(
                "val {name}: {} = remember {{ {} }}\n",
                kotlin_type(&state.ty),
                crate::generator::engine::expressions::expression(&state.initial)
            ));
        } else {
            out.push_str(&format!(
                "val {name}: {} = {}\n",
                kotlin_type(&state.ty),
                crate::generator::engine::expressions::expression(&state.initial)
            ));
        }
    }
    if !states.is_empty() {
        out.push('\n');
    }
}

fn render_body(
    body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match body {
        [] => {}
        [node] => render_node(node, module, features, depth, out),
        children => layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            children,
            &super::RenderScope { module, features },
            depth,
            out,
        ),
    }
}
