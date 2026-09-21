use std::collections::HashMap;

use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::{Module, Screen, ScreenId, State};
use nexa_syntax::ast;

use self::{
    components::lower_nodes,
    custom_components::{lower_components, retain_reachable},
    expressions::{lower_expr, references_state, resolve_declaration_type},
    themes::lower_theme,
};
use crate::Target;

mod components;
mod custom_components;
mod expressions;
mod styles;
mod themes;
mod warnings;

pub fn lower_with_warnings(
    mut app: ast::App,
    target: Target,
) -> Result<(Module, Vec<CompileWarning>), CompileError> {
    let warnings = warnings::analyze(&app, target);
    let themes = lower_theme(app.theme.as_ref())?;
    let mut screen_ids = HashMap::with_capacity(app.screens.len());
    for (index, screen) in app.screens.iter().enumerate() {
        if screen_ids
            .insert(screen.name.clone(), ScreenId(index))
            .is_some()
        {
            return Err(CompileError::new(
                screen.span,
                format!("screen `{}` is already declared", screen.name),
            ));
        }
    }
    if !app.screens.is_empty() && target != Target::All && !has_navigation_root(&app.body, target) {
        return Err(CompileError::new(
            app.span,
            "apps with screen declarations must have one top-level `NavigationStack(root: ScreenName)` in `body`",
        ));
    }

    let (components, component_signatures) = lower_components(
        std::mem::take(&mut app.components),
        &screen_ids,
        &themes,
        target,
    )?;

    let mut symbols = HashMap::new();
    let mut states = Vec::with_capacity(app.states.len());
    for declaration in app.states {
        if symbols.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!("`{}` is already declared", declaration.name),
            ));
        }
        let ty = resolve_declaration_type(&declaration, &symbols)?;
        let initial = lower_expr(&declaration.initial, Some(&ty), &symbols)?;
        if declaration.mutable && references_state(&declaration.initial) {
            return Err(CompileError::new(
                declaration.initial.span(),
                "mutable state initializers cannot refer to other state values yet",
            ));
        }
        symbols.insert(declaration.name.clone(), (ty.clone(), declaration.mutable));
        states.push(State {
            name: declaration.name,
            ty,
            initial,
            mutable: declaration.mutable,
        });
    }
    let mut screens = Vec::with_capacity(app.screens.len());
    for (index, screen) in app.screens.into_iter().enumerate() {
        let screen_body = lower_nodes(
            screen.body,
            &symbols,
            &screen_ids,
            &themes,
            &component_signatures,
            false,
            target,
        )?;
        screens.push(Screen {
            id: ScreenId(index),
            name: screen.name,
            body: screen_body,
        });
    }

    let body = lower_nodes(
        app.body,
        &symbols,
        &screen_ids,
        &themes,
        &component_signatures,
        true,
        target,
    )?;
    let components = retain_reachable(components, &body, &screens);
    let mut module = Module {
        app_name: app.name,
        states,
        screens,
        components,
        body,
    };
    crate::optimize::optimize(&mut module);
    Ok((module, warnings))
}

fn has_navigation_root(nodes: &[ast::Node], target: Target) -> bool {
    let mut active = Vec::new();
    collect_active_nodes(nodes, target, &mut active);
    matches!(active.as_slice(), [ast::Node::NavigationStack { .. }])
}

fn collect_active_nodes<'a>(
    nodes: &'a [ast::Node],
    target: Target,
    active: &mut Vec<&'a ast::Node>,
) {
    for node in nodes {
        match node {
            ast::Node::Platform {
                target: platform,
                children,
                ..
            } if platform_matches(*platform, target) => {
                collect_active_nodes(children, target, active);
            }
            ast::Node::Platform { .. } => {}
            node => active.push(node),
        }
    }
}

fn platform_matches(platform: ast::PlatformTarget, target: Target) -> bool {
    matches!(
        (platform, target),
        (ast::PlatformTarget::Ios, Target::Swift) | (ast::PlatformTarget::Android, Target::Kotlin)
    )
}
