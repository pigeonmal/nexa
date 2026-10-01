use std::collections::{HashMap, HashSet};

use nexa_diagnostics::CompileError;
use nexa_ir::{Component, FunctionLocal, Type};
use nexa_syntax::ast;

use super::{
    context::ExprContext,
    enum_symbols,
    expressions::{
        FunctionSignature, FunctionSignatures, collect_function_signatures, lower_expr,
        resolve_value_type,
    },
    lower_enum_declarations, lower_struct_declarations,
};
use crate::testing::{TestCase, TestStatement, TestSuite};

pub(super) fn lower_tests(
    tests: Vec<ast::TestDecl>,
    functions: &[ast::FunctionDecl],
    struct_declarations: &[ast::StructDecl],
    enum_declarations: &[ast::EnumDecl],
    components: &[Component],
) -> Result<TestSuite, CompileError> {
    if tests.is_empty() {
        return Ok(TestSuite::default());
    }

    let (ir_structs, structs) = lower_struct_declarations(struct_declarations)?;
    let enums = lower_enum_declarations(enum_declarations)?;
    let mut enum_names = enums
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect::<HashSet<_>>();
    enum_names.extend([
        "JsonError".to_owned(),
        "Permission".to_owned(),
        "PermissionStatus".to_owned(),
    ]);
    let mut enum_values = enum_symbols(&enums);
    for (name, cases) in [
        (
            "JsonError",
            [
                "invalidJson",
                "typeMismatch",
                "missingField",
                "invalidValue",
            ]
            .as_slice(),
        ),
        (
            "Permission",
            [
                "Camera",
                "Microphone",
                "Photos",
                "Location",
                "Notifications",
                "Contacts",
                "Calendar",
                "Bluetooth",
                "Motion",
            ]
            .as_slice(),
        ),
        (
            "PermissionStatus",
            ["Granted", "Denied", "Restricted", "NotDetermined"].as_slice(),
        ),
    ] {
        let value_type = Type::Enum(name.to_owned());
        for case in cases {
            enum_values.insert(format!("{name}.{case}"), (value_type.clone(), false));
        }
    }

    let mut signatures = collect_function_signatures(functions, &structs)?;
    add_struct_constructors(&mut signatures, &ir_structs);
    let components_by_name = components
        .iter()
        .map(|component| (component.name.as_str(), component))
        .collect::<std::collections::HashMap<_, _>>();

    let mut names = HashSet::with_capacity(tests.len());
    let mut lowered_tests = Vec::with_capacity(tests.len());
    for test in tests {
        if !names.insert(test.name.clone()) {
            return Err(with_test_source(
                CompileError::new(
                    test.span,
                    format!("test `{}` is declared more than once", test.name),
                ),
                test.source_file.as_deref(),
            ));
        }
        let mut symbols = enum_values.clone();
        let mounted_component = test
            .component
            .as_ref()
            .map(|component| {
                lower_test_component(
                    component,
                    &components_by_name,
                    &symbols,
                    &signatures,
                    &structs,
                    &enum_names,
                )
            })
            .transpose()
            .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
        if let Some(mount) = &mounted_component {
            let component = components_by_name[mount.name.as_str()];
            for binding in component
                .parameters
                .iter()
                .map(|parameter| (parameter.name.as_str(), &parameter.ty, false))
                .chain(
                    component
                        .states
                        .iter()
                        .map(|state| (state.name.as_str(), &state.ty, state.mutable)),
                )
            {
                let (name, ty, mutable) = binding;
                if symbols.contains_key(name) || signatures.contains_key(name) {
                    return Err(with_test_source(
                        CompileError::new(
                            test.span,
                            format!(
                                "mounted component binding `{name}` conflicts with a test value"
                            ),
                        ),
                        test.source_file.as_deref(),
                    ));
                }
                symbols.insert(name.to_owned(), (ty.clone(), mutable));
            }
        }
        let mut statements = Vec::with_capacity(test.statements.len());
        for statement in test.statements {
            match statement {
                ast::TestStatement::Let {
                    name,
                    ty,
                    initial,
                    span,
                } => {
                    if symbols.contains_key(&name) || signatures.contains_key(&name) {
                        return Err(with_test_source(
                            CompileError::new(
                                span,
                                format!("test local `{name}` is already declared"),
                            ),
                            test.source_file.as_deref(),
                        ));
                    }
                    let local_type = resolve_value_type(
                        &name,
                        ty.as_ref(),
                        &initial,
                        &symbols,
                        &signatures,
                        &structs,
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let lowered = lower_expr(
                        &initial,
                        Some(&local_type),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    symbols.insert(name.clone(), (local_type.clone(), false));
                    statements.push(TestStatement::Let {
                        local: FunctionLocal {
                            name,
                            ty: local_type,
                            initial: lowered,
                        },
                        span,
                    });
                }
                ast::TestStatement::Assert {
                    condition,
                    message,
                    span,
                } => {
                    let condition = lower_expr(
                        &condition,
                        Some(&Type::Bool),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let message = message
                        .as_ref()
                        .map(|message| {
                            lower_expr(
                                message,
                                Some(&Type::String),
                                &ExprContext::with_types(
                                    &symbols,
                                    &signatures,
                                    false,
                                    &structs,
                                    &enum_names,
                                ),
                            )
                        })
                        .transpose()
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Assert {
                        condition,
                        message,
                        span,
                    });
                }
                ast::TestStatement::Tap { label, span } => {
                    if mounted_component.is_none() {
                        return Err(with_test_source(
                            CompileError::new(
                                span,
                                "`tap(...)` requires a component target: add `for Component()` to the test",
                            ),
                            test.source_file.as_deref(),
                        ));
                    }
                    let label = lower_expr(
                        &label,
                        Some(&Type::String),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Tap { label, span });
                }
                ast::TestStatement::AssertText { value, span } => {
                    if mounted_component.is_none() {
                        return Err(with_test_source(
                            CompileError::new(
                                span,
                                "`assertText(...)` requires a component target: add `for Component()` to the test",
                            ),
                            test.source_file.as_deref(),
                        ));
                    }
                    let value = lower_expr(
                        &value,
                        Some(&Type::String),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::AssertText { value, span });
                }
                ast::TestStatement::TypeText {
                    placeholder,
                    value,
                    span,
                } => {
                    require_component_target(mounted_component.as_ref(), span, "typeText")?;
                    let context = ExprContext::with_types(
                        &symbols,
                        &signatures,
                        false,
                        &structs,
                        &enum_names,
                    );
                    let placeholder = lower_expr(&placeholder, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let value = lower_expr(&value, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::TypeText {
                        placeholder,
                        value,
                        span,
                    });
                }
                ast::TestStatement::Toggle { label, span } => {
                    require_component_target(mounted_component.as_ref(), span, "toggle")?;
                    let label = lower_expr(
                        &label,
                        Some(&Type::String),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Toggle { label, span });
                }
                ast::TestStatement::Slide { state, value, span } => {
                    require_component_target(mounted_component.as_ref(), span, "slide")?;
                    let context = ExprContext::with_types(
                        &symbols,
                        &signatures,
                        false,
                        &structs,
                        &enum_names,
                    );
                    let state = lower_expr(&state, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let value = lower_expr(
                        &value,
                        Some(&Type::Numeric(nexa_ir::NumericType::Float64)),
                        &context,
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Slide { state, value, span });
                }
                ast::TestStatement::Select { state, value, span } => {
                    require_component_target(mounted_component.as_ref(), span, "select")?;
                    let context = ExprContext::with_types(
                        &symbols,
                        &signatures,
                        false,
                        &structs,
                        &enum_names,
                    );
                    let state = lower_expr(&state, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let value = lower_expr(&value, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Select { state, value, span });
                }
                ast::TestStatement::Submit { placeholder, span } => {
                    require_component_target(mounted_component.as_ref(), span, "submit")?;
                    let placeholder = lower_expr(
                        &placeholder,
                        Some(&Type::String),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Submit { placeholder, span });
                }
                ast::TestStatement::AssertComponent { name, span } => {
                    require_component_target(mounted_component.as_ref(), span, "assertComponent")?;
                    let name = lower_expr(
                        &name,
                        Some(&Type::String),
                        &ExprContext::with_types(
                            &symbols,
                            &signatures,
                            false,
                            &structs,
                            &enum_names,
                        ),
                    )
                    .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::AssertComponent { name, span });
                }
                ast::TestStatement::Emit {
                    component,
                    event,
                    span,
                } => {
                    require_component_target(mounted_component.as_ref(), span, "emit")?;
                    let context = ExprContext::with_types(
                        &symbols,
                        &signatures,
                        false,
                        &structs,
                        &enum_names,
                    );
                    let component = lower_expr(&component, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    let event = lower_expr(&event, Some(&Type::String), &context)
                        .map_err(|error| with_test_source(error, test.source_file.as_deref()))?;
                    statements.push(TestStatement::Emit {
                        component,
                        event,
                        span,
                    });
                }
            }
        }
        lowered_tests.push(TestCase {
            name: test.name,
            source_file: test.source_file,
            span: test.span,
            component: mounted_component,
            statements,
        });
    }

    Ok(TestSuite {
        functions: Vec::new(),
        components: components.to_vec(),
        tests: lowered_tests,
    })
}

fn require_component_target(
    mount: Option<&crate::testing::TestComponentMount>,
    span: nexa_diagnostics::Span,
    operation: &str,
) -> Result<(), CompileError> {
    if mount.is_some() {
        Ok(())
    } else {
        Err(CompileError::new(
            span,
            format!(
                "`{operation}(...)` requires a component target: add `for Component()` to the test"
            ),
        ))
    }
}

fn lower_test_component(
    expression: &ast::Expr,
    components: &std::collections::HashMap<&str, &Component>,
    symbols: &HashMap<String, (Type, bool)>,
    signatures: &FunctionSignatures,
    structs: &super::expressions::StructTypes,
    enum_names: &HashSet<String>,
) -> Result<crate::testing::TestComponentMount, CompileError> {
    let (name, positional, named, type_arguments) = match expression {
        ast::Expr::Call(name, type_arguments, arguments, span) => {
            if !type_arguments.is_empty() {
                return Err(CompileError::new(
                    *span,
                    "headless component mounts do not accept type arguments",
                ));
            }
            (name.as_str(), Some(arguments.as_slice()), None, *span)
        }
        ast::Expr::CallNamed {
            name,
            type_arguments,
            arguments,
            span,
        } => {
            if !type_arguments.is_empty() {
                return Err(CompileError::new(
                    *span,
                    "headless component mounts do not accept type arguments",
                ));
            }
            (name.as_str(), None, Some(arguments), *span)
        }
        other => {
            return Err(CompileError::new(
                other.span(),
                "test component target must be a custom component call such as `Counter()`",
            ));
        }
    };
    let component = components.get(name).ok_or_else(|| {
        CompileError::new(
            type_arguments,
            format!("`{name}` is not a custom component available to headless tests"),
        )
    })?;

    let mut raw_arguments =
        HashMap::<String, &ast::Expr>::with_capacity(component.parameters.len());
    if let Some(positional) = positional {
        if positional.len() != component.parameters.len() {
            return Err(CompileError::new(
                type_arguments,
                format!(
                    "component `{name}` expects {} argument(s), got {}",
                    component.parameters.len(),
                    positional.len()
                ),
            ));
        }
        for (parameter, value) in component.parameters.iter().zip(positional) {
            raw_arguments.insert(parameter.name.clone(), value);
        }
    } else {
        if let Some(named) = named {
            raw_arguments.extend(named.iter().map(|(name, value)| (name.clone(), value)));
        }
    }
    if raw_arguments.len() != component.parameters.len() {
        return Err(CompileError::new(
            type_arguments,
            format!(
                "component `{name}` expects {} named argument(s), got {}",
                component.parameters.len(),
                raw_arguments.len()
            ),
        ));
    }

    let mut arguments = Vec::with_capacity(component.parameters.len());
    for parameter in &component.parameters {
        let value = raw_arguments.get(&parameter.name).ok_or_else(|| {
            CompileError::new(
                type_arguments,
                format!(
                    "component `{name}` is missing argument `{}`",
                    parameter.name
                ),
            )
        })?;
        arguments.push(lower_expr(
            value,
            Some(&parameter.ty),
            &ExprContext::with_types(symbols, signatures, false, structs, enum_names),
        )?);
    }
    if let Some(unknown) = raw_arguments.keys().find(|name| {
        !component
            .parameters
            .iter()
            .any(|parameter| &parameter.name == *name)
    }) {
        return Err(CompileError::new(
            type_arguments,
            format!("component `{name}` has no parameter `{unknown}`"),
        ));
    }

    Ok(crate::testing::TestComponentMount {
        name: name.to_owned(),
        arguments,
    })
}

pub(super) fn bind_test_functions(suite: &mut TestSuite, functions: &[nexa_ir::Function]) {
    if !suite.tests.is_empty() {
        suite.functions = functions.to_vec();
    }
}

fn add_struct_constructors(signatures: &mut FunctionSignatures, structs: &[nexa_ir::StructDecl]) {
    for structure in structs {
        let ty = Type::Struct {
            name: structure.name.clone(),
            fields: structure
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.clone()))
                .collect(),
        };
        signatures.insert(
            structure.name.clone(),
            FunctionSignature {
                parameters: structure
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.ty.clone()))
                    .collect(),
                type_parameters: Vec::new(),
                return_type: ty,
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
}

fn with_test_source(error: CompileError, source_file: Option<&str>) -> CompileError {
    match source_file {
        Some(source_file) => error.with_file(source_file),
        None => error,
    }
}
