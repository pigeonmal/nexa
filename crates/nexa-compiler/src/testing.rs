use std::collections::HashMap;

use nexa_diagnostics::Span;
use nexa_ir::{ArithmeticOp, BinaryOp, Expr, Function, FunctionLocal, NumericType, Type};

/// A statically typed `.nx` test body, evaluated only by the host CLI.
#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    pub source_file: Option<String>,
    pub span: Span,
    pub statements: Vec<TestStatement>,
}

#[derive(Clone, Debug, Default)]
pub struct TestSuite {
    /// Host-only copies of pure app functions referenced by test blocks.
    pub functions: Vec<Function>,
    pub tests: Vec<TestCase>,
}

#[derive(Clone, Debug)]
pub enum TestStatement {
    Let {
        local: FunctionLocal,
        span: Span,
    },
    Assert {
        condition: Expr,
        message: Option<Expr>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct TestFailure {
    pub name: String,
    pub source_file: Option<String>,
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct TestRunReport {
    pub passed: usize,
    pub passed_names: Vec<String>,
    pub failures: Vec<TestFailure>,
}

/// Evaluates deterministic, typed test expressions on the host. This evaluator
/// is used by `nexa test` only and is never included in generated apps.
pub fn run_tests(suite: &TestSuite) -> TestRunReport {
    let function_map = suite
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let mut report = TestRunReport::default();
    for test in &suite.tests {
        match run_test(&function_map, test) {
            Ok(()) => {
                report.passed += 1;
                report.passed_names.push(test.name.clone());
            }
            Err((span, message)) => report.failures.push(TestFailure {
                name: test.name.clone(),
                source_file: test.source_file.clone(),
                span,
                message,
            }),
        }
    }
    report
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Void,
    Bool(bool),
    String(String),
    Integer(i128),
    Float(f64),
    Enum(String, String),
    Array(Vec<Value>),
}

type Environment = HashMap<String, Value>;
type FunctionMap<'a> = HashMap<&'a str, &'a Function>;

fn run_test(functions: &FunctionMap<'_>, test: &TestCase) -> Result<(), (Span, String)> {
    let mut environment = Environment::new();
    for statement in &test.statements {
        match statement {
            TestStatement::Let { local, span } => {
                let value = eval_expr(&local.initial, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                environment.insert(local.name.clone(), value);
            }
            TestStatement::Assert {
                condition,
                message,
                span,
            } => {
                let condition = eval_expr(condition, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                match condition {
                    Value::Bool(true) => {}
                    Value::Bool(false) => {
                        let message = message
                            .as_ref()
                            .map(|message| eval_expr(message, &environment, functions, 0))
                            .transpose()
                            .map_err(|message| (*span, message))?
                            .and_then(|value| match value {
                                Value::String(value) => Some(value),
                                _ => None,
                            })
                            .unwrap_or_else(|| "assertion failed".to_owned());
                        return Err((*span, message));
                    }
                    _ => return Err((*span, "assertion did not evaluate to a Boolean".to_owned())),
                }
            }
        }
    }
    Ok(())
}

fn eval_function(
    function: &Function,
    arguments: Vec<Value>,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<Value, String> {
    if depth >= 128 {
        return Err(format!(
            "function call depth exceeded while evaluating `{}`",
            function.name
        ));
    }
    if function.is_async {
        return Err(format!(
            "async function `{}` cannot run in a synchronous test",
            function.name
        ));
    }
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "function `{}` expected {} argument(s), got {}",
            function.name,
            function.parameters.len(),
            arguments.len()
        ));
    }
    let mut environment = function
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, value)| (parameter.name.clone(), value))
        .collect::<Environment>();
    for local in &function.locals {
        let value = eval_expr(&local.initial, &environment, functions, depth + 1)?;
        environment.insert(local.name.clone(), value);
    }
    eval_expr(&function.body, &environment, functions, depth + 1)
}

fn eval_expr(
    expression: &Expr,
    environment: &Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<Value, String> {
    if depth >= 128 {
        return Err("expression evaluation depth exceeded".to_owned());
    }
    match expression {
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Number { raw, ty } => parse_number(raw, *ty),
        Expr::State(name, _) => environment
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown test value `{name}`")),
        Expr::EnumValue {
            enum_name,
            case_name,
        } => Ok(Value::Enum(enum_name.clone(), case_name.clone())),
        Expr::Add(left, right, ty) => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            let right = eval_expr(right, environment, functions, depth + 1)?;
            numeric_add(left, right, *ty)
        }
        Expr::Concat(left, right) => match (
            eval_expr(left, environment, functions, depth + 1)?,
            eval_expr(right, environment, functions, depth + 1)?,
        ) {
            (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
            _ => Err("string concatenation received a non-string value".to_owned()),
        },
        Expr::Arithmetic {
            op,
            left,
            right,
            ty,
        } => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            let right = eval_expr(right, environment, functions, depth + 1)?;
            numeric_arithmetic(left, right, *op, *ty)
        }
        Expr::Negate { value, ty } => match eval_expr(value, environment, functions, depth + 1)? {
            Value::Integer(value) => value
                .checked_neg()
                .map(Value::Integer)
                .ok_or_else(|| "integer negation overflowed".to_owned()),
            Value::Float(value) => Ok(Value::Float(-value)),
            _ => Err(format!("unary `-` received a non-{ty:?} value")),
        },
        Expr::Not(value) => match eval_expr(value, environment, functions, depth + 1)? {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            _ => Err("logical negation received a non-Boolean value".to_owned()),
        },
        Expr::Binary { op, left, right } => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            if *op == BinaryOp::And && left == Value::Bool(false) {
                return Ok(Value::Bool(false));
            }
            if *op == BinaryOp::Or && left == Value::Bool(true) {
                return Ok(Value::Bool(true));
            }
            let right = eval_expr(right, environment, functions, depth + 1)?;
            match op {
                BinaryOp::And => bool_pair(left, right, |left, right| left && right),
                BinaryOp::Or => bool_pair(left, right, |left, right| left || right),
                BinaryOp::Equal => Ok(Value::Bool(left == right)),
                BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => compare_values(left, right, *op),
                BinaryOp::Contains => Err("membership tests are not supported yet".to_owned()),
            }
        }
        Expr::Array(values) => Ok(Value::Array(
            values
                .iter()
                .map(|value| eval_expr(value, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Expr::Call {
            name,
            arguments,
            return_type,
            is_constructor,
            ..
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| eval_expr(argument, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            if *is_constructor {
                return Err(format!(
                    "value constructor `{name}` cannot run in a test expression"
                ));
            }
            if let Some(function) = functions.get(name.as_str()) {
                return eval_function(function, arguments, functions, depth + 1);
            }
            cast_builtin(name, arguments, return_type)
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => match eval_expr(condition, environment, functions, depth + 1)? {
            Value::Bool(true) => eval_expr(then_value, environment, functions, depth + 1),
            Value::Bool(false) => eval_expr(else_value, environment, functions, depth + 1),
            _ => Err("conditional test expression has a non-Boolean condition".to_owned()),
        },
        Expr::Interpolation(parts) => {
            let mut output = String::new();
            for part in parts {
                match part {
                    nexa_ir::InterpolatedPart::Literal(value) => output.push_str(value),
                    nexa_ir::InterpolatedPart::Value(value) => output.push_str(&display_value(
                        eval_expr(value, environment, functions, depth + 1)?,
                    )),
                }
            }
            Ok(Value::String(output))
        }
        _ => Err(format!(
            "`nexa test` cannot evaluate this expression yet: {}",
            expression_name(expression)
        )),
    }
}

fn expression_name(expression: &Expr) -> &'static str {
    match expression {
        Expr::Member { .. } => "member access",
        Expr::Index { .. } => "collection indexing",
        Expr::Map(_) => "map literal",
        Expr::Set(_) => "set literal",
        Expr::Pair(_, _) | Expr::Triple(_, _, _) => "tuple literal",
        Expr::NativeCall { .. } => "native API call",
        Expr::CollectionTransform { .. } => "collection transform",
        Expr::CollectionUtility { .. } => "collection utility",
        Expr::Await(_) => "async expression",
        _ => "expression",
    }
}

fn parse_number(raw: &str, ty: NumericType) -> Result<Value, String> {
    if matches!(ty, NumericType::Float32 | NumericType::Float64) {
        raw.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("invalid floating point literal `{raw}`"))
    } else {
        raw.parse::<i128>()
            .map(Value::Integer)
            .map_err(|_| format!("invalid integer literal `{raw}`"))
    }
}

fn numeric_add(left: Value, right: Value, ty: NumericType) -> Result<Value, String> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left
            .checked_add(right)
            .map(Value::Integer)
            .ok_or_else(|| "integer addition overflowed".to_owned()),
        (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        _ => Err(format!("`+` received values incompatible with {ty:?}")),
    }
}

fn numeric_arithmetic(
    left: Value,
    right: Value,
    op: ArithmeticOp,
    ty: NumericType,
) -> Result<Value, String> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => {
            let value = match op {
                ArithmeticOp::Subtract => left.checked_sub(right),
                ArithmeticOp::Multiply => left.checked_mul(right),
                ArithmeticOp::Divide if right != 0 => left.checked_div(right),
                ArithmeticOp::Remainder if right != 0 => left.checked_rem(right),
                ArithmeticOp::Divide | ArithmeticOp::Remainder => {
                    return Err("division by zero in test expression".to_owned());
                }
            };
            value
                .map(Value::Integer)
                .ok_or_else(|| "integer arithmetic overflowed".to_owned())
        }
        (Value::Float(left), Value::Float(right)) => {
            if right == 0.0 && matches!(op, ArithmeticOp::Divide | ArithmeticOp::Remainder) {
                return Err("division by zero in test expression".to_owned());
            }
            Ok(Value::Float(match op {
                ArithmeticOp::Subtract => left - right,
                ArithmeticOp::Multiply => left * right,
                ArithmeticOp::Divide => left / right,
                ArithmeticOp::Remainder => left % right,
            }))
        }
        _ => Err(format!(
            "arithmetic received values incompatible with {ty:?}"
        )),
    }
}

fn bool_pair(
    left: Value,
    right: Value,
    operation: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, String> {
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(operation(left, right))),
        _ => Err("logical operator received a non-Boolean value".to_owned()),
    }
}

