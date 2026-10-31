use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
    sync::Arc,
    time::Instant,
};

use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::walk::any_node;
use nexa_ir::{
    Action, BackgroundTask, DirectionConfig, Function, FunctionLocal, FunctionParameter, Module,
    Node, Screen, ScreenId, State, StatusBarConfig, Type, Widget, WidgetFamily,
};
use nexa_syntax::ast;

use self::{
    components::{
        contains_content, lower_actions_with_aliases, lower_nodes, validate_typed_error_recovery,
    },
    context::{ExprContext, ScreenSignature, ScreenSignatures, SemanticContext, TypeRegistries},
    custom_components::{lower_components, retain_reachable},
    expressions::{
        FunctionSignature, FunctionSignatures, StructTypes, collect_function_signatures,
        collect_plugin_components, collect_plugin_signatures, functions_with_error_handling,
        infer_expr_type, lower_expr, lower_for_iterable, lower_map_iterable, parse_return_type,
        parse_type, plugin_error_variant, record_native_alias, references_mutable_state,
        resolve_declaration_type, resolve_struct_type, resolve_value_type, type_name,
        validate_task_handle_state,
    },
    themes::lower_theme,
};
use crate::Target;

mod components;
mod context;
pub mod core_api;
mod custom_components;
mod expressions;
mod generics;
mod styles;
mod tests;
mod themes;
mod warnings;

pub(crate) use core_api::render_reference as render_core_api_reference;

fn with_screen_source(error: CompileError, source_file: Option<&str>) -> CompileError {
    match source_file {
        Some(file) => error.with_file(file),
        None => error,
    }
}

fn expression_uses_async(expression: &nexa_ir::Expr) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_expression(expression, &mut |nested| {
        found |= matches!(nested, nexa_ir::Expr::Await(_) | nexa_ir::Expr::TryAwait(_));
    });
    found
}

fn expression_uses_throwing_await(expression: &nexa_ir::Expr) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_expression(expression, &mut |nested| {
        found |= matches!(nested, nexa_ir::Expr::TryAwait(_));
    });
    found
}

fn expression_references_state(expression: &nexa_ir::Expr, name: &str) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_expression(expression, &mut |nested| {
        found |= matches!(nested, nexa_ir::Expr::State(state, _) if state == name)
            || matches!(nested, nexa_ir::Expr::AnimatedState(state, _) if state == name);
    });
    found
}

#[cfg(test)]
mod widget_capability_tests {
    use nexa_diagnostics::Span;
    use nexa_ir::{AccessibilityRole, Expr, Node, TextStyle};

    use super::validate_widget_body;

    #[test]
    fn widget_subset_accepts_static_content_primitives() {
        assert!(
            validate_widget_body(
                &[Node::Spacer {
                    width: None,
                    height: None
                }],
                Span::default()
            )
            .is_ok()
        );
    }

    #[test]
    fn widget_subset_accepts_cross_platform_text_emphasis() {
        let emphasized_text = Node::Text {
            value: Expr::String("Emphasis".to_owned()),
            style: TextStyle {
                strikethrough: true,
                italic: true,
                underline: true,
                font_design: Some(nexa_ir::TextFontDesign::Rounded),
                ..TextStyle::default()
            },
        };

        assert!(validate_widget_body(&[emphasized_text], Span::default()).is_ok());
    }

    #[test]
    fn widget_subset_accepts_accessibility_labels() {
        let labelled = Node::Accessibility {
            label: Expr::String("Task title".to_owned()),
            hint: None,
            value: None,
            role: AccessibilityRole::None,
            children: vec![Node::Text {
                value: Expr::String("Task title".to_owned()),
                style: TextStyle::default(),
            }],
        };
        assert!(validate_widget_body(&[labelled], Span::default()).is_ok());
    }

    #[test]
    fn widget_subset_rejects_platform_specific_accessibility_and_styles() {
        let role = Node::Accessibility {
            label: Expr::String("Task".to_owned()),
            hint: None,
            value: None,
            role: AccessibilityRole::Button,
            children: vec![Node::Spacer {
                width: None,
                height: None,
            }],
        };
        let error = validate_widget_body(&[role], Span::default())
            .expect_err("button role must have the same Glance semantics");
        assert!(error.message.contains("without a hint or role"));

        let styled_text = Node::Text {
            value: Expr::String("Task".to_owned()),
            style: TextStyle {
                padding: Some(4.0),
                ..TextStyle::default()
            },
        };
        let error = validate_widget_body(&[styled_text], Span::default())
            .expect_err("platform-specific text styling must be rejected by the compiler");
        assert!(error.message.contains("Text supports"));
    }

    #[test]
    fn widget_subset_rejects_stateful_or_unhandled_nodes() {
        let error = validate_widget_body(&[Node::Content], Span::default())
            .expect_err("unhandled content must not reach a backend");
        assert!(error.message.contains("app-only or stateful component"));

        let dynamic_link = Node::Link {
            url: Expr::Bool(true),
            children: Vec::new(),
        };
        let error = validate_widget_body(&[dynamic_link], Span::default())
            .expect_err("widget links require a static string URL");
        assert!(error.message.contains("static string URL"));
    }
}

/// Keep the widget IR inside the intersection that both native widget hosts
/// can render. Backends must not silently drop ordinary app controls.
fn validate_widget_body(nodes: &[Node], span: nexa_diagnostics::Span) -> Result<(), CompileError> {
    fn unsupported(name: &str, span: nexa_diagnostics::Span) -> CompileError {
        CompileError::new(
            span,
            format!(
                "widget content does not support `{name}`; use Text, Spacer, Divider, Column, Row, Stack, FastList, If, SystemIcon, accessibility labels, or a static Link"
            ),
        )
    }

    fn visit(nodes: &[Node], span: nexa_diagnostics::Span) -> Result<(), CompileError> {
        for node in nodes {
            match node {
                Node::Text { style, .. } => {
                    if style.padding.is_some()
                        || style.opacity.is_some()
                        || style.effects.has_modifiers()
                        || style.line_height.is_some()
                        || style.letter_spacing.is_some()
                        || style.selectable
                    {
                        return Err(CompileError::new(
                            span,
                            "widget Text supports font size, semantic font style, font design, weight, color, alignment, emphasis, and line limit",
                        ));
                    }
                }
                Node::Spacer { .. } | Node::Divider { .. } | Node::Shape { .. } => {}
                Node::SystemIcon { icon, .. } => {
                    if icon.android_widget_drawable().is_none() {
                        return Err(CompileError::new(
                            span,
                            format!("system icon `{icon:?}` has no Android widget equivalent"),
                        ));
                    }
                }
                Node::FontIcon { .. } => {
                    return Err(CompileError::new(
                        span,
                        "custom font glyph icons are not supported in Android widgets",
                    ));
                }
                Node::Layout {
                    style, children, ..
                } => {
                    if style.min_width.is_some()
                        || style.max_width.is_some()
                        || style.min_height.is_some()
                        || style.max_height.is_some()
                        || style.border_color.is_some()
                        || style.border_width.is_some()
                        || style.opacity.is_some()
                        || style.effects.has_modifiers()
                        || style.animation.is_some()
                    {
                        return Err(CompileError::new(
                            span,
                            "widget layout supports alignment, padding, width, height, background, and corner radius only",
                        ));
                    }
                    visit(children, span)?;
                }
                Node::Accessibility {
                    hint,
                    role,
                    children,
                    ..
                } => {
                    if hint.is_some() || !matches!(role, nexa_ir::AccessibilityRole::None) {
                        return Err(CompileError::new(
                            span,
                            "widget accessibility supports a label without a hint or role",
                        ));
                    }
                    visit(children, span)?;
                }
                Node::If {
                    then_body,
                    else_body,
                    transition,
                    ..
                } => {
                    if transition.is_some() {
                        return Err(CompileError::new(
                            span,
                            "widget `If` does not support animated transitions",
                        ));
                    }
                    visit(then_body, span)?;
                    if let Some(else_body) = else_body {
                        visit(else_body, span)?;
                    }
                }
                Node::Link { url, children } => {
                    if !matches!(url, nexa_ir::Expr::String(_)) {
                        return Err(CompileError::new(
                            span,
                            "widget `Link` requires a static string URL",
                        ));
                    }
                    visit(children, span)?;
                }
                Node::FastList { plan } => {
                    let common = match plan.as_ref() {
                        nexa_ir::ListPlan::Count { common, .. }
                        | nexa_ir::ListPlan::Items { common, .. } => common,
                        nexa_ir::ListPlan::Sections { .. } => {
                            return Err(CompileError::new(
                                span,
                                "widget `FastList` currently supports count and item sources, not sectioned lists",
                            ));
                        }
                    };
                    if common.axis != nexa_ir::ListAxis::Vertical {
                        return Err(CompileError::new(
                            span,
                            "widget `FastList` supports vertical layout only",
                        ));
                    }
                    if common.item_extent.is_some() || common.key.is_some() {
                        return Err(CompileError::new(
                            span,
                            "widget `FastList` does not support a custom row extent or row key yet",
                        ));
                    }
                    if common.reverse_layout
                        || common.page_snap
                        || common.scroll_position.is_some()
                        || common.on_end_reached.is_some()
                        || common.on_scroll.is_some()
                        || common.on_move.is_some()
                        || common.swipe_actions.is_some()
                        || common.sticky_header.is_some()
                        || common.refresh.is_some()
                    {
                        return Err(CompileError::new(
                            span,
                            "widget `FastList` supports only a vertical list with pure row content; scrolling, refresh, reorder, swipe actions, and sticky headers are unavailable",
                        ));
                    }
                    visit(&common.children, span)?;
                }
                Node::ComponentCall { name, .. } => {
                    return Err(unsupported(&format!("custom component `{name}`"), span));
                }
                Node::NativeComponentCall { name, .. } => {
                    return Err(unsupported(
                        &format!("native plugin component `{name}`"),
                        span,
                    ));
                }
                Node::NavigationStack { .. }
                | Node::NavigationSplitView { .. }
                | Node::NavigationLink { .. }
                | Node::NavigationBack { .. } => return Err(unsupported("navigation", span)),
                Node::OnAppear { .. }
                | Node::OnDisappear { .. }
                | Node::OnActive { .. }
                | Node::OnInactive { .. }
                | Node::OnBackground { .. } => {
                    return Err(unsupported("lifecycle callbacks", span));
                }
                Node::Appearance { .. }
                | Node::Form { .. }
                | Node::FormSection { .. }
                | Node::SafeAreaPadding { .. }
                | Node::SafeAreaInset { .. }
                | Node::ScrollHeader { .. } => {
                    return Err(unsupported("platform-specific container", span));
                }
                Node::Button { .. }
                | Node::TextInput { .. }
                | Node::Switch { .. }
                | Node::Slider { .. }
                | Node::ProgressBar { .. }
                | Node::ProgressRing { .. }
                | Node::SegmentedControl { .. }
                | Node::Picker { .. }
                | Node::PickerDate { .. }
                | Node::Image { .. }
                | Node::ContentUnavailable { .. }
                | Node::LinearGradient { .. }
                | Node::Pressable { .. }
                | Node::KeyboardAware { .. }
                | Node::BottomSheet { .. }
                | Node::Dialog { .. }
                | Node::ConfirmationDialog { .. }
                | Node::RefreshControl { .. }
                | Node::AppBottomBar { .. }
                | Node::PagePager { .. }
                | Node::Toolbar { .. }
                | Node::When { .. }
                | Node::StatusBar { .. }
                | Node::Direction { .. }
                | Node::Content => {
                    return Err(unsupported("app-only or stateful component", span));
                }
            }
        }
        Ok(())
    }

    visit(nodes, span)
}

fn resolve_class_member_type(
    syntax: &ast::TypeSyntax,
    structs: &StructTypes,
    plugins: &[ast::PluginDecl],
    allow_void: bool,
) -> Result<Type, CompileError> {
    let parsed = if allow_void {
        parse_return_type(syntax)?
    } else {
        parse_type(syntax)?
    };
    if let Type::Enum(path) = &parsed
        && let Some((namespace, native_name)) = path.split_once('.')
        && let Some(plugin) = plugins.iter().find(|plugin| plugin.namespace == namespace)
        && let Some(idl) = plugin.idl.as_ref()
        && idl.interfaces.iter().any(|interface| {
            interface.name == native_name
                && matches!(interface.kind, nexa_plugin_idl::InterfaceKind::NativeClass)
        })
    {
        return Ok(Type::Plugin {
            namespace: namespace.to_owned(),
            name: native_name.to_owned(),
        });
    }
    Ok(resolve_struct_type(&parsed, structs))
}

