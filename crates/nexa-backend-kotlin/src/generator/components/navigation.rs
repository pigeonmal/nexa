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
