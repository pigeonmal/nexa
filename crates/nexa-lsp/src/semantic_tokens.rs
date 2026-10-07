use nexa_syntax::{Token, TokenKind, catalog, tokenize};

use crate::{line_index::LineIndex, protocol::Position};

const TOKEN_TYPES: &[&str] = &[
    "namespace",
    "type",
    "class",
    "enum",
    "interface",
    "struct",
    "typeParameter",
    "parameter",
    "variable",
    "property",
    "enumMember",
    "function",
    "method",
    "macro",
    "keyword",
    "modifier",
    "comment",
    "string",
    "number",
    "operator",
    "decorator",
];

const TOKEN_MODIFIERS: &[&str] = &[
    "declaration",
    "definition",
    "readonly",
    "static",
    "deprecated",
    "abstract",
    "async",
    "modification",
    "documentation",
    "defaultLibrary",
];

pub fn token_types() -> &'static [&'static str] {
    TOKEN_TYPES
}

pub fn token_modifier_names() -> &'static [&'static str] {
    TOKEN_MODIFIERS
}

#[derive(Clone, Copy)]
enum Classification {
    Type,
    Class,
    Enum,
    Struct,
    Variable,
    Property,
    EnumMember,
    Function,
    Method,
    Keyword,
    Modifier,
    String,
    Number,
    Operator,
    Decorator,
}

impl Classification {
    fn index(self) -> u32 {
        match self {
            Self::Type => 1,
            Self::Class => 2,
            Self::Enum => 3,
            Self::Struct => 5,
            Self::Variable => 8,
            Self::Property => 9,
            Self::EnumMember => 10,
            Self::Function => 11,
            Self::Method => 12,
            Self::Keyword => 14,
            Self::Modifier => 15,
            Self::String => 17,
            Self::Number => 18,
            Self::Operator => 19,
            Self::Decorator => 20,
        }
    }
}

/// Returns the LSP semantic-token legend and delta-encoded token stream.
/// Token offsets come from the shared Nexa lexer and are converted to UTF-16
/// positions, matching the LSP default position encoding.
pub fn semantic_tokens(source: &str) -> (Vec<&'static str>, Vec<&'static str>, Vec<u32>) {
    let (Ok(tokens), index) = (tokenize(source), LineIndex::new(source)) else {
        return (TOKEN_TYPES.to_vec(), TOKEN_MODIFIERS.to_vec(), Vec::new());
    };
    let tokens: Vec<_> = tokens
        .into_iter()
        .filter(|token| token.kind != TokenKind::Eof)
        .collect();
    let declared_variables =
        collect_declarations(&tokens, &["state", "let", "var", "const", "parameter"]);
    let declared_functions = collect_declarations(&tokens, &["fn"]);
    let declared_structs = collect_declarations(&tokens, &["struct"]);
    let declared_classes = collect_declarations(&tokens, &["class", "screen", "component"]);
    let declared_enums = collect_declarations(&tokens, &["enum"]);
    let mut encoded = Vec::new();
    let mut previous = Position::default();
    let mut previous_end = 0u32;

    for (token_index, token) in tokens.iter().enumerate() {
        let Some(classification) = classify(
            token,
            token_index,
            &tokens,
            &declared_variables,
            &declared_functions,
            &declared_structs,
            &declared_classes,
            &declared_enums,
        ) else {
            continue;
        };
        let start = index.to_position(source, token.span.start);
        let end = index.to_position(source, token.span.end);
        let length = if end.line == start.line {
            end.character.saturating_sub(start.character)
        } else {
            // The lexer emits a raw-string token that may span lines. LSP
            // semantic tokens cannot cross a line, so emit just its first
            // line portion here; the raw-string content remains one token.
            source
                .get(token.span.start..token.span.end)
                .and_then(|text| text.split('\n').next())
                .map(|line| line.encode_utf16().count() as u32)
                .unwrap_or(0)
        };
        if length == 0 {
            continue;
        }
        let delta_line = start.line.saturating_sub(previous.line);
        let delta_start = if delta_line == 0 {
            start.character.saturating_sub(previous_end)
        } else {
            start.character
        };
        let modifiers = token_modifiers(token, token_index, &tokens, classification);
        encoded.extend([
            delta_line,
            delta_start,
            length,
            classification.index(),
            modifiers,
        ]);
        previous = start;
        previous_end = start.character.saturating_add(length);
    }
    (TOKEN_TYPES.to_vec(), TOKEN_MODIFIERS.to_vec(), encoded)
}

