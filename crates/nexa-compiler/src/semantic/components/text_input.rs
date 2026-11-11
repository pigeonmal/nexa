//! Typed text_input component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_text_input_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let themes = cx.themes;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "TextInput")?;
    let value = take_required_arg(&mut args, &inv.name, "value", span)?;
    let placeholder = take_required_arg(&mut args, &inv.name, "placeholder", span)?;
    let keyboard = args.remove("keyboardType");
    let secure = args.remove("isSecure");
    let autofill = args.remove("autofill");
    let return_key = args.remove("returnKeyType");
    let multiline = args.remove("multiline");
    let autocorrect = args.remove("autocorrect");
    let capitalization = args.remove("capitalization");
    let focused = args.remove("focused");
    let component_ref = args.remove("ref");
    let max_length = args.remove("maxLength");
    let font = lower_text_input_font(args.remove("font"))?;
    let font_size = optional_dimension(
        args.remove("fontSize"),
        "TextInput fontSize",
        Some(ast::ThemeTokenKind::FontSize),
        themes,
    )?;
    let font_family = lower_text_font_family(args.remove("fontFamily"), themes)?;
    let text_color = optional_color(args.remove("textColor"), "TextInput textColor", themes)?;
    let placeholder_color = optional_color(
        args.remove("placeholderColor"),
        "TextInput placeholderColor",
        themes,
    )?;
    let clearable = optional_bool(args.remove("clearable"), false, "clearable")?;
    let min_lines = lower_positive_integer(args.remove("minLines"), "TextInput minLines")?;
    let max_lines = lower_positive_integer(args.remove("maxLines"), "TextInput maxLines")?;
    let searchable = optional_bool(args.remove("searchable"), false, "searchable")?;
    let mut modifiers = inv.modifiers;
    let on_change = match take_modifier(&mut modifiers, "onChange") {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => {
            if parameters.len() != 1 {
                return Err(CompileError::new(
                    span,
                    ".onChange requires one String binding",
                ));
            }
            let Some(parameter) = parameters.into_iter().next() else {
                return Err(CompileError::new(
                    span,
                    ".onChange requires one String binding",
                ));
            };
            if symbols.contains_key(&parameter) {
                return Err(CompileError::new(
                    span,
                    format!(".onChange binding `{parameter}` shadows an existing value"),
                ));
            }
            let mut callback_symbols = symbols.clone();
            callback_symbols.insert(parameter.clone(), (Type::String, false));
            Some(nexa_ir::TextInputChange {
                parameter,
                actions: lower_actions_with_aliases(
                    actions,
                    &callback_symbols,
                    functions,
                    false,
                    native_aliases,
                    cx.type_registries(),
                )?,
            })
        }
        Some(_) => return Err(child_mismatch(span)),
        None => None,
    };
    let on_focus = match take_modifier(&mut modifiers, "onFocus") {
        Some(ast::ModifierBody::Actions(actions)) => Some(lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?),
        Some(_) => return Err(child_mismatch(span)),
        None => None,
    };
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    let state = require_mutable_binding(&value, &Type::String, symbols, span, "TextInput")?;
    let placeholder = require_string_literal(&placeholder, "TextInput placeholder")?;
    let keyboard = match keyboard {
        Some(ast::Expr::Name(name, _)) => match name.as_str() {
            "Text" | "text" => KeyboardType::Text,
            "Number" | "number" => KeyboardType::Number,
            "Email" | "email" => KeyboardType::Email,
            "Phone" | "phone" => KeyboardType::Phone,
            "Url" | "url" => KeyboardType::Url,
            _ => {
                return Err(CompileError::new(
                    span,
                    format!(
                        "unknown keyboard type `{name}`; expected Text, Number, Email, Phone, or Url"
                    ),
                ));
            }
        },
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "keyboard type must be a keyboard type name",
            ));
        }
        None => KeyboardType::Text,
    };
    let autofill = match autofill {
        Some(ast::Expr::Name(name, name_span)) => Some(match name.as_str() {
            "Username" | "username" => AutofillType::Username,
            "Password" | "password" => AutofillType::Password,
            "OneTimeCode" | "oneTimeCode" | "one_time_code" => AutofillType::OneTimeCode,
            _ => {
                return Err(CompileError::new(
                    name_span,
                    format!(
                        "unknown autofill type `{name}`; expected Username, Password, or OneTimeCode"
                    ),
                ));
            }
        }),
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "autofill type must be a supported autofill name",
            ));
        }
        None => None,
    };
    let return_key = match return_key {
        Some(ast::Expr::Name(name, name_span)) => Some(match name.as_str() {
            "Done" | "done" => ReturnKeyType::Done,
            "Search" | "search" => ReturnKeyType::Search,
            "Send" | "send" => ReturnKeyType::Send,
            "Next" | "next" => ReturnKeyType::Next,
            _ => {
                return Err(CompileError::new(
                    name_span,
                    format!(
                        "unknown return key type `{name}`; expected Done, Search, Send, or Next"
                    ),
                ));
            }
        }),
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "returnKeyType must be a return key name",
            ));
        }
        None => None,
    };
    let secure = optional_bool(secure, false, "secure")?;
    let multiline = optional_bool(multiline, false, "multiline")?;
    let autocorrect = optional_bool_override(autocorrect, "autocorrect")?;
    let capitalization = match capitalization {
        Some(ast::Expr::Name(name, name_span)) => Some(match name.as_str() {
            "None" => Capitalization::None,
            "Sentences" => Capitalization::Sentences,
            "Words" => Capitalization::Words,
            "Characters" => Capitalization::Characters,
            _ => {
                return Err(CompileError::new(
                    name_span,
                    format!(
                        "unknown capitalization `{name}`; expected None, Sentences, Words, or Characters"
                    ),
                ));
            }
        }),
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "capitalization must be a capitalization choice name",
            ));
        }
        None => None,
    };
    if secure && multiline {
        return Err(CompileError::new(
            span,
            "TextInput cannot be both secure and multiline",
        ));
    }
    if searchable && (secure || multiline || !actions.is_empty()) {
        return Err(CompileError::new(
            span,
            "searchable TextInput must be a single-line, non-secure field without submit actions",
        ));
    }
    if multiline && !actions.is_empty() {
        return Err(CompileError::new(
            span,
            "TextInput submit actions require a single-line field",
        ));
    }
    let focused = focused
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let component_ref = component_ref
        .map(|reference| {
            lower_expr(
                &reference,
                Some(&Type::ComponentRef {
                    namespace: "Nexa".to_owned(),
                    name: "TextInput".to_owned(),
                }),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    let max_length = lower_positive_integer(max_length, "TextInput maxLength")?;
    if min_lines
        .zip(max_lines)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(CompileError::new(
            span,
            "TextInput minLines cannot be greater than maxLines",
        ));
    }
    if !multiline && (min_lines.is_some() || max_lines.is_some()) {
        return Err(CompileError::new(
            span,
            "TextInput minLines and maxLines require `multiline: true`",
        ));
    }
    Ok(Node::TextInput {
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
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?,
        on_focus,
        on_change,
    })
}
