use nexa_codegen::SourceWriter;

pub(crate) fn number(value: f32) -> String {
    value.to_string()
}

/// Writes the indentation prefix for `depth`.
///
/// Prefer `SourceWriter::line_at` where the indent is immediately followed by a
/// line; this remains for the sequences that indent, then write fragments.
pub(crate) fn indent(out: &mut SourceWriter, depth: usize) {
    out.indent(depth);
}

pub(crate) fn swift_string(value: &str) -> String {
    format!("\"{}\"", swift_string_content(value))
}

pub(crate) fn swift_string_content(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        escaped.push_str(match character {
            '\\' => "\\\\",
            '"' => "\\\"",
            '\n' => "\\n",
            '\r' => "\\r",
            '\t' => "\\t",
            _ => "",
        });
        if !matches!(character, '\\' | '"' | '\n' | '\r' | '\t') {
            escaped.push(character);
        }
    }
    escaped
}