pub fn lower_with_warnings(
    app: ast::App,
    target: Target,
) -> Result<(Module, Vec<CompileWarning>, crate::testing::TestSuite), CompileError> {
    let (module, warnings, tests, _) =
        lower_with_warnings_in_mode(app, target, false, false, None, None, None, None, None)?;
    Ok((module, warnings, tests))
}

pub(super) fn lower_with_project_targets(
    app: ast::App,
    target: Target,
    dev_runtime: bool,
    custom_icons: &HashMap<String, nexa_ir::SystemIcon>,
) -> Result<(Module, Vec<CompileWarning>, crate::testing::TestSuite), CompileError> {
    let (module, warnings, tests, _) = lower_with_warnings_in_mode(
        app,
        target,
        dev_runtime,
        false,
        Some(custom_icons),
        None,
        None,
        None,
        None,
    )?;
    Ok((module, warnings, tests))
}

pub(crate) fn lower_with_project_targets_and_dev_cache(
    app: ast::App,
    target: Target,
    custom_icons: &HashMap<String, nexa_ir::SystemIcon>,
    cache: &mut DevFunctionLoweringCache,
    function_sources: &HashMap<String, String>,
    screen_sources: &HashMap<String, String>,
    app_source: Option<&str>,
) -> Result<(Module, Vec<CompileWarning>, crate::testing::TestSuite), CompileError> {
    let (module, warnings, tests, _) = lower_with_warnings_in_mode(
        app,
        target,
        true,
        false,
        Some(custom_icons),
        Some(cache),
        Some(function_sources),
        Some(screen_sources),
        app_source,
    )?;
    Ok((module, warnings, tests))
}

pub(super) fn lower_with_project_targets_and_optimization_report(
    app: ast::App,
    target: Target,
    dev_runtime: bool,
    custom_icons: &HashMap<String, nexa_ir::SystemIcon>,
) -> Result<
    (
        Module,
        Vec<CompileWarning>,
        crate::testing::TestSuite,
        crate::optimize::OptimizationReport,
    ),
    CompileError,
> {
    let (module, warnings, tests, report) = lower_with_warnings_in_mode(
        app,
        target,
        dev_runtime,
        true,
        Some(custom_icons),
        None,
        None,
        None,
        None,
    )?;
    Ok((module, warnings, tests, report.unwrap_or_default()))
}

#[allow(clippy::too_many_arguments)]
fn lower_with_warnings_in_mode(
    mut app: ast::App,
    target: Target,
    allow_nullable_generic_plugin_reads: bool,
    include_optimization_report: bool,
    custom_icons: Option<&HashMap<String, nexa_ir::SystemIcon>>,
    mut dev_function_cache: Option<&mut DevFunctionLoweringCache>,
    function_sources: Option<&HashMap<String, String>>,
    screen_sources: Option<&HashMap<String, String>>,
    app_source: Option<&str>,
) -> Result<
    (
        Module,
        Vec<CompileWarning>,
        crate::testing::TestSuite,
        Option<crate::optimize::OptimizationReport>,
    ),
    CompileError,
