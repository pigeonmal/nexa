use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{
    Action, Capitalization, Expr, ImageScale, ImageSource, KeyboardType, LayoutKind, ListSource,
    Node, NumericType, ScreenId, StatusBarConfig, StatusBarStyle, TextStyle, Type,
};
use nexa_syntax::ast;

use super::{
    custom_components::ComponentSignatures,
    expressions::{lower_expr, type_name},
    styles::{lower_style, optional_color, optional_dimension},
    themes::ThemeSymbols,
};
use crate::Target;

pub(super) fn lower_nodes(
    nodes: Vec<ast::Node>,
    symbols: &HashMap<String, (Type, bool)>,
    screen_ids: &HashMap<String, ScreenId>,
    themes: &ThemeSymbols,
    components: &ComponentSignatures,
    allow_navigation_stack: bool,
    target: Target,
) -> Result<Vec<Node>, CompileError> {
    let mut lowered = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            ast::Node::Platform {
                target: platform,
                children,
                ..
            } => {
                if target == Target::All || platform_matches(platform, target) {
                    lowered.extend(lower_nodes(
                        children,
                        symbols,
                        screen_ids,
                        themes,
                        components,
                        allow_navigation_stack,
                        target,
                    )?);
                }
            }
            node => {
                let allow_navigation =
                    allow_navigation_stack && matches!(node, ast::Node::NavigationStack { .. });
                lowered.push(lower_node(
                    node,
                    symbols,
                    screen_ids,
                    themes,
                    components,
                    allow_navigation,
                    target,
                )?);
            }
        }
    }
    Ok(lowered)
}

fn platform_matches(platform: ast::PlatformTarget, target: Target) -> bool {
    matches!(
        (platform, target),
        (ast::PlatformTarget::Ios, Target::Swift) | (ast::PlatformTarget::Android, Target::Kotlin)
    )
}

