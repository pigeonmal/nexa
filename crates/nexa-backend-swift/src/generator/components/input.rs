use nexa_codegen::SourceWriter;
use nexa_ir::{
    Action, AutofillType, Capitalization, ColorValue, Expr, KeyboardType, ReturnKeyType,
    TextInputChange, TextInputFont,
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
    pub(crate) focused: Option<&'a Expr>,
    pub(crate) component_ref: Option<&'a Expr>,
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
        component_ref,
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
    let placeholder = format!(
        "NexaLocale.localizedString(key: {}, value: {}, comment: {})",
        swift_string(placeholder),
        swift_string(placeholder),
        swift_string(comment.unwrap_or(""))
    );
    let optional_int =
        |value: Option<i32>| value.map_or_else(|| "nil".to_owned(), |value| value.to_string());
    let min_lines = optional_int(min_lines);
    let max_lines = optional_int(max_lines);
    let max_length = optional_int(max_length);
    let font_size = font_size.map_or_else(|| "nil".to_owned(), |size| size.to_string());
    let font_family = font_family.map_or_else(|| "nil".to_owned(), swift_string);
    let (font_relative_to, default_font_size) = match font {
        Some(TextInputFont::Title3) => (
            ".title3",
            nexa_codegen::design_system::TEXT_INPUT_TITLE3_FONT_SIZE,
        ),
        _ => (".body", nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE),
    };
    let text_color = text_color.map_or_else(|| "nil".to_owned(), colors::expression);
    let placeholder_color = placeholder_color.map_or_else(|| "nil".to_owned(), colors::expression);
    let keyboard = swift_keyboard(keyboard).unwrap_or(".default");
    let return_key = return_key.map_or(".default", swift_return_key);
    let autofill = autofill.map_or("nil", swift_autofill);
    let capitalization = match capitalization {
        Some(Capitalization::None) => ".none",
        Some(Capitalization::Words) => ".words",
        Some(Capitalization::Characters) => ".allCharacters",
        _ => ".sentences",
    };
    let autocorrect = autocorrect.map_or_else(|| "nil".to_owned(), |value| value.to_string());
    let initial_focus = focused.map_or_else(
        || "false".to_owned(),
        crate::generator::engine::expressions::expression,
    );
    let component_ref = component_ref.map_or_else(
        || "nil".to_owned(),
        crate::generator::engine::expressions::expression,
    );
    indent(out, depth);
    out.push_str(&format!(
        "NexaTextInputPrimitive(text: ${}, placeholder: {placeholder}, secure: {secure}, multiline: {multiline}, searchable: {searchable}, maxLength: {max_length}, fontSize: {font_size}, fontFamily: {font_family}, fontRelativeTo: {font_relative_to}, fontDefaultSize: {default_font_size}, textColor: {text_color}, placeholderColor: {placeholder_color}, clearable: {clearable}, minLines: {min_lines}, maxLines: {max_lines}, keyboard: {keyboard}, returnKey: {return_key}, autofill: {autofill}, autocorrect: {autocorrect}, capitalization: {capitalization}, initialFocus: {initial_focus}, componentRef: {component_ref}", state_name(state)
    ));
    if let Some(actions) = on_focus {
        out.push_str(", onFocusAction: {\n");
        render_actions(actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
    }
    if actions.is_empty() {
        out.push_str(", onSubmitAction: nil");
    } else {
        out.push_str(", onSubmitAction: {\n");
        render_actions(actions, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
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
                component_ref: None,
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
                component_ref: None,
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
                component_ref: None,
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
                component_ref: None,
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
    fn initial_focus_and_focus_event_are_separate_from_mutable_state() {
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
                focused: Some(&Expr::Bool(true)),
                component_ref: None,
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
        assert!(source.contains("initialFocus: true"));
        assert!(!source.contains("focusBinding"));
        assert!(source.contains("onFocusAction: {"));
        assert!(source.contains("nexa_didFocus = true"));
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
                component_ref: None,
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

        assert!(output.as_str().contains("returnKey: .next"));
        assert!(output.as_str().contains("onSubmitAction: {"));
        assert!(!output.as_str().contains(".onSubmit {"));
    }

    #[test]
    fn initial_focus_does_not_change_next_return_key_behavior() {
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
                focused: Some(&Expr::Bool(true)),
                component_ref: None,
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
        assert!(source.contains("returnKey: .next"));
        assert!(source.contains("initialFocus: true"));
        assert!(source.contains("onSubmitAction: {"));
        assert!(!source.contains("DispatchQueue.main.async"));
    }
}
