use nexa_diagnostics::CompileError;

use crate::{lexer, lexer::Kind};

#[derive(Clone)]
struct FormatItem {
    kind: ItemKind,
    line_breaks_before: usize,
    had_space_before: bool,
}

#[derive(Clone)]
enum ItemKind {
    Token(lexer::Token),
    Comment(String),
}

/// Format Nexa source with stable indentation and spacing while preserving
/// comments and string literal contents. Invalid or incomplete lexical input
/// returns the lexer diagnostic and is left for the editor to repair.
pub fn format_source(source: &str) -> Result<String, CompileError> {
    format_source_with_options(source, 4, true)
}

/// Format source using the indentation preferences supplied by an LSP client.
/// `tab_size` is capped to keep malformed client settings from growing output
/// without bound.
pub fn format_source_with_options(
    source: &str,
    tab_size: usize,
    insert_spaces: bool,
) -> Result<String, CompileError> {
    let items = format_items(source)?;
    let mut output = String::with_capacity(source.len());
    let mut indentation = 0usize;
    let indent_width = tab_size.clamp(1, 16);
    let mut previous: Option<Kind> = None;

    for item in items {
        match item.kind {
            ItemKind::Comment(comment) => {
                if item.line_breaks_before > 0 {
                    apply_line_breaks(&mut output, item.line_breaks_before);
                } else if !at_line_start(&output) {
                    output.push(' ');
                }
                write_indent(&mut output, indentation, indent_width, insert_spaces);
                output.push_str(&comment);
                output.push('\n');
                previous = None;
            }
            ItemKind::Token(token) => {
                let kind = token.kind.clone();
                if matches!(kind, Kind::RBrace) {
                    if !matches!(previous, Some(Kind::LBrace)) {
                        apply_line_breaks(&mut output, 1);
                    }
                    indentation = indentation.saturating_sub(1);
                    write_indent(&mut output, indentation, indent_width, insert_spaces);
                    output.push('}');
                    previous = Some(kind);
                    continue;
                }
                if matches!(kind, Kind::Semicolon) {
                    apply_line_breaks(&mut output, 1);
                    previous = None;
                    continue;
                }

                let is_else_or_catch = matches!(
                    &kind,
                    Kind::Ident(name) if matches!(name.as_str(), "else" | "catch")
                );
                let chained_block_member = matches!(previous, Some(Kind::RBrace))
                    && (is_else_or_catch || matches!(kind, Kind::Dot));
                if item.line_breaks_before > 0 && !chained_block_member {
                    apply_line_breaks(&mut output, item.line_breaks_before);
                } else if needs_space(
                    previous.as_ref(),
                    &kind,
                    item.had_space_before,
                    is_else_or_catch,
                ) {
                    output.push(' ');
                }
                write_indent(&mut output, indentation, indent_width, insert_spaces);
                let lexeme = source
                    .get(token.span.start..token.span.end)
                    .ok_or_else(|| {
                        CompileError::new(token.span, "token span is outside the source text")
                    })?;
                output.push_str(lexeme);

                if matches!(kind, Kind::LBrace) {
                    indentation = indentation.saturating_add(1);
                    output.push('\n');
                }
                previous = Some(kind);
            }
        }
    }

    while output.ends_with(' ') || output.ends_with('\t') {
        output.pop();
    }
    if source.ends_with('\n') && !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn format_items(source: &str) -> Result<Vec<FormatItem>, CompileError> {
    let tokens = lexer::lex(source)?;
    let mut items = Vec::new();
    let mut token_index = 0usize;
    let mut offset = 0usize;
    let mut line_breaks = 0usize;
    let mut had_space = false;

    while offset < source.len() {
        if let Some(token) = tokens.get(token_index) {
            if token.kind != Kind::Eof && token.span.start == offset {
                items.push(FormatItem {
                    kind: ItemKind::Token(token.clone()),
                    line_breaks_before: line_breaks,
                    had_space_before: had_space,
                });
                offset = token.span.end;
                token_index += 1;
                line_breaks = 0;
                had_space = false;
                continue;
            }
        }

        if source[offset..].starts_with("//") {
            let end = source[offset..]
                .find('\n')
                .map(|relative| offset + relative)
                .unwrap_or(source.len());
            items.push(FormatItem {
                kind: ItemKind::Comment(source[offset..end].to_owned()),
                line_breaks_before: line_breaks,
                had_space_before: had_space,
            });
            offset = end;
            line_breaks = 0;
            had_space = false;
            continue;
        }

        let Some(character) = source[offset..].chars().next() else {
            break;
        };
        if character == '\n' {
            line_breaks = line_breaks.saturating_add(1);
        } else if character.is_whitespace() {
            had_space = true;
        }
        offset += character.len_utf8();
    }
    Ok(items)
}

fn needs_space(
    previous: Option<&Kind>,
    current: &Kind,
    had_space_before: bool,
    is_else_or_catch: bool,
) -> bool {
    if at_line_start_kind(previous) {
        return false;
    }
    if is_else_or_catch && matches!(previous, Some(Kind::RBrace)) {
        return true;
    }
    if matches!(current, Kind::LBrace) {
        return true;
    }
    if matches!(
        current,
        Kind::RParen
            | Kind::RBracket
            | Kind::Comma
            | Kind::Semicolon
            | Kind::Dot
            | Kind::Question
            | Kind::Colon
    ) {
        return false;
    }
    if matches!(
        previous,
        Some(Kind::LParen | Kind::LBracket | Kind::Dot | Kind::At)
    ) {
        return false;
    }
    if matches!(previous, Some(Kind::Comma | Kind::Colon)) {
        return true;
    }
    if is_binary_operator(current) || previous.is_some_and(is_binary_operator) {
        return !matches!(current, Kind::Bang);
    }
    if matches!(current, Kind::Less | Kind::Greater)
        || matches!(previous, Some(Kind::Less | Kind::Greater))
    {
        return had_space_before;
    }
    if matches!(current, Kind::QuestionQuestion) || previous == Some(&Kind::QuestionQuestion) {
        return true;
    }
    match previous {
        Some(Kind::RBrace) => true,
        Some(Kind::RParen | Kind::RBracket) => matches!(current, Kind::LBrace),
        Some(Kind::Ident(_) | Kind::Number(_) | Kind::String(_) | Kind::Regex(_, _)) => {
            !matches!(current, Kind::LParen | Kind::LBracket | Kind::Dot)
        }
        _ => false,
    }
}

fn is_binary_operator(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Equal
            | Kind::PlusEqual
            | Kind::MinusEqual
            | Kind::StarEqual
            | Kind::SlashEqual
            | Kind::PercentEqual
            | Kind::EqualEqual
            | Kind::BangEqual
            | Kind::AndAnd
            | Kind::OrOr
            | Kind::LessEqual
            | Kind::GreaterEqual
            | Kind::Plus
            | Kind::Minus
            | Kind::Star
            | Kind::Slash
            | Kind::Percent
            | Kind::QuestionQuestion
    )
}

