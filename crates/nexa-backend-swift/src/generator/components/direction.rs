use nexa_codegen::SourceWriter;
use nexa_ir::DirectionConfig;

/// Renders the `.environment(\.layoutDirection, ...)` modifier that pins the
/// whole screen to one reading direction.
pub(crate) fn render(config: Option<DirectionConfig>, depth: usize, out: &mut SourceWriter) {
    let Some(config) = config else {
        return;
    };
    let direction = match config.style {
        nexa_ir::DirectionStyle::Ltr => "leftToRight",
        nexa_ir::DirectionStyle::Rtl => "rightToLeft",
    };
    out.push('\n');
    out.text_at(
        depth + 1,
        format_args!(".environment(\\.layoutDirection, .{direction})"),
    );
}
