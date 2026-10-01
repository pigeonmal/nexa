use nexa_codegen::SourceWriter;
use nexa_ir::{Action, Module};

use crate::generator::{
    controls,
    engine::imports::{ImportContext, ImportSet},
    utils,
};

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    imports.add(
        context.features.uses_tasks,
        "androidx.compose.runtime.rememberCoroutineScope",
    );
    imports.add(
        context.features.uses_tasks,
        "kotlinx.coroutines.CoroutineExceptionHandler",
    );
    imports.add(
        context.features.uses_tasks,
        "kotlinx.coroutines.CoroutineStart",
    );
    imports.add(
        context.features.uses_tasks,
        "kotlinx.coroutines.Dispatchers",
    );
    imports.add(context.features.uses_tasks, "kotlinx.coroutines.Job");
    imports.add(context.features.uses_tasks, "kotlinx.coroutines.launch");
    imports.add(
        context.has_on_appear,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        context.has_on_disappear || context.has_lifecycle_events || context.features.uses_tasks,
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

pub(crate) fn render_task_cancellation_on_dispose(
    states: &[nexa_ir::State],
    depth: usize,
    out: &mut SourceWriter,
) {
    let handles = states
        .iter()
        .filter(|state| {
            state.mutable
                && matches!(&state.ty, nexa_ir::Type::Optional(inner) if matches!(inner.as_ref(), nexa_ir::Type::TaskHandle))
        })
        .map(|state| nexa_codegen::names::state_name(&state.name))
        .collect::<Vec<_>>();
    if handles.is_empty() {
        return;
    }
    utils::indent(out, depth);
    out.push_str("DisposableEffect(Unit) {\n");
    utils::indent(out, depth + 1);
    out.push_str("onDispose {\n");
    for handle in handles {
        out.line_at(depth + 2, format_args!("{handle}?.cancel()"));
    }
    utils::indent(out, depth + 1);
    out.push_str("}\n");
    utils::indent(out, depth);
    out.push_str("}\n");
}

pub(crate) fn render_on_appear(actions: Option<&[Action]>, depth: usize, out: &mut SourceWriter) {
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

pub(crate) fn render_on_disappear(
    actions: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
) {
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

pub(crate) fn render_app(module: &Module, depth: usize, out: &mut SourceWriter) {
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

fn render_lifecycle_case(
    event: &str,
    actions: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
) {
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

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, State, Type};

    use super::render_task_cancellation_on_dispose;

    #[test]
    fn task_jobs_cancel_when_their_composition_scope_is_disposed() {
        let states = [State {
            name: "refreshTask".to_owned(),
            ty: Type::Optional(Box::new(Type::TaskHandle)),
            initial: Expr::Null(Type::Optional(Box::new(Type::TaskHandle))),
            mutable: true,
        }];
        let mut output = SourceWriter::new();

        render_task_cancellation_on_dispose(&states, 1, &mut output);

        assert!(output.contains(
            "DisposableEffect(Unit) {\n        onDispose {\n            nexa_refreshTask?.cancel()"
        ));
        assert!(output.ends_with("    }\n"));
    }
}
