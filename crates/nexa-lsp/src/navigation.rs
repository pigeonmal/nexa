use std::collections::HashMap;

use nexa_diagnostics::Span;
use nexa_syntax::{TokenKind, tokenize};

use crate::{line_index::LineIndex, protocol::Position};

#[derive(Clone, Debug)]
struct Declaration {
    name: String,
    uri: String,
    span: Span,
}

/// Resolve an identifier to its declaration among currently open source
/// documents. Same-document declarations are preferred; cross-file lookup is
/// used when a name has exactly one open declaration.
pub fn definition(
    documents: &HashMap<String, String>,
    uri: &str,
    position: Position,
) -> Option<(String, Span)> {
    let source = documents.get(uri)?;
    let (name, cursor_offset) = identifier_at(source, position)?;
    let declarations = collect_declarations(documents);
    let mut local: Vec<_> = declarations
        .iter()
        .filter(|declaration| declaration.name == name && declaration.uri == uri)
        .collect();

    if let Some(declaration) = local.iter().find(|declaration| {
        declaration.span.start <= cursor_offset && cursor_offset <= declaration.span.end
    }) {
        return Some((declaration.uri.clone(), declaration.span));
    }
    local.retain(|declaration| declaration.span.start <= cursor_offset);
    if let Some(declaration) = local
        .into_iter()
        .max_by_key(|declaration| declaration.span.start)
    {
        return Some((declaration.uri.clone(), declaration.span));
    }

    let matches: Vec<_> = declarations
        .iter()
        .filter(|declaration| declaration.name == name)
        .collect();
    (matches.len() == 1).then(|| (matches[0].uri.clone(), matches[0].span))
}

/// Find identifier occurrences in open documents. The shared lexer excludes
/// comments and string literals, avoiding the most common false references.
pub fn references(
    documents: &HashMap<String, String>,
    uri: &str,
    position: Position,
    include_declaration: bool,
) -> Vec<(String, Span)> {
    let Some(source) = documents.get(uri) else {
        return Vec::new();
    };
    let Some((name, _)) = identifier_at(source, position) else {
        return Vec::new();
    };
    if !collect_declarations(documents)
        .iter()
        .any(|declaration| declaration.name == name)
    {
        return Vec::new();
    }

    let declarations = collect_declarations(documents);
    let mut uris: Vec<_> = documents.keys().collect();
    uris.sort();
    let mut result = Vec::new();
    for document_uri in uris {
        let Some(document) = documents.get(document_uri) else {
            continue;
        };
        let Ok(tokens) = tokenize(document) else {
            continue;
        };
        for token in tokens {
            if matches!(&token.kind, TokenKind::Ident(token_name) if token_name == &name)
                && (include_declaration
                    || !declarations.iter().any(|declaration| {
                        declaration.uri == *document_uri && declaration.span == token.span
                    }))
            {
                result.push((document_uri.clone(), token.span));
            }
        }
    }
    result
}

fn identifier_at(source: &str, position: Position) -> Option<(String, usize)> {
    let index = LineIndex::new(source);
    let offset = index.to_offset(source, position)?;
    tokenize(source)
        .ok()?
        .into_iter()
        .find_map(|token| match token.kind {
            TokenKind::Ident(name) if token.span.start <= offset && offset <= token.span.end => {
                Some((name, offset))
            }
            _ => None,
        })
}

fn collect_declarations(documents: &HashMap<String, String>) -> Vec<Declaration> {
    let mut uris: Vec<_> = documents.keys().collect();
    uris.sort();
    let mut declarations = Vec::new();
    for uri in uris {
        let Some(source) = documents.get(uri) else {
            continue;
        };
        let Ok(tokens) = tokenize(source) else {
            continue;
        };
        for pair in tokens.windows(2) {
            let (TokenKind::Ident(keyword), TokenKind::Ident(name)) =
                (&pair[0].kind, &pair[1].kind)
            else {
                continue;
            };
            if matches!(
                keyword.as_str(),
                "app"
                    | "screen"
                    | "component"
                    | "service"
                    | "interface"
                    | "struct"
                    | "class"
                    | "enum"
                    | "error"
                    | "fn"
                    | "event"
                    | "prop"
                    | "property"
                    | "state"
                    | "let"
                    | "var"
                    | "const"
            ) {
                declarations.push(Declaration {
                    name: name.clone(),
                    uri: uri.clone(),
                    span: pair[1].span,
                });
            }
        }
    }
    declarations
}

#[cfg(test)]
mod tests {
    use super::{definition, references};
    use crate::protocol::Position;
    use std::collections::HashMap;

    #[test]
    fn resolves_local_declarations_and_cross_file_component_references() {
        let documents = HashMap::from([
            (
                "file:///app.nx".to_owned(),
                "import \"Row.nx\"\napp Tasks { body { TodoRow() } }\n".to_owned(),
            ),
            (
                "file:///Row.nx".to_owned(),
                "component TodoRow() { body { Text(\"TodoRow\") } }\n".to_owned(),
            ),
        ]);
        let target = definition(
            &documents,
            "file:///app.nx",
            Position {
                line: 1,
                character: 25,
            },
        )
        .expect("component declaration resolves across open files");
        assert_eq!(target.0, "file:///Row.nx");
        let found = references(
            &documents,
            "file:///app.nx",
            Position {
                line: 1,
                character: 25,
            },
            false,
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "file:///app.nx");
    }

    #[test]
    fn comments_and_string_literals_are_not_references() {
        let documents = HashMap::from([(
            "file:///app.nx".to_owned(),
            "app Tasks { state count: Int32 = 0\n body { Text(count) } }\n// count\nText(\"count\")\n".to_owned(),
        )]);
        let found = references(
            &documents,
            "file:///app.nx",
            Position {
                line: 1,
                character: 15,
            },
            true,
        );
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn resolves_plugin_contract_symbols_from_open_nxid_documents() {
        let documents = HashMap::from([
            (
                "file:///app.nx".to_owned(),
                "plugin \"plugins/sqlite\" as SQLite\napp Tasks { body { Button(\"Load\") { SQLite.Database.query(\"select name from tasks\") } } }\n".to_owned(),
            ),
            (
                "file:///plugins/sqlite/native.nxid".to_owned(),
                "native class Database { init(path: String)\n fn query(sql: String) -> Array<String>\n fn dispose() }\n".to_owned(),
            ),
        ]);
        let target = definition(
            &documents,
            "file:///app.nx",
            Position {
                line: 1,
                character: 52,
            },
        )
        .expect("method declaration should resolve in the open plugin contract");
        assert_eq!(target.0, "file:///plugins/sqlite/native.nxid");

        let found = references(
            &documents,
            "file:///plugins/sqlite/native.nxid",
            Position {
                line: 1,
                character: 4,
            },
            true,
        );
        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|(uri, _)| uri == "file:///app.nx"));
    }
}
