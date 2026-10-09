use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    Action, AutofillType, Capitalization, KeyboardType, ReturnKeyType, SystemIcon, TextInputChange,
    TextInputFont,
};

use crate::generator::{controls::render_actions, utils::indent};

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
    pub(crate) font: Option<TextInputFont>,
    pub(crate) min_lines: Option<i32>,
    pub(crate) max_lines: Option<i32>,
    pub(crate) searchable: bool,
    pub(crate) horizontal_padding: Option<u32>,
    pub(crate) weight_in_row: bool,
    pub(crate) actions: &'a [Action],
    pub(crate) on_change: Option<&'a TextInputChange>,
}

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_text_input,
        "androidx.compose.ui.res.stringResource",
    );
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
        features.uses_text_input_autofill
            || features.uses_focus
            || features.uses_text_input_searchable,
        "androidx.compose.ui.Modifier",
    );
    imports.add(
        features.uses_text_input_searchable,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_text_input_searchable,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_text_input_searchable,
        "androidx.compose.ui.unit.dp",
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
        font,
        min_lines,
        max_lines,
        searchable,
        horizontal_padding,
        weight_in_row,
        actions,
        on_change,
    } = props;
    indent(out, depth);
    out.push_str("NexaTextInputPrimitive(\n");
    out.line_at(depth + 1, format_args!("value = {},", state_name(state)));
    indent(out, depth + 1);
    out.push_str("onValueChange = { value ->\n");
    if let Some(change) = on_change {
        let updated_value = max_length
            .map(|limit| format!("value.take({limit})"))
            .unwrap_or_else(|| "value".to_owned());
        out.line_at(
            depth + 2,
            format_args!("val {} = {}", state_name(&change.parameter), updated_value),
        );
        out.line_at(
            depth + 2,
            format_args!("{} = {}", state_name(state), state_name(&change.parameter)),
        );
        render_actions(&change.actions, depth + 2, out);
    } else {
        out.line_at(
            depth + 2,
            format_args!(
                "{} = {}",
                state_name(state),
                max_length
                    .map(|limit| format!("value.take({limit})"))
                    .unwrap_or_else(|| "value".to_owned())
            ),
        );
    }
    out.line_at(depth + 1, format_args!("}},"));
    let placeholder = nexa_codegen::names::localization_resource_name(placeholder);
    out.line_at(
        depth + 1,
        format_args!("placeholder = stringResource(R.string.{placeholder}),"),
    );
    if searchable {
        let search_icon = SystemIcon::Shared("search".to_owned()).material_reference();
        out.line_at(
            depth + 1,
            format_args!(
                "searchable = true,\n{}searchIcon = {search_icon},",
                "    ".repeat(depth + 1)
            ),
        );
    } else {
        out.line_at(depth + 1, format_args!("searchable = false,"));
    }
    out.line_at(depth + 1, format_args!("secure = {secure},"));
    if weight_in_row || focused.is_some() || autofill.is_some() || horizontal_padding.is_some() {
        indent(out, depth + 1);
        out.push_str("modifier = Modifier");
        if let Some(horizontal_padding) = horizontal_padding {
            out.push_str(&format!(
                ".fillMaxWidth().padding(horizontal = {horizontal_padding}.dp)"
            ));
        }
        if weight_in_row {
            out.push_str(".weight(1f)");
        }
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
    if let Some(font) = font {
        let style = match font {
            TextInputFont::Body => "androidx.compose.material3.MaterialTheme.typography.bodyLarge",
            TextInputFont::Title3 => {
                "androidx.compose.material3.MaterialTheme.typography.titleMedium"
            }
        };
        out.line_at(depth + 1, format_args!("textStyle = {style},"));
    }
    if let Some(min_lines) = min_lines {
        out.line_at(depth + 1, format_args!("minLines = {min_lines},"));
    }
    if let Some(max_lines) = max_lines {
        out.line_at(depth + 1, format_args!("maxLines = {max_lines},"));
    }
    out.line_at(depth + 1, format_args!("singleLine = {},", !multiline));
    out.line_at(
        depth + 1,
        format_args!("keyboardOptions = KeyboardOptions("),
    );
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
    let effective_return_key = return_key.or_else(|| {
        if searchable {
            Some(ReturnKeyType::Search)
        } else {
            (!actions.is_empty()).then_some(ReturnKeyType::Done)
        }
    });
    if let Some(return_key) = effective_return_key {
        out.line_at(
            depth + 2,
            format_args!("imeAction = {},", kotlin_return_key(return_key)),
        );
    }
    out.line_at(depth + 1, format_args!("),"));
    if !actions.is_empty() {
        let callback = match effective_return_key.unwrap_or(ReturnKeyType::Done) {
            ReturnKeyType::Done => "onDone",
            ReturnKeyType::Search => "onSearch",
            ReturnKeyType::Send => "onSend",
            ReturnKeyType::Next => "onNext",
        };
        out.line_at(
            depth + 1,
            format_args!("keyboardActions = KeyboardActions("),
        );
        out.line_at(depth + 2, format_args!("{callback} = {{"));
        render_actions(actions, depth + 3, out);
        out.line_at(depth + 2, format_args!("}},"));
        out.line_at(depth + 1, format_args!("),"));
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

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::KeyboardType;

    use super::{TextInputProps, render_text_input};

    #[test]
    fn searchable_input_emits_a_native_search_affordance_and_ime_action() {
        let mut output = SourceWriter::new();
        render_text_input(
            TextInputProps {
                state: "query",
                placeholder: "Search items",
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
                min_lines: None,
                max_lines: None,
                searchable: true,
                horizontal_padding: None,
                weight_in_row: false,
                actions: &[],
                on_change: None,
            },
            0,
            &mut output,
        );

        assert!(output.as_str().contains("NexaTextInputPrimitive("));
        assert!(output.as_str().contains("searchable = true"));
        assert!(output.as_str().contains("searchIcon = Icons.Filled.Search"));
        assert!(output.as_str().contains("imeAction = ImeAction.Search"));
    }
}