pub(super) fn lower_node(
    node: ast::Node,
    symbols: &HashMap<String, (Type, bool)>,
    screen_ids: &HashMap<String, ScreenId>,
    themes: &ThemeSymbols,
    components: &ComponentSignatures,
    allow_navigation_stack: bool,
    target: Target,
) -> Result<Node, CompileError> {
    match node {
        ast::Node::Platform { .. } => {
            unreachable!("platform blocks are expanded by lower_nodes")
        }
        ast::Node::StatusBar { style, hidden, .. } => Ok(Node::StatusBar {
            config: lower_status_bar(style, hidden)?,
        }),
        ast::Node::Layout {
            kind,
            spacing,
            style,
            children,
            span: _,
        } => {
            let spacing = optional_dimension(
                spacing,
                "spacing",
                Some(ast::ThemeTokenKind::Spacing),
                themes,
            )?
            .unwrap_or(0.0);
            let style = lower_style(style, themes)?;
            let lowered = lower_nodes(
                children, symbols, screen_ids, themes, components, false, target,
            )?;
            let kind = match kind {
                ast::LayoutKind::Column => LayoutKind::Column,
                ast::LayoutKind::Row => LayoutKind::Row,
            };
            Ok(Node::Layout {
                kind,
                spacing,
                style,
                children: lowered,
            })
        }
        ast::Node::Text {
            value,
            color,
            font_size,
            ..
        } => {
            let value = lower_expr(&value, None, symbols)?;
            let color = optional_color(color, "text color", themes)?;
            let font_size = optional_dimension(
                font_size,
                "fontSize",
                Some(ast::ThemeTokenKind::FontSize),
                themes,
            )?;
            Ok(Node::Text {
                value,
                style: TextStyle { color, font_size },
            })
        }
        ast::Node::Button {
            label,
            actions,
            span,
        } => {
            let label = lower_expr(&label, Some(&Type::String), symbols)?;
            if !matches!(
                label,
                Expr::String(_) | Expr::Interpolation(_) | Expr::State(_, Type::String)
            ) {
                return Err(CompileError::new(span, "Button label must be a String"));
            }
            let lowered = lower_actions(actions, symbols)?;
            Ok(Node::Button {
                label,
                actions: lowered,
            })
        }
        ast::Node::TextInput {
            value,
            placeholder,
            keyboard,
            secure,
            multiline,
            autocorrect,
            capitalization,
            span,
        } => {
            let state = require_mutable_binding(&value, &Type::String, symbols, span, "TextInput")?;
            let placeholder = require_string_literal(&placeholder, "TextInput placeholder")?;
            let keyboard = match keyboard {
                Some(ast::Expr::Name(name, _)) => match name.as_str() {
                    "Text" => KeyboardType::Text,
                    "Number" => KeyboardType::Number,
                    "Email" => KeyboardType::Email,
                    "Phone" => KeyboardType::Phone,
                    "Url" => KeyboardType::Url,
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
            Ok(Node::TextInput {
                state,
                placeholder,
                keyboard,
                secure,
                multiline,
                autocorrect,
                capitalization,
            })
        }
        ast::Node::Switch { value, label, span } => {
            let state = require_mutable_binding(&value, &Type::Bool, symbols, span, "Switch")?;
            let label = require_string_literal(&label, "Switch label")?;
            Ok(Node::Switch { state, label })
        }
        ast::Node::Image {
            source,
            description,
            scale,
            placeholder,
            span,
        } => {
            let source = match source {
                ast::ImageSource::Asset(asset) => {
                    let asset = require_string_literal(&asset, "Image asset")?;
                    if !is_resource_name(&asset) {
                        return Err(CompileError::new(
                            span,
                            "Image asset must be an identifier suitable for an Android drawable resource",
                        ));
                    }
                    ImageSource::Asset(asset)
                }
                ast::ImageSource::Url(url) => {
                    let url = require_string_literal(&url, "Image URL")?;
                    if !is_https_url(&url) {
                        return Err(CompileError::new(
                            span,
                            "Image URL must be an absolute HTTPS URL",
                        ));
                    }
                    ImageSource::RemoteUrl(url)
                }
            };
            let description = require_string_literal(&description, "Image description")?;
            let scale = match scale {
                Some(ast::Expr::Name(name, _)) if name == "Fit" => ImageScale::Fit,
                Some(ast::Expr::Name(name, _)) if name == "Fill" => ImageScale::Fill,
                Some(expr) => {
                    return Err(CompileError::new(
                        expr.span(),
                        "Image scale must be `Fit` or `Fill`",
                    ));
                }
                None => ImageScale::Fit,
            };
            let placeholder = match placeholder {
                Some(expr) => {
                    if !matches!(&source, ImageSource::RemoteUrl(_)) {
                        return Err(CompileError::new(
                            expr.span(),
                            "Image `placeholder` can only be used with a remote URL",
                        ));
                    }
                    let placeholder = require_string_literal(&expr, "Image placeholder")?;
                    if !is_resource_name(&placeholder) {
                        return Err(CompileError::new(
                            expr.span(),
                            "Image placeholder must be an identifier suitable for a drawable resource",
                        ));
                    }
                    Some(placeholder)
                }
                None => None,
            };
            Ok(Node::Image {
                source,
                description,
                scale,
                placeholder,
            })
        }
        ast::Node::Pressable {
            disabled,
            children,
            actions,
            ..
        } => {
            let disabled = optional_bool(disabled, false, "disabled")?;
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, false, target,
            )?;
            let actions = lower_actions(actions, symbols)?;
            Ok(Node::Pressable {
                disabled,
                children: lowered_children,
                actions,
            })
        }
        ast::Node::NavigationStack { root, span } => {
            if !allow_navigation_stack {
                return Err(CompileError::new(
                    span,
                    "NavigationStack is only allowed as the app's top-level body",
                ));
            }
            let root = resolve_screen(&root, screen_ids, "NavigationStack root")?;
            Ok(Node::NavigationStack { root })
        }
        ast::Node::NavigationLink {
            destination,
            children,
            span,
        } => {
            let destination =
                resolve_screen(&destination, screen_ids, "NavigationLink destination").map_err(
                    |error| {
                        if screen_ids.is_empty() {
                            CompileError::new(span, "NavigationLink requires declared app screens")
                        } else {
                            error
                        }
                    },
                )?;
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, false, target,
            )?;
            Ok(Node::NavigationLink {
                destination,
                children: lowered_children,
            })
        }
        ast::Node::KeyboardAware { children, .. } => {
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, false, target,
            )?;
            Ok(Node::KeyboardAware {
                children: lowered_children,
            })
        }
        ast::Node::BottomSheet {
            is_presented,
            children,
            span,
        } => {
            let state =
                require_mutable_binding(&is_presented, &Type::Bool, symbols, span, "BottomSheet")?;
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, false, target,
            )?;
            Ok(Node::BottomSheet {
                state,
                children: lowered_children,
            })
        }
        ast::Node::FastList {
            source,
            index,
            item,
            children,
            span,
        } => {
            let row_index_type = Type::Numeric(NumericType::Int32);
            let index = binding_name(index, "index", "FastList index")?;
            let (source, item, item_type) = match source {
                ast::ListSource::Count(count) => {
                    if item.is_some() {
                        return Err(CompileError::new(
                            span,
                            "FastList `item` is only available with an `items` source",
                        ));
                    }
                    let count_value = lower_expr(&count, Some(&row_index_type), symbols)?;
                    if let ast::Expr::Number(raw, count_span) = &count {
                        if raw.parse::<i32>().is_ok_and(|value| value < 0) {
                            return Err(CompileError::new(
                                *count_span,
                                "FastList count must be non-negative",
                            ));
                        }
                    }
                    (ListSource::Count(count_value), None, None)
                }
                ast::ListSource::Items(collection) => {
                    let ast::Expr::Name(name, name_span) = collection else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList `items` must be an Array<T> binding",
                        ));
                    };
                    let Some((ty, _)) = symbols.get(&name) else {
                        return Err(CompileError::new(
                            name_span,
                            format!("unknown state `{name}`"),
                        ));
                    };
                    let Type::Array(element_type) = ty else {
                        return Err(CompileError::new(
                            name_span,
                            format!("FastList items `{name}` must have type Array<T>"),
                        ));
                    };
                    let item_name = binding_name(item, "item", "FastList item")?;
                    if item_name == index {
                        return Err(CompileError::new(
                            name_span,
                            "FastList item and index bindings must have different names",
                        ));
                    }
                    (
                        ListSource::Items {
                            collection: Expr::State(name, ty.clone()),
                            element_type: (**element_type).clone(),
                        },
                        Some(item_name),
                        Some((**element_type).clone()),
                    )
                }
            };
            let mut row_symbols = symbols.clone();
            row_symbols.insert(index.clone(), (row_index_type, false));
            if let (Some(item_name), Some(item_type)) = (&item, &item_type) {
                row_symbols.insert(item_name.clone(), (item_type.clone(), false));
            }
            let lowered_children = lower_nodes(
                children,
                &row_symbols,
                screen_ids,
                themes,
                components,
                false,
                target,
            )?;
            if lowered_children.is_empty() {
                return Err(CompileError::new(
                    span,
                    "FastList requires at least one row component",
                ));
            }
            Ok(Node::FastList {
                source,
                index,
                item,
                children: lowered_children,
            })
        }
        ast::Node::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            let condition = lower_expr(&condition, Some(&Type::Bool), symbols)?;
            let lowered_then = lower_nodes(
                then_body, symbols, screen_ids, themes, components, false, target,
            )?;
            let lowered_else = else_body
                .map(|body| {
                    lower_nodes(body, symbols, screen_ids, themes, components, false, target)
                })
                .transpose()?;
            Ok(Node::If {
                condition,
                then_body: lowered_then,
                else_body: lowered_else,
            })
        }
        ast::Node::ComponentCall {
            name,
            mut arguments,
            span,
        } => {
            let signature = components
                .get(&name)
                .ok_or_else(|| CompileError::new(span, format!("unknown component `{name}`")))?;
            for (argument_name, value) in &arguments {
                if !signature
                    .parameters
                    .iter()
                    .any(|(parameter_name, _)| parameter_name == argument_name)
                {
                    return Err(CompileError::new(
                        value.span(),
                        format!("component `{name}` has no parameter `{argument_name}`"),
                    ));
                }
            }
            let mut lowered_arguments = Vec::with_capacity(signature.parameters.len());
            for (parameter, ty) in &signature.parameters {
                let argument = arguments.remove(parameter).ok_or_else(|| {
                    CompileError::new(
                        span,
                        format!("component `{name}` requires parameter `{parameter}`"),
                    )
                })?;
                lowered_arguments
                    .push((parameter.clone(), lower_expr(&argument, Some(ty), symbols)?));
            }
            Ok(Node::ComponentCall {
                name,
                arguments: lowered_arguments,
            })
        }
    }
}

