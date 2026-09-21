use nexa_ir::{Capitalization, KeyboardType};

use nexa_codegen::names::state_name;

use super::utils::{indent, swift_string};

pub(super) fn render_text_input(
    state: &str,
    placeholder: &str,
    keyboard: KeyboardType,
    secure: bool,
    multiline: bool,
    autocorrect: Option<bool>,
    capitalization: Option<Capitalization>,
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
            "\n{}  .keyboardType({keyboard})",
            "    ".repeat(depth)
        ));
    }
    if let Some(capitalization) = capitalization {
        out.push_str(&format!(
            "\n{}  .textInputAutocapitalization({})",
            "    ".repeat(depth),
            swift_capitalization(capitalization)
        ));
    }
    if let Some(autocorrect) = autocorrect {
        out.push_str(&format!(
            "\n{}  .autocorrectionDisabled({})",
            "    ".repeat(depth),
            !autocorrect
        ));
    }
}

pub(super) fn swift_keyboard(keyboard: KeyboardType) -> Option<&'static str> {
    match keyboard {
        KeyboardType::Text => None,
        KeyboardType::Number => Some(".numberPad"),
        KeyboardType::Email => Some(".emailAddress"),
        KeyboardType::Phone => Some(".phonePad"),
        KeyboardType::Url => Some(".URL"),
    }
}

pub(super) fn swift_capitalization(capitalization: Capitalization) -> &'static str {
    match capitalization {
        Capitalization::None => ".never",
        Capitalization::Sentences => ".sentences",
        Capitalization::Words => ".words",
        Capitalization::Characters => ".characters",
    }
}
