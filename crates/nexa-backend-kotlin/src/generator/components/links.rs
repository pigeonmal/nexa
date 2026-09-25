use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node};

use crate::generator::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_link, "android.content.Intent");
    imports.add(
        features.uses_link || features.uses_navigation_uri,
        "android.net.Uri",
    );
    imports.add(features.uses_link, "androidx.compose.foundation.clickable");
    imports.add(
        features.uses_link,
        "androidx.compose.ui.platform.LocalContext",
    );
}

pub(crate) fn render_link(
    url: &Expr,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("Box(\n");
    indent(out, depth + 1);
    out.push_str("modifier = Modifier.clickable {\n");
    indent(out, depth + 2);
    out.push_str("val nexaLinkIntent = Intent(Intent.ACTION_VIEW, Uri.parse(");
    out.push_str(&expression(url));
    out.push_str("))\n");
    indent(out, depth + 2);
    out.push_str("if (nexaLinkIntent.resolveActivity(nexaLinkContext.packageManager) != null) {\n");
    indent(out, depth + 3);
    out.push_str("nexaLinkContext.startActivity(nexaLinkIntent)\n");
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("},\n");
    indent(out, depth);
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
