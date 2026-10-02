use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node, ViewTransition, WhenCase};

use crate::generator::{
    engine::{expressions, features::Features, imports::ImportSet},
    utils::indent,
};

use super::node_renderer::render_children;

pub(crate) struct RenderScope<'a> {
    pub(crate) module: &'a Module,
    pub(crate) features: &'a Features,
    pub(crate) depth: usize,
    pub(crate) out: &'a mut SourceWriter,
}

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let enabled = features.uses_conditional_transition;
    for name in [
        "AnimatedContent",
        "fadeIn",
        "fadeOut",
        "scaleIn",
        "scaleOut",
        "slideInVertically",
        "slideOutVertically",
        "slideInHorizontally",
        "slideOutHorizontally",
        "togetherWith",
    ] {
        imports.add(
            enabled,
            match name {
                "AnimatedContent" => "androidx.compose.animation.AnimatedContent",
                "fadeIn" => "androidx.compose.animation.fadeIn",
                "fadeOut" => "androidx.compose.animation.fadeOut",
                "scaleIn" => "androidx.compose.animation.scaleIn",
                "scaleOut" => "androidx.compose.animation.scaleOut",
                "slideInVertically" => "androidx.compose.animation.slideInVertically",
                "slideOutVertically" => "androidx.compose.animation.slideOutVertically",
                "slideInHorizontally" => "androidx.compose.animation.slideInHorizontally",
                "slideOutHorizontally" => "androidx.compose.animation.slideOutHorizontally",
                _ => "androidx.compose.animation.togetherWith",
            },
        );
    }
}

pub(crate) fn render_if(
    condition: &Expr,
    then_body: &[Node],
    else_body: Option<&[Node]>,
    transition: Option<ViewTransition>,
    scope: RenderScope<'_>,
) {
    let RenderScope {
        module,
        features,
        depth,
        out,
    } = scope;
    let Some(transition) = transition else {
        out.line_at(
            depth,
            format_args!("if ({}) {{", expressions::expression(condition)),
        );
        render_children(then_body, module, features, depth + 1, out);
        if let Some(else_body) = else_body {
            out.push('\n');
            indent(out, depth);
            out.push_str("} else {\n");
            render_children(else_body, module, features, depth + 1, out);
        }
        out.push('\n');
        indent(out, depth);
        out.push('}');
        return;
    };

    out.line_at(
        depth,
        format_args!(
            "AnimatedContent(targetState = {}, transitionSpec = {{ {} }}, label = \"if-transition\") {{ nexTarget ->",
            expressions::expression(condition),
            content_transform(transition)
        ),
    );
    out.line_at(depth + 1, format_args!("if (nexTarget) {{"));
    render_children(then_body, module, features, depth + 2, out);
    if let Some(else_body) = else_body {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("} else {\n");
        render_children(else_body, module, features, depth + 2, out);
    }
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_when(
    value: &Expr,
    cases: &[WhenCase],
    else_body: &[Node],
    transition: Option<ViewTransition>,
    scope: RenderScope<'_>,
) {
    let RenderScope {
        module,
        features,
        depth,
        out,
    } = scope;
    let Some(transition) = transition else {
        render_when_body(
            &expressions::expression(value),
            cases,
            else_body,
            module,
            features,
            depth,
            out,
        );
        return;
    };

    out.line_at(
        depth,
        format_args!(
            "AnimatedContent(targetState = {}, transitionSpec = {{ {} }}, label = \"when-transition\") {{ nexTarget ->",
            expressions::expression(value),
            content_transform(transition)
        ),
    );
    render_when_body(
        "nexTarget",
        cases,
        else_body,
        module,
        features,
        depth + 1,
        out,
    );
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_when_body(
    value: &str,
    cases: &[WhenCase],
    else_body: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("when ({value}) {{"));
    for case in cases {
        out.line_at(
            depth + 1,
            format_args!("{} -> {{", expressions::expression(&case.value)),
        );
        render_children(&case.body, module, features, depth + 2, out);
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    indent(out, depth + 1);
    out.push_str("else -> {\n");
    render_children(else_body, module, features, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}

fn content_transform(transition: ViewTransition) -> &'static str {
    match transition {
        ViewTransition::Fade => "fadeIn() togetherWith fadeOut()",
        ViewTransition::SlideFromBottom => {
            "slideInVertically { height -> height } togetherWith slideOutVertically { height -> height }"
        }
        ViewTransition::SlideFromLeft => {
            "slideInHorizontally { width -> -width } togetherWith slideOutHorizontally { width -> width }"
        }
        ViewTransition::SlideFromRight => {
            "slideInHorizontally { width -> width } togetherWith slideOutHorizontally { width -> -width }"
        }
        ViewTransition::Scale => "scaleIn() togetherWith scaleOut()",
    }
}
