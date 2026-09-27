//! Binding for generic plugin calls.
//!
//! A plugin method may declare value type parameters, as in
//! `fn setObject<T>(key: String, value: T) -> Bool`. Each call site binds
//! every one of them to a concrete value type - from the explicit type
//! arguments, from the argument expressions, or from the type the call is
//! expected to produce - and the bound type selects one generated value
//! codec. Nothing stays generic in the emitted code: the call becomes a
//! direct native call plus a concrete writer or reader function.

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{PluginCodec, Type};

use super::{
    context::TypeRegistries,
    expressions::{
        parse_type, require_hashable_key, resolve_struct_type, type_name, FunctionSignature,
    },
};

/// A generic call with every type parameter bound.
pub(super) struct ResolvedCall {
    /// Declared parameter types, substituted.
    pub parameters: Vec<(String, Type)>,
    /// Declared return type, substituted.
    pub return_type: Type,
    /// Codecs in the order the generated contract declares them: one per
    /// parameter that mentions a type parameter, then the return value.
    pub codecs: Vec<PluginCodec>,
}

/// Binds one generic plugin call. Returns `None` when the signature declares
/// no type parameters, so the caller keeps its non-generic path.
pub(super) fn resolve(
    qualified_name: &str,
    signature: &FunctionSignature,
    type_arguments: &[nexa_syntax::ast::TypeSyntax],
    argument_types: &[Option<Type>],
    expected: Option<&Type>,
    registries: TypeRegistries<'_>,
    span: Span,
) -> Result<Option<ResolvedCall>, CompileError> {
    let structs = registries.structs;
    if signature.type_parameters.is_empty() {
        if !type_arguments.is_empty() {
            return Err(CompileError::new(
                span,
                format!("`{qualified_name}` does not declare value type parameters"),
            ));
        }
        return Ok(None);
    }
    let mut bindings: Vec<(String, Type)> = Vec::with_capacity(signature.type_parameters.len());
    if !type_arguments.is_empty() {
        if type_arguments.len() != signature.type_parameters.len() {
            return Err(CompileError::new(
                span,
                format!(
                    "`{qualified_name}` expects {} value type argument(s), found {}",
                    signature.type_parameters.len(),
                    type_arguments.len()
                ),
            ));
        }
        for (parameter, argument) in signature.type_parameters.iter().zip(type_arguments) {
            let declared = resolve_struct_type(&parse_type(argument)?, structs);
            bindings.push((parameter.clone(), declared));
        }
    }
    for ((_, declared), actual) in signature.parameters.iter().zip(argument_types) {
        if let Some(actual) = actual {
            unify(declared, actual, &mut bindings).map_err(|message| {
                CompileError::new(span, format!("`{qualified_name}`: {message}"))
            })?;
        }
    }
    if let Some(expected) = expected {
        unify(&signature.return_type, expected, &mut bindings).map_err(|message| {
            CompileError::new(span, format!("`{qualified_name}`: {message}"))
        })?;
    }
    for parameter in &signature.type_parameters {
        if !bindings.iter().any(|(name, _)| name == parameter) {
            return Err(CompileError::new(
                span,
                format!(
                    "cannot infer value type `{parameter}` for `{qualified_name}`; \
                     bind it explicitly, as in `{qualified_name}<{parameter}>(...)`"
                ),
            ));
        }
    }
    let mut parameters = Vec::with_capacity(signature.parameters.len());
    for (name, declared) in &signature.parameters {
        let bound = substitute(declared, &bindings);
        require_storable(&bound, name, registries, span)?;
        parameters.push((name.clone(), bound));
    }
    let return_type = substitute(&signature.return_type, &bindings);
    let mut codecs = Vec::new();
    for ((_, declared), (_, bound)) in signature.parameters.iter().zip(&parameters) {
        if mentions_type_parameter(declared) {
            codecs.push(PluginCodec {
                ty: bound.clone(),
                decodes: false,
            });
        }
    }
    if mentions_type_parameter(&signature.return_type) {
        let value_type = match &return_type {
            Type::Optional(inner) => inner.as_ref(),
            other => other,
        };
        require_storable(value_type, "return value", registries, span)?;
        codecs.push(PluginCodec {
            ty: value_type.clone(),
            decodes: true,
        });
    }
    Ok(Some(ResolvedCall {
        parameters,
        return_type,
        codecs,
    }))
}

/// Whether a type mentions a type parameter at any depth.
pub(super) fn mentions_type_parameter(ty: &Type) -> bool {
    match ty {
        Type::TypeParam(_) => true,
        Type::Optional(inner) | Type::Array(inner) | Type::Set(inner) => {
            mentions_type_parameter(inner)
        }
        Type::Map(key, value) => mentions_type_parameter(key) || mentions_type_parameter(value),
        Type::Pair(first, second) => {
            mentions_type_parameter(first) || mentions_type_parameter(second)
        }
        Type::Triple(first, second, third) => {
            mentions_type_parameter(first)
                || mentions_type_parameter(second)
                || mentions_type_parameter(third)
        }
        Type::Result(value, error) => {
            mentions_type_parameter(value) || mentions_type_parameter(error)
        }
        Type::Void
        | Type::String
        | Type::Bytes
        | Type::Bool
        | Type::Numeric(_)
        | Type::Enum(_)
        | Type::Plugin { .. }
        | Type::NetworkResponse
        | Type::Struct { .. } => false,
    }
}

