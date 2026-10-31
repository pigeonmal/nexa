use nexa_codegen::SourceWriter;
use nexa_ir::{
    Action, AutofillType, Capitalization, ColorValue, KeyboardType, ReturnKeyType, TextInputChange,
    TextInputFont,
};

use nexa_codegen::names::state_name;

use crate::generator::{
    colors,
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
    pub(crate) font_size: Option<f32>,
    pub(crate) font_family: Option<&'a str>,
    pub(crate) text_color: Option<ColorValue>,
    pub(crate) placeholder_color: Option<ColorValue>,
    pub(crate) clearable: bool,
    pub(crate) min_lines: Option<i32>,
    pub(crate) max_lines: Option<i32>,
    pub(crate) searchable: bool,
    pub(crate) horizontal_padding: Option<u32>,
    pub(crate) actions: &'a [Action],
    pub(crate) on_focus: Option<&'a [Action]>,
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
        font_size,
        font_family,
        text_color,
        placeholder_color,
        clearable,
        min_lines,
        max_lines,
        searchable,
        horizontal_padding,
        actions,
        on_focus,
        on_change,
    } = props;
    let binding = format!("${}", state_name(state));
    let next_action_is_handled_by_primitive =
        return_key == Some(ReturnKeyType::Next) && focused.is_none();
    let placeholder = match comment {
        Some(comment) => format!(
            "Text({}, comment: {})",
            swift_string(placeholder),
            swift_string(comment)
        ),
        None => format!("Text({})", swift_string(placeholder)),
    };
    let max_length = max_length.map_or_else(|| "nil".to_owned(), |limit| limit.to_string());
    let font_size = font_size.map_or_else(|| "nil".to_owned(), |size| size.to_string());
    let font_family = font_family.map_or_else(|| "nil".to_owned(), swift_string);
    let font_relative_to = match font {
        Some(TextInputFont::Title3) => ".title3",
        Some(TextInputFont::Body) | None => ".body",
    };
    let default_font_size = match font {
        Some(TextInputFont::Title3) => nexa_codegen::design_system::TEXT_INPUT_TITLE3_FONT_SIZE,
        Some(TextInputFont::Body) | None => nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
    };
    let text_color = text_color.map_or_else(|| "nil".to_owned(), colors::expression);
    let placeholder_color = placeholder_color.map_or_else(|| "nil".to_owned(), colors::expression);
    let keeps_focus_on_next = return_key == Some(ReturnKeyType::Next) && focused.is_none();
    indent(out, depth);
    out.push_str(&format!(
        "NexaTextInputPrimitive(text: {binding}, placeholder: {placeholder}, secure: {secure}, multiline: {multiline}, searchable: {searchable}, maxLength: {max_length}, fontSize: {font_size}, fontFamily: {font_family}, fontRelativeTo: {font_relative_to}, fontDefaultSize: {default_font_size}, textColor: {text_color}, placeholderColor: {placeholder_color}, clearable: {clearable}, keepsFocusOnNext: {keeps_focus_on_next}"
    ));
    if next_action_is_handled_by_primitive && !actions.is_empty() {
        out.push_str(", onSubmitAction: {\n");
        render_actions(actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
    } else {
        out.push_str(", onSubmitAction: nil");
    }
    if let Some(change) = on_change {
        out.push_str(&format!(
            ", onChange: {{ {} in\n",
            state_name(&change.parameter)
        ));
        render_actions(&change.actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
    } else {
        out.push_str(", onChange: nil");
    }
    out.push(')');
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
        if let Some(actions) = on_focus {
            out.push_str(&format!(
                "\n{}.onChange(of: {}) {{ isFocused in\n",
                "    ".repeat(depth + 1),
                state_name(focused)
            ));
            indent(out, depth + 2);
            out.push_str("if isFocused {\n");
            render_actions(actions, depth + 3, out);
            indent(out, depth + 2);
            out.push_str("}\n");
            indent(out, depth + 1);
            out.push('}');
        }
    }
    if (!actions.is_empty() && !next_action_is_handled_by_primitive)
        || (return_key == Some(ReturnKeyType::Next) && focused.is_some())
    {
        out.push_str(&format!("\n{}.onSubmit {{\n", "    ".repeat(depth + 1)));
        if !actions.is_empty() {
            render_actions(actions, depth + 2, out);
        }
        if return_key == Some(ReturnKeyType::Next)
            && let Some(focused) = focused
        {
            indent(out, depth + 2);
            out.push_str("DispatchQueue.main.async {\n");
            indent(out, depth + 3);
            out.push_str(&format!("{} = true\n", state_name(focused)));
            indent(out, depth + 2);
            out.push('}');
        }
        indent(out, depth + 1);
        out.push('}');
    }
    if let Some(horizontal_padding) = horizontal_padding {
        out.push_str(&format!(
            "\n{}.padding(.horizontal, {horizontal_padding})",
            "    ".repeat(depth)
        ));
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
    use nexa_ir::{
        Action, Capitalization, Color, ColorValue, Expr, KeyboardType, ReturnKeyType,
        TextInputChange,
    };

    #[test]
    fn custom_input_style_and_clear_control_reach_the_native_field() {
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "query",
                placeholder: "Search songs",
                comment: None,
                keyboard: KeyboardType::Text,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: Some(ReturnKeyType::Search),
                autocorrect: Some(false),
                capitalization: Some(Capitalization::None),
                focused: None,
                max_length: Some(50),
                font: None,
                font_size: Some(16.38),
                font_family: Some("Gilroy-Regular"),
                text_color: Some(ColorValue::Static(Color {
                    red: 255,
                    green: 255,
                    blue: 255,
                    alpha: 255,
                })),
                placeholder_color: Some(ColorValue::Static(Color {
                    red: 159,
                    green: 159,
                    blue: 159,
                    alpha: 255,
                })),
                clearable: true,
                min_lines: None,
                max_lines: None,
                searchable: false,
                horizontal_padding: None,
                actions: &[],
                on_focus: None,
                on_change: None,
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains("fontSize: 16.38"));
        assert!(source.contains("fontFamily: \"Gilroy-Regular\""));
        assert!(source.contains("textColor: Color(red: 1.000000"));
        assert!(source.contains("placeholderColor: Color(red: 0.623529"));
        assert!(source.contains("clearable: true"));
    }

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
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: None,
                searchable: true,
                horizontal_padding: None,
                actions: &[],
                on_focus: None,
                on_change: None,
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains("NexaTextInputPrimitive(text: $nexa_query"));
        assert!(source.contains("searchable: true"));
        assert!(!source.contains("EmptyView().searchable"));
    }

    #[test]
    fn maximum_length_is_enforced_by_the_binding_setter() {
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "description",
                placeholder: "Description",
                comment: None,
                keyboard: KeyboardType::Text,
                secure: false,
                multiline: true,
                autofill: None,
                return_key: None,
                autocorrect: None,
                capitalization: None,
                focused: None,
                max_length: Some(1200),
                font: None,
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: Some(3),
                searchable: false,
                horizontal_padding: None,
                actions: &[],
                on_focus: None,
                on_change: None,
            },
            0,
            &mut output,
        );

        assert!(output.contains("NexaTextInputPrimitive(text: $nexa_description"));
        assert!(output.contains("maxLength: 1200"));
    }

    #[test]
    fn on_change_runs_from_native_input_binding_not_programmatic_state_changes() {
        let mut output = SourceWriter::new();
        let change = TextInputChange {
            parameter: "changed".to_owned(),
            actions: vec![],
        };
        render_text_input(
            TextInputProps {
                state: "value",
                placeholder: "Value",
                comment: None,
                keyboard: KeyboardType::Text,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: None,
                autocorrect: None,
                capitalization: None,
                focused: None,
                max_length: None,
                font: None,
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: None,
                searchable: false,
                horizontal_padding: None,
                actions: &[],
                on_focus: None,
                on_change: Some(&change),
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains(", onChange: { nexa_changed in"));
        assert!(!source.contains(".onChange(of:"));
    }

    #[test]
    fn focus_actions_run_only_when_a_tracked_text_input_gains_focus() {
        let focus_actions = [Action::Assign {
            name: "didFocus".to_owned(),
            value: Expr::Bool(true),
        }];
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "query",
                placeholder: "Search",
                comment: None,
                keyboard: KeyboardType::Text,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: None,
                autocorrect: None,
                capitalization: None,
                focused: Some("focused"),
                max_length: None,
                font: None,
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: None,
                searchable: false,
                horizontal_padding: None,
                actions: &[],
                on_focus: Some(&focus_actions),
                on_change: None,
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains(".focused($nexa_focused)"));
        assert!(source.contains(".onChange(of: nexa_focused) { isFocused in"));
        assert!(source.contains("if isFocused {") && source.contains("nexa_didFocus = true"));
    }

    #[test]
    fn next_return_key_keeps_the_uncontrolled_native_field_focused() {
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "email",
                placeholder: "Email",
                comment: None,
                keyboard: KeyboardType::Email,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: Some(ReturnKeyType::Next),
                autocorrect: None,
                capitalization: None,
                focused: None,
                max_length: None,
                font: None,
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: None,
                searchable: false,
                horizontal_padding: None,
                actions: &[nexa_ir::Action::Expression(nexa_ir::Expr::Bool(true))],
                on_focus: None,
                on_change: None,
            },
            0,
            &mut output,
        );

        assert!(output.as_str().contains("keepsFocusOnNext: true"));
        assert!(output.as_str().contains("onSubmitAction: {"));
        assert!(!output.as_str().contains(".onSubmit {"));
    }

    #[test]
    fn controlled_next_return_key_restores_its_focus_binding_after_submit() {
        let action = [nexa_ir::Action::Expression(nexa_ir::Expr::Bool(true))];
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "email",
                placeholder: "Email",
                comment: None,
                keyboard: KeyboardType::Email,
                secure: false,
                multiline: false,
                autofill: None,
                return_key: Some(ReturnKeyType::Next),
                autocorrect: None,
                capitalization: None,
                focused: Some("emailFocused"),
                max_length: None,
                font: None,
                font_size: None,
                font_family: None,
                text_color: None,
                placeholder_color: None,
                clearable: false,
                min_lines: None,
                max_lines: None,
                searchable: false,
                horizontal_padding: None,
                actions: &action,
                on_focus: None,
                on_change: None,
            },
            0,
            &mut output,
        );

        let source = output.as_str();
        assert!(source.contains("keepsFocusOnNext: false"));
        assert!(source.contains(".focused($nexa_emailFocused)"));
        assert!(source.contains("DispatchQueue.main.async {"));
        assert!(source.contains("nexa_emailFocused = true"));
    }
}
