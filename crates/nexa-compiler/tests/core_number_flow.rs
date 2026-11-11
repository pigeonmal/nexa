use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::{Node, NumberFlowFormat};
use nexa_testkit::TestProject;

const SOURCE: &str = r##"app NumberFlowExample {
    state price: Float64 = 42.99
    body {
        NumberFlow(
            value: price,
            format: Currency,
            currencyCode: "EUR",
            maximumFractionDigits: 2,
            animation: Spring(response: 0.35, damping: 0.82),
            fontSize: 32,
        )
    }
}"##;

fn compile(source: &str, target: Target) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new("nexa-core-number-flow");
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("test source should be written");
    compile_file_with_warnings_for_target(&entry, target)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

#[test]
fn number_flow_lowers_typed_value_and_generates_native_rolling_renderers() {
    for target in [Target::Swift, Target::Kotlin] {
        let module = compile(SOURCE, target).expect("NumberFlow should compile");
        let Some(Node::Text { style, .. }) = module.body.first() else {
            panic!("NumberFlow should reuse the shared Text IR node");
        };
        let number_flow = style.number_flow.as_ref().expect("NumberFlow is typed");
        assert_eq!(number_flow.format, NumberFlowFormat::Currency);
        assert_eq!(number_flow.currency_code.as_deref(), Some("EUR"));
        assert_eq!(number_flow.minimum_fraction_digits, None);
        assert_eq!(number_flow.maximum_fraction_digits, Some(2));
        let serialized_style = serde_json::to_value(style).expect("style serializes to Dev IR");
        assert_eq!(serialized_style["number_flow"]["format"], "Currency");
        assert_eq!(serialized_style["number_flow"]["currency_code"], "EUR");

        match target {
            Target::Swift => {
                let generated = nexa_backend_swift::SwiftBackend.generate_units(&module);
                let source = generated
                    .units
                    .iter()
                    .map(|unit| unit.contents.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(source.contains("NexaNumberFlowPrimitive("));
                assert!(source.contains("format: \"Currency\""));
                assert!(source.contains("currencyCode: \"EUR\""));
                assert!(source.contains("maximumFractionDigits: 2"));
                assert!(source.contains("NexaNumberFlowDigit"));
                assert!(source.contains("cachedDigitGlyphs"));
            }
            Target::Kotlin => {
                let (source, _) =
                    nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
                assert!(source.contains("NexaNumberFlowPrimitive("));
                assert!(source.contains("format = \"Currency\""));
                assert!(source.contains("currencyCode = \"EUR\""));
                assert!(source.contains("maximumFractionDigits = 2"));
                assert!(source.contains("AnimatedContent("));
                assert!(source.contains("remember(formatted)"));
                assert!(!source.contains(".animateContentSize()"));
            }
            Target::All => unreachable!("the test compiles one target at a time"),
        }
    }
}

#[test]
fn kotlin_emits_the_inset_helper_when_padding_is_used_without_scroll_content() {
    let module = compile(
        r#"app InsetsExample {
            body { Text("Padded", padding: 16) }
        }"#,
        Target::Kotlin,
    )
    .expect("padding compiles");
    let (source, _) = nexa_backend_kotlin::KotlinBackend.generate_with_project_features(&module);
    assert!(source.contains("NexaViewInsetsPrimitive("));
    assert!(
        source.contains(
            "@androidx.compose.runtime.Composable\ninternal fun NexaViewInsetsPrimitive("
        )
    );
}

#[test]
fn number_flow_validates_numeric_values_currency_and_fraction_precision() {
    let invalid_value = SOURCE.replace(
        "state price: Float64 = 42.99",
        "state price: String = \"42.99\"",
    );
    assert!(
        compile(&invalid_value, Target::Swift)
            .expect_err("string value is invalid")
            .contains("must be numeric")
    );

    let missing_currency = SOURCE.replace("            currencyCode: \"EUR\",\n", "");
    assert!(
        compile(&missing_currency, Target::Swift)
            .expect_err("currency code is required")
            .contains("requires `currencyCode`")
    );

    let invalid_precision = SOURCE.replace("maximumFractionDigits: 2", "maximumFractionDigits: 21");
    assert!(
        compile(&invalid_precision, Target::Swift)
            .expect_err("fraction precision is bounded")
            .contains("0 to 20")
    );

    let inverted_precision = SOURCE.replace(
        "maximumFractionDigits: 2",
        "minimumFractionDigits: 3, maximumFractionDigits: 2",
    );
    assert!(
        compile(&inverted_precision, Target::Swift)
            .expect_err("minimum cannot exceed maximum")
            .contains("cannot exceed")
    );

    let unsupported_currency_code = SOURCE.replace("\"EUR\"", "\"EURO\"");
    assert!(
        compile(&unsupported_currency_code, Target::Swift)
            .expect_err("currency code shape is validated")
            .contains("three-letter ISO 4217")
    );
}

#[test]
fn number_flow_defaults_to_number_and_percent_accepts_fractional_values() {
    let number_source = SOURCE
        .replace("            format: Currency,\n", "")
        .replace("            currencyCode: \"EUR\",\n", "")
        .replace("            maximumFractionDigits: 2,\n", "");
    let module = compile(&number_source, Target::Swift).expect("format defaults to Number");
    let Some(Node::Text { style, .. }) = module.body.first() else {
        panic!("NumberFlow should lower to Text");
    };
    let number_flow = style
        .number_flow
        .as_ref()
        .expect("NumberFlow config is typed");
    assert_eq!(number_flow.format, NumberFlowFormat::Number);
    assert_eq!(number_flow.currency_code, None);
    assert!(matches!(
        &number_flow.animation,
        nexa_ir::AnimationSpec::Spring {
            response,
            damping
        } if *response == 0.35 && *damping == 0.82
    ));

    let percent_source = number_source.replace("value: price,", "value: price, format: Percent,");
    let module =
        compile(&percent_source, Target::Kotlin).expect("Percent does not require a currency code");
    let Some(Node::Text { style, .. }) = module.body.first() else {
        panic!("NumberFlow should lower to Text");
    };
    assert_eq!(
        style.number_flow.as_ref().expect("typed config").format,
        NumberFlowFormat::Percent
    );
}
