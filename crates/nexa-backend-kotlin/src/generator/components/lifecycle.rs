use nexa_ir::{Action, Module};

use crate::generator::{
    controls,
    engine::imports::{ImportContext, ImportSet},
    utils,
};

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    imports.add(
        context.has_on_appear,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        context.has_on_disappear || context.has_lifecycle_events,
        "androidx.compose.runtime.DisposableEffect",
    );
    imports.add(
        context.has_lifecycle_events,
        "androidx.lifecycle.compose.LocalLifecycleOwner",
    );
    imports.add(context.has_lifecycle_events, "androidx.lifecycle.Lifecycle");
    imports.add(
        context.has_lifecycle_events,
        "androidx.lifecycle.LifecycleEventObserver",
    );
}

pub(crate) fn render_on_appear(actions: Option<&[Action]>, depth: usize, out: &mut String) {
    let Some(actions) = actions else {
        return;
    };
    utils::indent(out, depth);
    out.push_str("LaunchedEffect(Unit) {");
    if actions.is_empty() {
        out.push_str("}\n");
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 1, out);
    utils::indent(out, depth);
    out.push_str("}\n");
}

pub(crate) fn render_on_disappear(actions: Option<&[Action]>, depth: usize, out: &mut String) {
    let Some(actions) = actions else {
        return;
    };
    utils::indent(out, depth);
    out.push_str("DisposableEffect(Unit) {");
    if actions.is_empty() {
        out.push('\n');
        utils::indent(out, depth + 1);
        out.push_str("onDispose {}\n");
        utils::indent(out, depth);
        out.push_str("}\n");
        return;
    }
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str("onDispose {\n");
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push_str("}\n");
    utils::indent(out, depth);
    out.push_str("}\n");
}

pub(crate) fn render_app(module: &Module, depth: usize, out: &mut String) {
    if module.on_active.is_none() && module.on_inactive.is_none() && module.on_background.is_none()
    {
        return;
    }
    let indent = "    ".repeat(depth);
    let nested = "    ".repeat(depth + 1);
    let deep = "    ".repeat(depth + 2);
    out.push_str(&format!(
        "{indent}val nexaLifecycleOwner = LocalLifecycleOwner.current\n"
    ));
    out.push_str(&format!(
        "{indent}DisposableEffect(nexaLifecycleOwner) {{\n"
    ));
    out.push_str(&format!(
        "{nested}val nexaLifecycleObserver = LifecycleEventObserver {{ _, event ->\n"
    ));
    out.push_str(&format!("{deep}when (event) {{\n"));
    render_lifecycle_case("ON_RESUME", module.on_active.as_deref(), depth + 3, out);
    render_lifecycle_case("ON_PAUSE", module.on_inactive.as_deref(), depth + 3, out);
    render_lifecycle_case("ON_STOP", module.on_background.as_deref(), depth + 3, out);
    out.push_str(&format!("{}else -> Unit\n", "    ".repeat(depth + 3)));
    out.push_str(&format!("{deep}}}\n"));
    out.push_str(&format!("{nested}}}\n"));
    out.push_str(&format!(
        "{nested}nexaLifecycleOwner.lifecycle.addObserver(nexaLifecycleObserver)\n"
    ));
    out.push_str(&format!(
        "{nested}onDispose {{ nexaLifecycleOwner.lifecycle.removeObserver(nexaLifecycleObserver) }}\n"
    ));
    out.push_str(&format!("{indent}}}\n"));
}

fn render_lifecycle_case(event: &str, actions: Option<&[Action]>, depth: usize, out: &mut String) {
    let indent = "    ".repeat(depth);
    out.push_str(&format!("{indent}Lifecycle.Event.{event} -> {{\n"));
    if let Some(actions) = actions {
        if actions.is_empty() {
            out.push_str(&format!("{}Unit\n", "    ".repeat(depth + 1)));
        } else {
            controls::render_actions(actions, depth + 1, out);
        }
    } else {
        out.push_str(&format!("{}Unit\n", "    ".repeat(depth + 1)));
    }
    out.push_str(&format!("{indent}}}\n"));
}
