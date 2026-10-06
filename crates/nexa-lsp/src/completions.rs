//! Context-aware completion for `.nx` programs and project configuration.
//!
//! The compiler remains authoritative for validity. This module only indexes
//! the prefix before the caret, recognizes the active syntactic region, and
//! offers names from Nexa's language/component catalog and visible declarations.

use std::collections::HashSet;

use nexa_syntax::catalog;

use crate::line_index::LineIndex;
use crate::protocol::{CompletionItem, CompletionItemKind, Position, Range, TextEdit};

const SNIPPET_FORMAT: u8 = 2;

#[derive(Clone, Debug)]
struct Word {
    text: String,
    start: usize,
    end: usize,
    string: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Region {
    Root,
    AppMembers,
    Nodes,
    Actions,
    FunctionBody,
    Type,
    Expression,
    Config,
}

#[derive(Clone, Debug)]
struct CursorFacts {
    prefix: String,
    prefix_start: usize,
    cursor: usize,
    after_dot: bool,
    in_string: bool,
    words: Vec<Word>,
    open: Vec<(char, usize)>,
}

/// Compatibility entry point used by existing integrations. New callers
/// should pass the URI so config files receive config-aware suggestions.
pub fn get_completions(source: &str, position: Position) -> Vec<CompletionItem> {
    get_document_completions(source, position, false)
}

/// Computes completions for one document version and cursor position.
pub fn get_document_completions(
    source: &str,
    position: Position,
    is_config: bool,
) -> Vec<CompletionItem> {
    let index = LineIndex::new(source);
    let Some(cursor) = index.to_offset(source, position) else {
        return Vec::new();
    };
    let facts = cursor_facts(source, cursor);
    let region = if is_config {
        Region::Config
    } else {
        region_at(&facts)
    };
    let mut items = Vec::new();

    if facts.in_string {
        if region == Region::Config {
            add_config_value_completions(source, &facts, &mut items);
        }
        return finish(items, source, &facts, &index);
    }

    if facts.after_dot {
        add_member_completions(source, &facts, &mut items);
        return finish(items, source, &facts, &index);
    }

    if region == Region::Config {
        add_config_completions(source, &facts, &mut items);
        return finish(items, source, &facts, &index);
    }

    if let Some((call, open_index)) = active_call(&facts) {
        if let Some(parameter) = parameter_before_cursor(&facts, open_index) {
            add_enum_values(&parameter, source, &facts, &mut items);
            add_expression_values(source, &facts, &mut items);
        } else {
            add_call_argument_completions(call, &facts, open_index, &mut items);
            add_expression_values(source, &facts, &mut items);
        }
        return finish(items, source, &facts, &index);
    }

    if let Some((_, previous)) = facts.words.iter().rev().nth(1).zip(facts.words.last())
        && previous.text == ":"
    {
        add_enum_values(
            &facts.words[facts.words.len().saturating_sub(2)].text,
            source,
            &facts,
            &mut items,
        );
        add_expression_values(source, &facts, &mut items);
        return finish(items, source, &facts, &index);
    }

    match region_at(&facts) {
        Region::Root => add_root_completions(&facts, &mut items),
        Region::AppMembers => add_app_member_completions(&facts, &mut items),
        Region::Nodes => add_node_completions(source, &facts, &mut items),
        Region::Actions | Region::FunctionBody => {
            add_action_completions(source, &facts, &mut items)
        }
        Region::Type => add_type_completions(&facts, &mut items),
        Region::Expression => add_expression_values(source, &facts, &mut items),
        Region::Config => {}
    }
    finish(items, source, &facts, &index)
}

fn cursor_facts(source: &str, cursor: usize) -> CursorFacts {
    let before = &source[..cursor.min(source.len())];
    let mut prefix_start = before.len();
    for (offset, character) in before.char_indices().rev() {
        if is_identifier_continue(character) {
            prefix_start = offset;
        } else {
            break;
        }
    }
    let prefix = before[prefix_start..].to_owned();
    let before_prefix = before[..prefix_start].trim_end();
    let after_dot = before_prefix.ends_with('.');
    let (words, in_string) = scan_words(before);
    let mut open = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if word.string {
            continue;
        }
        match word.text.as_str() {
            "(" | "{" | "[" => open.push((word.text.chars().next().unwrap_or(' '), index)),
            ")" | "}" | "]" => {
                let matching = match word.text.as_str() {
                    ")" => '(',
                    "}" => '{',
                    "]" => '[',
                    _ => ' ',
                };
                if let Some(position) = open.iter().rposition(|(kind, _)| *kind == matching) {
                    open.truncate(position);
                }
            }
            _ => {}
        }
    }
    CursorFacts {
        prefix,
        prefix_start,
        cursor,
        after_dot,
        in_string,
        words,
        open,
    }
}

