use nexa_codegen::SourceWriter;
use nexa_ir::{AnimationSpec, Expr, Module, Node, ViewTransition, WhenCase};

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
    transition_animation: Option<AnimationSpec>,
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
            "AnimatedContent(targetState = {}, transitionSpec = {{ {} }}, label = \"if-transition\") {{ {} ->",
            expressions::expression(condition),
            content_transform(transition, transition_animation),
            transition_state_name(condition).unwrap_or_else(|| "nexTarget".to_owned()),
        ),
    );
    let target = transition_state_name(condition).unwrap_or_else(|| "nexTarget".to_owned());
    out.line_at(depth + 1, format_args!("if ({target}) {{"));
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
    else_body: Option<&[Node]>,
    transition: Option<ViewTransition>,
    transition_animation: Option<AnimationSpec>,
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
            "AnimatedContent(targetState = {}, transitionSpec = {{ {} }}, label = \"when-transition\") {{ {} ->",
            expressions::expression(value),
            content_transform(transition, transition_animation),
            transition_state_name(value).unwrap_or_else(|| "nexTarget".to_owned()),
        ),
    );
    render_when_body(
        &transition_state_name(value).unwrap_or_else(|| "nexTarget".to_owned()),
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

fn transition_state_name(expression: &Expr) -> Option<String> {
    match expression {
        Expr::State(name, _) | Expr::AnimatedState(name, _) if !name.contains("::") => {
            Some(nexa_codegen::names::state_name(name))
        }
        Expr::Located { expression, .. } => transition_state_name(expression),
        _ => None,
    }
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
    if let Some(else_body) = else_body {
        indent(out, depth + 1);
        out.push_str("else -> {\n");
        render_children(else_body, module, features, depth + 2, out);
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    indent(out, depth);
    out.push('}');
}

fn content_transform(transition: ViewTransition, animation: Option<AnimationSpec>) -> String {
    let animation_spec = animation.map(super::controls::kotlin_animation);
    let animation = animation_spec
        .as_ref()
        .map_or_else(String::new, |animation| {
            format!("(animationSpec = {animation})")
        });
    match transition {
        ViewTransition::Fade => {
            format!("fadeIn{animation} togetherWith fadeOut{animation}")
        }
        ViewTransition::FadeSlideFromTop => format!(
            "(fadeIn{animation} + slideInVertically{animation} {{ height -> -height }}) togetherWith (fadeOut{animation} + slideOutVertically{animation} {{ height -> -height }})"
        ),
        ViewTransition::SlideFromBottom => {
            format!(
                "slideInVertically{animation} {{ height -> height }} togetherWith slideOutVertically{animation} {{ height -> height }}"
            )
        }
        ViewTransition::SlideFromLeft => {
            format!(
                "slideInHorizontally{animation} {{ width -> -width }} togetherWith slideOutHorizontally{animation} {{ width -> width }}"
            )
        }
        ViewTransition::SlideFromRight => {
            format!(
                "slideInHorizontally{animation} {{ width -> width }} togetherWith slideOutHorizontally{animation} {{ width -> -width }}"
            )
        }
        ViewTransition::Scale => {
            let animation = animation_spec
                .as_ref()
                .map_or_else(String::new, |animation| {
                    format!(", animationSpec = {animation}")
                });
            format!(
                "scaleIn(initialScale = 0f{animation}) togetherWith scaleOut(targetScale = 0f{animation})"
            )
        }
    }
}

#[cfg(test)]
mod transition_tests {
    use super::content_transform;
    use nexa_ir::{AnimationEasing, AnimationSpec, ViewTransition};

    #[test]
    fn scale_transition_uses_explicit_zero_endpoints_with_and_without_animation() {
        assert_eq!(
            content_transform(ViewTransition::Scale, None),
            "scaleIn(initialScale = 0f) togetherWith scaleOut(targetScale = 0f)"
        );
        assert_eq!(
            content_transform(
                ViewTransition::Scale,
                Some(AnimationSpec::Tween {
                    duration: 0.32,
                    delay: 0.0,
                    easing: AnimationEasing::EaseOut,
                }),
            ),
            "scaleIn(initialScale = 0f, animationSpec = androidx.compose.animation.core.tween(durationMillis = 320, easing = androidx.compose.animation.core.LinearOutSlowInEasing)) togetherWith scaleOut(targetScale = 0f, animationSpec = androidx.compose.animation.core.tween(durationMillis = 320, easing = androidx.compose.animation.core.LinearOutSlowInEasing))"
        );
    }
}
