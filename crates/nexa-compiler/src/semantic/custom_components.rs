use std::collections::{HashMap, HashSet};

use nexa_diagnostics::CompileError;
use nexa_ir::{
    Component, ComponentParameter, Function, FunctionParameter, Node, Screen, State, Type,
};
use nexa_syntax::ast;

use super::{
    components::lower_nodes,
    context::{ExprContext, ScreenSignatures, SemanticContext},
    expressions::{
        FunctionSignature, FunctionSignatures, StructTypes, functions_with_error_handling,
        lower_expr, parse_return_type, parse_type, record_native_alias, references_mutable_state,
        resolve_declaration_type, resolve_struct_type, validate_task_handle_state,
    },
    themes::ThemeSymbols,
};
use crate::Target;

#[derive(Clone)]
pub(super) struct ComponentSignature {
    pub(super) parameters: Vec<(String, Type)>,
    pub(super) defaults: HashMap<String, ast::Expr>,
    pub(super) events: Vec<ComponentEventSignature>,
    pub(super) has_content_slot: bool,
    pub(super) native: bool,
    pub(super) supports_ref: bool,
}

#[derive(Clone)]
pub(super) struct ComponentEventSignature {
    pub(super) property: String,
    pub(super) parameters: Vec<(String, Type)>,
}

pub(super) type ComponentSignatures = HashMap<String, ComponentSignature>;

struct RefMethodLoweringContext<'a> {
    component_name: &'a str,
    component_symbols: &'a HashMap<String, (Type, bool)>,
    functions: &'a FunctionSignatures,
    structs: &'a StructTypes,
    enums: &'a HashSet<String>,
    nullable_generic_plugin_reads: bool,
    source_file: Option<&'a str>,
}