fn at_line_start_kind(previous: Option<&Kind>) -> bool {
    matches!(previous, None | Some(Kind::LBrace | Kind::Semicolon))
}

fn at_line_start(output: &str) -> bool {
    output.is_empty() || output.ends_with('\n')
}

fn apply_line_breaks(output: &mut String, requested: usize) {
    let requested = requested.clamp(1, 2);
    while output.ends_with(' ') || output.ends_with('\t') {
        output.pop();
    }
    let current = output.chars().rev().take_while(|ch| *ch == '\n').count();
    for _ in current..requested {
        output.push('\n');
    }
}

fn write_indent(output: &mut String, indentation: usize, width: usize, insert_spaces: bool) {
    if at_line_start(output) {
        if insert_spaces {
            output.extend(std::iter::repeat_n(' ', indentation.saturating_mul(width)));
        } else {
            output.extend(std::iter::repeat_n('\t', indentation));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn formats_blocks_and_spacing_deterministically() {
        let source = "app Counter{state count:Int32=0\nbody{Text(count)}}\n";
        let formatted = format_source(source).expect("source tokenizes");
        assert_eq!(
            formatted,
            "app Counter {\n    state count: Int32 = 0\n    body {\n        Text(count)\n    }\n}\n"
        );
        assert_eq!(
            format_source(&formatted).expect("formatted source tokenizes"),
            formatted
        );
        crate::parse_program(&formatted).expect("formatted source remains valid");
    }

    #[test]
    fn preserves_comments_and_string_contents() {
        let source =
            "app Demo { // app comment\n body { Text(\"// literal\") // row comment\n } }\n";
        let formatted = format_source(source).expect("source tokenizes");
        assert!(formatted.contains("// app comment"));
        assert!(formatted.contains("// row comment"));
        assert!(formatted.contains("\"// literal\""));
        crate::parse_program(&formatted).expect("comments and literals remain valid");
    }

    #[test]
    fn preserves_regex_literals_and_division() {
        let source = "app Demo { state expression: Regex=/id-[0-9]+/i\nstate ratio: Float64=8/2\nbody { Text(\"ready\") } }\n";
        let formatted = format_source(source).expect("regex and division tokenize");
        assert!(formatted.contains("Regex = /id-[0-9]+/i"));
        assert!(formatted.contains("Float64 = 8 / 2"));
        assert_eq!(
            format_source(&formatted).expect("formatted regex literal tokenizes"),
            formatted
        );
        crate::parse_program(&formatted).expect("formatted source remains valid");
    }
}
