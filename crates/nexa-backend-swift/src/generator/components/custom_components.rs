use nexa_codegen::SourceWriter;
use nexa_ir::{Component, LayoutKind, Module, Node, Type, ViewStyle, walk::walk_ir};

use crate::generator::{
    components::render_node, engine::types::swift_type, features::Features, layout, lifecycle,
    render_component_ref_state, render_immutable_state, render_native_object_state,
    render_native_object_state_uninitialized, render_state_initializers_in_init,
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
    out.push_str(&format!("\n@MainActor\npublic final class {name} {{\n"));
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| swift_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ");
        let result = swift_type(&method.return_type);
        let async_keyword = if method.is_async { " async" } else { "" };
        let throwing_keyword = if method.is_throwing { " throws" } else { "" };
        out.push_str(&format!(
            "    private var {method_name}Action: (({arguments}){async_keyword}{throwing_keyword} -> {result})?\n"
        ));
    }
    out.push_str("\n    public init() {}\n\n");
    for method in &component.ref_methods {
        let method_name = component_ref_method_name(method);
        let action_name = format!("{method_name}Action");
        let attach_name = format!("attach{}", capitalize(&method_name));
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| swift_type(&parameter.ty))
            .collect::<Vec<_>>()
            .join(", ");
        let parameters = method
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "_ {}: {}",
                    nexa_codegen::names::state_name(&parameter.name),
                    swift_type(&parameter.ty)
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
        let action_result = swift_type(&method.return_type);
        let (result, failure, returns_result) = match &method.return_type {
            Type::Result(success, failure) if !method.is_throwing => {
                (swift_type(success), swift_type(failure), true)
            }
            return_type => (
                swift_type(return_type),
                method
                    .failure_type
                    .as_ref()
                    .map(swift_type)
                    .unwrap_or_else(|| "Never".to_owned()),
                false,
            ),
        };
        let async_keyword = if method.is_async { " async" } else { "" };
        let throwing_keyword = if method.is_throwing { " throws" } else { "" };
        out.push_str(&format!(
            "    func {attach_name}(_ action: @escaping ({arguments}){async_keyword}{throwing_keyword} -> {action_result}) {{ self.{action_name} = action }}\n"
        ));
        out.push_str(&format!(
            "    public func {method_name}({parameters}){async_keyword} -> Result<{result}, NexaComponentRefError<{failure}>> {{\n"
        ));
        out.push_str(&format!(
            "        guard let action = {action_name} else {{ return .failure(.notAttached) }}\n"
        ));
        let call = if method.is_async {
            format!("await action({call_arguments})")
        } else {
            format!("action({call_arguments})")
        };
        if method.is_throwing {
            let call = if method.is_async {
                format!("try await action({call_arguments})")
            } else {
                format!("try action({call_arguments})")
            };
            out.push_str(&format!(
                "        do {{ return .success({call}) }}\n        catch let failure as {failure} {{ return .failure(.failure(failure)) }}\n        catch {{ preconditionFailure(\"component ref method threw an undeclared error\") }}\n    }}\n\n"
            ));
        } else if returns_result {
            out.push_str(&format!(
                "        switch {call} {{\n        case .success(let value): return .success(value)\n        case .failure(let failure): return .failure(.failure(failure))\n        }}\n    }}\n\n"
            ));
        } else {
            out.push_str(&format!("        return .success({call})\n    }}\n\n"));
        }
    }
    out.push_str("    func detach() {\n");
    for method in &component.ref_methods {
        out.push_str(&format!(
            "        {}Action = nil\n",
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
    let name = nexa_codegen::names::component_name(&component.name);
    let ref_type = component_ref_type_name(&component.name);
    let has_content_slot = component_has_content_slot(component);
    let needs_ios16 = component_uses_fast_list(component);
    if needs_ios16 {
        out.push_str("\n@available(iOS 16.0, *)\n");
    } else {
        out.push('\n');
    }
    if has_content_slot {
        out.push_str(&format!(
            "private struct {name}<SlotContent: View>: View {{\n"
        ));
    } else {
        out.push_str(&format!("private struct {name}: View {{\n"));
    }

    let cancel_task_scope = features
        .facts
        .untracked_task_launch_scopes
        .components
        .contains(&component.name);
    if cancel_task_scope {
        out.push_str("    @State private var nexaTaskScope = NexaTaskScope()\n");
    }

    for parameter in &component.parameters {
        out.push_str(&format!(
            "    private let {}: {}\n",
            nexa_codegen::names::state_name(&parameter.name),
            swift_type(&parameter.ty)
        ));
    }
    if has_content_slot {
        out.push_str("    private let nexaContent: () -> SlotContent\n");
    }
    if !component.ref_methods.is_empty() {
        out.push_str(&format!("    private let nexaComponentRef: {ref_type}?\n"));
    }
    let has_signals = component
        .states
        .iter()
        .any(|state| matches!(state.ty, nexa_ir::Type::Signal(_)));
    for state in &component.states {
        if render_component_ref_state(state, 1, out) {
            continue;
        }
        if state.is_native_class_constructor_binding() {
            if has_signals {
                render_native_object_state_uninitialized(state, 1, out);
            } else {
                render_native_object_state(state, 1, out);
            }
            continue;
        }
        if matches!(state.ty, nexa_ir::Type::Signal(_)) {
            out.push_str(&format!(
                "    @StateObject private var {}: {}\n",
                nexa_codegen::names::state_name(&state.name),
                swift_type(&state.ty),
            ));
            continue;
        }
        if state.mutable {
            out.push_str(&format!(
                "    @State private var {}: {} = {}\n",
                nexa_codegen::names::state_name(&state.name),
                swift_type(&state.ty),
                crate::generator::engine::expressions::expression(&state.initial)
            ));
        }
    }
    render_immutable_state(&component.states, 1, out);
    let uses_adaptive_color = features.component_uses_adaptive_color(&component.name);
    let uses_size_class = features.component_uses_size_class(&component.name);
    if uses_adaptive_color {
        out.push_str("    @Environment(\\.colorScheme) private var nexaColorScheme\n");
    }
    if uses_size_class {
        out.push_str(
            "    @Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n",
        );
        out.push_str("    @Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n");
    }
    let uses_window_metrics = features.component_uses_window_metrics(&component.name);
    if uses_window_metrics {
        out.push_str("    @Environment(\\.nexaWindowMetrics) private var nexaWindowMetrics\n");
        out.push_str("    @Environment(\\.layoutDirection) private var layoutDirection\n");
    }
    if features.component_uses_navigation(&component.name) {
        out.push_str("    @Environment(\\.nexaNavigationPath) private var nexaNavigationPath\n");
    }

    if !component.parameters.is_empty()
        || !component.states.is_empty()
        || has_content_slot
        || uses_adaptive_color
        || uses_size_class
        || uses_window_metrics
        || features.component_uses_navigation(&component.name)
    {
        out.push('\n');
    }
    out.push_str("    init(");
    let mut init_parameters = component
        .parameters
        .iter()
        .map(|parameter| {
            format!(
                "_ {}: {}",
                nexa_codegen::names::state_name(&parameter.name),
                swift_type(&parameter.ty)
            )
        })
        .collect::<Vec<_>>();
    if has_content_slot {
        init_parameters.push("@ViewBuilder nexaContent: @escaping () -> SlotContent".to_owned());
    }
    if !component.ref_methods.is_empty() {
        let ref_parameter = format!("nexaComponentRef: {ref_type}? = nil");
        if has_content_slot {
            let last = init_parameters.pop();
            init_parameters.push(ref_parameter);
            if let Some(last) = last {
                init_parameters.push(last);
            }
        } else {
            init_parameters.push(ref_parameter);
        }
    }
    out.push_str(&init_parameters.join(", "));
    if init_parameters.is_empty() && !has_signals {
        out.push_str(") {}\n\n");
    } else {
        out.push_str(") {\n");
        for parameter in &component.parameters {
            let name = nexa_codegen::names::state_name(&parameter.name);
            out.push_str(&format!("        self.{name} = {name}\n"));
        }
        if has_content_slot {
            out.push_str("        self.nexaContent = nexaContent\n");
        }
        if !component.ref_methods.is_empty() {
            out.push_str("        self.nexaComponentRef = nexaComponentRef\n");
        }
        if has_signals {
            let mut writer = nexa_codegen::SourceWriter::new();
            render_state_initializers_in_init(&component.states, 2, &mut writer);
            out.push_str(&writer.finish());
        }
        out.push_str("    }\n\n");
    }

    out.push_str("    var body: some View {\n");
    if component.on_appear.is_some()
        || component.on_disappear.is_some()
        || !component.ref_methods.is_empty()
    {
        // Attach lifecycle modifiers to a concrete View value. Chaining a
        // modifier after a root `if` result builder is ambiguous in Swift.
        out.push_str("        Group {\n");
        render_body(&component.body, module, features, 3, out);
        out.push_str("\n        }\n");
    } else {
        render_body(&component.body, module, features, 2, out);
    }
    lifecycle::render_on_appear(
        component.on_appear.as_deref(),
        component.on_appear_async,
        2,
        out,
    );
    lifecycle::render_on_disappear(
        component.on_disappear.as_deref(),
        &lifecycle::task_handles(&component.states),
        cancel_task_scope,
        2,
        out,
    );
    if !component.ref_methods.is_empty() {
        out.push_str("\n        .onAppear {\n");
        for method in &component.ref_methods {
            let method_name = component_ref_method_name(method);
            let attach_name = format!("attach{}", capitalize(&method_name));
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| nexa_codegen::names::state_name(&parameter.name))
                .collect::<Vec<_>>();
            out.push_str(&format!("            nexaComponentRef?.{attach_name} {{"));
            if !parameters.is_empty() {
                out.push(' ');
                out.push_str(&parameters.join(", "));
                out.push_str(" in");
            }
            out.push('\n');
            crate::generator::components::controls::render_actions(
                method.body_actions.as_deref().unwrap_or_default(),
                4,
                out,
            );
            out.push_str("            }\n");
        }
        out.push_str("        }\n        .onDisappear { nexaComponentRef?.detach() }");
    }
    out.push_str("\n    }");
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

fn component_uses_fast_list(component: &Component) -> bool {
    let mut found = false;
    walk_ir(
        &component.body,
        &mut |node| found |= matches!(node, Node::FastList { .. }),
        &mut |_| {},
    );
    found
}

fn render_body(
    body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match body {
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
