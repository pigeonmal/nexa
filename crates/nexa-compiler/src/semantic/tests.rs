use std::collections::HashSet;

use nexa_diagnostics::CompileError;
use nexa_ir::{FunctionLocal, Type};
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
            }
        }
        lowered_tests.push(TestCase {
            name: test.name,
            source_file: test.source_file,
            span: test.span,
            statements,
        });
    }

    Ok(TestSuite {
        functions: Vec::new(),
        tests: lowered_tests,
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
