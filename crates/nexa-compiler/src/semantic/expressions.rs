use std::collections::{BTreeMap, HashMap, HashSet};

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{
    ArithmeticOp as IrArithmeticOp, BinaryOp, CollectionTransform, CollectionUtilityKind, Expr,
    InterpolatedPart, MemberKind, NumericType, PluginCodec, TuplePosition, Type,
};
use nexa_plugin_idl::TypeRef;
use nexa_syntax::ast;

use super::context::{ExprContext, TypeRegistries};

#[derive(Clone)]
pub(super) struct FunctionSignature {
    pub(super) parameters: Vec<(String, Type)>,
    /// Declared plugin value type parameters, in declaration order. A call
    /// binds every one of them to a concrete value type.
    pub(super) type_parameters: Vec<String>,
    pub(super) return_type: Type,
    pub(super) is_async: bool,
    pub(super) is_throwing: bool,
    pub(super) receiver: Option<Type>,
    pub(super) is_constructor: bool,
    pub(super) is_mutable_property: bool,
    pub(super) error_handling_allowed: bool,
    pub(super) error_type: Option<PluginErrorType>,
}

#[derive(Clone)]
pub(super) struct PluginErrorType {
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) variants: Vec<PluginErrorVariant>,
}

#[derive(Clone)]
pub(super) struct PluginErrorVariant {
    pub(super) name: String,
    pub(super) parameters: Vec<(String, Type)>,
}

pub(super) type FunctionSignatures = HashMap<String, FunctionSignature>;
pub(super) type StructTypes = HashMap<String, Type>;
const ERROR_HANDLING_SCOPE_KEY: &str = "\0nexa_error_handling_scope";

#[derive(Clone)]
pub(super) struct PluginComponentSignature {
    pub(super) namespace: String,
    pub(super) name: String,
    pub(super) has_content_slot: bool,
    pub(super) parameters: Vec<(String, Type)>,
    pub(super) defaults: HashMap<String, ast::Expr>,
    pub(super) events: Vec<PluginComponentEventSignature>,
}

#[derive(Clone)]
pub(super) struct PluginComponentEventSignature {
    pub(super) property: String,
    pub(super) parameters: Vec<(String, Type)>,
}

