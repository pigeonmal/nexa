use crate::protocol::{CompletionItem, CompletionItemKind, Position};

/// Generates code completions for the given document and cursor position.
pub fn get_completions(_source: &str, _position: Position) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Layout and UI Components
    let components = [
        (
            "Column",
            "Vertical layout container with optional spacing",
            "Column(spacing: ${1:8}) {\n    $0\n}",
        ),
        (
            "Row",
            "Horizontal layout container with optional spacing",
            "Row(spacing: ${1:8}) {\n    $0\n}",
        ),
        (
            "Stack",
            "Overlapping layout container",
            "Stack {\n    $0\n}",
        ),
        (
            "VStack",
            "Vertical layout container alias",
            "Column(spacing: ${1:8}) {\n    $0\n}",
        ),
        (
            "HStack",
            "Horizontal layout container alias",
            "Row(spacing: ${1:8}) {\n    $0\n}",
        ),
        (
            "ZStack",
            "Overlapping layout container alias",
            "Stack {\n    $0\n}",
        ),
        ("Text", "Displays formatted text", "Text(\"${1:Label}\")"),
        (
            "Button",
            "Interactive button with click action handler",
            "Button(\"${1:Title}\") {\n    $0\n}",
        ),
        (
            "TextField",
            "Text input control bound to mutable state",
            "TextField(text: ${1:binding}, placeholder: \"${2:Enter text}\")",
        ),
        (
            "Image",
            "Displays an asset or network image",
            "Image(\"${1:icon}\")",
        ),
        (
            "FastList",
            "High-performance virtualized list view",
            "FastList(${1:items}) { ${2:item} ->\n    $0\n}",
        ),
        (
            "FastSectionedList",
            "Virtualized sectioned list with sticky headers",
            "FastSectionedList(${1:sections}) {\n    $0\n}",
        ),
        ("Spacer", "Flexible expanding space in stacks", "Spacer()"),
        ("Divider", "Visual separator line", "Divider()"),
        (
            "OnAppear",
            "Lifecycle trigger executed when the screen or component appears",
            "OnAppear {\n    $0\n}",
        ),
        (
            "OnDisappear",
            "Lifecycle trigger executed when the screen or component disappears",
            "OnDisappear {\n    $0\n}",
        ),
    ];

    for (name, doc, snippet) in components {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::Constructor),
            detail: Some(format!("Nexa UI Component: {name}")),
            documentation: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
        });
    }

    // 2. Modifiers
    let modifiers = [
        (
            "padding",
            "Applies uniform or directional padding",
            "padding(${1:16})",
        ),
        (
            "spacing",
            "Sets inter-element spacing in stacks",
            "spacing(${1:8})",
        ),
        ("fontSize", "Sets font size in points", "fontSize(${1:16})"),
        (
            "fontWeight",
            "Sets font weight (bold, medium, light)",
            "fontWeight(${1:bold})",
        ),
        (
            "color",
            "Sets foreground or text color",
            "color(\"${1:#000000}\")",
        ),
        (
            "background",
            "Sets background color or shape",
            "background(\"${1:#ffffff}\")",
        ),
        (
            "cornerRadius",
            "Sets corner rounding radius",
            "cornerRadius(${1:8})",
        ),
        ("width", "Explicit width dimension", "width(${1:100})"),
        ("height", "Explicit height dimension", "height(${1:50})"),
        (
            "opacity",
            "View opacity level (0.0 to 1.0)",
            "opacity(${1:0.8})",
        ),
        (
            "onClick",
            "Action executed when clicked or tapped",
            "onClick {\n    $0\n}",
        ),
    ];

    for (name, doc, snippet) in modifiers {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::Method),
            detail: Some(format!(".{name}(...)")),
            documentation: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
        });
    }

    // 3. Keywords
    let keywords = [
        ("app", "Defines the application root"),
        ("screen", "Defines an application screen route"),
        ("component", "Declares a reusable UI component"),
        ("state", "Declares mutable reactive state"),
        ("let", "Declares an immutable local binding"),
        ("fn", "Declares a function"),
        ("enum", "Declares an enumeration"),
        ("struct", "Declares a data structure"),
        ("plugin", "Imports a native or pure Nexa plugin"),
        ("try", "Initiates an error recovery block"),
        ("catch", "Catches typed or general errors"),
        ("case", "Pattern matches an error or enum variant"),
        ("return", "Returns from a function"),
        ("if", "Conditional branching"),
        ("else", "Alternative conditional branch"),
        ("while", "Repeats an action while condition is true"),
        ("for", "Iterates over a collection or range"),
        ("await", "Awaits an asynchronous operation"),
    ];

    for (kw, doc) in keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::Keyword),
            detail: Some("Nexa Keyword".to_string()),
            documentation: Some(doc.to_string()),
            insert_text: None,
        });
    }

    // 4. Built-in Types
    let types = [
        ("String", "UTF-8 string"),
        ("Int32", "32-bit signed integer"),
        ("Int64", "64-bit signed integer"),
        ("Float32", "32-bit floating point number"),
        ("Float64", "64-bit floating point number"),
        ("Bool", "Boolean value (true or false)"),
        ("Bytes", "Binary byte buffer"),
        ("Result", "Zero-overhead Result<T, E> error handling type"),
        ("Array", "Ordered generic list Array<T>"),
        ("Map", "Key-value dictionary Map<K, V>"),
        ("Set", "Unique element set Set<T>"),
        ("Ok", "Constructs a successful Result value: Ok(value)"),
        ("Err", "Constructs an error Result value: Err(error)"),
    ];

    for (name, doc) in types {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::Class),
            detail: Some(format!("Nexa Type: {name}")),
            documentation: Some(doc.to_string()),
            insert_text: None,
        });
    }

    items
}
