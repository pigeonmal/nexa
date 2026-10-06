use nexa_codegen::SourceWriter;
use nexa_ir::{
    Action, AutofillType, Capitalization, KeyboardType, ReturnKeyType, SystemIcon, TextInputChange,
    TextInputFont,
};

use nexa_codegen::names::state_name;

use crate::generator::{
    controls::render_actions,
    utils::{indent, swift_string},
};

pub(crate) struct TextInputProps<'a> {
    pub(crate) state: &'a str,
    pub(crate) placeholder: &'a str,
    pub(crate) comment: Option<&'a str>,
    pub(crate) keyboard: KeyboardType,
    pub(crate) secure: bool,
    pub(crate) multiline: bool,
    pub(crate) autofill: Option<AutofillType>,
    pub(crate) return_key: Option<ReturnKeyType>,
    pub(crate) autocorrect: Option<bool>,
    pub(crate) capitalization: Option<Capitalization>,
    pub(crate) focused: Option<&'a str>,
    pub(crate) max_length: Option<i32>,
    pub(crate) font: Option<TextInputFont>,
    pub(crate) min_lines: Option<i32>,
    pub(crate) max_lines: Option<i32>,
    pub(crate) searchable: bool,
    pub(crate) actions: &'a [Action],
    pub(crate) on_change: Option<&'a TextInputChange>,
}

pub(crate) fn render_text_input(props: TextInputProps<'_>, depth: usize, out: &mut SourceWriter) {
    let TextInputProps {
        state,
        placeholder,
        comment,
        keyboard,
        secure,
        multiline,
        autofill,
        return_key,
        autocorrect,
        capitalization,
        focused,
        max_length,
        font,
        min_lines,
        max_lines,
        searchable,
        actions,
        on_change,
    } = props;
    indent(out, depth);
    if searchable {
        let search_symbol = SystemIcon::Shared("search".to_owned()).sf_symbol_name();
        out.push_str("HStack(spacing: 8) {\n");
        out.line_at(
            depth + 1,
            format_args!(
                "Image(systemName: {}).foregroundStyle(.secondary)",
                swift_string(&search_symbol)
            ),
        );
        indent(out, depth + 1);
    }
    let control = if secure { "SecureField" } else { "TextField" };
    let placeholder = match comment {
        Some(comment) => format!(
            "Text({}, comment: {})",
            swift_string(placeholder),
            swift_string(comment)
        ),
        None => swift_string(placeholder),
    };
    if multiline {
        out.push_str(&format!(
            "TextField({placeholder}, text: ${}, axis: .vertical)",
            state_name(state)
        ));
    } else {
        out.push_str(&format!(
            "{control}({placeholder}, text: ${})",
            state_name(state)
        ));
    }
    if let Some(font) = font {
        let font = match font {
            TextInputFont::Body => ".body",
            TextInputFont::Title3 => ".title3",
        };
        out.push_str(&format!("\n{}.font({font})", "    ".repeat(depth + 1)));
    }
    let line_limit = match (min_lines, max_lines) {
        (Some(minimum), Some(maximum)) => Some(format!("{minimum}...{maximum}")),
        (Some(minimum), None) => Some(format!("{minimum}...")),
        (None, Some(maximum)) => Some(format!("...{maximum}")),
        (None, None) => None,
    };
    if let Some(line_limit) = line_limit {
        out.push_str(&format!(
            "\n{}.lineLimit({line_limit})",
            "    ".repeat(depth + 1)
        ));
    }
    if let Some(keyboard) = swift_keyboard(keyboard) {
        out.push_str(&format!(
            "\n{}.keyboardType({keyboard})",
            "    ".repeat(depth + 1)
        ));
    }
    if let Some(autofill) = autofill {
        out.push_str(&format!(
            "\n{}.textContentType({})",
            "    ".repeat(depth + 1),
            swift_autofill(autofill)
        ));
    }
    if let Some(return_key) = return_key {
        out.push_str(&format!(
            "\n{}.submitLabel({})",
            "    ".repeat(depth + 1),
            swift_return_key(return_key)
        ));
    }
    if let Some(capitalization) = capitalization {
        out.push_str(&format!(
            "\n{}.textInputAutocapitalization({})",
            "    ".repeat(depth + 1),
            swift_capitalization(capitalization)
        ));
    }
    if let Some(autocorrect) = autocorrect {
        out.push_str(&format!(
            "\n{}.autocorrectionDisabled({})",
            "    ".repeat(depth + 1),
            !autocorrect
        ));
    }
    if let Some(focused) = focused {
        out.push_str(&format!(
            "\n{}.focused(${})",
            "    ".repeat(depth + 1),
            state_name(focused)
        ));
    }
    if !actions.is_empty() {
        out.push_str(&format!("\n{}.onSubmit {{\n", "    ".repeat(depth + 1)));
        render_actions(actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
    }
    if let Some(change) = on_change {
        out.push_str(&format!(
            "\n{}.onChange(of: {}) {{ {} in\n",
            "    ".repeat(depth + 1),
            state_name(state),
            state_name(&change.parameter)
        ));
        render_actions(&change.actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
    }
    if let Some(max_length) = max_length {
        out.push_str(&format!(
            "\n{}.onChange(of: {}) {{ newValue in\n",
            "    ".repeat(depth + 1),
            state_name(state)
        ));
        out.line_at(
            depth + 2,
            format_args!(
                "if newValue.count > {} {{ {} = String(newValue.prefix(Int({}))) }}",
                max_length,
                state_name(state),
                max_length
            ),
        );
        indent(out, depth + 1);
        out.push('}');
    }
    if searchable {
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
}

pub(crate) fn swift_keyboard(keyboard: KeyboardType) -> Option<&'static str> {
    match keyboard {
        KeyboardType::Text => None,
        KeyboardType::Number => Some(".numberPad"),
        KeyboardType::Email => Some(".emailAddress"),
        KeyboardType::Phone => Some(".phonePad"),
        KeyboardType::Url => Some(".URL"),
    }
}

pub(crate) fn swift_capitalization(capitalization: Capitalization) -> &'static str {
    match capitalization {
        Capitalization::None => ".never",
        Capitalization::Sentences => ".sentences",
        Capitalization::Words => ".words",
        Capitalization::Characters => ".characters",
    }
}

pub(crate) fn swift_autofill(autofill: AutofillType) -> &'static str {
    match autofill {
        AutofillType::Username => ".username",
        AutofillType::Password => ".password",
        AutofillType::OneTimeCode => ".oneTimeCode",
    }
}

pub(crate) fn swift_return_key(return_key: ReturnKeyType) -> &'static str {
    match return_key {
        ReturnKeyType::Done => ".done",
        ReturnKeyType::Search => ".search",
        ReturnKeyType::Send => ".send",
        ReturnKeyType::Next => ".next",
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;

    use super::{TextInputProps, render_text_input};
    use nexa_ir::{Capitalization, KeyboardType};

    #[test]
    fn searchable_input_is_a_real_bound_native_text_field() {
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "query",
                placeholder: "Search items",
                comment: None,
                keyboard: KeyboardType::Text,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: None,
                autocorrect: Some(false),
                capitalization: Some(Capitalization::None),
                focused: None,
                max_length: None,
                font: None,
                min_lines: None,
                max_lines: None,
                searchable: true,
                actions: &[],
                on_change: None,
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains("HStack(spacing: 8)"));
        assert!(source.contains("Image(systemName: \"magnifyingglass\")"));
        assert!(source.contains("TextField(\"Search items\", text: $nexa_query)"));
        assert!(!source.contains("EmptyView().searchable"));
    }
}
