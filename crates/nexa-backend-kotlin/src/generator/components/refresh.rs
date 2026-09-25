use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::walk::contains_scrollable;
use nexa_ir::{Action, Module, Node};

use crate::generator::{
    components::render_children, controls::render_actions, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_refresh_control,
        "androidx.compose.material3.pulltorefresh.PullToRefreshBox",
    );
}

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
    out.push_str("PullToRefreshBox(\n");
    out.line_at(
        depth + 1,
        format_args!("isRefreshing = {},", state_name(state)),
    );
    indent(out, depth + 1);
    out.push_str("onRefresh = {\n");
    render_actions(actions, depth + 2, out);
    indent(out, depth + 1);
    out.push_str("},\n");
    indent(out, depth);
    out.push_str(") {\n");
    if !contains_scrollable(children) {
        indent(out, depth + 1);
        out.push_str("Column(modifier = Modifier.verticalScroll(rememberScrollState())) {\n");
        render_children(children, module, features, depth + 2, out);
        out.push('\n');
        indent(out, depth + 1);
        out.push('}');
    } else {
        render_children(children, module, features, depth + 1, out);
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
