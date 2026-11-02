use nexa_codegen::SourceWriter;
use nexa_codegen::names::{navigation_route_name, state_name};
use nexa_ir::{Expr, Module, Node, ScreenId, State, Type};

use crate::generator::{
    components::{appearance, bottom_bar, render_children},
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
        "androidx.compose.foundation.background",
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
    imports.add(context.has_navigation, "androidx.compose.material3.Icon");
    imports.add(
        context.has_navigation,
        "androidx.compose.material.icons.Icons",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.material.icons.automirrored.filled.ArrowBack",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.ui.input.nestedscroll.nestedScroll",
    );
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
        context.has_navigation,
        "androidx.activity.compose.BackHandler",
    );
    imports.add(context.has_navigation, "androidx.compose.runtime.getValue");
    imports.add(
        context.has_navigation,
        "androidx.navigation.compose.currentBackStackEntryAsState",
    );
    imports.add(
        context.has_navigation,
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
    imports.add(
        context.has_navigation,
        "androidx.compose.animation.AnimatedContentTransitionScope",
    );
    imports.add(
        context.has_navigation,
        "androidx.compose.animation.core.tween",
    );
}

pub(crate) fn render_back(label: &nexa_ir::Expr, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    out.push_str("TextButton(onClick = { if (navController.previousBackStackEntry != null) navController.popBackStack() }) {\n");
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
            "Icon(imageVector = Icons.Filled.ChevronRight, contentDescription = null, modifier = Modifier.size(22.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f))"
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

pub(crate) struct NavigationTitleOptions {
    pub(crate) mode: nexa_ir::NavigationTitleMode,
    pub(crate) alignment: nexa_ir::NavigationTitleAlignment,
    pub(crate) show_native_bar: bool,
}

pub(crate) fn render_navigation_stack(
    module: &Module,
    root: ScreenId,
    arguments: &[Expr],
    title: NavigationTitleOptions,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let NavigationTitleOptions {
        mode,
        alignment,
        show_native_bar,
    } = title;
    let app_appearance_mode = appearance::module_mode(module, root);
    indent(out, depth);
    out.push_str("val navController = rememberNavController()\n");
    indent(out, depth);
    out.push_str("val currentBackStackEntry by navController.currentBackStackEntryAsState()\n");
    indent(out, depth);
    out.push_str("BackHandler(enabled = currentBackStackEntry != null && navController.previousBackStackEntry != null) {\n");
    indent(out, depth + 1);
    out.push_str("navController.popBackStack()\n");
    indent(out, depth);
    out.push_str("}\n");
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
    for (parameter, direction) in [
        ("enterTransition", "Start"),
        ("exitTransition", "Start"),
        ("popEnterTransition", "End"),
        ("popExitTransition", "End"),
    ] {
        let transition = if parameter.starts_with("enter") || parameter.starts_with("popEnter") {
            "slideIntoContainer"
        } else {
            "slideOutOfContainer"
        };
        out.line_at(depth + 1, format_args!("{parameter} = {{"));
        out.line_at(depth + 2, format_args!("{transition}("));
        out.line_at(
            depth + 3,
            format_args!("towards = AnimatedContentTransitionScope.SlideDirection.{direction},"),
        );
        out.line_at(
            depth + 3,
            format_args!("animationSpec = tween(durationMillis = 350),"),
        );
        out.line_at(depth + 2, format_args!(")"));
        out.line_at(depth + 1, format_args!("}},"));
    }
    indent(out, depth);
    out.push_str(") {\n");
    for screen in &module.screens {
        indent(out, depth + 1);
        let route = route_pattern(screen);
        out.push_str(&format!(
            "composable(route = {}) {{ backStackEntry ->\n",
            kotlin_string(&route)
        ));
        let theme_depth = depth + 2;
        let themed_content_depth = if app_appearance_mode.is_some() {
            theme_depth + 2
        } else {
            theme_depth
        };
        if let Some(mode) = &app_appearance_mode {
            appearance::render_theme_start(mode, theme_depth, out);
        }
        let status_bar_config = screen.status_bar.or(module.status_bar);
        let wrap_status_bar_background =
            crate::generator::components::status_bar::render_background_start(
                status_bar_config,
                themed_content_depth,
                out,
            );
        let parameter_arguments = screen
            .parameters
            .iter()
            .map(route_parameter_expression)
            .collect::<Vec<_>>();
        render_screen_call(
            screen,
            &parameter_arguments,
            ScreenCallContext {
                is_root: screen.id == root,
                title_mode: mode,
                title_alignment: alignment,
                show_navigation_bar: show_native_bar,
            },
            features,
            themed_content_depth,
            out,
        );
        if wrap_status_bar_background {
            crate::generator::components::status_bar::render_background_end(
                themed_content_depth,
                out,
            );
        }
        if app_appearance_mode.is_some() {
            appearance::render_theme_end(theme_depth, out);
        }
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    indent(out, depth);
    out.push('}');
    out.push('\n');
    render_deep_link_dispatch(module, depth, out);
}

/// Emit each screen body once as a local composable in the app function.
/// Navigation hosts call these functions instead of inlining every route body
/// into every host. This keeps nested per-tab NavigationStacks finite while
/// retaining access to app state through Kotlin's lexical scope.
pub(crate) fn render_screen_functions(
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let opt_in_annotations = [
        "androidx.compose.material3.ExperimentalMaterial3Api::class",
        "androidx.compose.material3.ExperimentalMaterial3ExpressiveApi::class",
        "androidx.compose.foundation.layout.ExperimentalLayoutApi::class",
        "androidx.compose.foundation.ExperimentalFoundationApi::class",
    ];
    let mut screens = module.screens.iter().collect::<Vec<_>>();
    // A screen that contains its own NavigationStack calls the other screen
    // composables from inside that host. Emit those owners after their route
    // targets so Kotlin's local declarations are in scope at each call site.
    screens.sort_by_key(|screen| screen_contains_navigation_stack(screen));
    for screen in &screens {
        if !screen.states.is_empty() {
            render_route_state_type(screen, depth, out);
            out.push('\n');
        }
    }
    for screen in screens {
        out.push('\n');
        indent(out, depth);
        out.push_str(&format!("@OptIn({})\n", opt_in_annotations.join(", ")));
        out.line_at(depth, format_args!("@Composable"));
        indent(out, depth);
        out.push_str(&format!("fun {}(\n", screen_composable_name(screen.id)));
        out.line_at(depth + 1, format_args!("navController: NavHostController,"));
        out.line_at(
            depth + 1,
            format_args!("nexaBackStackEntry: androidx.navigation.NavBackStackEntry,"),
        );
        for parameter in &screen.parameters {
            out.line_at(
                depth + 1,
                format_args!(
                    "{}: {},",
                    state_name(&parameter.name),
                    kotlin_type(&parameter.ty)
                ),
            );
        }
        out.line_at(depth + 1, format_args!("nexaIsRoot: Boolean,"));
        out.line_at(depth + 1, format_args!("nexaShowNavigationBar: Boolean,"));
        out.line_at(depth + 1, format_args!("nexaTitleInline: Boolean,"));
        out.line_at(depth + 1, format_args!("nexaLeadingTitle: Boolean,"));
        if features.uses_shared_elements {
            out.line_at(
                depth + 1,
                format_args!("nexaAnimatedVisibilityScope: AnimatedVisibilityScope,"),
            );
        }
        out.line_at(depth, format_args!(") {{"));

        if features.screen_uses_task_scope(&screen.name) {
            out.line_at(
                depth + 1,
                format_args!("val nexaTaskScope = rememberCoroutineScope()"),
            );
        }
        let route_state = route_state_variable_name(screen.id);
        if !screen.states.is_empty() {
            render_route_state_access(screen, &route_state_name(screen.id), depth + 1, out);
        }
        for state in &screen.states {
            render_screen_state(state, &route_state, depth + 1, out);
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
            depth + 1,
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
                depth + 1,
                format_args!(
                    "val {} = remember {{ FocusRequester() }}",
                    crate::generator::input::focus_requester_name(binding)
                ),
            );
        }
        for binding in &focus_bindings {
            let state = state_name(binding);
            let requester = crate::generator::input::focus_requester_name(binding);
            out.line_at(depth + 1, format_args!("LaunchedEffect({state}) {{ if ({state}) {requester}.requestFocus() else {requester}.freeFocus() }}"));
        }

        let root_body = &screen.body;
        let navigation_toolbars = bottom_bar::tab_toolbars(root_body);
        let destination_body = bottom_bar::without_tab_toolbars(root_body);
        if navigation_toolbars.is_empty() {
            render_screen_content_function(
                "nexaScreenContent",
                root_body,
                module,
                features,
                depth + 1,
                out,
            );
            out.line_at(
                depth + 1,
                format_args!("if (nexaIsRoot || !nexaShowNavigationBar) {{"),
            );
            out.line_at(depth + 2, format_args!("nexaScreenContent()"));
            out.line_at(depth + 1, format_args!("}} else {{"));
            render_navigation_screen_chrome(
                screen,
                &navigation_toolbars,
                module,
                features,
                depth + 2,
                out,
            );
            out.line_at(depth + 2, format_args!(") {{ nexaScreenContent() }}"));
            out.line_at(depth + 2, format_args!("}}"));
        } else {
            render_screen_content_function(
                "nexaScreenRootContent",
                root_body,
                module,
                features,
                depth + 1,
                out,
            );
            render_screen_content_function(
                "nexaScreenDestinationContent",
                &destination_body,
                module,
                features,
                depth + 1,
                out,
            );
            out.line_at(
                depth + 1,
                format_args!("if (nexaIsRoot || !nexaShowNavigationBar) {{"),
            );
            out.line_at(depth + 2, format_args!("nexaScreenRootContent()"));
            out.line_at(depth + 1, format_args!("}} else {{"));
            render_navigation_screen_chrome(
                screen,
                &navigation_toolbars,
                module,
                features,
                depth + 2,
                out,
            );
            out.line_at(
                depth + 2,
                format_args!(") {{ nexaScreenDestinationContent() }}"),
            );
            out.line_at(depth + 2, format_args!("}}"));
        }
        crate::generator::components::lifecycle::render_on_appear(
            screen.on_appear.as_deref(),
            depth + 1,
            out,
        );
        crate::generator::components::lifecycle::render_on_disappear(
            screen.on_disappear.as_deref(),
            depth + 1,
            out,
        );
        crate::generator::components::lifecycle::render_task_cancellation_on_dispose(
            &screen.states,
            depth + 1,
            out,
        );
        crate::generator::components::status_bar::render(
            screen.status_bar.or(module.status_bar),
            features.uses_status_bar,
            depth + 1,
            out,
        );
        out.line_at(depth, format_args!("}}"));
    }
}

fn render_screen_content_function(
    name: &str,
    body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("@Composable"));
    out.line_at(depth, format_args!("fun {name}() {{"));
    if features.uses_shared_elements {
        out.line_at(
            depth + 1,
            format_args!("CompositionLocalProvider(LocalNexaAnimatedVisibilityScope provides nexaAnimatedVisibilityScope) {{"),
        );
        render_screen_body(body, module, features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}}"));
    } else {
        render_screen_body(body, module, features, depth + 1, out);
    }
    out.line_at(depth, format_args!("}}"));
}

