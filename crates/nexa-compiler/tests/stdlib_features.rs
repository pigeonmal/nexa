use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn regex_apis_compile_on_both_backends() {
    let project = TestProject::new("nexa-regex-test");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app RegexApp {
    state matched = false
    state items: Array<String> = []
    state replaced = ""
    body {
        OnAppear {
            matched = Regex.isMatch("^[0-9]+$", "12345")
            items = Regex.matches("[a-z]+", "apple banana cherry")
            replaced = Regex.replace("[0-9]", "item123", "X")
        }
        Text("\(matched)")
    }
}
"#,
    )
    .expect("test file should be written");

    // Swift target verification
    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("Regex should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(
        swift_out.contains("nexaRegexIsMatch(\"^[0-9]+$\", in: \"12345\")"),
        "Swift output should call nexaRegexIsMatch: {swift_out}"
    );
    assert!(
        swift_out.contains("nexaRegexMatches(\"[a-z]+\", in: \"apple banana cherry\")"),
        "Swift output should call nexaRegexMatches: {swift_out}"
    );
    assert!(
        swift_out.contains("nexaRegexReplace(\"[0-9]\", in: \"item123\", with: \"X\")"),
        "Swift output should call nexaRegexReplace: {swift_out}"
    );

    // Kotlin target verification
    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("Regex should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(
        kotlin_out.contains(".containsMatchIn(\"12345\")"),
        "Kotlin output should call containsMatchIn: {kotlin_out}"
    );
    assert!(
        kotlin_out.contains(".findAll(\"apple banana cherry\").map { it.value }.toList()"),
        "Kotlin output should call findAll: {kotlin_out}"
    );
    assert!(
        kotlin_out.contains("Regex(\"[0-9]\").replace(\"item123\", \"X\")"),
        "Kotlin output should call replace: {kotlin_out}"
    );
}

#[test]
fn string_manipulation_suite_compiles_on_both_backends() {
    let project = TestProject::new("nexa-string-suite-test");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app StringApp {
    state raw = "  Hello World  "
    state trimmed = ""
    state lower = ""
    state upper = ""
    state parts: Array<String> = []
    state hasWorld = false
    state starts = false
    state ends = false
    body {
        OnAppear {
            trimmed = raw.trim()
            lower = raw.toLowercase()
            upper = raw.toUppercase()
            parts = raw.split(" ")
            hasWorld = raw.contains("World")
            starts = raw.startsWith("  Hello")
            ends = raw.endsWith("World  ")
        }
        Text(trimmed)
    }
}
"#,
    )
    .expect("test file should be written");

    // Swift target verification
    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("String methods should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(swift_out.contains(".trimmingCharacters(in: .whitespacesAndNewlines)"));
    assert!(swift_out.contains(".lowercased()"));
    assert!(swift_out.contains(".uppercased()"));
    assert!(swift_out.contains(".components(separatedBy: \" \")"));
    assert!(swift_out.contains(".contains(\"World\")"));
    assert!(swift_out.contains(".hasPrefix(\"  Hello\")"));
    assert!(swift_out.contains(".hasSuffix(\"World  \")"));

    // Kotlin target verification
    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("String methods should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(kotlin_out.contains(".trim()"));
    assert!(kotlin_out.contains(".lowercase()"));
    assert!(kotlin_out.contains(".uppercase()"));
    assert!(kotlin_out.contains(".split(\" \")"));
    assert!(kotlin_out.contains(".contains(\"World\")"));
    assert!(kotlin_out.contains(".startsWith(\"  Hello\")"));
    assert!(kotlin_out.contains(".endsWith(\"World  \")"));
}

#[test]
fn map_methods_compile_on_both_backends() {
    let project = TestProject::new("nexa-map-test");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app MapApp {
    state scores: Map<String, Int32> = ["alice": 100, "bob": 85]
    state aliceScore: Int32? = null
    state hasBob = false
    state names: Array<String> = []
    state values: Array<Int32> = []
    body {
        OnAppear {
            scores.set("charlie", 92)
            aliceScore = scores.get("alice")
            hasBob = scores.contains("bob")
            names = scores.keys()
            values = scores.values()
        }
        Text("\(hasBob)")
    }
}
"#,
    )
    .expect("test file should be written");

    // Swift target verification
    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("Map methods should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(swift_out.contains("nexa_scores[\"charlie\"] = 92"));
    assert!(swift_out.contains("nexa_scores[\"alice\"]"));
    assert!(swift_out.contains("(nexa_scores[\"bob\"] != nil)"));
    assert!(swift_out.contains("Array(nexa_scores.keys)"));
    assert!(swift_out.contains("Array(nexa_scores.values)"));

    // Kotlin target verification
    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("Map methods should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(kotlin_out.contains("nexa_scores[\"charlie\"] = 92"));
    assert!(kotlin_out.contains("nexa_scores[\"alice\"]"));
    assert!(kotlin_out.contains("nexa_scores.containsKey(\"bob\")"));
    assert!(kotlin_out.contains("nexa_scores.keys.toList()"));
    assert!(kotlin_out.contains("nexa_scores.values.toList()"));
}

#[test]
fn json_typed_parse_and_stringify_compile_on_both_backends() {
    let project = TestProject::new("nexa-json-test");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
struct UserProfile {
    id: String,
    score: Int32,
}

app JsonApp {
    state profile = UserProfile("u1", 42)
    state encoded = ""
    state decoded: Result<UserProfile, JsonError>? = null
    body {
        OnAppear {
            encoded = JSON.stringify(profile)
            decoded = JSON.parse<UserProfile>(encoded)
        }
        Text(encoded)
    }
}
"#,
    )
    .expect("test file should be written");

    let swift_compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("JSON should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift_compiled.module);
    assert!(swift_out.contains("nexaJsonStringify"));
    assert!(swift_out.contains("nexaJsonParse"));

    let kotlin_compiled = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("JSON should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_compiled.module);
    assert!(kotlin_out.contains("nexaJsonStringify"));
    assert!(kotlin_out.contains("nexaJsonParse"));
}
