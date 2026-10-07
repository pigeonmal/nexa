use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn functions_and_class_methods_with_throws_emit_throwing_signatures_on_both_backends() {
    let project = TestProject::new("nexa-throws-functions");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
class DataService {
    fn validate(code: Int32) throws -> Bool {
        return code > 0
    }

    async fn fetch(id: String) throws -> String {
        return "data:" + id
    }
}

async fn runPipeline() throws -> String {
    let service = DataService()
    let valid = service.validate(42)
    let payload = await service.fetch("sample")
    return payload
}

app ThrowsApp {
    state result = ""
    body {
        OnAppear async {
            try {
                result = await runPipeline()
            } catch {
                result = "fallback"
            }
        }
        Text(result)
    }
}
"#,
    )
    .expect("test file should be written");

    // Swift target verification
    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("throwing functions and methods should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);

    assert!(
        swift_out.contains("func nexa_fn_validate(_ nexa_code: Int32) throws -> Bool"),
        "Swift output should declare throwing synchronous method: {swift_out}"
    );
    assert!(
        swift_out.contains("func nexa_fn_fetch(_ nexa_id: String) async throws -> String"),
        "Swift output should declare throwing async method: {swift_out}"
    );
    assert!(
        swift_out.contains("func nexa_fn_runPipeline() async throws -> String"),
        "Swift output should declare throwing top-level function: {swift_out}"
    );
    assert!(
        swift_out.contains("try nexa_service.nexa_fn_validate(42)"),
        "Swift output should invoke synchronous throwing call with `try`: {swift_out}"
    );
    assert!(
        swift_out.contains("(try await nexa_service.nexa_fn_fetch("),
        "Swift output should invoke async throwing call with `try await`: {swift_out}"
    );

    // Kotlin target verification
    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("throwing functions and methods should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(
        kotlin_out.contains("fun nexa_fn_validate(nexa_code: Int): Boolean"),
        "Kotlin output should declare method: {kotlin_out}"
    );
    assert!(
        kotlin_out.contains("suspend fun nexa_fn_fetch(nexa_id: String): String"),
        "Kotlin output should declare suspend method: {kotlin_out}"
    );
}

#[test]
fn calling_throwing_function_without_error_handling_fails_compilation() {
    let project = TestProject::new("nexa-unhandled-throws");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
fn risk() throws -> Int32 {
    return 100
}

app UnhandledApp {
    state count = 0
    body {
        OnAppear {
            count = risk()
        }
        Text("\(count)")
    }
}
"#,
    )
    .expect("test file should be written");

    let Err(err) = compile_file_with_warnings_for_target(&entry, Target::Swift) else {
        panic!("calling a throwing function without try/catch or throws must fail");
    };
    assert!(
        err.message.contains("may throw") || err.message.contains("try"),
        "Error message should mention unhandled throwing call, got: {}",
        err.message
    );
}

#[test]
fn throwing_function_allows_direct_throwing_calls_without_try_catch() {
    let project = TestProject::new("nexa-throwing-caller");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
fn stepA() throws -> Int32 {
    return 10
}

fn stepB() throws -> Int32 {
    let a = stepA()
    return a * 2
}

app NestedApp {
    state total = 0
    body {
        OnAppear {
            try {
                total = stepB()
            } catch {
                total = 0
            }
        }
        Text("\(total)")
    }
}
"#,
    )
    .expect("test file should be written");

    let compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("stepB declared with `throws` must be allowed to call stepA without try/catch");
    assert_eq!(compiled.module.functions.len(), 2);
    assert!(compiled.module.functions.iter().all(|f| f.is_throwing));
}

#[test]
fn flat_and_chained_catch_compiles_on_both_backends() {
    let project = TestProject::new("nexa-flat-catch");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
fn compute() throws -> Int32 {
    return 42
}

app FlatCatchApp {
    state value = 0
    body {
        OnAppear {
            try {
                value = compute()
            } catch (err) {
                value = -1
            }
        }
        Text("\(value)")
    }
}
"#,
    )
    .expect("test file should be written");

    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("flat catch should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(swift_out.contains("do {"));
    assert!(swift_out.contains("} catch {"));

    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("flat catch should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(kotlin_out.contains("try {"));
    assert!(kotlin_out.contains("} catch (_: Exception) {"));
}

#[test]
fn postfix_try_operator_propagates_in_throwing_function() {
    let project = TestProject::new("nexa-postfix-try");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
fn parseData(val: Result<Int32, String>) throws -> Int32 {
    let unwrapped = val?
    return unwrapped + 1
}

app TryOpApp {
    state num = 0
    body {
        OnAppear {
            try {
                num = parseData(Ok(10))
            } catch {
                num = -1
            }
        }
        Text("\(num)")
    }
}
"#,
    )
    .expect("test file should be written");

    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("postfix ? should compile in throwing function for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(swift_out.contains("try nexa_val.get()"));

    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("postfix ? should compile in throwing function for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(kotlin_out.contains("nexa_val.getOrThrow()"));
}
