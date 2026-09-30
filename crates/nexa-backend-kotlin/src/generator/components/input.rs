use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, AutofillType, Capitalization, KeyboardType, ReturnKeyType};

use crate::generator::{
    controls::render_actions,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

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
    imports.add(
        features.uses_text_input_autofill || features.uses_focus,
        "androidx.compose.ui.Modifier",
    );
    imports.add(
        features.uses_text_input_autofill,
        "androidx.compose.ui.autofill.ContentType",
    );
    imports.add(
        features.uses_text_input_autofill,
        "androidx.compose.ui.semantics.contentType",
    );
    imports.add(
        features.uses_text_input_autofill,
        "androidx.compose.ui.semantics.semantics",
    );
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
    if focused.is_some() || autofill.is_some() {
        indent(out, depth + 1);
        out.push_str("modifier = Modifier");
        if let Some(focused) = focused {
            out.push('\n');
            indent(out, depth + 2);
            out.push_str(&format!(
                ".focusRequester({})",
                focus_requester_name(focused)
            ));
            out.push('\n');
            indent(out, depth + 2);
            out.push_str(&format!(
                ".onFocusChanged {{ {} = it.isFocused }}",
                state_name(focused)
            ));
        }
        if let Some(autofill) = autofill {
            out.push('\n');
            indent(out, depth + 2);
            out.push_str(&format!(
                ".semantics {{ contentType = {} }}",
                kotlin_autofill(autofill)
            ));
        }
        out.push_str(",\n");
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
    let effective_return_key =
        return_key.or_else(|| (!actions.is_empty()).then_some(ReturnKeyType::Done));
    if let Some(return_key) = effective_return_key {
        out.line_at(
            depth + 2,
            format_args!("imeAction = {},", kotlin_return_key(return_key)),
        );
    }
    indent(out, depth + 1);
    out.push_str("),");
    if secure {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("visualTransformation = PasswordVisualTransformation(),");
    }
    if !actions.is_empty() {
        let callback = match effective_return_key.unwrap_or(ReturnKeyType::Done) {
            ReturnKeyType::Done => "onDone",
            ReturnKeyType::Search => "onSearch",
            ReturnKeyType::Send => "onSend",
            ReturnKeyType::Next => "onNext",
        };
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("keyboardActions = KeyboardActions(\n");
        indent(out, depth + 2);
        out.push_str(&format!("{callback} = {{\n"));
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

pub(crate) fn kotlin_autofill(autofill: AutofillType) -> &'static str {
    match autofill {
        AutofillType::Username => "ContentType.Username",
        AutofillType::Password => "ContentType.Password",
        AutofillType::OneTimeCode => "ContentType.SmsOtpCode",
    }
}

pub(crate) fn kotlin_return_key(return_key: ReturnKeyType) -> &'static str {
    match return_key {
        ReturnKeyType::Done => "ImeAction.Done",
        ReturnKeyType::Search => "ImeAction.Search",
        ReturnKeyType::Send => "ImeAction.Send",
        ReturnKeyType::Next => "ImeAction.Next",
    }
}
