use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(_features: &Features, _imports: &mut ImportSet) {}

pub(crate) fn render_refresh_control(
    state: &str,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("NexaRefreshControlPrimitive(\n");
    out.line_at(
        depth + 1,
        format_args!("isRefreshing = {},", state_name(state)),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "scrollContent = {},",
            !nexa_ir::walk::contains_scrollable(children)
        ),
    );
    out.line_at(depth + 1, format_args!("onRefresh = {{"));
    render_actions(actions, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(") {{"));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
