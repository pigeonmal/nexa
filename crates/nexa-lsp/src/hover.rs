use crate::protocol::{Hover, MarkupContent, Position};

/// Returns hover documentation for the token under the cursor.
pub fn get_hover(source: &str, position: Position) -> Option<Hover> {
    let word = extract_word_at_position(source, position)?;
    let doc = hover_documentation(&word)?;

    Some(Hover {
        contents: MarkupContent {
            kind: "markdown".to_string(),
            value: doc,
        },
        range: None,
    })
}

fn extract_word_at_position(source: &str, position: Position) -> Option<String> {
    let line = source.lines().nth(position.line as usize)?;
    let char_idx = position.character as usize;
    if char_idx > line.len() {
        return None;
    }

    let bytes = line.as_bytes();
    let mut start = char_idx;
    while start > 0 && is_ident_char(bytes[start - 1] as char) {
        start -= 1;
    }

    let mut end = char_idx;
    while end < bytes.len() && is_ident_char(bytes[end] as char) {
        end += 1;
    }

    if start == end {
        None
    } else {
        Some(line[start..end].to_string())
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '?'
}

fn hover_documentation(token: &str) -> Option<String> {
    match token {
        "Column" | "VStack" => Some(
            "### `Column`\n\nVertical stack container that lays out children in a single column.\n\n```nx\nColumn(spacing: 8) {\n    Text(\"Top\")\n    Text(\"Bottom\")\n}\n```\n\n**Native Mapping:**\n- iOS: `SwiftUI.VStack`\n- Android: `androidx.compose.foundation.layout.Column`".to_string(),
        ),
        "Row" | "HStack" => Some(
            "### `Row`\n\nHorizontal stack container that lays out children in a single row.\n\n```nx\nRow(spacing: 8) {\n    Text(\"Left\")\n    Text(\"Right\")\n}\n```\n\n**Native Mapping:**\n- iOS: `SwiftUI.HStack`\n- Android: `androidx.compose.foundation.layout.Row`".to_string(),
        ),
        "Stack" | "ZStack" => Some(
            "### `Stack`\n\nOverlay stack container that overlays its child views on top of each other.\n\n```nx\nStack {\n    Image(\"background\")\n    Text(\"Overlay Text\")\n}\n```\n\n**Native Mapping:**\n- iOS: `SwiftUI.ZStack`\n- Android: `androidx.compose.foundation.layout.Box`".to_string(),
        ),
        "FastList" => Some(
            "### `FastList`\n\nUltra-high-performance virtualized list view. Emits specialized view types without `AnyView` type erasure on iOS and unboxed state flows on Android.\n\n```nx\nFastList(items) { item ->\n    Text(item.title)\n}\n```".to_string(),
        ),
        "Text" => Some(
            "### `Text`\n\nRenders a read-only text string.\n\n```nx\nText(\"Hello, Nexa!\")\n    .fontSize(18)\n    .fontWeight(bold)\n```".to_string(),
        ),
        "Button" => Some(
            "### `Button`\n\nStandard clickable push button with declarative action closure.\n\n```nx\nButton(\"Submit\") {\n    count = count + 1\n}\n```".to_string(),
        ),
        "TextField" => Some(
            "### `TextField`\n\nEditable single-line text input bound to reactive state.\n\n```nx\nTextField(text: username, placeholder: \"Enter username\")\n```".to_string(),
        ),
        "Result" => Some(
            "### `Result<T, E>`\n\nZero-overhead error handling type representing either success (`Ok(T)`) or failure (`Err(E)`).\n\nUse the postfix `?` operator to propagate errors upwards seamlessly.\n\n```nx\nfn load_user() -> Result<User, AppError> {\n    let user = fetch_user()?;\n    return Ok(user);\n}\n```".to_string(),
        ),
        "Ok" => Some(
            "### `Ok(value)`\n\nConstructs a successful `Result<T, E>` variant containing `value`.".to_string(),
        ),
        "Err" => Some(
            "### `Err(error)`\n\nConstructs an error `Result<T, E>` variant containing `error`.".to_string(),
        ),
        "state" => Some(
            "### `state` Keyword\n\nDeclares reactive mutable view state. When changed, only affected view leaves recompose.\n\n```nx\nstate count = 0\nstate is_loading: Bool = false\n```".to_string(),
        ),
        "app" => Some(
            "### `app` Keyword\n\nDeclares the root application entry point.\n\n```nx\napp MyApp {\n    body {\n        Text(\"Welcome\")\n    }\n}\n```".to_string(),
        ),
        "screen" => Some(
            "### `screen` Keyword\n\nDeclares a navigable screen route with independent state and navigation bar options.".to_string(),
        ),
        "component" => Some(
            "### `component` Keyword\n\nDeclares a reusable custom UI component with typed input parameters and content slots.".to_string(),
        ),
        "plugin" => Some(
            "### `plugin` Keyword\n\nImports a typed native plugin or pure Nexa package contract.\n\n```nx\nplugin \"./plugins/video\" as Video\n```".to_string(),
        ),
        _ => None,
    }
}
