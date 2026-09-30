//! The built-in screen orientation API validates its closed mode set and
//! lowers each request to one typed, synchronous native call.

use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{Expr, Type};
use nexa_testkit::TestProject;

fn compile(name: &str, mode: &str) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new(name);
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        format!(
            "app OrientationApp {{ body {{ Button(\"Rotate\") {{ Screen.lockOrientation(mode: {mode}) }} }} }}\n"
        ),
    )
    .expect("orientation app should be written");
    compile_file_with_warnings_for_target(&entry, Target::Swift)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

#[test]
fn canonical_modes_lower_to_synchronous_screen_calls() {
    for mode in ["Portrait", "Landscape", "All"] {
        let module = compile("nexa-screen-orientation", mode)
            .unwrap_or_else(|error| panic!("{mode} should compile: {error}"));
        let mut calls = Vec::new();
        let mut visit_node = |_: &nexa_ir::Node| {};
        let mut visit_expr = |expression: &Expr| {
            if let Expr::NativeCall {
                namespace,
                name,
                arguments,
                return_type,
                is_async,
                ..
            } = expression
                && namespace == "Screen"
            {
                calls.push((
                    name.clone(),
                    arguments.clone(),
                    return_type.clone(),
                    *is_async,
                ));
            }
        };
        nexa_ir::walk::walk_ir(&module.body, &mut visit_node, &mut visit_expr);
        assert_eq!(calls.len(), 1);
        let (name, arguments, return_type, is_async) = &calls[0];
        assert_eq!(name, "lockOrientation");
        assert_eq!(*return_type, Type::Void);
        assert!(!is_async);
        assert_eq!(arguments.len(), 1);
        assert_eq!(arguments[0].0, "mode");
        assert!(matches!(&arguments[0].1, Expr::String(value) if value == mode));
    }
}

#[test]
fn unknown_modes_are_rejected_at_compile_time() {
    let error = compile("nexa-screen-orientation-invalid", "Sideways")
        .expect_err("unsupported orientations must not silently map to a valid mode");
    assert!(error.contains("Portrait`, `Landscape`, or `All"), "{error}");
}