/// A small tolerant scanner: unlike the compiler lexer, it deliberately
/// accepts incomplete strings, calls, and blocks while the user is typing.
fn scan_words(source: &str) -> (Vec<Word>, bool) {
    let mut words = Vec::new();
    let mut cursor = 0;
    let mut in_string = false;
    while cursor < source.len() {
        let rest = &source[cursor..];
        let character = match rest.chars().next() {
            Some(character) => character,
            None => break,
        };
        if character.is_whitespace() {
            cursor += character.len_utf8();
            continue;
        }
        if rest.starts_with("//") {
            cursor += 2;
            while cursor < source.len() && !source[cursor..].starts_with('\n') {
                cursor += source[cursor..].chars().next().map_or(1, char::len_utf8);
            }
            continue;
        }
        let start = cursor;
        if character == '"' {
            cursor += 1;
            let content_start = cursor;
            let mut escaped = false;
            let mut closed = false;
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap_or(' ');
                cursor += next.len_utf8();
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == '"' {
                    closed = true;
                    break;
                }
            }
            if !closed {
                in_string = true;
            }
            words.push(Word {
                text: source[content_start..if closed { cursor - 1 } else { cursor }].to_owned(),
                start,
                end: cursor,
                string: true,
            });
            continue;
        }
        if is_identifier_start(character) {
            cursor += character.len_utf8();
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap_or(' ');
                if !is_identifier_continue(next) {
                    break;
                }
                cursor += next.len_utf8();
            }
        } else if character.is_ascii_digit() {
            cursor += 1;
            while cursor < source.len()
                && (source.as_bytes()[cursor].is_ascii_digit() || source.as_bytes()[cursor] == b'.')
            {
                cursor += 1;
            }
        } else if cursor + 1 < source.len()
            && matches!(
                &source[cursor..cursor + 2],
                "??" | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "+=" | "-=" | ".."
            )
        {
            cursor += 2;
        } else {
            cursor += character.len_utf8();
        }
        words.push(Word {
            text: source[start..cursor].to_owned(),
            start,
            end: cursor,
            string: false,
        });
    }
    (words, in_string)
}

fn region_at(facts: &CursorFacts) -> Region {
    if let Some(colon) = facts.words.iter().rposition(|word| word.text == ":") {
        let tail = facts.words.get(colon + 1..).unwrap_or_default();
        if !tail
            .iter()
            .any(|word| matches!(word.text.as_str(), "=" | "," | ")" | "{"))
            && facts
                .words
                .get(colon.wrapping_sub(1))
                .is_some_and(|word| is_identifier(&word.text))
        {
            return Region::Type;
        }
    }
    let Some((_, open_index)) = facts.open.iter().rev().find(|(kind, _)| *kind == '{') else {
        return Region::Root;
    };
    let owner = previous_identifier(&facts.words, *open_index);
    match owner.as_deref() {
        Some("body") => Region::Nodes,
        Some("app") => Region::AppMembers,
        Some("fn") => Region::FunctionBody,
        Some("if" | "else" | "for" | "while" | "catch" | "OnAppear" | "OnDisappear") => {
            Region::Actions
        }
        Some(name) if catalog::component_schema(name).is_some() => {
            let schema = catalog::component_schema(name);
            if matches!(
                schema.map(|schema| schema.children),
                Some(
                    catalog::ChildModel::Nodes
                        | catalog::ChildModel::Tabs
                        | catalog::ChildModel::SplitPanes
                        | catalog::ChildModel::ListRows
                )
            ) {
                Region::Nodes
            } else {
                Region::Actions
            }
        }
        _ => {
            // Body is generally separated from its declaration, for example
            // `component Row(...) { body { ... } }`; inspect enclosing owners.
            if facts.open.iter().any(|(_, index)| {
                previous_identifier(&facts.words, *index).as_deref() == Some("body")
            }) {
                Region::Nodes
            } else if facts.open.iter().any(|(_, index)| {
                previous_identifier(&facts.words, *index).as_deref() == Some("app")
            }) {
                Region::AppMembers
            } else {
                Region::Expression
            }
        }
    }
}

fn previous_identifier(words: &[Word], before: usize) -> Option<String> {
    words[..before]
        .iter()
        .rev()
        .find(|word| !word.string && is_identifier(&word.text))
        .map(|word| word.text.clone())
}

fn active_call(facts: &CursorFacts) -> Option<(String, usize)> {
    let (_, index) = facts.open.iter().rev().find(|(kind, _)| *kind == '(')?;
    let name = previous_identifier(&facts.words, *index)?;
    Some((name, *index))
}