> {
    let screen_environment = dev_function_cache.as_ref().map(|_| {
        screen_lowering_environment_key(
            &app,
            target,
            allow_nullable_generic_plugin_reads,
            custom_icons,
        )
    });
    let app_source_file = app.source_file.clone();
    let warnings_started = Instant::now();
    let warnings = if let Some(cache) = dev_function_cache.as_mut() {
        if let Some(cached) = &cache.cached_warnings {
            cached.clone()
        } else {
            let computed = warnings::analyze(&app, target);
            cache.cached_warnings = Some(computed.clone());
            computed
        }
    } else {
        warnings::analyze(&app, target)
    };
    if allow_nullable_generic_plugin_reads {
        eprintln!(
            "Nexa semantic {target:?} warnings: {} ms",
            warnings_started.elapsed().as_millis()
        );
    }
    let screen_env_matches = dev_function_cache
        .as_ref()
        .zip(screen_environment.as_ref())
        .is_some_and(|(cache, environment)| cache.screen_environment.as_ref() == Some(environment));
    let is_warm_decl = screen_env_matches
        && dev_function_cache
            .as_ref()
            .and_then(|c| c.program_env.as_ref())
            .is_some();
    let decl_started = Instant::now();

    let env = if is_warm_decl {
        dev_function_cache
            .as_ref()
            .unwrap()
            .program_env
            .as_ref()
            .unwrap()
            .clone()
    } else {
        let themes = lower_theme(app.theme.as_ref())?;
        let enum_declarations = lower_enum_declarations(&app.enums)?;
        let (struct_declarations, mut struct_types) = lower_struct_declarations(&app.structs)?;
        let mut class_types = HashMap::with_capacity(app.classes.len());
        for declaration in &app.classes {
            if class_types.contains_key(&declaration.name)
                || struct_types.contains_key(&declaration.name)
            {
                return Err(CompileError::new(
                    declaration.span,
                    format!("type `{}` is already declared", declaration.name),
                ));
            }
            class_types.insert(
                declaration.name.clone(),
                Type::Class {
                    name: declaration.name.clone(),
                    fields: Vec::new(),
                    constructor_parameter_count: declaration.constructor_parameters.len(),
                },
            );
        }
        for declaration in &app.classes {
            let fields = declaration
                .constructor_parameters
                .iter()
                .map(|parameter| {
                    Ok((
                        parameter.name.clone(),
                        resolve_class_member_type(&parameter.ty, &struct_types, &app.plugins, false)?,
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            class_types.insert(
                declaration.name.clone(),
                Type::Class {
                    name: declaration.name.clone(),
                    fields,
                    constructor_parameter_count: declaration.constructor_parameters.len(),
                },
            );
        }
        struct_types.extend(class_types.clone());
        let mut enum_symbols = enum_symbols(&enum_declarations);
        enum_symbols.extend(plugin_enum_symbols(&app.plugins));
        let mut enum_names = enum_declarations
            .iter()
            .map(|declaration| declaration.name.as_str())
            .collect::<std::collections::HashSet<_>>();
        enum_names.extend(app.classes.iter().map(|class| class.name.as_str()));
        enum_names.insert("JsonError");
        for case in [
            "invalidJson",
            "typeMismatch",
            "missingField",
            "invalidValue",
        ] {
            enum_symbols.insert(
                format!("JsonError.{case}"),
                (Type::Enum("JsonError".to_owned()), false),
            );
        }
        let owned_enum_names = enum_names
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<std::collections::HashSet<String>>();
        let validate_types_started = Instant::now();
        validate_declared_types(&app, &enum_names, &struct_types)?;
        let validate_types_ms = validate_types_started.elapsed().as_millis();
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
        let fn_signatures_started = Instant::now();
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
        let fn_signatures_ms = fn_signatures_started.elapsed().as_millis();
        let classes_started = Instant::now();
        for declaration in &app.classes {
            let Some(Type::Class {
                name,
                mut fields,
                constructor_parameter_count,
            }) = class_types.get(&declaration.name).cloned()
            else {
                return Err(CompileError::new(
                    declaration.span,
                    format!("class type `{}` was not resolved", declaration.name),
                ));
            };
            let mut property_symbols = fields
                .iter()
                .map(|(name, ty)| (name.clone(), (ty.clone(), false)))
                .collect::<HashMap<_, _>>();
            for field in &declaration.fields {
                let ty = match &field.ty {
                    Some(ty) => resolve_class_member_type(ty, &struct_types, &app.plugins, false)?,
                    None => {
                        let inferred = infer_expr_type(&field.initial, &property_symbols, &function_signatures)
                            .ok_or_else(|| CompileError::new(field.span, format!("cannot infer the type of class property `{}`; add a type annotation", field.name)))?;
                        if generics::mentions_type_parameter(&inferred) {
                            return Err(CompileError::new(
                                field.span,
                                format!(
                                    "cannot infer the type of class property `{}`; add a type annotation",
                                    field.name
                                ),
                            ));
                        }
                        resolve_struct_type(&inferred, &struct_types)
                    }
                };
                property_symbols.insert(field.name.clone(), (ty.clone(), false));
                fields.push((field.name.clone(), ty));
            }
            class_types.insert(
                declaration.name.clone(),
                Type::Class {
                    name,
                    fields,
                    constructor_parameter_count,
                },
            );
        }
        struct_types.extend(class_types.clone());
        let mut class_method_declarations = Vec::new();
        for declaration in &app.classes {
            let Some(receiver) = class_types.get(&declaration.name).cloned() else {
                return Err(CompileError::new(
                    declaration.span,
                    format!("class type `{}` was not resolved", declaration.name),
                ));
            };
            let Type::Class { .. } = &receiver else {
                return Err(CompileError::new(
                    declaration.span,
                    format!("invalid class type for `{}`", declaration.name),
                ));
            };
            let parameters = declaration
                .constructor_parameters
                .iter()
                .map(|parameter| {
                    Ok((
                        parameter.name.clone(),
                        resolve_class_member_type(&parameter.ty, &struct_types, &app.plugins, false)?,
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            if function_signatures.contains_key(&declaration.name) {
                return Err(CompileError::new(
                    declaration.span,
                    format!("class `{}` conflicts with a function", declaration.name),
                ));
            }
            function_signatures.insert(
                declaration.name.clone(),
                FunctionSignature {
                    parameters: parameters.clone(),
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
            let mut method_names = HashSet::new();
            for method in &declaration.methods {
                if method.failure_type.is_some() {
                    return Err(CompileError::new(
                        method.span,
                        "typed `throws` is supported on reusable component ref methods only",
                    ));
                }
                if !method_names.insert(method.name.as_str()) {
                    return Err(CompileError::new(
                        method.span,
                        format!(
                            "class method `{}` is overloaded or declared more than once",
                            method.name
                        ),
                    ));
                }
                let key = format!("{}.{}", declaration.name, method.name);
                if function_signatures.contains_key(&key) {
                    return Err(CompileError::new(
                        method.span,
                        format!("method `{key}` conflicts with an existing declaration"),
                    ));
                }
                let method_parameters = method
                    .parameters
                    .iter()
                    .map(|parameter| {
                        Ok((
                            parameter.name.clone(),
                            resolve_class_member_type(
                                &parameter.ty,
                                &struct_types,
                                &app.plugins,
                                false,
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>, CompileError>>()?;
                let return_type =
                    resolve_class_member_type(&method.return_type, &struct_types, &app.plugins, true)?;
                function_signatures.insert(
                    key.clone(),
                    FunctionSignature {
                        parameters: method_parameters,
                        type_parameters: Vec::new(),
                        return_type,
                        is_async: method.is_async,
                        is_throwing: method.is_throwing,
                        receiver: Some(receiver.clone()),
                        is_constructor: false,
                        is_mutable_property: false,
                        error_handling_allowed: false,
                        error_type: None,
                    },
                );
                let mut lowered_method = method.clone();
                lowered_method.name = key;
                class_method_declarations.push(lowered_method);
            }
            for method in &declaration.static_methods {
                if method.failure_type.is_some() {
                    return Err(CompileError::new(
                        method.span,
                        "typed `throws` is supported on reusable component ref methods only",
                    ));
                }
                let key = format!("{}.{}", declaration.name, method.name);
                if function_signatures.contains_key(&key) {
                    return Err(CompileError::new(
                        method.span,
                        format!("class method `{key}` conflicts with an existing declaration"),
                    ));
                }
                let method_parameters = method
                    .parameters
                    .iter()
                    .map(|parameter| {
                        Ok((
                            parameter.name.clone(),
                            resolve_class_member_type(
                                &parameter.ty,
                                &struct_types,
                                &app.plugins,
                                false,
                            )?,
                        ))
                    })
                    .collect::<Result<Vec<_>, CompileError>>()?;
                let return_type =
                    resolve_class_member_type(&method.return_type, &struct_types, &app.plugins, true)?;
                function_signatures.insert(
                    key.clone(),
                    FunctionSignature {
                        parameters: method_parameters,
                        type_parameters: Vec::new(),
                        return_type,
                        is_async: method.is_async,
                        is_throwing: method.is_throwing,
                        receiver: None,
                        is_constructor: false,
                        is_mutable_property: false,
                        error_handling_allowed: false,
                        error_type: None,
                    },
                );
                let mut lowered_method = method.clone();
                lowered_method.name = key;
                class_method_declarations.push(lowered_method);
            }
        }
        let classes_ms = classes_started.elapsed().as_millis();
        if allow_nullable_generic_plugin_reads {
            eprintln!(
                "Nexa semantic {target:?} decl setup: validate types {validate_types_ms} ms, fn signatures {fn_signatures_ms} ms, classes {classes_ms} ms"
            );
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
                    type_parameters: Vec::new(),
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
        let screen_signatures = collect_screen_signatures(&app.screens, &struct_types)?;
        custom_components::collect_ref_method_signatures(
            &app.components,
            &struct_types,
            &mut function_signatures,
        )?;
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
                    supports_ref: true,
                },
            );
        }
        let mut component_global_symbols = enum_symbols.clone();
        for global in &app.globals {
            let ty = resolve_declaration_type(
                global,
                &component_global_symbols,
                &function_signatures,
                &struct_types,
            )?;
            component_global_symbols.insert(global.name.clone(), (ty, false));
        }
        let component_lowering_started = Instant::now();
        let (components, component_signatures) = lower_components(
            std::mem::take(&mut app.components),
            custom_components::ComponentLoweringContext {
                screen_ids: &screen_signatures,
                themes: &themes,
                functions: &function_signatures,
                structs: &struct_types,
                enums: &owned_enum_names,
                enum_symbols: &enum_symbols,
                global_symbols: &component_global_symbols,
                external_signatures: &native_component_signatures,
                target,
                allow_nullable_generic_plugin_reads,
                custom_icons,
            },
        )?;
        if allow_nullable_generic_plugin_reads {
            eprintln!(
                "Nexa semantic {target:?} components: {} ms",
                component_lowering_started.elapsed().as_millis()
            );
        }
        let test_suite = tests::lower_tests(
            std::mem::take(&mut app.tests),
            &app.functions,
            &app.structs,
            &app.enums,
            &components,
        )?;
        let background_tasks = app
            .background_tasks
            .into_iter()
            .map(|task| {
                if task.interval_minutes < 15 {
                    return Err(CompileError::new(
                        task.span,
                        "background task interval must be at least 15 minutes on Android",
                    ));
                }
                if !valid_background_task_identifier(&task.identifier) {
                    return Err(CompileError::new(
                        task.span,
                        format!(
                            "background task identifier `{}` must use reverse-domain notation, such as `dev.example.app.refresh`",
                            task.identifier
                        ),
                    ));
                }
                let actions = lower_actions_with_aliases(
                    task.body,
                    &enum_symbols,
                    &function_signatures,
                    true,
                    &HashMap::new(),
                    TypeRegistries {
                        structs: &struct_types,
                        enums: &owned_enum_names,
                        allow_nullable_generic_plugin_reads,
                        screen_ids: None,
                        allow_navigation_actions: false,
                        source_file: task.source_file.as_deref(),
                    },
                )?;
                Ok(BackgroundTask {
                    name: task.name,
                    identifier: task.identifier,
                    interval_minutes: task.interval_minutes,
                    actions,
                })
            })
            .collect::<Result<Vec<_>, CompileError>>()?;

        let global_names = app
            .globals
            .iter()
            .map(|global| global.name.clone())
            .collect::<HashSet<_>>();
        let mut declarations = app.globals;
        declarations.extend(app.states);
        let mut symbols = enum_symbols.clone();
        let mut states = Vec::with_capacity(declarations.len());
        let mut native_aliases = HashMap::new();
        for declaration in declarations {
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
            validate_task_handle_state(
                &ty,
                declaration.mutable,
                matches!(&declaration.initial, ast::Expr::Null(_)),
                declaration.span,
            )?;
            let state_context = ExprContext::with_types(
                &symbols,
                &function_signatures,
                false,
                &struct_types,
                &owned_enum_names,
            )
            .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads)
            .with_source_file(declaration.source_file.as_deref());
            let initial = lower_expr(&declaration.initial, Some(&ty), &state_context)?;
            let mutable = declaration.mutable;
            if mutable && references_mutable_state(&declaration.initial, &symbols) {
                return Err(CompileError::new(
                    declaration.initial.span(),
                    "mutable state initializers cannot refer to mutable state values",
                ));
            }
            record_native_alias(&declaration.name, &ty, &initial, &mut native_aliases);
            symbols.insert(declaration.name.clone(), (ty.clone(), mutable));
            states.push(State {
                name: declaration.name,
                ty,
                initial,
                mutable,
            });
        }
        for class in &app.classes {
            for field in &class.static_fields {
                let qualified_name = format!("{}::{}", class.name, field.name);
                if symbols.contains_key(&qualified_name)
                    || function_signatures.contains_key(&qualified_name)
                {
                    return Err(CompileError::new(
                        field.span,
                        format!("class property `{}` is already declared", field.name),
                    ));
                }
                let ty = match &field.ty {
                    Some(ty) => resolve_class_member_type(ty, &struct_types, &app.plugins, false)?,
                    None => {
                        let inferred = infer_expr_type(&field.initial, &symbols, &function_signatures).ok_or_else(|| CompileError::new(
                            field.span,
                            format!("cannot infer the type of static class property `{}`; add a type annotation", field.name),
                        ))?;
                        if generics::mentions_type_parameter(&inferred) {
                            return Err(CompileError::new(
                                field.span,
                                format!(
                                    "cannot infer the type of static class property `{}`; add a type annotation",
                                    field.name
                                ),
                            ));
                        }
                        resolve_struct_type(&inferred, &struct_types)
                    }
                };
                let initial = lower_expr(
                    &field.initial,
                    Some(&ty),
                    &ExprContext::with_types(
                        &symbols,
                        &function_signatures,
                        false,
                        &struct_types,
                        &owned_enum_names,
                    )
                    .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
                )?;
                symbols.insert(qualified_name.clone(), (ty.clone(), false));
                states.push(State {
                    name: qualified_name,
                    ty,
                    initial,
                    mutable: false,
                });
            }
        }
        let global_symbols = states
            .iter()
            .filter(|state| global_names.contains(&state.name) || state.name.contains("::"))
            .map(|state| (state.name.clone(), (state.ty.clone(), false)))
            .collect::<HashMap<_, _>>();
        let mut class_initializers = HashMap::with_capacity(app.classes.len());
        for declaration in &app.classes {
            let mut initializers = Vec::with_capacity(declaration.fields.len());
            let mut symbols = HashMap::new();
            if let Some(Type::Class {
                fields,
                constructor_parameter_count,
                ..
            }) = class_types.get(&declaration.name)
            {
                for (name, ty) in fields.iter().take(*constructor_parameter_count) {
                    symbols.insert(name.clone(), (ty.clone(), false));
                }
                for field in &declaration.fields {
                    let Some((_, ty)) = fields.iter().find(|(name, _)| name == &field.name) else {
                        return Err(CompileError::new(
                            field.span,
                            format!("class property `{}` has no resolved type", field.name),
                        ));
                    };
                    let initial = lower_expr(
                        &field.initial,
                        Some(ty),
                        &ExprContext::with_types(
                            &symbols,
                            &function_signatures,
                            false,
                            &struct_types,
                            &owned_enum_names,
                        )
                        .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
                    )?;
                    symbols.insert(field.name.clone(), (ty.clone(), false));
                    initializers.push(FunctionLocal {
                        name: field.name.clone(),
                        ty: ty.clone(),
                        initial,
                    });
                }
            }
            class_initializers.insert(declaration.name.clone(), initializers);
        }

        let new_env = ProgramEnvironment {
            themes,
            enum_declarations,
            struct_declarations,
            struct_types,
            class_types,
            enum_symbols,
            owned_enum_names,
            function_signatures,
            screen_signatures,
            components,
            component_signatures,
            test_suite,
            background_tasks,
            states,
            global_names,
            symbols,
            native_aliases,
            global_symbols,
            class_initializers,
            class_method_declarations,
        };
        let env_arc = Arc::new(new_env);
        if let Some(cache) = dev_function_cache.as_mut() {
            cache.program_env = Some(env_arc.clone());
        }
        env_arc
    };

    let themes = &env.themes;
    let enum_declarations = env.enum_declarations.clone();
    let struct_declarations = env.struct_declarations.clone();
    let struct_types = &env.struct_types;
    let class_types = &env.class_types;
    let enum_symbols = &env.enum_symbols;
    let owned_enum_names = &env.owned_enum_names;
    let function_signatures = &env.function_signatures;
    let screen_signatures = &env.screen_signatures;
    let components = env.components.clone();
    let component_signatures = &env.component_signatures;
    let mut test_suite = env.test_suite.clone();
    let background_tasks = env.background_tasks.clone();
    let mut states = env.states.clone();
    let global_names = &env.global_names;
    let symbols = &env.symbols;
    let native_aliases = &env.native_aliases;
    let global_symbols = &env.global_symbols;
    let class_initializers = &env.class_initializers;
    let class_method_declarations = env.class_method_declarations.clone();
    if allow_nullable_generic_plugin_reads {
        eprintln!(
            "Nexa semantic {target:?} decl setup: {} ms (reused: {is_warm_decl})",
            decl_started.elapsed().as_millis()
        );
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
            "apps with screen declarations must provide one top-level `NavigationStack` root in every body branch",
        ));
    }
    let mut function_declarations = std::mem::take(&mut app.functions);
    function_declarations.extend(class_method_declarations);
    let function_lowering_started = Instant::now();
    let mut functions = lower_functions(
        function_declarations,
        FunctionLoweringContext {
            signatures: function_signatures,
            structs: struct_types,
            enums: owned_enum_names,
            enum_symbols,
            global_symbols,
            class_initializers,
            allow_nullable_generic_plugin_reads,
        },
        is_warm_decl,
        dev_function_cache
            .as_mut()
            .zip(function_sources)
            .map(|(cache, sources)| (&mut **cache, sources)),
    )?;
    if allow_nullable_generic_plugin_reads {
        eprintln!(
            "Nexa semantic {target:?} functions: {} ms",
            function_lowering_started.elapsed().as_millis()
        );
    }
    for function in &mut functions {
        if function.receiver.is_some()
            && let Some((class_name, _)) = function.name.split_once('.')
            && let Some(receiver) = class_types.get(class_name)
        {
            function.receiver = Some(receiver.clone());
        }
    }
    tests::bind_test_functions(&mut test_suite, &functions);
    let mut all_state_names = states
        .iter()
        .map(|state| state.name.clone())
        .collect::<HashSet<_>>();
    let screen_widget_app_lowering_started = Instant::now();
    let screen_env_matches = dev_function_cache
        .as_ref()
        .zip(screen_environment.as_ref())
        .is_some_and(|(cache, environment)| cache.screen_environment.as_ref() == Some(environment));

    let mut cached_screens = if screen_env_matches {
        dev_function_cache
            .as_mut()
            .map(|cache| std::mem::take(&mut cache.screens))
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut screens = Vec::with_capacity(app.screens.len());
    let mut unchanged_screen_indices = Vec::new();
    let mut next_cached_screens = HashMap::with_capacity(app.screens.len());

    for (index, screen) in app.screens.into_iter().enumerate() {
        let current_source = screen_sources.and_then(|sources| sources.get(&screen.name));
        if screen_env_matches
            && let Some(mut cached) = cached_screens.remove(&screen.name)
            && current_source.is_some_and(|source| *source == cached.source)
        {
            for state in &cached.screen.states {
                all_state_names.insert(state.name.clone());
            }
            cached.screen.id = ScreenId(index);
            unchanged_screen_indices.push(index);
            screens.push(cached.screen.clone());
            next_cached_screens.insert(screen.name, cached);
            continue;
        }

        let screen_source_file = screen.source_file.as_deref();
        macro_rules! screen_try {
            ($result:expr) => {
                $result.map_err(|error| with_screen_source(error, screen_source_file))?
            };
        }

        let mut screen_symbols = symbols.clone();
        let mut screen_native_aliases = native_aliases.clone();
        let Some(screen_signature) = screen_signatures.get(&screen.name) else {
            return Err(with_screen_source(
                CompileError::new(
                    screen.span,
                    format!("screen `{}` has no resolved route signature", screen.name),
                ),
                screen_source_file,
            ));
        };
        for parameter in &screen_signature.parameters {
            if screen_symbols.contains_key(&parameter.name)
                || all_state_names.contains(&parameter.name)
                || function_signatures.contains_key(&parameter.name)
            {
                return Err(with_screen_source(
                    CompileError::new(
                        screen.span,
                        format!(
                            "screen parameter `{}` conflicts with an app binding or function",
                            parameter.name
                        ),
                    ),
                    screen_source_file,
                ));
            }
            screen_symbols.insert(parameter.name.clone(), (parameter.ty.clone(), false));
        }
        let mut screen_states = Vec::with_capacity(screen.states.len());
        for declaration in screen.states {
            if screen_symbols.contains_key(&declaration.name)
                || !all_state_names.insert(declaration.name.clone())
            {
                return Err(with_screen_source(
                    CompileError::new(
                        declaration.span,
                        format!(
                            "screen state `{}` conflicts with another app or screen state",
                            declaration.name
                        ),
                    ),
                    screen_source_file,
                ));
            }
            if function_signatures.contains_key(&declaration.name) {
                return Err(with_screen_source(
                    CompileError::new(
                        declaration.span,
                        format!(
                            "screen state `{}` is already declared as a function",
                            declaration.name
                        ),
                    ),
                    screen_source_file,
                ));
            }
            let ty = screen_try!(resolve_declaration_type(
                &declaration,
                &screen_symbols,
                function_signatures,
                struct_types,
            ));
            screen_try!(validate_task_handle_state(
                &ty,
                declaration.mutable,
                matches!(&declaration.initial, ast::Expr::Null(_)),
                declaration.span,
            ));
            let state_context = ExprContext::with_types(
                &screen_symbols,
                function_signatures,
                false,
                struct_types,
                owned_enum_names,
            )
            .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads)
            .with_source_file(screen_source_file);
            let initial =
                screen_try!(lower_expr(&declaration.initial, Some(&ty), &state_context,));
            let mutable = declaration.mutable;
            if mutable && references_mutable_state(&declaration.initial, &screen_symbols) {
                return Err(with_screen_source(
                    CompileError::new(
                        declaration.initial.span(),
                        "mutable screen state initializers cannot refer to mutable state values",
                    ),
                    screen_source_file,
                ));
            }
            record_native_alias(&declaration.name, &ty, &initial, &mut screen_native_aliases);
            screen_symbols.insert(declaration.name.clone(), (ty.clone(), mutable));
            screen_states.push(State {
                name: declaration.name,
                ty,
                initial,
                mutable,
            });
        }
        let cx = SemanticContext::new(
            &screen_symbols,
            screen_signatures,
            themes,
            component_signatures,
            function_signatures,
            &screen_native_aliases,
            struct_types,
            owned_enum_names,
            target,
            allow_nullable_generic_plugin_reads,
        )
        .with_navigation(false, true)
        .with_optional_custom_icons(custom_icons)
        .with_source_file(screen_source_file);
        let screen_body = screen_try!(lower_nodes(screen.body, &cx));
        let (status_bar, screen_body) =
            screen_try!(extract_status_bar(screen_body, screen.span, "screen"));
        if screen_body.iter().any(contains_content) {
            return Err(with_screen_source(
                CompileError::new(
                    screen.span,
                    "Content() is only available inside a custom component declaration",
                ),
                screen_source_file,
            ));
        }
        if screen_body.iter().any(contains_direction) {
            return Err(with_screen_source(
                CompileError::new(
                    app.span,
                    "Direction is only allowed at the app body's top level",
                ),
                screen_source_file,
            ));
        }
        if screen_body.iter().any(contains_on_active)
            || screen_body.iter().any(contains_on_inactive)
            || screen_body.iter().any(contains_on_background)
        {
            return Err(with_screen_source(
                CompileError::new(
                    screen.span,
                    "app lifecycle events are only allowed at the app body's top level",
                ),
                screen_source_file,
            ));
        }
        let (on_appear, on_appear_async, screen_body) =
            screen_try!(extract_on_appear(screen_body, screen.span, "screen"));
        let (on_disappear, screen_body) =
            screen_try!(extract_on_disappear(screen_body, screen.span, "screen"));
        let screen_name = screen.name;
        let lowered_screen = Screen {
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
        };
        let mut struct_names = HashSet::new();
        crate::optimize::collect_screen_struct_names(&lowered_screen, &mut struct_names);
        screens.push(lowered_screen.clone());
        if let Some(source) = current_source {
            next_cached_screens.insert(
                screen_name,
                CachedScreen {
                    source: source.to_owned(),
                    screen: lowered_screen,
                    struct_names,
                },
            );
        }
    }

    if let Some(cache) = dev_function_cache.as_mut() {
        cache.screen_environment = screen_environment;
        cache.screen_struct_facts = next_cached_screens
            .iter()
            .map(|(name, cached)| (name.clone(), cached.struct_names.clone()))
            .collect();
        cache.screens = next_cached_screens;
        cache.unchanged_screen_indices = unchanged_screen_indices.clone();
    }

    if allow_nullable_generic_plugin_reads {
        let reused_count = unchanged_screen_indices.len();
        let lowered_count = screens.len() - reused_count;
        eprintln!(
            "Nexa semantic {target:?} screen cache: {reused_count} reused, {lowered_count} lowered ({} ms)",
            screen_widget_app_lowering_started.elapsed().as_millis()
        );
    }

    let mut widget_names = HashSet::with_capacity(app.widgets.len());
    let mut widgets = Vec::with_capacity(app.widgets.len());
    for widget in app.widgets {
        let source_file = widget.source_file.as_deref();
        let entry_type = infer_expr_type(&widget.entry_provider, symbols, function_signatures)
            .ok_or_else(|| {
                with_screen_source(
                    CompileError::new(
                        widget.entry_provider.span(),
                        "cannot infer widget entry provider return type",
                    ),
                    source_file,
                )
            })?;
        let Type::Struct { .. } = &entry_type else {
            return Err(with_screen_source(
                CompileError::new(
                    widget.entry_provider.span(),
                    "widget entry provider must return a declared struct type",
                ),
                source_file,
            ));
        };
        if !widget_names.insert(widget.name.clone()) {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    format!("widget `{}` is already declared", widget.name),
                ),
                source_file,
            ));
        }
        if (widget.configuration_title.is_some() || widget.configuration_description.is_some())
            && widget.configuration.is_none()
        {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    "widget configuration title and description require `configuration`",
                ),
                source_file,
            ));
        }
        let mut seen_families = HashSet::new();
        let families = widget.families.iter().map(|family| {
            if !seen_families.insert(family.as_str()) {
                return Err(CompileError::new(widget.span, format!("widget family `{family}` is listed more than once")));
            }
            match family.as_str() {
                "Small" => Ok(WidgetFamily::Small),
                "Medium" => Ok(WidgetFamily::Medium),
                "Large" => Ok(WidgetFamily::Large),
                "ExtraLarge" => Ok(WidgetFamily::ExtraLarge),
                _ => Err(CompileError::new(widget.span, format!("unknown widget family `{family}`; expected Small, Medium, Large, or ExtraLarge"))),
            }
        }).collect::<Result<Vec<_>, _>>().map_err(|error| with_screen_source(error, source_file))?;
        if families.is_empty() {
            return Err(with_screen_source(
                CompileError::new(widget.span, "widget requires at least one supported family"),
                source_file,
            ));
        }
        let refresh_seconds = match &widget.refresh_seconds {
            ast::Expr::Number(raw, _) => raw.parse::<u32>().ok().filter(|seconds| *seconds > 0),
            _ => None,
        }
        .ok_or_else(|| {
            with_screen_source(
                CompileError::new(
                    widget.refresh_seconds.span(),
                    "`refreshSeconds` must be a positive integer literal",
                ),
                source_file,
            )
        })?;
        let mut widget_symbols = enum_symbols
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<HashMap<_, _>>();
        widget_symbols.extend(global_symbols.clone());
        if enum_symbols.contains_key("__enum::WidgetFamily") {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    "`WidgetFamily` is reserved for Nexa widget size selection",
                ),
                source_file,
            ));
        }
        if widget_symbols.contains_key("entry") {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    "widget content uses the reserved `entry` binding",
                ),
                source_file,
            ));
        }
        if widget_symbols.contains_key("family") {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    "widget content uses the reserved `family` binding",
                ),
                source_file,
            ));
        }
        let configuration = if let Some(default) = &widget.configuration {
            if widget_symbols.contains_key("configuration") {
                return Err(with_screen_source(
                    CompileError::new(
                        widget.span,
                        "widget content uses the reserved `configuration` binding",
                    ),
                    source_file,
                ));
            }
            let ty = infer_expr_type(default, &widget_symbols, function_signatures).ok_or_else(|| {
                with_screen_source(
                    CompileError::new(
                        default.span(),
                        "cannot infer the widget configuration type; provide a declared struct value",
                    ),
                    source_file,
                )
            })?;
            let Type::Struct { fields, .. } = &ty else {
                return Err(with_screen_source(
                    CompileError::new(
                        default.span(),
                        "widget configuration must be a declared struct with selectable enum fields",
                    ),
                    source_file,
                ));
            };
            if fields.is_empty() {
                return Err(with_screen_source(
                    CompileError::new(
                        default.span(),
                        "widget configuration must contain at least one field",
                    ),
                    source_file,
                ));
            }
            for (field_name, field_type) in fields {
                if !matches!(field_type, Type::Enum(enum_name) if !enum_name.contains('.')) {
                    return Err(with_screen_source(
                        CompileError::new(
                            default.span(),
                            format!(
                                "widget configuration field `{field_name}` must use a declared enum type"
                            ),
                        ),
                        source_file,
                    ));
                }
            }
            let default_value = lower_expr(
                default,
                Some(&ty),
                &ExprContext::with_types(
                    &widget_symbols,
                    function_signatures,
                    false,
                    struct_types,
                    owned_enum_names,
                )
                .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
            )
            .map_err(|error| with_screen_source(error, source_file))?;
            widget_symbols.insert("configuration".to_owned(), (ty.clone(), false));
            Some(nexa_ir::WidgetConfiguration {
                ty,
                default: default_value,
            })
        } else {
            None
        };
        let family_type = Type::Enum("WidgetFamily".to_owned());
        widget_symbols.insert("family".to_owned(), (family_type.clone(), false));
        for family in ["Small", "Medium", "Large", "ExtraLarge"] {
            widget_symbols.insert(
                format!("WidgetFamily.{family}"),
                (family_type.clone(), false),
            );
        }
        widget_symbols.insert("entry".to_owned(), (entry_type.clone(), false));
        let mut provider_symbols = enum_symbols
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<HashMap<_, _>>();
        provider_symbols.extend(global_symbols.clone());
        if let Some(configuration) = &configuration {
            provider_symbols.insert(
                "configuration".to_owned(),
                (configuration.ty.clone(), false),
            );
        }
        provider_symbols.insert("family".to_owned(), (family_type.clone(), false));
        for family in ["Small", "Medium", "Large", "ExtraLarge"] {
            provider_symbols.insert(
                format!("WidgetFamily.{family}"),
                (family_type.clone(), false),
            );
        }
        let widget_context = SemanticContext::new(
            &widget_symbols,
            screen_signatures,
            themes,
            component_signatures,
            function_signatures,
            native_aliases,
            struct_types,
            owned_enum_names,
            target,
            allow_nullable_generic_plugin_reads,
        )
        .with_navigation(false, false)
        .with_optional_custom_icons(custom_icons)
        .with_source_file(source_file);
        let body = lower_nodes(widget.body, &widget_context)
            .map_err(|error| with_screen_source(error, source_file))?;
        validate_widget_body(&body, widget.span)
            .map_err(|error| with_screen_source(error, source_file))?;
        if body.iter().any(contains_content) {
            return Err(with_screen_source(
                CompileError::new(
                    widget.span,
                    "Content() is only available inside a custom component declaration",
                ),
                source_file,
            ));
        }
        let provider_functions = functions_with_error_handling(function_signatures, true);
        let entry_provider = lower_expr(
            &widget.entry_provider,
            Some(&entry_type),
            &ExprContext::with_types(
                &provider_symbols,
                &provider_functions,
                true,
                struct_types,
                owned_enum_names,
            )
            .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
        )
        .map_err(|error| with_screen_source(error, source_file))?;
        let entry_provider_async = expression_uses_async(&entry_provider);
        let entry_provider_throws = expression_uses_throwing_await(&entry_provider);
        if entry_provider_async && widget.placeholder_provider.is_none() {
            return Err(with_screen_source(
                CompileError::new(
                    widget.entry_provider.span(),
                    "an async widget entry requires a synchronous `placeholder` value",
                ),
                source_file,
            ));
        }
        if entry_provider_async && expression_references_state(&entry_provider, "family") {
            return Err(with_screen_source(
                CompileError::new(
                    widget.entry_provider.span(),
                    "async widget entries cannot depend on `family`; load family-independent data and select rows in the widget body",
                ),
                source_file,
            ));
        }
        if entry_provider_throws && widget.placeholder_provider.is_none() {
            return Err(with_screen_source(
                CompileError::new(
                    widget.entry_provider.span(),
                    "a throwing widget entry requires a synchronous `placeholder` fallback",
                ),
                source_file,
            ));
        }
        let placeholder_provider = if let Some(placeholder) = &widget.placeholder_provider {
            let placeholder_type =
                infer_expr_type(placeholder, &provider_symbols, function_signatures).ok_or_else(
                    || {
                        with_screen_source(
                            CompileError::new(
                                placeholder.span(),
                                "cannot infer widget placeholder return type",
                            ),
                            source_file,
                        )
                    },
                )?;
            if placeholder_type != entry_type {
                return Err(with_screen_source(
                    CompileError::new(
                        placeholder.span(),
                        "widget `placeholder` must return the same struct type as `entry`",
                    ),
                    source_file,
                ));
            }
            let lowered = lower_expr(
                placeholder,
                Some(&entry_type),
                &ExprContext::with_types(
                    &provider_symbols,
                    function_signatures,
                    false,
                    struct_types,
                    owned_enum_names,
                )
                .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads),
            )
            .map_err(|error| with_screen_source(error, source_file))?;
            if expression_references_state(&lowered, "family") {
                return Err(with_screen_source(
                    CompileError::new(
                        placeholder.span(),
                        "widget `placeholder` cannot depend on `family`; it must be available synchronously on every platform",
                    ),
                    source_file,
                ));
            }
            Some(lowered)
        } else {
            None
        };
        widgets.push(Widget {
            name: widget.name,
            display_name: widget.display_name,
            description: widget.description,
            configuration_title: widget.configuration_title,
            configuration_description: widget.configuration_description,
            configuration,
            entry_provider,
            placeholder_provider,
            entry_provider_async,
            entry_provider_throws,
            entry_type,
            families,
            refresh_seconds,
            body,
        });
    }

    let app_body_cached = if is_warm_decl
        && let Some(cache) = dev_function_cache.as_ref()
        && let Some(cached_app) = cache.cached_app_body.as_ref()
        && let Some(current_source) = app_source
        && cached_app.source_snippet == current_source
    {
        Some(cached_app.clone())
    } else {
        None
    };

    let (
        body,
        status_bar,
        direction,
        on_appear,
        on_appear_async,
        on_disappear,
        on_active,
        on_inactive,
        on_background,
    ) = if let Some(cached) = app_body_cached {
        (
            cached.body,
            cached.status_bar,
            cached.direction,
            cached.on_appear,
            cached.on_appear_async,
            cached.on_disappear,
            cached.on_active,
            cached.on_inactive,
            cached.on_background,
        )
    } else {
        let cx = SemanticContext::new(
            symbols,
            screen_signatures,
            themes,
            component_signatures,
            function_signatures,
            native_aliases,
            struct_types,
            owned_enum_names,
            target,
            allow_nullable_generic_plugin_reads,
        )
        .with_navigation(true, false)
        .with_optional_custom_icons(custom_icons)
        .with_source_file(app_source_file.as_deref());
        let body = lower_nodes(app.body, &cx)?;
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
        let (on_active, body) =
            extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Active)?;
        let (on_inactive, body) =
            extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Inactive)?;
        let (on_background, body) =
            extract_lifecycle_event(body, app.span, "app", LifecycleEvent::Background)?;
        if let Some(cache) = dev_function_cache.as_mut()
            && let Some(current_source) = app_source
        {
            cache.cached_app_body = Some(CachedAppBody {
                source_snippet: current_source.to_owned(),
                body: body.clone(),
                status_bar,
                direction,
                on_appear: on_appear.clone(),
                on_appear_async,
                on_disappear: on_disappear.clone(),
                on_active: on_active.clone(),
                on_inactive: on_inactive.clone(),
                on_background: on_background.clone(),
            });
        }
        (
            body,
            status_bar,
            direction,
            on_appear,
            on_appear_async,
            on_disappear,
            on_active,
            on_inactive,
            on_background,
        )
    };
    let components = if dev_function_cache.is_some() {
        components
    } else {
        retain_reachable(components, &body, &screens, &widgets)
    };
    let plugins = app
        .plugins
        .iter()
        .filter(|plugin| !plugin.pure)
        .map(|plugin| nexa_ir::Plugin {
            namespace: plugin.namespace.clone(),
            idl_path: plugin.path.clone(),
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
    let global_states = states
        .iter()
        .filter(|state| global_names.contains(&state.name) || state.name.contains("::"))
        .cloned()
        .collect();
    states.retain(|state| !global_names.contains(&state.name) && !state.name.contains("::"));
    let mut module = Module {
        app_name: app.name,
        plugins,
        plugin_assets,
        enums: enum_declarations,
        structs: struct_declarations,
        functions,
        background_tasks,
        states,
        globals: global_states,
        screens,
        widgets,
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
    if allow_nullable_generic_plugin_reads {
        eprintln!(
            "Nexa semantic {target:?} screens/widgets/app: {} ms",
            screen_widget_app_lowering_started.elapsed().as_millis()
        );
    }
    if module_uses_json_error(&module) {
        module.enums.push(nexa_ir::EnumDecl {
            name: "JsonError".to_owned(),
            cases: vec![
                "invalidJson".to_owned(),
                "typeMismatch".to_owned(),
                "missingField".to_owned(),
                "invalidValue".to_owned(),
            ],
        });
    }
    let validation_started = Instant::now();
    validate_module_callback_disposal(&module, function_signatures)?;
    let validation_elapsed = validation_started.elapsed();
    let optimization_started = Instant::now();
    let optimization_report = if include_optimization_report {
        Some(crate::optimize::optimize_with_report(&mut module))
    } else if let Some(cache) = dev_function_cache.as_mut() {
        let reused_count = cache.unchanged_function_names.len();
        crate::optimize::optimize_with_cached_function_names(
            &mut module,
            &cache.unchanged_function_names,
            &cache.optimization_facts,
            Some(&cache.unchanged_screen_indices),
            Some(&cache.screen_struct_facts),
        );
        cache.update_cached_functions(&module);
        if allow_nullable_generic_plugin_reads {
            eprintln!("Nexa semantic optimized function cache: {reused_count} reused");
        }
        None
    } else {
        crate::optimize::optimize(&mut module);
        None
    };
    if allow_nullable_generic_plugin_reads {
        eprintln!(
            "Nexa semantic {target:?} callback validation: {} ms, optimization: {} ms",
            validation_elapsed.as_millis(),
            optimization_started.elapsed().as_millis()
        );
    }
    Ok((module, warnings, test_suite, optimization_report))
}

fn valid_background_task_identifier(identifier: &str) -> bool {
    let labels = identifier.split('.').collect::<Vec<_>>();
    labels.len() >= 3
        && labels.iter().all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
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
        for (actions, kind) in [
            (component.on_appear.as_deref(), CallbackKind::Lifecycle),
            (component.on_disappear.as_deref(), CallbackKind::OnDisappear),
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

fn module_uses_json_error(module: &Module) -> bool {
    fn type_uses_json_error(ty: &Type) -> bool {
        match ty {
            Type::Enum(name) => name == "JsonError",
            Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) | Type::Signal(inner) => {
                type_uses_json_error(inner)
            }
            Type::Map(key, value) | Type::Pair(key, value) | Type::Result(key, value) => {
                type_uses_json_error(key) || type_uses_json_error(value)
            }
            Type::ComponentRefError(inner) => type_uses_json_error(inner),
            Type::Triple(first, second, third) => {
                type_uses_json_error(first)
                    || type_uses_json_error(second)
                    || type_uses_json_error(third)
            }
            Type::Struct { fields, .. } => fields
                .iter()
                .any(|(_, field_type)| type_uses_json_error(field_type)),
            Type::Class { fields, .. } => fields.iter().any(|(_, ty)| type_uses_json_error(ty)),
            Type::Void
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Numeric(_)
            | Type::TypeParam(_)
            | Type::Plugin { .. }
            | Type::TaskHandle
            | Type::NetworkResponse
            | Type::ComponentRef { .. } => false,
        }
    }

    let has_declared_type = module
        .structs
        .iter()
        .flat_map(|declaration| declaration.fields.iter())
        .any(|field| type_uses_json_error(&field.ty))
        || module
            .states
            .iter()
            .any(|state| type_uses_json_error(&state.ty))
        || module.screens.iter().any(|screen| {
            screen
                .states
                .iter()
                .any(|state| type_uses_json_error(&state.ty))
                || screen
                    .parameters
                    .iter()
                    .any(|parameter| type_uses_json_error(&parameter.ty))
        })
        || module.components.iter().any(|component| {
            component
                .states
                .iter()
                .any(|state| type_uses_json_error(&state.ty))
                || component
                    .parameters
                    .iter()
                    .any(|parameter| type_uses_json_error(&parameter.ty))
        })
        || module.functions.iter().any(|function| {
            type_uses_json_error(&function.return_type)
                || function
                    .parameters
                    .iter()
                    .any(|parameter| type_uses_json_error(&parameter.ty))
                || function
                    .locals
                    .iter()
                    .any(|local| type_uses_json_error(&local.ty))
        });
    if has_declared_type {
        return true;
    }

    fn expression_uses_json_error(expression: &nexa_ir::Expr, found: &mut bool) {
        nexa_ir::walk::walk_expression(expression, &mut |expression| match expression {
            nexa_ir::Expr::EnumValue { enum_name, .. } if enum_name == "JsonError" => {
                *found = true;
            }
            nexa_ir::Expr::NativeCall {
                namespace,
                name: _,
                return_type,
                codecs,
                ..
            } => {
                *found |= namespace == "Json";
                *found |= type_uses_json_error(return_type)
                    || codecs.iter().any(|codec| type_uses_json_error(&codec.ty));
            }
            nexa_ir::Expr::Call { return_type, .. } => {
                *found |= type_uses_json_error(return_type);
            }
            _ => {}
        });
    }

    let mut found = false;
    {
        let mut visit_node = |_: &Node| {};
        let mut visit_expression =
            |expression: &nexa_ir::Expr| expression_uses_json_error(expression, &mut found);
        nexa_ir::walk::walk_ir(&module.body, &mut visit_node, &mut visit_expression);
    }
    for state in &module.states {
        expression_uses_json_error(&state.initial, &mut found);
    }
    for function in &module.functions {
        for local in &function.locals {
            expression_uses_json_error(&local.initial, &mut found);
        }
        expression_uses_json_error(&function.body, &mut found);
        if let Some(actions) = &function.body_actions {
            nexa_ir::walk::walk_actions(actions, &mut |expression| {
                expression_uses_json_error(expression, &mut found)
            });
        }
    }
    for task in &module.background_tasks {
        nexa_ir::walk::walk_actions(&task.actions, &mut |expression| {
            expression_uses_json_error(expression, &mut found)
        });
    }
    for screen in &module.screens {
        {
            let mut visit_node = |_: &Node| {};
            let mut visit_expression =
                |expression: &nexa_ir::Expr| expression_uses_json_error(expression, &mut found);
            nexa_ir::walk::walk_ir(&screen.body, &mut visit_node, &mut visit_expression);
        }
        for state in &screen.states {
            expression_uses_json_error(&state.initial, &mut found);
        }
    }
    for component in &module.components {
        {
            let mut visit_node = |_: &Node| {};
            let mut visit_expression =
                |expression: &nexa_ir::Expr| expression_uses_json_error(expression, &mut found);
            nexa_ir::walk::walk_ir(&component.body, &mut visit_node, &mut visit_expression);
        }
        for state in &component.states {
            expression_uses_json_error(&state.initial, &mut found);
        }
    }
    for actions in [
        &module.on_appear,
        &module.on_disappear,
        &module.on_active,
        &module.on_inactive,
        &module.on_background,
    ]
    .into_iter()
    .flatten()
    {
        nexa_ir::walk::walk_actions(actions, &mut |expression| {
            expression_uses_json_error(expression, &mut found)
        });
    }
    for screen in &module.screens {
        for actions in [&screen.on_appear, &screen.on_disappear]
            .into_iter()
            .flatten()
        {
            nexa_ir::walk::walk_actions(actions, &mut |expression| {
                expression_uses_json_error(expression, &mut found)
            });
        }
    }
    found
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
                | "JsonError"
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
                    | "JsonError"
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
            let Some(resolved_type) = types.get(&declaration.name) else {
                return Err(in_source_file(
                    CompileError::new(
                        declaration.span,
                        format!("struct `{}` could not be resolved", declaration.name),
                    ),
                    declaration.source_file.as_deref(),
                ));
            };
            let Type::Struct { fields, .. } = resolved_type.clone() else {
                return Err(in_source_file(
                    CompileError::new(
                        declaration.span,
                        format!(
                            "struct `{}` resolved to a non-struct type",
                            declaration.name
                        ),
                    ),
                    declaration.source_file.as_deref(),
                ));
            };
            Ok(nexa_ir::StructDecl {
                name: declaration.name.clone(),
                fields: fields
                    .into_iter()
                    .map(|(name, ty)| nexa_ir::StructField { name, ty })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, CompileError>>()?;
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
        Type::Signal(inner) => Ok(Type::Signal(Box::new(resolve_struct_members(
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

fn plugin_enum_symbols(plugins: &[ast::PluginDecl]) -> HashMap<String, (nexa_ir::Type, bool)> {
    let mut symbols = HashMap::new();
    for plugin in plugins.iter().filter(|plugin| !plugin.pure) {
        let Some(idl) = plugin.idl.as_ref() else {
            continue;
        };
        for declaration in idl.types.iter().filter(|declaration| {
            matches!(
                declaration.kind,
                nexa_plugin_idl::NamedTypeKind::Enum | nexa_plugin_idl::NamedTypeKind::Error
            )
        }) {
            let ty = Type::Plugin {
                namespace: plugin.namespace.clone(),
                name: declaration.name.clone(),
            };
            for case in declaration
                .cases
                .iter()
                .filter(|case| case.parameters.is_empty())
            {
                symbols.insert(
                    format!("{}.{}.{}", plugin.namespace, declaration.name, case.name),
                    (ty.clone(), false),
                );
            }
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
    let mut plugin_types = HashSet::new();
    plugin_types.insert("Regex.Regex".to_owned());
    plugin_types.insert("Regex.RegexMatch".to_owned());
    plugin_types.insert("Regex.Range".to_owned());
    for plugin in &app.plugins {
        let Some(idl) = plugin.idl.as_ref() else {
            continue;
        };
        for declaration in &idl.types {
            plugin_types.insert(format!("{}.{}", plugin.namespace, declaration.name));
        }
        for interface in &idl.interfaces {
            if interface.kind == nexa_plugin_idl::InterfaceKind::NativeClass {
                let qualified_name = format!("{}.{}", plugin.namespace, interface.name);
                native_class_types.insert(qualified_name.clone());
                plugin_types.insert(qualified_name);
                if interface.name == plugin.namespace {
                    native_class_types.insert(interface.name.clone());
                }
            } else if interface.kind == nexa_plugin_idl::InterfaceKind::NativeComponent {
                plugin_types.insert(format!(
                    "{}.{}.__component_ref",
                    plugin.namespace, interface.name
                ));
            }
        }
    }
    for component in &app.components {
        if !component.ref_methods.is_empty() {
            plugin_types.insert(format!("Nexa.{}.__component_ref", component.name));
        }
    }

    for declaration in &app.structs {
        for field in &declaration.fields {
            let ty = resolve_struct_type(&parse_type(&field.ty)?, struct_types);
            validate_type_names(&ty, enum_names, &plugin_types, field.ty.span())
                .map_err(|error| in_source_file(error, declaration.source_file.as_deref()))?;
        }
    }
    for declaration in &app.states {
        if let Some(ty) = &declaration.ty {
            validate_type_names(
                &resolve_struct_type(&parse_type(ty)?, struct_types),
                enum_names,
                &plugin_types,
                ty.span(),
            )?;
        }
    }
    for declaration in &app.functions {
        for parameter in &declaration.parameters {
            validate_type_names(
                &resolve_struct_type(&parse_type(&parameter.ty)?, struct_types),
                enum_names,
                &plugin_types,
                parameter.ty.span(),
            )?;
        }
        validate_type_names(
            &resolve_struct_type(&parse_return_type(&declaration.return_type)?, struct_types),
            enum_names,
            &plugin_types,
            declaration.return_type.span(),
        )?;
        for statement in &declaration.body {
            if let ast::Stmt::Let { ty: Some(ty), .. } = statement {
                validate_type_names(
                    &resolve_struct_type(&parse_type(ty)?, struct_types),
                    enum_names,
                    &plugin_types,
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
                &plugin_types,
                parameter.ty.span(),
            )?;
        }
        for state in &declaration.states {
            if let Some(ty) = &state.ty {
                validate_type_names(
                    &resolve_struct_type(&parse_type(ty)?, struct_types),
                    enum_names,
                    &plugin_types,
                    ty.span(),
                )?;
            }
        }
        for method in &declaration.ref_methods {
            for parameter in &method.parameters {
                validate_component_type_names(
                    &resolve_struct_type(&parse_type(&parameter.ty)?, struct_types),
                    enum_names,
                    &native_class_types,
                    &plugin_types,
                    parameter.ty.span(),
                )?;
            }
            validate_component_type_names(
                &resolve_struct_type(&parse_return_type(&method.return_type)?, struct_types),
                enum_names,
                &native_class_types,
                &plugin_types,
                method.return_type.span(),
            )?;
        }
    }
    Ok(())
}

fn validate_component_type_names(
    ty: &Type,
    enum_names: &std::collections::HashSet<&str>,
    native_class_types: &HashSet<String>,
    plugin_types: &HashSet<String>,
    span: nexa_diagnostics::Span,
) -> Result<(), CompileError> {
    match ty {
        Type::Enum(name) if native_class_types.contains(name) => Ok(()),
        Type::Plugin { namespace, name }
            if plugin_types.contains(&format!("{namespace}.{name}")) =>
        {
            Ok(())
        }
        Type::ComponentRef { namespace, name }
            if (namespace == "Nexa" && matches!(name.as_str(), "FastList" | "BottomSheet"))
                || plugin_types.contains(&format!("{namespace}.{name}.__component_ref")) =>
        {
            Ok(())
        }
        Type::ComponentRef { namespace, name } => Err(CompileError::new(
            span,
            format!("unknown native component ref target `{namespace}.{name}`"),
        )),
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) | Type::Signal(inner) => {
            validate_component_type_names(inner, enum_names, native_class_types, plugin_types, span)
        }
        Type::Map(key, value) | Type::Pair(key, value) => {
            validate_component_type_names(key, enum_names, native_class_types, plugin_types, span)?;
            validate_component_type_names(value, enum_names, native_class_types, plugin_types, span)
        }
        Type::Triple(first, second, third) => {
            validate_component_type_names(
                first,
                enum_names,
                native_class_types,
                plugin_types,
                span,
            )?;
            validate_component_type_names(
                second,
                enum_names,
                native_class_types,
                plugin_types,
                span,
            )?;
            validate_component_type_names(third, enum_names, native_class_types, plugin_types, span)
        }
        _ => validate_type_names(ty, enum_names, plugin_types, span),
    }
}

fn validate_type_names(
    ty: &Type,
    enum_names: &std::collections::HashSet<&str>,
    plugin_types: &HashSet<String>,
    span: nexa_diagnostics::Span,
) -> Result<(), CompileError> {
    match ty {
        Type::TypeParam(_) => Ok(()),
        Type::Enum(name) if !enum_names.contains(name.as_str()) && !is_builtin_enum_name(name) => {
            Err(CompileError::new(span, format!("unknown type `{name}`")))
        }
        Type::Plugin { namespace, name }
            if !plugin_types.contains(&format!("{namespace}.{name}")) =>
        {
            Err(CompileError::new(
                span,
                format!("unknown plugin type `{namespace}.{name}`"),
            ))
        }
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) | Type::Signal(inner) => {
            validate_type_names(inner, enum_names, plugin_types, span)
        }
        Type::ComponentRefError(inner) => {
            validate_type_names(inner, enum_names, plugin_types, span)
        }
        Type::Map(key, value) | Type::Pair(key, value) | Type::Result(key, value) => {
            validate_type_names(key, enum_names, plugin_types, span)?;
            validate_type_names(value, enum_names, plugin_types, span)
        }
        Type::Triple(first, second, third) => {
            validate_type_names(first, enum_names, plugin_types, span)?;
            validate_type_names(second, enum_names, plugin_types, span)?;
            validate_type_names(third, enum_names, plugin_types, span)
        }
        Type::Struct { fields, .. } => fields
            .iter()
            .try_for_each(|(_, field)| validate_type_names(field, enum_names, plugin_types, span)),
        Type::Class { fields, .. } => fields
            .iter()
            .try_for_each(|(_, field)| validate_type_names(field, enum_names, plugin_types, span)),
        Type::Void
        | Type::String
        | Type::Bytes
        | Type::Bool
        | Type::Numeric(_)
        | Type::Enum(_)
        | Type::Plugin { .. }
        | Type::TaskHandle
        | Type::NetworkResponse
        | Type::ComponentRef { .. } => Ok(()),
    }
}

fn is_builtin_enum_name(name: &str) -> bool {
    matches!(name, "Permission" | "PermissionStatus" | "JsonError")
}

#[derive(Clone, Copy)]
struct FunctionLoweringContext<'a> {
    signatures: &'a FunctionSignatures,
    structs: &'a StructTypes,
    enums: &'a HashSet<String>,
    enum_symbols: &'a HashMap<String, (Type, bool)>,
    global_symbols: &'a HashMap<String, (Type, bool)>,
    class_initializers: &'a HashMap<String, Vec<FunctionLocal>>,
    allow_nullable_generic_plugin_reads: bool,
}

/// Reuses typed function bodies across DevRuntime reloads when their source
/// and the declaration environment they resolve against remain unchanged.
#[derive(Default)]
pub(crate) struct DevFunctionLoweringCache {
    environment: Option<FunctionLoweringEnvironment>,
    functions: HashMap<String, CachedFunction>,
    all_functions: Option<Vec<Function>>,
    optimization_facts: HashMap<String, crate::optimize::FunctionOptimizationFacts>,
    unchanged_function_names: HashSet<String>,
    cached_function_sources: Option<HashMap<String, String>>,
    screen_environment: Option<String>,
    screens: HashMap<String, CachedScreen>,
    unchanged_screen_indices: Vec<usize>,
    screen_struct_facts: HashMap<String, HashSet<String>>,
    cached_warnings: Option<Vec<CompileWarning>>,
    program_env: Option<Arc<ProgramEnvironment>>,
    cached_app_body: Option<CachedAppBody>,
}

#[derive(Clone)]
pub(crate) struct CachedAppBody {
    pub(crate) source_snippet: String,
    pub(crate) body: Vec<Node>,
    pub(crate) status_bar: Option<StatusBarConfig>,
    pub(crate) direction: Option<DirectionConfig>,
    pub(crate) on_appear: Option<Vec<Action>>,
    pub(crate) on_appear_async: bool,
    pub(crate) on_disappear: Option<Vec<Action>>,
    pub(crate) on_active: Option<Vec<Action>>,
    pub(crate) on_inactive: Option<Vec<Action>>,
    pub(crate) on_background: Option<Vec<Action>>,
}

#[derive(Clone)]
struct ProgramEnvironment {
    themes: themes::ThemeSymbols,
    enum_declarations: Vec<nexa_ir::EnumDecl>,
    struct_declarations: Vec<nexa_ir::StructDecl>,
    struct_types: StructTypes,
    class_types: HashMap<String, Type>,
    enum_symbols: HashMap<String, (Type, bool)>,
    owned_enum_names: HashSet<String>,
    function_signatures: FunctionSignatures,
    screen_signatures: HashMap<String, ScreenSignature>,
    components: Vec<nexa_ir::Component>,
    component_signatures: custom_components::ComponentSignatures,
    test_suite: crate::testing::TestSuite,
    background_tasks: Vec<BackgroundTask>,
    states: Vec<State>,
    global_names: HashSet<String>,
    symbols: HashMap<String, (Type, bool)>,
    native_aliases: HashMap<String, String>,
    global_symbols: HashMap<String, (Type, bool)>,
    class_initializers: HashMap<String, Vec<FunctionLocal>>,
    class_method_declarations: Vec<ast::FunctionDecl>,
}

#[derive(Clone, PartialEq)]
struct FunctionLoweringEnvironment {
    signatures: FunctionSignatures,
    structs: StructTypes,
    enums: HashSet<String>,
    enum_symbols: HashMap<String, (Type, bool)>,
    global_symbols: HashMap<String, (Type, bool)>,
    class_initializers: HashMap<String, Vec<FunctionLocal>>,
}

struct CachedFunction {
    source: String,
    function: Option<Function>,
}

#[derive(Clone)]
struct CachedScreen {
    source: String,
    screen: Screen,
    struct_names: HashSet<String>,
}

impl DevFunctionLoweringCache {
    pub(crate) fn change_hint(&self, module: &Module) -> crate::DevChangeHint {
        crate::DevChangeHint {
            unchanged_function_indices: module
                .functions
                .iter()
                .enumerate()
                .filter_map(|(index, function)| {
                    self.unchanged_function_names
                        .contains(&function.name)
                        .then_some(index)
                })
                .collect(),
            unchanged_screen_indices: self.unchanged_screen_indices.clone(),
        }
    }

    fn update_cached_functions(&mut self, module: &Module) {
        if self.unchanged_function_names.len() == module.functions.len() {
            return;
        }
        let all_module_names: HashSet<&str> =
            module.functions.iter().map(|f| f.name.as_str()).collect();
        let dirty_names: HashSet<&str> = module
            .functions
            .iter()
            .filter_map(|f| {
                (!self.unchanged_function_names.contains(&f.name)).then_some(f.name.as_str())
            })
            .collect();
        let module_dirty_functions = module
            .functions
            .iter()
            .filter(|f| dirty_names.contains(f.name.as_str()))
            .map(|f| (f.name.as_str(), f))
            .collect::<HashMap<_, _>>();
        for (name, cached) in &mut self.functions {
            if let Some(function) = module_dirty_functions.get(name.as_str()) {
                cached.function = Some((*function).clone());
            }
        }
        self.functions
            .retain(|name, _| all_module_names.contains(name.as_str()));
        self.unchanged_function_names
            .retain(|name| all_module_names.contains(name.as_str()));
        self.optimization_facts
            .retain(|name, _| all_module_names.contains(name.as_str()));
        let declared_functions = self
            .functions
            .keys()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        for name in dirty_names {
            if let Some(cached) = self.functions.get(name)
                && let Some(function) = cached.function.as_ref()
            {
                self.optimization_facts.insert(
                    name.to_string(),
                    crate::optimize::function_optimization_facts(function, &declared_functions),
                );
            }
        }
        self.all_functions = Some(module.functions.clone());
    }
}

fn screen_lowering_environment_key(
    app: &ast::App,
    target: Target,
    allow_nullable_generic_plugin_reads: bool,
    custom_icons: Option<&HashMap<String, nexa_ir::SystemIcon>>,
) -> String {
    let function_signatures = app
        .functions
        .iter()
        .map(|function| {
            let parameters = function
                .parameters
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.as_str(),
                        type_syntax_cache_key(&parameter.ty),
                    )
                })
                .collect::<Vec<_>>();
            (
                function.name.as_str(),
                function.is_async,
                function.is_throwing,
                function.failure_type.as_ref().map(type_syntax_cache_key),
                parameters,
                type_syntax_cache_key(&function.return_type),
            )
        })
        .collect::<Vec<_>>();
    let screen_signatures = app
        .screens
        .iter()
        .map(|screen| {
            let parameters = screen
                .parameters
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.as_str(),
                        type_syntax_cache_key(&parameter.ty),
                    )
                })
                .collect::<Vec<_>>();
            (screen.name.as_str(), parameters)
        })
        .collect::<Vec<_>>();
    let custom_icons = custom_icons
        .map(|icons| {
            let mut icons = icons.iter().collect::<Vec<_>>();
            icons.sort_by(|left, right| left.0.cmp(right.0));
            format!("{icons:?}")
        })
        .unwrap_or_default();
    format!(
        "{:?}",
        (
            (
                target,
                allow_nullable_generic_plugin_reads,
                &app.name,
                &app.plugins,
                &app.enums,
                &app.structs,
                &app.classes,
                &app.states,
                &app.globals,
                function_signatures,
            ),
            (
                screen_signatures,
                &app.components,
                &app.theme,
                &app.body,
                custom_icons,
            ),
        )
    )
}

fn type_syntax_cache_key(ty: &ast::TypeSyntax) -> String {
    match ty {
        ast::TypeSyntax::Named(name, _) => name.clone(),
        ast::TypeSyntax::Generic(name, arguments, _) => format!(
            "{name}<{}>",
            arguments
                .iter()
                .map(type_syntax_cache_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ast::TypeSyntax::Optional(inner, _) => format!("{}?", type_syntax_cache_key(inner)),
    }
}

fn lower_functions(
    declarations: Vec<ast::FunctionDecl>,
    context: FunctionLoweringContext<'_>,
    is_warm_decl: bool,
    mut dev_cache: Option<(&mut DevFunctionLoweringCache, &HashMap<String, String>)>,
) -> Result<Vec<Function>, CompileError> {
    if is_warm_decl
        && let Some((cache, sources)) = &mut dev_cache
        && cache.cached_function_sources.as_ref() == Some(*sources)
        && let Some(all_funcs) = &cache.all_functions
    {
        cache.unchanged_function_names = all_funcs.iter().map(|f| f.name.clone()).collect();
        eprintln!(
            "Nexa semantic function cache: fast batch hit ({} functions)",
            all_funcs.len()
        );
        return Ok(all_funcs.clone());
    }

    let (environment, can_reuse) = if let Some((cache, _)) = dev_cache.as_ref() {
        if is_warm_decl {
            (cache.environment.clone(), true)
        } else {
            let env = FunctionLoweringEnvironment {
                signatures: context.signatures.clone(),
                structs: context.structs.clone(),
                enums: context.enums.clone(),
                enum_symbols: context.enum_symbols.clone(),
                global_symbols: context.global_symbols.clone(),
                class_initializers: context.class_initializers.clone(),
            };
            let matches = cache.environment.as_ref() == Some(&env);
            (Some(env), matches)
        }
    } else {
        (None, false)
    };
    if !can_reuse && let Some((cache, _)) = dev_cache.as_mut() {
        cache.functions.clear();
        cache.optimization_facts.clear();
        cache.all_functions = None;
    }

    let mut lowered = Vec::with_capacity(declarations.len());
    let mut cache_hit_names = HashSet::new();
    let mut cache_hits = 0usize;

    if let Some((cache, sources)) = dev_cache {
        for declaration in declarations {
            let name = declaration.name.clone();
            let source = sources
                .get(&name)
                .cloned()
                .unwrap_or_else(|| format!("{declaration:?}"));

            let hit = if can_reuse {
                cache.functions.get(&name).and_then(|c| {
                    if c.source == source {
                        c.function.clone()
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            let function = match hit {
                Some(func) => {
                    cache_hits += 1;
                    cache_hit_names.insert(name);
                    func
                }
                None => {
                    let func = lower_function(declaration, context)?;
                    cache.functions.insert(
                        name.clone(),
                        CachedFunction {
                            source,
                            function: Some(func.clone()),
                        },
                    );
                    cache.optimization_facts.remove(&name);
                    func
                }
            };
            lowered.push(function);
        }

        let current_names: HashSet<&str> = lowered.iter().map(|f| f.name.as_str()).collect();
        cache.functions.retain(|name, _| current_names.contains(name.as_str()));
        cache.cached_function_sources = Some((*sources).clone());
        cache.environment = environment;
        cache.unchanged_function_names = cache_hit_names;
        eprintln!(
            "Nexa semantic function cache: {cache_hits} reused, {} lowered",
            lowered.len().saturating_sub(cache_hits)
        );
    } else {
        for declaration in declarations {
            lowered.push(lower_function(declaration, context)?);
        }
    }
    Ok(lowered)
}

fn lower_function(
    declaration: ast::FunctionDecl,
    context: FunctionLoweringContext<'_>,
) -> Result<Function, CompileError> {
    let FunctionLoweringContext {
        signatures,
        structs,
        enums,
        enum_symbols,
        global_symbols,
        class_initializers,
        allow_nullable_generic_plugin_reads,
    } = context;
    let signature = signatures.get(&declaration.name).ok_or_else(|| {
        CompileError::new(
            declaration.span,
            format!(
                "function signature for `{}` was not collected",
                declaration.name
            ),
        )
    })?;
    let symbols = signature
        .parameters
        .iter()
        .map(|(name, ty)| (name.clone(), (ty.clone(), false)))
        .collect::<HashMap<_, _>>();
    let mut symbols = symbols;
    if let Some(receiver @ Type::Class { fields, .. }) = signature.receiver.as_ref() {
        symbols.insert("this".to_owned(), (receiver.clone(), false));
        for (name, ty) in fields {
            symbols
                .entry(name.clone())
                .or_insert_with(|| (ty.clone(), false));
        }
    }
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
    let scoped_signatures = signature
        .is_throwing
        .then(|| functions_with_error_handling(signatures, true));
    let lowering_signatures = scoped_signatures.as_ref().unwrap_or(signatures);
    if !has_simple_function_body(&declaration.body) {
        let (body_actions, returns) = lower_function_statements(
            declaration.body.clone(),
            &symbols,
            lowering_signatures,
            &signature.return_type,
            signature.is_async,
            structs,
            enums,
            allow_nullable_generic_plugin_reads,
            declaration.span,
            declaration.source_file.as_deref(),
            0,
        )?;
        if !returns && signature.return_type != Type::Void {
            return Err(CompileError::new(
                declaration.span,
                format!(
                    "function `{}` must return a value of type `{}` on every path",
                    declaration.name,
                    type_name(&signature.return_type)
                ),
            ));
        }
        return Ok(Function {
            class_initializers: signature
                .receiver
                .as_ref()
                .and_then(|_| declaration.name.split_once('.'))
                .and_then(|(class, _)| class_initializers.get(class).cloned())
                .unwrap_or_default(),
            name: declaration.name,
            receiver: signature.receiver.clone(),
            is_async: signature.is_async,
            is_throwing: signature.is_throwing,
            failure_type: None,
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
        });
    }
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
                    &ExprContext::with_types(
                        &symbols,
                        lowering_signatures,
                        signature.is_async,
                        structs,
                        enums,
                    )
                    .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads)
                    .with_source_file(declaration.source_file.as_deref()),
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
            | ast::Stmt::TaskLaunch { span, .. }
            | ast::Stmt::TaskCancel { span, .. }
            | ast::Stmt::WithAnimation { span, .. }
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
        &ExprContext::with_types(
            &symbols,
            lowering_signatures,
            signature.is_async,
            structs,
            enums,
        )
        .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads)
        .with_source_file(declaration.source_file.as_deref()),
    )?;
    Ok(Function {
        class_initializers: signature
            .receiver
            .as_ref()
            .and_then(|_| declaration.name.split_once('.'))
            .and_then(|(class, _)| class_initializers.get(class).cloned())
            .unwrap_or_default(),
        name: declaration.name,
        receiver: signature.receiver.clone(),
        is_async: signature.is_async,
        is_throwing: signature.is_throwing,
        failure_type: None,
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
        body_actions: None,
    })
}

fn has_simple_function_body(statements: &[ast::Stmt]) -> bool {
    let Some(ast::Stmt::Return { .. }) = statements.last() else {
        return false;
    };
    statements[..statements.len() - 1]
        .iter()
        .all(|statement| matches!(statement, ast::Stmt::Let { .. }))
}

fn function_expression_context<'a>(
    symbols: &'a HashMap<String, (Type, bool)>,
    functions: &'a FunctionSignatures,
    structs: &'a StructTypes,
    enums: &'a HashSet<String>,
    allow_await: bool,
    allow_nullable_generic_plugin_reads: bool,
    source_file: Option<&'a str>,
) -> ExprContext<'a> {
    ExprContext::with_types(symbols, functions, allow_await, structs, enums)
        .with_nullable_generic_plugin_reads(allow_nullable_generic_plugin_reads)
        .with_source_file(source_file)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::semantic) fn lower_function_statements(
    statements: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    return_type: &Type,
    allow_await: bool,
    structs: &StructTypes,
    enums: &HashSet<String>,
    allow_nullable_generic_plugin_reads: bool,
    function_span: nexa_diagnostics::Span,
    source_file: Option<&str>,
    loop_depth: usize,
) -> Result<(Vec<Action>, bool), CompileError> {
    let mut symbols = symbols.clone();
    let mut lowered = Vec::with_capacity(statements.len());
    let mut definitely_returns = false;
    for statement in statements {
        if definitely_returns {
            return Err(CompileError::new(
                function_span,
                "unreachable statement after a function body that always returns",
            ));
        }
        match statement {
            ast::Stmt::Let {
                name,
                ty,
                initial,
                span,
            } => {
                if symbols.contains_key(&name) {
                    return Err(CompileError::new(
                        span,
                        format!("local constant `{name}` is already declared"),
                    ));
                }
                let local_type =
                    resolve_value_type(&name, ty.as_ref(), &initial, &symbols, functions, structs)?;
                let value = lower_expr(
                    &initial,
                    Some(&local_type),
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                symbols.insert(name.clone(), (local_type.clone(), false));
                lowered.push(Action::Let {
                    name,
                    ty: local_type,
                    value,
                });
            }
            ast::Stmt::Expression { expression, .. } => {
                let value = lower_expr(
                    &expression,
                    None,
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                lowered.push(Action::Expression(value));
            }
            ast::Stmt::Assign { name, value, span } => {
                let Some((ty, mutable)) = symbols.get(&name) else {
                    return Err(CompileError::new(
                        span,
                        format!("unknown value `{name}` in function body"),
                    ));
                };
                if !mutable {
                    return Err(CompileError::new(
                        span,
                        format!("`{name}` is immutable and cannot be assigned in a function body"),
                    ));
                }
                let value = lower_expr(
                    &value,
                    Some(ty),
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                lowered.push(Action::Assign { name, value });
            }
            ast::Stmt::CollectionMutation {
                name,
                method,
                type_arguments,
                arguments,
                span,
            } => {
                let Some((receiver_type, _)) = symbols.get(&name) else {
                    return Err(CompileError::new(
                        span,
                        format!("unknown value `{name}` in function body"),
                    ));
                };
                if !matches!(receiver_type, Type::Plugin { .. } | Type::Class { .. }) {
                    return Err(CompileError::new(
                        span,
                        "collection mutation statements are only supported for class or plugin method calls in functions",
                    ));
                }
                let call = ast::Expr::MethodCall {
                    base: Box::new(ast::Expr::Name(name, span)),
                    name: method,
                    type_arguments,
                    arguments,
                    named_arguments: std::collections::BTreeMap::new(),
                    span,
                };
                let value = lower_expr(
                    &call,
                    None,
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                lowered.push(Action::Expression(value));
            }
            ast::Stmt::Return { value, .. } => {
                let value = lower_expr(
                    &value,
                    Some(return_type),
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                lowered.push(Action::Return { value });
                definitely_returns = true;
            }
            ast::Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                let (then_branch, then_returns) = lower_function_statements(
                    then_branch,
                    &symbols,
                    functions,
                    return_type,
                    allow_await,
                    structs,
                    enums,
                    allow_nullable_generic_plugin_reads,
                    function_span,
                    source_file,
                    loop_depth,
                )?;
                let (else_branch, else_returns) = else_branch
                    .map(|branch| {
                        lower_function_statements(
                            branch,
                            &symbols,
                            functions,
                            return_type,
                            allow_await,
                            structs,
                            enums,
                            allow_nullable_generic_plugin_reads,
                            function_span,
                            source_file,
                            loop_depth,
                        )
                    })
                    .transpose()?
                    .map_or((None, false), |(branch, returns)| (Some(branch), returns));
                definitely_returns = then_returns && else_returns;
                lowered.push(Action::If {
                    condition,
                    then_branch,
                    else_branch,
                });
            }
            ast::Stmt::For {
                name,
                iterable,
                body,
                span,
            } => {
                if symbols.contains_key(&name) {
                    return Err(CompileError::new(
                        span,
                        format!("loop binding `{name}` shadows an existing binding"),
                    ));
                }
                let (iterable, element_type) = lower_for_iterable(
                    &iterable,
                    &symbols,
                    functions,
                    allow_await,
                    TypeRegistries {
                        structs,
                        enums,
                        allow_nullable_generic_plugin_reads,
                        screen_ids: None,
                        allow_navigation_actions: false,
                        source_file,
                    },
                    span,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(name.clone(), (element_type, false));
                let (body, _) = lower_function_statements(
                    body,
                    &loop_symbols,
                    functions,
                    return_type,
                    allow_await,
                    structs,
                    enums,
                    allow_nullable_generic_plugin_reads,
                    function_span,
                    source_file,
                    loop_depth + 1,
                )?;
                lowered.push(Action::For {
                    name,
                    iterable,
                    body,
                });
            }
            ast::Stmt::ForMap {
                key_name,
                value_name,
                iterable,
                body,
                span,
            } => {
                if key_name == value_name
                    || symbols.contains_key(&key_name)
                    || symbols.contains_key(&value_name)
                {
                    return Err(CompileError::new(
                        span,
                        "map loop bindings must be distinct and cannot shadow an existing binding",
                    ));
                }
                let (iterable, key_type, value_type) = lower_map_iterable(
                    &iterable,
                    &symbols,
                    functions,
                    allow_await,
                    TypeRegistries {
                        structs,
                        enums,
                        allow_nullable_generic_plugin_reads,
                        screen_ids: None,
                        allow_navigation_actions: false,
                        source_file,
                    },
                    span,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(key_name.clone(), (key_type, false));
                loop_symbols.insert(value_name.clone(), (value_type, false));
                let (body, _) = lower_function_statements(
                    body,
                    &loop_symbols,
                    functions,
                    return_type,
                    allow_await,
                    structs,
                    enums,
                    allow_nullable_generic_plugin_reads,
                    function_span,
                    source_file,
                    loop_depth + 1,
                )?;
                lowered.push(Action::ForMap {
                    key_name,
                    value_name,
                    iterable,
                    body,
                });
            }
            ast::Stmt::While {
                condition,
                body,
                span: _,
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    &function_expression_context(
                        &symbols,
                        functions,
                        structs,
                        enums,
                        allow_await,
                        allow_nullable_generic_plugin_reads,
                        source_file,
                    ),
                )?;
                let (body, _) = lower_function_statements(
                    body,
                    &symbols,
                    functions,
                    return_type,
                    allow_await,
                    structs,
                    enums,
                    allow_nullable_generic_plugin_reads,
                    function_span,
                    source_file,
                    loop_depth + 1,
                )?;
                lowered.push(Action::While { condition, body });
            }
            ast::Stmt::Break { span } => {
                if loop_depth == 0 {
                    return Err(CompileError::new(
                        span,
                        "`break` is only allowed inside a loop",
                    ));
                }
                lowered.push(Action::Break);
            }
            ast::Stmt::Continue { span } => {
                if loop_depth == 0 {
                    return Err(CompileError::new(
                        span,
                        "`continue` is only allowed inside a loop",
                    ));
                }
                lowered.push(Action::Continue);
            }
            ast::Stmt::TryCatch {
                body,
                error_catches,
                catch_body,
                span,
            } => {
                let handled_functions = functions_with_error_handling(functions, true);
                let (body, body_returns) = lower_function_statements(
                    body,
                    &symbols,
                    &handled_functions,
                    return_type,
                    allow_await,
                    structs,
                    enums,
                    allow_nullable_generic_plugin_reads,
                    function_span,
                    source_file,
                    loop_depth,
                )?;
                let mut lowered_catches = Vec::with_capacity(error_catches.len());
                let mut catches_return = true;
                for arm in error_catches {
                    let variant = plugin_error_variant(
                        functions,
                        &arm.namespace,
                        &arm.error_name,
                        &arm.variant,
                    )
                    .ok_or_else(|| {
                        CompileError::new(
                            arm.span,
                            format!(
                                "unknown plugin error variant `{}.{}.{}`",
                                arm.namespace, arm.error_name, arm.variant
                            ),
                        )
                    })?;
                    if variant.parameters.len() != arm.bindings.len() {
                        return Err(CompileError::new(
                            arm.span,
                            format!(
                                "error variant `{}.{}` provides {} payload value(s), but the catch case binds {}",
                                arm.error_name,
                                arm.variant,
                                variant.parameters.len(),
                                arm.bindings.len()
                            ),
                        ));
                    }
                    let mut arm_symbols = symbols.clone();
                    let parameters = variant
                        .parameters
                        .iter()
                        .zip(&arm.bindings)
                        .map(|((payload_name, payload_type), binding)| {
                            (binding.clone(), payload_name.clone(), payload_type.clone())
                        })
                        .collect::<Vec<_>>();
                    for (binding, _, ty) in &parameters {
                        if arm_symbols
                            .insert(binding.clone(), (ty.clone(), false))
                            .is_some()
                        {
                            return Err(CompileError::new(
                                arm.span,
                                format!(
                                    "catch payload `{binding}` conflicts with an existing value"
                                ),
                            ));
                        }
                    }
                    let (actions, returns) = lower_function_statements(
                        arm.body,
                        &arm_symbols,
                        functions,
                        return_type,
                        allow_await,
                        structs,
                        enums,
                        allow_nullable_generic_plugin_reads,
                        function_span,
                        source_file,
                        loop_depth,
                    )?;
                    catches_return &= returns;
                    lowered_catches.push(nexa_ir::ErrorCatchArm {
                        namespace: arm.namespace,
                        error_type: arm.error_name,
                        variant: arm.variant,
                        parameters,
                        body: actions,
                    });
                }
                let (catch_body, catch_returns) = catch_body
                    .map(|branch| {
                        lower_function_statements(
                            branch,
                            &symbols,
                            functions,
                            return_type,
                            allow_await,
                            structs,
                            enums,
                            allow_nullable_generic_plugin_reads,
                            function_span,
                            source_file,
                            loop_depth,
                        )
                    })
                    .transpose()?
                    .map_or((None, true), |(branch, returns)| (Some(branch), returns));
                validate_typed_error_recovery(
                    &body,
                    &lowered_catches,
                    catch_body.is_some(),
                    functions,
                    span,
                )?;
                definitely_returns = body_returns && catches_return && catch_returns;
                lowered.push(Action::TryCatch {
                    body,
                    error_catches: lowered_catches,
                    catch_body,
                });
            }
            unsupported => {
                return Err(CompileError::new(
                    function_span,
                    format!(
                        "function bodies currently support `let`, expression, `return`, `if`, and `try/catch`; found unsupported `{}`",
                        function_statement_name(&unsupported)
                    ),
                ));
            }
        }
    }
    Ok((lowered, definitely_returns))
}

fn function_statement_name(statement: &ast::Stmt) -> &'static str {
    match statement {
        ast::Stmt::Expression { .. } => "expression",
        ast::Stmt::Let { .. } => "let",
        ast::Stmt::Assign { .. } => "assignment",
        ast::Stmt::NativePropertyAssign { .. } => "native property assignment",
        ast::Stmt::NativeEventSubscribe { .. } => "native event subscription",
        ast::Stmt::CollectionMutation { .. } => "collection mutation",
        ast::Stmt::TaskLaunch { .. } => "task launch",
        ast::Stmt::TaskCancel { .. } => "task cancellation",
        ast::Stmt::WithAnimation { .. } => "animation block",
        ast::Stmt::If { .. } => "if",
        ast::Stmt::For { .. } => "for loop",
        ast::Stmt::ForMap { .. } => "map loop",
        ast::Stmt::While { .. } => "while loop",
        ast::Stmt::TryCatch { .. } => "try/catch",
        ast::Stmt::Break { .. } => "break",
        ast::Stmt::Continue { .. } => "continue",
        ast::Stmt::Return { .. } => "return",
    }
}

fn has_navigation_root(nodes: &[ast::Node], target: Target) -> bool {
    let mut active = Vec::new();
    collect_active_nodes(nodes, target, &mut active);
    match active.as_slice() {
        [ast::Node::ComponentInvocation(inv)] => inv.name == "NavigationStack",
        [
            ast::Node::If {
                then_body,
                else_body: Some(else_body),
                ..
            },
        ] => has_navigation_root(then_body, target) && has_navigation_root(else_body, target),
        [
            ast::Node::When {
                cases,
                else_body,
                has_else: true,
                ..
            },
        ] => {
            !cases.is_empty()
                && cases
                    .iter()
                    .all(|case| has_navigation_root(&case.body, target))
                && has_navigation_root(else_body, target)
        }
        _ => false,
    }
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

/// The appearance hook a screen body declared.
///
/// A screen may declare `OnAppear` and `OnDisappear` among its nodes, but both
/// attach to the view itself on each platform, so they lift out of the body.
/// This carries the actions to run, whether the appearance hook is async, and
/// the body with both hooks removed.
type AppearanceHooks = (Option<Vec<Action>>, bool, Vec<Node>);

fn extract_on_appear(
    nodes: Vec<Node>,
    span: nexa_diagnostics::Span,
    scope: &str,
) -> Result<AppearanceHooks, CompileError> {
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
            codecs: Vec::new(),
            return_type: Type::Void,
            source_span: None,
            is_async: false,
            is_throwing: false,
        }
    }

    fn fixture(states: Vec<State>, callback_use: &str) -> (Module, FunctionSignatures) {
        let module = Module {
            widgets: Vec::new(),
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states,
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::Button {
                label: Expr::String("Dispose".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                accessibility_label: None,
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
            is_throwing: false,
            is_constructor: true,
        }
    }

    fn disposable_functions() -> FunctionSignatures {
        HashMap::from([(
            "VideoPlayer.dispose".to_owned(),
            FunctionSignature {
                type_parameters: Vec::new(),
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
            widgets: Vec::new(),
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![state("player", constructor())],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![Node::NativeComponentCall {
                namespace: "Video".to_owned(),
                name: "VideoView".to_owned(),
                arguments: Vec::new(),
                component_ref: None,
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
            widgets: Vec::new(),
            app_name: "Test".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            globals: Vec::new(),
            states: vec![state("player", constructor())],
            screens: Vec::new(),
            components: Vec::new(),
            body: vec![
                Node::Button {
                    label: Expr::String("Dispose".to_owned()),
                    icon: None,
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    accessibility_label: None,
                    actions: vec![Action::Expression(player_call("dispose", "player"))],
                },
                Node::ComponentCall {
                    name: "PlayerSurface".to_owned(),
                    arguments: vec![(
                        "player".to_owned(),
                        Expr::State("player".to_owned(), player_type()),
                    )],
                    component_ref: None,
                    children: None,
                },
                Node::NativeComponentCall {
                    namespace: "Video".to_owned(),
                    name: "VideoView".to_owned(),
                    arguments: vec![(
                        "player".to_owned(),
                        Expr::State("player".to_owned(), player_type()),
                    )],
                    component_ref: None,
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
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    accessibility_label: None,
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
                style: None,
                size: None,
                shape: None,
                tint: None,
                glass: false,
                accessibility_label: None,
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
