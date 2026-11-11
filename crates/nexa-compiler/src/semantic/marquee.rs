//! Typed container motion and display-only subtree validation.
use super::{context::ExprContext, expressions::lower_expr};
use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Expr, MarqueeSpec, Module, Node, Type};
use nexa_syntax::ast;
use std::collections::{BTreeMap, HashSet};

pub(super) fn is_marquee(expr: &ast::Expr) -> bool {
    matches!(expr, ast::Expr::Call(name, ..) | ast::Expr::CallNamed { name, .. } | ast::Expr::Name(name, _) if name == "Marquee")
}

pub(super) fn lower(expr: ast::Expr, cx: &ExprContext<'_>) -> Result<MarqueeSpec, CompileError> {
    let span = expr.span();
    let mut args = match expr {
        ast::Expr::Call(name, types, values, _)
            if name == "Marquee" && types.is_empty() && values.is_empty() =>
        {
            BTreeMap::new()
        }
        ast::Expr::CallNamed {
            name,
            type_arguments,
            arguments,
            ..
        } if name == "Marquee" && type_arguments.is_empty() => arguments,
        _ => {
            return Err(CompileError::new(
                span,
                "use Marquee() or Marquee(speed: 32, gap: 24, delay: 1, playing: true)",
            ));
        }
    };
    for key in args.keys() {
        if key == "axis" {
            return Err(CompileError::new(
                span,
                "Marquee has no axis parameter: Row moves horizontally and Column vertically",
            ));
        }
        if !nexa_syntax::catalog::MARQUEE_PARAMETER_DOCS
            .iter()
            .any(|parameter| parameter.name == key)
        {
            return Err(CompileError::new(
                span,
                format!(
                    "unknown Marquee parameter `{key}`; expected speed, gap, delay, edgeFade, or playing"
                ),
            ));
        }
    }
    let mut number = |key: &str, default: f64, positive: bool| -> Result<f64, CompileError> {
        let Some(expr) = args.remove(key) else {
            return Ok(default);
        };
        let value = match &expr {
            ast::Expr::Number(raw, _) => raw.parse::<f64>().ok(),
            _ => None,
        }
        .filter(|v| v.is_finite() && if positive { *v > 0.0 } else { *v >= 0.0 });
        value.ok_or_else(|| {
            CompileError::new(
                expr.span(),
                format!(
                    "Marquee {key} must be a finite {} numeric literal",
                    if positive { "positive" } else { "nonnegative" }
                ),
            )
        })
    };
    let speed = number("speed", 32.0, true)?;
    let gap = number("gap", 24.0, false)?;
    let delay = number("delay", 1.0, false)?;
    let edge_fade = number("edgeFade", 0.0, false)?;
    let playing = args
        .remove("playing")
        .map(|expr| lower_expr(&expr, Some(&Type::Bool), cx))
        .transpose()?
        .unwrap_or(Expr::Bool(true));
    Ok(MarqueeSpec {
        speed,
        gap,
        delay,
        edge_fade,
        playing: Box::new(playing),
    })
}

pub(super) fn validate(module: &Module) -> Result<(), CompileError> {
    let mut error = None;
    let mut inspect = |nodes: &[Node]| {
        nexa_ir::walk::walk_ir(
            nodes,
            &mut |node| {
                if error.is_some() {
                    return;
                }
                if let Node::Layout {
                    style, children, ..
                } = node
                    && style.marquee.is_some()
                {
                    let mut seen = HashSet::new();
                    if !display_only(children, module, &mut seen) {
                        error = Some(CompileError::new(
                            Span::default(),
                            "Marquee supports display-only content; remove interactive/selectable content, scrolling, native surfaces, and nested marquees (including inside custom components)",
                        ));
                    }
                }
            },
            &mut |_| {},
        );
    };
    inspect(&module.body);
    for screen in &module.screens {
        inspect(&screen.body);
    }
    for component in &module.components {
        inspect(&component.body);
    }
    error.map_or(Ok(()), Err)
}

fn display_only(nodes: &[Node], module: &Module, seen: &mut HashSet<String>) -> bool {
    let mut valid = true;
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| {
            if !valid {
                return;
            }
            valid = match node {
                Node::Layout { style, .. } => style.marquee.is_none(),
                Node::Text { style, .. } => !style.selectable,
                Node::ComponentCall {
                    name,
                    component_ref,
                    ..
                } => {
                    component_ref.is_none()
                        && (!seen.insert(name.clone())
                            || module
                                .components
                                .iter()
                                .find(|c| c.name == *name)
                                .is_some_and(|c| display_only(&c.body, module, seen)))
                }
                Node::ViewInsets { .. }
                | Node::Appearance { .. }
                | Node::Accessibility { .. }
                | Node::If { .. }
                | Node::When { .. }
                | Node::Content
                | Node::OnAppear { .. }
                | Node::OnDisappear { .. }
                | Node::OnActive { .. }
                | Node::OnInactive { .. }
                | Node::OnBackground { .. }
                | Node::Spacer { .. }
                | Node::Divider { .. }
                | Node::Shape { .. }
                | Node::Image { .. }
                | Node::SystemIcon { .. }
                | Node::FontIcon { .. }
                | Node::LinearGradient { .. } => true,
                _ => false,
            };
        },
        &mut |_| {},
    );
    valid
}