fn parameter_before_cursor(facts: &CursorFacts, open_index: usize) -> Option<String> {
    let active = facts.words.get(open_index + 1..)?;
    let depth = active
        .iter()
        .rev()
        .take_while(|word| !matches!(word.text.as_str(), ","))
        .collect::<Vec<_>>();
    for pair in depth.windows(2) {
        if pair[1].text == ":" && is_identifier(&pair[0].text) {
            return Some(pair[0].text.clone());
        }
    }
    None
}

fn used_arguments(facts: &CursorFacts, open_index: usize) -> HashSet<String> {
    let mut used = HashSet::new();
    let mut depth = 0i32;
    let words = facts.words.get(open_index + 1..).unwrap_or_default();
    for pair in words.windows(2) {
        match pair[0].text.as_str() {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => depth -= 1,
            _ => {}
        }
        if depth == 0 && pair[1].text == ":" && is_identifier(&pair[0].text) {
            used.insert(pair[0].text.clone());
        }
    }
    used
}

fn add_call_argument_completions(
    call: String,
    facts: &CursorFacts,
    open_index: usize,
    items: &mut Vec<CompletionItem>,
) {
    let used = used_arguments(facts, open_index);
    let names = if let Some(schema) = catalog::component_schema(&call) {
        schema
            .arguments
            .iter()
            .map(|argument| argument.name)
            .collect::<Vec<_>>()
    } else {
        api_parameters(&call)
    };
    for name in names {
        if used.contains(name) || !name.starts_with(&facts.prefix) {
            continue;
        }
        let required = catalog::component_schema(&call)
            .and_then(|schema| {
                schema
                    .arguments
                    .iter()
                    .find(|argument| argument.name == name)
            })
            .is_some_and(|argument| argument.required);
        let snippet = format!("{name}: ${{1}}",);
        push_item(
            items,
            name.to_owned(),
            Some(CompletionItemKind::Field),
            Some(if required {
                format!("Required {call} parameter")
            } else {
                format!("{call} parameter")
            }),
            None,
            &snippet,
        );
    }
}

fn api_parameters(call: &str) -> Vec<&'static str> {
    match call {
        "Locale.displayName" | "displayName" => vec!["languageCode"],
        "Number.formatCurrency" | "formatCurrency" => vec!["amount", "currencyCode"],
        "Storage.getString" | "getString" | "Storage.delete" | "delete" => vec!["key"],
        "Storage.setString" | "setString" => vec!["key", "value"],
        "Time.elapsed" | "elapsed" => vec!["since"],
        "Time.sleep" | "sleep" => vec!["milliseconds"],
        "Time.iso8601"
        | "iso8601"
        | "Time.startOfDay"
        | "startOfDay"
        | "Time.localizedDate"
        | "localizedDate"
        | "Time.localizedTime"
        | "localizedTime"
        | "Time.localizedDateTime"
        | "localizedDateTime" => vec!["timestamp"],
        "Time.iso8601ToMillis" | "iso8601ToMillis" => vec!["text"],
        "Path.join" | "join" => vec!["path", "component"],
        "File.exists" | "exists" | "File.readText" | "readText" | "File.delete" => vec!["path"],
        "File.writeText" | "writeText" => vec!["path", "contents"],
        "Haptics.impact" | "impact" => vec!["style"],
        "Haptics.notification" | "notification" => vec!["kind"],
        "Log.info" | "Log.warning" | "Log.error" | "info" | "warning" | "error" => vec!["message"],
        _ => Vec::new(),
    }
}

