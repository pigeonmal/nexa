use std::collections::{HashMap, HashSet};

use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::{
    Action, DirectionConfig, Function, FunctionLocal, FunctionParameter, Module, Node, Screen,
    ScreenId, State, StatusBarConfig, Type,
};
use nexa_syntax::ast;

use self::{
    components::{contains_content, lower_nodes},
    custom_components::{lower_components, retain_reachable},
    expressions::{
        FunctionSignature, FunctionSignatures, StructTypes, collect_function_signatures,
        collect_plugin_signatures, lower_expr, parse_type, references_state,
        resolve_declaration_type, resolve_struct_type, resolve_value_type,
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
    let enum_declarations = lower_enum_declarations(&app.enums)?;
    let (struct_declarations, struct_types) = lower_struct_declarations(&app.structs)?;
    let enum_symbols = enum_symbols(&enum_declarations);
    let enum_names = enum_declarations
        .iter()
        .map(|declaration| declaration.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    validate_declared_types(&app, &enum_names, &struct_types)?;
    for declaration in &struct_declarations {
        if enum_names.contains(declaration.name.as_str()) {
            return Err(CompileError::new(
                app.span,
                format!(
                    "struct `{}` conflicts with an enum of the same name",
                    declaration.name
                ),
            ));
        }
    }
    for declaration in &app.functions {
        if struct_types.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!(
                    "function `{}` conflicts with a struct of the same name",
                    declaration.name
                ),
            ));
        }
    }
    for declaration in &app.components {
        if struct_types.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!(
                    "component `{}` conflicts with a struct of the same name",
                    declaration.name
                ),
            ));
        }
    }
    let mut function_signatures = collect_function_signatures(&app.functions, &struct_types)?;
    for (name, signature) in collect_plugin_signatures(&app.plugins)? {
        if function_signatures.contains_key(&name) {
            return Err(CompileError::new(
                app.span,
                format!("plugin method `{name}` conflicts with an app function"),
            ));
        }
        function_signatures.insert(name, signature);
    }
    for declaration in &struct_declarations {
        if function_signatures.contains_key(&declaration.name) {
            return Err(CompileError::new(
                app.span,
                format!(
                    "struct `{}` conflicts with a function of the same name",
                    declaration.name
                ),
            ));
        }
        function_signatures.insert(
            declaration.name.clone(),
            FunctionSignature {
                parameters: declaration
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty.clone()))
                    .collect(),
                return_type: Type::Struct {
                    name: declaration.name.clone(),
                    fields: declaration
                        .fields
                        .iter()
                        .map(|field| (field.name.clone(), field.ty.clone()))
                        .collect(),
                },
                is_async: false,
                is_throwing: false,
            },
        );
    }
    for declaration in &enum_declarations {
        if function_signatures.contains_key(&declaration.name) {
            return Err(CompileError::new(
                app.span,
                format!(
                    "enum `{}` conflicts with a function of the same name",
                    declaration.name
                ),
            ));
        }
        if app
            .components
            .iter()
            .any(|component| component.name == declaration.name)
        {
            return Err(CompileError::new(
                app.span,
                format!(
                    "enum `{}` conflicts with a component of the same name",
                    declaration.name
                ),
            ));
        }
    }
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
        &struct_types,
        &enum_symbols,
        target,
    )?;

    let functions = lower_functions(
        std::mem::take(&mut app.functions),
        &function_signatures,
        &struct_types,
        &enum_symbols,
    )?;

    let mut symbols = enum_symbols.clone();
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
        let ty =
            resolve_declaration_type(&declaration, &symbols, &function_signatures, &struct_types)?;
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
    let mut all_state_names = states
        .iter()
        .map(|state| state.name.clone())
        .collect::<HashSet<_>>();
    let mut screens = Vec::with_capacity(app.screens.len());
    for (index, screen) in app.screens.into_iter().enumerate() {
        let mut screen_symbols = symbols.clone();
        let mut screen_states = Vec::with_capacity(screen.states.len());
        for declaration in screen.states {
            if screen_symbols.contains_key(&declaration.name)
                || !all_state_names.insert(declaration.name.clone())
            {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "screen state `{}` conflicts with another app or screen state",
                        declaration.name
                    ),
                ));
            }
            if function_signatures.contains_key(&declaration.name) {
                return Err(CompileError::new(
                    declaration.span,
                    format!(
                        "screen state `{}` is already declared as a function",
                        declaration.name
                    ),
                ));
            }
            let ty = resolve_declaration_type(
                &declaration,
                &screen_symbols,
                &function_signatures,
                &struct_types,
            )?;
            let initial = lower_expr(
                &declaration.initial,
                Some(&ty),
                &screen_symbols,
                &function_signatures,
                false,
            )?;
            if declaration.mutable && references_state(&declaration.initial) {
                return Err(CompileError::new(
                    declaration.initial.span(),
                    "mutable screen state initializers cannot refer to other state values yet",
                ));
            }
            screen_symbols.insert(declaration.name.clone(), (ty.clone(), declaration.mutable));
            screen_states.push(State {
                name: declaration.name,
                ty,
                initial,
                mutable: declaration.mutable,
            });
        }
        let screen_body = lower_nodes(
            screen.body,
            &screen_symbols,
            &screen_ids,
            &themes,
            &component_signatures,
            &function_signatures,
            false,
            target,
        )?;
        let (status_bar, screen_body) = extract_status_bar(screen_body, screen.span, "screen")?;
        if screen_body.iter().any(contains_content) {
            return Err(CompileError::new(
                screen.span,
                "Content() is only available inside a custom component declaration",
            ));
        }
        if screen_body.iter().any(contains_direction) {
            return Err(CompileError::new(
                app.span,
                "Direction is only allowed at the app body's top level",
            ));
        }
        if screen_body.iter().any(contains_on_active)
            || screen_body.iter().any(contains_on_inactive)
            || screen_body.iter().any(contains_on_background)
        {
            return Err(CompileError::new(
                screen.span,
                "app lifecycle events are only allowed at the app body's top level",
            ));
        }
        let (on_appear, on_appear_async, screen_body) =
            extract_on_appear(screen_body, screen.span, "screen")?;
        let (on_disappear, screen_body) = extract_on_disappear(screen_body, screen.span, "screen")?;
        screens.push(Screen {
            id: ScreenId(index),
            name: screen.name,
            states: screen_states,
            body: screen_body,
            status_bar,
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
    let (status_bar, body) = extract_status_bar(body, app.span, "app")?;
    if body.iter().any(contains_content) {
        return Err(CompileError::new(
            app.span,
            "Content() is only available inside a custom component declaration",
        ));
    }
    let (direction, body) = extract_direction(body, app.span)?;
    let (on_appear, on_appear_async, body) = extract_on_appear(body, app.span, "app")?;
    let (on_disappear, body) = extract_on_disappear(body, app.span, "app")?;
    let (on_active, body) = extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Active)?;
    let (on_inactive, body) =
        extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Inactive)?;
    let (on_background, body) =
        extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Background)?;
    let components = retain_reachable(components, &body, &screens);
    let plugins = app
        .plugins
        .iter()
        .map(|plugin| nexa_ir::Plugin {
            namespace: plugin.namespace.clone(),
            idl_path: plugin.path.clone(),
        })
        .collect();
    let mut module = Module {
        app_name: app.name,
        plugins,
        enums: enum_declarations,
        structs: struct_declarations,
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
        on_active,
        on_inactive,
        on_background,
    };
    crate::optimize::optimize(&mut module);
    Ok((module, warnings))
}

