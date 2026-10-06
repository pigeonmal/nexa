use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{Expr, Type};
use nexa_testkit::TestProject;

#[test]
fn alternate_icon_selection_is_a_typed_async_core_call() {
    let project = TestProject::new("nexa-core-app-icon");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app AppIconExample {
    body {
        OnAppear async {
            let applied: Bool = await AppIcon.set(null)
        }
        Column { Text("Icons") }
    }
}
"#,
    )
    .expect("app source should be written");

    let module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("AppIcon.set should compile to a native typed call")
        .module;
    let capabilities = nexa_ir::capabilities::analyze(&module);
    assert!(capabilities.uses_app_icon_api);

    let mut app_icon_call = false;
    let mut visit_expression = |expression: &Expr| {
        if let Expr::NativeCall {
            namespace,
            name,
            arguments,
            return_type,
            is_async,
            ..
        } = expression
            && namespace == "AppIcon"
            && name == "set"
        {
            app_icon_call = arguments.len() == 1 && *return_type == Type::Bool && *is_async;
        }
    };
    let mut visit_node = |_: &nexa_ir::Node| {};
    nexa_ir::walk::walk_ir(&module.body, &mut visit_node, &mut visit_expression);
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
        nexa_ir::walk::walk_actions(actions, &mut visit_expression);
    }
    assert!(
        app_icon_call,
        "AppIcon.set should preserve its nullable name and async result"
    );
}
