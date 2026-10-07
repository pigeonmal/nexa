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
fn regex_values_return_typed_matches_ranges_and_capture_groups() {
    let project = TestProject::new("nexa-regex-match-values-test");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app RegexMatchApp {
    state expression: Regex = Regex(pattern: "([A-Z][a-z]+) ([0-9]+)")
    state literal: Regex = /([A-Z][a-z]+) ([0-9]+)/i
    state matched: Bool = false
    state first: RegexMatch? = null
    state all: Array<RegexMatch> = []
    state range: Range? = null
    state start: Int64 = 0
    state text: String = ""
    state groups: Array<String?> = []
    state replaced: String = ""
    body {
        OnAppear {
            matched = expression.matches("Order 42")
            matched = literal.matches("Order 42")
            first = expression.find(text: "Order 42")
            range = first?.range
            start = range?.lowerBound ?? 0
            all = expression.findAll("Order 42, Item 7")
            text = first?.value ?? ""
            groups = first?.groups ?? []
            replaced = expression.replace("Order 42", with: "Order #")
        }
        Text(text)
    }
}
"#,
    )
    .expect("Regex match test app should be written");

    let swift = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("Regex values should compile for Swift");
    let swift_out = nexa_backend_swift::SwiftBackend.generate(&swift.module);
    assert!(swift_out.contains("NexaRegex(pattern: \"([A-Z][a-z]+) ([0-9]+)\")"));
    assert!(
        swift_out.contains("NexaRegex(pattern: \"(?i)([A-Z][a-z]+) ([0-9]+)\")"),
        "Swift output should include the case-insensitive Regex literal: {swift_out}"
    );
    assert!(swift_out.contains(".matches(\"Order 42\")"));
    assert!(swift_out.contains(".find(\"Order 42\")"));
    assert!(swift_out.contains(".findAll(\"Order 42, Item 7\")"));
    assert!(swift_out.contains(".replace(\"Order 42\", with: \"Order #\")"));
    assert!(swift_out.contains("struct NexaRegexRange"));
    assert!(swift_out.contains("let range: NexaRegexRange"));
    assert!(swift_out.contains("lowerBound: Int64(match.range.location)"));
    assert!(swift_out.contains(".lowerBound"));
    assert!(swift_out.contains("let groups: [String?]"));

    let kotlin = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("Regex values should compile for Kotlin");
    let kotlin_out = nexa_backend_kotlin::KotlinBackend.generate(&kotlin.module);
    assert!(kotlin_out.contains("NexaRegex(\"([A-Z][a-z]+) ([0-9]+)\")"));
    assert!(
        kotlin_out.contains("NexaRegex(\"(?i)([A-Z][a-z]+) ([0-9]+)\")"),
        "Kotlin output should include the case-insensitive Regex literal: {kotlin_out}"
    );
    assert!(kotlin_out.contains(".matches(\"Order 42\")"));
    assert!(kotlin_out.contains(".find(\"Order 42\")"));
    assert!(kotlin_out.contains(".findAll(\"Order 42, Item 7\")"));
    assert!(kotlin_out.contains(".replace(\"Order 42\", \"Order #\")"));
    assert!(kotlin_out.contains("data class NexaRegexRange"));
    assert!(kotlin_out.contains("val range: NexaRegexRange"));
    assert!(kotlin_out.contains("NexaRegexRange(match.range.first.toLong(),"));
    assert!(kotlin_out.contains(".lowerBound"));
    assert!(kotlin_out.contains("val groups: List<String?>"));
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