fn lower_enum_declarations(
    declarations: &[ast::EnumDecl],
) -> Result<Vec<nexa_ir::EnumDecl>, CompileError> {
    let mut names = std::collections::HashSet::with_capacity(declarations.len());
    let mut lowered = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        if matches!(
            declaration.name.as_str(),
            "String"
                | "Bool"
                | "Int8"
                | "Int16"
                | "Int32"
                | "Int64"
                | "UInt8"
                | "UInt16"
                | "UInt32"
                | "UInt64"
                | "Float32"
                | "Float64"
                | "Array"
                | "Set"
                | "Map"
                | "Pair"
                | "Triple"
                | "Permission"
                | "PermissionStatus"
                | "Theme"
                | "Layout"
        ) {
            return Err(CompileError::new(
                declaration.span,
                format!("enum name `{}` is reserved", declaration.name),
            ));
        }
        if !names.insert(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!("enum `{}` is declared more than once", declaration.name),
            ));
        }
        lowered.push(nexa_ir::EnumDecl {
            name: declaration.name.clone(),
            cases: declaration
                .cases
                .iter()
                .map(|case| case.name.clone())
                .collect(),
        });
    }
    Ok(lowered)
}

fn lower_struct_declarations(
    declarations: &[ast::StructDecl],
) -> Result<(Vec<nexa_ir::StructDecl>, StructTypes), CompileError> {
    let mut raw = HashMap::with_capacity(declarations.len());
    let mut names = std::collections::HashSet::with_capacity(declarations.len());
    for declaration in declarations {
        let fields = (|| {
            if matches!(
                declaration.name.as_str(),
                "String"
                    | "Bool"
                    | "Int8"
                    | "Int16"
                    | "Int32"
                    | "Int64"
                    | "UInt8"
                    | "UInt16"
                    | "UInt32"
                    | "UInt64"
                    | "Float32"
                    | "Float64"
                    | "Array"
                    | "Set"
                    | "Map"
                    | "Pair"
                    | "Triple"
                    | "Permission"
                    | "PermissionStatus"
                    | "Theme"
                    | "Layout"
            ) {
                return Err(CompileError::new(
                    declaration.span,
                    format!("struct name `{}` is reserved", declaration.name),
                ));
            }
            if !names.insert(&declaration.name) {
                return Err(CompileError::new(
                    declaration.span,
                    format!("struct `{}` is declared more than once", declaration.name),
                ));
            }
            declaration
                .fields
                .iter()
                .map(|field| Ok((field.name.clone(), parse_type(&field.ty)?)))
                .collect::<Result<Vec<_>, CompileError>>()
        })()
        .map_err(|error| in_source_file(error, declaration.source_file.as_deref()))?;
        raw.insert(declaration.name.clone(), fields);
    }

    let mut types = HashMap::with_capacity(declarations.len());
    for declaration in declarations {
        resolve_struct_definition(
            &declaration.name,
            &raw,
            &mut types,
            &mut Vec::new(),
            declaration.span,
        )
        .map_err(|error| in_source_file(error, declaration.source_file.as_deref()))?;
    }
    let lowered = declarations
        .iter()
        .map(|declaration| {
            let Type::Struct { fields, .. } = types
                .get(&declaration.name)
                .expect("resolved struct declaration")
                .clone()
            else {
                unreachable!("struct resolver always produces a struct type")
            };
            nexa_ir::StructDecl {
                name: declaration.name.clone(),
                fields: fields
                    .into_iter()
                    .map(|(name, ty)| nexa_ir::StructField { name, ty })
                    .collect(),
            }
        })
        .collect();
    Ok((lowered, types))
}