pub(super) fn collect_plugin_components(
    plugins: &[ast::PluginDecl],
) -> Result<Vec<PluginComponentSignature>, CompileError> {
    let mut components = Vec::new();
    for plugin in plugins {
        if plugin.pure {
            continue;
        }
        let Some(idl) = plugin.idl.as_ref() else {
            return Err(CompileError::new(
                plugin.span,
                format!("plugin `{}` has not been resolved", plugin.namespace),
            ));
        };
        for interface in &idl.interfaces {
            if !matches!(
                interface.kind,
                nexa_plugin_idl::InterfaceKind::NativeComponent
            ) {
                continue;
            }
            if !interface.constructors.is_empty() || !interface.methods.is_empty() {
                return Err(CompileError::new(
                    plugin.span,
                    format!(
                        "native component `{}.{}` may declare properties and events only",
                        plugin.namespace, interface.name
                    ),
                ));
            }
            let parameters = interface
                .properties
                .iter()
                .map(|property| {
                    plugin_type(&plugin.namespace, &property.ty, false, &[])
                        .map(|ty| (property.name.clone(), ty))
                        .map_err(|message| CompileError::new(plugin.span, message))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let defaults = interface
                .properties
                .iter()
                .filter_map(|property| {
                    property
                        .default
                        .as_ref()
                        .map(|value| (property.name.clone(), idl_literal_expr(value, plugin.span)))
                })
                .collect();
            let events = interface
                .events
                .iter()
                .map(|event| {
                    let parameters = event
                        .parameters
                        .iter()
                        .map(|parameter| {
                            plugin_type(&plugin.namespace, &parameter.ty, false, &[])
                                .map(|ty| (parameter.name.clone(), ty))
                                .map_err(|message| CompileError::new(plugin.span, message))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(PluginComponentEventSignature {
                        property: nexa_plugin_idl::event_callback_property(&event.name),
                        parameters,
                    })
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            components.push(PluginComponentSignature {
                namespace: plugin.namespace.clone(),
                name: interface.name.clone(),
                has_content_slot: interface.has_content_slot,
                parameters,
                defaults,
                events,
            });
        }
    }
    Ok(components)
}

fn idl_literal_expr(value: &nexa_plugin_idl::Literal, span: Span) -> ast::Expr {
    match value {
        nexa_plugin_idl::Literal::String(value) => ast::Expr::String(value.clone(), span),
        nexa_plugin_idl::Literal::Number(value) => ast::Expr::Number(value.clone(), span),
        nexa_plugin_idl::Literal::Bool(value) => ast::Expr::Bool(*value, span),
        nexa_plugin_idl::Literal::Null => ast::Expr::Null(span),
    }
}

pub(super) fn collect_plugin_signatures(
    plugins: &[ast::PluginDecl],
) -> Result<FunctionSignatures, CompileError> {
    let mut signatures = HashMap::new();
    for plugin in plugins {
        if plugin.pure {
            continue;
        }
        if is_core_native_namespace(&plugin.namespace) {
            return Err(CompileError::new(
                plugin.span,
                format!(
                    "plugin namespace `{}` is reserved for Nexa native APIs",
                    plugin.namespace
                ),
            ));
        }
        let Some(idl) = plugin.idl.as_ref() else {
            return Err(CompileError::new(
                plugin.span,
                format!("plugin `{}` has not been resolved", plugin.namespace),
            ));
        };
        if idl.interfaces.is_empty() {
            return Err(CompileError::new(
                plugin.span,
                format!(
                    "plugin `{}` does not declare a native interface",
                    plugin.namespace
                ),
            ));
        }
        for interface in &idl.interfaces {
            let native_class =
                matches!(interface.kind, nexa_plugin_idl::InterfaceKind::NativeClass);
            let native_component = matches!(
                interface.kind,
                nexa_plugin_idl::InterfaceKind::NativeComponent
            );
            if native_component {
                continue;
            }
            if !native_class && interface.name != plugin.namespace {
                return Err(CompileError::new(
                    plugin.span,
                    format!(
                        "plugin interface `{}` is not exported as `{}`; use the plugin alias as the service name",
                        interface.name, plugin.namespace
                    ),
                ));
            }
            if native_class {
                if interface.constructors.len() > 1 {
                    return Err(CompileError::new(
                        plugin.span,
                        format!(
                            "native class `{}` declares multiple constructors; Nexa requires one constructor",
                            interface.name
                        ),
                    ));
                }
                let constructor = interface.constructors.first();
                let parameters = constructor
                    .map(|constructor| {
                        constructor
                            .parameters
                            .iter()
                            .map(|parameter| {
                                plugin_type(&plugin.namespace, &parameter.ty, false, &[])
                                    .map(|ty| (parameter.name.clone(), ty))
                                    .map_err(|message| CompileError::new(plugin.span, message))
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .unwrap_or_default();
                let class_type = Type::Plugin {
                    namespace: plugin.namespace.clone(),
                    name: interface.name.clone(),
                };
                let qualified_key = format!("{}.{}", plugin.namespace, interface.name);
                let signature = FunctionSignature {
                    parameters,
                    type_parameters: Vec::new(),
                    return_type: class_type.clone(),
                    is_async: false,
                    is_throwing: false,
                    receiver: None,
                    is_constructor: true,
                    is_mutable_property: false,
                    error_handling_allowed: false,
                    error_type: None,
                };
                if signatures
                    .insert(qualified_key.clone(), signature.clone())
                    .is_some()
                {
                    return Err(CompileError::new(
                        plugin.span,
                        format!(
                            "native class constructor `{qualified_key}` is declared more than once"
                        ),
                    ));
                }
                // Preserve the concise `VideoPlayer()` spelling when the
                // plugin alias and class name are identical.
                if interface.name == plugin.namespace {
                    signatures.insert(interface.name.clone(), signature);
                }
            }
            for method in &interface.methods {
                if native_class
                    && method.name == "dispose"
                    && (method.is_async
                        || !method.parameters.is_empty()
                        || method.return_type.name != "Void"
                        || method.throws.is_some())
                {
                    return Err(CompileError::new(
                        plugin.span,
                        format!(
                            "native class `{}.{}` must be synchronous, non-throwing, return `Void`, and take no parameters",
                            interface.name, method.name
                        ),
                    ));
                }
                let key = if native_class {
                    format!("{}.{}", interface.name, method.name)
                } else {
                    format!("{}.{}", plugin.namespace, method.name)
                };
                if signatures.contains_key(&key) {
                    return Err(CompileError::new(
                        plugin.span,
                        format!("plugin method `{key}` is declared more than once"),
                    ));
                }
                let return_type = plugin_type(
                    &plugin.namespace,
                    &method.return_type,
                    true,
                    &method.type_parameters,
                )
                .map_err(|message| CompileError::new(plugin.span, message))?;
                let parameters = method
                    .parameters
                    .iter()
                    .map(|parameter| {
                        plugin_type(
                            &plugin.namespace,
                            &parameter.ty,
                            false,
                            &method.type_parameters,
                        )
                        .map(|ty| (parameter.name.clone(), ty))
                        .map_err(|message| CompileError::new(plugin.span, message))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let error_reference = method.throws.as_ref().or_else(|| {
                    (method.return_type.name == "Result")
                        .then(|| method.return_type.arguments.get(1))
                        .flatten()
                });
                let error_type = error_reference
                    .map(|reference| plugin_error_type(&plugin.namespace, idl, reference))
                    .transpose()
                    .map_err(|message| CompileError::new(plugin.span, message))?;
                signatures.insert(
                    key,
                    FunctionSignature {
                        parameters,
                        type_parameters: method.type_parameters.clone(),
                        return_type,
                        is_async: method.is_async,
                        is_throwing: method.return_type.name == "Result" || method.throws.is_some(),
                        receiver: native_class.then(|| Type::Plugin {
                            namespace: plugin.namespace.clone(),
                            name: interface.name.clone(),
                        }),
                        is_constructor: false,
                        is_mutable_property: false,
                        error_handling_allowed: false,
                        error_type,
                    },
                );
            }
            if native_class {
                for property in &interface.properties {
                    let key = format!("{}.#property.{}", interface.name, property.name);
                    if signatures.contains_key(&key) {
                        return Err(CompileError::new(
                            plugin.span,
                            format!(
                                "native class property `{}` is declared more than once",
                                property.name
                            ),
                        ));
                    }
                    let return_type = plugin_type(&plugin.namespace, &property.ty, false, &[])
                        .map_err(|message| CompileError::new(plugin.span, message))?;
                    signatures.insert(
                        key,
                        FunctionSignature {
                            parameters: Vec::new(),
                            type_parameters: Vec::new(),
                            return_type,
                            is_async: false,
                            is_throwing: false,
                            receiver: Some(Type::Plugin {
                                namespace: plugin.namespace.clone(),
                                name: interface.name.clone(),
                            }),
                            is_constructor: false,
                            is_mutable_property: property.mutable,
                            error_handling_allowed: false,
                            error_type: None,
                        },
                    );
                }
                for event in &interface.events {
                    let key = format!("{}.#event.{}", interface.name, event.name);
                    if signatures.contains_key(&key) {
                        return Err(CompileError::new(
                            plugin.span,
                            format!(
                                "native class event `{}.{}` is declared more than once",
                                interface.name, event.name
                            ),
                        ));
                    }
                    let parameters = event
                        .parameters
                        .iter()
                        .map(|parameter| {
                            plugin_type(&plugin.namespace, &parameter.ty, false, &[])
                                .map(|ty| (parameter.name.clone(), ty))
                                .map_err(|message| CompileError::new(plugin.span, message))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    signatures.insert(
                        key,
                        FunctionSignature {
                            parameters,
                            type_parameters: Vec::new(),
                            return_type: Type::Void,
                            is_async: false,
                            is_throwing: false,
                            receiver: Some(Type::Plugin {
                                namespace: plugin.namespace.clone(),
                                name: interface.name.clone(),
                            }),
                            is_constructor: false,
                            is_mutable_property: false,
                            error_handling_allowed: false,
                            error_type: None,
                        },
                    );
                }
            }
        }
        // Value types declared by a native plugin use the same direct member
        // syntax as app structs. Register their fields as immutable plugin
        // members so event payloads and native method results can be read
        // without flattening them into separate callback parameters.
        for declaration in idl
            .types
            .iter()
            .filter(|declaration| declaration.kind == nexa_plugin_idl::NamedTypeKind::Struct)
        {
            let receiver = Type::Plugin {
                namespace: plugin.namespace.clone(),
                name: declaration.name.clone(),
            };
            for field in &declaration.fields {
                let key = format!("{}.#property.{}", declaration.name, field.name);
                if signatures.contains_key(&key) {
                    return Err(CompileError::new(
                        plugin.span,
                        format!(
                            "plugin value field `{}.{}` is declared more than once",
                            declaration.name, field.name
                        ),
                    ));
                }
                let return_type = plugin_type(&plugin.namespace, &field.ty, false, &[])
                    .map_err(|message| CompileError::new(plugin.span, message))?;
                signatures.insert(
                    key,
                    FunctionSignature {
                        parameters: Vec::new(),
                        type_parameters: Vec::new(),
                        return_type,
                        is_async: false,
                        is_throwing: false,
                        receiver: Some(receiver.clone()),
                        is_constructor: false,
                        is_mutable_property: false,
                        error_handling_allowed: false,
                        error_type: None,
                    },
                );
            }
        }
    }
    Ok(signatures)
}

/// Resolves an IDL type reference to a language type. `scope` holds the
/// method's value type parameters; a name in scope becomes
/// [`Type::TypeParam`], which every call site must bind before code
/// generation.
fn plugin_type(
    namespace: &str,
    ty: &TypeRef,
    return_position: bool,
    scope: &[String],
) -> Result<Type, String> {
    let mut result = match ty.name.as_str() {
        "Void" if return_position => Type::Void,
        "Void" => return Err("`Void` is only valid as a native method return type".to_owned()),
        "String" => Type::String,
        "Bool" => Type::Bool,
        "Int8" => Type::Numeric(NumericType::Int8),
        "Int16" => Type::Numeric(NumericType::Int16),
        "Int32" => Type::Numeric(NumericType::Int32),
        "Int64" => Type::Numeric(NumericType::Int64),
        "UInt8" => Type::Numeric(NumericType::UInt8),
        "UInt16" => Type::Numeric(NumericType::UInt16),
        "UInt32" => Type::Numeric(NumericType::UInt32),
        "UInt64" => Type::Numeric(NumericType::UInt64),
        "Float32" => Type::Numeric(NumericType::Float32),
        "Float64" => Type::Numeric(NumericType::Float64),
        "Bytes" => Type::Bytes,
        "Array" if ty.arguments.len() == 1 => Type::Array(Box::new(plugin_type(
            namespace,
            &ty.arguments[0],
            false,
            scope,
        )?)),
        "Set" if ty.arguments.len() == 1 => Type::Set(Box::new(plugin_type(
            namespace,
            &ty.arguments[0],
            false,
            scope,
        )?)),
        "Map" if ty.arguments.len() == 2 => Type::Map(
            Box::new(plugin_type(namespace, &ty.arguments[0], false, scope)?),
            Box::new(plugin_type(namespace, &ty.arguments[1], false, scope)?),
        ),
        "Pair" if ty.arguments.len() == 2 => Type::Pair(
            Box::new(plugin_type(namespace, &ty.arguments[0], false, scope)?),
            Box::new(plugin_type(namespace, &ty.arguments[1], false, scope)?),
        ),
        "Triple" if ty.arguments.len() == 3 => Type::Triple(
            Box::new(plugin_type(namespace, &ty.arguments[0], false, scope)?),
            Box::new(plugin_type(namespace, &ty.arguments[1], false, scope)?),
            Box::new(plugin_type(namespace, &ty.arguments[2], false, scope)?),
        ),
        "Result" if return_position && ty.arguments.len() == 2 => {
            plugin_type(namespace, &ty.arguments[0], false, scope)?
        }
        "Result" => {
            return Err(
                "`Result<Success, Failure>` is only valid as a plugin return type".to_owned(),
            );
        }
        name if scope.iter().any(|parameter| parameter == name) => {
            if !ty.arguments.is_empty() {
                return Err(format!(
                    "type parameter `{name}` cannot take type arguments"
                ));
            }
            Type::TypeParam(name.to_owned())
        }
        name => Type::Plugin {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        },
    };
    if ty.optional {
        result = Type::Optional(Box::new(result));
    }
    Ok(result)
}

fn plugin_error_type(
    namespace: &str,
    idl: &nexa_plugin_idl::PluginIdl,
    reference: &TypeRef,
) -> Result<PluginErrorType, String> {
    let declaration = idl
        .types
        .iter()
        .find(|declaration| {
            declaration.name == reference.name
                && declaration.kind == nexa_plugin_idl::NamedTypeKind::Error
        })
        .ok_or_else(|| format!("`{}` is not a declared plugin error type", reference.name))?;
    let variants = declaration
        .cases
        .iter()
        .map(|variant| {
            let parameters = variant
                .parameters
                .iter()
                .map(|parameter| {
                    plugin_type(namespace, &parameter.ty, false, &[])
                        .map(|ty| (parameter.name.clone(), ty))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PluginErrorVariant {
                name: variant.name.clone(),
                parameters,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(PluginErrorType {
        namespace: namespace.to_owned(),
        name: declaration.name.clone(),
        variants,
    })
}

pub(super) fn collect_function_signatures(
    declarations: &[ast::FunctionDecl],
    structs: &StructTypes,
) -> Result<FunctionSignatures, CompileError> {
    let mut signatures = HashMap::with_capacity(declarations.len());
    for declaration in declarations {
        if signatures.contains_key(&declaration.name) {
            return Err(CompileError::new(
                declaration.span,
                format!("function `{}` is already declared", declaration.name),
            ));
        }
        let mut names = std::collections::HashSet::with_capacity(declaration.parameters.len());
        let mut parameters = Vec::with_capacity(declaration.parameters.len());
        for parameter in &declaration.parameters {
            if !names.insert(parameter.name.as_str()) {
                return Err(CompileError::new(
                    parameter.span,
                    format!(
                        "function parameter `{}` is declared more than once",
                        parameter.name
                    ),
                ));
            }
            parameters.push((
                parameter.name.clone(),
                resolve_struct_type(&parse_type(&parameter.ty)?, structs),
            ));
        }
        signatures.insert(
            declaration.name.clone(),
            FunctionSignature {
                parameters,
                type_parameters: Vec::new(),
                return_type: resolve_struct_type(&parse_type(&declaration.return_type)?, structs),
                is_async: declaration.is_async,
                is_throwing: false,
                receiver: None,
                is_constructor: false,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        );
    }
    Ok(signatures)
}

pub(super) fn references_state(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(_, _) => true,
        ast::Expr::EnumCase { .. } => false,
        ast::Expr::Add(left, right, _)
        | ast::Expr::Arithmetic(left, _, right, _)
        | ast::Expr::Binary(left, _, right, _) => references_state(left) || references_state(right),
        ast::Expr::Negate(value, _) | ast::Expr::Not(value, _) => references_state(value),
        ast::Expr::Array(items, _) => items.iter().any(references_state),
        ast::Expr::Map(entries, _) => entries
            .iter()
            .any(|(key, value)| references_state(key) || references_state(value)),
        ast::Expr::Pair(first, second, _) => references_state(first) || references_state(second),
        ast::Expr::Triple(first, second, third, _) => {
            references_state(first) || references_state(second) || references_state(third)
        }
        ast::Expr::Call(_, _, arguments, _) => arguments.iter().any(references_state),
        ast::Expr::CallNamed { arguments, .. } => arguments.values().any(references_state),
        ast::Expr::MethodCall {
            base,
            arguments,
            named_arguments,
            ..
        } => {
            references_state(base)
                || arguments.iter().any(references_state)
                || named_arguments.values().any(references_state)
        }
        ast::Expr::Closure { body, .. } => references_state(body),
        ast::Expr::QualifiedCall {
            arguments,
            named_arguments,
            ..
        } => {
            arguments.iter().any(references_state) || named_arguments.values().any(references_state)
        }
        ast::Expr::Index {
            collection, index, ..
        } => references_state(collection) || references_state(index),
        ast::Expr::Member { base, .. } => match base.as_ref() {
            ast::Expr::Name(name, _) if name.chars().next().is_some_and(char::is_uppercase) => {
                false
            }
            _ => references_state(base),
        },
        ast::Expr::Range {
            start, end, step, ..
        } => {
            references_state(start)
                || references_state(end)
                || step.as_deref().is_some_and(references_state)
        }
        ast::Expr::Coalesce(left, right, _) => references_state(left) || references_state(right),
        ast::Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => {
            references_state(condition)
                || references_state(then_value)
                || references_state(else_value)
        }
        ast::Expr::Await(value, _) | ast::Expr::Try { expr: value, .. } => references_state(value),
        ast::Expr::Interpolation(parts, _) => parts.iter().any(|part| match part {
            ast::StringPart::Name(_) => true,
            ast::StringPart::Expression(expression) => references_state(expression),
            ast::StringPart::Literal(_) => false,
        }),
        ast::Expr::String(_, _)
        | ast::Expr::Number(_, _)
        | ast::Expr::Bool(_, _)
        | ast::Expr::Null(_)
        | ast::Expr::ThemeToken(_, _)
        | ast::Expr::IsRegularWidth(_)
        | ast::Expr::IsCompactWidth(_)
        | ast::Expr::IsRegularHeight(_)
        | ast::Expr::IsCompactHeight(_) => false,
    }
}

pub(super) fn record_native_alias(
    name: &str,
    ty: &Type,
    initial: &Expr,
    aliases: &mut HashMap<String, String>,
) {
    if !matches!(ty, Type::Plugin { .. }) {
        return;
    }
    let Expr::State(source, _) = initial else {
        return;
    };
    let root = aliases
        .get(source)
        .cloned()
        .unwrap_or_else(|| source.clone());
    aliases.insert(name.to_owned(), root);
}

pub(super) fn lower_expr(
    expr: &ast::Expr,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
) -> Result<Expr, CompileError> {
    // A non-null value may be promoted to its optional type without a runtime
    // wrapper. Keep an already-optional expression and the `null` literal on
    // the outer type so their nullability is checked exactly.
    let expected = match expected {
        Some(Type::Optional(inner))
            if !matches!(expr, ast::Expr::Null(_))
                && !matches!(expr, ast::Expr::Conditional { .. })
                && !matches!(
                    infer_expr_type(expr, ctx.symbols, ctx.functions),
                    Some(Type::Optional(_))
                ) =>
        {
            Some(inner.as_ref())
        }
        _ => expected,
    };
    match expr {
        ast::Expr::String(value, _) => {
            require_expected(expected, &Type::String, expr.span())?;
            Ok(Expr::String(value.clone()))
        }
        ast::Expr::Interpolation(parts, span) => {
            require_expected(expected, &Type::String, *span)?;
            let mut lowered = Vec::with_capacity(parts.len());
            for part in parts {
                match part {
                    ast::StringPart::Literal(value) => {
                        lowered.push(InterpolatedPart::Literal(value.clone()));
                    }
                    ast::StringPart::Name(name) => {
                        let value = lower_expr(&ast::Expr::Name(name.clone(), *span), None, ctx)?;
                        lowered.push(InterpolatedPart::Value(Box::new(value)));
                    }
                    ast::StringPart::Expression(expression) => {
                        lowered.push(InterpolatedPart::Value(Box::new(lower_expr(
                            expression, None, ctx,
                        )?)));
                    }
                }
            }
            Ok(Expr::Interpolation(lowered))
        }
        ast::Expr::Bool(value, _) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::Bool(*value))
        }
        ast::Expr::Conditional {
            condition,
            then_value,
            else_value,
            span,
        } => {
            let value_type = expected
                .cloned()
                .or_else(|| {
                    conditional_value_type(then_value, else_value, ctx.symbols, ctx.functions)
                })
                .ok_or_else(|| {
                    CompileError::new(
                        *span,
                        "cannot infer the `if` expression value type; add an explicit type context",
                    )
                })?;
            let condition = lower_expr(condition, Some(&Type::Bool), ctx)?;
            let then_value = lower_expr(then_value, Some(&value_type), ctx)?;
            let else_value = lower_expr(else_value, Some(&value_type), ctx)?;
            require_expected(expected, &value_type, *span)?;
            Ok(Expr::Conditional {
                condition: Box::new(condition),
                then_value: Box::new(then_value),
                else_value: Box::new(else_value),
                value_type,
            })
        }
        ast::Expr::Null(span) => {
            let Some(Type::Optional(inner)) = expected else {
                return Err(CompileError::new(
                    *span,
                    "`null` requires an explicitly typed optional value",
                ));
            };
            Ok(Expr::Null(Type::Optional(inner.clone())))
        }
        ast::Expr::IsRegularWidth(_) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::IsRegularWidth)
        }
        ast::Expr::IsCompactWidth(_) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::IsCompactWidth)
        }
        ast::Expr::IsRegularHeight(_) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::IsRegularHeight)
        }
        ast::Expr::IsCompactHeight(_) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::IsCompactHeight)
        }
        ast::Expr::Array(items, span) => {
            let element_type = match expected {
                Some(Type::Array(element_type)) => element_type,
                Some(Type::Set(element_type)) => element_type,
                _ => {
                    return Err(CompileError::new(
                        *span,
                        "`[...]` literals require an explicit Array<T> or Set<T> type",
                    ));
                }
            };
            let mut lowered = Vec::with_capacity(items.len());
            for item in items {
                lowered.push(lower_expr(item, Some(element_type), ctx)?);
            }
            if matches!(expected, Some(Type::Set(_))) {
                Ok(Expr::Set(lowered))
            } else {
                Ok(Expr::Array(lowered))
            }
        }
        ast::Expr::Map(entries, span) => {
            let Some(Type::Map(key_type, value_type)) = expected else {
                return Err(CompileError::new(
                    *span,
                    "map literals require an explicit Map<K, V> type",
                ));
            };
            let mut lowered = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                lowered.push((
                    lower_expr(key, Some(key_type), ctx)?,
                    lower_expr(value, Some(value_type), ctx)?,
                ));
            }
            Ok(Expr::Map(lowered))
        }
        ast::Expr::Pair(first, second, span) => {
            let Some(Type::Pair(first_type, second_type)) = expected else {
                return Err(CompileError::new(
                    *span,
                    "Pair(...) requires an explicit Pair<A, B> type",
                ));
            };
            Ok(Expr::Pair(
                Box::new(lower_expr(first, Some(first_type), ctx)?),
                Box::new(lower_expr(second, Some(second_type), ctx)?),
            ))
        }
        ast::Expr::Triple(first, second, third, span) => {
            let Some(Type::Triple(first_type, second_type, third_type)) = expected else {
                return Err(CompileError::new(
                    *span,
                    "Triple(...) requires an explicit Triple<A, B, C> type",
                ));
            };
            Ok(Expr::Triple(
                Box::new(lower_expr(first, Some(first_type), ctx)?),
                Box::new(lower_expr(second, Some(second_type), ctx)?),
                Box::new(lower_expr(third, Some(third_type), ctx)?),
            ))
        }
        ast::Expr::Number(raw, span) => {
            let ty = match expected {
                Some(Type::Numeric(ty)) => *ty,
                Some(other) => {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "number cannot be used where {} is expected",
                            type_name(other)
                        ),
                    ));
                }
                None if raw.contains('.') => NumericType::Float64,
                None => NumericType::Int32,
            };
            validate_number(raw, ty, *span)?;
            Ok(Expr::Number {
                raw: raw.clone(),
                ty,
            })
        }
        ast::Expr::Name(name, span) => {
            if matches!(expected, Some(Type::Enum(enum_name)) if enum_name == "Permission") {
                if !is_permission_case(name) {
                    return Err(CompileError::new(
                        *span,
                        format!("unknown permission `{name}`"),
                    ));
                }
                return Ok(Expr::EnumValue {
                    enum_name: "Permission".to_owned(),
                    case_name: name.clone(),
                });
            }
            let Some((ty, _)) = ctx.symbols.get(name) else {
                return Err(CompileError::new(*span, format!("unknown state `{name}`")));
            };
            require_expected(expected, ty, *span)?;
            Ok(Expr::State(name.clone(), ty.clone()))
        }
        ast::Expr::EnumCase {
            enum_name,
            case_name,
            span,
        } => {
            if enum_name == "PermissionStatus" {
                if !is_permission_status_case(case_name) {
                    return Err(CompileError::new(
                        *span,
                        format!("unknown permission status `{enum_name}.{case_name}`"),
                    ));
                }
                let ty = Type::Enum("PermissionStatus".to_owned());
                require_expected(expected, &ty, *span)?;
                return Ok(Expr::EnumValue {
                    enum_name: enum_name.clone(),
                    case_name: case_name.clone(),
                });
            }
            if enum_name == "Permission" {
                if !is_permission_case(case_name) {
                    return Err(CompileError::new(
                        *span,
                        format!("unknown permission `{enum_name}.{case_name}`"),
                    ));
                }
                let ty = Type::Enum("Permission".to_owned());
                require_expected(expected, &ty, *span)?;
                return Ok(Expr::EnumValue {
                    enum_name: enum_name.clone(),
                    case_name: case_name.clone(),
                });
            }
            let key = format!("{enum_name}.{case_name}");
            let Some((ty @ Type::Enum(_), _)) = ctx.symbols.get(&key) else {
                return Err(CompileError::new(
                    *span,
                    format!("unknown enum case `{enum_name}.{case_name}`"),
                ));
            };
            require_expected(expected, ty, *span)?;
            Ok(Expr::EnumValue {
                enum_name: enum_name.clone(),
                case_name: case_name.clone(),
            })
        }
        ast::Expr::ThemeToken(name, span) => Err(CompileError::new(
            *span,
            format!("`Theme.{name}` can only be used in supported style options"),
        )),
        ast::Expr::Add(left, right, span) => {
            if expected == Some(&Type::String)
                || matches!(
                    infer_expr_type(left, ctx.symbols, ctx.functions),
                    Some(Type::String)
                )
                || matches!(
                    infer_expr_type(right, ctx.symbols, ctx.functions),
                    Some(Type::String)
                )
            {
                let left = lower_expr(left, Some(&Type::String), ctx)?;
                let right = lower_expr(right, Some(&Type::String), ctx)?;
                return Ok(Expr::Concat(Box::new(left), Box::new(right)));
            }
            let ty = expected
                .and_then(as_numeric_type)
                .or_else(|| {
                    infer_expr_type(left, ctx.symbols, ctx.functions)
                        .and_then(|ty| as_numeric_type(&ty))
                })
                .or_else(|| {
                    infer_expr_type(right, ctx.symbols, ctx.functions)
                        .and_then(|ty| as_numeric_type(&ty))
                })
                .ok_or_else(|| {
                    CompileError::new(*span, "`+` requires numeric values of the same type")
                })?;
            let numeric = Type::Numeric(ty);
            if expected.is_some_and(|expected| expected != &numeric) {
                return Err(CompileError::new(
                    *span,
                    format!(
                        "`+` produces {}, which does not match the expected type",
                        type_name(&numeric)
                    ),
                ));
            }
            let left = lower_expr(left, Some(&numeric), ctx)?;
            let right = lower_expr(right, Some(&numeric), ctx)?;
            if expr_numeric_type(&left) != Some(ty) || expr_numeric_type(&right) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "both sides of `+` must have the same numeric type",
                ));
            }
            Ok(Expr::Add(Box::new(left), Box::new(right), ty))
        }
        ast::Expr::Arithmetic(left, operator, right, span) => {
            let ty = expected
                .and_then(as_numeric_type)
                .or_else(|| {
                    infer_expr_type(left, ctx.symbols, ctx.functions)
                        .and_then(|ty| as_numeric_type(&ty))
                })
                .or_else(|| {
                    infer_expr_type(right, ctx.symbols, ctx.functions)
                        .and_then(|ty| as_numeric_type(&ty))
                })
                .ok_or_else(|| {
                    CompileError::new(*span, "arithmetic operators require numeric values")
                })?;
            let numeric = Type::Numeric(ty);
            if expected.is_some_and(|expected| expected != &numeric) {
                return Err(CompileError::new(
                    *span,
                    format!(
                        "arithmetic produces {}, which does not match the expected type",
                        type_name(&numeric)
                    ),
                ));
            }
            let left = lower_expr(left, Some(&numeric), ctx)?;
            let right = lower_expr(right, Some(&numeric), ctx)?;
            if expr_numeric_type(&left) != Some(ty) || expr_numeric_type(&right) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "both arithmetic operands must have the same numeric type",
                ));
            }
            let op = match operator {
                ast::ArithmeticOp::Subtract => IrArithmeticOp::Subtract,
                ast::ArithmeticOp::Multiply => IrArithmeticOp::Multiply,
                ast::ArithmeticOp::Divide => IrArithmeticOp::Divide,
                ast::ArithmeticOp::Remainder => IrArithmeticOp::Remainder,
            };
            Ok(Expr::Arithmetic {
                op,
                left: Box::new(left),
                right: Box::new(right),
                ty,
            })
        }
        ast::Expr::Negate(value, span) => {
            let ty = expected
                .and_then(as_numeric_type)
                .or_else(|| {
                    infer_expr_type(value, ctx.symbols, ctx.functions)
                        .and_then(|ty| as_numeric_type(&ty))
                })
                .ok_or_else(|| CompileError::new(*span, "unary `-` requires a numeric value"))?;
            let numeric = Type::Numeric(ty);
            if expected.is_some_and(|expected| expected != &numeric) {
                return Err(CompileError::new(
                    *span,
                    format!(
                        "unary `-` produces {}, which does not match the expected type",
                        type_name(&numeric)
                    ),
                ));
            }
            let value = lower_expr(value, Some(&numeric), ctx)?;
            if expr_numeric_type(&value) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "unary `-` requires a numeric value",
                ));
            }
            Ok(Expr::Negate {
                value: Box::new(value),
                ty,
            })
        }
        ast::Expr::Not(value, _span) => {
            let value = lower_expr(value, Some(&Type::Bool), ctx)?;
            Ok(Expr::Not(Box::new(value)))
        }
        ast::Expr::Binary(left, operator, right, span) => {
            lower_binary(left, *operator, right, *span, expected, ctx)
        }
        ast::Expr::Call(name, type_arguments, arguments, span) => {
            lower_call(name, type_arguments, arguments, *span, expected, ctx, false)
        }
        ast::Expr::CallNamed {
            name,
            type_arguments,
            arguments,
            span,
        } => lower_named_call(name, type_arguments, arguments, *span, expected, ctx, false),
        ast::Expr::MethodCall {
            base,
            name,
            type_arguments,
            arguments,
            named_arguments,
            span,
        } => {
            if matches!(
                infer_expr_type(base, ctx.symbols, ctx.functions),
                Some(Type::Plugin { .. })
            ) {
                lower_plugin_method_call(
                    base,
                    name,
                    CallArguments {
                        type_arguments,
                        arguments,
                        named_arguments,
                        span: *span,
                        expected,
                        ctx,
                        awaited: false,
                    },
                )
            } else {
                if !type_arguments.is_empty() {
                    return Err(CompileError::new(
                        *span,
                        "type arguments are only supported on plugin methods",
                    ));
                }
                if !named_arguments.is_empty() {
                    return Err(CompileError::new(
                        *span,
                        "named arguments are not supported for this method call",
                    ));
                }
                if is_collection_utility(name) {
                    lower_collection_utility(base, name, arguments, *span, expected, ctx)
                } else {
                    lower_collection_transform(base, name, arguments, *span, expected, ctx)
                }
            }
        }
        ast::Expr::Closure { span, .. } => Err(CompileError::new(
            *span,
            "closures are only valid as collection transformation callbacks",
        )),
        ast::Expr::QualifiedCall {
            namespace,
            name,
            type_arguments,
            arguments,
            named_arguments,
            span,
        } => lower_native_call(
            namespace,
            name,
            CallArguments {
                type_arguments,
                arguments,
                named_arguments,
                span: *span,
                expected,
                ctx,
                awaited: false,
            },
        ),
        ast::Expr::Index {
            collection,
            index,
            optional,
            span,
        } => {
            let inferred_collection_type = infer_expr_type(collection, ctx.symbols, ctx.functions);
            let (index_type, result_type, collection_type) = match inferred_collection_type {
                Some(Type::Array(_)) if *optional => {
                    return Err(CompileError::new(
                        *span,
                        "`?[index]` requires an optional Array<T> or Map<K, V> value",
                    ));
                }
                Some(Type::Map(_, _)) if *optional => {
                    return Err(CompileError::new(
                        *span,
                        "`?[index]` requires an optional Array<T> or Map<K, V> value",
                    ));
                }
                Some(Type::Array(element_type)) => (
                    Type::Numeric(NumericType::Int32),
                    (*element_type).clone(),
                    Type::Array(element_type),
                ),
                Some(Type::Map(key_type, value_type)) => (
                    (*key_type).clone(),
                    Type::Optional(value_type.clone()),
                    Type::Map(key_type, value_type),
                ),
                Some(Type::Optional(inner)) if *optional => match *inner {
                    Type::Array(element_type) => (
                        Type::Numeric(NumericType::Int32),
                        Type::Optional(element_type.clone()),
                        Type::Optional(Box::new(Type::Array(element_type))),
                    ),
                    Type::Map(key_type, value_type) => (
                        (*key_type).clone(),
                        Type::Optional(value_type.clone()),
                        Type::Optional(Box::new(Type::Map(key_type, value_type))),
                    ),
                    _ => {
                        return Err(CompileError::new(
                            *span,
                            "optional collection indexing requires an optional Array<T> or Map<K, V> value",
                        ));
                    }
                },
                Some(Type::Optional(_)) => {
                    return Err(CompileError::new(
                        *span,
                        "optional collections require `?[index]` for safe indexing",
                    ));
                }
                _ => {
                    return Err(CompileError::new(
                        *span,
                        "collection indexing requires an Array<T> or Map<K, V> value",
                    ));
                }
            };
            let lowered_collection = lower_expr(collection, Some(&collection_type), ctx)?;
            let lowered_index = lower_expr(index, Some(&index_type), ctx)?;
            require_expected(expected, &result_type, *span)?;
            Ok(Expr::Index {
                collection: Box::new(lowered_collection),
                index: Box::new(lowered_index),
                optional: *optional,
                collection_type,
                element_type: result_type,
            })
        }
        ast::Expr::Member {
            base,
            name,
            optional,
            span,
        } => {
            if !*optional
                && name == "isOnline"
                && matches!(base.as_ref(), ast::Expr::Name(namespace, _) if namespace == "Network")
            {
                let return_type = Type::Bool;
                require_expected(expected, &return_type, *span)?;
                return Ok(Expr::NativeCall {
                    receiver: None,
                    namespace: "Network".to_owned(),
                    name: "isOnline".to_owned(),
                    arguments: Vec::new(),
                    codecs: Vec::new(),
                    return_type,
                    is_async: false,
                    is_throwing: false,
                });
            }
            if !*optional && let ast::Expr::Name(enum_name, _) = base.as_ref() {
                if enum_name == "PermissionStatus" && is_permission_status_case(name) {
                    let ty = Type::Enum("PermissionStatus".to_owned());
                    require_expected(expected, &ty, *span)?;
                    return Ok(Expr::EnumValue {
                        enum_name: enum_name.clone(),
                        case_name: name.clone(),
                    });
                }
                if enum_name == "Permission" && is_permission_case(name) {
                    let ty = Type::Enum("Permission".to_owned());
                    require_expected(expected, &ty, *span)?;
                    return Ok(Expr::EnumValue {
                        enum_name: enum_name.clone(),
                        case_name: name.clone(),
                    });
                }
                let key = format!("{enum_name}.{name}");
                if let Some((ty @ Type::Enum(_), _)) = ctx.symbols.get(&key) {
                    require_expected(expected, ty, *span)?;
                    return Ok(Expr::EnumValue {
                        enum_name: enum_name.clone(),
                        case_name: name.clone(),
                    });
                }
            }
            let Some(base_type) = infer_expr_type(base, ctx.symbols, ctx.functions) else {
                return Err(CompileError::new(
                    *span,
                    format!("cannot access member `{name}` on an untyped value"),
                ));
            };
            let (member_base_type, field_type) = if *optional {
                let Type::Optional(inner) = &base_type else {
                    return Err(CompileError::new(
                        *span,
                        "`?.` requires an optional Pair, Triple, struct, or collection value",
                    ));
                };
                let Some(field_type) = member_field_type_with_plugins(inner, name, ctx.functions)
                else {
                    return Err(CompileError::new(
                        *span,
                        format!("`{}` has no member `{name}`", type_name(inner)),
                    ));
                };
                (base_type.clone(), Type::Optional(Box::new(field_type)))
            } else {
                if matches!(base_type, Type::Optional(_)) {
                    return Err(CompileError::new(
                        *span,
                        "optional values require `?.` for member access",
                    ));
                }
                let Some(field_type) =
                    member_field_type_with_plugins(&base_type, name, ctx.functions)
                else {
                    return Err(CompileError::new(
                        *span,
                        format!("`{}` has no member `{name}`", type_name(&base_type)),
                    ));
                };
                (base_type.clone(), field_type)
            };
            let base = lower_expr(base, Some(&member_base_type), ctx)?;
            require_expected(expected, &field_type, *span)?;
            let tuple_type = match &member_base_type {
                Type::Optional(inner) => inner.as_ref(),
                base_type => base_type,
            };
            // The (base type, member name) pair was validated above; record
            // the dispatch decision once so renderers match on it instead of
            // re-deriving legality.
            let kind = match (tuple_type, name.as_str()) {
                (Type::Pair(_, _), "first") | (Type::Triple(_, _, _), "first") => {
                    MemberKind::TupleIndex(TuplePosition::First)
                }
                (Type::Pair(_, _), "second") | (Type::Triple(_, _, _), "second") => {
                    MemberKind::TupleIndex(TuplePosition::Second)
                }
                (Type::Triple(_, _, _), "third") => MemberKind::TupleIndex(TuplePosition::Third),
                (Type::Struct { .. }, field) => MemberKind::StructField(field.to_owned()),
                (Type::Plugin { .. }, field) => MemberKind::PluginField(field.to_owned()),
                (Type::NetworkResponse, "statusCode") => MemberKind::NetworkStatusCode,
                (Type::NetworkResponse, "headers") => MemberKind::NetworkHeaders,
                (Type::NetworkResponse, "body") => MemberKind::NetworkBody,
                (Type::Array(_) | Type::Set(_) | Type::Map(_, _), "count") => {
                    MemberKind::CollectionCount
                }
                (Type::Array(_) | Type::Set(_) | Type::Map(_, _), "isEmpty") => {
                    MemberKind::CollectionIsEmpty
                }
                _ => {
                    return Err(CompileError::new(
                        *span,
                        format!("`{}` has no member `{name}`", type_name(&member_base_type)),
                    ));
                }
            };
            Ok(Expr::Member {
                base: Box::new(base),
                name: name.clone(),
                optional: *optional,
                base_type: member_base_type,
                field_type,
                kind,
            })
        }
        ast::Expr::Range { span, .. } => Err(CompileError::new(
            *span,
            "ranges are only valid as `for` loop iterables",
        )),
        ast::Expr::Coalesce(left, right, span) => {
            let Some(Type::Optional(inner)) = infer_expr_type(left, ctx.symbols, ctx.functions)
            else {
                return Err(CompileError::new(
                    *span,
                    "left side of `??` must be an optional value",
                ));
            };
            let optional_type = Type::Optional(inner.clone());
            let lowered_left = lower_expr(left, Some(&optional_type), ctx)?;
            // A generic plugin call reports its *unsubstituted* signature type
            // to inference, so the concrete value type comes from the lowered
            // call: the right side must see the type the left actually bound.
            let inner = match lowered_type(&lowered_left) {
                Some(Type::Optional(resolved)) => *resolved,
                _ => *inner,
            };
            let lowered_right = lower_expr(right, Some(&inner), ctx)?;
            require_expected(expected, &inner, *span)?;
            Ok(Expr::Coalesce(
                Box::new(lowered_left),
                Box::new(lowered_right),
            ))
        }
        ast::Expr::Await(value, span) => {
            if !ctx.allow_await {
                return Err(CompileError::new(
                    *span,
                    "`await` is only allowed in an async function or `OnAppear async` block",
                ));
            }
            let call = match value.as_ref() {
                ast::Expr::Call(name, type_arguments, arguments, call_span) => lower_call(
                    name,
                    type_arguments,
                    arguments,
                    *call_span,
                    expected,
                    ctx,
                    true,
                )?,
                ast::Expr::CallNamed {
                    name,
                    type_arguments,
                    arguments,
                    span: call_span,
                } => lower_named_call(
                    name,
                    type_arguments,
                    arguments,
                    *call_span,
                    expected,
                    ctx,
                    true,
                )?,
                ast::Expr::QualifiedCall {
                    namespace,
                    name,
                    type_arguments,
                    arguments,
                    named_arguments,
                    span: call_span,
                } => lower_native_call(
                    namespace,
                    name,
                    CallArguments {
                        type_arguments,
                        arguments,
                        named_arguments,
                        span: *call_span,
                        expected,
                        ctx,
                        awaited: true,
                    },
                )?,
                ast::Expr::MethodCall {
                    base,
                    name,
                    type_arguments,
                    arguments,
                    named_arguments,
                    span: call_span,
                } if matches!(
                    infer_expr_type(base, ctx.symbols, ctx.functions),
                    Some(Type::Plugin { .. })
                ) =>
                {
                    lower_plugin_method_call(
                        base,
                        name,
                        CallArguments {
                            type_arguments,
                            arguments,
                            named_arguments,
                            span: *call_span,
                            expected,
                            ctx,
                            awaited: true,
                        },
                    )?
                }
                _ => {
                    return Err(CompileError::new(
                        *span,
                        "`await` must be applied to an async function call",
                    ));
                }
            };
            if call.is_throwing_call() {
                if ctx
                    .functions
                    .values()
                    .any(|signature| signature.error_handling_allowed)
                {
                    Ok(Expr::TryAwait(Box::new(call)))
                } else {
                    Err(CompileError::new(
                        *span,
                        "native async call may throw; wrap it in a `try { ... } catch { ... }` action",
                    ))
                }
            } else {
                Ok(Expr::Await(Box::new(call)))
            }
        }
        ast::Expr::Try { expr: inner, span } => {
            let lowered = lower_expr(inner, None, ctx)?;
            let ty = infer_expr_type(inner, ctx.symbols, ctx.functions).ok_or_else(|| {
                CompileError::new(*span, "cannot infer type of expression for `?` operator")
            })?;
            let (val_ty, err_ty) = match ty {
                Type::Result(v, e) => (*v, *e),
                other => {
                    return Err(CompileError::new(
                        *span,
                        format!(
                            "the `?` operator can only be applied to `Result` values, found `{}`",
                            type_name(&other)
                        ),
                    ));
                }
            };
            require_expected(expected, &val_ty, *span)?;
            Ok(Expr::Try {
                expr: Box::new(lowered),
                value_type: val_ty,
                error_type: err_ty,
            })
        }
    }
}

