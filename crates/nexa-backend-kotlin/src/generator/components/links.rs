use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node};

use crate::generator::{components::render_children, expressions::expression, features::Features};

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
    out.line_at(depth, format_args!("NexaLinkPrimitive(onClick = {{"));
    out.line_at(
        depth + 1,
        format_args!(
            "val nexaLinkIntent = Intent(Intent.ACTION_VIEW, Uri.parse({}))",
            expression(url)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "if (nexaLinkIntent.resolveActivity(nexaLinkContext.packageManager) != null) {{"
        ),
    );
    out.line_at(
        depth + 2,
        format_args!("nexaLinkContext.startActivity(nexaLinkIntent)"),
    );
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}) {{"));
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
}
