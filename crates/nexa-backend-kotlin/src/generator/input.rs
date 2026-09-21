use nexa_codegen::names::state_name;
use nexa_ir::{Capitalization, KeyboardType};

use super::utils::{indent, kotlin_string};

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
    out.push_str("TextField(\n");
    indent(out, depth + 1);
    out.push_str(&format!("value = {},\n", state_name(state)));
    indent(out, depth + 1);
    out.push_str(&format!(
        "onValueChange = {{ {} = it }},\n",
        state_name(state)
    ));
    indent(out, depth + 1);
    out.push_str(&format!(
        "placeholder = {{ Text({}) }},\n",
        kotlin_string(placeholder)
    ));
    indent(out, depth + 1);
    out.push_str(&format!("singleLine = {},\n", !multiline));
    indent(out, depth + 1);
    out.push_str("keyboardOptions = KeyboardOptions(\n");
    indent(out, depth + 2);
    out.push_str(&format!("keyboardType = {},\n", kotlin_keyboard(keyboard)));
    if let Some(capitalization) = capitalization {
        indent(out, depth + 2);
        out.push_str(&format!(
            "capitalization = {},\n",
            kotlin_capitalization(capitalization)
        ));
    }
    if let Some(autocorrect) = autocorrect {
        indent(out, depth + 2);
        out.push_str(&format!("autoCorrectEnabled = {autocorrect},\n"));
    }
    indent(out, depth + 1);
    out.push_str("),");
    if secure {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("visualTransformation = PasswordVisualTransformation(),");
    }
    out.push('\n');
    indent(out, depth);
    out.push(')');
}

pub(super) fn kotlin_keyboard(keyboard: KeyboardType) -> &'static str {
    match keyboard {
        KeyboardType::Text => "NativeKeyboardType.Text",
        KeyboardType::Number => "NativeKeyboardType.Number",
        KeyboardType::Email => "NativeKeyboardType.Email",
        KeyboardType::Phone => "NativeKeyboardType.Phone",
        KeyboardType::Url => "NativeKeyboardType.Uri",
    }
}

pub(super) fn kotlin_capitalization(capitalization: Capitalization) -> &'static str {
    match capitalization {
        Capitalization::None => "NativeKeyboardCapitalization.None",
        Capitalization::Sentences => "NativeKeyboardCapitalization.Sentences",
        Capitalization::Words => "NativeKeyboardCapitalization.Words",
        Capitalization::Characters => "NativeKeyboardCapitalization.Characters",
    }
}
