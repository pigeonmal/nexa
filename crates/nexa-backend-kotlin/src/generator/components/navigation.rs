use nexa_codegen::names::{navigation_route_name, state_name};
use nexa_ir::{Expr, Module, Node, ScreenId, Type};

use crate::generator::{
    components::render_children,
    expressions::text_expression,
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::{ImportContext, ImportSet};

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
        "androidx.compose.ui.platform.LocalContext",
    );
    imports.add(
        features.uses_navigation_link || features.uses_navigation_back,
        "androidx.compose.material3.TextButton",
    );
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

pub(crate) fn render_back(label: &nexa_ir::Expr, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str("TextButton(onClick = { navController.popBackStack() }) {\n");
    indent(out, depth + 1);
    out.push_str(&format!("Text({})\n", text_expression(label)));
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_link(
    destination: ScreenId,
    arguments: &[Expr],
    guard: Option<&Expr>,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    if matches!(guard, Some(Expr::Bool(false))) {
        render_children(children, module, features, depth, out);
        return;
    }
    indent(out, depth);
    out.push_str(&format!(
        "TextButton(onClick = {{ navController.navigate({}) }}{}) {{\n",
        route_value(module, destination, arguments),
        guard.map_or(String::new(), |guard| {
            format!(
                ", enabled = {}",
                crate::generator::engine::expressions::expression(guard)
            )
        })
    ));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    arguments: &[Expr],
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("val navController = rememberNavController()\n");
    indent(out, depth);
    out.push_str("NavHost(\n");
    indent(out, depth + 1);
    out.push_str("navController = navController,\n");
    indent(out, depth + 1);
    out.push_str(&format!(
        "startDestination = {},\n",
        route_value(module, root, arguments)
    ));
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
                indent(out, depth + 2);
                out.push_str(&format!(
                    "val {} = {}\n",
                    state_name(&parameter.name),
                    route_parameter_expression(parameter)
                ));
            }
        }
        for state in &screen.states {
            render_screen_state(state, depth + 2, out);
        }
        let focus_bindings = crate::generator::features::collect_focus_bindings(&screen.body);
        for binding in &focus_bindings {
            indent(out, depth + 2);
            out.push_str(&format!(
                "val {} = remember {{ FocusRequester() }}\n",
                crate::generator::input::focus_requester_name(binding)
            ));
        }
        for binding in &focus_bindings {
            let state = state_name(binding);
            let requester = crate::generator::input::focus_requester_name(binding);
            indent(out, depth + 2);
            out.push_str(&format!(
                "LaunchedEffect({state}) {{ if ({state}) {requester}.requestFocus() else {requester}.freeFocus() }}\n"
            ));
        }
        render_children(&screen.body, module, features, depth + 2, out);
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

fn render_deep_link_dispatch(module: &Module, depth: usize, out: &mut String) {
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
        indent(out, depth + 2);
        out.push_str(&format!(
            "segments.size == {} && segments[0].equals({}, ignoreCase = true) -> {{\n",
            screen.parameters.len() + 1,
            kotlin_string(&route_slug(&screen.name))
        ));
        let mut arguments = Vec::new();
        for (index, parameter) in screen.parameters.iter().enumerate() {
            let name = format!("nexaArgument{index}");
            let raw = format!("segments[{}]", index + 1);
            let parser = match &parameter.ty {
                Type::String => {
                    indent(out, depth + 3);
                    out.push_str(&format!(
                        "val {name} = {raw}.ifEmpty {{ return@LaunchedEffect }}\n"
                    ));
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
            indent(out, depth + 3);
            out.push_str(&format!(
                "val {name} = {raw}.{parser}() ?: return@LaunchedEffect\n"
            ));
            arguments.push(name);
        }
        let mut route = kotlin_string(&navigation_route_name(screen.id));
        for argument in arguments {
            route.push_str(&format!(" + \"/\" + Uri.encode({argument}.toString())"));
        }
        indent(out, depth + 3);
        out.push_str(&format!("navController.navigate({route})\n"));
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

fn render_screen_state(state: &nexa_ir::State, depth: usize, out: &mut String) {
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
    } else if state.is_native_class_instance_binding() {
        out.push_str(&format!(
            "val {name}: {} = remember {{ {} }}\n",
            state.ty.kotlin(),
            crate::generator::expressions::expression(&state.initial)
        ));
    } else {
        out.push_str(&format!(
            "val {name}: {} = {}\n",
            state.ty.kotlin(),
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