fn lower_collection_transform(
    base: &ast::Expr,
    name: &str,
    arguments: &[ast::Expr],
    span: Span,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
) -> Result<Expr, CompileError> {
    let operation = match name {
        "map" => CollectionTransform::Map,
        "filter" => CollectionTransform::Filter,
        "reduce" => CollectionTransform::Reduce,
        _ => {
            return Err(CompileError::new(
                span,
                format!(
                    "unknown collection method `{name}`; supported methods are `map`, `filter`, and `reduce`"
                ),
            ));
        }
    };
    let Some(base_type) = infer_expr_type(base, ctx.symbols, ctx.functions) else {
        return Err(CompileError::new(
            span,
            "collection transformation requires a typed Array<T> value",
        ));
    };
    let Type::Array(element_type) = &base_type else {
        return Err(CompileError::new(
            span,
            "collection transformations currently support Array<T> values only",
        ));
    };
    let (initial, closure) = match operation {
        CollectionTransform::Map | CollectionTransform::Filter => {
            if arguments.len() != 1 {
                return Err(CompileError::new(
                    span,
                    format!("`{name}` expects exactly one closure argument"),
                ));
            }
            (None, &arguments[0])
        }
        CollectionTransform::Reduce => {
            if arguments.len() != 2 {
                return Err(CompileError::new(
                    span,
                    "`reduce` expects an initial value and one closure argument",
                ));
            }
            (Some(&arguments[0]), &arguments[1])
        }
    };
    let ast::Expr::Closure {
        parameters,
        body,
        span: closure_span,
    } = closure
    else {
        return Err(CompileError::new(
            closure.span(),
            "collection transformations require an inline closure such as `{ item -> item }`",
        ));
    };
    let initial_type = initial.and_then(|value| infer_expr_type(value, ctx.symbols, ctx.functions));
    let lowered_initial = match initial {
        Some(value) => Some(Box::new(lower_expr(value, initial_type.as_ref(), ctx)?)),
        None => None,
    };
    let mut parameter_types = vec![element_type.as_ref().clone()];
    if operation == CollectionTransform::Reduce {
        parameter_types.insert(
            0,
            initial_type.clone().ok_or_else(|| {
                CompileError::new(
                    initial.expect("reduce always has an initial value").span(),
                    "cannot infer the type of the reduce initial value",
                )
            })?,
        );
    }
    if parameters.len() != parameter_types.len() {
        return Err(CompileError::new(
            *closure_span,
            format!(
                "`{name}` closure expects {} parameter(s), found {}",
                parameter_types.len(),
                parameters.len()
            ),
        ));
    }
    let mut scoped_symbols = ctx.symbols.clone();
    let mut names = HashSet::with_capacity(parameters.len());
    for (parameter, parameter_type) in parameters.iter().zip(parameter_types.iter()) {
        if !names.insert(parameter) {
            return Err(CompileError::new(
                *closure_span,
                format!("closure parameter `{parameter}` is declared more than once"),
            ));
        }
        scoped_symbols.insert(parameter.clone(), (parameter_type.clone(), false));
    }
    let body_expected = match operation {
        CollectionTransform::Filter => Some(Type::Bool),
        CollectionTransform::Map => expected.and_then(|ty| match ty {
            Type::Array(element) => Some(element.as_ref().clone()),
            _ => None,
        }),
        CollectionTransform::Reduce => expected.cloned(),
    };
    let inferred_body_type =
        infer_expr_type(body, &scoped_symbols, ctx.functions).or(body_expected.clone());
    let lowered_body = lower_expr(
        body,
        body_expected.as_ref().or(inferred_body_type.as_ref()),
        &ExprContext::new(&scoped_symbols, ctx.functions, false),
    )?;
    let Some(body_type) = inferred_body_type else {
        return Err(CompileError::new(
            *closure_span,
            "cannot infer the closure result type; add an explicit collection type",
        ));
    };
    if operation == CollectionTransform::Filter && body_type != Type::Bool {
        return Err(CompileError::new(
            *closure_span,
            "`filter` closures must return Bool",
        ));
    }
    let result_type = match operation {
        CollectionTransform::Map => Type::Array(Box::new(body_type)),
        CollectionTransform::Filter => Type::Array(Box::new(element_type.as_ref().clone())),
        CollectionTransform::Reduce => body_type,
    };
    require_expected(expected, &result_type, span)?;
    let lowered_base = lower_expr(base, Some(&base_type), ctx)?;
    Ok(Expr::CollectionTransform {
        operation,
        collection: Box::new(lowered_base),
        initial: lowered_initial,
        closure: Box::new(Expr::Closure {
            parameters: parameters.clone(),
            body: Box::new(lowered_body),
        }),
    })
}

