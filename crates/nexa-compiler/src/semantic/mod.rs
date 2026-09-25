use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
};

use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::walk::any_node;
use nexa_ir::{
    Action, DirectionConfig, Function, FunctionLocal, FunctionParameter, Module, Node, Screen,
    ScreenId, State, StatusBarConfig, Type,
};
use nexa_syntax::ast;

use self::{
    components::{ScreenSignature, ScreenSignatures, contains_content, lower_nodes},
    custom_components::{lower_components, retain_reachable},
    expressions::{
        FunctionSignature, FunctionSignatures, StructTypes, collect_function_signatures,
        collect_plugin_components, collect_plugin_signatures, lower_expr, parse_type,
        record_native_alias, references_state, resolve_declaration_type, resolve_struct_type,
        resolve_value_type,
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
                receiver: None,
                is_constructor: true,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
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
    let screen_signatures = collect_screen_signatures(&app.screens, &struct_types)?;
    if !app.screens.is_empty() && target != Target::All && !has_navigation_root(&app.body, target) {
        return Err(CompileError::new(
            app.span,
            "apps with screen declarations must have one top-level `NavigationStack(root: ScreenName)` in `body`",
        ));
    }

    let mut native_component_signatures = custom_components::ComponentSignatures::new();
    for native in collect_plugin_components(&app.plugins)? {
        let qualified_name = format!("{}.{}", native.namespace, native.name);
        native_component_signatures.insert(
            qualified_name,
            custom_components::ComponentSignature {
                parameters: native.parameters,
                defaults: native.defaults,
                has_content_slot: native.has_content_slot,
                events: native
                    .events
                    .into_iter()
                    .map(|event| custom_components::ComponentEventSignature {
                        property: event.property,
                        parameters: event.parameters,
                    })
                    .collect(),
                native: true,
            },
        );
    }
    let (components, component_signatures) = lower_components(
        std::mem::take(&mut app.components),
        &screen_signatures,
        &themes,
        &function_signatures,
        &struct_types,
        &enum_symbols,
        &native_component_signatures,
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
    let mut native_aliases = HashMap::new();
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
        record_native_alias(&declaration.name, &ty, &initial, &mut native_aliases);
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
        let mut screen_native_aliases = native_aliases.clone();
        let screen_signature = screen_signatures
            .get(&screen.name)
            .expect("screen signature exists for every screen declaration");
        for parameter in &screen_signature.parameters {
            if screen_symbols.contains_key(&parameter.name)
                || all_state_names.contains(&parameter.name)
                || function_signatures.contains_key(&parameter.name)
            {
                return Err(CompileError::new(
                    screen.span,
                    format!(
                        "screen parameter `{}` conflicts with an app binding or function",
                        parameter.name
                    ),
                ));
            }
            screen_symbols.insert(parameter.name.clone(), (parameter.ty.clone(), false));
        }
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
            record_native_alias(&declaration.name, &ty, &initial, &mut screen_native_aliases);
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
            &screen_signatures,
            &themes,
            &component_signatures,
            &function_signatures,
            &screen_native_aliases,
            false,
            true,
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
        let screen_name = screen.name;
        screens.push(Screen {
            id: ScreenId(index),
            name: screen_name.clone(),
            parameters: screen_signatures
                .get(&screen_name)
                .map(|signature| signature.parameters.clone())
                .unwrap_or_default(),
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
        &screen_signatures,
        &themes,
        &component_signatures,
        &function_signatures,
        &native_aliases,
        true,
        false,
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
        .filter(|plugin| !plugin.pure)
        .map(|plugin| nexa_ir::Plugin {
            namespace: plugin.namespace.clone(),
            idl_path: plugin.path.clone(),
            ios_sources: plugin.ios_sources.clone(),
            android_sources: plugin.android_sources.clone(),
            cpp_sources: plugin.cpp_sources.clone(),
            cpp_headers: plugin.cpp_headers.clone(),
            cpp_standard: plugin.cpp_standard,
            ios_min_version: plugin.ios_min_version.clone(),
            android_min_sdk: plugin.android_min_sdk,
            ios_frameworks: plugin.ios_frameworks.clone(),
            ios_xcframeworks: plugin.ios_xcframeworks.clone(),
            ios_resources: plugin.ios_resources.clone(),
            ios_privacy_manifest: plugin.ios_privacy_manifest.clone(),
            swift_packages: plugin
                .swift_packages
                .iter()
                .map(|dependency| nexa_ir::SwiftPackage {
                    url: dependency.url.clone(),
                    from: dependency.from.clone(),
                    products: dependency.products.clone(),
                })
                .collect(),
            maven_dependencies: plugin.maven_dependencies.clone(),
            android_aars: plugin.android_aars.clone(),
            android_resources: plugin.android_resources.clone(),
            android_proguard_rules: plugin.android_proguard_rules.clone(),
            android_maven_repositories: plugin.android_maven_repositories.clone(),
            ios_usage_descriptions: plugin.ios_usage_descriptions.clone(),
            ios_entitlements: plugin
                .ios_entitlements
                .iter()
                .map(|(key, value)| {
                    let value = match value {
                        nexa_syntax::ast::PluginEntitlementValue::String(value) => {
                            nexa_ir::PluginEntitlementValue::String(value.clone())
                        }
                        nexa_syntax::ast::PluginEntitlementValue::Bool(value) => {
                            nexa_ir::PluginEntitlementValue::Bool(*value)
                        }
                        nexa_syntax::ast::PluginEntitlementValue::Strings(values) => {
                            nexa_ir::PluginEntitlementValue::Strings(values.clone())
                        }
                    };
                    (key.clone(), value)
                })
                .collect(),
            ios_linker_flags: plugin.ios_linker_flags.clone(),
            android_permissions: plugin.android_permissions.clone(),
        })
        .collect();
    let plugin_assets = app
        .plugins
        .iter()
        .filter_map(|plugin| {
            let assets_path = plugin.assets_path.clone()?;
            let plugin_root = Path::new(&plugin.path).parent()?;
            let is_reachable = !plugin.pure
                || components.iter().any(|component| {
                    component
                        .source_file
                        .as_deref()
                        .is_some_and(|source| Path::new(source).starts_with(plugin_root))
                });
            is_reachable.then_some(nexa_ir::PluginAsset {
                root: assets_path,
                package_root: Some(plugin_root.display().to_string()),
            })
        })
        .collect();
    let mut module = Module {
        app_name: app.name,
        plugins,
        plugin_assets,
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
    validate_module_callback_disposal(&module, &function_signatures)?;
    crate::optimize::optimize(&mut module);
    Ok((module, warnings))
}

fn validate_module_callback_disposal(
    module: &Module,
    functions: &FunctionSignatures,
) -> Result<(), CompileError> {
    let mut shared_aliases = HashMap::new();
    for state in module.states.iter().chain(
        module
            .screens
            .iter()
            .flat_map(|screen| screen.states.iter()),
    ) {
        expressions::record_native_alias(
            &state.name,
            &state.ty,
            &state.initial,
            &mut shared_aliases,
        );
    }
    let mut shared_identities = HashMap::new();
    let mut shared_labels = HashMap::new();
    let mut shared_owners = HashMap::new();
    for state in module.states.iter().chain(
        module
            .screens
            .iter()
            .flat_map(|screen| screen.states.iter()),
    ) {
        if is_disposable_native_type(&state.ty, functions) {
            let identity = shared_aliases
                .get(&state.name)
                .cloned()
                .unwrap_or_else(|| state.name.clone());
            shared_identities.insert(state.name.clone(), identity.clone());
            shared_labels.entry(identity.clone()).or_insert(identity);
        }
    }
    for state in &module.states {
        if is_disposable_native_type(&state.ty, functions) {
            let identity = shared_identities
                .get(&state.name)
                .cloned()
                .unwrap_or_else(|| state.name.clone());
            shared_owners
                .entry(identity)
                .or_insert_with(|| "app".to_owned());
        }
    }
    for screen in &module.screens {
        for state in &screen.states {
            if is_disposable_native_type(&state.ty, functions) {
                let identity = shared_identities
                    .get(&state.name)
                    .cloned()
                    .unwrap_or_else(|| state.name.clone());
                shared_owners
                    .entry(identity)
                    .or_insert_with(|| format!("screen:{}", screen.name));
            }
        }
    }

    let mut shared_accesses = Vec::new();
    let app_scope = "app";
    push_node_callback_accesses(
        &mut shared_accesses,
        &module.body,
        &shared_identities,
        functions,
        app_scope,
    );
    for (actions, kind) in [
        (module.on_appear.as_deref(), CallbackKind::Lifecycle),
        (module.on_disappear.as_deref(), CallbackKind::OnDisappear),
        (module.on_active.as_deref(), CallbackKind::Lifecycle),
        (module.on_inactive.as_deref(), CallbackKind::Lifecycle),
        (module.on_background.as_deref(), CallbackKind::Lifecycle),
    ] {
        if let Some(actions) = actions {
            push_callback_accesses(
                &mut shared_accesses,
                actions,
                &shared_identities,
                functions,
                kind,
                app_scope,
            );
        }
    }
    for screen in &module.screens {
        let scope = format!("screen:{}", screen.name);
        push_node_callback_accesses(
            &mut shared_accesses,
            &screen.body,
            &shared_identities,
            functions,
            &scope,
        );
        for (actions, kind) in [
            (screen.on_appear.as_deref(), CallbackKind::Lifecycle),
            (screen.on_disappear.as_deref(), CallbackKind::OnDisappear),
        ] {
            if let Some(actions) = actions {
                push_callback_accesses(
                    &mut shared_accesses,
                    actions,
                    &shared_identities,
                    functions,
                    kind,
                    &scope,
                );
            }
        }
    }
    validate_callback_disposal_accesses(&shared_accesses, &shared_labels, &shared_owners)?;

    for screen in &module.screens {
        let mut identities = shared_identities.clone();
        let mut labels = shared_labels.clone();
        for parameter in &screen.parameters {
            if is_disposable_native_type(&parameter.ty, functions) {
                let identity = format!("@screen:{}:{}", screen.name, parameter.name);
                identities.insert(parameter.name.clone(), identity.clone());
                labels.insert(identity, parameter.name.clone());
            }
        }
        for state in &screen.states {
            if is_disposable_native_type(&state.ty, functions) {
                let root = shared_aliases
                    .get(&state.name)
                    .cloned()
                    .unwrap_or_else(|| state.name.clone());
                let identity = identities.get(&root).cloned().unwrap_or(root.clone());
                identities.insert(state.name.clone(), identity.clone());
                labels.entry(identity).or_insert(root);
            }
        }
        let mut accesses = Vec::new();
        let scope = format!("screen:{}", screen.name);
        push_node_callback_accesses(&mut accesses, &screen.body, &identities, functions, &scope);
        for (actions, kind) in [
            (screen.on_appear.as_deref(), CallbackKind::Lifecycle),
            (screen.on_disappear.as_deref(), CallbackKind::OnDisappear),
        ] {
            if let Some(actions) = actions {
                push_callback_accesses(
                    &mut accesses,
                    actions,
                    &identities,
                    functions,
                    kind,
                    &scope,
                );
            }
        }
        validate_callback_disposal_accesses(&accesses, &labels, &shared_owners)?;
    }

    for component in &module.components {
        let mut aliases = HashMap::new();
        for state in &component.states {
            expressions::record_native_alias(&state.name, &state.ty, &state.initial, &mut aliases);
        }
        let mut identities = HashMap::new();
        let mut labels = HashMap::new();
        for parameter in &component.parameters {
            if is_disposable_native_type(&parameter.ty, functions) {
                let identity = format!("@component:{}:param:{}", component.name, parameter.name);
                identities.insert(parameter.name.clone(), identity.clone());
                labels.insert(identity, parameter.name.clone());
            }
        }
        for state in &component.states {
            if is_disposable_native_type(&state.ty, functions) {
                let root = aliases
                    .get(&state.name)
                    .cloned()
                    .unwrap_or_else(|| state.name.clone());
                let identity = identities
                    .get(&root)
                    .cloned()
                    .unwrap_or_else(|| format!("@component:{}:{root}", component.name));
                identities.insert(state.name.clone(), identity.clone());
                labels.entry(identity).or_insert(root);
            }
        }
        let mut accesses = Vec::new();
        let scope = format!("component:{}", component.name);
        push_node_callback_accesses(
            &mut accesses,
            &component.body,
            &identities,
            functions,
            &scope,
        );
        validate_callback_disposal_accesses(&accesses, &labels, &HashMap::new())?;
    }

    Ok(())
}

#[derive(Default)]
struct CallbackDisposalAccesses {
    kind: CallbackKind,
    scope: String,
    uses: BTreeSet<String>,
    disposals: BTreeSet<String>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CallbackKind {
    #[default]
    UiEvent,
    Lifecycle,
    OnDisappear,
}

fn validate_callback_disposal_accesses(
    accesses: &[CallbackDisposalAccesses],
    labels: &HashMap<String, String>,
    owners: &HashMap<String, String>,
) -> Result<(), CompileError> {
    let mut uses_by_identity: std::collections::BTreeMap<&str, Vec<(usize, &str)>> =
        std::collections::BTreeMap::new();
    let mut disposals_by_identity: std::collections::BTreeMap<
        &str,
        Vec<(usize, CallbackKind, &str)>,
    > = std::collections::BTreeMap::new();
    for (callback_index, access) in accesses.iter().enumerate() {
        for identity in &access.uses {
            uses_by_identity
                .entry(identity)
                .or_default()
                .push((callback_index, &access.scope));
        }
        for identity in &access.disposals {
            disposals_by_identity.entry(identity).or_default().push((
                callback_index,
                access.kind,
                &access.scope,
            ));
        }
    }

    for (identity, disposals) in disposals_by_identity {
        if disposals.len() > 1 {
            return Err(cross_callback_disposal_error(
                identity,
                labels,
                "may be disposed in multiple callbacks",
            ));
        }
        let (dispose_callback, kind, scope) = disposals[0];
        if kind == CallbackKind::OnDisappear
            && owners
                .get(identity)
                .is_some_and(|owner_scope| owner_scope != scope)
        {
            return Err(cross_callback_disposal_error(
                identity,
                labels,
                "can only be disposed by its owning app or screen `OnDisappear`",
            ));
        }
        if kind != CallbackKind::OnDisappear
            && uses_by_identity.get(identity).is_some_and(|uses| {
                uses.iter()
                    .any(|(use_callback, _)| *use_callback != dispose_callback)
            })
        {
            return Err(cross_callback_disposal_error(
                identity,
                labels,
                "is disposed in one callback and used in another",
            ));
        }
    }
    Ok(())
}

fn push_node_callback_accesses(
    accesses: &mut Vec<CallbackDisposalAccesses>,
    nodes: &[Node],
    identities: &HashMap<String, String>,
    functions: &FunctionSignatures,
    scope: &str,
) {
    push_component_lifetime_accesses(accesses, nodes, identities, functions, scope);
    nexa_ir::walk::walk_callback_actions(nodes, &mut |actions| {
        push_callback_accesses(
            accesses,
            actions,
            identities,
            functions,
            CallbackKind::UiEvent,
            scope,
        );
    });
}

fn push_component_lifetime_accesses(
    accesses: &mut Vec<CallbackDisposalAccesses>,
    nodes: &[Node],
    identities: &HashMap<String, String>,
    functions: &FunctionSignatures,
    scope: &str,
) {
    let mut uses = BTreeSet::new();
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| {
            let arguments = match node {
                Node::ComponentCall { arguments, .. }
                | Node::NativeComponentCall { arguments, .. } => arguments,
                _ => return,
            };
            for (_, argument) in arguments {
                if let nexa_ir::Expr::State(name, ty) = argument
                    && is_disposable_native_type(ty, functions)
                {
                    uses.insert(callback_identity(name, accesses.len(), identities));
                }
            }
        },
        &mut |_| {},
    );
    if !uses.is_empty() {
        // A rendered component can keep its native object argument alive and
        // use it between callbacks. Treat that view lifetime as an access so
        // UI-event disposal cannot invalidate an object still in the tree.
        accesses.push(CallbackDisposalAccesses {
            scope: scope.to_owned(),
            uses,
            ..CallbackDisposalAccesses::default()
        });
    }
}

fn push_callback_accesses(
    accesses: &mut Vec<CallbackDisposalAccesses>,
    actions: &[Action],
    identities: &HashMap<String, String>,
    functions: &FunctionSignatures,
    kind: CallbackKind,
    scope: &str,
) {
    let callback_index = accesses.len();
    let mut access = CallbackDisposalAccesses {
        kind,
        scope: scope.to_owned(),
        ..CallbackDisposalAccesses::default()
    };
    nexa_ir::walk::walk_callback_expressions(actions, &mut |expression| match expression {
        nexa_ir::Expr::State(name, ty) if is_disposable_native_type(ty, functions) => {
            access
                .uses
                .insert(callback_identity(name, callback_index, identities));
        }
        nexa_ir::Expr::NativeCall {
            receiver: Some(receiver),
            name,
            ..
        } if name == "dispose" => {
            if let nexa_ir::Expr::State(binding, ty) = receiver.as_ref()
                && is_disposable_native_type(ty, functions)
            {
                access
                    .disposals
                    .insert(callback_identity(binding, callback_index, identities));
            }
        }
        _ => {}
    });
    accesses.push(access);
}

fn callback_identity(
    name: &str,
    callback_index: usize,
    identities: &HashMap<String, String>,
) -> String {
    identities
        .get(name)
        .cloned()
        .unwrap_or_else(|| format!("@callback:{callback_index}:{name}"))
}

fn cross_callback_disposal_error(
    identity: &str,
    labels: &HashMap<String, String>,
    detail: &str,
) -> CompileError {
    let name = labels.get(identity).map_or(identity, String::as_str);
    CompileError::new(
        nexa_diagnostics::Span::default(),
        format!(
            "native class instance `{name}` {detail}; callback execution order cannot be proven"
        ),
    )
}

fn is_disposable_native_type(ty: &Type, functions: &FunctionSignatures) -> bool {
    let Type::Plugin { name, .. } = ty else {
        return false;
    };
    functions
        .get(&format!("{name}.dispose"))
        .is_some_and(|signature| {
            signature.receiver.as_ref() == Some(ty)
                && signature.parameters.is_empty()
                && signature.return_type == Type::Void
                && !signature.is_async
                && !signature.is_throwing
        })
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
    let mut native_class_types = HashSet::new();
    for plugin in &app.plugins {
        let Some(idl) = plugin.idl.as_ref() else {
            continue;
        };
        for interface in &idl.interfaces {
            if interface.kind == nexa_plugin_idl::InterfaceKind::NativeClass {
                native_class_types.insert(format!("{}.{}", plugin.namespace, interface.name));
                if interface.name == plugin.namespace {
                    native_class_types.insert(interface.name.clone());
                }
            }
        }
    }

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
            validate_component_type_names(
                &resolve_struct_type(&parse_type(&parameter.ty)?, struct_types),
                enum_names,
                &native_class_types,
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

fn validate_component_type_names(
    ty: &Type,
    enum_names: &std::collections::HashSet<&str>,
    native_class_types: &HashSet<String>,
    span: nexa_diagnostics::Span,
) -> Result<(), CompileError> {
    match ty {
        Type::Enum(name) if native_class_types.contains(name) => Ok(()),
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) => {
            validate_component_type_names(inner, enum_names, native_class_types, span)
        }
        Type::Map(key, value) | Type::Pair(key, value) => {
            validate_component_type_names(key, enum_names, native_class_types, span)?;
            validate_component_type_names(value, enum_names, native_class_types, span)
        }
        Type::Triple(first, second, third) => {
            validate_component_type_names(first, enum_names, native_class_types, span)?;
            validate_component_type_names(second, enum_names, native_class_types, span)?;
            validate_component_type_names(third, enum_names, native_class_types, span)
        }
        _ => validate_type_names(ty, enum_names, span),
    }
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
        Type::Map(key, value) | Type::Pair(key, value) | Type::Result(key, value) => {
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
        Type::Void
        | Type::String
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
                    ast::Stmt::Expression { span, .. } => {
                        return Err(CompileError::new(
                            span,
                            "expression statements are only allowed in event handlers",
                        ));
                    }
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
                    | ast::Stmt::NativePropertyAssign { span, .. }
                    | ast::Stmt::NativeEventSubscribe { span, .. }
                    | ast::Stmt::CollectionMutation { span, .. }
                    | ast::Stmt::If { span, .. }
                    | ast::Stmt::For { span, .. }
                    | ast::Stmt::ForMap { span, .. }
                    | ast::Stmt::While { span, .. }
                    | ast::Stmt::TryCatch { span, .. }
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
    matches!(
        active.as_slice(),
        [ast::Node::ComponentInvocation(inv)] if inv.name == "NavigationStack"
    )
}

fn collect_screen_signatures(
    screens: &[ast::ScreenDecl],
    structs: &StructTypes,
) -> Result<ScreenSignatures, CompileError> {
    let mut signatures = ScreenSignatures::with_capacity(screens.len());
    for (index, screen) in screens.iter().enumerate() {
        let parameters = screen
            .parameters
            .iter()
            .map(|parameter| {
                let ty = resolve_struct_type(&parse_type(&parameter.ty)?, structs);
                if !is_route_parameter_type(&ty) {
                    return Err(CompileError::new(
                        parameter.span,
                        "screen route parameters must be String, Bool, or a numeric type",
                    ));
                }
                Ok(nexa_ir::FunctionParameter {
                    name: parameter.name.clone(),
                    ty,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        signatures.insert(
            screen.name.clone(),
            ScreenSignature {
                id: ScreenId(index),
                parameters,
            },
        );
    }
    Ok(signatures)
}

fn is_route_parameter_type(ty: &Type) -> bool {
    matches!(ty, Type::String | Type::Bool | Type::Numeric(_))
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
            ast::Node::ComponentInvocation(inv)
                if matches!(
                    inv.name.as_str(),
                    "StatusBar"
                        | "Direction"
                        | "OnAppear"
                        | "OnDisappear"
                        | "OnActive"
                        | "OnInactive"
                        | "OnBackground"
                ) => {}
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
    any_node(std::slice::from_ref(node), |node| {
        matches!(node, Node::StatusBar { .. })
    })
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
    any_node(std::slice::from_ref(node), |node| {
        matches!(node, Node::Direction { .. })
    })
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
    any_node(std::slice::from_ref(node), |node| {
        matches!(node, Node::OnAppear { .. })
    })
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
    any_node(std::slice::from_ref(node), |node| {
        matches!(
            (event, node),
            (LifecycleEvent::Active, Node::OnActive { .. })
                | (LifecycleEvent::Inactive, Node::OnInactive { .. })
                | (LifecycleEvent::Background, Node::OnBackground { .. })
        )
    })
}

pub(super) fn contains_on_disappear(node: &Node) -> bool {
    any_node(std::slice::from_ref(node), |node| {
        matches!(node, Node::OnDisappear { .. })
    })
}

#[cfg(test)]
mod callback_disposal_tests {
    use std::collections::HashMap;

    use nexa_ir::{
        Action, Expr, Module, NativeComponentEventHandler, Node, Screen, ScreenId, State, Type,
    };

    use super::{FunctionSignature, FunctionSignatures, validate_module_callback_disposal};

    fn player_type() -> Type {
        Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        }
    }

    fn player_call(name: &str, receiver: &str) -> Expr {
        Expr::NativeCall {
            receiver: Some(Box::new(Expr::State(receiver.to_owned(), player_type()))),
            namespace: "Video".to_owned(),
            name: name.to_owned(),
            arguments: Vec::new(),
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
        }
    }

    fn fixture(states: Vec<State>, callback_use: &str) -> (Module, FunctionSignatures) {
        let module = Module {
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states,
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Button {
                label: Expr::String("Dispose".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                actions: vec![Action::Expression(player_call("dispose", "player"))],
            }],
            status_bar: None,
            direction: None,
            on_appear: Some(vec![Action::Expression(player_call("play", callback_use))]),
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        (module, disposable_functions())
    }

    fn state(name: &str, initial: Expr) -> State {
        State {
            name: name.to_owned(),
            ty: player_type(),
            initial,
            mutable: false,
        }
    }

    fn screen(
        id: usize,
        name: &str,
        states: Vec<State>,
        body: Vec<Node>,
        on_disappear: Option<Vec<Action>>,
    ) -> Screen {
        Screen {
            id: ScreenId(id),
            name: name.to_owned(),
            parameters: Vec::new(),
            states,
            body,
            status_bar: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear,
        }
    }

    fn constructor() -> Expr {
        Expr::Call {
            name: "Video.VideoPlayer".to_owned(),
            arguments: Vec::new(),
            return_type: player_type(),
            is_async: false,
            is_constructor: true,
        }
    }

    fn disposable_functions() -> FunctionSignatures {
        HashMap::from([(
            "VideoPlayer.dispose".to_owned(),
            FunctionSignature {
                parameters: Vec::new(),
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
                receiver: Some(player_type()),
                is_constructor: false,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        )])
    }

    #[test]
    fn rejects_dispose_in_one_ui_callback_and_use_in_another() {
        let (module, functions) = fixture(vec![state("player", constructor())], "player");
        let error = validate_module_callback_disposal(&module, &functions)
            .expect_err("callback ordering cannot prove player remains alive");

        assert!(
            error
                .to_string()
                .contains("is disposed in one callback and used in another")
        );
        assert!(error.to_string().contains("player"));
    }

    #[test]
    fn callback_lifetime_analysis_resolves_immutable_aliases() {
        let (module, functions) = fixture(
            vec![
                state("player", constructor()),
                state(
                    "playerAlias",
                    Expr::State("player".to_owned(), player_type()),
                ),
            ],
            "playerAlias",
        );
        let error = validate_module_callback_disposal(&module, &functions)
            .expect_err("alias accesses refer to the disposed instance");

        assert!(error.to_string().contains("instance `player`"));
    }

    #[test]
    fn independent_native_objects_can_be_used_across_callbacks() {
        let (module, functions) = fixture(
            vec![
                state("player", constructor()),
                state("other", constructor()),
            ],
            "other",
        );

        validate_module_callback_disposal(&module, &functions)
            .expect("disposal of one instance must not invalidate another");
    }

    #[test]
    fn native_component_event_actions_are_included_in_callback_analysis() {
        let mut module = Module {
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: vec![state("player", constructor())],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::NativeComponentCall {
                namespace: "Video".to_owned(),
                name: "VideoView".to_owned(),
                arguments: Vec::new(),
                children: None,
                event_handlers: vec![NativeComponentEventHandler {
                    property: "onTapped".to_owned(),
                    parameters: Vec::new(),
                    actions: vec![Action::Expression(player_call("dispose", "player"))],
                }],
            }],
            status_bar: None,
            direction: None,
            on_appear: Some(vec![Action::Expression(player_call("play", "player"))]),
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };

        let functions = disposable_functions();
        let error = validate_module_callback_disposal(&module, &functions)
            .expect_err("component events and lifecycle callbacks share the instance");
        assert!(error.to_string().contains("player"));

        module.on_appear = None;
        validate_module_callback_disposal(&module, &functions)
            .expect("a single callback may dispose its own instance");
    }

    #[test]
    fn native_component_arguments_keep_instances_alive_until_view_disappearance() {
        let mut module = Module {
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: vec![state("player", constructor())],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::Button {
                    label: Expr::String("Dispose".to_owned()),
                    icon: None,
                    loading: None,
                    disabled: None,
                    actions: vec![Action::Expression(player_call("dispose", "player"))],
                },
                Node::ComponentCall {
                    name: "PlayerSurface".to_owned(),
                    arguments: vec![(
                        "player".to_owned(),
                        Expr::State("player".to_owned(), player_type()),
                    )],
                    children: None,
                },
                Node::NativeComponentCall {
                    namespace: "Video".to_owned(),
                    name: "VideoView".to_owned(),
                    arguments: vec![(
                        "player".to_owned(),
                        Expr::State("player".to_owned(), player_type()),
                    )],
                    children: None,
                    event_handlers: Vec::new(),
                },
            ],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let functions = disposable_functions();
        let error = validate_module_callback_disposal(&module, &functions).expect_err(
            "a rendered native component may still use its player after the button callback",
        );
        assert!(
            error
                .to_string()
                .contains("is disposed in one callback and used in another")
        );

        module.body.remove(0);
        module.on_disappear = Some(vec![Action::Expression(player_call("dispose", "player"))]);
        validate_module_callback_disposal(&module, &functions)
            .expect("view disappearance is the terminal lifetime boundary for the component");
    }

    #[test]
    fn screen_disappearance_only_disposes_objects_owned_by_that_screen() {
        let (mut module, functions) =
            fixture(vec![state("sharedPlayer", constructor())], "sharedPlayer");
        module.body.clear();
        module.on_appear = None;
        module.screens = vec![
            screen(
                0,
                "Library",
                Vec::new(),
                Vec::new(),
                Some(vec![Action::Expression(player_call(
                    "dispose",
                    "sharedPlayer",
                ))]),
            ),
            screen(
                1,
                "Details",
                Vec::new(),
                vec![Node::Button {
                    label: Expr::String("Play".to_owned()),
                    icon: None,
                    loading: None,
                    disabled: None,
                    actions: vec![Action::Expression(player_call("play", "sharedPlayer"))],
                }],
                None,
            ),
        ];
        let error = validate_module_callback_disposal(&module, &functions)
            .expect_err("one screen cannot release an app-owned player used by another route");
        assert!(
            error
                .to_string()
                .contains("owning app or screen `OnDisappear`")
        );

        module.screens[0].on_disappear = None;
        validate_module_callback_disposal(&module, &functions).expect(
            "app-owned resources may be shared by successive routes and outlive each screen",
        );

        module.states.clear();
        module.screens = vec![screen(
            0,
            "PlayerScreen",
            vec![state("screenPlayer", constructor())],
            vec![Node::Button {
                label: Expr::String("Play".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                actions: vec![Action::Expression(player_call("play", "screenPlayer"))],
            }],
            Some(vec![Action::Expression(player_call(
                "dispose",
                "screenPlayer",
            ))]),
        )];
        validate_module_callback_disposal(&module, &functions)
            .expect("a screen may release its own native object when it disappears");
    }
}