/// Binds every type parameter `declared` mentions from the concrete
/// `actual` type. Types that cannot constrain a binding are left to the
/// ordinary argument assignability check.
///
/// An `actual` that still mentions a type parameter comes from a contextual
/// type that was read from an unsubstituted signature, so it carries no
/// information and must not bind anything.
fn unify(declared: &Type, actual: &Type, bindings: &mut Vec<(String, Type)>) -> Result<(), String> {
    if mentions_type_parameter(actual) {
        return Ok(());
    }
    match (declared, actual) {
        (Type::TypeParam(parameter), actual) => {
            if let Some((_, bound)) = bindings
                .iter()
                .find(|(name, _)| name == parameter)
                && bound != actual
            {
                return Err(format!(
                    "value type `{parameter}` is bound to {} and {} at the same time",
                    type_name(bound),
                    type_name(actual)
                ));
            }
            if !bindings.iter().any(|(name, _)| name == parameter) {
                bindings.push((parameter.clone(), actual.clone()));
            }
            Ok(())
        }
        (Type::Optional(declared), Type::Optional(actual))
        | (Type::Array(declared), Type::Array(actual))
        | (Type::Set(declared), Type::Set(actual)) => {
            unify(declared, actual, bindings)
        }
        (Type::Map(declared_key, declared_value), Type::Map(actual_key, actual_value)) => {
            unify(declared_key, actual_key, bindings)?;
            unify(declared_value, actual_value, bindings)
        }
        _ => Ok(()),
    }
}

/// Replaces every bound type parameter with its concrete type.
fn substitute(ty: &Type, bindings: &[(String, Type)]) -> Type {
    let lookup = |name: &str| {
        bindings
            .iter()
            .find(|(parameter, _)| parameter == name)
            .map(|(_, bound)| bound.clone())
    };
    match ty {
        Type::TypeParam(name) => lookup(name).unwrap_or_else(|| ty.clone()),
        Type::Optional(inner) => Type::Optional(Box::new(substitute(inner, bindings))),
        Type::Array(element) => Type::Array(Box::new(substitute(element, bindings))),
        Type::Set(element) => Type::Set(Box::new(substitute(element, bindings))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute(key, bindings)),
            Box::new(substitute(value, bindings)),
        ),
        Type::Pair(first, second) => Type::Pair(
            Box::new(substitute(first, bindings)),
            Box::new(substitute(second, bindings)),
        ),
        Type::Triple(first, second, third) => Type::Triple(
            Box::new(substitute(first, bindings)),
            Box::new(substitute(second, bindings)),
            Box::new(substitute(third, bindings)),
        ),
        Type::Result(value, error) => Type::Result(
            Box::new(substitute(value, bindings)),
            Box::new(substitute(error, bindings)),
        ),
        other => other.clone(),
    }
}

/// A bound type must be a value a generated codec can carry: a scalar, a
/// byte buffer, an enum, a value struct, or a collection of those. Optionals
/// are rejected because a missing value is a `null` in the contract, not a
/// distinct value in the store.
fn require_storable(
    ty: &Type,
    context: &str,
    registries: TypeRegistries<'_>,
    span: Span,
) -> Result<(), CompileError> {
    let TypeRegistries { structs, enums } = registries;
    match ty {
        Type::String | Type::Bytes | Type::Bool | Type::Numeric(_) => Ok(()),
        Type::Enum(name) => {
            if enums.contains(name.as_str()) {
                Ok(())
            } else {
                Err(CompileError::new(
                    span,
                    format!("unknown enum type `{name}` in plugin value type for {context}"),
                ))
            }
        }
        Type::Struct { name, fields } => {
            if !structs.contains_key(name) {
                return Err(CompileError::new(
                    span,
                    format!("unknown struct type `{name}` in plugin value type for {context}"),
                ));
            }
            for (field, field_type) in fields {
                require_storable(field_type, &format!("`{name}.{field}`"), registries, span)?;
            }
            Ok(())
        }
        Type::Array(element) | Type::Set(element) => {
            if matches!(ty, Type::Set(_)) {
                require_hashable_key(element, span, "Set elements")?;
            }
            require_storable(element, context, registries, span)
        }
        Type::Map(key, value) => {
            require_hashable_key(key, span, "Map keys")?;
            require_storable(value, context, registries, span)
        }
        Type::Optional(_) => Err(CompileError::new(
            span,
            format!(
                "plugin value type for {context} must not be optional; use a missing key to mean `null`"
            ),
        )),
        Type::Void
        | Type::TypeParam(_)
        | Type::Pair(..)
        | Type::Triple(..)
        | Type::Result(..)
        | Type::Plugin { .. }
        | Type::NetworkResponse => Err(CompileError::new(
            span,
            format!(
                "plugin value type for {context} must be a scalar, `Bytes`, an enum, a struct, or a collection of those; found {}",
                type_name(ty)
            ),
        )),
    }
}
