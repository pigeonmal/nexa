use nexa_syntax::{Token, TokenKind, ast, catalog, tokenize};
use serde_json::{Value, json};

use crate::{line_index::LineIndex, protocol::Position};

/// Build signature help for a built-in component or a user function declared
/// in an open document.
pub fn signature_help(source: &str, position: Position) -> Option<Value> {
    let tokens = tokenize(source).ok()?;
    let cursor = LineIndex::new(source).to_offset(source, position)?;
    let mut open_calls = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.span.start > cursor {
            break;
        }
        match token.kind {
            TokenKind::LParen => open_calls.push(index),
            TokenKind::RParen => {
                open_calls.pop();
            }
            _ => {}
        }
    }
    let open_index = *open_calls.last()?;
    let callee = tokens.get(open_index.checked_sub(1)?)?;
    let TokenKind::Ident(name) = &callee.kind else {
        return None;
    };
    let active_parameter = active_parameter(&tokens, open_index, cursor);
    let signature = component_signature(name).or_else(|| function_signature(source, name))?;
    let parameter_count = signature["parameters"].as_array().map_or(0, Vec::len);
    let active_parameter = active_parameter.min(parameter_count.saturating_sub(1));
    Some(json!({
        "signatures": [signature],
        "activeSignature": 0,
        "activeParameter": active_parameter
    }))
}

fn active_parameter(tokens: &[Token], open_index: usize, cursor: usize) -> usize {
    let mut nested = 0usize;
    let mut commas = 0usize;
    for token in tokens.iter().skip(open_index + 1) {
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => nested += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                nested = nested.saturating_sub(1)
            }
            TokenKind::Comma if nested == 0 => commas += 1,
            _ => {}
        }
    }
    commas
}

fn component_signature(name: &str) -> Option<Value> {
    let component = catalog::component(name)?;
    let schema = catalog::component_schema(name)?;
    let mut parameters = Vec::new();
    let mut labels = Vec::new();
    match schema.positional {
        catalog::PositionalModel::None => {}
        catalog::PositionalModel::Single => {
            labels.push("value".to_owned());
            parameters.push(json!({ "label": "value" }));
        }
        catalog::PositionalModel::ListSource => {
            labels.push("source".to_owned());
            parameters.push(json!({ "label": "source" }));
        }
    }
    for argument in schema.arguments {
        let label = if argument.required {
            argument.name.to_owned()
        } else {
            format!("{}?", argument.name)
        };
        labels.push(label.clone());
        parameters.push(json!({ "label": label }));
    }
    Some(json!({
        "label": format!("{}({})", name, labels.join(", ")),
        "documentation": component.summary,
        "parameters": parameters
    }))
}

fn function_signature(source: &str, name: &str) -> Option<Value> {
    let program = nexa_syntax::parse_program(source).ok()?;
    let mut functions = program.functions.iter().collect::<Vec<_>>();
    if let Some(app) = &program.app {
        functions.extend(app.functions.iter());
    }
    let mut matching = functions
        .into_iter()
        .filter(|function| function.name == name);
    let function = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let parameters = function
        .parameters
        .iter()
        .map(|parameter| {
            let label = format!("{}: {}", parameter.name, type_label(&parameter.ty));
            json!({ "label": label })
        })
        .collect::<Vec<_>>();
    let signature_parameters = function
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, type_label(&parameter.ty)))
        .collect::<Vec<_>>();
    Some(json!({
        "label": format!(
            "{}fn {}({}) -> {}",
            if function.is_async { "async " } else { "" },
            function.name,
            signature_parameters.join(", "),
            type_label(&function.return_type),
        ),
        "documentation": "User-defined Nexa function",
        "parameters": parameters
    }))
}

fn type_label(ty: &ast::TypeSyntax) -> String {
    match ty {
        ast::TypeSyntax::Named(name, _) => name.clone(),
        ast::TypeSyntax::Generic(name, arguments, _) => format!(
            "{}<{}>",
            name,
            arguments
                .iter()
                .map(type_label)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ast::TypeSyntax::Optional(inner, _) => format!("{}?", type_label(inner)),
    }
}

#[cfg(test)]
mod tests {
    use super::signature_help;
    use crate::protocol::Position;

    #[test]
    fn resolves_user_function_parameters_and_nested_argument_index() {
        let source = "fn loadUser(id: String, includePrivate: Bool) -> String { return id }\napp A { body { Text(loadUser(\"u1\", true)) } }\n";
        let position = Position {
            line: 1,
            character: 37,
        };
        let help = signature_help(source, position).expect("signature help inside loadUser call");
        assert!(
            help["signatures"][0]["label"]
                .as_str()
                .expect("signature label")
                .contains("loadUser(id: String, includePrivate: Bool) -> String")
        );
        assert_eq!(help["activeParameter"], 1);
    }

    #[test]
    fn uses_catalog_signatures_for_components() {
        let source = "app A { body { Text(\"Hello\", key: \"greeting\") } }\n";
        let help = signature_help(
            source,
            Position {
                line: 0,
                character: 36,
            },
        )
        .expect("component signature");
        assert!(
            help["signatures"][0]["label"]
                .as_str()
                .expect("signature label")
                .starts_with("Text(value")
        );
    }
}
