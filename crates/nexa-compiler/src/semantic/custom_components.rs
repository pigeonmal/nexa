use std::collections::{HashMap, HashSet};

use nexa_diagnostics::CompileError;
use nexa_ir::{Component, ComponentParameter, Node, Screen, State, Type};
use nexa_syntax::ast;

use super::{
    components::lower_nodes,
    expressions::{
        FunctionSignatures, lower_expr, parse_type, references_state, resolve_declaration_type,
    },
    themes::ThemeSymbols,
};
use crate::Target;

#[derive(Clone)]
pub(super) struct ComponentSignature {
    pub(super) parameters: Vec<(String, Type)>,
}

pub(super) type ComponentSignatures = HashMap<String, ComponentSignature>;

pub(super) fn retain_reachable(
    components: Vec<Component>,
    body: &[Node],
    screens: &[Screen],
) -> Vec<Component> {
    let components_by_name = components
        .iter()
        .map(|component| (component.name.as_str(), component))
        .collect::<HashMap<_, _>>();
    let mut pending = HashSet::new();
    for node in body
        .iter()
        .chain(screens.iter().flat_map(|screen| screen.body.iter()))
    {
        collect_ir_component_calls(node, &mut pending);
    }

    let mut reachable = HashSet::new();
    while let Some(name) = pending.iter().next().cloned() {
        pending.remove(&name);
        if !reachable.insert(name.clone()) {
            continue;
        }
        if let Some(component) = components_by_name.get(name.as_str()) {
            for node in &component.body {
                collect_ir_component_calls(node, &mut pending);
            }
        }
    }

    components
        .into_iter()
        .filter(|component| reachable.contains(&component.name))
        .collect()
}

pub(super) fn lower_components(
    declarations: Vec<ast::ComponentDecl>,
    screen_ids: &HashMap<String, nexa_ir::ScreenId>,
    themes: &ThemeSymbols,
    functions: &FunctionSignatures,
    target: Target,
) -> Result<(Vec<Component>, ComponentSignatures), CompileError> {
    let signatures = collect_signatures(&declarations)?;
    validate_acyclic(&declarations, &signatures)?;

    let mut components = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let source_file = declaration.source_file.clone();
        let result = lower_component(
            declaration,
            &signatures,
            screen_ids,
            themes,
            functions,
            target,
        );
        components.push(result.map_err(|error| in_file(error, source_file.as_deref()))?);
    }
    Ok((components, signatures))
}

fn collect_signatures(
    declarations: &[ast::ComponentDecl],
) -> Result<ComponentSignatures, CompileError> {
    let mut signatures = HashMap::with_capacity(declarations.len());
    for declaration in declarations {
        let result = (|| {
            if is_builtin_component(&declaration.name) {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "`{}` is a built-in component name and cannot be redeclared",
                        declaration.name
                    ),
                ));
            }
            if signatures.contains_key(&declaration.name) {
                return Err(CompileError::new(
                    declaration.span,
                    format!("component `{}` is already declared", declaration.name),
                ));
            }
            let mut parameter_names = HashSet::with_capacity(declaration.parameters.len());
            let mut parameters = Vec::with_capacity(declaration.parameters.len());
            for parameter in &declaration.parameters {
                if !parameter_names.insert(parameter.name.as_str()) {
                    return Err(CompileError::new(
                        parameter.span,
                        format!(
                            "component parameter `{}` is declared more than once",
                            parameter.name
                        ),
                    ));
                }
                parameters.push((parameter.name.clone(), parse_type(&parameter.ty)?));
            }
            Ok(ComponentSignature { parameters })
        })()
        .map_err(|error| in_file(error, declaration.source_file.as_deref()))?;
        signatures.insert(declaration.name.clone(), result);
    }
    Ok(signatures)
}

