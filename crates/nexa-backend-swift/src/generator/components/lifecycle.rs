use nexa_codegen::SourceWriter;
use nexa_ir::{Action, Module};

use crate::generator::{controls, utils};

/// Renders the `.onAppear` / `.task` modifier.
///
/// An `onAppear` declared `async` cannot run synchronously inside SwiftUI's
/// appearance callback, so it becomes a `.task` instead -- SwiftUI cancels it
/// if the view disappears mid-flight.
pub(crate) fn render_on_appear(
    actions: Option<&[Action]>,
    asynchronous: bool,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(if asynchronous { ".task {" } else { ".onAppear {" });
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

pub(crate) fn render_on_disappear(
    actions: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(actions) = actions else {
        return;
    };
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(".onDisappear {");
    if actions.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    controls::render_actions(actions, depth + 2, out);
    utils::indent(out, depth + 1);
    out.push('}');
}

/// Renders the `onChange(of: scenePhase)` modifier that dispatches the app's
/// active / inactive / background hooks to a single SwiftUI observation point.
pub(crate) fn render_scene_phase(module: &Module, depth: usize, out: &mut SourceWriter) {
    if module.on_active.is_none() && module.on_inactive.is_none() && module.on_background.is_none()
    {
        return;
    }
    out.push('\n');
    utils::indent(out, depth + 1);
    out.push_str(".onChange(of: nexaScenePhase) { phase in\n");
    utils::indent(out, depth + 2);
    out.push_str("switch phase {\n");
    for (phase, actions) in [
        ("active", module.on_active.as_deref()),
        ("inactive", module.on_inactive.as_deref()),
        ("background", module.on_background.as_deref()),
    ] {
        out.line_at(depth + 3, format_args!("case .{phase}:"));
        // An undeclared phase breaks so the switch stays exhaustive, and an
        // empty hook list is a no-op rather than an empty block.
        match actions {
            Some(actions) if !actions.is_empty() => {
                controls::render_actions(actions, depth + 4, out);
            }
            _ => {
                utils::indent(out, depth + 4);
                out.push_str("break\n");
            }
        }
    }
    utils::indent(out, depth + 3);
    out.push_str("@unknown default:\n");
    utils::indent(out, depth + 4);
    out.push_str("break\n");
    utils::indent(out, depth + 2);
    out.push_str("}\n");
    utils::indent(out, depth + 1);
    out.push('}');
}
