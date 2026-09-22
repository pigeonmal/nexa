use nexa_codegen::names::state_name;
use nexa_ir::{Action, Capitalization, KeyboardType};

use super::{
    controls::render_actions,
    utils::{indent, kotlin_string},
};

pub(super) fn render_text_input(
    state: &str,
    placeholder: &str,
    keyboard: KeyboardType,
    secure: bool,
    multiline: bool,
    autocorrect: Option<bool>,
    capitalization: Option<Capitalization>,
    focused: Option<&str>,
    actions: &[Action],
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
    if let Some(focused) = focused {
        indent(out, depth + 1);
        out.push_str("modifier = Modifier\n");
        indent(out, depth + 2);
        out.push_str(&format!(
            ".focusRequester({})\n",
            focus_requester_name(focused)
        ));
        indent(out, depth + 2);
        out.push_str(&format!(
            ".onFocusChanged {{ {} = it.isFocused }},\n",
            state_name(focused)
        ));
    }
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
    if !actions.is_empty() {
        indent(out, depth + 2);
        out.push_str("imeAction = ImeAction.Done,\n");
    }
    indent(out, depth + 1);
    out.push_str("),");
    if secure {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("visualTransformation = PasswordVisualTransformation(),");
    }
    if !actions.is_empty() {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("keyboardActions = KeyboardActions(\n");
        indent(out, depth + 2);
        out.push_str("onDone = {\n");
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("},\n");
        indent(out, depth + 1);
        out.push_str("),");
    }
    out.push('\n');
    indent(out, depth);
    out.push(')');
}

pub(super) fn focus_requester_name(name: &str) -> String {
    format!("{}Requester", state_name(name))
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