fn lower_component(
    declaration: ast::ComponentDecl,
    signatures: &ComponentSignatures,
    screen_ids: &HashMap<String, nexa_ir::ScreenId>,
    themes: &ThemeSymbols,
    functions: &FunctionSignatures,
    target: Target,
) -> Result<Component, CompileError> {
    if declaration
        .body
        .iter()
        .any(|node| contains_navigation_link(node, target))
    {
        return Err(CompileError::new(
            declaration.span,
            "`NavigationLink` inside a custom component is not supported yet",
        ));
    }
    let signature = &signatures[&declaration.name];
    let mut symbols = HashMap::with_capacity(signature.parameters.len() + declaration.states.len());
    for (name, ty) in &signature.parameters {
        symbols.insert(name.clone(), (ty.clone(), false));
    }

    let mut states = Vec::with_capacity(declaration.states.len());
    for state in declaration.states {
        if symbols.contains_key(&state.name) {
            return Err(CompileError::new(
                state.span,
                format!(
                    "`{}` is already declared in component `{}`",
                    state.name, declaration.name
                ),
            ));
        }
        if functions.contains_key(&state.name) {
            return Err(CompileError::new(
                state.span,
                format!("`{}` is already declared as an app function", state.name),
            ));
        }
        let ty = resolve_declaration_type(&state, &symbols, functions)?;
        let initial = lower_expr(&state.initial, Some(&ty), &symbols, functions, false)?;
        if state.mutable && references_state(&state.initial) {
            return Err(CompileError::new(
                state.initial.span(),
                "mutable component state initializers cannot refer to other state values yet",
            ));
        }
        symbols.insert(state.name.clone(), (ty.clone(), state.mutable));
        states.push(State {
            name: state.name,
            ty,
            initial,
            mutable: state.mutable,
        });
    }

    let body = lower_nodes(
        declaration.body,
        &symbols,
        screen_ids,
        themes,
        signatures,
        functions,
        false,
        target,
    )?;
    if body.iter().any(super::contains_status_bar) {
        return Err(CompileError::new(
            declaration.span,
            "StatusBar is only allowed at the app body's top level",
        ));
    }
    if body.iter().any(super::contains_direction) {
        return Err(CompileError::new(
            declaration.span,
            "Direction is only allowed at the app body's top level",
        ));
    }
    if body.iter().any(super::contains_on_appear) {
        return Err(CompileError::new(
            declaration.span,
            "OnAppear is only allowed at the app body's top level",
        ));
    }
    if body.iter().any(super::contains_on_disappear) {
        return Err(CompileError::new(
            declaration.span,
            "OnDisappear is only allowed at an app or screen body's top level",
        ));
    }

    let parameters = signature
        .parameters
        .iter()
        .map(|(name, ty)| ComponentParameter {
            name: name.clone(),
            ty: ty.clone(),
        })
        .collect();
    Ok(Component {
        name: declaration.name,
        parameters,
        states,
        body,
    })
}

fn validate_acyclic(
    declarations: &[ast::ComponentDecl],
    signatures: &ComponentSignatures,
) -> Result<(), CompileError> {
    let dependencies = declarations
        .iter()
        .map(|component| {
            let mut calls = Vec::new();
            for node in &component.body {
                collect_component_calls(node, &mut calls);
            }
            calls.retain(|name| signatures.contains_key(name));
            (component.name.clone(), calls)
        })
        .collect::<HashMap<_, _>>();
    let declaration_by_name = declarations
        .iter()
        .map(|component| (component.name.as_str(), component))
        .collect::<HashMap<_, _>>();
    let mut completed = HashSet::new();
    let mut active = HashSet::new();

    for declaration in declarations {
        visit_component(
            &declaration.name,
            &dependencies,
            &declaration_by_name,
            &mut completed,
            &mut active,
        )
        .map_err(|error| in_file(error, declaration.source_file.as_deref()))?;
    }
    Ok(())
}

fn visit_component(
    name: &str,
    dependencies: &HashMap<String, Vec<String>>,
    declarations: &HashMap<&str, &ast::ComponentDecl>,
    completed: &mut HashSet<String>,
    active: &mut HashSet<String>,
) -> Result<(), CompileError> {
    if completed.contains(name) {
        return Ok(());
    }
    let Some(declaration) = declarations.get(name) else {
        return Ok(());
    };
    if !active.insert(name.to_owned()) {
        return Err(in_file(
            CompileError::new(
                declaration.span,
                format!(
                    "recursive custom component composition involving `{name}` is not supported"
                ),
            ),
            declaration.source_file.as_deref(),
        ));
    }
    if let Some(children) = dependencies.get(name) {
        for child in children {
            visit_component(child, dependencies, declarations, completed, active)?;
        }
    }
    active.remove(name);
    completed.insert(name.to_owned());
    Ok(())
}