fn lower_status_bar(
    style: Option<ast::Expr>,
    hidden: Option<ast::Expr>,
) -> Result<StatusBarConfig, CompileError> {
    let style = match style {
        Some(ast::Expr::Name(name, name_span)) => match name.as_str() {
            "Default" => StatusBarStyle::Default,
            "Light" => StatusBarStyle::Light,
            "Dark" => StatusBarStyle::Dark,
            _ => {
                return Err(CompileError::new(
                    name_span,
                    "StatusBar style must be `Default`, `Light`, or `Dark`",
                ));
            }
        },
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "StatusBar style must be `Default`, `Light`, or `Dark`",
            ));
        }
        None => StatusBarStyle::Default,
    };
    let hidden = match hidden {
        Some(ast::Expr::Bool(value, _)) => value,
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "StatusBar hidden must be `true` or `false`",
            ));
        }
        None => false,
    };
    Ok(StatusBarConfig { style, hidden })
}

fn binding_name(
    expr: Option<ast::Expr>,
    default: &str,
    role: &str,
) -> Result<String, CompileError> {
    match expr {
        Some(ast::Expr::Name(name, _)) => Ok(name),
        Some(expr) => Err(CompileError::new(
            expr.span(),
            format!("{role} must be an identifier"),
        )),
        None => Ok(default.to_owned()),
    }
}