fn add_member_completions(source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    let Some(receiver) = facts
        .words
        .iter()
        .rev()
        .find(|word| !word.string && word.end <= facts.prefix_start && is_identifier(&word.text))
        .map(|word| word.text.as_str())
    else {
        add_modifiers(&facts.prefix, items);
        return;
    };
    let members: &[(&str, &str)] = match receiver {
        "Locale" => &[
            ("currentLanguageCode", "Current device language code"),
            (
                "preferredLanguageCodes",
                "Ordered device language preferences",
            ),
            ("displayName", "Localized language name"),
            ("localized", "Localize source text for native API values"),
        ],
        "Number" => &[(
            "formatCurrency",
            "Format an amount using the current locale",
        )],
        "Storage" => &[
            ("getString", "Read a persisted string"),
            ("setString", "Write a persisted string"),
            ("delete", "Delete a persisted key"),
            ("clear", "Clear app storage"),
        ],
        "Time" => &[
            ("now", "Current epoch milliseconds"),
            ("monotonic", "Monotonic clock reading"),
            ("elapsed", "Elapsed monotonic time"),
            ("sleep", "Suspend for a duration"),
            ("iso8601", "Format an ISO 8601 date"),
            ("iso8601ToMillis", "Parse an ISO 8601 date"),
            ("startOfDay", "Start of the local calendar day"),
            ("addCalendarDays", "Add local calendar days"),
            (
                "localizedDate",
                "Format a timestamp as a native-locale date",
            ),
            (
                "localizedTime",
                "Format a timestamp as a native-locale time",
            ),
            (
                "localizedDateTime",
                "Format a timestamp as a native-locale date and time",
            ),
        ],
        "File" => &[
            ("exists", "Check whether a file exists"),
            ("readText", "Read a UTF-8 file"),
            ("writeText", "Write a UTF-8 file"),
            ("delete", "Delete a file"),
        ],
        "Path" => &[
            ("documents", "Documents directory"),
            ("caches", "Caches directory"),
            ("temporary", "Temporary directory"),
            ("appSupport", "Application support directory"),
            ("join", "Join a path component"),
        ],
        "Screen" => &[("lockOrientation", "Set the supported screen orientation")],
        "Crypto" => &[
            ("sha256", "SHA-256 digest"),
            ("sha512", "SHA-512 digest"),
            ("hmacSha256", "HMAC-SHA256 digest"),
            ("randomBytes", "Generate random bytes"),
        ],
        "Keyboard" => &[("dismiss", "Dismiss the software keyboard")],
        "Clipboard" => &[
            ("setText", "Write clipboard text"),
            ("getText", "Read clipboard text"),
            ("hasText", "Check clipboard content"),
        ],
        "Network" => &[
            ("fetch", "Send an HTTP request"),
            ("download", "Download a file"),
            ("upload", "Upload a file"),
            ("isOnline", "Current network availability"),
        ],
        "Json" => &[
            ("parse", "Decode a typed JSON value"),
            ("stringify", "Encode a typed JSON value"),
        ],
        _ => &[],
    };
    for (name, description) in members {
        if name.starts_with(&facts.prefix) {
            push_item(
                items,
                (*name).to_owned(),
                Some(CompletionItemKind::Method),
                Some((*description).to_owned()),
                None,
                &format!("{name}(${{1}})"),
            );
        }
    }
    if members.is_empty() {
        if let Some(schema) = catalog::component_schema(receiver) {
            for modifier in schema
                .modifiers
                .iter()
                .filter(|modifier| modifier.name.starts_with(&facts.prefix))
            {
                push_item(
                    items,
                    modifier.name.to_owned(),
                    Some(CompletionItemKind::Method),
                    Some(modifier.name.to_owned()),
                    None,
                    &format!("{}(${{1}})", modifier.name),
                );
            }
        }
        add_user_members(source, receiver, &facts.prefix, items);
    }
}

