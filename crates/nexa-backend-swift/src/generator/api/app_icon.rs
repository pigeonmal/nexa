use nexa_codegen::SourceWriter;

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"@MainActor
func nexaSetAlternateAppIcon(_ name: String?) async -> Bool {
    guard UIApplication.shared.supportsAlternateIcons else { return false }
    return await withCheckedContinuation { continuation in
        UIApplication.shared.setAlternateIconName(name) { error in
            continuation.resume(returning: error == nil)
        }
    }
}
"#,
    );
}