fn render_navigation_screen_chrome(
    screen: &nexa_ir::Screen,
    toolbars: &[(nexa_ir::ToolbarPlacement, &[Node])],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("NexaNavigationScreenPrimitive("));
    out.line_at(
        depth + 1,
        format_args!("title = {},", kotlin_string(&screen.name)),
    );
    out.line_at(depth + 1, format_args!("inline = nexaTitleInline,"));
    out.line_at(depth + 1, format_args!("leadingTitle = nexaLeadingTitle,"));
    out.line_at(
        depth + 1,
        format_args!(
            "hasTrailingActions = {},",
            toolbars.iter().any(|(placement, children)| {
                *placement == nexa_ir::ToolbarPlacement::Trailing && !children.is_empty()
            })
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("onBack = {{ if (navController.previousBackStackEntry != null) navController.popBackStack() }},"),
    );
    for (parameter, placement) in [
        ("leadingActions", nexa_ir::ToolbarPlacement::Leading),
        ("actions", nexa_ir::ToolbarPlacement::Trailing),
    ] {
        if toolbars
            .iter()
            .any(|(toolbar_placement, _)| *toolbar_placement == placement)
        {
            out.line_at(depth + 1, format_args!("{parameter} = {{"));
            bottom_bar::render_tab_toolbar_items(
                toolbars,
                placement,
                module,
                features,
                depth + 2,
                out,
            );
            out.line_at(depth + 1, format_args!("}},"));
        }
    }
}