fn add_user_members(source: &str, receiver: &str, prefix: &str, items: &mut Vec<CompletionItem>) {
    let Ok(program) = nexa_syntax::parse_program(source) else {
        return;
    };
    let classes = program
        .classes
        .iter()
        .chain(program.app.iter().flat_map(|app| app.classes.iter()));
    for class in classes {
        let matches_receiver =
            class.name == receiver || source.contains(&format!("{receiver} = {}(", class.name));
        if !matches_receiver {
            continue;
        }
        for field in class.fields.iter().chain(class.static_fields.iter()) {
            if field.name.starts_with(prefix) {
                push_item(
                    items,
                    field.name.clone(),
                    Some(CompletionItemKind::Field),
                    Some(format!(
                        "{}: {}",
                        field.name,
                        field
                            .ty
                            .as_ref()
                            .map(|ty| format!("{ty:?}"))
                            .unwrap_or_default()
                    )),
                    None,
                    &field.name,
                );
            }
        }
        for method in class.methods.iter().chain(class.static_methods.iter()) {
            if method.name.starts_with(prefix) {
                let params = method
                    .parameters
                    .iter()
                    .map(|parameter| format!("{}: {:?}", parameter.name, parameter.ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                push_item(
                    items,
                    method.name.clone(),
                    Some(CompletionItemKind::Method),
                    Some(format!("{}({params})", method.name)),
                    None,
                    &format!("{}(${{1}})", method.name),
                );
            }
        }
    }
}

fn add_root_completions(facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for (label, summary, snippet) in [
        (
            "app",
            "Application entry point",
            "app ${1:MyApp} {\n    body {\n        $0\n    }\n}",
        ),
        (
            "screen",
            "Named navigation screen",
            "screen ${1:Home} {\n    body {\n        $0\n    }\n}",
        ),
        (
            "component",
            "Reusable UI component",
            "component ${1:MyComponent} {\n    body {\n        $0\n    }\n}",
        ),
        (
            "struct",
            "Value type",
            "struct ${1:MyModel} {\n    ${2:value}: ${3:String}\n}",
        ),
        (
            "class",
            "Reference type with methods",
            "class ${1:MyClass} {\n    init(${2:id}: ${3:String}) { }\n    $0\n}",
        ),
        (
            "enum",
            "Enumeration",
            "enum ${1:Status} {\n    ${2:ready}\n}",
        ),
        (
            "fn",
            "Function",
            "fn ${1:name}(${2:value}: ${3:String}) -> ${4:Void} {\n    $0\n}",
        ),
    ] {
        if label.starts_with(&facts.prefix) {
            push_item(
                items,
                label.to_owned(),
                Some(CompletionItemKind::Snippet),
                Some(summary.to_owned()),
                None,
                snippet,
            );
        }
    }
    add_type_completions(facts, items);
}

fn add_app_member_completions(facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for (label, summary, snippet) in [
        ("body", "Application UI tree", "body {\n    $0\n}"),
        (
            "state",
            "Mutable reactive state",
            "state ${1:value}: ${2:String} = ${3:\"\"}",
        ),
        (
            "let",
            "Immutable binding",
            "let ${1:value}: ${2:String} = ${3:\"\"}",
        ),
        (
            "fn",
            "App function",
            "fn ${1:name}() -> ${2:Void} {\n    $0\n}",
        ),
        (
            "screen",
            "Navigation screen",
            "screen ${1:Home} {\n    body {\n        $0\n    }\n}",
        ),
        (
            "component",
            "Reusable component",
            "component ${1:Row} {\n    body {\n        $0\n    }\n}",
        ),
        (
            "enum",
            "Enumeration",
            "enum ${1:Status} {\n    ${2:ready}\n}",
        ),
        (
            "struct",
            "Value type",
            "struct ${1:Model} {\n    ${2:value}: ${3:String}\n}",
        ),
    ] {
        if label.starts_with(&facts.prefix) {
            push_item(
                items,
                label.to_owned(),
                Some(CompletionItemKind::Snippet),
                Some(summary.to_owned()),
                None,
                snippet,
            );
        }
    }
    add_type_completions(facts, items);
}

fn add_node_completions(source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for entry in catalog::COMPONENTS
        .iter()
        .filter(|entry| entry.name.starts_with(&facts.prefix))
    {
        push_item(
            items,
            entry.name.to_owned(),
            Some(CompletionItemKind::Constructor),
            Some(entry.summary.to_owned()),
            None,
            entry.snippet,
        );
    }
    for keyword in ["if", "for", "when"] {
        if keyword.starts_with(&facts.prefix) {
            push_item(
                items,
                keyword.to_owned(),
                Some(CompletionItemKind::Keyword),
                None,
                None,
                keyword,
            );
        }
    }
    add_visible_symbols(source, facts, items);
}

fn add_action_completions(source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for keyword in [
        "if",
        "for",
        "when",
        "let",
        "return",
        "await",
        "try",
        "throw",
        "Log",
        "Locale",
        "Number",
        "Time",
        "File",
        "Path",
        "Storage",
        "SecureStorage",
        "Network",
        "Json",
        "Crypto",
        "Haptics",
        "Clipboard",
        "Keyboard",
        "Screen",
    ] {
        if keyword.starts_with(&facts.prefix) {
            push_item(
                items,
                keyword.to_owned(),
                Some(CompletionItemKind::Keyword),
                None,
                None,
                keyword,
            );
        }
    }
    add_visible_symbols(source, facts, items);
}

fn add_expression_values(source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for (name, detail) in [
        ("true", "Boolean literal"),
        ("false", "Boolean literal"),
        ("null", "Optional value"),
        ("this", "Current class instance"),
    ] {
        if name.starts_with(&facts.prefix) {
            push_item(
                items,
                name.to_owned(),
                Some(CompletionItemKind::Keyword),
                Some(detail.to_owned()),
                None,
                name,
            );
        }
    }
    for name in [
        "Locale",
        "Number",
        "Time",
        "File",
        "Path",
        "Storage",
        "SecureStorage",
        "Network",
        "Json",
        "Crypto",
        "Haptics",
        "Clipboard",
        "Keyboard",
        "Screen",
        "Ok",
        "Err",
    ] {
        if name.starts_with(&facts.prefix) {
            push_item(
                items,
                name.to_owned(),
                Some(CompletionItemKind::Module),
                Some("Nexa built-in API".to_owned()),
                None,
                name,
            );
        }
    }
    add_visible_symbols(source, facts, items);
}

fn add_visible_symbols(source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    let mut names = HashSet::new();
    for pair in facts.words.windows(2) {
        if matches!(
            pair[0].text.as_str(),
            "state" | "let" | "var" | "fn" | "screen" | "component" | "struct" | "class" | "enum"
        ) && !pair[1].string
            && is_identifier(&pair[1].text)
            && pair[1].start < facts.cursor
        {
            names.insert(pair[1].text.clone());
        }
    }
    if let Ok(program) = nexa_syntax::parse_program(source)
        && let Some(app) = &program.app
    {
        names.extend(app.states.iter().map(|state| state.name.clone()));
        names.extend(app.globals.iter().map(|binding| binding.name.clone()));
        names.extend(app.screens.iter().map(|screen| screen.name.clone()));
        names.extend(app.functions.iter().map(|function| function.name.clone()));
        names.extend(app.structs.iter().map(|structure| structure.name.clone()));
        names.extend(app.classes.iter().map(|class| class.name.clone()));
        names.extend(app.enums.iter().map(|enumeration| enumeration.name.clone()));
    }
    for name in names {
        if name.starts_with(&facts.prefix) {
            push_item(
                items,
                name.clone(),
                Some(CompletionItemKind::Variable),
                Some("Visible Nexa declaration".to_owned()),
                None,
                &name,
            );
        }
    }
}

fn add_type_completions(facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    for entry in catalog::TYPES
        .iter()
        .filter(|entry| entry.name.starts_with(&facts.prefix))
    {
        push_item(
            items,
            entry.name.to_owned(),
            Some(CompletionItemKind::Class),
            Some(entry.summary.to_owned()),
            None,
            entry.name,
        );
    }
}

fn add_enum_values(
    parameter: &str,
    source: &str,
    facts: &CursorFacts,
    items: &mut Vec<CompletionItem>,
) {
    let values: &[&str] = match parameter {
        "alignment" => &[
            "Leading",
            "Center",
            "Trailing",
            "Top",
            "Bottom",
            "TopLeading",
            "TopTrailing",
            "BottomLeading",
            "BottomTrailing",
        ],
        "fontWeight" => &["Normal", "Medium", "Semibold", "Bold"],
        "style" => &[
            "Plain",
            "Bordered",
            "BorderedProminent",
            "Borderless",
            "Automatic",
            "Destructive",
        ],
        "size" => &["Small", "Medium", "Large"],
        "shape" => &["Capsule", "Circle", "RoundedRectangle"],
        "placement" => &["Leading", "Trailing", "Principal", "BottomBar"],
        "direction" => &[
            "TopToBottom",
            "BottomToTop",
            "LeadingToTrailing",
            "TrailingToLeading",
        ],
        "role" => &[
            "Image",
            "Button",
            "Link",
            "Header",
            "SearchField",
            "TextField",
        ],
        "mode" if source.contains("Screen.lockOrientation") => &["Portrait", "Landscape", "All"],
        _ => &[],
    };
    for value in values
        .iter()
        .filter(|value| value.starts_with(&facts.prefix))
    {
        push_item(
            items,
            (*value).to_owned(),
            Some(CompletionItemKind::EnumMember),
            Some(format!("Value for `{parameter}`")),
            None,
            value,
        );
    }
}

fn add_config_completions(_source: &str, facts: &CursorFacts, items: &mut Vec<CompletionItem>) {
    let names = block_owners(&facts.words, &facts.open);
    let current = names.last().map(String::as_str).unwrap_or("");
    let (fields, summary): (&[&str], &str) = match current {
        "config" => (
            &[
                "app",
                "flavors",
                "dependencies",
                "assets",
                "permissions",
                "plugins",
                "ios",
                "android",
            ],
            "Project config block",
        ),
        "app" => (
            &[
                "displayName",
                "version",
                "buildNumber",
                "stagingSuffix",
                "deepLinks",
                "orientation",
            ],
            "App config field",
        ),
        "ios" => (
            &["minVersion", "bundleIdentifier", "icon", "arch"],
            "iOS config field",
        ),
        "android" => (
            &[
                "minSdk",
                "targetSdk",
                "applicationId",
                "icon",
                "arch",
                "cronet",
            ],
            "Android config field",
        ),
        "cronet" => (&["provider", "diskCacheSizeMb"], "Android Cronet field"),
        "assets" => (&["icon", "splash"], "Assets config field"),
        _ if names.iter().any(|name| name == "dependencies") => (
            &["id", "path", "git", "rev", "package"],
            "Plugin dependency field",
        ),
        _ if names.iter().any(|name| name == "plugins") => (&[], "Plugin option"),
        _ if names.iter().any(|name| name == "flavors") => (&["suffix"], "Build flavor field"),
        _ => (&[], "Configuration field"),
    };
    if current == "permissions" {
        for permission in [
            "camera",
            "microphone",
            "photos",
            "location",
            "notifications",
            "bluetooth",
            "contacts",
            "calendar",
            "speechRecognition",
        ] {
            if permission.starts_with(&facts.prefix) {
                push_item(
                    items,
                    permission.to_owned(),
                    Some(CompletionItemKind::Field),
                    Some("Permission purpose string".to_owned()),
                    None,
                    &format!("{permission}: \"${{1:Why the app needs this permission}}\""),
                );
            }
        }
        return;
    }
    if current == "flavors" && !names.iter().any(|name| name.starts_with("flavors")) {
        return;
    }
    let used = config_used_fields(&facts.words, &facts.open);
    for field in fields
        .iter()
        .filter(|field| field.starts_with(&facts.prefix) && !used.contains(**field))
    {
        let (value, snippet) = config_field_insert(current, field);
        push_item(
            items,
            (*field).to_owned(),
            Some(CompletionItemKind::Field),
            Some(format!("{summary}: {field}")),
            None,
            &format!("{field}: {snippet}"),
        );
        let _ = value;
    }
}

fn add_modifiers(prefix: &str, items: &mut Vec<CompletionItem>) {
    for modifier in catalog::DOT_MODIFIERS
        .iter()
        .filter(|item| item.name.starts_with(prefix))
    {
        push_item(
            items,
            modifier.name.to_owned(),
            Some(CompletionItemKind::Method),
            Some(modifier.summary.to_owned()),
            None,
            modifier.snippet,
        );
    }
}

fn block_owners(words: &[Word], open: &[(char, usize)]) -> Vec<String> {
    open.iter()
        .filter(|(kind, _)| *kind == '{')
        .filter_map(|(_, index)| previous_identifier(words, *index))
        .collect()
}

fn config_used_fields(words: &[Word], open: &[(char, usize)]) -> HashSet<String> {
    let Some((_, index)) = open.iter().rev().find(|(kind, _)| *kind == '{') else {
        return HashSet::new();
    };
    let mut used = HashSet::new();
    for pair in words.get(index + 1..).unwrap_or_default().windows(2) {
        if pair[1].text == ":" && is_identifier(&pair[0].text) {
            used.insert(pair[0].text.clone());
        }
    }
    used
}

fn config_field_insert(owner: &str, field: &str) -> (&'static str, &'static str) {
    match field {
        "app" | "ios" | "android" | "assets" | "flavors" | "dependencies" | "permissions"
        | "plugins" | "cronet" => ("", "{\n    $0\n}"),
        "buildNumber" | "minSdk" | "targetSdk" | "diskCacheSizeMb" => ("number", "${1:1}"),
        "deepLinks" | "arch" => ("array", "[\"${1:value}\"]"),
        "orientation" => ("enum", "\"${1|all,portrait,portrait-phones|}\""),
        "provider" if owner == "cronet" => ("enum", "\"${1|play-services,embedded,system|}\""),
        "" => ("", "$0"),
        _ => ("string", "\"${1:value}\""),
    }
}

fn add_config_value_completions(
    source: &str,
    facts: &CursorFacts,
    items: &mut Vec<CompletionItem>,
) {
    let Some(field) = facts
        .words
        .iter()
        .rev()
        .find(|word| !word.string && is_identifier(&word.text))
        .map(|word| word.text.as_str())
    else {
        return;
    };
    let values: &[&str] = match field {
        "orientation" => &["all", "portrait", "portrait-phones"],
        "provider" if source.contains("cronet") => &["play-services", "embedded", "system"],
        "arch" => &["arm64", "x86_64", "arm64-v8a", "armeabi-v7a"],
        _ => &[],
    };
    for value in values
        .iter()
        .filter(|value| value.starts_with(&facts.prefix))
    {
        push_item(
            items,
            (*value).to_owned(),
            Some(CompletionItemKind::EnumMember),
            Some(format!("Valid `{field}` value")),
            None,
            value,
        );
    }
}

fn finish(
    mut items: Vec<CompletionItem>,
    source: &str,
    facts: &CursorFacts,
    index: &LineIndex,
) -> Vec<CompletionItem> {
    let start = index.to_position(source, facts.prefix_start);
    let end = index.to_position(source, facts.cursor);
    for item in &mut items {
        item.filter_text = Some(item.label.clone());
        item.sort_text = Some(format!("{:02}_{}", completion_rank(item.kind), item.label));
        item.text_edit = Some(TextEdit {
            range: Range { start, end },
            new_text: item
                .insert_text
                .clone()
                .unwrap_or_else(|| item.label.clone()),
        });
        if item
            .insert_text
            .as_ref()
            .is_some_and(|text| text.contains("${"))
        {
            item.insert_text_format = Some(SNIPPET_FORMAT);
        }
    }
    items.sort_by(|a, b| a.sort_text.cmp(&b.sort_text));
    items.dedup_by(|a, b| a.label == b.label);
    items
}

fn completion_rank(kind: Option<CompletionItemKind>) -> u8 {
    match kind {
        Some(
            CompletionItemKind::Variable | CompletionItemKind::Field | CompletionItemKind::Property,
        ) => 0,
        Some(CompletionItemKind::Method | CompletionItemKind::Function) => 1,
        Some(
            CompletionItemKind::Constructor
            | CompletionItemKind::Class
            | CompletionItemKind::Struct,
        ) => 2,
        Some(CompletionItemKind::Snippet) => 3,
        _ => 4,
    }
}

fn push_item(
    items: &mut Vec<CompletionItem>,
    label: String,
    kind: Option<CompletionItemKind>,
    detail: Option<String>,
    documentation: Option<String>,
    insert_text: &str,
) {
    items.push(CompletionItem {
        label,
        kind,
        detail,
        documentation,
        insert_text: Some(insert_text.to_owned()),
        filter_text: None,
        sort_text: None,
        insert_text_format: None,
        text_edit: None,
    });
}

fn is_identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_identifier_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_identifier_continue)
}