fn resolve_struct_definition(
    name: &str,
    raw: &HashMap<String, Vec<(String, Type)>>,
    cache: &mut StructTypes,
    active: &mut Vec<String>,
    span: nexa_diagnostics::Span,
) -> Result<Type, CompileError> {
    if let Some(ty) = cache.get(name) {
        return Ok(ty.clone());
    }
    if active.iter().any(|active_name| active_name == name) {
        return Err(CompileError::new(
            span,
            format!("recursive struct `{name}` is not supported in value types"),
        ));
    }
    let Some(raw_fields) = raw.get(name) else {
        return Err(CompileError::new(span, format!("unknown struct `{name}`")));
    };
    active.push(name.to_owned());
    let mut fields = Vec::with_capacity(raw_fields.len());
    for (field_name, field_type) in raw_fields {
        fields.push((
            field_name.clone(),
            resolve_struct_members(field_type, raw, cache, active, span)?,
        ));
    }
    active.pop();
    let ty = Type::Struct {
        name: name.to_owned(),
        fields,
    };
    cache.insert(name.to_owned(), ty.clone());
    Ok(ty)
}

fn resolve_struct_members(
    ty: &Type,
    raw: &HashMap<String, Vec<(String, Type)>>,
    cache: &mut StructTypes,
    active: &mut Vec<String>,
    span: nexa_diagnostics::Span,
) -> Result<Type, CompileError> {
    match ty {
        Type::Enum(name) if raw.contains_key(name) => {
            resolve_struct_definition(name, raw, cache, active, span)
        }
        Type::Optional(inner) => Ok(Type::Optional(Box::new(resolve_struct_members(
            inner, raw, cache, active, span,
        )?))),
        Type::Array(inner) => Ok(Type::Array(Box::new(resolve_struct_members(
            inner, raw, cache, active, span,
        )?))),
        Type::Set(inner) => Ok(Type::Set(Box::new(resolve_struct_members(
            inner, raw, cache, active, span,
        )?))),
        Type::Map(key, value) => Ok(Type::Map(
            Box::new(resolve_struct_members(key, raw, cache, active, span)?),
            Box::new(resolve_struct_members(value, raw, cache, active, span)?),
        )),
        Type::Pair(first, second) => Ok(Type::Pair(
            Box::new(resolve_struct_members(first, raw, cache, active, span)?),
            Box::new(resolve_struct_members(second, raw, cache, active, span)?),
        )),
        Type::Triple(first, second, third) => Ok(Type::Triple(
            Box::new(resolve_struct_members(first, raw, cache, active, span)?),
            Box::new(resolve_struct_members(second, raw, cache, active, span)?),
            Box::new(resolve_struct_members(third, raw, cache, active, span)?),
        )),
        _ => Ok(ty.clone()),
    }
}