fn is_collection_utility(name: &str) -> bool {
    matches!(name, "random" | "shuffled" | "reverse" | "slice")
}

fn lower_collection_utility(
    base: &ast::Expr,
    name: &str,
    arguments: &[ast::Expr],
    span: Span,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
) -> Result<Expr, CompileError> {
    let operation = match name {
        "random" => CollectionUtilityKind::Random,
        "shuffled" => CollectionUtilityKind::Shuffled,
        "reverse" => CollectionUtilityKind::Reverse,
        "slice" => CollectionUtilityKind::Slice,
        _ => {
            return Err(CompileError::new(
                span,
                format!("unknown collection method `{name}`"),
            ));
        }
    };
    let Some(Type::Array(element_type)) = infer_expr_type(base, ctx.symbols, ctx.functions) else {
        return Err(CompileError::new(
            span,
            format!("`{name}` requires an Array<T> receiver"),
        ));
    };

    let (start, end, inclusive) = match operation {
        CollectionUtilityKind::Random
        | CollectionUtilityKind::Shuffled
        | CollectionUtilityKind::Reverse => {
            if !arguments.is_empty() {
                return Err(CompileError::new(
                    span,
                    format!("`{name}` does not accept arguments"),
                ));
            }
            (None, None, false)
        }
        CollectionUtilityKind::Slice => {
            let [
                ast::Expr::Range {
                    start,
                    end,
                    inclusive,
                    step,
                    span: range_span,
                },
            ] = arguments
            else {
                return Err(CompileError::new(
                    span,
                    "`slice` expects one range argument, such as `slice(0..3)`",
                ));
            };
            if step.is_some() {
                return Err(CompileError::new(
                    *range_span,
                    "`slice` does not support a stepped range",
                ));
            }
            let int32 = Type::Numeric(NumericType::Int32);
            (
                Some(Box::new(lower_expr(start, Some(&int32), ctx)?)),
                Some(Box::new(lower_expr(end, Some(&int32), ctx)?)),
                *inclusive,
            )
        }
    };

    let result_type = collection_utility_result_type(operation, &element_type);
    require_expected(expected, &result_type, span)?;
    let collection = lower_expr(base, Some(&Type::Array(element_type.clone())), ctx)?;
    Ok(Expr::CollectionUtility {
        operation,
        collection: Box::new(collection),
        start,
        end,
        inclusive,
        element_type: *element_type,
    })
}