/// Adds typed `ComponentRef<T>` constructors and method signatures before any
/// app or component expressions are lowered.
pub(super) fn collect_ref_method_signatures(
    declarations: &[ast::ComponentDecl],
    structs: &StructTypes,
    functions: &mut FunctionSignatures,
) -> Result<(), CompileError> {
    for component in declarations {
        if component.ref_methods.is_empty() {
            continue;
        }
        let receiver = Type::ComponentRef {
            namespace: "Nexa".to_owned(),
            name: component.name.clone(),
        };
        let constructor_key = format!("Nexa.{}.__component_ref", component.name);
        if functions.contains_key(&constructor_key) {
            return Err(CompileError::new(
                component.span,
                format!(
                    "component ref target `{}` is already declared",
                    component.name
                ),
            ));
        }
        functions.insert(
            constructor_key,
            FunctionSignature {
                parameters: Vec::new(),
                type_parameters: Vec::new(),
                return_type: receiver.clone(),
                is_async: false,
                is_throwing: false,
                receiver: None,
                is_constructor: true,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        );
        for method in &component.ref_methods {
            if method.failure_type.is_some() && !method.is_throwing {
                return Err(CompileError::new(
                    method.span,
                    "a component ref failure type requires `throws`",
                ));
            }
            let return_type = resolve_component_type(
                &parse_return_type(&method.return_type)?,
                structs,
                functions,
            );
            if method.is_throwing && matches!(return_type, Type::Result(_, _)) {
                return Err(CompileError::new(
                    method.span,
                    "a component ref method cannot use both `throws` and a `Result` return type; use one failure style",
                ));
            }
            let error_type = if method.is_throwing {
                let declared = method.failure_type.as_ref().ok_or_else(|| {
                    CompileError::new(
                        method.span,
                        "throwing component ref methods must name their plugin error type, for example `throws Video.PlayerFailure`",
                    )
                })?;
                let ast::TypeSyntax::Named(declared_name, _) = declared else {
                    return Err(CompileError::new(
                        declared.span(),
                        "component ref failure type must name a plugin error type",
                    ));
                };
                let matching_error = functions
                    .values()
                    .filter_map(|signature| signature.error_type.as_ref())
                    .find(|error| format!("{}.{}", error.namespace, error.name) == *declared_name)
                    .cloned()
                    .ok_or_else(|| {
                        CompileError::new(
                            declared.span(),
                            format!("`{declared_name}` is not a declared plugin error type"),
                        )
                    })?;
                Some(matching_error)
            } else {
                None
            };
            let key = format!("Nexa.{}.{}", component.name, method.name);
            if functions.contains_key(&key) {
                return Err(CompileError::new(
                    method.span,
                    format!("component ref method `{}` is already declared", method.name),
                ));
            }
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    Ok((
                        parameter.name.clone(),
                        resolve_component_type(&parse_type(&parameter.ty)?, structs, functions),
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            functions.insert(
                key,
                FunctionSignature {
                    parameters,
                    type_parameters: Vec::new(),
                    return_type,
                    is_async: method.is_async,
                    is_throwing: method.is_throwing,
                    receiver: Some(receiver.clone()),
                    is_constructor: false,
                    is_mutable_property: false,
                    error_handling_allowed: false,
                    error_type,
                },
            );
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct ComponentLoweringContext<'a> {
    pub(super) screen_ids: &'a ScreenSignatures,
    pub(super) themes: &'a ThemeSymbols,
    pub(super) functions: &'a FunctionSignatures,
    pub(super) structs: &'a StructTypes,
    pub(super) enums: &'a HashSet<String>,
    pub(super) enum_symbols: &'a HashMap<String, (Type, bool)>,
    pub(super) global_symbols: &'a HashMap<String, (Type, bool)>,
    pub(super) external_signatures: &'a ComponentSignatures,
    pub(super) target: Target,
    pub(super) nullable_generic_plugin_reads: bool,
    pub(super) custom_icons: Option<&'a HashMap<String, nexa_ir::SystemIcon>>,
}

pub(super) fn retain_reachable(
    components: Vec<Component>,
    body: &[Node],
    screens: &[Screen],
    widgets: &[nexa_ir::Widget],
) -> Vec<Component> {
    let components_by_name = components
        .iter()
        .map(|component| (component.name.as_str(), component))
        .collect::<HashMap<_, _>>();
    let mut pending = HashSet::new();
    for node in body
        .iter()
        .chain(screens.iter().flat_map(|screen| screen.body.iter()))
        .chain(widgets.iter().flat_map(|widget| widget.body.iter()))
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
    context: ComponentLoweringContext<'_>,
) -> Result<(Vec<Component>, ComponentSignatures), CompileError> {
    let mut signatures = collect_signatures(&declarations, context.structs, context.functions)?;
    for (name, signature) in context.external_signatures {
        if signatures.contains_key(name) {
            let span = declarations
                .iter()
                .find(|declaration| declaration.name == *name)
                .map(|declaration| declaration.span)
                .unwrap_or_else(nexa_diagnostics::Span::default);
            return Err(CompileError::new(
                span,
                format!("native component `{name}` conflicts with a custom component"),
            ));
        }
        signatures.insert(name.clone(), signature.clone());
    }
    validate_acyclic(&declarations, &signatures)?;

    let mut components = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let source_file = declaration.source_file.clone();
        let result = lower_component(declaration, &signatures, context);
        components.push(result.map_err(|error| in_file(error, source_file.as_deref()))?);
    }
    Ok((components, signatures))
}

fn collect_signatures(
    declarations: &[ast::ComponentDecl],
    structs: &StructTypes,
    functions: &FunctionSignatures,
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
                parameters.push((
                    parameter.name.clone(),
                    resolve_component_type(&parse_type(&parameter.ty)?, structs, functions),
                ));
            }
            Ok(ComponentSignature {
                parameters,
                defaults: HashMap::new(),
                events: Vec::new(),
                has_content_slot: declaration.body.iter().any(contains_content_slot),
                native: false,
                supports_ref: !declaration.ref_methods.is_empty(),
            })
        })()
        .map_err(|error| in_file(error, declaration.source_file.as_deref()))?;
        signatures.insert(declaration.name.clone(), result);
    }
    Ok(signatures)
}