fn in_source_file(error: CompileError, source_file: Option<&str>) -> CompileError {
    if let Some(file) = source_file {
        error.with_file(file)
    } else {
        error
    }
}

fn enum_symbols(declarations: &[nexa_ir::EnumDecl]) -> HashMap<String, (nexa_ir::Type, bool)> {
    let mut symbols = HashMap::new();
    for declaration in declarations {
        let ty = nexa_ir::Type::Enum(declaration.name.clone());
        symbols.insert(format!("__enum::{}", declaration.name), (ty.clone(), false));
        for case in &declaration.cases {
            symbols.insert(
                format!("{}.{}", declaration.name, case),
                (ty.clone(), false),
            );
        }
    }
    symbols
}

fn validate_declared_types(
    app: &ast::App,
    enum_names: &std::collections::HashSet<&str>,
    struct_types: &StructTypes,
) -> Result<(), CompileError> {
    for declaration in &app.structs {
        for field in &declaration.fields {
            let ty = resolve_struct_type(&parse_type(&field.ty)?, struct_types);
            validate_type_names(&ty, enum_names, field.ty.span())
                .map_err(|error| in_source_file(error, declaration.source_file.as_deref()))?;
        }
    }
    for declaration in &app.states {
        if let Some(ty) = &declaration.ty {
            validate_type_names(
                &resolve_struct_type(&parse_type(ty)?, struct_types),
                enum_names,
                ty.span(),
            )?;
        }
    }
    for declaration in &app.functions {
        for parameter in &declaration.parameters {
            validate_type_names(
                &resolve_struct_type(&parse_type(&parameter.ty)?, struct_types),
                enum_names,
                parameter.ty.span(),
            )?;
        }
        validate_type_names(
            &resolve_struct_type(&parse_type(&declaration.return_type)?, struct_types),
            enum_names,
            declaration.return_type.span(),
        )?;
        for statement in &declaration.body {
            if let ast::Stmt::Let { ty: Some(ty), .. } = statement {
                validate_type_names(
                    &resolve_struct_type(&parse_type(ty)?, struct_types),
                    enum_names,
                    ty.span(),
                )?;
            }
        }
    }
    for declaration in &app.components {
        for parameter in &declaration.parameters {
            validate_type_names(
                &resolve_struct_type(&parse_type(&parameter.ty)?, struct_types),
                enum_names,
                parameter.ty.span(),
            )?;
        }
        for state in &declaration.states {
            if let Some(ty) = &state.ty {
                validate_type_names(
                    &resolve_struct_type(&parse_type(ty)?, struct_types),
                    enum_names,
                    ty.span(),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_type_names(
    ty: &Type,
    enum_names: &std::collections::HashSet<&str>,
    span: nexa_diagnostics::Span,
) -> Result<(), CompileError> {
    match ty {
        Type::Enum(name) if !enum_names.contains(name.as_str()) && !is_builtin_enum_name(name) => {
            Err(CompileError::new(span, format!("unknown type `{name}`")))
        }
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) => {
            validate_type_names(inner, enum_names, span)
        }
        Type::Map(key, value) | Type::Pair(key, value) => {
            validate_type_names(key, enum_names, span)?;
            validate_type_names(value, enum_names, span)
        }
        Type::Triple(first, second, third) => {
            validate_type_names(first, enum_names, span)?;
            validate_type_names(second, enum_names, span)?;
            validate_type_names(third, enum_names, span)
        }
        Type::Struct { fields, .. } => fields
            .iter()
            .try_for_each(|(_, field)| validate_type_names(field, enum_names, span)),
        Type::String
        | Type::Bool
        | Type::Numeric(_)
        | Type::Enum(_)
        | Type::Plugin { .. }
        | Type::NetworkResponse => Ok(()),
    }
}

fn is_builtin_enum_name(name: &str) -> bool {
    matches!(name, "Permission" | "PermissionStatus")
}

fn lower_functions(
    declarations: Vec<ast::FunctionDecl>,
    signatures: &FunctionSignatures,
    structs: &StructTypes,
    enum_symbols: &HashMap<String, (Type, bool)>,
) -> Result<Vec<Function>, CompileError> {
    declarations
        .into_iter()
        .map(|declaration| {
            let signature = signatures
                .get(&declaration.name)
                .expect("collected signature");
            let symbols = signature
                .parameters
                .iter()
                .map(|(name, ty)| (name.clone(), (ty.clone(), false)))
                .collect::<HashMap<_, _>>();
            let mut symbols = symbols;
            symbols.extend(enum_symbols.iter().map(|(name, value)| (name.clone(), value.clone())));
            let mut locals = Vec::new();
            let mut return_value = None;
            for statement in declaration.body {
                match statement {
                    ast::Stmt::Let {
                        name,
                        ty,
                        initial,
                        span,
                    } => {
                        if return_value.is_some() {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "function `{}` cannot declare a local after `return`",
                                    declaration.name
                                ),
                            ));
                        }
                        if symbols.contains_key(&name) {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "local constant `{name}` is already declared in function `{}`",
                                    declaration.name
                                ),
                            ));
                        }
                        let local_type = resolve_value_type(
                            &name,
                            ty.as_ref(),
                            &initial,
                            &symbols,
                            signatures,
                            structs,
                        )?;
                        let lowered = lower_expr(
                            &initial,
                            Some(&local_type),
                            &symbols,
                            signatures,
                            signature.is_async,
                        )?;
                        symbols.insert(name.clone(), (local_type.clone(), false));
                        locals.push(FunctionLocal {
                            name,
                            ty: local_type,
                            initial: lowered,
                        });
                    }
                    ast::Stmt::Return { value, span: _ } => {
                        if return_value.replace(value).is_some() {
                            return Err(CompileError::new(
                                declaration.span,
                                format!(
                                    "function `{}` must contain exactly one `return` statement",
                                    declaration.name
                                ),
                            ));
                        }
                    }
                    ast::Stmt::Assign { span, .. }
                    | ast::Stmt::CollectionMutation { span, .. }
                    | ast::Stmt::If { span, .. }
                    | ast::Stmt::For { span, .. }
                    | ast::Stmt::ForMap { span, .. }
                    | ast::Stmt::While { span, .. }
                    | ast::Stmt::Break { span }
                    | ast::Stmt::Continue { span } => {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "function `{}` supports only `let` declarations and one `return` statement",
                                declaration.name
                            ),
                        ));
                    }
                }
            }
            let value = return_value.ok_or_else(|| {
                CompileError::new(
                    declaration.span,
                    format!(
                        "function `{}` must contain exactly one `return` statement",
                        declaration.name
                    ),
                )
            })?;
            let body = lower_expr(
                &value,
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
                locals,
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
            ast::Node::OnActive { .. } => {}
            ast::Node::OnInactive { .. } => {}
            ast::Node::OnBackground { .. } => {}
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
    scope: &str,
) -> Result<(Option<StatusBarConfig>, Vec<Node>), CompileError> {
    let mut config = None;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            Node::StatusBar { config: value } => {
                if config.replace(value).is_some() {
                    return Err(CompileError::new(
                        span,
                        format!("a {scope} can declare only one top-level StatusBar"),
                    ));
                }
            }
            node if contains_status_bar(&node) => {
                return Err(CompileError::new(
                    span,
                    format!("StatusBar is only allowed at the {scope} body's top level"),
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
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. } => children.iter().any(contains_status_bar),
        Node::FastList {
            children,
            sticky_header,
            section_header,
            ..
        } => {
            children.iter().any(contains_status_bar)
                || sticky_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_status_bar))
                || section_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_status_bar))
        }
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
        Node::When {
            cases, else_body, ..
        } => {
            cases
                .iter()
                .any(|case| case.body.iter().any(contains_status_bar))
                || else_body.iter().any(contains_status_bar)
        }
        Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::Content
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => false,
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
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. } => children.iter().any(contains_direction),
        Node::FastList {
            children,
            sticky_header,
            section_header,
            ..
        } => {
            children.iter().any(contains_direction)
                || sticky_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_direction))
                || section_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_direction))
        }
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
        Node::When {
            cases, else_body, ..
        } => {
            cases
                .iter()
                .any(|case| case.body.iter().any(contains_direction))
                || else_body.iter().any(contains_direction)
        }
        Node::StatusBar { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::Content
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => false,
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
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. } => children.iter().any(contains_on_appear),
        Node::FastList {
            children,
            sticky_header,
            section_header,
            ..
        } => {
            children.iter().any(contains_on_appear)
                || sticky_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_on_appear))
                || section_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_on_appear))
        }
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
        Node::When {
            cases, else_body, ..
        } => {
            cases
                .iter()
                .any(|case| case.body.iter().any(contains_on_appear))
                || else_body.iter().any(contains_on_appear)
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
        | Node::Content
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => false,
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

#[derive(Clone, Copy)]
enum LifecycleEvent {
    Active,
    Inactive,
    Background,
}

fn extract_lifecycle_event(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
    scope: &str,
    event: LifecycleEvent,
) -> Result<(Option<Vec<Action>>, Vec<Node>), CompileError> {
    let mut actions = None;
    let mut body = Vec::with_capacity(nodes.len());
    for node in nodes {
        let event_actions = match (event, &node) {
            (LifecycleEvent::Active, Node::OnActive { actions })
            | (LifecycleEvent::Inactive, Node::OnInactive { actions })
            | (LifecycleEvent::Background, Node::OnBackground { actions }) => Some(actions.clone()),
            _ => None,
        };
        if let Some(value) = event_actions {
            if actions.replace(value).is_some() {
                return Err(CompileError::new(
                    span,
                    format!(
                        "a {scope} can declare only one top-level {} lifecycle callback",
                        lifecycle_event_name(event)
                    ),
                ));
            }
        } else if contains_lifecycle_event(&node, event) {
            return Err(CompileError::new(
                span,
                format!(
                    "{} lifecycle callbacks are only allowed at the {scope} body's top level",
                    lifecycle_event_name(event)
                ),
            ));
        } else {
            body.push(node);
        }
    }
    Ok((actions, body))
}

fn lifecycle_event_name(event: LifecycleEvent) -> &'static str {
    match event {
        LifecycleEvent::Active => "OnActive",
        LifecycleEvent::Inactive => "OnInactive",
        LifecycleEvent::Background => "OnBackground",
    }
}

