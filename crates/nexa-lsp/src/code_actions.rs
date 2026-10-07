use std::collections::{HashMap, HashSet};

use nexa_syntax::ast::{ComponentDecl, TypeSyntax};
use serde_json::{Value, json};

use crate::{line_index::LineIndex, protocol::Position};

/// Return safe quick fixes for diagnostics in the LSP request context.
pub fn code_actions(source: &str, uri: &str, context: &Value) -> Vec<Value> {
    code_actions_with_documents(source, uri, context, std::iter::empty())
}

/// Like [`code_actions`], with other open documents available for component
/// declarations used by the current source file.
pub fn code_actions_with_documents<'a>(
    source: &'a str,
    uri: &str,
    context: &Value,
    other_documents: impl IntoIterator<Item = &'a str>,
) -> Vec<Value> {
    let Some(diagnostics) = context.get("diagnostics").and_then(Value::as_array) else {
        return Vec::new();
    };

    let documents = std::iter::once(source)
        .chain(other_documents)
        .collect::<Vec<_>>();
    let mut components = HashMap::<String, ComponentDecl>::new();
    let mut enum_cases = HashMap::<String, String>::new();
    for document in documents {
        let Ok(program) = nexa_syntax::parse_program(document) else {
            continue;
        };
        for component in program.components {
            components
                .entry(component.name.clone())
                .or_insert(component);
        }
        for declaration in program.enums {
            if let Some(case) = declaration.cases.first() {
                enum_cases
                    .entry(declaration.name)
                    .or_insert_with(|| case.name.clone());
            }
        }
    }

    diagnostics
        .iter()
        .flat_map(|diagnostic| {
            let Some(message) = diagnostic.get("message").and_then(Value::as_str) else {
                return Vec::new();
            };
            if message.contains("may throw; wrap the call in a `try { ... } catch { ... }` action")
            {
                wrap_throwing_statement(source, uri, diagnostic)
                    .into_iter()
                    .collect()
            } else {
                add_missing_component_arguments(source, uri, diagnostic, &components, &enum_cases)
                    .into_iter()
                    .collect()
            }
        })
        .collect()
}

fn wrap_throwing_statement(source: &str, uri: &str, diagnostic: &Value) -> Option<Value> {
    let start: Position =
        serde_json::from_value(diagnostic.get("range")?.get("start")?.clone()).ok()?;
    let index = LineIndex::new(source);
    let offset = index.to_offset(source, start)?;
    let line_start = source[..offset]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |newline| line_start + newline);
    let line = source.get(line_start..line_end)?.trim_end_matches('\r');
    let indentation = line
        .chars()
        .take_while(|character| matches!(character, ' ' | '\t'))
        .collect::<String>();
    let statement = line.trim();
    if statement.is_empty()
        || statement.contains('{')
        || statement.contains('}')
        || !(statement.starts_with("await ")
            || statement.starts_with("return ")
            || statement.starts_with("let ")
            || statement.starts_with("var ")
            || statement.contains(" = ")
            || statement.contains('.'))
    {
        return None;
    }

    let replacement = format!(
        "{indentation}try {{\n{indentation}    {statement}\n{indentation}}} catch {{\n{indentation}    Log.error(message: \"Operation failed\")\n{indentation}}}"
    );
    Some(json!({
        "title": "Wrap in try/catch",
        "kind": "quickfix",
        "isPreferred": false,
        "diagnostics": [diagnostic],
        "edit": {
            "changes": {
            (uri): [{
                    "range": {
                        "start": { "line": start.line, "character": 0 },
                        "end": index.to_position(source, line_end)
                    },
                    "newText": replacement
                }]
            }
        }
    }))
}

