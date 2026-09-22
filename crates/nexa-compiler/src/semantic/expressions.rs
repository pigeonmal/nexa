use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{BinaryOp, Expr, InterpolatedPart, NumericType, Type};
use nexa_syntax::ast;

#[derive(Clone)]
pub(super) struct FunctionSignature {
    pub(super) parameters: Vec<(String, Type)>,
    pub(super) return_type: Type,
    pub(super) is_async: bool,
}

pub(super) type FunctionSignatures = HashMap<String, FunctionSignature>;

pub(super) fn collect_function_signatures(
    declarations: &[ast::FunctionDecl],
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
            parameters.push((parameter.name.clone(), parse_type(&parameter.ty)?));
        }
        signatures.insert(
            declaration.name.clone(),
            FunctionSignature {
                parameters,
                return_type: parse_type(&declaration.return_type)?,
                is_async: declaration.is_async,
            },
        );
    }
    Ok(signatures)
}

pub(super) fn references_state(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(_, _) => true,
        ast::Expr::Add(left, right, _) | ast::Expr::Binary(left, _, right, _) => {
            references_state(left) || references_state(right)
        }
        ast::Expr::Not(value, _) => references_state(value),
        ast::Expr::Array(items, _) => items.iter().any(references_state),
        ast::Expr::Map(entries, _) => entries
            .iter()
            .any(|(key, value)| references_state(key) || references_state(value)),
        ast::Expr::Pair(first, second, _) => references_state(first) || references_state(second),
        ast::Expr::Triple(first, second, third, _) => {
            references_state(first) || references_state(second) || references_state(third)
        }
        ast::Expr::Call(_, arguments, _) => arguments.iter().any(references_state),
        ast::Expr::Index {
            collection, index, ..
        } => references_state(collection) || references_state(index),
        ast::Expr::Member { base, .. } => references_state(base),
        ast::Expr::Range {
            start, end, step, ..
        } => {
            references_state(start)
                || references_state(end)
                || step.as_deref().is_some_and(references_state)
        }
        ast::Expr::Coalesce(left, right, _) => references_state(left) || references_state(right),
        ast::Expr::Await(value, _) => references_state(value),
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
        | ast::Expr::IsRegularWidth(_) => false,
    }
}

