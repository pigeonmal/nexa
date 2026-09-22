use std::collections::HashMap;

use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::{
    Action, DirectionConfig, Function, FunctionParameter, Module, Node, Screen, ScreenId, State,
    StatusBarConfig,
};
use nexa_syntax::ast;

use self::{
    components::lower_nodes,
    custom_components::{lower_components, retain_reachable},
    expressions::{
        FunctionSignatures, collect_function_signatures, lower_expr, references_state,
        resolve_declaration_type,
    },
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
    let function_signatures = collect_function_signatures(&app.functions)?;
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
        &function_signatures,
        target,
    )?;

    let functions = lower_functions(std::mem::take(&mut app.functions), &function_signatures)?;

    let mut symbols = HashMap::new();
    let mut states = Vec::with_capacity(app.states.len());
    for declaration in app.states {
        if symbols.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!("`{}` is already declared", declaration.name),
            ));
        }
        if function_signatures.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!("`{}` is already declared as a function", declaration.name),
            ));
        }
        let ty = resolve_declaration_type(&declaration, &symbols, &function_signatures)?;
        let initial = lower_expr(
            &declaration.initial,
            Some(&ty),
            &symbols,
            &function_signatures,
            false,
        )?;
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
            &function_signatures,
            false,
            target,
        )?;
        if screen_body.iter().any(contains_status_bar) {
            return Err(CompileError::new(
                app.span,
                "StatusBar is only allowed once at the app body's top level",
            ));
        }
        if screen_body.iter().any(contains_direction) {
            return Err(CompileError::new(
                app.span,
                "Direction is only allowed at the app body's top level",
            ));
        }
        let (on_appear, on_appear_async, screen_body) =
            extract_on_appear(screen_body, screen.span, "screen")?;
        let (on_disappear, screen_body) = extract_on_disappear(screen_body, screen.span, "screen")?;
        screens.push(Screen {
            id: ScreenId(index),
            name: screen.name,
            body: screen_body,
            on_appear,
            on_appear_async,
            on_disappear,
        });
    }

    let body = lower_nodes(
        app.body,
        &symbols,
        &screen_ids,
        &themes,
        &component_signatures,
        &function_signatures,
        true,
        target,
    )?;
    let (status_bar, body) = extract_status_bar(body, app.span)?;
    let (direction, body) = extract_direction(body, app.span)?;
    let (on_appear, on_appear_async, body) = extract_on_appear(body, app.span, "app")?;
    let (on_disappear, body) = extract_on_disappear(body, app.span, "app")?;
    let components = retain_reachable(components, &body, &screens);
    let mut module = Module {
        app_name: app.name,
        functions,
        states,
        screens,
        components,
        body,
        status_bar,
        direction,
        on_appear,
        on_appear_async,
        on_disappear,
    };
    crate::optimize::optimize(&mut module);
    Ok((module, warnings))
}

fn lower_functions(
    declarations: Vec<ast::FunctionDecl>,
    signatures: &FunctionSignatures,
) -> Result<Vec<Function>, CompileError> {
    declarations
        .into_iter()
        .map(|declaration| {
            let signature = signatures
                .get(&declaration.name)
                .expect("collected signature");
            let [statement] = declaration.body.as_slice() else {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "function `{}` must contain exactly one `return` statement",
                        declaration.name
                    ),
                ));
            };
            let ast::Stmt::Return { value, .. } = statement else {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "function `{}` must contain exactly one `return` statement",
                        declaration.name
                    ),
                ));
            };
            let symbols = signature
                .parameters
                .iter()
                .map(|(name, ty)| (name.clone(), (ty.clone(), false)))
                .collect::<HashMap<_, _>>();
            let body = lower_expr(
                value,
                Some(&signature.return_type),
                &symbols,
                signatures,
                signature.is_async,
            )?;
            Ok(Function {
                name: declaration.name,
                is_async: signature.is_async,
                parameters: signature
                    .parameters
                    .iter()
                    .map(|(name, ty)| FunctionParameter {
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                    .collect(),
                return_type: signature.return_type.clone(),
                body,
            })
        })
        .collect()
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
            ast::Node::StatusBar { .. } => {}
            ast::Node::Direction { .. } => {}
            ast::Node::OnAppear { .. } => {}
            ast::Node::OnDisappear { .. } => {}
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

