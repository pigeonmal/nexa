use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

const SOURCE: &str = r##"struct TimedWord {
    text: String,
    startTimeSeconds: Float64,
    endTimeSeconds: Float64,
    hasTrailingSpace: Bool?,
}

app TimedTextExample {
    state words: Array<TimedWord> = [
        TimedWord(text: "Stay", startTimeSeconds: 1.0, endTimeSeconds: 1.4, hasTrailingSpace: true),
        TimedWord(text: "close", startTimeSeconds: 1.5, endTimeSeconds: 2.0, hasTrailingSpace: false),
    ]
    state position: Float64 = 1.0
    state isActive: Bool = true
    state isPlaying: Bool = true
    state revealEnabled: Bool = true
    state seekRevision: Int32 = 0

    body {
        TextReveal(
            "Stay close",
            segments: words,
            position: position,
            isActive: isActive,
            isPlaying: isPlaying,
            revealEnabled: revealEnabled,
            seekRevision: seekRevision,
            fontSize: 29,
            fontFamily: "Gilroy-Regular",
            fontWeight: Bold,
            color: "#FFFFFF",
            lineHeight: 38,
            letterSpacing: -0.55,
        )
    }
}
"##;

fn compile(source: &str, target: Target) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new("nexa-core-text-reveal");
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("test source should be written");
    compile_file_with_warnings_for_target(&entry, target)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

#[test]
fn text_reveal_keeps_segment_values_typed_and_generates_both_native_accessors() {
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(SOURCE, target).expect("timed text should compile");
        let Some(Node::Text { style, .. }) = module.body.first() else {
            panic!("TextReveal must reuse the shared Text IR node");
        };
        let reveal = style
            .reveal
            .as_ref()
            .expect("reveal configuration should be typed");
        assert!(matches!(
            reveal.segment_type,
            nexa_ir::Type::Struct { ref name, .. } if name == "TimedWord"
        ));
        assert_eq!(reveal.dim_opacity, 0.38);
        assert_eq!(reveal.lead_time, 0.3);
        assert_eq!(reveal.target_offset, 0.56);
        assert_eq!(reveal.smoothing_duration, 0.25);

        match target {
            Target::Swift => {
                let generated = nexa_backend_swift::SwiftBackend.generate_units(&module);
                let source = generated
                    .units
                    .iter()
                    .map(|unit| unit.contents.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(source.contains("NexaTextRevealPrimitive("));
                assert!(source.contains("segments: nexa_words"));
                assert!(source.contains("textOf: { $0.nexa_field_text }"));
                assert!(source.contains("Double($0.nexa_field_startTimeSeconds)"));
                assert!(source.contains("$0.nexa_field_hasTrailingSpace ?? true"));
                assert!(source.contains("isActive:"));
                assert!(source.contains("isPlaying:"));
                assert!(source.contains("revealEnabled:"));
                assert!(source.contains("CADisplayLink"));
            }
            Target::Kotlin => {
                let (source, _) =
                    nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
                assert!(source.contains("NexaTextRevealPrimitive("));
                assert!(source.contains("segments = nexa_words"));
                assert!(source.contains("textOf = { segment -> segment.nexa_field_text }"));
                assert!(source.contains("segment.nexa_field_startTimeSeconds.toDouble()"));
                assert!(source.contains("segment.nexa_field_hasTrailingSpace ?: true"));
                assert!(source.contains("isActive ="));
                assert!(source.contains("isPlaying ="));
                assert!(source.contains("revealEnabled ="));
                assert!(source.contains("drawText(textLayout, color = resolvedColor)"));
            }
            Target::All => unreachable!("this test compiles one native target at a time"),
        }
    }
}

#[test]
fn text_reveal_uses_explicit_boolean_names_and_rejects_ambiguous_legacy_names() {
    let legacy = SOURCE
        .replace("isActive:", "active:")
        .replace("isPlaying:", "playing:")
        .replace("revealEnabled:", "enabled:");
    let error = compile(&legacy, Target::Swift).expect_err("legacy boolean names are removed");
    assert!(
        error.contains("unknown option `active`"),
        "unexpected diagnostic: {error}"
    );
}

#[test]
fn text_reveal_rejects_untyped_or_incomplete_segments() {
    let invalid = SOURCE
        .replace("Array<TimedWord>", "Array<String>")
        .replace(
            "TimedWord(text: \"Stay\", startTimeSeconds: 1.0, endTimeSeconds: 1.4, hasTrailingSpace: true),",
            "\"Stay\",",
        )
        .replace(
            "TimedWord(text: \"close\", startTimeSeconds: 1.5, endTimeSeconds: 2.0, hasTrailingSpace: false),",
            "\"close\",",
        );
    let error = compile(&invalid, Target::Swift).expect_err("string arrays are not timed segments");
    assert!(
        error.contains("elements must be declared structs"),
        "{error}"
    );

    let missing_time = SOURCE
        .replace("endTimeSeconds: Float64,", "durationSeconds: Float64,")
        .replace("endTimeSeconds: 1.4", "durationSeconds: 1.4")
        .replace("endTimeSeconds: 2.0", "durationSeconds: 2.0");
    let error =
        compile(&missing_time, Target::Swift).expect_err("segments need explicit end times");
    assert!(error.contains("numeric `endTimeSeconds` field"), "{error}");
}

#[test]
fn text_reveal_timing_fields_are_numeric() {
    let invalid = SOURCE
        .replace("startTimeSeconds: Float64,", "startTimeSeconds: String,")
        .replace("startTimeSeconds: 1.0", "startTimeSeconds: \"one\"")
        .replace(
            "startTimeSeconds: 1.5",
            "startTimeSeconds: \"one and a half\"",
        );
    let error = compile(&invalid, Target::Swift).expect_err("text timing must be numeric");
    assert!(
        error.contains("numeric `startTimeSeconds` field"),
        "{error}"
    );
}