pub(super) fn lower_expr(
    expr: &ast::Expr,
    expected: Option<&Type>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
) -> Result<Expr, CompileError> {
    // A non-null value may be promoted to its optional type without a runtime
    // wrapper. Keep an already-optional expression and the `null` literal on
    // the outer type so their nullability is checked exactly.
    let expected = match expected {
        Some(Type::Optional(inner))
            if !matches!(expr, ast::Expr::Null(_))
                && !matches!(
                    infer_expr_type(expr, symbols, functions),
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
                        let value = lower_expr(
                            &ast::Expr::Name(name.clone(), *span),
                            None,
                            symbols,
                            functions,
                            allow_await,
                        )?;
                        lowered.push(InterpolatedPart::Value(Box::new(value)));
                    }
                    ast::StringPart::Expression(expression) => {
                        lowered.push(InterpolatedPart::Value(Box::new(lower_expr(
                            expression,
                            None,
                            symbols,
                            functions,
                            allow_await,
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
                lowered.push(lower_expr(
                    item,
                    Some(element_type),
                    symbols,
                    functions,
                    allow_await,
                )?);
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
                    lower_expr(key, Some(key_type), symbols, functions, allow_await)?,
                    lower_expr(value, Some(value_type), symbols, functions, allow_await)?,
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
                Box::new(lower_expr(
                    first,
                    Some(first_type),
                    symbols,
                    functions,
                    allow_await,
                )?),
                Box::new(lower_expr(
                    second,
                    Some(second_type),
                    symbols,
                    functions,
                    allow_await,
                )?),
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
                Box::new(lower_expr(
                    first,
                    Some(first_type),
                    symbols,
                    functions,
                    allow_await,
                )?),
                Box::new(lower_expr(
                    second,
                    Some(second_type),
                    symbols,
                    functions,
                    allow_await,
                )?),
                Box::new(lower_expr(
                    third,
                    Some(third_type),
                    symbols,
                    functions,
                    allow_await,
                )?),
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
            let Some((ty, _)) = symbols.get(name) else {
                return Err(CompileError::new(*span, format!("unknown state `{name}`")));
            };
            require_expected(expected, ty, *span)?;
            Ok(Expr::State(name.clone(), ty.clone()))
        }
        ast::Expr::ThemeToken(name, span) => Err(CompileError::new(
            *span,
            format!("`Theme.{name}` can only be used in supported style options"),
        )),
        ast::Expr::Add(left, right, span) => {
            let ty = expected
                .and_then(as_numeric_type)
                .or_else(|| {
                    infer_expr_type(left, symbols, functions).and_then(|ty| as_numeric_type(&ty))
                })
                .or_else(|| {
                    infer_expr_type(right, symbols, functions).and_then(|ty| as_numeric_type(&ty))
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
            let left = lower_expr(left, Some(&numeric), symbols, functions, allow_await)?;
            let right = lower_expr(right, Some(&numeric), symbols, functions, allow_await)?;
            if expr_numeric_type(&left) != Some(ty) || expr_numeric_type(&right) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "both sides of `+` must have the same numeric type",
                ));
            }
            Ok(Expr::Add(Box::new(left), Box::new(right), ty))
        }
        ast::Expr::Not(value, _span) => {
            let value = lower_expr(value, Some(&Type::Bool), symbols, functions, allow_await)?;
            Ok(Expr::Not(Box::new(value)))
        }
        ast::Expr::Binary(left, operator, right, span) => lower_binary(
            left,
            *operator,
            right,
            *span,
            expected,
            symbols,
            functions,
            allow_await,
        ),
        ast::Expr::Call(name, arguments, span) => lower_call(
            name,
            arguments,
            *span,
            expected,
            symbols,
            functions,
            allow_await,
            false,
        ),
        ast::Expr::Index {
            collection,
            index,
            optional,
            span,
        } => {
            let inferred_collection_type = infer_expr_type(collection, symbols, functions);
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
            let lowered_collection = lower_expr(
                collection,
                Some(&collection_type),
                symbols,
                functions,
                allow_await,
            )?;
            let lowered_index =
                lower_expr(index, Some(&index_type), symbols, functions, allow_await)?;
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
            let Some(base_type) = infer_expr_type(base, symbols, functions) else {
                return Err(CompileError::new(
                    *span,
                    format!("cannot access member `{name}` on an untyped value"),
                ));
            };
            let (member_base_type, field_type) = if *optional {
                let Type::Optional(inner) = &base_type else {
                    return Err(CompileError::new(
                        *span,
                        "`?.` requires an optional Pair or Triple value",
                    ));
                };
                let Some(field_type) = member_field_type(inner, name) else {
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
                let Some(field_type) = member_field_type(&base_type, name) else {
                    return Err(CompileError::new(
                        *span,
                        format!("`{}` has no member `{name}`", type_name(&base_type)),
                    ));
                };
                (base_type.clone(), field_type)
            };
            let base = lower_expr(
                base,
                Some(&member_base_type),
                symbols,
                functions,
                allow_await,
            )?;
            require_expected(expected, &field_type, *span)?;
            Ok(Expr::Member {
                base: Box::new(base),
                name: name.clone(),
                optional: *optional,
                base_type: member_base_type,
                field_type,
            })
        }
        ast::Expr::Range { span, .. } => Err(CompileError::new(
            *span,
            "ranges are only valid as `for` loop iterables",
        )),
        ast::Expr::Coalesce(left, right, span) => {
            let Some(Type::Optional(inner)) = infer_expr_type(left, symbols, functions) else {
                return Err(CompileError::new(
                    *span,
                    "left side of `??` must be an optional value",
                ));
            };
            let optional_type = Type::Optional(inner.clone());
            let lowered_left =
                lower_expr(left, Some(&optional_type), symbols, functions, allow_await)?;
            let lowered_right = lower_expr(right, Some(&inner), symbols, functions, allow_await)?;
            require_expected(expected, &inner, *span)?;
            Ok(Expr::Coalesce(
                Box::new(lowered_left),
                Box::new(lowered_right),
            ))
        }
        ast::Expr::Await(value, span) => {
            if !allow_await {
                return Err(CompileError::new(
                    *span,
                    "`await` is only allowed in an async function or `OnAppear async` block",
                ));
            }
            let ast::Expr::Call(name, arguments, call_span) = value.as_ref() else {
                return Err(CompileError::new(
                    *span,
                    "`await` must be applied to an async function call",
                ));
            };
            let call = lower_call(
                name,
                arguments,
                *call_span,
                expected,
                symbols,
                functions,
                allow_await,
                true,
            )?;
            Ok(Expr::Await(Box::new(call)))
        }
    }
}

fn lower_call(
    name: &str,
    arguments: &[ast::Expr],
    span: Span,
    expected: Option<&Type>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    awaited: bool,
) -> Result<Expr, CompileError> {
    let Some(signature) = functions.get(name) else {
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
    if awaited && !allow_await {
        return Err(CompileError::new(
            span,
            "`await` is only allowed in an async function or `OnAppear async` block",
        ));
    }
    require_expected(expected, &signature.return_type, span)?;
    let lowered = arguments
        .iter()
        .zip(&signature.parameters)
        .map(|(argument, (_, ty))| lower_expr(argument, Some(ty), symbols, functions, allow_await))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expr::Call {
        name: name.to_owned(),
        arguments: lowered,
        return_type: signature.return_type.clone(),
        is_async: signature.is_async,
    })
}

fn lower_binary(
    left: &ast::Expr,
    operator: ast::BinaryOp,
    right: &ast::Expr,
    span: Span,
    expected: Option<&Type>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
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
        let Some(collection_type) = infer_expr_type(right, symbols, functions) else {
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
        let left = lower_expr(left, Some(element_type), symbols, functions, allow_await)?;
        let right = lower_expr(
            right,
            Some(&collection_type),
            symbols,
            functions,
            allow_await,
        )?;
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
        let left = lower_expr(left, Some(&bool_type), symbols, functions, allow_await)?;
        let right = lower_expr(right, Some(&bool_type), symbols, functions, allow_await)?;
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
    let left_type = infer_expr_type(left, symbols, functions);
    let right_type = infer_expr_type(right, symbols, functions);
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
            let left = lower_expr(left, None, symbols, functions, allow_await)?;
            let right = lower_expr(right, None, symbols, functions, allow_await)?;
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
    if !is_ordered
        && !matches!(
            common_type,
            Type::Numeric(_) | Type::Bool | Type::String | Type::Optional(_)
        )
    {
        return Err(CompileError::new(
            span,
            "equality is supported for numeric, Bool, and String values",
        ));
    }
    let left = lower_expr(left, Some(&common_type), symbols, functions, allow_await)?;
    let right = lower_expr(right, Some(&common_type), symbols, functions, allow_await)?;
    Ok(Expr::Binary {
        op: ir_operator,
        left: Box::new(left),
        right: Box::new(right),
    })
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
        | ast::Expr::Not(_, _)
        | ast::Expr::Binary(_, _, _, _) => Some(Type::Bool),
        ast::Expr::Number(raw, _) => Some(Type::Numeric(if raw.contains('.') {
            NumericType::Float64
        } else {
            NumericType::Int32
        })),
        ast::Expr::Name(name, _) => symbols.get(name).map(|(ty, _)| ty.clone()),
        ast::Expr::Call(name, _, _) => functions
            .get(name)
            .map(|signature| signature.return_type.clone()),
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
        } => infer_expr_type(base, symbols, functions).and_then(|base_type| {
            if *optional {
                let Type::Optional(inner) = base_type else {
                    return None;
                };
                member_field_type(&inner, name).map(|field| Type::Optional(Box::new(field)))
            } else {
                member_field_type(&base_type, name)
            }
        }),
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
        ast::Expr::Await(value, _) => infer_expr_type(value, symbols, functions),
        ast::Expr::ThemeToken(_, _) => None,
        ast::Expr::Add(left_expr, right_expr, _) => {
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
        _ => None,
    }
}

pub(super) fn parse_type(syntax: &ast::TypeSyntax) -> Result<Type, CompileError> {
    match syntax {
        ast::TypeSyntax::Named(name, span) => parse_named_type(name, *span),
        ast::TypeSyntax::Optional(inner, _) => Ok(Type::Optional(Box::new(parse_type(inner)?))),
        ast::TypeSyntax::Generic(name, arguments, span) => {
            let expected_arity = match name.as_str() {
                "Array" | "Set" => 1,
                "Map" | "Pair" => 2,
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

pub(super) fn resolve_declaration_type(
    declaration: &ast::StateDecl,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Result<Type, CompileError> {
    resolve_value_type(
        &declaration.name,
        declaration.ty.as_ref(),
        &declaration.initial,
        symbols,
        functions,
    )
}

pub(super) fn resolve_value_type(
    name: &str,
    annotation: Option<&ast::TypeSyntax>,
    initial: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
) -> Result<Type, CompileError> {
    let ty = match annotation {
        Some(syntax) => parse_type(syntax)?,
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
        Type::Pair(first, second) => {
            validate_type_constraints(first, span)?;
            validate_type_constraints(second, span)
        }
        Type::Triple(first, second, third) => {
            validate_type_constraints(first, span)?;
            validate_type_constraints(second, span)?;
            validate_type_constraints(third, span)
        }
        Type::Optional(inner) => validate_type_constraints(inner, span),
        Type::String | Type::Bool | Type::Numeric(_) => Ok(()),
    }
}

fn require_hashable_key(ty: &Type, span: Span, description: &str) -> Result<(), CompileError> {
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

fn parse_named_type(name: &str, span: Span) -> Result<Type, CompileError> {
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
        _ => return Err(CompileError::new(span, format!("unknown type `{name}`"))),
    };
    Ok(ty)
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
        Expr::Number { ty, .. } | Expr::Add(_, _, ty) => Some(*ty),
        Expr::State(_, Type::Numeric(ty)) => Some(*ty),
        _ => None,
    }
}
fn require_expected(
    expected: Option<&Type>,
    actual: &Type,
    span: Span,
) -> Result<(), CompileError> {
    if let Some(expected) = expected {
        if expected != actual {
            return Err(CompileError::new(
                span,
                format!(
                    "expected {}, found {}",
                    type_name(expected),
                    type_name(actual)
                ),
            ));
        }
    }
    Ok(())
}
pub(super) fn type_name(ty: &Type) -> String {
    match ty {
        Type::String => "String".to_owned(),
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