fn infer_collection_utility_type(
    base: &ast::Expr,
    name: &str,
    arguments: &[ast::Expr],
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Option<Type> {
    let operation = match name {
        "random" => CollectionUtilityKind::Random,
        "shuffled" => CollectionUtilityKind::Shuffled,
        "reverse" => CollectionUtilityKind::Reverse,
        "slice" => CollectionUtilityKind::Slice,
        _ => return None,
    };
    let Type::Array(element_type) = infer_expr_type(base, symbols, functions)? else {
        return None;
    };
    match operation {
        CollectionUtilityKind::Random if arguments.is_empty() => Some(Type::Optional(element_type)),
        CollectionUtilityKind::Shuffled | CollectionUtilityKind::Reverse
            if arguments.is_empty() =>
        {
            Some(Type::Array(element_type))
        }
        CollectionUtilityKind::Slice
            if matches!(arguments, [ast::Expr::Range { step: None, .. }]) =>
        {
            Some(Type::Array(element_type))
        }
        _ => None,
    }
}

fn collection_utility_result_type(operation: CollectionUtilityKind, element_type: &Type) -> Type {
    match operation {
        CollectionUtilityKind::Random => Type::Optional(Box::new(element_type.clone())),
        CollectionUtilityKind::Shuffled
        | CollectionUtilityKind::Reverse
        | CollectionUtilityKind::Slice => Type::Array(Box::new(element_type.clone())),
    }
}

fn infer_collection_transform_type(
    base: &ast::Expr,
    name: &str,
    arguments: &[ast::Expr],
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Option<Type> {
    let operation = match name {
        "map" => CollectionTransform::Map,
        "filter" => CollectionTransform::Filter,
        "reduce" => CollectionTransform::Reduce,
        _ => return None,
    };
    let Type::Array(element_type) = infer_expr_type(base, symbols, functions)? else {
        return None;
    };
    let (initial, closure) = match operation {
        CollectionTransform::Map | CollectionTransform::Filter if arguments.len() == 1 => {
            (None, &arguments[0])
        }
        CollectionTransform::Reduce if arguments.len() == 2 => (Some(&arguments[0]), &arguments[1]),
        _ => return None,
    };
    let ast::Expr::Closure {
        parameters, body, ..
    } = closure
    else {
        return None;
    };
    let initial_type = initial.and_then(|value| infer_expr_type(value, symbols, functions));
    let mut parameter_types = vec![element_type.as_ref().clone()];
    if operation == CollectionTransform::Reduce {
        parameter_types.insert(0, initial_type?);
    }
    if parameters.len() != parameter_types.len() {
        return None;
    }
    let mut scoped_symbols = symbols.clone();
    for (parameter, parameter_type) in parameters.iter().zip(parameter_types) {
        scoped_symbols.insert(parameter.clone(), (parameter_type, false));
    }
    let body_type = infer_expr_type(body, &scoped_symbols, functions)?;
    match operation {
        CollectionTransform::Map => Some(Type::Array(Box::new(body_type))),
        CollectionTransform::Filter if body_type == Type::Bool => Some(Type::Array(element_type)),
        CollectionTransform::Reduce => Some(body_type),
        _ => None,
    }
}

fn lower_call(
    name: &str,
    type_arguments: &[ast::TypeSyntax],
    arguments: &[ast::Expr],
    span: Span,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
    awaited: bool,
) -> Result<Expr, CompileError> {
    if !type_arguments.is_empty() {
        return Err(CompileError::new(
            span,
            "type arguments are only supported on plugin methods",
        ));
    }
    if name == "Ok" {
        if arguments.len() != 1 {
            return Err(CompileError::new(span, "`Ok` expects exactly 1 argument"));
        }
        let (expected_val, expected_err) = match expected {
            Some(Type::Result(v, e)) => (Some(v.as_ref()), Some(e.as_ref())),
            _ => (None, None),
        };
        let lowered_val = lower_expr(&arguments[0], expected_val, ctx)?;
        let val_ty =
            infer_expr_type(&arguments[0], ctx.symbols, ctx.functions).unwrap_or(Type::Void);
        let err_ty = expected_err.cloned().unwrap_or(Type::String);
        let result_ty = Type::Result(Box::new(val_ty.clone()), Box::new(err_ty.clone()));
        require_expected(expected, &result_ty, span)?;
        return Ok(Expr::ResultOk {
            value: Box::new(lowered_val),
            value_type: val_ty,
            error_type: err_ty,
        });
    }
    if name == "Err" {
        if arguments.len() != 1 {
            return Err(CompileError::new(span, "`Err` expects exactly 1 argument"));
        }
        let (expected_val, expected_err) = match expected {
            Some(Type::Result(v, e)) => (Some(v.as_ref()), Some(e.as_ref())),
            _ => (None, None),
        };
        let lowered_err = lower_expr(&arguments[0], expected_err, ctx)?;
        let err_ty =
            infer_expr_type(&arguments[0], ctx.symbols, ctx.functions).unwrap_or(Type::String);
        let val_ty = expected_val.cloned().unwrap_or(Type::Void);
        let result_ty = Type::Result(Box::new(val_ty.clone()), Box::new(err_ty.clone()));
        require_expected(expected, &result_ty, span)?;
        return Ok(Expr::ResultErr {
            error: Box::new(lowered_err),
            value_type: val_ty,
            error_type: err_ty,
        });
    }
    let Some(signature) = ctx.functions.get(name) else {
        return Err(CompileError::new(
            span,
            format!("unknown function `{name}`"),
        ));
    };
    if arguments.len() != signature.parameters.len() {
        return Err(CompileError::new(
            span,
            format!(
                "function `{name}` expects {} argument(s), found {}",
                signature.parameters.len(),
                arguments.len()
            ),
        ));
    }
    if signature.is_async && !awaited {
        return Err(CompileError::new(
            span,
            format!("async function `{name}` must be awaited"),
        ));
    }
    if awaited && !signature.is_async {
        return Err(CompileError::new(
            span,
            format!("function `{name}` is not async and cannot be awaited"),
        ));
    }
    if awaited && !ctx.allow_await {
        return Err(CompileError::new(
            span,
            "`await` is only allowed in an async function or `OnAppear async` block",
        ));
    }
    require_expected(expected, &signature.return_type, span)?;
    let lowered = arguments
        .iter()
        .zip(&signature.parameters)
        .map(|(argument, (_, ty))| lower_expr(argument, Some(ty), ctx))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expr::Call {
        name: name.to_owned(),
        arguments: lowered,
        return_type: signature.return_type.clone(),
        is_async: signature.is_async,
        is_constructor: signature.is_constructor
            || matches!(
                &signature.return_type,
                Type::Struct {
                    name: struct_name,
                    ..
                } if struct_name == name
            ),
    })
}

fn lower_named_call(
    name: &str,
    type_arguments: &[ast::TypeSyntax],
    named_arguments: &BTreeMap<String, ast::Expr>,
    span: Span,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
    awaited: bool,
) -> Result<Expr, CompileError> {
    let Some(signature) = ctx.functions.get(name) else {
        return Err(CompileError::new(
            span,
            format!("unknown function `{name}`"),
        ));
    };
    for argument in named_arguments.keys() {
        if !signature
            .parameters
            .iter()
            .any(|(parameter, _)| parameter == argument)
        {
            return Err(CompileError::new(
                named_arguments[argument].span(),
                format!("function `{name}` has no parameter named `{argument}`"),
            ));
        }
    }
    let arguments = signature
        .parameters
        .iter()
        .map(|(parameter, _)| {
            named_arguments.get(parameter).cloned().ok_or_else(|| {
                CompileError::new(
                    span,
                    format!("function `{name}` is missing argument `{parameter}`"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if named_arguments.len() != signature.parameters.len() {
        return Err(CompileError::new(
            span,
            format!(
                "function `{name}` expects {} named argument(s), found {}",
                signature.parameters.len(),
                named_arguments.len()
            ),
        ));
    }
    lower_call(
        name,
        type_arguments,
        &arguments,
        span,
        expected,
        ctx,
        awaited,
    )
}

#[derive(Clone, Copy)]
struct CallArguments<'a, 'ctx> {
    type_arguments: &'a [ast::TypeSyntax],
    arguments: &'a [ast::Expr],
    named_arguments: &'a BTreeMap<String, ast::Expr>,
    span: Span,
    expected: Option<&'a Type>,
    ctx: &'a ExprContext<'ctx>,
    awaited: bool,
}

fn lower_native_call(
    namespace: &str,
    name: &str,
    call: CallArguments<'_, '_>,
) -> Result<Expr, CompileError> {
    let CallArguments {
        arguments,
        named_arguments,
        span,
        expected,
        ctx,
        awaited,
        ..
    } = call;
    let qualified_name = format!("{namespace}.{name}");
    if !is_core_native_namespace(namespace) && ctx.functions.contains_key(&qualified_name) {
        return lower_plugin_call(namespace, name, call);
    }
    if namespace == "Json" {
        return lower_json_call(name, call);
    }
    if !arguments.is_empty() {
        return Err(CompileError::new(
            span,
            "built-in native API calls require named arguments",
        ));
    }
    // One named argument a built-in native API call accepts: its parameter
    // name, the parameter's type, and the argument the call site supplied.
    type ArgumentSpecs = Vec<(&'static str, Type, Option<ast::Expr>)>;
    let (return_type, is_async, specs): (Type, bool, ArgumentSpecs) = match qualified_name.as_str()
    {
        "Network.fetch" => (Type::NetworkResponse, true, network_specs(span, false)),
        "Network.download" => (Type::Bool, true, network_specs(span, true)),
        "Network.upload" => (
            Type::NetworkResponse,
            true,
            vec![
                ("url", Type::String, None),
                ("file", Type::String, None),
                (
                    "fields",
                    Type::Map(Box::new(Type::String), Box::new(Type::String)),
                    Some(ast::Expr::Map(Vec::new(), span)),
                ),
            ],
        ),
        "Path.documents" | "Path.caches" | "Path.temporary" | "Path.appSupport" => {
            (Type::String, false, Vec::new())
        }
        "Path.join" => (
            Type::String,
            false,
            vec![
                ("path", Type::String, None),
                ("component", Type::String, None),
            ],
        ),
        "File.exists" => (Type::Bool, false, vec![("path", Type::String, None)]),
        "File.readText" => (Type::String, true, vec![("path", Type::String, None)]),
        "File.writeText" => (
            Type::Bool,
            true,
            vec![
                ("path", Type::String, None),
                ("contents", Type::String, None),
            ],
        ),
        "File.delete" => (Type::Bool, true, vec![("path", Type::String, None)]),
        "Keyboard.dismiss" => (Type::Void, false, Vec::new()),
        "Screen.lockOrientation" => (Type::Void, false, vec![("mode", Type::String, None)]),
        "Bytes.fromText" => (Type::Bytes, false, vec![("text", Type::String, None)]),
        "Bytes.fromArray" => (
            Type::Bytes,
            false,
            vec![(
                "values",
                Type::Array(Box::new(Type::Numeric(NumericType::UInt8))),
                None,
            )],
        ),
        "Bytes.count" => (
            Type::Numeric(NumericType::Int32),
            false,
            vec![("bytes", Type::Bytes, None)],
        ),
        "Permissions.status" => (
            Type::Enum("PermissionStatus".to_owned()),
            true,
            vec![("permission", Type::Enum("Permission".to_owned()), None)],
        ),
        "Permissions.request" => (
            Type::Enum("PermissionStatus".to_owned()),
            true,
            vec![("permission", Type::Enum("Permission".to_owned()), None)],
        ),
        "Time.now" => (Type::Numeric(NumericType::Int64), false, Vec::new()),
        "Time.monotonic" => (Type::Numeric(NumericType::Int64), false, Vec::new()),
        "Time.elapsed" => (
            Type::Numeric(NumericType::Int64),
            false,
            vec![("since", Type::Numeric(NumericType::Int64), None)],
        ),
        "Time.sleep" => (
            Type::Void,
            true,
            vec![("milliseconds", Type::Numeric(NumericType::Int64), None)],
        ),
        "Time.iso8601" => (
            Type::String,
            false,
            vec![("timestamp", Type::Numeric(NumericType::Int64), None)],
        ),
        "Time.iso8601ToMillis" => (
            Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
            false,
            vec![("text", Type::String, None)],
        ),
        "Number.formatCurrency" => (
            Type::String,
            false,
            vec![
                ("amount", Type::Numeric(NumericType::Float64), None),
                ("currencyCode", Type::String, None),
            ],
        ),
        "Crypto.sha256" | "Crypto.sha512" => {
            (Type::String, false, vec![("text", Type::String, None)])
        }
        "Crypto.hmacSha256" => (
            Type::String,
            false,
            vec![("key", Type::String, None), ("message", Type::String, None)],
        ),
        "Crypto.randomBytes" => (
            Type::String,
            false,
            vec![("count", Type::Numeric(NumericType::Int32), None)],
        ),
        "SecureStorage.get" => (
            Type::Optional(Box::new(Type::String)),
            true,
            // Secure storage operations can fail because the OS denies access,
            // the device is locked, or the protected data cannot be decoded.
            // Keep failures explicit so callers can recover with `try`.
            vec![("key", Type::String, None)],
        ),
        "SecureStorage.set" => (
            Type::Void,
            true,
            vec![("key", Type::String, None), ("value", Type::String, None)],
        ),
        "SecureStorage.delete" => (Type::Void, true, vec![("key", Type::String, None)]),
        "SecureStorage.clear" => (Type::Void, true, Vec::new()),
        "Storage.getString" => (
            Type::Optional(Box::new(Type::String)),
            false,
            vec![("key", Type::String, None)],
        ),
        "Storage.setString" => (
            Type::Void,
            false,
            vec![("key", Type::String, None), ("value", Type::String, None)],
        ),
        "Storage.delete" => (Type::Void, false, vec![("key", Type::String, None)]),
        "Storage.clear" => (Type::Void, false, Vec::new()),
        "Clipboard.setText" => (Type::Void, false, vec![("text", Type::String, None)]),
        "Clipboard.getText" => (Type::Optional(Box::new(Type::String)), false, Vec::new()),
        "Clipboard.hasText" => (Type::Bool, false, Vec::new()),
        "Haptics.impact" => (Type::Void, false, vec![("style", Type::String, None)]),
        "Haptics.notification" => (Type::Void, false, vec![("kind", Type::String, None)]),
        "Haptics.selection" => (Type::Void, false, Vec::new()),
        "Log.info" | "Log.warning" | "Log.error" => {
            (Type::Void, false, vec![("message", Type::String, None)])
        }
        _ => {
            return Err(CompileError::new(
                span,
                format!("unknown native API `{qualified_name}`"),
            ));
        }
    };
    if is_async && !awaited {
        return Err(CompileError::new(
            span,
            format!("async native call `{qualified_name}` must be awaited"),
        ));
    }
    if awaited && !is_async {
        return Err(CompileError::new(
            span,
            format!("native call `{qualified_name}` is not async and cannot be awaited"),
        ));
    }
    if awaited && !ctx.allow_await {
        return Err(CompileError::new(
            span,
            "`await` is only allowed in an async function or `OnAppear async` block",
        ));
    }
    require_expected(expected, &return_type, span)?;

    let known = specs.iter().map(|(name, ..)| *name).collect::<HashSet<_>>();
    if let Some(unknown) = named_arguments
        .keys()
        .find(|name| !known.contains(name.as_str()))
    {
        return Err(CompileError::new(
            span,
            format!("unknown option `{unknown}` for `{qualified_name}`"),
        ));
    }
    let mut lowered = Vec::with_capacity(specs.len());
    for (argument_name, argument_type, default) in specs {
        let argument = named_arguments
            .get(argument_name)
            .or(default.as_ref())
            .ok_or_else(|| {
                CompileError::new(
                    span,
                    format!("`{qualified_name}` requires `{argument_name}`"),
                )
            })?;
        let lowered_argument = match (qualified_name.as_str(), argument_name, argument) {
            ("Haptics.impact", "style", ast::Expr::Name(name, _))
                if matches!(name.as_str(), "Light" | "Medium" | "Heavy") =>
            {
                Expr::String(name.clone())
            }
            ("Haptics.notification", "kind", ast::Expr::Name(name, _))
                if matches!(name.as_str(), "Success" | "Error") =>
            {
                Expr::String(name.clone())
            }
            ("Screen.lockOrientation", "mode", ast::Expr::Name(name, _))
                if matches!(name.as_str(), "Portrait" | "Landscape" | "All") =>
            {
                Expr::String(name.clone())
            }
            ("Screen.lockOrientation", "mode", value) => {
                return Err(CompileError::new(
                    value.span(),
                    "Screen.lockOrientation mode must be `Portrait`, `Landscape`, or `All`",
                ));
            }
            ("Haptics.impact", "style", value) => {
                return Err(CompileError::new(
                    value.span(),
                    "Haptics.impact style must be `Light`, `Medium`, or `Heavy`",
                ));
            }
            ("Haptics.notification", "kind", value) => {
                return Err(CompileError::new(
                    value.span(),
                    "Haptics.notification kind must be `Success` or `Error`",
                ));
            }
            _ => lower_expr(argument, Some(&argument_type), ctx)?,
        };
        lowered.push((argument_name.to_owned(), lowered_argument));
    }
    native_plan(&qualified_name, lowered, span)
}

/// Builds the validated IR plan for a core native call. Every required
/// argument was just lowered above; a missing entry is a typed error, never
/// a panic, and the returned variants carry no optional bindings for
/// renderers to unwrap.
fn native_plan(
    qualified_name: &str,
    lowered: Vec<(String, Expr)>,
    span: Span,
) -> Result<Expr, CompileError> {
    let take = |name: &str| {
        lowered
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| CompileError::new(span, format!("`{qualified_name}` requires `{name}`")))
    };
    let network_request = || {
        let body = match take("body")? {
            Expr::Null(_) => None,
            body => Some(Box::new(body)),
        };
        Ok::<_, CompileError>(nexa_ir::NetworkRequest {
            url: Box::new(take("url")?),
            method: Box::new(take("method")?),
            headers: Box::new(take("headers")?),
            timeout: Box::new(take("timeout")?),
            use_cache: Box::new(take("useCache")?),
            follow_redirects: Box::new(take("followRedirects")?),
            max_response_bytes: Box::new(take("maxResponseBytes")?),
            certificate_pins: Box::new(take("certificatePins")?),
            body,
        })
    };
    match qualified_name {
        "Network.fetch" => Ok(Expr::NetworkFetch(Box::new(network_request()?))),
        "Network.download" => Ok(Expr::NetworkDownload {
            destination: Box::new(take("destinationPath")?),
            request: Box::new(network_request()?),
        }),
        "Network.upload" => Ok(Expr::NativeCall {
            receiver: None,
            namespace: "Network".to_owned(),
            name: "upload".to_owned(),
            arguments: vec![
                ("url".to_owned(), take("url")?),
                ("file".to_owned(), take("file")?),
                ("fields".to_owned(), take("fields")?),
            ],
            codecs: Vec::new(),
            return_type: Type::NetworkResponse,
            is_async: true,
            is_throwing: true,
        }),
        "Path.documents" | "Path.caches" | "Path.temporary" | "Path.appSupport" => {
            let Some((namespace, name)) = qualified_name.split_once('.') else {
                return Err(CompileError::new(span, "invalid built-in path API"));
            };
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                arguments: Vec::new(),
                codecs: Vec::new(),
                return_type: Type::String,
                is_async: false,
                is_throwing: false,
            })
        }
        "Path.join" => Ok(Expr::PathJoin {
            path: Box::new(take("path")?),
            component: Box::new(take("component")?),
        }),
        "File.exists" => Ok(Expr::FileExists {
            path: Box::new(take("path")?),
        }),
        "File.readText" => Ok(Expr::FileReadText {
            path: Box::new(take("path")?),
        }),
        "File.writeText" => Ok(Expr::FileWriteText {
            path: Box::new(take("path")?),
            contents: Box::new(take("contents")?),
        }),
        "File.delete" => Ok(Expr::FileDelete {
            path: Box::new(take("path")?),
        }),
        "Keyboard.dismiss" => Ok(Expr::NativeCall {
            receiver: None,
            namespace: "Keyboard".to_owned(),
            name: "dismiss".to_owned(),
            arguments: Vec::new(),
            codecs: Vec::new(),
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
        }),
        "Screen.lockOrientation" => Ok(Expr::NativeCall {
            receiver: None,
            namespace: "Screen".to_owned(),
            name: "lockOrientation".to_owned(),
            arguments: vec![("mode".to_owned(), take("mode")?)],
            codecs: Vec::new(),
            return_type: Type::Void,
            is_async: false,
            is_throwing: false,
        }),
        "Permissions.status" => Ok(Expr::PermissionOp {
            op: nexa_ir::PermissionOpKind::Status,
            permission: Box::new(take("permission")?),
        }),
        "Permissions.request" => Ok(Expr::PermissionOp {
            op: nexa_ir::PermissionOpKind::Request,
            permission: Box::new(take("permission")?),
        }),
        "Bytes.fromText" => Ok(Expr::BytesFromText {
            text: Box::new(take("text")?),
        }),
        "Bytes.fromArray" => Ok(Expr::BytesFromArray {
            values: Box::new(take("values")?),
        }),
        "Bytes.count" => Ok(Expr::BytesCount {
            bytes: Box::new(take("bytes")?),
        }),
        "Time.now" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Now,
            arguments: Vec::new(),
            return_type: Type::Numeric(NumericType::Int64),
            is_async: false,
        }),
        "Time.monotonic" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Monotonic,
            arguments: Vec::new(),
            return_type: Type::Numeric(NumericType::Int64),
            is_async: false,
        }),
        "Time.elapsed" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Elapsed,
            arguments: vec![take("since")?],
            return_type: Type::Numeric(NumericType::Int64),
            is_async: false,
        }),
        "Time.sleep" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Sleep,
            arguments: vec![take("milliseconds")?],
            return_type: Type::Void,
            is_async: true,
        }),
        "Time.iso8601" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Iso8601,
            arguments: vec![take("timestamp")?],
            return_type: Type::String,
            is_async: false,
        }),
        "Time.iso8601ToMillis" => Ok(Expr::TimeCall {
            method: nexa_ir::TimeMethod::Iso8601ToMillis,
            arguments: vec![take("text")?],
            return_type: Type::Optional(Box::new(Type::Numeric(NumericType::Int64))),
            is_async: false,
        }),
        "Number.formatCurrency" => Ok(Expr::NativeCall {
            receiver: None,
            namespace: "Number".to_owned(),
            name: "formatCurrency".to_owned(),
            arguments: vec![
                ("amount".to_owned(), take("amount")?),
                ("currencyCode".to_owned(), take("currencyCode")?),
            ],
            codecs: Vec::new(),
            return_type: Type::String,
            is_async: false,
            is_throwing: false,
        }),
        "Crypto.sha256" | "Crypto.sha512" | "Crypto.hmacSha256" | "Crypto.randomBytes" => {
            let Some((namespace, name)) = qualified_name.split_once('.') else {
                return Err(CompileError::new(span, "invalid built-in crypto API"));
            };
            let argument_names: &[&str] = match name {
                "sha256" | "sha512" => &["text"],
                "hmacSha256" => &["key", "message"],
                _ => &["count"],
            };
            let arguments = argument_names
                .iter()
                .map(|argument| Ok(((*argument).to_owned(), take(argument)?)))
                .collect::<Result<Vec<_>, CompileError>>()?;
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type: Type::String,
                is_async: false,
                is_throwing: false,
            })
        }
        "SecureStorage.get"
        | "SecureStorage.set"
        | "SecureStorage.delete"
        | "SecureStorage.clear" => {
            let Some((namespace, name)) = qualified_name.split_once('.') else {
                return Err(CompileError::new(
                    span,
                    "invalid built-in secure storage API",
                ));
            };
            let arguments = match name {
                "get" | "delete" => vec![("key".to_owned(), take("key")?)],
                "set" => vec![
                    ("key".to_owned(), take("key")?),
                    ("value".to_owned(), take("value")?),
                ],
                _ => Vec::new(),
            };
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type: if name == "get" {
                    Type::Optional(Box::new(Type::String))
                } else {
                    Type::Void
                },
                is_async: true,
                is_throwing: true,
            })
        }
        "Storage.getString" | "Storage.setString" | "Storage.delete" | "Storage.clear" => {
            let arguments = match qualified_name {
                "Storage.getString" | "Storage.delete" => {
                    vec![("key".to_owned(), take("key")?)]
                }
                "Storage.setString" => vec![
                    ("key".to_owned(), take("key")?),
                    ("value".to_owned(), take("value")?),
                ],
                _ => Vec::new(),
            };
            let (namespace, name) = qualified_name
                .split_once('.')
                .ok_or_else(|| CompileError::new(span, "invalid storage API"))?;
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type: if name == "getString" {
                    Type::Optional(Box::new(Type::String))
                } else {
                    Type::Void
                },
                is_async: false,
                is_throwing: false,
            })
        }
        "Clipboard.setText" | "Clipboard.getText" | "Clipboard.hasText" => {
            let Some((namespace, name)) = qualified_name.split_once('.') else {
                return Err(CompileError::new(span, "invalid clipboard API"));
            };
            let arguments = if name == "setText" {
                vec![("text".to_owned(), take("text")?)]
            } else {
                Vec::new()
            };
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: namespace.to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type: match name {
                    "getText" => Type::Optional(Box::new(Type::String)),
                    "hasText" => Type::Bool,
                    _ => Type::Void,
                },
                is_async: false,
                is_throwing: false,
            })
        }
        "Haptics.impact" | "Haptics.notification" | "Haptics.selection" => {
            let name = match qualified_name {
                "Haptics.impact" => "impact",
                "Haptics.notification" => "notification",
                "Haptics.selection" => "selection",
                _ => {
                    return Err(CompileError::new(
                        span,
                        format!("unknown native API `{qualified_name}`"),
                    ));
                }
            };
            let arguments = match qualified_name {
                "Haptics.impact" => vec![("style".to_owned(), take("style")?)],
                "Haptics.notification" => vec![("kind".to_owned(), take("kind")?)],
                _ => Vec::new(),
            };
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: "Haptics".to_owned(),
                name: name.to_owned(),
                arguments,
                codecs: Vec::new(),
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
            })
        }
        "Log.info" | "Log.warning" | "Log.error" => Ok(Expr::LogCall {
            method: match qualified_name {
                "Log.info" => nexa_ir::LogMethod::Info,
                "Log.warning" => nexa_ir::LogMethod::Warning,
                _ => nexa_ir::LogMethod::Error,
            },
            message: Box::new(take("message")?),
        }),
        _ => Err(CompileError::new(
            span,
            format!("unknown native API `{qualified_name}`"),
        )),
    }
}

fn is_core_native_namespace(namespace: &str) -> bool {
    matches!(
        namespace,
        "Network"
            | "Path"
            | "File"
            | "Permissions"
            | "Bytes"
            | "Time"
            | "Log"
            | "Keyboard"
            | "Number"
            | "Json"
            | "Crypto"
            | "SecureStorage"
            | "Storage"
            | "Clipboard"
            | "Haptics"
    )
}

fn lower_json_call(name: &str, call: CallArguments<'_, '_>) -> Result<Expr, CompileError> {
    let CallArguments {
        type_arguments,
        arguments,
        named_arguments,
        span,
        expected,
        ctx,
        awaited,
    } = call;
    if awaited {
        return Err(CompileError::new(
            span,
            format!("native call `Json.{name}` is not async and cannot be awaited"),
        ));
    }
    if !arguments.is_empty() {
        return Err(CompileError::new(
            span,
            "built-in native API calls require named arguments",
        ));
    }
    match name {
        "parse" => {
            let raw_type = Type::String;
            let expected_value = match expected {
                Some(Type::Result(value, error)) if matches!(error.as_ref(), Type::Enum(error_name) if error_name == "JsonError") => {
                    Some(value.as_ref().clone())
                }
                Some(other) => {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "`Json.parse` returns `Result<T, JsonError>`, found expected {}",
                            type_name(other)
                        ),
                    ));
                }
                None => None,
            };
            if type_arguments.len() > 1 {
                return Err(CompileError::new(
                    span,
                    "`Json.parse` accepts one type argument, for example `Json.parse<User>(raw: text)`",
                ));
            }
            let explicit_value = type_arguments
                .first()
                .map(parse_type)
                .transpose()?
                .map(|ty| resolve_struct_type(&ty, ctx.structs));
            let value_type = match (explicit_value, expected_value) {
                (Some(explicit), Some(expected)) if explicit != expected => {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "`Json.parse` type argument is {}, but the expected result contains {}",
                            type_name(&explicit),
                            type_name(&expected)
                        ),
                    ));
                }
                (Some(explicit), _) => explicit,
                (None, Some(expected)) => expected,
                (None, None) => {
                    return Err(CompileError::new(
                        span,
                        "`Json.parse` needs a result type; write `Json.parse<User>(raw: text)` or annotate the result as `Result<User, JsonError>`",
                    ));
                }
            };
            validate_json_value_type(&value_type, ctx.enums, span)?;
            let return_type = Type::Result(
                Box::new(value_type.clone()),
                Box::new(Type::Enum("JsonError".to_owned())),
            );
            require_expected(expected, &return_type, span)?;
            let raw = named_arguments.get("raw").ok_or_else(|| {
                CompileError::new(span, "`Json.parse` requires the `raw` argument")
            })?;
            if named_arguments.keys().any(|key| key != "raw") {
                return Err(CompileError::new(
                    span,
                    "`Json.parse` accepts only the `raw` argument",
                ));
            }
            let lowered = lower_expr(raw, Some(&raw_type), ctx)?;
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: "Json".to_owned(),
                name: "parse".to_owned(),
                arguments: vec![("raw".to_owned(), lowered)],
                codecs: vec![PluginCodec {
                    ty: value_type,
                    decodes: true,
                }],
                return_type,
                is_async: false,
                is_throwing: false,
            })
        }
        "stringify" => {
            if type_arguments.len() > 1 {
                return Err(CompileError::new(
                    span,
                    "`Json.stringify` accepts at most one type argument",
                ));
            }
            let value = named_arguments.get("value").ok_or_else(|| {
                CompileError::new(span, "`Json.stringify` requires the `value` argument")
            })?;
            if named_arguments.keys().any(|key| key != "value") {
                return Err(CompileError::new(
                    span,
                    "`Json.stringify` accepts only the `value` argument",
                ));
            }
            let inferred = infer_expr_type(value, ctx.symbols, ctx.functions)
                .map(|ty| resolve_struct_type(&ty, ctx.structs));
            let explicit = type_arguments
                .first()
                .map(parse_type)
                .transpose()?
                .map(|ty| resolve_struct_type(&ty, ctx.structs));
            let value_type = match (explicit, inferred) {
                (Some(explicit), Some(inferred)) if explicit != inferred => {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "`Json.stringify` type argument is {}, but the value has type {}",
                            type_name(&explicit),
                            type_name(&inferred)
                        ),
                    ));
                }
                (Some(explicit), _) => explicit,
                (None, Some(inferred)) => inferred,
                (None, None) => {
                    return Err(CompileError::new(
                        span,
                        "cannot infer the JSON value type; add a type argument such as `Json.stringify<User>(value: user)`",
                    ));
                }
            };
            validate_json_value_type(&value_type, ctx.enums, span)?;
            let lowered = lower_expr(value, Some(&value_type), ctx)?;
            Ok(Expr::NativeCall {
                receiver: None,
                namespace: "Json".to_owned(),
                name: "stringify".to_owned(),
                arguments: vec![("value".to_owned(), lowered)],
                codecs: vec![PluginCodec {
                    ty: value_type,
                    decodes: false,
                }],
                return_type: Type::String,
                is_async: false,
                is_throwing: false,
            })
        }
        _ => Err(CompileError::new(
            span,
            format!("unknown native API `Json.{name}`"),
        )),
    }
}