struct ScreenCallContext {
    is_root: bool,
    show_navigation_bar: bool,
    title_mode: nexa_ir::NavigationTitleMode,
    title_alignment: nexa_ir::NavigationTitleAlignment,
}

fn render_screen_call(
    screen: &nexa_ir::Screen,
    arguments: &[String],
    context: ScreenCallContext,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str(&format!("{}(\n", screen_composable_name(screen.id)));
    out.line_at(depth + 1, format_args!("navController = navController,"));
    out.line_at(
        depth + 1,
        format_args!("nexaBackStackEntry = backStackEntry,"),
    );
    for (parameter, value) in screen.parameters.iter().zip(arguments) {
        out.line_at(
            depth + 1,
            format_args!("{} = {},", state_name(&parameter.name), value),
        );
    }
    out.line_at(depth + 1, format_args!("nexaIsRoot = {},", context.is_root));
    out.line_at(
        depth + 1,
        format_args!("nexaShowNavigationBar = {},", context.show_navigation_bar),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "nexaTitleInline = {},",
            matches!(context.title_mode, nexa_ir::NavigationTitleMode::Inline)
        ),
    );
    let leading_title = match context.title_alignment {
        nexa_ir::NavigationTitleAlignment::Automatic
        | nexa_ir::NavigationTitleAlignment::Center => false,
        nexa_ir::NavigationTitleAlignment::Leading => true,
    };
    out.line_at(
        depth + 1,
        format_args!("nexaLeadingTitle = {leading_title},"),
    );
    if features.uses_shared_elements {
        out.line_at(
            depth + 1,
            format_args!("nexaAnimatedVisibilityScope = this,"),
        );
    }
    out.line_at(depth, format_args!(")"));
}

