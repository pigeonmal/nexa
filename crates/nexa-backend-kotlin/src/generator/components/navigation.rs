use nexa_codegen::SourceWriter;
use nexa_codegen::names::{navigation_route_name, state_name};
use nexa_ir::{Expr, Module, Node, ScreenId, Type};

use crate::generator::{
    components::{appearance, render_children},
    engine::types::kotlin_type,
    expressions::text_expression,
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::{ImportContext, ImportSet};

use super::RenderScope;

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    let features = context.features;
    imports.add(context.has_navigation, "android.app.Activity");
    imports.add(context.has_navigation, "android.content.Context");
    imports.add(context.has_navigation, "android.content.ContextWrapper");
    imports.add(context.has_navigation, "android.net.Uri");
    imports.add(
        context.has_navigation,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.material3.IconButton",
    );
    imports.add(context.has_navigation, "androidx.compose.material3.Text");
    imports.add(context.has_navigation, "androidx.compose.ui.Modifier");
    imports.add(
        context.has_navigation,
        "androidx.compose.ui.platform.LocalContext",
    );
    imports.add(
        features.uses_navigation_back,
        "androidx.compose.material3.TextButton",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.foundation.clickable",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.ui.Alignment",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.material3.Icon",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.material.icons.Icons",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.material.icons.filled.ChevronRight",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_navigation_link, "androidx.compose.ui.unit.dp");
    imports.add(
        features.uses_navigation_back,
        "androidx.compose.material3.Text",
    );
    imports.add(
        features.uses_navigation_link,
        "androidx.navigation.NavHostController",
    );
    imports.add(
        context.has_navigation,
        "androidx.navigation.compose.NavHost",
    );
    imports.add(
        context.has_navigation,
        "androidx.navigation.compose.composable",
    );
    imports.add(
        context.has_navigation,
        "androidx.navigation.compose.rememberNavController",
    );
}

pub(crate) fn render_back(label: &nexa_ir::Expr, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    out.push_str("TextButton(onClick = { navController.popBackStack() }) {\n");
    out.line_at(depth + 1, format_args!("Text({})", text_expression(label)));
    indent(out, depth);
    out.push('}');
}

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
    let route = route_value(scope.module, destination, arguments);
    let enabled = guard.map_or_else(
        || "true".to_owned(),
        crate::generator::engine::expressions::expression,
    );
    out.line_at(depth, format_args!("NexaNavigationLinkPrimitive("));
    out.line_at(depth + 1, format_args!("enabled = {enabled},"));
    out.line_at(
        depth + 1,
        format_args!("onClick = {{ navController.navigate({route}) }},"),
    );
    out.line_at(depth + 1, format_args!("trailing = {{"));
    out.line_at(
        depth + 2,
        format_args!(
            "Icon(imageVector = Icons.Filled.ChevronRight, contentDescription = null, modifier = Modifier.size(16.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)"
        ),
    );
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(") {{"));
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_toolbar_link(
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
    let route = route_value(scope.module, destination, arguments);
    let enabled = guard.map_or_else(
        || "true".to_owned(),
        crate::generator::engine::expressions::expression,
    );
    out.line_at(
        depth,
        format_args!(
            "IconButton(onClick = {{ navController.navigate({route}) }}, enabled = {enabled}) {{"
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    crate::generator::utils::indent(out, depth);
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
    let app_appearance_mode = appearance::module_mode(module, root);
    indent(out, depth);
    out.push_str("val navController = rememberNavController()\n");
    indent(out, depth);
    out.push_str("NavHost(\n");
    indent(out, depth + 1);
    out.push_str("navController = navController,\n");
    out.line_at(
        depth + 1,
        format_args!(
            "startDestination = {},",
            route_value(module, root, arguments)
        ),
    );
    indent(out, depth);
    out.push_str(") {\n");
    for screen in &module.screens {
        indent(out, depth + 1);
        let route = route_pattern(screen);
        if screen.parameters.is_empty() {
            out.push_str(&format!(
                "composable(route = {route}) {{\n",
                route = kotlin_string(&route)
            ));
        } else {
            out.push_str(&format!(
                "composable(route = {}) {{ backStackEntry ->\n",
                kotlin_string(&route)
            ));
            for parameter in &screen.parameters {
                out.line_at(
                    depth + 2,
                    format_args!(
                        "val {} = {}",
                        state_name(&parameter.name),
                        route_parameter_expression(parameter)
                    ),
                );
            }
        }
        if features.uses_tasks {
            out.line_at(
                depth + 2,
                format_args!("val nexaTaskScope = rememberCoroutineScope()"),
            );
        }
        for state in &screen.states {
            render_screen_state(state, depth + 2, out);
        }
        let mut animated_targets = crate::generator::state::animated_state_targets(&screen.body);
        for actions in [screen.on_appear.as_deref(), screen.on_disappear.as_deref()]
            .into_iter()
            .flatten()
        {
            crate::generator::state::add_animated_state_targets(actions, &mut animated_targets);
        }
        crate::generator::state::render_animated_state_aliases(
            &screen.states,
            &animated_targets,
            depth + 2,
            out,
        );
        let focus_bindings = features
            .facts
            .focus_bindings
            .screens
            .get(&screen.name)
            .cloned()
            .unwrap_or_default();
        for binding in &focus_bindings {
            out.line_at(
                depth + 2,
                format_args!(
                    "val {} = remember {{ FocusRequester() }}",
                    crate::generator::input::focus_requester_name(binding)
                ),
            );
        }
        for binding in &focus_bindings {
            let state = state_name(binding);
            let requester = crate::generator::input::focus_requester_name(binding);
            out.line_at(depth + 2, format_args!("LaunchedEffect({state}) {{ if ({state}) {requester}.requestFocus() else {requester}.freeFocus() }}"
            ));
        }
        if features.uses_shared_elements {
            out.line_at(
                depth + 2,
                format_args!("val nexaAnimatedVisibilityScope = this"),
            );
        }
        let theme_depth = depth + 2;
        let themed_content_depth = if app_appearance_mode.is_some() {
            theme_depth + 2
        } else {
            theme_depth
        };
        if let Some(mode) = &app_appearance_mode {
            appearance::render_theme_start(mode, theme_depth, out);
        }
        if screen.id == root {
            render_screen_body(screen, module, features, themed_content_depth, out);
        } else {
            out.line_at(
                themed_content_depth,
                format_args!("NexaNavigationScreenPrimitive("),
            );
            out.line_at(
                themed_content_depth + 1,
                format_args!("title = {},", kotlin_string(&screen.name)),
            );
            out.line_at(
                themed_content_depth + 1,
                format_args!("onBack = {{ navController.popBackStack() }},"),
            );
            out.line_at(themed_content_depth, format_args!(") {{"));
            render_screen_body(screen, module, features, themed_content_depth + 1, out);
            out.push('\n');
            out.line_at(themed_content_depth, format_args!("}}"));
        }
        if app_appearance_mode.is_some() {
            appearance::render_theme_end(theme_depth, out);
        }
        if screen.on_appear.is_some() || screen.on_disappear.is_some() {
            out.push('\n');
        }
        crate::generator::components::lifecycle::render_on_appear(
            screen.on_appear.as_deref(),
            depth + 2,
            out,
        );
        crate::generator::components::lifecycle::render_on_disappear(
            screen.on_disappear.as_deref(),
            depth + 2,
            out,
        );
        crate::generator::components::lifecycle::render_task_cancellation_on_dispose(
            &screen.states,
            depth + 2,
            out,
        );
        if screen.status_bar.or(module.status_bar).is_some() {
            out.push('\n');
        }
        crate::generator::components::status_bar::render(
            screen.status_bar.or(module.status_bar),
            features.uses_status_bar,
            depth + 2,
            out,
        );
        if screen.on_appear.is_none() && screen.on_disappear.is_none() {
            out.push('\n');
        }
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    indent(out, depth);
    out.push('}');
    out.push('\n');
    render_deep_link_dispatch(module, depth, out);
}

fn render_screen_body(
    screen: &nexa_ir::Screen,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if features.uses_shared_elements {
        out.line_at(
            depth,
            format_args!(
                "CompositionLocalProvider(LocalNexaAnimatedVisibilityScope provides nexaAnimatedVisibilityScope) {{"
            ),
        );
        render_children(&screen.body, module, features, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
    } else {
        render_children(&screen.body, module, features, depth, out);
    }
}

fn render_deep_link_dispatch(module: &Module, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    out.push_str("var nexaContext: Context = LocalContext.current\n");
    indent(out, depth);
    out.push_str("while (nexaContext is ContextWrapper && nexaContext !is Activity) nexaContext = nexaContext.baseContext\n");
    indent(out, depth);
    out.push_str("val nexaDeepLink = (nexaContext as? Activity)?.intent?.data\n");
    indent(out, depth);
    out.push_str("LaunchedEffect(nexaDeepLink) {\n");
    indent(out, depth + 1);
    out.push_str("val uri = nexaDeepLink ?: return@LaunchedEffect\n");
    indent(out, depth + 1);
    out.push_str("val segments = buildList {\n");
    indent(out, depth + 2);
    out.push_str("if (uri.scheme !in listOf(\"http\", \"https\")) uri.host?.let(::add)\n");
    indent(out, depth + 2);
    out.push_str("addAll(uri.pathSegments)\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("when {\n");
    for screen in &module.screens {
        out.line_at(
            depth + 2,
            format_args!(
                "segments.size == {} && segments[0].equals({}, ignoreCase = true) -> {{",
                screen.parameters.len() + 1,
                kotlin_string(&route_slug(&screen.name))
            ),
        );
        let mut arguments = Vec::new();
        for (index, parameter) in screen.parameters.iter().enumerate() {
            let name = format!("nexaArgument{index}");
            let raw = format!("segments[{}]", index + 1);
            let parser = match &parameter.ty {
                Type::String => {
                    out.line_at(
                        depth + 3,
                        format_args!("val {name} = {raw}.ifEmpty {{ return@LaunchedEffect }}"),
                    );
                    arguments.push(name);
                    continue;
                }
                Type::Bool => "toBooleanStrictOrNull",
                Type::Numeric(numeric) => match numeric {
                    nexa_ir::NumericType::Int8 => "toByteOrNull",
                    nexa_ir::NumericType::Int16 => "toShortOrNull",
                    nexa_ir::NumericType::Int32 => "toIntOrNull",
                    nexa_ir::NumericType::Int64 => "toLongOrNull",
                    nexa_ir::NumericType::UInt8 => "toUByteOrNull",
                    nexa_ir::NumericType::UInt16 => "toUShortOrNull",
                    nexa_ir::NumericType::UInt32 => "toUIntOrNull",
                    nexa_ir::NumericType::UInt64 => "toULongOrNull",
                    nexa_ir::NumericType::Float32 => "toFloatOrNull",
                    nexa_ir::NumericType::Float64 => "toDoubleOrNull",
                },
                _ => unreachable!("screen route arguments are restricted to scalar types"),
            };
            out.line_at(
                depth + 3,
                format_args!("val {name} = {raw}.{parser}() ?: return@LaunchedEffect"),
            );
            arguments.push(name);
        }
        let mut route = kotlin_string(&navigation_route_name(screen.id));
        for argument in arguments {
            route.push_str(&format!(" + \"/\" + Uri.encode({argument}.toString())"));
        }
        out.line_at(depth + 3, format_args!("navController.navigate({route})"));
        indent(out, depth + 2);
        out.push_str("}\n");
    }
    indent(out, depth + 1);
    out.push_str("}\n");
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

fn render_screen_state(state: &nexa_ir::State, depth: usize, out: &mut SourceWriter) {
    let name = state_name(&state.name);
    indent(out, depth);
    if state.mutable {
        if crate::generator::state::is_mutable_collection(state) {
            out.push_str(&format!(
                "val {name} = remember {{ {} }}\n",
                crate::generator::state::kotlin_state_initializer(state)
            ));
        } else {
            out.push_str(&format!(
                "var {name} by remember {{ {} }}\n",
                crate::generator::state::kotlin_state_initializer(state)
            ));
        }
    } else if state.is_native_class_instance_binding()
        || matches!(state.ty, nexa_ir::Type::Signal(_))
    {
        out.push_str(&format!(
            "val {name}: {} = remember {{ {} }}\n",
            kotlin_type(&state.ty),
            crate::generator::expressions::expression(&state.initial)
        ));
    } else {
        out.push_str(&format!(
            "val {name}: {} = {}\n",
            kotlin_type(&state.ty),
            crate::generator::expressions::expression(&state.initial)
        ));
    }
}

fn route_pattern(screen: &nexa_ir::Screen) -> String {
    let mut route = navigation_route_name(screen.id);
    for parameter in &screen.parameters {
        route.push('/');
        route.push('{');
        route.push_str(&state_name(&parameter.name));
        route.push('}');
    }
    route
}

fn route_value(module: &Module, destination: ScreenId, arguments: &[Expr]) -> String {
    let screen = &module.screens[destination.0];
    if arguments.is_empty() {
        return kotlin_string(&navigation_route_name(destination));
    }
    let mut value = kotlin_string(&navigation_route_name(destination));
    for (argument, parameter) in arguments.iter().zip(&screen.parameters) {
        value.push_str(" + \"/\" + ");
        let rendered = crate::generator::engine::expressions::expression(argument);
        if matches!(parameter.ty, Type::String) {
            value.push_str(&format!("Uri.encode({rendered})"));
        } else {
            value.push_str(&rendered);
        }
    }
    value
}

fn route_parameter_expression(parameter: &nexa_ir::FunctionParameter) -> String {
    let key = state_name(&parameter.name);
    let value = format!("requireNotNull(backStackEntry.arguments?.getString(\"{key}\"))");
    match parameter.ty {
        Type::String => value,
        Type::Bool => format!("{value}.toBoolean()"),
        Type::Numeric(numeric) => match numeric {
            nexa_ir::NumericType::Int8 => format!("{value}.toByte()"),
            nexa_ir::NumericType::Int16 => format!("{value}.toShort()"),
            nexa_ir::NumericType::Int32 => format!("{value}.toInt()"),
            nexa_ir::NumericType::Int64 => format!("{value}.toLong()"),
            nexa_ir::NumericType::UInt8 => format!("{value}.toUByte()"),
            nexa_ir::NumericType::UInt16 => format!("{value}.toUShort()"),
            nexa_ir::NumericType::UInt32 => format!("{value}.toUInt()"),
            nexa_ir::NumericType::UInt64 => format!("{value}.toULong()"),
            nexa_ir::NumericType::Float32 => format!("{value}.toFloat()"),
            nexa_ir::NumericType::Float64 => format!("{value}.toDouble()"),
        },
        _ => unreachable!("semantic lowering restricts route parameters to scalar types"),
    }
}