fn compare_values(left: Value, right: Value, op: BinaryOp) -> Result<Value, String> {
    let ordering = match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left.partial_cmp(&right),
        (Value::Float(left), Value::Float(right)) => left.partial_cmp(&right),
        (Value::String(left), Value::String(right)) => left.partial_cmp(&right),
        _ => return Err("comparison received values of incompatible types".to_owned()),
    }
    .ok_or_else(|| "comparison is undefined for these values".to_owned())?;
    Ok(Value::Bool(match op {
        BinaryOp::Less => ordering.is_lt(),
        BinaryOp::LessEqual => !ordering.is_gt(),
        BinaryOp::Greater => ordering.is_gt(),
        BinaryOp::GreaterEqual => !ordering.is_lt(),
        _ => return Err("invalid ordered comparison".to_owned()),
    }))
}

fn cast_builtin(name: &str, arguments: Vec<Value>, return_type: &Type) -> Result<Value, String> {
    if arguments.len() != 1 {
        return Err(format!("unknown test function `{name}`"));
    }
    let value = arguments.into_iter().next().unwrap_or(Value::Void);
    if matches!(name, "String" | "toString") || *return_type == Type::String {
        return Ok(Value::String(display_value(value)));
    }
    if matches!(
        return_type,
        Type::Numeric(NumericType::Float32 | NumericType::Float64)
    ) {
        return match value {
            Value::Integer(value) => Ok(Value::Float(value as f64)),
            Value::Float(value) => Ok(Value::Float(value)),
            _ => Err(format!("cannot convert this value to `{name}`")),
        };
    }
    if matches!(return_type, Type::Numeric(_)) {
        return match value {
            Value::Integer(value) => Ok(Value::Integer(value)),
            Value::Float(value) if value.is_finite() => Ok(Value::Integer(value as i128)),
            _ => Err(format!("cannot convert this value to `{name}`")),
        };
    }
    Err(format!(
        "test expression called unsupported function `{name}`"
    ))
}

fn display_value(value: Value) -> String {
    match value {
        Value::Void => "()".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value,
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Enum(name, case) => format!("{name}.{case}"),
        Value::Array(values) => format!(
            "[{}]",
            values
                .into_iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
