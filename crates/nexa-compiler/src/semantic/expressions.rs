use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{BinaryOp, Expr, NumericType, Type};
use nexa_syntax::ast;

pub(super) fn references_state(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(_, _) => true,
        ast::Expr::Add(left, right, _) | ast::Expr::Binary(left, _, right, _) => {
            references_state(left) || references_state(right)
        }
        ast::Expr::Not(value, _) => references_state(value),
        ast::Expr::Array(items, _) => items.iter().any(references_state),
        ast::Expr::String(_, _)
        | ast::Expr::Number(_, _)
        | ast::Expr::Bool(_, _)
        | ast::Expr::ThemeToken(_, _) => false,
    }
}

pub(super) fn lower_expr(
    expr: &ast::Expr,
    expected: Option<&Type>,
    symbols: &HashMap<String, (Type, bool)>,
) -> Result<Expr, CompileError> {
    match expr {
        ast::Expr::String(value, _) => {
            require_expected(expected, &Type::String, expr.span())?;
            Ok(Expr::String(value.clone()))
        }
        ast::Expr::Bool(value, _) => {
            require_expected(expected, &Type::Bool, expr.span())?;
            Ok(Expr::Bool(*value))
        }
        ast::Expr::Array(items, span) => {
            let Some(Type::Array(element_type)) = expected else {
                return Err(CompileError::new(
                    *span,
                    "array literals require an explicit Array<T> type",
                ));
            };
            let mut lowered = Vec::with_capacity(items.len());
            for item in items {
                lowered.push(lower_expr(item, Some(element_type), symbols)?);
            }
            Ok(Expr::Array(lowered))
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
                .or_else(|| infer_expr_type(left, symbols).and_then(|ty| as_numeric_type(&ty)))
                .or_else(|| infer_expr_type(right, symbols).and_then(|ty| as_numeric_type(&ty)))
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
            let left = lower_expr(left, Some(&numeric), symbols)?;
            let right = lower_expr(right, Some(&numeric), symbols)?;
            if expr_numeric_type(&left) != Some(ty) || expr_numeric_type(&right) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "both sides of `+` must have the same numeric type",
                ));
            }
            Ok(Expr::Add(Box::new(left), Box::new(right), ty))
        }
        ast::Expr::Not(value, _span) => {
            let value = lower_expr(value, Some(&Type::Bool), symbols)?;
            Ok(Expr::Not(Box::new(value)))
        }
        ast::Expr::Binary(left, operator, right, span) => {
            lower_binary(left, *operator, right, *span, expected, symbols)
        }
    }
}

fn lower_binary(
    left: &ast::Expr,
    operator: ast::BinaryOp,
    right: &ast::Expr,
    span: Span,
    expected: Option<&Type>,
    symbols: &HashMap<String, (Type, bool)>,
) -> Result<Expr, CompileError> {
    let ir_operator = match operator {
        ast::BinaryOp::And => BinaryOp::And,
        ast::BinaryOp::Or => BinaryOp::Or,
        ast::BinaryOp::Equal => BinaryOp::Equal,
        ast::BinaryOp::NotEqual => BinaryOp::NotEqual,
        ast::BinaryOp::Less => BinaryOp::Less,
        ast::BinaryOp::LessEqual => BinaryOp::LessEqual,
        ast::BinaryOp::Greater => BinaryOp::Greater,
        ast::BinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
    };
    let is_logical = matches!(operator, ast::BinaryOp::And | ast::BinaryOp::Or);
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
        let left = lower_expr(left, Some(&bool_type), symbols)?;
        let right = lower_expr(right, Some(&bool_type), symbols)?;
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
    let left_type = infer_expr_type(left, symbols);
    let right_type = infer_expr_type(right, symbols);
    let common_type = match (left_type, right_type) {
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
            let left = lower_expr(left, None, symbols)?;
            let right = lower_expr(right, None, symbols)?;
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
    if !is_ordered && !matches!(common_type, Type::Numeric(_) | Type::Bool | Type::String) {
        return Err(CompileError::new(
            span,
            "equality is supported for numeric, Bool, and String values",
        ));
    }
    let left = lower_expr(left, Some(&common_type), symbols)?;
    let right = lower_expr(right, Some(&common_type), symbols)?;
    Ok(Expr::Binary {
        op: ir_operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn infer_expr_type(expr: &ast::Expr, symbols: &HashMap<String, (Type, bool)>) -> Option<Type> {
    match expr {
        ast::Expr::String(_, _) => Some(Type::String),
        ast::Expr::Bool(_, _) | ast::Expr::Not(_, _) | ast::Expr::Binary(_, _, _, _) => {
            Some(Type::Bool)
        }
        ast::Expr::Number(raw, _) => Some(Type::Numeric(if raw.contains('.') {
            NumericType::Float64
        } else {
            NumericType::Int32
        })),
        ast::Expr::Name(name, _) => symbols.get(name).map(|(ty, _)| ty.clone()),
        ast::Expr::ThemeToken(_, _) => None,
        ast::Expr::Add(left_expr, right_expr, _) => {
            let left_type = infer_expr_type(left_expr, symbols);
            let right_type = infer_expr_type(right_expr, symbols);
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
            .and_then(|item| infer_expr_type(item, symbols))
            .map(|item_type| Type::Array(Box::new(item_type))),
    }
}

fn as_numeric_type(ty: &Type) -> Option<NumericType> {
    match ty {
        Type::Numeric(numeric_type) => Some(*numeric_type),
        _ => None,
    }
}

pub(super) fn parse_type(syntax: &ast::TypeSyntax) -> Result<Type, CompileError> {
    match syntax {
        ast::TypeSyntax::Named(name, span) => parse_named_type(name, *span),
        ast::TypeSyntax::Generic(name, arguments, span) if name == "Array" => {
            if arguments.len() != 1 {
                return Err(CompileError::new(
                    *span,
                    "Array expects exactly one element type, such as Array<String>",
                ));
            }
            Ok(Type::Array(Box::new(parse_type(&arguments[0])?)))
        }
        ast::TypeSyntax::Generic(name, _, span) => Err(CompileError::new(
            *span,
            format!("generic type `{name}` is not supported yet"),
        )),
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