fn collect_declarations(tokens: &[Token], declaration_keywords: &[&str]) -> Vec<String> {
    tokens
        .windows(2)
        .filter_map(|pair| match (&pair[0].kind, &pair[1].kind) {
            (TokenKind::Ident(keyword), TokenKind::Ident(name))
                if declaration_keywords.contains(&keyword.as_str()) =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn classify(
    token: &Token,
    index: usize,
    tokens: &[Token],
    variables: &[String],
    functions: &[String],
    structs: &[String],
    classes: &[String],
    enums: &[String],
) -> Option<Classification> {
    match &token.kind {
        TokenKind::String(_) | TokenKind::Regex(_, _) => Some(Classification::String),
        TokenKind::Number(_) => Some(Classification::Number),
        TokenKind::At => Some(Classification::Decorator),
        TokenKind::Equal
        | TokenKind::PlusEqual
        | TokenKind::MinusEqual
        | TokenKind::StarEqual
        | TokenKind::SlashEqual
        | TokenKind::PercentEqual
        | TokenKind::EqualEqual
        | TokenKind::BangEqual
        | TokenKind::AndAnd
        | TokenKind::OrOr
        | TokenKind::LessEqual
        | TokenKind::GreaterEqual
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Bang
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::Question
        | TokenKind::QuestionQuestion => Some(Classification::Operator),
        TokenKind::Ident(name) => {
            let previous = index.checked_sub(1).and_then(|i| tokens.get(i));
            let next = tokens.get(index + 1);
            let declaration = previous.and_then(ident).unwrap_or_default();
            if matches!(declaration, "async" | "throws") {
                return Some(Classification::Modifier);
            }
            if matches!(declaration, "state" | "let" | "var" | "const") {
                return Some(Classification::Variable);
            }
            if declaration == "fn" {
                return Some(Classification::Function);
            }
            if declaration == "struct" {
                return Some(Classification::Struct);
            }
            if matches!(declaration, "screen" | "component" | "class") {
                return Some(Classification::Class);
            }
            if declaration == "enum" {
                return Some(Classification::Enum);
            }
            if declaration == "case" {
                return Some(Classification::EnumMember);
            }
            if catalog::keyword(name).is_some() {
                return Some(if matches!(name.as_str(), "async" | "throws") {
                    Classification::Modifier
                } else {
                    Classification::Keyword
                });
            }
            if catalog::builtin_type(name).is_some() {
                return Some(Classification::Type);
            }
            if catalog::component(name).is_some() {
                return Some(Classification::Class);
            }
            if declared_match(name, structs) {
                return Some(Classification::Struct);
            }
            if declared_match(name, classes) {
                return Some(Classification::Class);
            }
            if declared_match(name, enums) {
                return Some(Classification::Enum);
            }
            if declared_match(name, functions) {
                return Some(Classification::Function);
            }
            if variables.iter().any(|variable| variable == name) {
                return Some(Classification::Variable);
            }
            if previous.is_some_and(|previous| previous.kind == TokenKind::Dot) {
                return Some(
                    if matches!(next.map(|token| &token.kind), Some(TokenKind::LParen)) {
                        Classification::Method
                    } else {
                        Classification::Property
                    },
                );
            }
            if next.is_some_and(|next| next.kind == TokenKind::Colon) {
                return Some(Classification::Property);
            }
            if matches!(next.map(|token| &token.kind), Some(TokenKind::LParen)) {
                return Some(Classification::Function);
            }
            if name.chars().next().is_some_and(char::is_uppercase) {
                return Some(Classification::Type);
            }
            None
        }
        _ => None,
    }
}

fn token_modifiers(
    token: &Token,
    index: usize,
    tokens: &[Token],
    classification: Classification,
) -> u32 {
    let name = ident(token).unwrap_or_default();
    let is_declaration = index > 0
        && tokens
            .get(index - 1)
            .and_then(ident)
            .is_some_and(|keyword| {
                matches!(
                    keyword,
                    "state"
                        | "let"
                        | "var"
                        | "const"
                        | "fn"
                        | "struct"
                        | "class"
                        | "screen"
                        | "component"
                        | "enum"
                        | "case"
                )
            });
    let writes = tokens.get(index + 1).is_some_and(|next| {
        matches!(
            next.kind,
            TokenKind::Equal
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual
        )
    });
    let mut modifiers = 0;
    if is_declaration {
        modifiers |= 1 << 0;
        modifiers |= 1 << 1;
    }
    if matches!(classification, Classification::Variable) && writes && !is_declaration {
        modifiers |= 1 << 7;
    }
    if catalog::component(name).is_some() || catalog::builtin_type(name).is_some() {
        modifiers |= 1 << 9;
    }
    modifiers
}

fn ident(token: &Token) -> Option<&str> {
    match &token.kind {
        TokenKind::Ident(name) => Some(name),
        _ => None,
    }
}

fn declared_match(name: &str, declarations: &[String]) -> bool {
    declarations.iter().any(|declaration| declaration == name)
}

#[cfg(test)]
mod tests {
    use super::semantic_tokens;

    #[test]
    fn emits_delta_encoded_keywords_types_components_and_state_writes() {
        let source = "app Counter {\n state count: Int32 = 0\n body { Text(count) }\n}\n";
        let (types, modifiers, data) = semantic_tokens(source);
        assert!(types.contains(&"class"));
        assert!(types.contains(&"keyword"));
        assert!(modifiers.contains(&"modification"));
        assert_eq!(data.len() % 5, 0);
        assert!(data.as_chunks::<5>().0.iter().any(|token| token[3] == 8));
        assert!(data.as_chunks::<5>().0.iter().any(|token| token[3] == 2));
    }

    #[test]
    fn semantic_tokens_respect_utf16_columns() {
        let source = "state title = \"é\"\nText(title)\n";
        let (_, _, data) = semantic_tokens(source);
        let first_token = data.as_chunks::<5>().0.first().expect("keyword token");
        assert_eq!(first_token[0], 0);
        assert_eq!(first_token[1], 0);
    }

    #[test]
    fn regex_literals_are_highlighted_as_string_literals() {
        let literal = "/order-[0-9]+/i";
        let source =
            format!("app Orders {{ state matcher: Regex = {literal} body {{ Text(\"ready\") }} }}");
        let (_, _, data) = semantic_tokens(&source);
        assert!(
            data.as_chunks::<5>()
                .0
                .iter()
                .any(|token| token[2] == literal.len() as u32 && token[3] == 17)
        );
    }
}
