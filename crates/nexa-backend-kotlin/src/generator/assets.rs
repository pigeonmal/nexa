/// Emits the small, feature-gated drawable resolver used by generated images.
///
/// Asset names stay source-level strings so projects can provide platform
/// resources without changing generated Kotlin. Resolution is remembered per
/// composition and falls back to a transparent painter when an optional asset
/// is not present, keeping generated projects buildable before assets are added.
pub(super) fn render(out: &mut String) {
    out.push_str(
        r#"
@Composable
private fun nexaDrawablePainter(name: String): Painter {
    val context = LocalContext.current
    val resourceId = remember(context, name) {
        context.resources.getIdentifier(
            name,
            "drawable",
            context.packageName,
        )
    }
    return if (resourceId == 0) {
        ColorPainter(Color.Transparent)
    } else {
        painterResource(resourceId)
    }
}

"#,
    );
}
