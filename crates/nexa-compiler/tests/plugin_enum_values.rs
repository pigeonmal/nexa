use std::fs;

use nexa_ir::{Expr, walk::walk_ir};
use nexa_testkit::TestProject;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};

#[test]
fn qualified_plugin_enum_cases_lower_to_typed_ir_values() {
    let project = TestProject::new("nexa-plugin-enum-values");
    let plugin = project.join("camera");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.test.camera\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        "enum Facing { back front }\n\
         native component CameraView { prop facing: Facing }\n",
    )
    .expect("plugin contract should be written");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        "plugin \"camera\" as Camera\n\
         app Demo { body { Camera.CameraView(facing: Camera.Facing.back) } }\n",
    )
    .expect("app source should be written");

    let module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("qualified plugin enum cases should type-check")
        .module;
    let mut found = false;
    let mut visit_node = |_: &nexa_ir::Node| {};
    let mut visit_expr = |expression: &Expr| {
        if matches!(expression, Expr::PluginEnumValue { namespace, enum_name, case_name }
            if namespace == "Camera" && enum_name == "Facing" && case_name == "back")
        {
            found = true;
        }
    };
    walk_ir(&module.body, &mut visit_node, &mut visit_expr);
    assert!(
        found,
        "the component prop should contain the typed native enum case"
    );
}
