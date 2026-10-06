//! Source-text localization records derived from typed UI and native API expressions.
//!
//! Literal UI strings are their own keys. Interpolations use named holes such
//! as `"Welcome, {name}"`, keeping translator files readable while native
//! resource backends render platform-specific format placeholders.

use std::collections::BTreeMap;

use crate::{
    Expr, InterpolatedPart, Module, NumericType, Type,
    walk::{walk_actions, walk_expression, walk_ir},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEntry {
    pub key: String,
    pub comment: Option<String>,
    pub arguments: Vec<String>,
    pub argument_types: Vec<String>,
    pub apple_key: String,
    pub apple_value: String,
    pub android_value: String,
}

/// Returns the human-readable source key for a static string or interpolation.
pub fn source_key(expression: &Expr) -> Option<String> {
    match expression {
        Expr::LocalizedText { key, .. } => Some(key.clone()),
        Expr::String(value) => Some(value.clone()),
        Expr::Interpolation(parts) => {
            let mut key = String::new();
            let mut index = 0;
            for part in parts {
                match part {
                    InterpolatedPart::Literal(value) => key.push_str(value),
                    InterpolatedPart::Value(value) => {
                        key.push('{');
                        key.push_str(&argument_name(value, index));
                        key.push('}');
                        index += 1;
                    }
                }
            }
            Some(key)
        }
        _ => None,
    }
}

/// The typed expressions interpolated by a localized source string.
pub fn interpolation_arguments(expression: &Expr) -> Vec<&Expr> {
    let expression = match expression {
        Expr::LocalizedText { value, .. } => value.as_ref(),
        expression => expression,
    };
    match expression {
        Expr::Interpolation(parts) => parts
            .iter()
            .filter_map(|part| match part {
                InterpolatedPart::Value(value) => Some(value.as_ref()),
                InterpolatedPart::Literal(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn is_integer_argument(expression: &Expr) -> bool {
    matches!(
        expression_type(expression),
        Some(
            NumericType::Int8
                | NumericType::Int16
                | NumericType::Int32
                | NumericType::Int64
                | NumericType::UInt8
                | NumericType::UInt16
                | NumericType::UInt32
                | NumericType::UInt64
        )
    )
}

/// Builds a source-language format value for Android string resources.
/// Android accepts `%s` for every value because Java's formatter stringifies
/// numbers and booleans without boxing in generated Compose call sites.
pub fn android_format_value(expression: &Expr) -> Option<String> {
    match expression {
        Expr::LocalizedText { value, .. } => android_format_value(value),
        Expr::String(value) => Some(value.clone()),
        Expr::Interpolation(parts) => {
            let mut value = String::new();
            let mut index = 0;
            for part in parts {
                match part {
                    InterpolatedPart::Literal(literal) => value.push_str(literal),
                    InterpolatedPart::Value(argument) => {
                        index += 1;
                        value.push_str(&format!("%{index}${}", android_format_kind(argument)));
                    }
                }
            }
            Some(value)
        }
        _ => None,
    }
}

/// Builds a source-language format value for Apple's string catalog.
pub fn apple_format_value(expression: &Expr) -> Option<String> {
    match expression {
        Expr::LocalizedText { value, .. } => apple_format_value(value),
        Expr::String(value) => Some(value.clone()),
        Expr::Interpolation(parts) => {
            let mut value = String::new();
            let mut index = 0;
            for part in parts {
                match part {
                    InterpolatedPart::Literal(literal) => value.push_str(literal),
                    InterpolatedPart::Value(argument) => {
                        index += 1;
                        let specifier = match expression_type(argument) {
                            Some(NumericType::Int32) => "d",
                            Some(NumericType::Int64) => "lld",
                            Some(NumericType::Float32 | NumericType::Float64) => "f",
                            _ => "@",
                        };
                        value.push_str(&format!("%{index}${specifier}"));
                    }
                }
            }
            Some(value)
        }
        _ => None,
    }
}

/// Apple's SwiftUI string catalog key for a localizable string interpolation.
/// The formatted fallback keeps explicit positions for `String(format:)`, but
/// `Text("Hello, \(name)")` is extracted under an unnumbered `%@` key.
pub fn apple_catalog_key(expression: &Expr) -> Option<String> {
    let formatted = apple_format_value(expression)?;
    let mut output = String::with_capacity(formatted.len());
    let mut cursor = 0;
    while cursor < formatted.len() {
        let rest = &formatted[cursor..];
        if let Some(after_percent) = rest.strip_prefix('%') {
            let digit_count = after_percent.bytes().take_while(u8::is_ascii_digit).count();
            if digit_count > 0 && after_percent.as_bytes().get(digit_count) == Some(&b'$') {
                output.push('%');
                cursor += 1 + digit_count + 1;
                continue;
            }
            output.push('%');
            cursor += 1;
            continue;
        }
        let character = rest.chars().next()?;
        output.push(character);
        cursor += character.len_utf8();
    }
    Some(output)
}

pub fn humanize_identifier(value: &str) -> String {
    let mut result = String::new();
    let mut previous = None;
    for character in value.chars() {
        if !character.is_ascii_alphanumeric() {
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            previous = None;
            continue;
        }
        if character.is_ascii_uppercase()
            && previous
                .is_some_and(|prior: char| prior.is_ascii_lowercase() || prior.is_ascii_digit())
        {
            result.push(' ');
        }
        if result.is_empty() || result.ends_with(' ') {
            result.push(character.to_ascii_uppercase());
        } else {
            result.push(character);
        }
        previous = Some(character);
    }
    result
}

pub fn widget_display_name(widget: &crate::Widget) -> String {
    widget
        .display_name
        .clone()
        .unwrap_or_else(|| humanize_identifier(&widget.name))
}

pub fn widget_description(widget: &crate::Widget) -> String {
    widget
        .description
        .clone()
        .unwrap_or_else(|| format!("{} widget", humanize_identifier(&widget.name)))
}

pub fn widget_configuration_title(widget: &crate::Widget) -> String {
    widget
        .configuration_title
        .clone()
        .unwrap_or_else(|| widget_display_name(widget))
}

/// Extract localized UI literals and explicit `Locale.localized` values from
/// the full module, including functions used by native APIs. Repeated source
/// text has one entry; distinct translator comments are retained in stable
/// order.
pub fn extract(module: &Module) -> BTreeMap<String, TextEntry> {
    let mut entries = BTreeMap::<String, TextEntry>::new();
    for widget in &module.widgets {
        for value in [widget_display_name(widget), widget_description(widget)] {
            insert_plain_text(&mut entries, value);
        }
        if widget.configuration.is_some() {
            insert_plain_text(&mut entries, widget_configuration_title(widget));
            if let Some(description) = &widget.configuration_description {
                insert_plain_text(&mut entries, description.clone());
            }
            insert_plain_text(&mut entries, "Save".to_owned());
            if let Some(crate::WidgetConfiguration {
                ty: Type::Struct { fields, .. },
                ..
            }) = &widget.configuration
            {
                for (field, ty) in fields {
                    let Type::Enum(enum_name) = ty else {
                        continue;
                    };
                    insert_plain_text(&mut entries, humanize_identifier(field));
                    if let Some(declaration) =
                        module.enums.iter().find(|item| item.name == *enum_name)
                    {
                        for case in &declaration.cases {
                            insert_plain_text(&mut entries, humanize_identifier(case));
                        }
                    }
                }
            }
        }
    }
    for nodes in std::iter::once(&module.body)
        .chain(module.screens.iter().map(|screen| &screen.body))
        .chain(module.components.iter().map(|component| &component.body))
        .chain(module.widgets.iter().map(|widget| &widget.body))
    {
        walk_ir(
            nodes,
            &mut |node| {
                if let crate::Node::AppBottomBar { tabs, .. } = node {
                    for tab in tabs {
                        insert_text(&mut entries, tab.label.clone(), tab.comment.clone());
                        if let Some(title) = &tab.navigation_title {
                            insert_text(&mut entries, title.clone(), None);
                        }
                        if let Some(prompt) = &tab.search_prompt {
                            insert_text(&mut entries, prompt.clone(), None);
                        }
                    }
                }
                if let crate::Node::TextInput {
                    placeholder,
                    comment,
                    ..
                } = node
                    && !placeholder.trim().is_empty()
                {
                    insert_text(&mut entries, placeholder.clone(), comment.clone());
                }
            },
            &mut |_| {},
        );
        walk_ir(nodes, &mut |_| {}, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
    }

    for function in &module.functions {
        for local in function.locals.iter().chain(&function.class_initializers) {
            walk_expression(&local.initial, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
        walk_expression(&function.body, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
        if let Some(actions) = &function.body_actions {
            walk_actions(actions, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
    }

    for state in module.states.iter().chain(&module.globals) {
        walk_expression(&state.initial, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
    }
    for actions in [
        module.on_appear.as_deref(),
        module.on_disappear.as_deref(),
        module.on_active.as_deref(),
        module.on_inactive.as_deref(),
        module.on_background.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        walk_actions(actions, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
    }
    for task in &module.background_tasks {
        walk_actions(&task.actions, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
    }
    for screen in &module.screens {
        for state in &screen.states {
            walk_expression(&state.initial, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
        for actions in [screen.on_appear.as_deref(), screen.on_disappear.as_deref()]
            .into_iter()
            .flatten()
        {
            walk_actions(actions, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
    }
    for component in &module.components {
        for state in &component.states {
            walk_expression(&state.initial, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
        for actions in [
            component.on_appear.as_deref(),
            component.on_disappear.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            walk_actions(actions, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
    }
    for widget in &module.widgets {
        walk_expression(&widget.entry_provider, &mut |expression| {
            insert_localized_expression(&mut entries, expression)
        });
        if let Some(provider) = &widget.placeholder_provider {
            walk_expression(provider, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
        if let Some(configuration) = &widget.configuration {
            walk_expression(&configuration.default, &mut |expression| {
                insert_localized_expression(&mut entries, expression)
            });
        }
    }
    entries
}

fn insert_localized_expression(entries: &mut BTreeMap<String, TextEntry>, expression: &Expr) {
    let Expr::LocalizedText {
        key,
        value,
        comment,
    } = expression
    else {
        return;
    };
    let mut arguments = Vec::new();
    let mut argument_types = Vec::new();
    for (index, argument) in interpolation_arguments(value).into_iter().enumerate() {
        arguments.push(argument_name(argument, index));
        argument_types.push(argument_type_name(argument));
    }
    let Some(apple_value) = apple_format_value(value) else {
        return;
    };
    let Some(apple_key) = apple_catalog_key(value) else {
        return;
    };
    let Some(android_value) = android_format_value(value) else {
        return;
    };
    insert_entry(
        entries,
        TextEntry {
            key: key.clone(),
            comment: comment.clone(),
            arguments,
            argument_types,
            apple_key,
            apple_value,
            android_value,
        },
    );
}

fn insert_plain_text(entries: &mut BTreeMap<String, TextEntry>, value: String) {
    insert_text(entries, value, None);
}

fn insert_text(entries: &mut BTreeMap<String, TextEntry>, value: String, comment: Option<String>) {
    insert_entry(
        entries,
        TextEntry {
            key: value.clone(),
            comment,
            arguments: Vec::new(),
            argument_types: Vec::new(),
            apple_key: value.clone(),
            apple_value: value.clone(),
            android_value: value,
        },
    );
}

fn insert_entry(entries: &mut BTreeMap<String, TextEntry>, new_entry: TextEntry) {
    match entries.get_mut(&new_entry.key) {
        Some(entry) => merge_comment(&mut entry.comment, new_entry.comment.as_deref()),
        None => {
            entries.insert(new_entry.key.clone(), new_entry);
        }
    }
}

fn merge_comment(target: &mut Option<String>, comment: Option<&str>) {
    let Some(comment) = comment.filter(|comment| !comment.trim().is_empty()) else {
        return;
    };
    match target {
        Some(existing) if existing.split("\n").any(|line| line == comment) => {}
        Some(existing) => {
            existing.push('\n');
            existing.push_str(comment);
        }
        None => *target = Some(comment.to_owned()),
    }
}

pub fn argument_name(expression: &Expr, index: usize) -> String {
    match expression {
        Expr::State(name, _) | Expr::AnimatedState(name, _) => name
            .rsplit_once("::")
            .map_or(name.as_str(), |(_, property)| property)
            .to_owned(),
        Expr::Member { name, .. } => name.clone(),
        _ => format!("value{}", index + 1),
    }
}

fn expression_type(expression: &Expr) -> Option<NumericType> {
    match expression {
        Expr::State(_, Type::Numeric(ty)) | Expr::AnimatedState(_, Type::Numeric(ty)) => Some(*ty),
        Expr::Member {
            field_type: Type::Numeric(ty),
            ..
        }
        | Expr::Index {
            element_type: Type::Numeric(ty),
            ..
        }
        | Expr::Call {
            return_type: Type::Numeric(ty),
            ..
        }
        | Expr::NativeCall {
            return_type: Type::Numeric(ty),
            ..
        }
        | Expr::TimeCall {
            return_type: Type::Numeric(ty),
            ..
        } => Some(*ty),
        Expr::Number { ty, .. } => Some(*ty),
        _ => None,
    }
}

fn argument_type_name(expression: &Expr) -> String {
    if let Some(ty) = expression_type(expression) {
        return format!("{ty:?}");
    }
    match expression {
        Expr::State(_, Type::Bool) | Expr::AnimatedState(_, Type::Bool) | Expr::Bool(_) => {
            "Bool".to_owned()
        }
        Expr::Member {
            field_type: Type::Bool,
            ..
        }
        | Expr::Index {
            element_type: Type::Bool,
            ..
        } => "Bool".to_owned(),
        Expr::State(_, Type::String)
        | Expr::AnimatedState(_, Type::String)
        | Expr::String(_)
        | Expr::Member {
            field_type: Type::String,
            ..
        }
        | Expr::Index {
            element_type: Type::String,
            ..
        }
        | Expr::Call {
            return_type: Type::String,
            ..
        }
        | Expr::NativeCall {
            return_type: Type::String,
            ..
        } => "String".to_owned(),
        _ => "String".to_owned(),
    }
}

fn android_format_kind(expression: &Expr) -> char {
    match expression_type(expression) {
        Some(NumericType::Float32 | NumericType::Float64) => 'f',
        Some(_) => 'd',
        None => 's',
    }
}
