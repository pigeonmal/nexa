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

#[test]
fn optional_plugin_enum_payloads_map_to_payload_or_null_on_both_targets() {
    for (target, target_name) in [(Target::Swift, "swift"), (Target::Kotlin, "kotlin")] {
        let project = TestProject::new(&format!("nexa-optional-plugin-enum-{target_name}"));
        let plugin = project.join("sqlite");
        fs::create_dir_all(&plugin).expect("plugin directory should be created");
        fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.test.sqlite\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
        )
        .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            "enum Value { nullValue int64(value: Int64) }\n\
             native class Database {\n\
                 init(name: String)\n\
                 fn execute(parameters: Array<Value>) -> Bool\n\
             }\n",
        )
        .expect("plugin contract should be written");
        let entry = project.join("App.nx");
        fs::write(
            &entry,
            "plugin \"sqlite\" as SQLite\n\
             app Demo {\n\
                 let database = SQLite.Database(\"todo\")\n\
             let id: Int64? = 41\n\
                 state saved: Bool = database.execute([id, null])\n\
                 body { Text(saved) }\n\
             }\n",
        )
        .expect("app source should be written");

        let module = compile_file_with_warnings_for_target(&entry, target)
            .unwrap_or_else(|error| {
                panic!("nullable plugin values should compile for {target_name}: {error}")
            })
            .module;
        let saved = module
            .states
            .iter()
            .find(|state| state.name == "saved")
            .expect("saved state should exist");
        let Expr::NativeCall { arguments, .. } = &saved.initial else {
            panic!("saved initializer should call the native database method");
        };
        let Expr::Array(values) = &arguments[0].1 else {
            panic!("SQLite parameter list should remain a statically typed array");
        };
        assert!(
            matches!(
                &values[0],
                Expr::PluginEnumOptionalConstructor {
                    namespace,
                    enum_name,
                    case_name,
                    null_case_name,
                    payload_name,
                    ..
                } if namespace == "SQLite"
                    && enum_name == "Value"
                    && case_name == "int64"
                    && null_case_name == "nullValue"
                    && payload_name == "value"
            ),
            "optional Int64 must map to the typed payload or SQL NULL"
        );
        assert!(
            matches!(
                &values[1],
                Expr::PluginEnumValue { namespace, enum_name, case_name }
                    if namespace == "SQLite" && enum_name == "Value" && case_name == "nullValue"
            ),
            "the explicit null literal should keep its zero-payload case"
        );
    }
}
