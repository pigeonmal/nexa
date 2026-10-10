use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node};

use crate::generator::{components::render_children, expressions::expression};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_link, "Foundation");
}

pub(crate) fn render_link(
    url: &Expr,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaLinkPrimitive(destination: URL(string: {})) {{",
            expression(url)
        ),
    );
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
}
