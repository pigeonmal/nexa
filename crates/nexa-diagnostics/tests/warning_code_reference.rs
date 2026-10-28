#[test]
fn warning_code_reference_matches_the_diagnostics_catalog() {
    let path = nexa_testkit::workspace_root().join("docs/language-guide.md");
    nexa_testkit::assert_region(
        &path,
        "warning-codes",
        &nexa_diagnostics::render_warning_code_reference(),
        "docs/language-guide.md#warning-codes",
    );
}
