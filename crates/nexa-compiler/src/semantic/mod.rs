use std::collections::HashMap;

use nexa_diagnostics::CompileError;
use nexa_ir::{Module, Screen, ScreenId, State};
use nexa_syntax::ast;

use self::{
    components::lower_node,
    custom_components::{lower_components, retain_reachable},
    expressions::{lower_expr, references_state, resolve_declaration_type},
    themes::lower_theme,
};

mod components;
mod custom_components;
mod expressions;
mod styles;
mod themes;

pub fn lower(mut app: ast::App) -> Result<Module, CompileError> {
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
    if !app.screens.is_empty()
        && !matches!(app.body.as_slice(), [ast::Node::NavigationStack { .. }])
    {
        return Err(CompileError::new(
            app.span,
            "apps with screen declarations must have one top-level `NavigationStack(root: ScreenName)` in `body`",
        ));
    }

    let (components, component_signatures) =
        lower_components(std::mem::take(&mut app.components), &screen_ids, &themes)?;

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
        let mut screen_body = Vec::with_capacity(screen.body.len());
        for node in screen.body {
            screen_body.push(lower_node(
                node,
                &symbols,
                &screen_ids,
                &themes,
                &component_signatures,
                false,
            )?);
        }
        screens.push(Screen {
            id: ScreenId(index),
            name: screen.name,
            body: screen_body,
        });
    }

    let mut body = Vec::with_capacity(app.body.len());
    for node in app.body {
        let is_navigation_root = matches!(&node, ast::Node::NavigationStack { .. });
        body.push(lower_node(
            node,
            &symbols,
            &screen_ids,
            &themes,
            &component_signatures,
            is_navigation_root,
        )?);
    }
    let components = retain_reachable(components, &body, &screens);
    let mut module = Module {
        app_name: app.name,
        states,
        screens,
        components,
        body,
    };
    crate::optimize::optimize(&mut module);
    Ok(module)
}