fn screen_composable_name(screen: ScreenId) -> String {
    format!("nexaScreen{}", screen.0)
}

fn screen_contains_navigation_stack(screen: &nexa_ir::Screen) -> bool {
    let mut contains_navigation_stack = false;
    nexa_ir::walk::walk_ir(
        &screen.body,
        &mut |node| {
            contains_navigation_stack |= matches!(node, Node::NavigationStack { .. });
        },
        &mut |_| {},
    );
    contains_navigation_stack
}

fn render_screen_body(
    content: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    // Match SwiftUI's screen-level `.frame(maxWidth: .infinity, maxHeight: .infinity)`.
    // Propagating the minimum constraints ensures a root Column/Stack can lay out
    // weighted spacers and backgrounds against the actual screen viewport.
    out.line_at(
        depth,
        format_args!("Box(modifier = Modifier.fillMaxSize(), propagateMinConstraints = true) {{"),
    );
    if features.uses_shared_elements {
        out.line_at(
            depth + 1,
            format_args!(
                "CompositionLocalProvider(LocalNexaAnimatedVisibilityScope provides nexaAnimatedVisibilityScope) {{"
            ),
        );
        render_children(content, module, features, depth + 2, out);
        out.push('\n');
        indent(out, depth + 1);
        out.push('}');
    } else {
        render_children(content, module, features, depth + 1, out);
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
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
                "segments.isNotEmpty() && segments.size <= {} && segments[0].equals({}, ignoreCase = true) -> {{",
                screen.parameters.len() + 1,
                kotlin_string(&route_slug(&screen.name))
            ),
        );
        let mut arguments = Vec::new();
        for (index, parameter) in screen.parameters.iter().enumerate() {
            let name = format!("nexaArgument{index}");
            let raw = format!("nexaRouteArgument{index}");
            out.line_at(
                depth + 3,
                format_args!(
                    "val {raw} = segments.getOrNull({}) ?: uri.getQueryParameter({}) ?: return@LaunchedEffect",
                    index + 1,
                    kotlin_string(&parameter.name)
                ),
            );
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

fn route_state_name(screen: ScreenId) -> String {
    format!("NexaRouteState{}", screen.0)
}

fn route_state_variable_name(screen: ScreenId) -> String {
    format!("nexaRouteState{}", screen.0)
}

fn route_state_field_name(state: &State) -> String {
    state_name(&state.name)
}

fn route_state_initial_name(state: &State) -> String {
    format!("nexaInitial{}", state_name(&state.name))
}

fn render_route_state_type(screen: &nexa_ir::Screen, depth: usize, out: &mut SourceWriter) {
    let class_name = route_state_name(screen.id);
    indent(out, depth);
    out.push_str(&format!("class {class_name}(\n"));
    for state in &screen.states {
        out.line_at(
            depth + 1,
            format_args!(
                "{}: {},",
                route_state_initial_name(state),
                kotlin_type(&state.ty)
            ),
        );
    }
    indent(out, depth);
    out.push_str(") : androidx.lifecycle.ViewModel() {\n");
    for state in &screen.states {
        let field = route_state_field_name(state);
        let initial = route_state_initial_name(state);
        indent(out, depth + 1);
        if crate::generator::state::is_mutable_collection(state) {
            let initializer = match &state.ty {
                Type::Array(element) => format!(
                    "mutableStateListOf<{}>().also {{ it.addAll({initial}) }}",
                    kotlin_type(element)
                ),
                Type::Set(element) => format!(
                    "mutableStateSetOf<{}>().also {{ it.addAll({initial}) }}",
                    kotlin_type(element)
                ),
                Type::Map(key, value) => format!(
                    "mutableStateMapOf<{}, {}>().also {{ it.putAll({initial}) }}",
                    kotlin_type(key),
                    kotlin_type(value)
                ),
                _ => format!("mutableStateOf<{}>({initial})", kotlin_type(&state.ty)),
            };
            out.push_str(&format!("val {field} = {initializer}\n"));
        } else if state.mutable {
            let initializer = match state.ty {
                Type::Numeric(nexa_ir::NumericType::Int32) => {
                    format!("mutableIntStateOf({initial})")
                }
                Type::Numeric(nexa_ir::NumericType::Int64) => {
                    format!("mutableLongStateOf({initial})")
                }
                Type::Numeric(nexa_ir::NumericType::Float32) => {
                    format!("mutableFloatStateOf({initial})")
                }
                Type::Numeric(nexa_ir::NumericType::Float64) => {
                    format!("mutableDoubleStateOf({initial})")
                }
                _ => format!("mutableStateOf<{}>({initial})", kotlin_type(&state.ty)),
            };
            out.push_str(&format!("val {field} = {initializer}\n"));
        } else {
            out.push_str(&format!(
                "val {field}: {} = {initial}\n",
                kotlin_type(&state.ty)
            ));
        }
    }
    indent(out, depth);
    out.push_str("}\n");
}

fn render_route_state_access(
    screen: &nexa_ir::Screen,
    class_name: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let store_key = kotlin_string(&format!("nexa-route-state-{}", screen.id.0));
    let state_variable = route_state_variable_name(screen.id);
    indent(out, depth);
    out.push_str(&format!(
        "val {state_variable} = nexaBackStackEntry.viewModelStore.get({store_key}) as? {class_name} ?: run {{\n"
    ));
    for state in &screen.states {
        out.line_at(
            depth + 1,
            format_args!(
                "val {}: {} = {}",
                route_state_field_name(state),
                kotlin_type(&state.ty),
                crate::generator::expressions::expression(&state.initial)
            ),
        );
    }
    let constructor_arguments = screen
        .states
        .iter()
        .map(|state| {
            format!(
                "{} = {}",
                route_state_initial_name(state),
                route_state_field_name(state)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    out.line_at(
        depth + 1,
        format_args!(
            "{class_name}({constructor_arguments}).also {{ nexaBackStackEntry.viewModelStore.put({store_key}, it) }}"
        ),
    );
    indent(out, depth);
    out.push_str("}\n");
}

fn render_screen_state(state: &State, route_state: &str, depth: usize, out: &mut SourceWriter) {
    let name = state_name(&state.name);
    indent(out, depth);
    if state.mutable && !crate::generator::state::is_mutable_collection(state) {
        out.push_str(&format!("var {name} by {route_state}.{name}\n"));
    } else if crate::generator::state::is_mutable_collection(state) {
        out.push_str(&format!("val {name} = {route_state}.{name}\n"));
    } else {
        out.push_str(&format!(
            "val {name}: {} = {route_state}.{name}\n",
            kotlin_type(&state.ty)
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

pub(crate) fn action_route_value(destination: ScreenId, arguments: &[Expr]) -> String {
    let mut value = kotlin_string(&navigation_route_name(destination));
    for argument in arguments {
        value.push_str(" + \"/\" + Uri.encode(");
        value.push_str(&crate::generator::engine::expressions::expression(argument));
        value.push_str(".toString())");
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