fn resolve_component_type(
    ty: &Type,
    structs: &StructTypes,
    functions: &FunctionSignatures,
) -> Type {
    let ty = resolve_struct_type(ty, structs);
    match ty {
        Type::Enum(name) => functions
            .get(&name)
            .filter(|signature| signature.is_constructor)
            .map(|signature| signature.return_type.clone())
            .unwrap_or(Type::Enum(name)),
        Type::Optional(inner) => {
            Type::Optional(Box::new(resolve_component_type(&inner, structs, functions)))
        }
        Type::Array(inner) => {
            Type::Array(Box::new(resolve_component_type(&inner, structs, functions)))
        }
        Type::Set(inner) => Type::Set(Box::new(resolve_component_type(&inner, structs, functions))),
        Type::Map(key, value) => Type::Map(
            Box::new(resolve_component_type(&key, structs, functions)),
            Box::new(resolve_component_type(&value, structs, functions)),
        ),
        Type::Pair(first, second) => Type::Pair(
            Box::new(resolve_component_type(&first, structs, functions)),
            Box::new(resolve_component_type(&second, structs, functions)),
        ),
        Type::Triple(first, second, third) => Type::Triple(
            Box::new(resolve_component_type(&first, structs, functions)),
            Box::new(resolve_component_type(&second, structs, functions)),
            Box::new(resolve_component_type(&third, structs, functions)),
        ),
        _ => ty,
    }
}