fn add_missing_component_arguments(
    source: &str,
    uri: &str,
    diagnostic: &Value,
    components: &HashMap<String, ComponentDecl>,
    enum_cases: &HashMap<String, String>,
) -> Option<Value> {
    let message = diagnostic.get("message")?.as_str()?;
    let (name, problem) = message.strip_prefix("component `")?.split_once('`')?;
    if !(problem.contains("requires parameter")
        || problem.contains("missing argument")
        || problem.contains("expects "))
    {
        return None;
    }
    let component = components.get(name)?;
    let start: Position =
        serde_json::from_value(diagnostic.get("range")?.get("start")?.clone()).ok()?;
    let index = LineIndex::new(source);
    let offset = index.to_offset(source, start)?;
    let (open, close) = invocation_parentheses(source, offset, name)?;
    let arguments = source.get(open + 1..close)?;
    if arguments.contains(['\n', '\r']) || arguments.contains("//") {
        return None;
    }
    let provided = named_argument_names(arguments)?;
    let missing = component
        .parameters
        .iter()
        .filter(|parameter| !provided.contains(&parameter.name))
        .map(|parameter| {
            placeholder_for_type(&parameter.ty, enum_cases)
                .map(|placeholder| format!("{}: {placeholder}", parameter.name))
        })
        .collect::<Option<Vec<_>>>()?;
    if missing.is_empty() {
        return None;
    }

    let trimmed = arguments.trim_end();
    let insertion_offset = open + 1 + trimmed.len();
    let separator = if trimmed.is_empty() {
        ""
    } else if trimmed.ends_with(',') {
        " "
    } else {
        ", "
    };
    let edit_position = index.to_position(source, insertion_offset);
    let inserted = format!("{separator}{}", missing.join(", "));
    let updated = format!(
        "{}{}{}",
        &source[..insertion_offset],
        inserted,
        &source[insertion_offset..]
    );
    nexa_syntax::parse_program(&updated).ok()?;

    Some(json!({
        "title": "Add missing required component arguments",
        "kind": "quickfix",
        "isPreferred": false,
        "diagnostics": [diagnostic],
        "edit": {
            "changes": {
                (uri): [{
                    "range": {
                        "start": { "line": edit_position.line, "character": edit_position.character },
                        "end": { "line": edit_position.line, "character": edit_position.character }
                    },
                    "newText": inserted
                }]
            }
        }
    }))
}

fn invocation_parentheses(source: &str, offset: usize, name: &str) -> Option<(usize, usize)> {
    let relative_start = source.get(offset..)?.find(name)?;
    let start = offset + relative_start;
    if start > 0
        && source[..start]
            .chars()
            .next_back()
            .is_some_and(is_identifier_character)
    {
        return None;
    }
    let mut open = start + name.len();
    while source
        .as_bytes()
        .get(open)
        .is_some_and(u8::is_ascii_whitespace)
    {
        open += 1;
    }
    if source.as_bytes().get(open) != Some(&b'(') {
        return None;
    }

    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = open;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            cursor = bytes[cursor..]
                .iter()
                .position(|candidate| *candidate == b'\n')
                .map_or(bytes.len(), |newline| cursor + newline);
        } else if byte == b'(' {
            depth += 1;
        } else if byte == b')' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some((open, cursor));
            }
        }
        cursor += 1;
    }
    None
}

fn named_argument_names(arguments: &str) -> Option<HashSet<String>> {
    let mut names = HashSet::new();
    for argument in split_top_level(arguments, b',')? {
        let argument = argument.trim();
        if argument.is_empty() {
            continue;
        }
        let colon = top_level_colon(argument)?;
        let name = argument[..colon].trim();
        if name.is_empty() || !name.chars().all(is_identifier_character) {
            return None;
        }
        names.insert(name.to_owned());
    }
    Some(names)
}