fn resolve_screen(
    expr: &ast::Expr,
    screen_ids: &HashMap<String, ScreenId>,
    role: &str,
) -> Result<ScreenId, CompileError> {
    let ast::Expr::Name(name, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            format!("{role} must be a declared screen name"),
        ));
    };
    screen_ids
        .get(name)
        .copied()
        .ok_or_else(|| CompileError::new(*span, format!("unknown screen `{name}` in {role}")))
}

fn lower_actions(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
) -> Result<Vec<Action>, CompileError> {
    let mut lowered = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            ast::Stmt::Assign { name, value, span } => {
                let Some((ty, mutable)) = symbols.get(&name) else {
                    return Err(CompileError::new(span, format!("unknown state `{name}`")));
                };
                if !mutable {
                    return Err(CompileError::new(
                        span,
                        format!("`{name}` is immutable and cannot be assigned"),
                    ));
                }
                let value = lower_expr(&value, Some(ty), symbols)?;
                lowered.push(Action::Assign { name, value });
            }
            ast::Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition = lower_expr(&condition, Some(&Type::Bool), symbols)?;
                lowered.push(Action::If {
                    condition,
                    then_branch: lower_actions(then_branch, symbols)?,
                    else_branch: else_branch
                        .map(|branch| lower_actions(branch, symbols))
                        .transpose()?,
                });
            }
        }
    }
    Ok(lowered)
}

fn require_mutable_binding(
    expr: &ast::Expr,
    expected_type: &Type,
    symbols: &HashMap<String, (Type, bool)>,
    span: Span,
    component: &str,
) -> Result<String, CompileError> {
    let ast::Expr::Name(name, _) = expr else {
        return Err(CompileError::new(
            span,
            format!("{component} requires a mutable state binding"),
        ));
    };
    let Some((ty, mutable)) = symbols.get(name) else {
        return Err(CompileError::new(
            expr.span(),
            format!("unknown state `{name}`"),
        ));
    };
    if ty != expected_type {
        return Err(CompileError::new(
            expr.span(),
            format!(
                "{component} binding `{name}` must have type {}",
                type_name(expected_type)
            ),
        ));
    }
    if !mutable {
        return Err(CompileError::new(
            expr.span(),
            format!("{component} binding `{name}` must be mutable"),
        ));
    }
    Ok(name.clone())
}

fn require_string_literal(expr: &ast::Expr, field: &str) -> Result<String, CompileError> {
    match expr {
        ast::Expr::String(value, _) => Ok(value.clone()),
        _ => Err(CompileError::new(
            expr.span(),
            format!("{field} must be a string literal"),
        )),
    }
}

fn optional_bool(
    expr: Option<ast::Expr>,
    default: bool,
    field: &str,
) -> Result<bool, CompileError> {
    match expr {
        Some(ast::Expr::Bool(value, _)) => Ok(value),
        Some(expr) => Err(CompileError::new(
            expr.span(),
            format!("`{field}` must be `true` or `false`"),
        )),
        None => Ok(default),
    }
}

fn optional_bool_override(
    expr: Option<ast::Expr>,
    field: &str,
) -> Result<Option<bool>, CompileError> {
    match expr {
        Some(ast::Expr::Bool(value, _)) => Ok(Some(value)),
        Some(expr) => Err(CompileError::new(
            expr.span(),
            format!("`{field}` must be `true` or `false`"),
        )),
        None => Ok(None),
    }
}

fn is_resource_name(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_https_url(value: &str) -> bool {
    let Some(authority_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    !authority.is_empty()
        && !authority.starts_with('.')
        && !authority.chars().any(char::is_whitespace)
}
