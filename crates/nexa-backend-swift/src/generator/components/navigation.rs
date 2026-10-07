use nexa_codegen::SourceWriter;
use nexa_codegen::names::navigation_case_name;
use nexa_ir::{Expr, Module, Node, ScreenId, Type};

use crate::generator::engine::expressions::expression;
use crate::generator::engine::types::swift_type;
use crate::generator::{
    components::render_children,
    utils::{indent, swift_string},
};
use crate::generator::{
    features::Features, render_immutable_state, render_native_object_state,
    render_native_object_state_uninitialized, render_state_initializers_in_init, status_bar,
};

use super::lifecycle;
use nexa_ir::State;

use super::RenderScope;

pub(crate) fn render_link(
    destination: ScreenId,
    arguments: &[Expr],
    guard: Option<&Expr>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    if matches!(guard, Some(Expr::Bool(false))) {
        render_children(children, scope.module, scope.features, depth, out);
        return;
    }
    out.line_at(
        depth,
        format_args!(
            "NavigationLink(value: {}) {{",
            route_value(destination, arguments)
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if let Some(guard) = guard {
        out.push_str(&format!(".disabled(!({}))", expression(guard)));
    }
}

pub(crate) fn render_back(
    label: &nexa_ir::Expr,
    comment: Option<&str>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("Button {{"));
    indent(out, depth + 1);
    out.push_str("nexaDismiss()\n");
    out.line_at(depth, format_args!("}} label: {{"));
    out.line_at(
        depth + 1,
        format_args!(
            "{}",
            crate::generator::expressions::localized_text_view(label, comment)
        ),
    );
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    arguments: &[Expr],
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("NavigationStack(path: $__nexaNavigationPath) {\n");
    indent(out, depth + 1);
    out.push_str(&screen_call(
        module,
        root,
        arguments,
        features,
        "Self.__nexaRootScreenIdentity",
    ));
    out.push('\n');
    indent(out, depth + 2);
    out.push_str(".navigationDestination(for: NexaNavigationRoute.self) { route in\n");
    indent(out, depth + 3);
    out.push_str("switch route {\n");
    for screen in &module.screens {
        indent(out, depth + 3);
        if screen.parameters.is_empty() {
            out.push_str(&format!(
                "case let .{}(routeIdentity):\n",
                navigation_case_name(screen.id)
            ));
        } else {
            out.push_str(&format!(
                "case let .{}(routeIdentity, {}):\n",
                navigation_case_name(screen.id),
                screen
                    .parameters
                    .iter()
                    .map(|parameter| nexa_codegen::names::state_name(&parameter.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        indent(out, depth + 4);
        let destination = screen_call(
            module,
            screen.id,
            &screen
                .parameters
                .iter()
                .map(|parameter| Expr::State(parameter.name.clone(), parameter.ty.clone()))
                .collect::<Vec<_>>(),
            features,
            "routeIdentity",
        );
        out.push_str(&format!(
            "{destination}\n.navigationTitle({})\n.nexaLargeTitleDisplayMode()\n.toolbar(.hidden, for: .tabBar)",
            swift_string(&screen.name)
        ));
        out.push('\n');
    }
    indent(out, depth + 3);
    out.push_str("}\n");
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str(".onOpenURL { url in\n");
    render_deep_link_dispatch(module, depth + 2, out);
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}

fn render_deep_link_dispatch(module: &Module, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    out.push_str("let scheme = url.scheme?.lowercased() ?? \"\"\n");
    indent(out, depth);
    out.push_str(
        "var segments = url.pathComponents.dropFirst().map { $0.removingPercentEncoding ?? $0 }\n",
    );
    indent(out, depth);
    out.push_str("if scheme != \"http\", scheme != \"https\", let host = url.host { segments.insert(host, at: 0) }\n");
    indent(out, depth);
    out.push_str("guard let route = segments.first?.lowercased() else { return }\n");
    indent(out, depth);
    out.push_str("switch route {\n");
    for screen in &module.screens {
        out.line_at(
            depth + 1,
            format_args!("case \"{}\":", route_slug(&screen.name)),
        );
        out.line_at(
            depth + 2,
            format_args!(
                "guard segments.count == {} else {{ return }}",
                screen.parameters.len() + 1
            ),
        );
        let mut arguments = vec!["UUID()".to_owned()];
        for (index, parameter) in screen.parameters.iter().enumerate() {
            let raw = format!("segments[{}]", index + 1);
            let name = format!("nexaArgument{index}");
            match &parameter.ty {
                Type::String => arguments.push(raw),
                Type::Bool => {
                    out.line_at(
                        depth + 2,
                        format_args!("guard let {name} = Bool({raw}) else {{ return }}"),
                    );
                    arguments.push(name);
                }
                Type::Numeric(_) => {
                    out.line_at(
                        depth + 2,
                        format_args!(
                            "guard let {name} = {}({raw}) else {{ return }}",
                            swift_type(&parameter.ty)
                        ),
                    );
                    arguments.push(name);
                }
                _ => unreachable!("screen route arguments are restricted to scalar types"),
            }
        }
        indent(out, depth + 2);
        out.push_str("__nexaNavigationPath = NavigationPath()\n");
        out.line_at(
            depth + 2,
            format_args!(
                "__nexaNavigationPath.append(NexaNavigationRoute.{}({}))",
                navigation_case_name(screen.id),
                arguments.join(", ")
            ),
        );
    }
    indent(out, depth + 1);
    out.push_str("default: break\n");
    indent(out, depth);
    out.push_str("}\n");
}

fn route_slug(name: &str) -> String {
    let mut slug = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            slug.push('-');
        }
        slug.extend(character.to_lowercase());
    }
    slug
}

pub(crate) fn render_screen_view(
    screen: &nexa_ir::Screen,
    module: &Module,
    features: &Features,
    out: &mut SourceWriter,
) {
    out.push('\n');
    let focus_bindings = crate::generator::screen_focus_bindings(screen, features);
    out.push_str(&format!(
        "private struct {}: View {{\n",
        screen_view_name(screen.id)
    ));
    for state in &module.states {
        if !screen_state_is_passed_as_binding(state, &focus_bindings) {
            continue;
        }
        out.line_at(
            1,
            format_args!(
                "{} private var {}: {}",
                if focus_bindings.contains(&state.name) {
                    "@FocusState.Binding"
                } else {
                    "@Binding"
                },
                nexa_codegen::names::state_name(&state.name),
                swift_type(&state.ty)
            ),
        );
    }
    for state in &module.states {
        if state.is_native_class_constructor_binding()
            && !state.mutable
            && !focus_bindings.contains(&state.name)
        {
            out.line_at(
                1,
                format_args!(
                    "private let {}: {}",
                    nexa_codegen::names::state_name(&state.name),
                    swift_type(&state.ty)
                ),
            );
        }
    }
    for parameter in &screen.parameters {
        out.line_at(
            1,
            format_args!(
                "private let {}: {}",
                nexa_codegen::names::state_name(&parameter.name),
                swift_type(&parameter.ty)
            ),
        );
    }
    let has_signals = screen
        .states
        .iter()
        .any(|state| matches!(state.ty, nexa_ir::Type::Signal(_)));
    for state in &screen.states {
        if state.is_native_class_constructor_binding() {
            if has_signals {
                render_native_object_state_uninitialized(state, 1, out);
            } else {
                render_native_object_state(state, 1, out);
            }
        } else if matches!(state.ty, nexa_ir::Type::Signal(_)) {
            out.line_at(
                1,
                format_args!(
                    "@StateObject private var {}: {}",
                    nexa_codegen::names::state_name(&state.name),
                    swift_type(&state.ty)
                ),
            );
        } else if state.mutable && !focus_bindings.contains(&state.name) {
            out.line_at(
                1,
                format_args!(
                    "@State private var {}: {} = {}",
                    nexa_codegen::names::state_name(&state.name),
                    swift_type(&state.ty),
                    expression(&state.initial)
                ),
            );
        }
    }
    render_immutable_state(&module.states, 1, out);
    render_immutable_state(&screen.states, 1, out);
    for binding in &focus_bindings {
        if module.states.iter().any(|state| state.name == *binding) {
            continue;
        }
        out.line_at(
            1,
            format_args!(
                "@FocusState private var {}: Bool",
                nexa_codegen::names::state_name(binding)
            ),
        );
    }
    if features.app_uses_adaptive_color {
        indent(out, 1);
        out.push_str("@Environment(\\.colorScheme) private var nexaColorScheme\n");
    }
    if features.app_uses_size_class {
        indent(out, 1);
        out.push_str("@Environment(\\.horizontalSizeClass) private var nexaHorizontalSizeClass\n");
        indent(out, 1);
        out.push_str("@Environment(\\.verticalSizeClass) private var nexaVerticalSizeClass\n");
    }
    if features.uses_navigation_back {
        indent(out, 1);
        out.push_str("@Environment(\\.dismiss) private var nexaDismiss\n");
    }
    out.push_str("\n    init(");
    let mut init_parameters = Vec::new();
    init_parameters.extend(screen.parameters.iter().map(|parameter| {
        format!(
            "_ {}: {}",
            nexa_codegen::names::state_name(&parameter.name),
            swift_type(&parameter.ty)
        )
    }));
    init_parameters.extend(module.states.iter().filter_map(|state| {
        if screen_state_is_passed_as_binding(state, &focus_bindings) {
            Some(format!(
                "{}: {}",
                nexa_codegen::names::state_name(&state.name),
                if focus_bindings.contains(&state.name) {
                    "FocusState<Bool>.Binding".to_owned()
                } else {
                    format!("Binding<{}>", swift_type(&state.ty))
                }
            ))
        } else if state.is_native_class_constructor_binding()
            && !state.mutable
            && !focus_bindings.contains(&state.name)
        {
            Some(format!(
                "{}: {}",
                nexa_codegen::names::state_name(&state.name),
                swift_type(&state.ty)
            ))
        } else {
            None
        }
    }));
    out.push_str(&init_parameters.join(", "));
    if init_parameters.is_empty() && !has_signals {
        out.push_str(") {}\n\n");
    } else {
        out.push_str(") {\n");
        for parameter in &screen.parameters {
            let name = nexa_codegen::names::state_name(&parameter.name);
            out.push_str(&format!("        self.{name} = {name}\n"));
        }
        for state in &module.states {
            if screen_state_is_passed_as_binding(state, &focus_bindings) {
                let name = nexa_codegen::names::state_name(&state.name);
                out.push_str(&format!("        self._{name} = {name}\n"));
            } else if state.is_native_class_constructor_binding()
                && !state.mutable
                && !focus_bindings.contains(&state.name)
            {
                let name = nexa_codegen::names::state_name(&state.name);
                out.push_str(&format!("        self.{name} = {name}\n"));
            }
        }
        if has_signals {
            let mut writer = nexa_codegen::SourceWriter::new();
            render_state_initializers_in_init(&screen.states, 2, &mut writer);
            out.push_str(&writer.finish());
        }
        out.push_str("    }\n\n");
    }
    out.push_str("    var body: some View {\n");
    out.push_str("        Group {\n");
    let content = screen
        .body
        .iter()
        .filter(|node| !matches!(node, Node::Toolbar { .. }))
        .cloned()
        .collect::<Vec<_>>();
    render_children(&content, module, features, 3, out);
    out.push_str("\n        }\n");
    for toolbar in screen
        .body
        .iter()
        .filter(|node| matches!(node, Node::Toolbar { .. }))
    {
        super::bottom_bar::render_navigation_toolbar(toolbar, module, features, 2, out);
        out.push('\n');
    }
    lifecycle::render_on_appear(screen.on_appear.as_deref(), screen.on_appear_async, 2, out);
    lifecycle::render_on_disappear(
        screen.on_disappear.as_deref(),
        &lifecycle::task_handles(&screen.states),
        2,
        out,
    );
    status_bar::render(screen.status_bar.or(module.status_bar), 2, out);
    out.push_str("\n    }\n");
    super::bottom_bar::render_bottom_bar_helpers(&screen.body, module, features, out);
    out.push_str("}\n");
}

fn screen_view_name(screen: ScreenId) -> String {
    format!("NexaScreen{}", screen.0)
}

fn route_value(destination: ScreenId, arguments: &[Expr]) -> String {
    let route = navigation_case_name(destination);
    let mut values = vec!["UUID()".to_owned()];
    values.extend(arguments.iter().map(expression));
    format!("NexaNavigationRoute.{route}({})", values.join(", "))
}

fn screen_call(
    module: &Module,
    screen: ScreenId,
    arguments: &[Expr],
    features: &Features,
    route_identity: &str,
) -> String {
    let destination = &module.screens[screen.0];
    let focus_bindings = crate::generator::screen_focus_bindings(destination, features);
    let app_focus_bindings = crate::generator::app_focus_bindings(module, features);
    let mut values = Vec::new();
    values.extend(arguments.iter().map(expression));
    for state in &module.states {
        let name = nexa_codegen::names::state_name(&state.name);
        if screen_state_is_passed_as_binding(state, &focus_bindings) {
            let binding = if focus_bindings.contains(&state.name) {
                format!("${name}")
            } else if app_focus_bindings.contains(&state.name)
                || state.is_native_class_constructor_binding()
            {
                format!("Binding(get: {{ {name} }}, set: {{ {name} = $0 }})")
            } else {
                format!("${name}")
            };
            values.push(format!("{name}: {binding}"));
        } else if state.is_native_class_constructor_binding()
            && !state.mutable
            && !focus_bindings.contains(&state.name)
        {
            values.push(format!("{name}: {name}"));
        }
    }
    format!(
        "{}({}).id({route_identity})",
        screen_view_name(screen),
        values.join(", ")
    )
}

fn screen_state_is_passed_as_binding(
    state: &State,
    _focus_bindings: &std::collections::BTreeSet<String>,
) -> bool {
    state.mutable
        || (state.is_native_class_instance_binding()
            && !state.is_native_class_constructor_binding())
}