fn split_top_level(source: &str, delimiter: u8) -> Option<Vec<&str>> {
    let bytes = source.as_bytes();
    let mut sections = Vec::new();
    let mut start = 0usize;
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'(' => round += 1,
            b')' => round = round.checked_sub(1)?,
            b'[' => square += 1,
            b']' => square = square.checked_sub(1)?,
            b'{' => curly += 1,
            b'}' => curly = curly.checked_sub(1)?,
            _ if byte == delimiter && round == 0 && square == 0 && curly == 0 => {
                sections.push(&source[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if in_string || round != 0 || square != 0 || curly != 0 {
        return None;
    }
    sections.push(&source[start..]);
    Some(sections)
}

fn top_level_colon(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'(' => round += 1,
            b')' => round = round.checked_sub(1)?,
            b'[' => square += 1,
            b']' => square = square.checked_sub(1)?,
            b'{' => curly += 1,
            b'}' => curly = curly.checked_sub(1)?,
            b':' if round == 0 && square == 0 && curly == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn placeholder_for_type(ty: &TypeSyntax, enum_cases: &HashMap<String, String>) -> Option<String> {
    match ty {
        TypeSyntax::Optional(_, _) => Some("null".to_owned()),
        TypeSyntax::Named(name, _) => match name.as_str() {
            "String" => Some("\"\"".to_owned()),
            "Bool" => Some("false".to_owned()),
            "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64" => {
                Some("0".to_owned())
            }
            "Float32" | "Float64" => Some("0.0".to_owned()),
            "Range" => Some("0..<0".to_owned()),
            "Regex" => Some("Regex(pattern: \".*\")".to_owned()),
            _ => enum_cases.get(name).map(|case| format!("{name}.{case}")),
        },
        TypeSyntax::Generic(name, _, _) if name == "Array" || name == "Set" => {
            Some("[]".to_owned())
        }
        TypeSyntax::Generic(name, _, _) if name == "Map" => Some("[:]".to_owned()),
        TypeSyntax::Generic(name, arguments, _) if name == "Pair" && arguments.len() == 2 => {
            Some(format!(
                "Pair({}, {})",
                placeholder_for_type(&arguments[0], enum_cases)?,
                placeholder_for_type(&arguments[1], enum_cases)?
            ))
        }
        TypeSyntax::Generic(name, arguments, _) if name == "Triple" && arguments.len() == 3 => {
            Some(format!(
                "Triple({}, {}, {})",
                placeholder_for_type(&arguments[0], enum_cases)?,
                placeholder_for_type(&arguments[1], enum_cases)?,
                placeholder_for_type(&arguments[2], enum_cases)?
            ))
        }
        _ => None,
    }
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::{code_actions, code_actions_with_documents};
    use crate::line_index::LineIndex;
    use serde_json::json;

    #[test]
    fn wraps_an_unhandled_action_statement_in_flat_catch_syntax() {
        let source = "app Demo {\n    body {\n        Button(\"Save\") {\n            await storage.save()\n        }\n    }\n}\n";
        let actions = code_actions(
            source,
            "file:///app.nx",
            &json!({
                "diagnostics": [{
                    "message": "plugin API `Storage.save` may throw; wrap the call in a `try { ... } catch { ... }` action to handle its failure",
                    "range": {
                        "start": { "line": 3, "character": 18 },
                        "end": { "line": 3, "character": 30 }
                    }
                }]
            }),
        );
        assert_eq!(actions.len(), 1);
        let new_text = actions[0]["edit"]["changes"]["file:///app.nx"][0]["newText"]
            .as_str()
            .expect("quick fix edit");
        assert!(new_text.contains("            try {\n                await storage.save()"));
        assert!(new_text.contains("            } catch {\n                Log.error"));
        let statement_start = source
            .find("            await storage.save()")
            .expect("throwing action line");
        let line_start = source[..statement_start]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let line_end = source[statement_start..]
            .find('\n')
            .map_or(source.len(), |newline| statement_start + newline);
        let rewritten = format!(
            "{}{}{}",
            &source[..line_start],
            new_text,
            &source[line_end..]
        );
        nexa_syntax::parse_program(&rewritten).expect("quick fix emits valid catch syntax");
    }

    #[test]
    fn skips_multiline_or_non_action_source_lines() {
        let source = "app Demo { body { Button(\"Save\") { await storage.save() } } }\n";
        let actions = code_actions(
            source,
            "file:///app.nx",
            &json!({
                "diagnostics": [{
                    "message": "plugin API `Storage.save` may throw; wrap the call in a `try { ... } catch { ... }` action to handle its failure",
                    "range": {
                        "start": { "line": 0, "character": 43 },
                        "end": { "line": 0, "character": 55 }
                    }
                }]
            }),
        );
        assert!(actions.is_empty());
    }

    #[test]
    fn adds_typed_placeholders_for_missing_cross_file_component_arguments() {
        let component_source = r#"
enum Priority { low, high }
component TaskCard(title: String, priority: Priority, completed: Bool, tags: Array<String>, metadata: Map<String, Int32>) {
    body { Text(title) }
}
"#;
        let source = "import \"TaskCard.nx\"\napp Todo { body { TaskCard(title: \"Inbox\") } }\n";
        let offset = source.find("TaskCard(title").expect("component call");
        let position = LineIndex::new(source).to_position(source, offset);
        let actions = code_actions_with_documents(
            source,
            "file:///app.nx",
            &json!({
                "diagnostics": [{
                    "message": "component `TaskCard` expects 5 named argument(s), got 1",
                    "range": { "start": position, "end": position }
                }]
            }),
            [component_source],
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0]["title"],
            "Add missing required component arguments"
        );
        let change = &actions[0]["edit"]["changes"]["file:///app.nx"][0];
        assert_eq!(
            change["newText"],
            ", priority: Priority.low, completed: false, tags: [], metadata: [:]"
        );
        let edit_position: crate::Position =
            serde_json::from_value(change["range"]["start"].clone()).expect("edit position");
        let edit_offset = LineIndex::new(source)
            .to_offset(source, edit_position)
            .expect("edit offset");
        let updated = format!(
            "{}{}{}",
            &source[..edit_offset],
            change["newText"].as_str().expect("inserted arguments"),
            &source[edit_offset..]
        );
        nexa_syntax::parse_program(&updated).expect("quick fix keeps the source parseable");
    }

    #[test]
    fn omits_component_quick_fix_when_no_safe_value_exists() {
        let source = "component Card(model: TaskModel) { body { Text(\"Task\") } }\napp Demo { body { Card() } }\n";
        let offset = source.find("Card()").expect("component call");
        let position = LineIndex::new(source).to_position(source, offset);
        let actions = code_actions(
            source,
            "file:///app.nx",
            &json!({
                "diagnostics": [{
                    "message": "component `Card` expects 1 named argument(s), got 0",
                    "range": { "start": position, "end": position }
                }]
            }),
        );
        assert!(actions.is_empty());
    }
}