fn validate_json_value_type(
    ty: &Type,
    enum_names: &HashSet<String>,
    span: Span,
) -> Result<(), CompileError> {
    let unsupported = |message: &str| CompileError::new(span, message);
    match ty {
        Type::String | Type::Bytes | Type::Bool | Type::Numeric(_) => Ok(()),
        Type::Enum(name) if enum_names.contains(name) => Ok(()),
        Type::Enum(name) => Err(CompileError::new(
            span,
            format!("unknown JSON enum type `{name}`"),
        )),
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) => {
            validate_json_value_type(inner, enum_names, span)
        }
        Type::Map(key, value) if matches!(key.as_ref(), Type::String) => {
            validate_json_value_type(value, enum_names, span)
        }
        Type::Map(_, _) => Err(unsupported(
            "JSON maps require `String` keys because JSON object keys are strings",
        )),
        Type::Pair(first, second) => {
            validate_json_value_type(first, enum_names, span)?;
            validate_json_value_type(second, enum_names, span)
        }
        Type::Result(success, failure) => {
            if !matches!(failure.as_ref(), Type::Enum(_)) {
                return Err(unsupported("JSON Result values require an enum error type"));
            }
            validate_json_value_type(success, enum_names, span)?;
            validate_json_value_type(failure, enum_names, span)
        }
        Type::Triple(first, second, third) => {
            validate_json_value_type(first, enum_names, span)?;
            validate_json_value_type(second, enum_names, span)?;
            validate_json_value_type(third, enum_names, span)
        }
        Type::Struct { fields, .. } => fields
            .iter()
            .try_for_each(|(_, field)| validate_json_value_type(field, enum_names, span)),
        Type::Void => Err(unsupported("`Void` is not a JSON value type")),
        Type::TypeParam(_) => Err(unsupported(
            "JSON codecs require a concrete value type at compile time",
        )),
        Type::Plugin { .. } => Err(unsupported(
            "native class instances cannot be represented as JSON values",
        )),
        Type::NetworkResponse => Err(unsupported(
            "`NetworkResponse` cannot be represented directly as a JSON value; decode its body string instead",
        )),
    }
}

fn lower_plugin_call(
    namespace: &str,
    name: &str,
    call: CallArguments<'_, '_>,
) -> Result<Expr, CompileError> {
    let CallArguments {
        type_arguments,
        arguments,
        named_arguments,
        span,
        expected,
        ctx,
        awaited,
    } = call;
    let qualified_name = format!("{namespace}.{name}");
    let signature = ctx
        .functions
        .get(&qualified_name)
        .expect("plugin signature checked before lowering");
    reject_unhandled_plugin_errors(&qualified_name, signature, span)?;
    if signature.is_async && !awaited {
        return Err(CompileError::new(
            span,
            format!("async plugin call `{qualified_name}` must be awaited"),
        ));
    }
    if awaited && !signature.is_async {
        return Err(CompileError::new(
            span,
            format!("plugin call `{qualified_name}` is not async and cannot be awaited"),
        ));
    }
    if awaited && !ctx.allow_await {
        return Err(CompileError::new(
            span,
            "`await` is only allowed in an async function or `OnAppear async` block",
        ));
    }
    let argument_types = arguments
        .iter()
        .map(|argument| infer_expr_type(argument, ctx.symbols, ctx.functions))
        .collect::<Vec<_>>();
    let resolved = super::generics::resolve(
        &qualified_name,
        signature,
        type_arguments,
        &argument_types,
        expected,
        TypeRegistries {
            structs: ctx.structs,
            enums: ctx.enums,
        },
        span,
    )?;
    let (parameter_types, return_type, codecs) = match resolved {
        Some(resolved) => (resolved.parameters, resolved.return_type, resolved.codecs),
        None => (
            signature.parameters.clone(),
            signature.return_type.clone(),
            Vec::new(),
        ),
    };
    if !expected.is_some_and(crate::semantic::generics::mentions_type_parameter) {
        require_expected(expected, &return_type, span)?;
    }
    if !named_arguments.is_empty() {
        return Err(CompileError::new(
            span,
            "Nexa plugin calls use positional arguments in declaration order",
        ));
    }
    if arguments.len() != parameter_types.len() {
        return Err(CompileError::new(
            span,
            format!(
                "plugin method `{qualified_name}` expects {} argument(s), found {}",
                parameter_types.len(),
                arguments.len()
            ),
        ));
    }
    let mut lowered = Vec::with_capacity(parameter_types.len());
    for ((argument_name, argument_type), argument) in parameter_types.iter().zip(arguments.iter()) {
        lowered.push((
            argument_name.clone(),
            lower_expr(argument, Some(argument_type), ctx)?,
        ));
    }
    if signature.is_constructor {
        let Type::Plugin {
            name: class_name, ..
        } = &signature.return_type
        else {
            return Err(CompileError::new(
                span,
                format!("plugin constructor `{qualified_name}` has an invalid return type"),
            ));
        };
        return Ok(Expr::Call {
            name: class_name.clone(),
            arguments: lowered.into_iter().map(|(_, argument)| argument).collect(),
            return_type: signature.return_type.clone(),
            is_async: false,
            is_constructor: true,
        });
    }
    Ok(Expr::NativeCall {
        receiver: None,
        namespace: namespace.to_owned(),
        name: name.to_owned(),
        arguments: lowered,
        codecs,
        return_type,
        is_async: signature.is_async,
        is_throwing: signature.is_throwing,
    })
}

fn lower_plugin_method_call(
    base: &ast::Expr,
    name: &str,
    call: CallArguments<'_, '_>,
) -> Result<Expr, CompileError> {
    let CallArguments {
        type_arguments,
        arguments,
        named_arguments,
        span,
        expected,
        ctx,
        awaited,
    } = call;
    let Some(base_type) = infer_expr_type(base, ctx.symbols, ctx.functions) else {
        return Err(CompileError::new(
            span,
            "native object method calls require a native class instance",
        ));
    };
    let Type::Plugin {
        namespace,
        name: class,
    } = &base_type
    else {
        return Err(CompileError::new(
            span,
            "native object method calls require a native class instance",
        ));
    };
    let qualified_name = format!("{class}.{name}");
    let Some(signature) = ctx.functions.get(&qualified_name) else {
        return Err(CompileError::new(
            span,
            format!("unknown native class method `{class}.{name}`"),
        ));
    };
    reject_unhandled_plugin_errors(&qualified_name, signature, span)?;
    if signature.receiver.as_ref() != Some(&base_type) {
        return Err(CompileError::new(
            span,
            format!("method `{name}` is not available on `{class}`"),
        ));
    }
    if !named_arguments.is_empty() {
        return Err(CompileError::new(
            span,
            "Nexa plugin calls use positional arguments in declaration order",
        ));
    }
    if arguments.len() != signature.parameters.len() {
        return Err(CompileError::new(
            span,
            format!(
                "native class method `{class}.{name}` expects {} argument(s), found {}",
                signature.parameters.len(),
                arguments.len()
            ),
        ));
    }
    if signature.is_async && !awaited {
        return Err(CompileError::new(
            span,
            format!("async native class method `{class}.{name}` must be awaited"),
        ));
    }
    if awaited && !signature.is_async {
        return Err(CompileError::new(
            span,
            format!("native class method `{class}.{name}` is not async and cannot be awaited"),
        ));
    }
    if awaited && !ctx.allow_await {
        return Err(CompileError::new(
            span,
            "`await` is only allowed in an async function or `OnAppear async` block",
        ));
    }
    let argument_types = arguments
        .iter()
        .map(|argument| infer_expr_type(argument, ctx.symbols, ctx.functions))
        .collect::<Vec<_>>();
    let resolved = super::generics::resolve(
        &qualified_name,
        signature,
        type_arguments,
        &argument_types,
        expected,
        TypeRegistries {
            structs: ctx.structs,
            enums: ctx.enums,
        },
        span,
    )?;
    let (parameter_types, return_type, codecs) = match resolved {
        Some(resolved) => (resolved.parameters, resolved.return_type, resolved.codecs),
        None => (
            signature.parameters.clone(),
            signature.return_type.clone(),
            Vec::new(),
        ),
    };
    if !expected.is_some_and(crate::semantic::generics::mentions_type_parameter) {
        require_expected(expected, &return_type, span)?;
    }
    let receiver = lower_expr(base, Some(&base_type), ctx)?;
    let lowered = parameter_types
        .iter()
        .zip(arguments.iter())
        .map(|((_, ty), argument)| lower_expr(argument, Some(ty), ctx))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expr::NativeCall {
        receiver: Some(Box::new(receiver)),
        namespace: namespace.clone(),
        name: name.to_owned(),
        codecs,
        return_type,
        arguments: parameter_types
            .iter()
            .map(|(name, _)| name.clone())
            .zip(lowered)
            .collect(),
        is_async: signature.is_async,
        is_throwing: signature.is_throwing,
    })
}

fn reject_unhandled_plugin_errors(
    qualified_name: &str,
    signature: &FunctionSignature,
    span: Span,
) -> Result<(), CompileError> {
    if signature.is_throwing && !signature.error_handling_allowed {
        return Err(CompileError::new(
            span,
            format!(
                "plugin API `{qualified_name}` may throw; wrap the call in a `try {{ ... }} catch {{ ... }}` action to handle its failure"
            ),
        ));
    }
    Ok(())
}

pub(super) fn functions_with_error_handling(
    functions: &FunctionSignatures,
    allowed: bool,
) -> FunctionSignatures {
    let mut scoped = functions
        .iter()
        .map(|(name, signature)| {
            let mut signature = signature.clone();
            signature.error_handling_allowed = allowed && signature.is_throwing;
            (name.clone(), signature)
        })
        .collect::<FunctionSignatures>();
    if allowed {
        // Built-in throwing calls such as File.readText do not have normal
        // plugin signatures. Carry the lexical catch scope through the same
        // expression-lowering context with an impossible-to-collide key; this
        // marker is never resolved as a source-level call or emitted to IR.
        scoped.insert(
            ERROR_HANDLING_SCOPE_KEY.to_owned(),
            FunctionSignature {
                parameters: Vec::new(),
                type_parameters: Vec::new(),
                return_type: Type::Void,
                is_async: false,
                is_throwing: true,
                receiver: None,
                is_constructor: false,
                is_mutable_property: false,
                error_handling_allowed: true,
                error_type: None,
            },
        );
    }
    scoped
}

pub(super) fn plugin_error_variant(
    functions: &FunctionSignatures,
    namespace: &str,
    error_name: &str,
    variant_name: &str,
) -> Option<PluginErrorVariant> {
    functions.values().find_map(|signature| {
        let error_type = signature.error_type.as_ref()?;
        if error_type.namespace != namespace || error_type.name != error_name {
            return None;
        }
        error_type
            .variants
            .iter()
            .find(|variant| variant.name == variant_name)
            .cloned()
    })
}

fn network_specs(span: Span, download: bool) -> Vec<(&'static str, Type, Option<ast::Expr>)> {
    let string = || Type::String;
    let optional_string = || Type::Optional(Box::new(Type::String));
    let empty_map = ast::Expr::Map(Vec::new(), span);
    let empty_set = ast::Expr::Array(Vec::new(), span);
    let mut specs = vec![
        ("url", string(), None),
        (
            "method",
            string(),
            Some(ast::Expr::String("GET".to_owned(), span)),
        ),
        ("body", optional_string(), Some(ast::Expr::Null(span))),
        (
            "headers",
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
            Some(empty_map),
        ),
        (
            "timeout",
            Type::Numeric(NumericType::Float64),
            Some(ast::Expr::Number("30".to_owned(), span)),
        ),
        ("useCache", Type::Bool, Some(ast::Expr::Bool(true, span))),
        (
            "followRedirects",
            Type::Bool,
            Some(ast::Expr::Bool(true, span)),
        ),
        (
            "maxResponseBytes",
            Type::Numeric(NumericType::Int64),
            Some(ast::Expr::Number("67108864".to_owned(), span)),
        ),
        (
            "certificatePins",
            Type::Set(Box::new(Type::String)),
            Some(empty_set),
        ),
    ];
    if download {
        specs.insert(1, ("destinationPath", string(), None));
    }
    specs
}

