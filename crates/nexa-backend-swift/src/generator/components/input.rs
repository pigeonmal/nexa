use nexa_codegen::SourceWriter;
use nexa_ir::{Action, AutofillType, Capitalization, KeyboardType, ReturnKeyType};

use nexa_codegen::names::state_name;

use crate::generator::{
    controls::render_actions,
    utils::{indent, swift_string},
};

pub(crate) struct TextInputProps<'a> {
    pub(crate) state: &'a str,
    pub(crate) placeholder: &'a str,
    pub(crate) keyboard: KeyboardType,
    pub(crate) secure: bool,
    pub(crate) multiline: bool,
    pub(crate) autofill: Option<AutofillType>,
    pub(crate) return_key: Option<ReturnKeyType>,
    pub(crate) autocorrect: Option<bool>,
    pub(crate) capitalization: Option<Capitalization>,
    pub(crate) focused: Option<&'a str>,
    pub(crate) max_length: Option<i32>,
    pub(crate) actions: &'a [Action],
}

pub(crate) fn render_text_input(props: TextInputProps<'_>, depth: usize, out: &mut SourceWriter) {
    let TextInputProps {
        state,
        placeholder,
        keyboard,
        secure,
        multiline,
        autofill,
        return_key,
        autocorrect,
        capitalization,
        focused,
        max_length,
        actions,
    } = props;
    indent(out, depth);
    let control = if secure { "SecureField" } else { "TextField" };
    if multiline {
        out.push_str(&format!(
            "TextField({}, text: ${}, axis: .vertical)",
            swift_string(placeholder),
            state_name(state)
        ));
    } else {
        out.push_str(&format!(
            "{control}({}, text: ${})",
            swift_string(placeholder),
            state_name(state)
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
