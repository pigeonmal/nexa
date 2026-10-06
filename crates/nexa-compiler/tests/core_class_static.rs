use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn static_class_properties_emit_native_members_and_allow_static_only_classes() {
    let project = TestProject::new("nexa-class-static");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
class AppConfig {
    static let appName: String = "Nexa"
    static let defaultCount: Int32 = 3

    static fn welcome(prefix: String) -> String {
        return prefix + AppConfig.appName
    }

    static async fn asyncWelcome(prefix: String) -> String {
        return prefix + AppConfig.appName
    }
}

class AppRepository(val prefix: String) {
    async fn load(suffix: String) -> String {
        return prefix + suffix
    }
}

app StaticClassExample {
    let repository = AppRepository("Hello, ")

    body {
        Text(AppConfig.appName)
        Text(AppConfig.welcome("Hello, "))
        Text(AppConfig.welcome(prefix: "Hello, "))
        OnAppear async {
            Log.info(message: await AppConfig.asyncWelcome("Hello, "))
            Log.info(message: await repository.load("Nexa"))
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("static class members should compile for both targets")
            .module;
        assert!(
            module
                .globals
                .iter()
                .any(|global| { global.name == "AppConfig::appName" && !global.mutable })
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| { function.name == "AppConfig.asyncWelcome" && function.is_async })
        );
        assert!(module.functions.iter().any(|function| {
            function.name == "AppRepository.load"
                && function.is_async
                && function.receiver.is_some()
        }));

        let (native, expected_class, expected_member, expected_access) = match target {
            Target::Swift => (
                nexa_backend_swift::SwiftBackend.generate(&module),
                "final class NexaAppConfig",
                "static let nexa_appName: String = \"Nexa\"",
                "NexaAppConfig.nexa_appName",
            ),
            Target::Kotlin => (
                nexa_backend_kotlin::KotlinBackend.generate(&module),
                "class NexaAppConfig()",
                "val nexa_appName: String = \"Nexa\"",
                "NexaAppConfig.nexa_appName",
            ),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains(expected_class), "{native}");
        assert!(native.contains(expected_member), "{native}");
        assert!(native.contains(expected_access), "{native}");
        match target {
            Target::Swift => {
                assert!(native.contains("static func nexa_fn_welcome"), "{native}");
                assert!(
                    native.contains("NexaAppConfig.nexa_fn_welcome(\"Hello, \")"),
                    "{native}"
                );
                assert!(
                    native.contains("static func nexa_fn_asyncWelcome")
                        && native.contains("async -> String"),
                    "{native}"
                );
                assert!(
                    native.contains("func nexa_fn_load(_ nexa_suffix: String) async -> String"),
                    "{native}"
                );
                assert!(
                    native.contains("await nexa_repository.nexa_fn_load(\"Nexa\")"),
                    "{native}"
                );
            }
            Target::Kotlin => {
                assert!(native.contains("companion object"), "{native}");
                assert!(native.contains("fun nexa_fn_welcome"), "{native}");
                assert!(
                    native.contains("NexaAppConfig.nexa_fn_welcome(\"Hello, \")"),
                    "{native}"
                );
                assert!(
                    native.contains("suspend fun nexa_fn_asyncWelcome"),
                    "{native}"
                );
                assert!(
                    native.contains("suspend fun nexa_fn_load(nexa_suffix: String): String"),
                    "{native}"
                );
                assert!(
                    native.contains("nexa_repository.nexa_fn_load(\"Nexa\")"),
                    "{native}"
                );
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
        assert!(!native.contains("nexa_AppConfig::appName"), "{native}");
    }
}