fn lower_binary(
    left: &ast::Expr,
    operator: ast::BinaryOp,
    right: &ast::Expr,
    span: Span,
    expected: Option<&Type>,
    ctx: &ExprContext<'_>,
) -> Result<Expr, CompileError> {
    let ir_operator = match operator {
        ast::BinaryOp::And => BinaryOp::And,
        ast::BinaryOp::Or => BinaryOp::Or,
        ast::BinaryOp::Contains => BinaryOp::Contains,
        ast::BinaryOp::Equal => BinaryOp::Equal,
        ast::BinaryOp::NotEqual => BinaryOp::NotEqual,
        ast::BinaryOp::Less => BinaryOp::Less,
        ast::BinaryOp::LessEqual => BinaryOp::LessEqual,
        ast::BinaryOp::Greater => BinaryOp::Greater,
        ast::BinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
    };
    let is_logical = matches!(operator, ast::BinaryOp::And | ast::BinaryOp::Or);
    if matches!(operator, ast::BinaryOp::Contains) {
        if expected.is_some_and(|expected| expected != &Type::Bool) {
            return Err(CompileError::new(
                span,
                "membership expressions produce Bool",
            ));
        }
        let Some(collection_type) = infer_expr_type(right, ctx.symbols, ctx.functions) else {
            return Err(CompileError::new(
                span,
                "the right side of `in` must be an Array<T>, Set<T>, or Map<K, V>",
            ));
        };
        let element_type = match &collection_type {
            Type::Array(element) | Type::Set(element) => element.as_ref(),
            Type::Map(key, _) => key.as_ref(),
            _ => {
                return Err(CompileError::new(
                    span,
                    "the right side of `in` must be an Array<T>, Set<T>, or Map<K, V>",
                ));
            }
        };
        if !matches!(element_type, Type::String | Type::Bool | Type::Numeric(_)) {
            return Err(CompileError::new(
                span,
                "membership currently supports scalar elements and keys only",
            ));
        }
        let left = lower_expr(left, Some(element_type), ctx)?;
        let right = lower_expr(right, Some(&collection_type), ctx)?;
        return Ok(Expr::Contains {
            value: Box::new(left),
            collection: Box::new(right),
            collection_type,
        });
    }
    let is_ordered = matches!(
        operator,
        ast::BinaryOp::Less
            | ast::BinaryOp::LessEqual
            | ast::BinaryOp::Greater
            | ast::BinaryOp::GreaterEqual
    );
    if is_logical {
        if expected.is_some_and(|expected| expected != &Type::Bool) {
            return Err(CompileError::new(span, "logical expressions produce Bool"));
        }
        let bool_type = Type::Bool;
        let left = lower_expr(left, Some(&bool_type), ctx)?;
        let right = lower_expr(right, Some(&bool_type), ctx)?;
        return Ok(Expr::Binary {
            op: ir_operator,
            left: Box::new(left),
            right: Box::new(right),
        });
    }

    if expected.is_some_and(|expected| expected != &Type::Bool) {
        return Err(CompileError::new(
            span,
            "comparison expressions produce Bool",
        ));
    }
    let left_type = infer_expr_type(left, ctx.symbols, ctx.functions);
    let right_type = infer_expr_type(right, ctx.symbols, ctx.functions);
    let common_type = match (left_type, right_type) {
        (None, Some(right_type)) if !is_ordered && matches!(left, ast::Expr::Null(_)) => right_type,
        (Some(left_type), None) if !is_ordered && matches!(right, ast::Expr::Null(_)) => left_type,
        (Some(left_type), Some(right_type)) if left_type == right_type => left_type,
        (Some(left_type @ Type::Numeric(_)), Some(Type::Numeric(_)))
            if matches!(right, ast::Expr::Number(_, _)) =>
        {
            left_type
        }
        (Some(Type::Numeric(_)), Some(right_type @ Type::Numeric(_)))
            if matches!(left, ast::Expr::Number(_, _)) =>
        {
            right_type
        }
        (Some(left_type), Some(right_type)) => {
            return Err(CompileError::new(
                span,
                format!(
                    "cannot compare {} with {} without an explicit conversion",
                    type_name(&left_type),
                    type_name(&right_type)
                ),
            ));
        }
        (Some(ty), None) | (None, Some(ty)) => ty,
        (None, None) => {
            let left = lower_expr(left, None, ctx)?;
            let right = lower_expr(right, None, ctx)?;
            let _ = (left, right);
            return Err(CompileError::new(
                span,
                "comparison requires typed scalar values",
            ));
        }
    };
    if is_ordered && !matches!(common_type, Type::Numeric(_)) {
        return Err(CompileError::new(
            span,
            "ordering comparisons are supported for numeric values only",
        ));
    }
    if !is_ordered && !is_equatable_type(&common_type) {
        return Err(CompileError::new(
            span,
            "equality is supported for scalar, enum, optional, collection, pair, triple, and value-struct types",
        ));
    }
    let left = lower_expr(left, Some(&common_type), ctx)?;
    let right = lower_expr(right, Some(&common_type), ctx)?;
    Ok(Expr::Binary {
        op: ir_operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn is_equatable_type(ty: &Type) -> bool {
    match ty {
        Type::String | Type::Bool | Type::Numeric(_) | Type::Enum(_) => true,
        // A type parameter is only ever compared after it is bound, and a
        // bound value type is checked on its own.
        Type::TypeParam(_) => false,
        Type::Optional(inner) => is_equatable_type(inner),
        Type::Array(element) | Type::Set(element) => is_equatable_type(element),
        Type::Map(key, value) => is_equatable_type(key) && is_equatable_type(value),
        Type::Pair(first, second) | Type::Result(first, second) => {
            is_equatable_type(first) && is_equatable_type(second)
        }
        Type::Triple(first, second, third) => {
            is_equatable_type(first) && is_equatable_type(second) && is_equatable_type(third)
        }
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|(_, field)| is_equatable_type(field))
        }
        Type::Void | Type::Bytes | Type::Plugin { .. } | Type::NetworkResponse => false,
    }
}

pub(super) fn infer_expr_type(
    expr: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Option<Type> {
    match expr {
        ast::Expr::String(_, _) | ast::Expr::Interpolation(_, _) => Some(Type::String),
        ast::Expr::Bool(_, _)
        | ast::Expr::IsRegularWidth(_)
        | ast::Expr::IsCompactWidth(_)
        | ast::Expr::IsRegularHeight(_)
        | ast::Expr::IsCompactHeight(_)
        | ast::Expr::Not(_, _)
        | ast::Expr::Binary(_, _, _, _) => Some(Type::Bool),
        ast::Expr::Number(raw, _) => Some(Type::Numeric(if raw.contains('.') {
            NumericType::Float64
        } else {
            NumericType::Int32
        })),
        ast::Expr::Name(name, _) => symbols.get(name).map(|(ty, _)| ty.clone()),
        ast::Expr::EnumCase {
            enum_name,
            case_name,
            ..
        } => {
            if enum_name == "PermissionStatus" && is_permission_status_case(case_name) {
                Some(Type::Enum("PermissionStatus".to_owned()))
            } else if enum_name == "Permission" && is_permission_case(case_name) {
                Some(Type::Enum("Permission".to_owned()))
            } else {
                symbols
                    .get(&format!("{enum_name}.{case_name}"))
                    .map(|(ty, _)| ty.clone())
            }
        }
        ast::Expr::Call(name, _, args, _) => {
            if name == "Ok" && args.len() == 1 {
                infer_expr_type(&args[0], symbols, functions)
                    .map(|v| Type::Result(Box::new(v), Box::new(Type::String)))
            } else if name == "Err" && args.len() == 1 {
                infer_expr_type(&args[0], symbols, functions)
                    .map(|e| Type::Result(Box::new(Type::Void), Box::new(e)))
            } else {
                functions
                    .get(name)
                    .map(|signature| signature.return_type.clone())
            }
        }
        ast::Expr::CallNamed { name, .. } => functions
            .get(name)
            .map(|signature| signature.return_type.clone()),
        ast::Expr::QualifiedCall {
            namespace, name, ..
        } => match (namespace.as_str(), name.as_str()) {
            ("Network", "fetch" | "upload") => Some(Type::NetworkResponse),
            ("Network", "download")
            | ("File", "writeText")
            | ("File", "delete")
            | ("File", "exists") => Some(Type::Bool),
            ("Path", "documents")
            | ("Path", "caches")
            | ("Path", "temporary")
            | ("Path", "appSupport")
            | ("Path", "join")
            | ("File", "readText") => Some(Type::String),
            ("Bytes", "fromText") | ("Bytes", "fromArray") => Some(Type::Bytes),
            ("Bytes", "count") => Some(Type::Numeric(NumericType::Int32)),
            ("Clipboard", "getText") => Some(Type::Optional(Box::new(Type::String))),
            ("Clipboard", "hasText") => Some(Type::Bool),
            ("Clipboard", "setText") => Some(Type::Void),
            ("Storage", "getString") => Some(Type::Optional(Box::new(Type::String))),
            ("Storage", "setString" | "delete" | "clear") => Some(Type::Void),
            ("Haptics", "impact" | "notification" | "selection") => Some(Type::Void),
            ("Screen", "lockOrientation") => Some(Type::Void),
            ("Time", "now") | ("Time", "monotonic") => Some(Type::Numeric(NumericType::Int64)),
            ("Time", "sleep") => Some(Type::Void),
            ("Time", "iso8601") => Some(Type::String),
            ("Time", "iso8601ToMillis") => {
                Some(Type::Optional(Box::new(Type::Numeric(NumericType::Int64))))
            }
            ("Permissions", "status") => Some(Type::Enum("PermissionStatus".to_owned())),
            ("Permissions", "request") => Some(Type::Enum("PermissionStatus".to_owned())),
            _ => functions
                .get(&format!("{namespace}.{name}"))
                .map(|signature| signature.return_type.clone()),
        },
        ast::Expr::Index {
            collection,
            optional,
            ..
        } => match infer_expr_type(collection, symbols, functions) {
            Some(Type::Array(element_type)) => Some(if *optional {
                Type::Optional(element_type)
            } else {
                *element_type
            }),
            Some(Type::Map(_, value_type)) => Some(Type::Optional(value_type)),
            Some(Type::Optional(inner)) if *optional => match *inner {
                Type::Array(element_type) => Some(Type::Optional(element_type)),
                Type::Map(_, value_type) => Some(Type::Optional(value_type)),
                _ => None,
            },
            _ => None,
        },
        ast::Expr::Member {
            base,
            name,
            optional,
            ..
        } => {
            if !*optional
                && name == "isOnline"
                && matches!(base.as_ref(), ast::Expr::Name(namespace, _) if namespace == "Network")
            {
                return Some(Type::Bool);
            }
            if !*optional && let ast::Expr::Name(enum_name, _) = base.as_ref() {
                if enum_name == "PermissionStatus" && is_permission_status_case(name) {
                    return Some(Type::Enum("PermissionStatus".to_owned()));
                }
                if enum_name == "Permission" && is_permission_case(name) {
                    return Some(Type::Enum("Permission".to_owned()));
                }
                if let Some((ty @ Type::Enum(_), _)) = symbols.get(&format!("{enum_name}.{name}")) {
                    return Some(ty.clone());
                }
            }
            infer_expr_type(base, symbols, functions).and_then(|base_type| {
                if *optional {
                    let Type::Optional(inner) = base_type else {
                        return None;
                    };
                    member_field_type_with_plugins(&inner, name, functions)
                        .map(|field| Type::Optional(Box::new(field)))
                } else {
                    member_field_type_with_plugins(&base_type, name, functions)
                }
            })
        }
        ast::Expr::MethodCall {
            base,
            name,
            arguments,
            ..
        } => {
            if let Some(Type::Plugin { name: class, .. }) =
                infer_expr_type(base, symbols, functions)
            {
                functions
                    .get(&format!("{class}.{name}"))
                    .filter(|signature| signature.receiver.is_some())
                    .map(|signature| signature.return_type.clone())
            } else {
                infer_collection_utility_type(base, name, arguments, symbols, functions).or_else(
                    || infer_collection_transform_type(base, name, arguments, symbols, functions),
                )
            }
        }
        ast::Expr::Range { .. } => None,
        ast::Expr::Null(_) => None,
        ast::Expr::Coalesce(left, right, _) => {
            let Some(Type::Optional(inner)) = infer_expr_type(left, symbols, functions) else {
                return None;
            };
            let right_type = infer_expr_type(right, symbols, functions)?;
            if *inner == right_type {
                Some(*inner)
            } else {
                None
            }
        }
        ast::Expr::Conditional {
            then_value,
            else_value,
            ..
        } => conditional_value_type(then_value, else_value, symbols, functions),
        ast::Expr::Await(value, _) => infer_expr_type(value, symbols, functions),
        ast::Expr::Try { expr, .. } => match infer_expr_type(expr, symbols, functions)? {
            Type::Result(value_type, _) => Some(*value_type),
            _ => None,
        },
        ast::Expr::ThemeToken(_, _) => None,
        ast::Expr::Closure { .. } => None,
        ast::Expr::Add(left_expr, right_expr, _) => {
            let left_type = infer_expr_type(left_expr, symbols, functions);
            let right_type = infer_expr_type(right_expr, symbols, functions);
            match (left_type, right_type) {
                (Some(Type::String), Some(Type::String)) => Some(Type::String),
                (Some(Type::Numeric(left)), Some(Type::Numeric(right))) if left == right => {
                    Some(Type::Numeric(left))
                }
                (Some(Type::Numeric(left)), Some(Type::Numeric(_)))
                    if matches!(right_expr.as_ref(), ast::Expr::Number(_, _)) =>
                {
                    Some(Type::Numeric(left))
                }
                (Some(Type::Numeric(_)), Some(Type::Numeric(right)))
                    if matches!(left_expr.as_ref(), ast::Expr::Number(_, _)) =>
                {
                    Some(Type::Numeric(right))
                }
                (Some(ty @ Type::Numeric(_)), None) | (None, Some(ty @ Type::Numeric(_))) => {
                    Some(ty)
                }
                _ => None,
            }
        }
        ast::Expr::Arithmetic(left_expr, _, right_expr, _) => {
            let left_type = infer_expr_type(left_expr, symbols, functions);
            let right_type = infer_expr_type(right_expr, symbols, functions);
            match (left_type, right_type) {
                (Some(Type::Numeric(left)), Some(Type::Numeric(right))) if left == right => {
                    Some(Type::Numeric(left))
                }
                (Some(Type::Numeric(left)), Some(Type::Numeric(_)))
                    if matches!(right_expr.as_ref(), ast::Expr::Number(_, _)) =>
                {
                    Some(Type::Numeric(left))
                }
                (Some(Type::Numeric(_)), Some(Type::Numeric(right)))
                    if matches!(left_expr.as_ref(), ast::Expr::Number(_, _)) =>
                {
                    Some(Type::Numeric(right))
                }
                (Some(ty @ Type::Numeric(_)), None) | (None, Some(ty @ Type::Numeric(_))) => {
                    Some(ty)
                }
                _ => None,
            }
        }
        ast::Expr::Negate(value, _) => infer_expr_type(value, symbols, functions),
        ast::Expr::Array(items, _) => items
            .first()
            .and_then(|item| infer_expr_type(item, symbols, functions))
            .map(|item_type| Type::Array(Box::new(item_type))),
        ast::Expr::Map(entries, _) => {
            let (key, value) = entries.first()?;
            Some(Type::Map(
                Box::new(infer_expr_type(key, symbols, functions)?),
                Box::new(infer_expr_type(value, symbols, functions)?),
            ))
        }
        ast::Expr::Pair(first, second, _) => Some(Type::Pair(
            Box::new(infer_expr_type(first, symbols, functions)?),
            Box::new(infer_expr_type(second, symbols, functions)?),
        )),
        ast::Expr::Triple(first, second, third, _) => Some(Type::Triple(
            Box::new(infer_expr_type(first, symbols, functions)?),
            Box::new(infer_expr_type(second, symbols, functions)?),
            Box::new(infer_expr_type(third, symbols, functions)?),
        )),
    }
}

fn conditional_value_type(
    then_value: &ast::Expr,
    else_value: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Option<Type> {
    let then_type = infer_expr_type(then_value, symbols, functions);
    let else_type = infer_expr_type(else_value, symbols, functions);
    match (then_type, else_type) {
        (Some(then_type), Some(else_type)) if then_type == else_type => Some(then_type),
        (Some(Type::Optional(inner)), Some(else_type)) if *inner == else_type => {
            Some(Type::Optional(inner))
        }
        (Some(then_type), Some(Type::Optional(inner))) if then_type == *inner => {
            Some(Type::Optional(inner))
        }
        (None, Some(else_type)) if matches!(then_value, ast::Expr::Null(_)) => {
            Some(optional_type(else_type))
        }
        (Some(then_type), None) if matches!(else_value, ast::Expr::Null(_)) => {
            Some(optional_type(then_type))
        }
        _ => None,
    }
}

fn optional_type(ty: Type) -> Type {
    if matches!(ty, Type::Optional(_)) {
        ty
    } else {
        Type::Optional(Box::new(ty))
    }
}

fn as_numeric_type(ty: &Type) -> Option<NumericType> {
    match ty {
        Type::Numeric(numeric_type) => Some(*numeric_type),
        _ => None,
    }
}

fn member_field_type(base_type: &Type, name: &str) -> Option<Type> {
    match (base_type, name) {
        (Type::Pair(first, _), "first") => Some((**first).clone()),
        (Type::Pair(_, second), "second") => Some((**second).clone()),
        (Type::Triple(first, _, _), "first") => Some((**first).clone()),
        (Type::Triple(_, second, _), "second") => Some((**second).clone()),
        (Type::Triple(_, _, third), "third") => Some((**third).clone()),
        (Type::Struct { fields, .. }, field_name) => fields
            .iter()
            .find(|(field, _)| field == field_name)
            .map(|(_, ty)| ty.clone()),
        (Type::NetworkResponse, "statusCode") => Some(Type::Numeric(NumericType::Int32)),
        (Type::NetworkResponse, "headers") => Some(Type::Map(
            Box::new(Type::String),
            Box::new(Type::Array(Box::new(Type::String))),
        )),
        (Type::NetworkResponse, "body") => Some(Type::String),
        (Type::Array(_) | Type::Set(_) | Type::Map(_, _), "count") => {
            Some(Type::Numeric(NumericType::Int32))
        }
        (Type::Array(_) | Type::Set(_) | Type::Map(_, _), "isEmpty") => Some(Type::Bool),
        _ => None,
    }
}

fn member_field_type_with_plugins(
    base_type: &Type,
    name: &str,
    functions: &FunctionSignatures,
) -> Option<Type> {
    if let Type::Plugin { name: class, .. } = base_type {
        return functions
            .get(&format!("{class}.#property.{name}"))
            .filter(|signature| signature.receiver.as_ref() == Some(base_type))
            .map(|signature| signature.return_type.clone());
    }
    member_field_type(base_type, name)
}

pub(super) fn parse_type(syntax: &ast::TypeSyntax) -> Result<Type, CompileError> {
    match syntax {
        ast::TypeSyntax::Named(name, span) => parse_named_type(name, *span),
        ast::TypeSyntax::Optional(inner, _) => Ok(Type::Optional(Box::new(parse_type(inner)?))),
        ast::TypeSyntax::Generic(name, arguments, span) => {
            let expected_arity = match name.as_str() {
                "Array" | "Set" => 1,
                "Map" | "Pair" | "Result" => 2,
                "Triple" => 3,
                _ => {
                    return Err(CompileError::new(
                        *span,
                        format!("generic type `{name}` is not supported yet"),
                    ));
                }
            };
            if arguments.len() != expected_arity {
                let example = match name.as_str() {
                    "Array" => "Array<String>",
                    "Set" => "Set<String>",
                    "Map" => "Map<String, Int32>",
                    "Pair" => "Pair<String, Int32>",
                    "Result" => "Result<String, String>",
                    "Triple" => "Triple<String, Int32, Bool>",
                    _ => unreachable!(),
                };
                return Err(CompileError::new(
                    *span,
                    format!(
                        "{name} expects exactly {expected_arity} type arguments, such as {example}"
                    ),
                ));
            }
            let types = arguments
                .iter()
                .map(parse_type)
                .collect::<Result<Vec<_>, _>>()?;
            match name.as_str() {
                "Array" => Ok(Type::Array(Box::new(types.into_iter().next().unwrap()))),
                "Set" => {
                    let element = types.into_iter().next().unwrap();
                    require_hashable_key(&element, *span, "Set elements")?;
                    Ok(Type::Set(Box::new(element)))
                }
                "Map" => {
                    let mut types = types.into_iter();
                    let key = types.next().unwrap();
                    let value = types.next().unwrap();
                    require_hashable_key(&key, *span, "Map keys")?;
                    Ok(Type::Map(Box::new(key), Box::new(value)))
                }
                "Pair" => {
                    let mut types = types.into_iter();
                    Ok(Type::Pair(
                        Box::new(types.next().unwrap()),
                        Box::new(types.next().unwrap()),
                    ))
                }
                "Result" => {
                    let mut types = types.into_iter();
                    Ok(Type::Result(
                        Box::new(types.next().unwrap()),
                        Box::new(types.next().unwrap()),
                    ))
                }
                "Triple" => {
                    let mut types = types.into_iter();
                    Ok(Type::Triple(
                        Box::new(types.next().unwrap()),
                        Box::new(types.next().unwrap()),
                        Box::new(types.next().unwrap()),
                    ))
                }
                _ => unreachable!("generic type name checked above"),
            }
        }
    }
}

pub(super) fn resolve_struct_type(ty: &Type, structs: &StructTypes) -> Type {
    match ty {
        Type::Enum(name) => structs.get(name).cloned().unwrap_or_else(|| ty.clone()),
        Type::Optional(inner) => Type::Optional(Box::new(resolve_struct_type(inner, structs))),
        Type::Array(element) => Type::Array(Box::new(resolve_struct_type(element, structs))),
        Type::Set(element) => Type::Set(Box::new(resolve_struct_type(element, structs))),
        Type::Map(key, value) => Type::Map(
            Box::new(resolve_struct_type(key, structs)),
            Box::new(resolve_struct_type(value, structs)),
        ),
        Type::Pair(first, second) => Type::Pair(
            Box::new(resolve_struct_type(first, structs)),
            Box::new(resolve_struct_type(second, structs)),
        ),
        Type::Result(value, error) => Type::Result(
            Box::new(resolve_struct_type(value, structs)),
            Box::new(resolve_struct_type(error, structs)),
        ),
        Type::Triple(first, second, third) => Type::Triple(
            Box::new(resolve_struct_type(first, structs)),
            Box::new(resolve_struct_type(second, structs)),
            Box::new(resolve_struct_type(third, structs)),
        ),
        Type::Void
        | Type::TypeParam(_)
        | Type::String
        | Type::Bytes
        | Type::Bool
        | Type::Numeric(_)
        | Type::Plugin { .. }
        | Type::NetworkResponse
        | Type::Struct { .. } => ty.clone(),
    }
}

pub(super) fn resolve_declaration_type(
    declaration: &ast::StateDecl,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    structs: &StructTypes,
) -> Result<Type, CompileError> {
    resolve_value_type(
        &declaration.name,
        declaration.ty.as_ref(),
        &declaration.initial,
        symbols,
        functions,
        structs,
    )
}

pub(super) fn resolve_value_type(
    name: &str,
    annotation: Option<&ast::TypeSyntax>,
    initial: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    structs: &StructTypes,
) -> Result<Type, CompileError> {
    let ty = match annotation {
        Some(syntax) => resolve_struct_type(&parse_type(syntax)?, structs),
        None => infer_expr_type(initial, symbols, functions).ok_or_else(|| {
            CompileError::new(
                initial.span(),
                format!(
                    "cannot infer the type of `{}`; add an explicit `: Type` annotation (empty collections need one)",
                    name
                ),
            )
        })?,
    };
    validate_type_constraints(&ty, initial.span())?;
    Ok(ty)
}

fn validate_type_constraints(ty: &Type, span: Span) -> Result<(), CompileError> {
    match ty {
        Type::TypeParam(_) => Ok(()),
        Type::Array(element) | Type::Set(element) => {
            if matches!(ty, Type::Set(_)) {
                require_hashable_key(element, span, "Set elements")?;
            }
            validate_type_constraints(element, span)
        }
        Type::Map(key, value) => {
            require_hashable_key(key, span, "Map keys")?;
            validate_type_constraints(value, span)
        }
        Type::Pair(first, second) | Type::Result(first, second) => {
            validate_type_constraints(first, span)?;
            validate_type_constraints(second, span)
        }
        Type::Triple(first, second, third) => {
            validate_type_constraints(first, span)?;
            validate_type_constraints(second, span)?;
            validate_type_constraints(third, span)
        }
        Type::Optional(inner) => validate_type_constraints(inner, span),
        Type::Void
        | Type::String
        | Type::Bytes
        | Type::Bool
        | Type::Numeric(_)
        | Type::Enum(_)
        | Type::Plugin { .. }
        | Type::NetworkResponse
        | Type::Struct { .. } => Ok(()),
    }
}

pub(super) fn require_hashable_key(
    ty: &Type,
    span: Span,
    description: &str,
) -> Result<(), CompileError> {
    if matches!(ty, Type::String | Type::Bool | Type::Numeric(_)) {
        Ok(())
    } else {
        Err(CompileError::new(
            span,
            format!(
                "{description} must use a scalar hashable type (String, Bool, or a numeric type); found {}",
                type_name(ty)
            ),
        ))
    }
}

fn parse_named_type(name: &str, _span: Span) -> Result<Type, CompileError> {
    let ty = match name {
        "String" => Type::String,
        "Bool" => Type::Bool,
        "Int8" => Type::Numeric(NumericType::Int8),
        "Int16" => Type::Numeric(NumericType::Int16),
        "Int32" => Type::Numeric(NumericType::Int32),
        "Int64" => Type::Numeric(NumericType::Int64),
        "UInt8" => Type::Numeric(NumericType::UInt8),
        "UInt16" => Type::Numeric(NumericType::UInt16),
        "UInt32" => Type::Numeric(NumericType::UInt32),
        "UInt64" => Type::Numeric(NumericType::UInt64),
        "Float32" => Type::Numeric(NumericType::Float32),
        "Float64" => Type::Numeric(NumericType::Float64),
        // A byte buffer is a first-class value type: it can be a state, a
        // struct field, and a plugin parameter.
        "Bytes" => Type::Bytes,
        _ => Type::Enum(name.to_owned()),
    };
    Ok(ty)
}

fn is_permission_case(name: &str) -> bool {
    matches!(
        name,
        "Camera"
            | "Microphone"
            | "Photos"
            | "Location"
            | "Notifications"
            | "Contacts"
            | "Calendar"
            | "Bluetooth"
            | "Motion"
    )
}

fn is_permission_status_case(name: &str) -> bool {
    matches!(name, "granted" | "denied" | "restricted" | "notDetermined")
}

fn validate_number(raw: &str, ty: NumericType, span: Span) -> Result<(), CompileError> {
    let fits = match ty {
        NumericType::Int8 => integer_fits(raw, i8::MIN as i128, i8::MAX as i128),
        NumericType::Int16 => integer_fits(raw, i16::MIN as i128, i16::MAX as i128),
        NumericType::Int32 => integer_fits(raw, i32::MIN as i128, i32::MAX as i128),
        NumericType::Int64 => integer_fits(raw, i64::MIN as i128, i64::MAX as i128),
        NumericType::UInt8 => integer_fits(raw, 0, u8::MAX as i128),
        NumericType::UInt16 => integer_fits(raw, 0, u16::MAX as i128),
        NumericType::UInt32 => integer_fits(raw, 0, u32::MAX as i128),
        NumericType::UInt64 => raw.parse::<u64>().is_ok() && !raw.contains('.'),
        NumericType::Float32 => raw.parse::<f32>().is_ok_and(f32::is_finite),
        NumericType::Float64 => raw.parse::<f64>().is_ok_and(f64::is_finite),
    };
    if fits {
        Ok(())
    } else {
        Err(CompileError::new(
            span,
            format!(
                "numeric literal `{raw}` is outside the range of {}",
                numeric_name(ty)
            ),
        ))
    }
}

fn integer_fits(raw: &str, min: i128, max: i128) -> bool {
    !raw.contains('.')
        && raw
            .parse::<i128>()
            .is_ok_and(|value| value >= min && value <= max)
}
fn expr_numeric_type(expr: &Expr) -> Option<NumericType> {
    match expr {
        Expr::Number { ty, .. }
        | Expr::Add(_, _, ty)
        | Expr::Arithmetic { ty, .. }
        | Expr::Negate { ty, .. } => Some(*ty),
        Expr::State(_, Type::Numeric(ty)) => Some(*ty),
        _ => lowered_type(expr).as_ref().and_then(as_numeric_type),
    }
}
fn require_expected(
    expected: Option<&Type>,
    actual: &Type,
    span: Span,
) -> Result<(), CompileError> {
    if let Some(expected) = expected
        && expected != actual
    {
        return Err(CompileError::new(
            span,
            format!(
                "expected {}, found {}",
                type_name(expected),
                type_name(actual)
            ),
        ));
    }
    Ok(())
}
/// The type of an already-lowered expression, when the IR node carries one.
///
/// Semantic lowering resolves a generic plugin call's type parameters at the
/// call site, so the lowered node knows more than the source-level inference
/// pass can: reading the type back from the node is how a surrounding
/// expression sees the concrete value type.
pub(super) fn lowered_type(expr: &Expr) -> Option<Type> {
    match expr {
        Expr::Call { return_type, .. }
        | Expr::NativeCall { return_type, .. }
        | Expr::TimeCall { return_type, .. } => Some(return_type.clone()),
        Expr::Await(value) | Expr::TryAwait(value) => lowered_type(value),
        Expr::State(_, ty) => Some(ty.clone()),
        Expr::Null(ty) => Some(ty.clone()),
        Expr::Member { field_type, .. } => Some(field_type.clone()),
        Expr::Index { element_type, .. } => Some(element_type.clone()),
        Expr::Concat(_, _) => Some(Type::String),
        Expr::CollectionUtility {
            operation,
            element_type,
            ..
        } => Some(collection_utility_result_type(*operation, element_type)),
        Expr::Conditional { value_type, .. } => Some(value_type.clone()),
        _ => None,
    }
}

pub(super) fn type_name(ty: &Type) -> String {
    match ty {
        Type::Void => "Void".to_owned(),
        Type::String => "String".to_owned(),
        Type::Bytes => "Bytes".to_owned(),
        Type::Bool => "Bool".to_owned(),
        Type::Numeric(num) => numeric_name(*num).to_owned(),
        Type::Array(element) => format!("Array<{}>", type_name(element)),
        Type::Set(element) => format!("Set<{}>", type_name(element)),
        Type::Map(key, value) => format!("Map<{}, {}>", type_name(key), type_name(value)),
        Type::Pair(first, second) => {
            format!("Pair<{}, {}>", type_name(first), type_name(second))
        }
        Type::Triple(first, second, third) => format!(
            "Triple<{}, {}, {}>",
            type_name(first),
            type_name(second),
            type_name(third)
        ),
        Type::Enum(name) => name.clone(),
        Type::Plugin { namespace, name } => format!("{namespace}.{name}"),
        Type::TypeParam(name) => name.clone(),
        Type::NetworkResponse => "NetworkResponse".to_owned(),
        Type::Struct { name, .. } => name.clone(),
        Type::Result(value, error) => {
            format!("Result<{}, {}>", type_name(value), type_name(error))
        }
        Type::Optional(inner) => format!("{}?", type_name(inner)),
    }
}
fn numeric_name(ty: NumericType) -> &'static str {
    match ty {
        NumericType::Int8 => "Int8",
        NumericType::Int16 => "Int16",
        NumericType::Int32 => "Int32",
        NumericType::Int64 => "Int64",
        NumericType::UInt8 => "UInt8",
        NumericType::UInt16 => "UInt16",
        NumericType::UInt32 => "UInt32",
        NumericType::UInt64 => "UInt64",
        NumericType::Float32 => "Float32",
        NumericType::Float64 => "Float64",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use nexa_diagnostics::Span;
    use nexa_ir::{NumericType, Type};
    use nexa_syntax::ast;

    use super::{
        CallArguments, FunctionSignature, FunctionSignatures, collect_plugin_signatures,
        lower_plugin_call, lower_plugin_method_call, member_field_type_with_plugins,
    };
    use crate::semantic::context::ExprContext;

    fn throwing_signature(receiver: Option<Type>) -> FunctionSignature {
        FunctionSignature {
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: Type::Void,
            is_async: true,
            is_throwing: true,
            receiver,
            is_constructor: false,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: None,
        }
    }

    fn plugin_decl(idl: nexa_plugin_idl::PluginIdl) -> ast::PluginDecl {
        ast::PluginDecl {
            path: "sensors".to_owned(),
            namespace: "Sensors".to_owned(),
            span: Span::default(),
            idl: Some(idl),
            pure: false,
            assets_path: None,
            ios_sources: Vec::new(),
            android_sources: Vec::new(),
            cpp_sources: Vec::new(),
            cpp_headers: Vec::new(),
            cpp_standard: None,
            ios_min_version: None,
            android_min_sdk: None,
            ios_frameworks: Vec::new(),
            ios_xcframeworks: Vec::new(),
            ios_resources: Vec::new(),
            ios_privacy_manifest: None,
            swift_packages: Vec::new(),
            maven_dependencies: Vec::new(),
            android_aars: Vec::new(),
            android_resources: Vec::new(),
            android_proguard_rules: Vec::new(),
            android_maven_repositories: Vec::new(),
            ios_usage_descriptions: Vec::new(),
            ios_entitlements: Vec::new(),
            ios_background_modes: Vec::new(),
            ios_linker_flags: Vec::new(),
            android_permissions: Vec::new(),
            android_picture_in_picture: false,
            android_media_playback_service: None,
        }
    }

    #[test]
    fn exposes_plugin_value_struct_fields_to_member_access() {
        let idl = nexa_plugin_idl::parse(
            "struct MotionReading { x: Float64 } native class Sensors { init() fn dispose() event sample(reading: MotionReading) }",
        )
        .expect("plugin IDL is valid");
        let functions =
            collect_plugin_signatures(&[plugin_decl(idl)]).expect("plugin signatures are valid");
        let reading_type = Type::Plugin {
            namespace: "Sensors".to_owned(),
            name: "MotionReading".to_owned(),
        };

        assert_eq!(
            member_field_type_with_plugins(&reading_type, "x", &functions),
            Some(Type::Numeric(NumericType::Float64))
        );
        assert_eq!(
            member_field_type_with_plugins(&reading_type, "missing", &functions),
            None
        );
    }

    #[test]
    fn rejects_throwing_service_calls_without_a_recovery_block() {
        let functions: FunctionSignatures =
            HashMap::from([("Camera.capture".to_owned(), throwing_signature(None))]);
        let symbols = HashMap::new();
        let context = ExprContext::new(&symbols, &functions, true);
        let error = lower_plugin_call(
            "Camera",
            "capture",
            CallArguments {
                type_arguments: &[],
                arguments: &[],
                named_arguments: &BTreeMap::new(),
                span: Span::default(),
                expected: None,
                ctx: &context,
                awaited: true,
            },
        )
        .expect_err("throwing service calls must require typed handling");
        assert!(error.to_string().contains("may throw"));
    }

    #[test]
    fn rejects_throwing_native_class_methods_without_a_recovery_block() {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
        let functions = HashMap::from([(
            "VideoPlayer.prepare".to_owned(),
            throwing_signature(Some(player_type)),
        )]);
        let context = ExprContext::new(&symbols, &functions, true);
        let error = lower_plugin_method_call(
            &ast::Expr::Name("player".to_owned(), Span::default()),
            "prepare",
            CallArguments {
                type_arguments: &[],
                arguments: &[],
                named_arguments: &BTreeMap::new(),
                span: Span::default(),
                expected: None,
                ctx: &context,
                awaited: true,
            },
        )
        .expect_err("throwing object methods must require typed handling");
        assert!(error.to_string().contains("may throw"));
    }

    #[test]
    fn requires_native_class_disposal_to_be_synchronous_void_and_parameterless() {
        let idl = nexa_plugin_idl::parse("native class VideoPlayer { async fn dispose() }")
            .expect("IDL syntax is valid even when disposal semantics are not");
        let plugin = plugin_decl(idl);

        let error = match collect_plugin_signatures(&[plugin]) {
            Ok(_) => panic!("async disposal cannot provide deterministic cleanup"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("must be synchronous"));
    }
}
