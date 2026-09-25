use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, Capitalization, KeyboardType};

use crate::generator::{
    controls::render_actions,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_text_input,
        "androidx.compose.foundation.text.KeyboardOptions",
    );
    imports.add(
        features.uses_text_input_submit,
        "androidx.compose.foundation.text.KeyboardActions",
    );
    imports.add(
        features.uses_focus,
        "androidx.compose.ui.focus.FocusRequester",
    );
    imports.add(
        features.uses_focus,
        "androidx.compose.ui.focus.focusRequester",
    );
    imports.add(
        features.uses_focus,
        "androidx.compose.ui.focus.onFocusChanged",
    );
    imports.add(
        features.uses_focus,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(
        features.uses_capitalization,
        "androidx.compose.ui.text.input.KeyboardCapitalization as NativeKeyboardCapitalization",
    );
    imports.add(
        features.uses_text_input,
        "androidx.compose.ui.text.input.KeyboardType as NativeKeyboardType",
    );
    imports.add(
        features.uses_text_input_submit,
        "androidx.compose.ui.text.input.ImeAction",
    );
    imports.add(
        features.uses_secure_text_input,
        "androidx.compose.ui.text.input.PasswordVisualTransformation",
    );
    imports.add(
        features.uses_text_input,
        "androidx.compose.material3.TextField",
    );
    imports.add(features.uses_text_input, "androidx.compose.material3.Text");
}

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
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("TextField(\n");
    out.line_at(depth + 1, format_args!("value = {},", state_name(state)));
    out.line_at(
        depth + 1,
        format_args!(
            "onValueChange = {{ value -> {} = {} }},",
            state_name(state),
            max_length
                .map(|limit| format!("value.take({limit})"))
                .unwrap_or_else(|| "value".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("placeholder = {{ Text({}) }},", kotlin_string(placeholder)),
    );
    if let Some(focused) = focused {
        indent(out, depth + 1);
        out.push_str("modifier = Modifier\n");
        out.line_at(
            depth + 2,
            format_args!(".focusRequester({})", focus_requester_name(focused)),
        );
        out.line_at(
            depth + 2,
            format_args!(
                ".onFocusChanged {{ {} = it.isFocused }},",
                state_name(focused)
            ),
        );
    }
    out.line_at(depth + 1, format_args!("singleLine = {},", !multiline));
    indent(out, depth + 1);
    out.push_str("keyboardOptions = KeyboardOptions(\n");
    out.line_at(
        depth + 2,
        format_args!("keyboardType = {},", kotlin_keyboard(keyboard)),
    );
    if let Some(capitalization) = capitalization {
        out.line_at(
            depth + 2,
            format_args!(
                "capitalization = {},",
                kotlin_capitalization(capitalization)
            ),
        );
    }
    if let Some(autocorrect) = autocorrect {
        out.line_at(
            depth + 2,
            format_args!("autoCorrectEnabled = {autocorrect},"),
        );
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

pub(crate) fn focus_requester_name(name: &str) -> String {
    format!("{}Requester", state_name(name))
}

pub(crate) fn kotlin_keyboard(keyboard: KeyboardType) -> &'static str {
    match keyboard {
        KeyboardType::Text => "NativeKeyboardType.Text",
        KeyboardType::Number => "NativeKeyboardType.Number",
        KeyboardType::Email => "NativeKeyboardType.Email",
        KeyboardType::Phone => "NativeKeyboardType.Phone",
        KeyboardType::Url => "NativeKeyboardType.Uri",
    }
}

pub(crate) fn kotlin_capitalization(capitalization: Capitalization) -> &'static str {
    match capitalization {
        Capitalization::None => "NativeKeyboardCapitalization.None",
        Capitalization::Sentences => "NativeKeyboardCapitalization.Sentences",
        Capitalization::Words => "NativeKeyboardCapitalization.Words",
        Capitalization::Characters => "NativeKeyboardCapitalization.Characters",
    }
}
