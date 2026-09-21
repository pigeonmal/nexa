use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{Expr, NumericType, Type};
use nexa_syntax::ast;

pub(super) fn references_state(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Name(_, _) => true,
        ast::Expr::Add(left, right, _) => references_state(left) || references_state(right),
        ast::Expr::Array(items, _) => items.iter().any(references_state),
        ast::Expr::String(_, _) | ast::Expr::Number(_, _) | ast::Expr::Bool(_, _) => false,
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
        ast::Expr::Add(left, right, span) => {
            let left = lower_expr(left, expected, symbols)?;
            let Some(ty) = expr_numeric_type(&left) else {
                return Err(CompileError::new(
                    *span,
                    "`+` is only supported for numeric values in this language version",
                ));
            };
            let right = lower_expr(right, Some(&Type::Numeric(ty)), symbols)?;
            if expr_numeric_type(&right) != Some(ty) {
                return Err(CompileError::new(
                    *span,
                    "both sides of `+` must have the same numeric type",
                ));
            }
            Ok(Expr::Add(Box::new(left), Box::new(right), ty))
        }
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