fn collect_component_calls(node: &ast::Node, calls: &mut Vec<String>) {
    match node {
        ast::Node::ComponentCall { name, .. } => calls.push(name.clone()),
        ast::Node::Layout { children, .. }
        | ast::Node::Platform { children, .. }
        | ast::Node::Pressable { children, .. }
        | ast::Node::NavigationLink { children, .. }
        | ast::Node::Link { children, .. }
        | ast::Node::Accessibility { children, .. }
        | ast::Node::KeyboardAware { children, .. }
        | ast::Node::BottomSheet { children, .. }
        | ast::Node::RefreshControl { children, .. }
        | ast::Node::FastList { children, .. } => {
            for child in children {
                collect_component_calls(child, calls);
            }
        }
        ast::Node::AppBottomBar { tabs, .. } => {
            for tab in tabs {
                for child in &tab.children {
                    collect_component_calls(child, calls);
                }
            }
        }
        ast::Node::If {
            then_body,
            else_body,
            ..
        } => {
            for child in then_body.iter().chain(else_body.iter().flatten()) {
                collect_component_calls(child, calls);
            }
        }
        ast::Node::Text { .. }
        | ast::Node::Button { .. }
        | ast::Node::StatusBar { .. }
        | ast::Node::TextInput { .. }
        | ast::Node::Switch { .. }
        | ast::Node::Image { .. }
        | ast::Node::NavigationStack { .. }
        | ast::Node::Direction { .. }
        | ast::Node::OnAppear { .. }
        | ast::Node::OnDisappear { .. } => {}
    }
}

fn contains_navigation_link(node: &ast::Node, target: Target) -> bool {
    match node {
        ast::Node::NavigationLink { .. } => true,
        ast::Node::Platform {
            target: platform,
            children,
            ..
        } => {
            (target == Target::All || platform_matches(*platform, target))
                && children
                    .iter()
                    .any(|child| contains_navigation_link(child, target))
        }
        ast::Node::Layout { children, .. }
        | ast::Node::Pressable { children, .. }
        | ast::Node::KeyboardAware { children, .. }
        | ast::Node::BottomSheet { children, .. }
        | ast::Node::Link { children, .. }
        | ast::Node::Accessibility { children, .. }
        | ast::Node::RefreshControl { children, .. }
        | ast::Node::FastList { children, .. } => children
            .iter()
            .any(|child| contains_navigation_link(child, target)),
        ast::Node::AppBottomBar { tabs, .. } => tabs.iter().any(|tab| {
            tab.children
                .iter()
                .any(|child| contains_navigation_link(child, target))
        }),
        ast::Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body
                .iter()
                .any(|child| contains_navigation_link(child, target))
                || else_body.as_ref().is_some_and(|body| {
                    body.iter()
                        .any(|child| contains_navigation_link(child, target))
                })
        }
        ast::Node::StatusBar { .. } => false,
        ast::Node::Text { .. }
        | ast::Node::Button { .. }
        | ast::Node::TextInput { .. }
        | ast::Node::Switch { .. }
        | ast::Node::Image { .. }
        | ast::Node::NavigationStack { .. }
        | ast::Node::ComponentCall { .. }
        | ast::Node::Direction { .. }
        | ast::Node::OnAppear { .. }
        | ast::Node::OnDisappear { .. } => false,
    }
}

fn platform_matches(platform: ast::PlatformTarget, target: Target) -> bool {
    matches!(
        (platform, target),
        (ast::PlatformTarget::Ios, Target::Swift) | (ast::PlatformTarget::Android, Target::Kotlin)
    )
}

fn collect_ir_component_calls(node: &Node, calls: &mut HashSet<String>) {
    match node {
        Node::ComponentCall { name, .. } => {
            calls.insert(name.clone());
        }
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::FastList { children, .. } => {
            for child in children {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::AppBottomBar { tabs, .. } => {
            for tab in tabs {
                for child in &tab.children {
                    collect_ir_component_calls(child, calls);
                }
            }
        }
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            for child in then_body.iter().chain(else_body.iter().flatten()) {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::StatusBar { .. } => {}
        Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. } => {}
    }
}

fn in_file(error: CompileError, file: Option<&str>) -> CompileError {
    if let Some(file) = file {
        error.with_file(file)
    } else {
        error
    }
}

fn is_builtin_component(name: &str) -> bool {
    matches!(
        name,
        "Column"
            | "Row"
            | "Text"
            | "Button"
            | "TextInput"
            | "Switch"
            | "Image"
            | "Pressable"
            | "NavigationStack"
            | "NavigationLink"
            | "KeyboardAware"
            | "FastList"
    )
}
