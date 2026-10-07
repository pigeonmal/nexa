use serde_json::{Value, json};

use crate::{line_index::LineIndex, protocol::Position};

/// Return safe, source-local quick fixes for diagnostics in the LSP request
/// context. Currently this covers unhandled throwing action calls.
pub fn code_actions(source: &str, uri: &str, context: &Value) -> Vec<Value> {
    let Some(diagnostics) = context.get("diagnostics").and_then(Value::as_array) else {
        return Vec::new();
    };
    diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let message = diagnostic.get("message")?.as_str()?;
            if !message.contains("may throw; wrap the call in a `try { ... } catch { ... }` action")
            {
                return None;
            }
            wrap_throwing_statement(source, uri, diagnostic)
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

#[cfg(test)]
mod tests {
    use super::code_actions;
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
}
