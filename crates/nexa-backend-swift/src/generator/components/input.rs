use nexa_ir::{Action, Capitalization, KeyboardType};

use nexa_codegen::names::state_name;

use crate::generator::{
    controls::render_actions,
    utils::{indent, swift_string},
};

pub(crate) fn render_text_input(
    state: &str,
    placeholder: &str,
    keyboard: KeyboardType,
    secure: bool,
    multiline: bool,
    autocorrect: Option<bool>,
    capitalization: Option<Capitalization>,
    focused: Option<&str>,
    max_length: Option<i32>,
    actions: &[Action],
    depth: usize,
    out: &mut String,
) {
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
        indent(out, depth + 2);
        out.push_str(&format!(
            "if newValue.count > {} {{ {} = String(newValue.prefix(Int({}))) }}\n",
            max_length,
            state_name(state),
            max_length
        ));
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