pub(super) fn contains_on_active(node: &Node) -> bool {
    contains_lifecycle_event(node, LifecycleEvent::Active)
}

pub(super) fn contains_on_inactive(node: &Node) -> bool {
    contains_lifecycle_event(node, LifecycleEvent::Inactive)
}

pub(super) fn contains_on_background(node: &Node) -> bool {
    contains_lifecycle_event(node, LifecycleEvent::Background)
}

fn contains_lifecycle_event(node: &Node, event: LifecycleEvent) -> bool {
    match (event, node) {
        (LifecycleEvent::Active, Node::OnActive { .. })
        | (LifecycleEvent::Inactive, Node::OnInactive { .. })
        | (LifecycleEvent::Background, Node::OnBackground { .. }) => true,
        _ => match node {
            Node::Layout { children, .. }
            | Node::NavigationLink { children, .. }
            | Node::Link { children, .. }
            | Node::Accessibility { children, .. }
            | Node::KeyboardAware { children, .. }
            | Node::BottomSheet { children, .. }
            | Node::RefreshControl { children, .. }
            | Node::Pressable { children, .. } => children
                .iter()
                .any(|child| contains_lifecycle_event(child, event)),
            Node::FastList {
                children,
                sticky_header,
                section_header,
                ..
            } => {
                children
                    .iter()
                    .any(|child| contains_lifecycle_event(child, event))
                    || sticky_header.as_deref().is_some_and(|header| {
                        header
                            .iter()
                            .any(|child| contains_lifecycle_event(child, event))
                    })
                    || section_header.as_deref().is_some_and(|header| {
                        header
                            .iter()
                            .any(|child| contains_lifecycle_event(child, event))
                    })
            }
            Node::AppBottomBar { tabs, .. } => tabs.iter().any(|tab| {
                tab.children
                    .iter()
                    .any(|child| contains_lifecycle_event(child, event))
            }),
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                then_body
                    .iter()
                    .any(|child| contains_lifecycle_event(child, event))
                    || else_body.as_deref().is_some_and(|body| {
                        body.iter()
                            .any(|child| contains_lifecycle_event(child, event))
                    })
            }
            Node::When {
                cases, else_body, ..
            } => {
                cases.iter().any(|case| {
                    case.body
                        .iter()
                        .any(|child| contains_lifecycle_event(child, event))
                }) || else_body
                    .iter()
                    .any(|child| contains_lifecycle_event(child, event))
            }
            Node::StatusBar { .. }
            | Node::Direction { .. }
            | Node::OnAppear { .. }
            | Node::OnDisappear { .. }
            | Node::OnActive { .. }
            | Node::OnInactive { .. }
            | Node::OnBackground { .. }
            | Node::Text { .. }
            | Node::Button { .. }
            | Node::TextInput { .. }
            | Node::Switch { .. }
            | Node::Image { .. }
            | Node::NavigationStack { .. }
            | Node::ComponentCall { .. }
            | Node::Content => false,
        },
    }
}

pub(super) fn contains_on_disappear(node: &Node) -> bool {
    match node {
        Node::OnDisappear { .. } => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. } => children.iter().any(contains_on_disappear),
        Node::FastList {
            children,
            sticky_header,
            section_header,
            ..
        } => {
            children.iter().any(contains_on_disappear)
                || sticky_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_on_disappear))
                || section_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_on_disappear))
        }
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
        Node::When {
            cases, else_body, ..
        } => {
            cases
                .iter()
                .any(|case| case.body.iter().any(contains_on_disappear))
                || else_body.iter().any(contains_on_disappear)
        }
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. }
        | Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::ComponentCall { .. }
        | Node::Content => false,
    }
}
