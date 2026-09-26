//! SwiftUI state declaration emitters.

use nexa_codegen::SourceWriter;
use nexa_ir::State;

use super::{expressions::expression, types::swift_type, utils};

/// Renders the `let` bindings for states SwiftUI must not own.
///
/// A state only reaches the body as a plain `let` when it is never reassigned
/// and is not a native class instance; anything else lives in `@State`,
/// `@StateObject`, or a `@Binding` on the screen that owns it.
pub(crate) fn render_immutable_state(states: &[State], depth: usize, out: &mut SourceWriter) {
    let immutable = states
        .iter()
        .filter(|state| !state.mutable && !state.is_native_class_instance_binding())
        .collect::<Vec<_>>();
    for state in immutable {
        out.line_at(
            depth,
            format_args!(
                "let {}: {} = {}",
                nexa_codegen::names::state_name(&state.name),
                swift_type(&state.ty),
                expression(&state.initial)
            ),
        );
    }
    if states
        .iter()
        .any(|state| !state.mutable && !state.is_native_class_instance_binding())
    {
        out.push('\n');
    }
}

/// Renders an identity-stable SwiftUI storage property for a plugin class
/// instance.
///
/// The constructor must appear only in the `@StateObject` initializer: SwiftUI
/// runs that initializer exactly once per view identity, so a native object
/// holding a decoder, a player, or a sensor session is not torn down and
/// rebuilt on every recomputation. A mutable binding writes through the same
/// box, keeping object identity intact across updates.
pub(crate) fn render_native_object_state(state: &State, depth: usize, out: &mut SourceWriter) {
    if !state.is_native_class_constructor_binding() {
        return;
    }
    let name = nexa_codegen::names::state_name(&state.name);
    let storage = format!("__nexaNativeObjectStorage_{name}");
    out.line_at(
        depth,
        format_args!(
            "@StateObject private var {storage} = NexaNativeObjectStorage {{ {} }}",
            expression(&state.initial)
        ),
    );
    out.line_at(
        depth,
        format_args!("private var {name}: {} {{", swift_type(&state.ty)),
    );
    out.line_at(depth + 1, format_args!("get {{ {storage}.value }}"));
    if state.mutable {
        out.line_at(
            depth + 1,
            format_args!("nonmutating set {{ {storage}.value = newValue }}"),
        );
    }
    utils::indent(out, depth);
    out.push_str("}\n");
}