fn extract_status_bar(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
) -> Result<(Option<StatusBarConfig>, Vec<Node>), CompileError> {
    let mut config = None;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::StatusBar { config: value } => {
                if config.replace(value).is_some() {
                    return Err(CompileError::new(
                        span,
                        "an app can declare only one top-level StatusBar",
                    ));
                }
            }
            node if contains_status_bar(&node) => {
                return Err(CompileError::new(
                    span,
                    "StatusBar is only allowed at the app body's top level",
                ));
            }
            node => body.push(node),
        }
    }
    Ok((config, body))
}

pub(super) fn contains_status_bar(node: &Node) -> bool {
    match node {
        Node::StatusBar { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_status_bar),
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .any(|tab| tab.children.iter().any(contains_status_bar)),
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_status_bar)
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(contains_status_bar))
        }
        Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. } => false,
    }
}

fn extract_direction(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
) -> Result<(Option<DirectionConfig>, Vec<Node>), CompileError> {
    let mut config = None;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::Direction { config: value } => {
                if config.replace(value).is_some() {
                    return Err(CompileError::new(
                        span,
                        "an app can declare only one top-level Direction",
                    ));
                }
            }
            node if contains_direction(&node) => {
                return Err(CompileError::new(
                    span,
                    "Direction is only allowed at the app body's top level",
                ));
            }
            node => body.push(node),
        }
    }
    Ok((config, body))
}

pub(super) fn contains_direction(node: &Node) -> bool {
    match node {
        Node::Direction { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_direction),
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .any(|tab| tab.children.iter().any(contains_direction)),
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_direction)
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(contains_direction))
        }
        Node::StatusBar { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. } => false,
    }
}

fn extract_on_appear(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
    scope: &str,
) -> Result<(Option<Vec<Action>>, bool, Vec<Node>), CompileError> {
    let mut actions = None;
    let mut asynchronous = false;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::OnAppear {
                actions: value,
                asynchronous: value_is_async,
            } => {
                if actions.replace(value).is_some() {
                    return Err(CompileError::new(
                        span,
                        format!("a {scope} can declare only one top-level OnAppear"),
                    ));
                }
                asynchronous = value_is_async;
            }
            node if contains_on_appear(&node) => {
                return Err(CompileError::new(
                    span,
                    format!("OnAppear is only allowed at the {scope} body's top level"),
                ));
            }
            node => body.push(node),
        }
    }
    Ok((actions, asynchronous, body))
}

pub(super) fn contains_on_appear(node: &Node) -> bool {
    match node {
        Node::OnAppear { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_on_appear),
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .any(|tab| tab.children.iter().any(contains_on_appear)),
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_on_appear)
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(contains_on_appear))
        }
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::OnDisappear { .. } => false,
    }
}

fn extract_on_disappear(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
    scope: &str,
) -> Result<(Option<Vec<Action>>, Vec<Node>), CompileError> {
    let mut actions = None;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::OnDisappear { actions: value } => {
                if actions.replace(value).is_some() {
                    return Err(CompileError::new(
                        span,
                        format!("a {scope} can declare only one top-level OnDisappear"),
                    ));
                }
            }
            node if contains_on_disappear(&node) => {
                return Err(CompileError::new(
                    span,
                    format!("OnDisappear is only allowed at the {scope} body's top level"),
                ));
            }
            node => body.push(node),
        }
    }
    Ok((actions, body))
}

pub(super) fn contains_on_disappear(node: &Node) -> bool {
    match node {
        Node::OnDisappear { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_on_disappear),
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .any(|tab| tab.children.iter().any(contains_on_disappear)),
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_on_disappear)
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(contains_on_disappear))
        }
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. } => false,
    }
}