fn lower_component(
    declaration: ast::ComponentDecl,
    signatures: &ComponentSignatures,
    context: ComponentLoweringContext<'_>,
) -> Result<Component, CompileError> {
    let ComponentLoweringContext {
        screen_ids,
        themes,
        functions,
        structs,
        enums,
        enum_symbols,
        global_symbols,
        target,
        nullable_generic_plugin_reads,
        ..
    } = context;
    let source_file = declaration.source_file.clone();
    let ref_method_declarations = declaration.ref_methods.clone();
    let signature = &signatures[&declaration.name];
    let mut symbols = HashMap::with_capacity(signature.parameters.len() + declaration.states.len());
    symbols.extend(
        enum_symbols
            .iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    symbols.extend(
        global_symbols
            .iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    for (name, ty) in &signature.parameters {
        symbols.insert(name.clone(), (ty.clone(), false));
    }

    let mut states = Vec::with_capacity(declaration.states.len());
    let mut native_aliases = HashMap::new();
    for (name, ty) in &signature.parameters {
        if super::components::class_has_dispose_method(ty, functions) {
            native_aliases.insert(name.clone(), format!("@borrowed-native-parameter:{name}"));
        }
    }
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
        let ty = resolve_declaration_type(&state, &symbols, functions, structs)?;
        validate_task_handle_state(
            &ty,
            state.mutable,
            matches!(&state.initial, ast::Expr::Null(_)),
            state.span,
        )?;
        let state_context = ExprContext::with_types(&symbols, functions, false, structs, enums)
            .with_nullable_generic_plugin_reads(nullable_generic_plugin_reads)
            .with_source_file(source_file.as_deref());
        let initial = lower_expr(&state.initial, Some(&ty), &state_context)?;
        let mutable = state.mutable;
        if mutable && references_mutable_state(&state.initial, &symbols) {
            return Err(CompileError::new(
                state.initial.span(),
                "mutable component state initializers cannot refer to mutable state values",
            ));
        }
        record_native_alias(&state.name, &ty, &initial, &mut native_aliases);
        symbols.insert(state.name.clone(), (ty.clone(), mutable));
        states.push(State {
            name: state.name,
            ty,
            initial,
            mutable,
        });
    }

    let cx = SemanticContext::new(
        &symbols,
        screen_ids,
        themes,
        signatures,
        functions,
        &native_aliases,
        structs,
        enums,
        target,
        nullable_generic_plugin_reads,
    )
    .with_navigation(false, !screen_ids.is_empty())
    .with_optional_custom_icons(context.custom_icons)
    .with_source_file(source_file.as_deref());
    let body = lower_nodes(declaration.body, &cx)?;
    let (on_appear, on_appear_async, body) =
        super::extract_on_appear(body, declaration.span, "component")?;
    let (on_disappear, body) = super::extract_on_disappear(body, declaration.span, "component")?;
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
    if body.iter().any(super::contains_on_active)
        || body.iter().any(super::contains_on_inactive)
        || body.iter().any(super::contains_on_background)
    {
        return Err(CompileError::new(
            declaration.span,
            "app lifecycle events are only allowed at the app body's top level",
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
    let ref_methods = lower_ref_methods(
        ref_method_declarations,
        RefMethodLoweringContext {
            component_name: &declaration.name,
            component_symbols: &symbols,
            functions,
            structs,
            enums,
            nullable_generic_plugin_reads,
            source_file: source_file.as_deref(),
        },
    )?;
    Ok(Component {
        name: declaration.name,
        source_file,
        parameters,
        states,
        body,
        ref_methods,
        on_appear,
        on_appear_async,
        on_disappear,
    })
}

fn lower_ref_methods(
    declarations: Vec<ast::FunctionDecl>,
    context: RefMethodLoweringContext<'_>,
) -> Result<Vec<Function>, CompileError> {
    let RefMethodLoweringContext {
        component_name,
        component_symbols,
        functions,
        structs,
        enums,
        nullable_generic_plugin_reads,
        source_file,
    } = context;
    declarations
        .into_iter()
        .map(|declaration| {
            let name = format!("Nexa.{component_name}.{}", declaration.name);
            let signature = functions.get(&name).ok_or_else(|| {
                CompileError::new(
                    declaration.span,
                    format!("component ref signature for `{name}` was not collected"),
                )
            })?;
            let mut symbols = component_symbols.clone();
            for (parameter, ty) in &signature.parameters {
                if symbols.contains_key(parameter) {
                    return Err(CompileError::new(
                        declaration.span,
                        format!("component ref method parameter `{parameter}` conflicts with a component binding"),
                    ));
                }
                symbols.insert(parameter.clone(), (ty.clone(), false));
            }
            let scoped_functions = if signature.is_throwing {
                functions_with_error_handling(functions, true)
            } else {
                functions.clone()
            };
            let (body_actions, returns) = super::lower_function_statements(
                declaration.body,
                &symbols,
                &scoped_functions,
                &signature.return_type,
                signature.is_async,
                structs,
                enums,
                nullable_generic_plugin_reads,
                declaration.span,
                source_file,
                0,
            )?;
            if let Some(failure) = method_failure_type(signature) {
                validate_component_ref_failure_type(
                    &body_actions,
                    functions,
                    &failure,
                    declaration.span,
                )?;
            }
            if !returns && signature.return_type != Type::Void {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "component ref method `{}` must return a value of type `{}` on every path",
                        declaration.name,
                        super::expressions::type_name(&signature.return_type),
                    ),
                ));
            }
            Ok(Function {
                name,
                receiver: signature.receiver.clone(),
                class_initializers: Vec::new(),
                is_async: signature.is_async,
                is_throwing: signature.is_throwing,
                failure_type: signature.error_type.as_ref().map(|error| Type::Plugin {
                    namespace: error.namespace.clone(),
                    name: error.name.clone(),
                }),
                parameters: signature
                    .parameters
                    .iter()
                    .map(|(name, ty)| FunctionParameter {
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                    .collect(),
                locals: Vec::new(),
                return_type: signature.return_type.clone(),
                body: nexa_ir::Expr::Bool(false),
                body_actions: Some(body_actions),
            })
        })
        .collect()
}

fn method_failure_type(signature: &FunctionSignature) -> Option<Type> {
    signature.error_type.as_ref().map(|error| Type::Plugin {
        namespace: error.namespace.clone(),
        name: error.name.clone(),
    })
}

fn validate_component_ref_failure_type(
    actions: &[nexa_ir::Action],
    functions: &FunctionSignatures,
    expected_failure: &Type,
    span: nexa_diagnostics::Span,
) -> Result<(), CompileError> {
    let mut mismatch = None;
    nexa_ir::walk::walk_actions(actions, &mut |expression| {
        let (receiver, namespace, name, is_throwing, source_span) = match expression {
            nexa_ir::Expr::NativeCall {
                receiver,
                namespace,
                name,
                is_throwing,
                source_span,
                ..
            } => (
                receiver.as_deref(),
                namespace.as_str(),
                name.as_str(),
                *is_throwing,
                *source_span,
            ),
            nexa_ir::Expr::Call {
                name, is_throwing, ..
            } if *is_throwing => {
                mismatch = Some((name.clone(), span));
                return;
            }
            _ => return,
        };
        if !is_throwing {
            return;
        }
        let signature_name = if let Some(receiver) = receiver {
            match super::expressions::lowered_type(receiver) {
                Some(Type::Plugin {
                    namespace: _,
                    name: class_name,
                }) => format!("{class_name}.{name}"),
                _ => format!("{namespace}.{name}"),
            }
        } else {
            format!("{namespace}.{name}")
        };
        let actual_failure = functions
            .get(&signature_name)
            .and_then(|signature| signature.error_type.as_ref())
            .map(|error| Type::Plugin {
                namespace: error.namespace.clone(),
                name: error.name.clone(),
            });
        if actual_failure.as_ref() != Some(expected_failure) {
            mismatch = Some((signature_name, source_span.unwrap_or(span)));
        }
    });
    if let Some((name, span)) = mismatch {
        return Err(CompileError::new(
            span,
            format!(
                "component ref method can only propagate its declared failure type `{}`; call `{name}` declares a different or untyped failure",
                super::expressions::type_name(expected_failure)
            ),
        ));
    }
    Ok(())
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
        ast::Node::ComponentCall { name, children, .. } => {
            calls.push(name.clone());
            if let Some(children) = children {
                for child in children {
                    collect_component_calls(child, calls);
                }
            }
        }
        ast::Node::NativeComponentCall { children, .. } => {
            if let Some(children) = children {
                for child in children {
                    collect_component_calls(child, calls);
                }
            }
        }
        ast::Node::ComponentInvocation(inv) => {
            for group in invocation_child_nodes(inv) {
                for child in group {
                    collect_component_calls(child, calls);
                }
            }
        }
        ast::Node::Platform { children, .. } => {
            for child in children {
                collect_component_calls(child, calls);
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
        ast::Node::When {
            cases, else_body, ..
        } => {
            for case in cases {
                for child in &case.body {
                    collect_component_calls(child, calls);
                }
            }
            for child in else_body {
                collect_component_calls(child, calls);
            }
        }
    }
}

/// Child-node groups of a built-in invocation: child block plus node-block
/// modifiers. Action bodies hold statements only and are skipped, exactly as
/// the previous per-variant walkers did.
fn invocation_child_nodes(inv: &ast::ComponentInvocation) -> Vec<&Vec<ast::Node>> {
    let mut groups = Vec::new();
    match &inv.children {
        ast::ChildBody::Nodes(children) => groups.push(children),
        ast::ChildBody::Tabs(tabs) => {
            for tab in tabs {
                groups.push(&tab.children);
            }
        }
        ast::ChildBody::AppBottomBar { tabs, custom_bar } => {
            for tab in tabs {
                groups.push(&tab.children);
            }
            if let Some(custom_bar) = custom_bar {
                groups.push(custom_bar);
            }
        }
        ast::ChildBody::Rows(rows) => groups.push(&rows.children),
        ast::ChildBody::SplitPanes { sidebar, detail } => {
            groups.push(sidebar);
            groups.push(detail);
        }
        ast::ChildBody::None | ast::ChildBody::Actions(_) => {}
    }
    for modifier in &inv.modifiers {
        if let ast::ModifierBody::Nodes(nodes) = &modifier.body {
            groups.push(nodes);
        }
    }
    groups
}

fn collect_ir_component_calls(node: &Node, calls: &mut HashSet<String>) {
    match node {
        Node::ComponentCall { name, children, .. } => {
            calls.insert(name.clone());
            if let Some(children) = children {
                for child in children {
                    collect_ir_component_calls(child, calls);
                }
            }
        }
        Node::NativeComponentCall { children, .. } => {
            if let Some(children) = children {
                for child in children {
                    collect_ir_component_calls(child, calls);
                }
            }
        }
        Node::Content
        | Node::Spacer { .. }
        | Node::Divider { .. }
        | Node::Shape { .. }
        | Node::ContentUnavailable { .. } => {}
        Node::ViewInsets { child, .. } => collect_ir_component_calls(child, calls),
        Node::Layout { children, .. }
        | Node::Form { children, .. }
        | Node::FormSection { children, .. }
        | Node::Toolbar { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::Dialog { children, .. }
        | Node::ConfirmationDialog { children, .. }
        | Node::RefreshControl { children, .. } => {
            for child in children {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::SafeAreaInset {
            content, children, ..
        } => {
            for child in content.iter().chain(children) {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::ScrollHeader {
            header, children, ..
        } => {
            for child in header.iter().chain(children) {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::NavigationSplitView {
            sidebar, detail, ..
        } => {
            for child in sidebar.iter().chain(detail) {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::FastList { plan } => {
            for child in plan.children() {
                collect_ir_component_calls(child, calls);
            }
            if let Some(sticky_header) = plan.sticky_header() {
                for child in sticky_header {
                    collect_ir_component_calls(child, calls);
                }
            }
            if let Some(section_header) = plan.section_header() {
                for child in section_header {
                    collect_ir_component_calls(child, calls);
                }
            }
        }
        Node::AppBottomBar {
            tabs, custom_bar, ..
        } => {
            for tab in tabs {
                for child in &tab.children {
                    collect_ir_component_calls(child, calls);
                }
            }
            if let Some(custom_bar) = custom_bar {
                for child in custom_bar {
                    collect_ir_component_calls(child, calls);
                }
            }
        }
        Node::PagePager { pages, .. } => {
            for page in pages {
                for child in page {
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
        Node::When {
            cases, else_body, ..
        } => {
            for case in cases {
                for child in &case.body {
                    collect_ir_component_calls(child, calls);
                }
            }
            for child in else_body.iter().flatten() {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::Appearance { children, .. } => {
            for child in children {
                collect_ir_component_calls(child, calls);
            }
        }
        Node::StatusBar { .. } => {}
        Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Slider { .. }
        | Node::ProgressBar { .. }
        | Node::ProgressRing { .. }
        | Node::SegmentedControl { .. }
        | Node::Picker { .. }
        | Node::PickerDate { .. }
        | Node::Image { .. }
        | Node::SystemIcon { .. }
        | Node::FontIcon { .. }
        | Node::LinearGradient { .. }
        | Node::NavigationStack { .. }
        | Node::NavigationBack { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => {}
    }
}

fn in_file(error: CompileError, file: Option<&str>) -> CompileError {
    if let Some(file) = file {
        error.with_file(file)
    } else {
        error
    }
}

fn contains_content_slot(node: &ast::Node) -> bool {
    match node {
        ast::Node::ComponentInvocation(inv) if inv.name == "Content" => true,
        ast::Node::ComponentCall { children, .. } => children
            .as_ref()
            .is_some_and(|children| children.iter().any(contains_content_slot)),
        ast::Node::NativeComponentCall { children, .. } => children
            .as_ref()
            .is_some_and(|children| children.iter().any(contains_content_slot)),
        ast::Node::ComponentInvocation(inv) => invocation_child_nodes(inv)
            .iter()
            .any(|group| group.iter().any(contains_content_slot)),
        ast::Node::Platform { children, .. } => children.iter().any(contains_content_slot),
        ast::Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_content_slot)
                || else_body
                    .as_ref()
                    .is_some_and(|body| body.iter().any(contains_content_slot))
        }
        ast::Node::When {
            cases, else_body, ..
        } => cases
            .iter()
            .flat_map(|case| &case.body)
            .chain(else_body)
            .any(contains_content_slot),
    }
}

fn is_builtin_component(name: &str) -> bool {
    // Single source of truth: every catalogued built-in (primary name or
    // alias) is reserved and cannot be redeclared as a custom component.
    nexa_syntax::catalog::is_component(name)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap, HashSet};

    use nexa_diagnostics::Span;
    use nexa_ir::Type;
    use nexa_syntax::ast;

    use super::{ComponentLoweringContext, lower_components};
    use crate::{Target, semantic::expressions::FunctionSignature, semantic::themes::ThemeSymbols};

    #[test]
    fn native_class_component_parameters_are_borrowed() {
        let span = Span::default();
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let functions = HashMap::from([
            (
                "Video.VideoPlayer".to_owned(),
                FunctionSignature {
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: player_type.clone(),
                    is_async: false,
                    is_throwing: false,
                    receiver: None,
                    is_constructor: true,
                    is_mutable_property: false,
                    error_handling_allowed: false,
                    error_type: None,
                },
            ),
            (
                "VideoPlayer.dispose".to_owned(),
                FunctionSignature {
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: Type::Void,
                    is_async: false,
                    is_throwing: false,
                    receiver: Some(player_type),
                    is_constructor: false,
                    is_mutable_property: false,
                    error_handling_allowed: false,
                    error_type: None,
                },
            ),
        ]);
        let declaration = ast::ComponentDecl {
            name: "PlayerSurface".to_owned(),
            parameters: vec![ast::ComponentParameter {
                name: "player".to_owned(),
                ty: ast::TypeSyntax::Named("Video.VideoPlayer".to_owned(), span),
                span,
            }],
            states: Vec::new(),
            ref_methods: Vec::new(),
            body: vec![ast::Node::ComponentInvocation(ast::ComponentInvocation {
                name: "Button".to_owned(),
                span,
                positional: vec![ast::Expr::String("Dispose".to_owned(), span)],
                arguments: BTreeMap::new(),
                flags: Vec::new(),
                children: ast::ChildBody::Actions(vec![ast::Stmt::Expression {
                    expression: ast::Expr::MethodCall {
                        type_arguments: Vec::new(),
                        base: Box::new(ast::Expr::Name("player".to_owned(), span)),
                        name: "dispose".to_owned(),
                        arguments: Vec::new(),
                        named_arguments: BTreeMap::new(),
                        span,
                    },
                    span,
                }]),
                modifiers: Vec::new(),
            })],
            span,
            source_file: None,
        };

        let result = lower_components(
            vec![declaration],
            ComponentLoweringContext {
                screen_ids: &HashMap::new(),
                themes: &ThemeSymbols::default(),
                functions: &functions,
                structs: &HashMap::new(),
                enums: &HashSet::new(),
                enum_symbols: &HashMap::new(),
                global_symbols: &HashMap::new(),
                external_signatures: &HashMap::new(),
                target: Target::Swift,
                nullable_generic_plugin_reads: false,
                custom_icons: None,
            },
        );
        let Err(error) = result else {
            panic!("a component receives native class values as borrowed parameters");
        };

        assert!(
            error
                .to_string()
                .contains("component parameter `player` is borrowed")
        );
    }
}
