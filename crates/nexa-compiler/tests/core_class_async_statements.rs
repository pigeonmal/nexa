use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Action;
use nexa_testkit::TestProject;

#[test]
fn async_instance_methods_lower_statement_bodies_for_both_targets() {
    let project = TestProject::new("nexa-class-async-statements");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
class Repository {
    async fn initialize() -> Bool {
        return true
    }

    async fn readValue() -> Int32 {
        if await (this.initialize()) {
            return 1
        } else {
            return 0
        }
    }

    async fn loadValue() -> Int32 {
        try {
            let value: Int32 = await this.readValue()
            Log.info(message: "loaded")
            return value
        } catch {
            else { return 0 }
        }
    }
}

app AsyncRepository {
    let repository = Repository()
    body {
        OnAppear async {
            let value = await repository.loadValue()
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("async class methods with statement bodies should compile")
            .module;
        let read = module
            .functions
            .iter()
            .find(|function| function.name == "Repository.readValue")
            .expect("instance method should be lowered");
        assert!(read.is_async);
        let read_body = read
            .body_actions
            .as_deref()
            .expect("if body uses statement IR");
        assert!(matches!(read_body.first(), Some(Action::If { .. })));

        let load = module
            .functions
            .iter()
            .find(|function| function.name == "Repository.loadValue")
            .expect("async repository method should be lowered");
        let load_body = load
            .body_actions
            .as_deref()
            .expect("try body uses statement IR");
        assert!(matches!(load_body.first(), Some(Action::TryCatch { .. })));
        let Some(Action::TryCatch {
            body,
            catch_body: Some(catch_body),
            ..
        }) = load_body.first()
        else {
            unreachable!("try/catch should retain its success and fallback blocks");
        };
        assert!(matches!(body.first(), Some(Action::Let { name, .. }) if name == "value"));
        assert!(
            body.iter()
                .any(|action| matches!(action, Action::Return { .. }))
        );
        assert!(matches!(catch_body.first(), Some(Action::Return { .. })));
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name == "Repository.initialize" && function.is_async)
        );
    }
}
