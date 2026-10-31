use nexa_codegen::SourceWriter;
use nexa_ir::{AnimationSpec, Expr, Module, Node, ViewTransition, WhenCase};

use crate::generator::{
    components::{controls, node_renderer::render_children},
    engine::features::Features,
    utils::indent,
};

pub(crate) struct ConditionalRenderContext<'a> {
    pub(crate) module: &'a Module,
    pub(crate) features: &'a Features,
    pub(crate) depth: usize,
}

pub(crate) fn render_if(
    condition: &Expr,
    then_body: &[Node],
    else_body: Option<&[Node]>,
    transition: Option<ViewTransition>,
    transition_animation: Option<AnimationSpec>,
    context: ConditionalRenderContext<'_>,
    out: &mut SourceWriter,
) {
    let ConditionalRenderContext {
        module,
        features,
        depth,
    } = context;
    let condition_expression = crate::generator::engine::expressions::expression(condition);
    if let Some(transition) = transition {
        let animation = transition_animation
            .map(controls::swift_animation)
            .unwrap_or_else(|| ".default".to_owned());
        if let Some(state_name) = transition_state_name(condition) {
            out.line_at(
                depth,
                format_args!(
                    "NexaAnimatedWhen(value: {condition_expression}, animation: {animation}, transition: {}) {{ {state_name} in",
                    swift_transition(transition)
                ),
            );
            render_if_body(
                &state_name,
                then_body,
                else_body,
                module,
                features,
                depth + 1,
                out,
            );
            out.line_at(depth, format_args!("}}"));
            return;
        }

        render_if_body(
            &condition_expression,
            then_body,
            else_body,
            module,
            features,
            depth,
            out,
        );
        out.push_str(&format!(
            ".id({condition_expression}).transition({}).animation({animation}, value: {condition_expression})",
            swift_transition(transition)
        ));
    } else {
        render_if_body(
            &condition_expression,
            then_body,
            else_body,
            module,
            features,
            depth,
            out,
        );
        out.push_str(".transition(.identity)");
    }
}

pub(crate) fn render_when(
    value: &Expr,
    cases: &[WhenCase],
    else_body: Option<&[Node]>,
    transition: Option<ViewTransition>,
    transition_animation: Option<AnimationSpec>,
    context: ConditionalRenderContext<'_>,
    out: &mut SourceWriter,
) {
    let ConditionalRenderContext {
        module,
        features,
        depth,
    } = context;
    let value_expression = crate::generator::engine::expressions::expression(value);
    if let Some(transition) = transition {
        let animation = transition_animation
            .map(controls::swift_animation)
            .unwrap_or_else(|| ".default".to_owned());
        if let Some(state_name) = transition_state_name(value) {
            out.line_at(
                depth,
                format_args!(
                    "NexaAnimatedWhen(value: {value_expression}, animation: {animation}, transition: {}) {{ {state_name} in",
                    swift_transition(transition)
                ),
            );
            render_when_body(
                &state_name,
                cases,
                else_body,
                module,
                features,
                depth + 1,
                out,
            );
            out.line_at(depth, format_args!("}}"));
            return;
        }

        render_when_body(
            &value_expression,
            cases,
            else_body,
            module,
            features,
            depth,
            out,
        );
        out.push_str(&format!(
            ".id({value_expression}).transition({}).animation({animation}, value: {value_expression})",
            swift_transition(transition)
        ));
    } else {
        render_when_body(
            &value_expression,
            cases,
            else_body,
            module,
            features,
            depth,
            out,
        );
        out.push_str(".transition(.identity)");
    }
}

fn render_if_body(
    condition: &str,
    then_body: &[Node],
    else_body: Option<&[Node]>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let content_depth = depth + 1;
    out.line_at(depth, format_args!("Group {{"));
    out.line_at(content_depth, format_args!("if {condition} {{"));
    render_children(then_body, module, features, content_depth + 1, out);
    if let Some(else_body) = else_body {
        out.push('\n');
        indent(out, content_depth);
        out.push_str("} else {\n");
        render_children(else_body, module, features, content_depth + 1, out);
    }
    out.push('\n');
    indent(out, content_depth);
    out.push('}');
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_when_body(
    value: &str,
    cases: &[WhenCase],
    else_body: Option<&[Node]>,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let content_depth = depth + 1;
    out.line_at(depth, format_args!("Group {{"));
    out.line_at(content_depth, format_args!("switch {value} {{"));
    for case in cases {
        out.line_at(
            content_depth + 1,
            format_args!(
                "case {}:",
                crate::generator::engine::expressions::expression(&case.value)
            ),
        );
        render_children(&case.body, module, features, content_depth + 2, out);
        out.push('\n');
    }
    if let Some(else_body) = else_body {
        indent(out, content_depth + 1);
        out.push_str("default:\n");
        render_children(else_body, module, features, content_depth + 2, out);
        out.push('\n');
    }
    indent(out, content_depth);
    out.push('}');
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn transition_state_name(expression: &Expr) -> Option<String> {
    match expression {
        Expr::State(name, _) | Expr::AnimatedState(name, _) if !name.contains("::") => {
            Some(nexa_codegen::names::state_name(name))
        }
        Expr::Located { expression, .. } => transition_state_name(expression),
        _ => None,
    }
}

fn swift_transition(transition: ViewTransition) -> &'static str {
    match transition {
        ViewTransition::Fade => ".opacity",
        ViewTransition::FadeSlideFromTop => {
            "AnyTransition.asymmetric(insertion: .opacity.combined(with: .move(edge: .top)), removal: .opacity.combined(with: .move(edge: .top)))"
        }
        ViewTransition::SlideFromBottom => ".move(edge: .bottom)",
        ViewTransition::SlideFromLeft => ".move(edge: .leading)",
        ViewTransition::SlideFromRight => ".move(edge: .trailing)",
        ViewTransition::Scale => ".scale(scale: 0.0, anchor: .center)",
    }
}