#[cfg(test)]
mod tests {
    use super::{SNIPPET_FORMAT, get_completions, get_document_completions};
    use crate::protocol::Position;

    fn labels_at(source: &str, line: u32, character: u32) -> Vec<String> {
        get_completions(source, Position { line, character })
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    #[test]
    fn app_body_suggests_components_but_action_body_suggests_symbols_and_statements() {
        let source = "app P {\n state count: Int32 = 0\n body {\n  Tex\n  Button(\"Add\") {\n   coun\n  }\n }\n}";
        let nodes = labels_at(source, 3, 5);
        assert!(nodes.contains(&"Text".to_owned()));
        assert!(!nodes.contains(&"app".to_owned()));
        let action = labels_at(source, 5, 3);
        assert!(action.contains(&"count".to_owned()));
        assert!(action.contains(&"if".to_owned()));
        assert!(!action.contains(&"Column".to_owned()));
    }

    #[test]
    fn component_arguments_offer_only_contextual_schema_fields() {
        let source = "app P { body { Column(spacing: 8, padd) { Text(\"x\") } } }";
        let offset = source.find("padd").unwrap_or(0) + 4;
        let line_index = crate::line_index::LineIndex::new(source);
        let labels = get_completions(source, line_index.to_position(source, offset));
        let labels = labels
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"padding".to_owned()));
        assert!(!labels.contains(&"spacing".to_owned()));
        assert!(!labels.contains(&"fontSize".to_owned()));
    }

    #[test]
    fn text_components_offer_the_translator_comment_argument() {
        let source = "app P { body { Text(\"Save\", comm) } }";
        let index = crate::line_index::LineIndex::new(source);
        let cursor = source.find("comm").unwrap_or(0) + "comm".len();
        let labels = get_completions(source, index.to_position(source, cursor))
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"comment".to_owned()));
    }

    #[test]
    fn member_completion_uses_the_actual_namespace() {
        let source = "app P { body { Text(Locale.dis) } }";
        let offset = source.find("dis)").unwrap_or(0) + 3;
        let index = crate::line_index::LineIndex::new(source);
        let labels = get_completions(source, index.to_position(source, offset))
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"displayName".to_owned()));
        assert!(!labels.contains(&"onTap".to_owned()));
    }

    #[test]
    fn config_file_completion_is_specific_to_the_active_block() {
        let source = "config {\n app {\n  orient\n }\n}";
        let index = crate::line_index::LineIndex::new(source);
        let labels = get_document_completions(
            source,
            index.to_position(source, source.find("orient").unwrap_or(0) + 6),
            true,
        )
        .into_iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
        assert!(labels.contains(&"orientation".to_owned()));
        assert!(!labels.contains(&"targetSdk".to_owned()));
    }

    #[test]
    fn snippets_replace_the_prefix_and_declare_snippet_format() {
        let source = "app P { body { But } }";
        let index = crate::line_index::LineIndex::new(source);
        let items = get_completions(
            source,
            index.to_position(source, source.find("But").unwrap_or(0) + 3),
        );
        let button = items
            .iter()
            .find(|item| item.label == "Button")
            .expect("button snippet");
        assert_eq!(button.insert_text_format, Some(SNIPPET_FORMAT));
        assert_eq!(
            button.text_edit.as_ref().map(|edit| edit.new_text.as_str()),
            button.insert_text.as_deref()
        );
    }

    #[test]
    fn comments_and_incomplete_strings_do_not_create_fake_member_contexts() {
        let source = "app P { body { Text(\".onTap\") // .onTap\n Tex } }";
        let items = labels_at(source, 1, 5);
        assert!(items.contains(&"Text".to_owned()));
        let string_items = get_completions(
            "app P { body { Text(\"unterminated",
            Position {
                line: 0,
                character: 34,
            },
        );
        assert!(string_items.is_empty());
    }
}
