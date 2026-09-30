use std::collections::{HashMap, HashSet};

use nexa_diagnostics::CompileError;
use nexa_ir::{Component, ComponentParameter, Node, Screen, State, Type};
use nexa_syntax::ast;

use super::{
    components::lower_nodes,
    context::{ExprContext, ScreenSignatures, SemanticContext},
    expressions::{
        FunctionSignatures, StructTypes, lower_expr, parse_type, record_native_alias,
        references_state, resolve_declaration_type, resolve_struct_type,
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
}

#[derive(Clone)]
pub(super) struct ComponentEventSignature {
    pub(super) property: String,
    pub(super) parameters: Vec<(String, Type)>,
}

pub(super) type ComponentSignatures = HashMap<String, ComponentSignature>;

#[derive(Clone, Copy)]
pub(super) struct ComponentLoweringContext<'a> {
    pub(super) screen_ids: &'a ScreenSignatures,
    pub(super) themes: &'a ThemeSymbols,
    pub(super) functions: &'a FunctionSignatures,
    pub(super) structs: &'a StructTypes,
    pub(super) enums: &'a HashSet<String>,
    pub(super) enum_symbols: &'a HashMap<String, (Type, bool)>,
    pub(super) external_signatures: &'a ComponentSignatures,
    pub(super) target: Target,
    pub(super) allow_nullable_generic_plugin_reads: bool,
}

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
        target,
        allow_nullable_generic_plugin_reads,
        ..
    } = context;
    let source_file = declaration.source_file.clone();
    let signature = &signatures[&declaration.name];
    let mut symbols = HashMap::with_capacity(signature.parameters.len() + declaration.states.len());
    symbols.extend(
        enum_symbols
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
        let initial = lower_expr(
            &state.initial,
            Some(&ty),
            &ExprContext::with_types(&symbols, functions, false, structs, enums)
                .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
        )?;
        if state.mutable && references_state(&state.initial) {
            return Err(CompileError::new(
                state.initial.span(),
                "mutable component state initializers cannot refer to other state values yet",
            ));
        }
        record_native_alias(&state.name, &ty, &initial, &mut native_aliases);
        symbols.insert(state.name.clone(), (ty.clone(), state.mutable));
        states.push(State {
            name: state.name,
            ty,
            initial,
            mutable: state.mutable,
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
        allow_nullable_generic_plugin_reads,
    )
    .with_navigation(false, false);
    let body = lower_nodes(declaration.body, &cx)?;
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
    Ok(Component {
        name: declaration.name,
        source_file,
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
        ast::ChildBody::Rows(rows) => groups.push(&rows.children),
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
        Node::Content | Node::Spacer | Node::Divider { .. } => {}
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::Dialog { children, .. }
        | Node::RefreshControl { children, .. } => {
            for child in children {
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
        Node::When {
            cases, else_body, ..
        } => {
            for case in cases {
                for child in &case.body {
                    collect_ir_component_calls(child, calls);
                }
            }
            for child in else_body {
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
        | Node::Image { .. }
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
                external_signatures: &HashMap::new(),
                target: Target::Swift,
                allow_nullable_generic_plugin_reads: false,
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
