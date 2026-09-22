use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{
    AccessibilityRole, Action, BottomBarTab, Capitalization, DirectionConfig, DirectionStyle, Expr,
    FontWeight, ImageScale, ImageSource, KeyboardType, LayoutKind, ListSource, Node, NumericType,
    ScreenId, StatusBarConfig, StatusBarStyle, TextStyle, Type, WhenCase,
};
use nexa_syntax::ast;

use super::{
    custom_components::ComponentSignatures,
    expressions::{FunctionSignatures, infer_expr_type, lower_expr, type_name},
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
    functions: &FunctionSignatures,
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
                        functions,
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
                    functions,
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
    functions: &FunctionSignatures,
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
        ast::Node::Direction { value, .. } => Ok(Node::Direction {
            config: lower_direction(value)?,
        }),
        ast::Node::OnAppear {
            actions,
            asynchronous,
            ..
        } => Ok(Node::OnAppear {
            actions: lower_actions(actions, symbols, functions, asynchronous)?,
            asynchronous,
        }),
        ast::Node::OnDisappear { actions, .. } => Ok(Node::OnDisappear {
            actions: lower_actions(actions, symbols, functions, false)?,
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
                children, symbols, screen_ids, themes, components, functions, false, target,
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
            font_weight,
            line_limit,
            line_height,
            letter_spacing,
            selectable,
            ..
        } => {
            let value = lower_expr(&value, None, symbols, functions, false)?;
            let color = optional_color(color, "text color", themes)?;
            let font_size = optional_dimension(
                font_size,
                "fontSize",
                Some(ast::ThemeTokenKind::FontSize),
                themes,
            )?;
            let font_weight = lower_font_weight(font_weight)?;
            let line_limit = lower_line_limit(line_limit)?;
            let line_height = optional_dimension(line_height, "lineHeight", None, themes)?;
            let letter_spacing = optional_dimension(letter_spacing, "letterSpacing", None, themes)?;
            let selectable = optional_bool(selectable, false, "selectable")?;
            Ok(Node::Text {
                value,
                style: TextStyle {
                    color,
                    font_size,
                    font_weight,
                    line_limit,
                    line_height,
                    letter_spacing,
                    selectable,
                },
            })
        }
        ast::Node::Button {
            label,
            loading,
            disabled,
            actions,
            span,
        } => {
            let label = lower_expr(&label, Some(&Type::String), symbols, functions, false)?;
            if !matches!(
                label,
                Expr::String(_) | Expr::Interpolation(_) | Expr::State(_, Type::String)
            ) {
                return Err(CompileError::new(span, "Button label must be a String"));
            }
            let loading = loading
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?;
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?;
            let lowered = lower_actions(actions, symbols, functions, false)?;
            Ok(Node::Button {
                label,
                loading,
                disabled,
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
            focused,
            actions,
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
            if multiline && !actions.is_empty() {
                return Err(CompileError::new(
                    span,
                    "TextInput submit actions require a single-line field",
                ));
            }
            let focused = focused
                .map(|value| {
                    require_mutable_binding(&value, &Type::Bool, symbols, span, "TextInput focus")
                })
                .transpose()?;
            Ok(Node::TextInput {
                state,
                placeholder,
                keyboard,
                secure,
                multiline,
                autocorrect,
                capitalization,
                focused,
                actions: lower_actions(actions, symbols, functions, false)?,
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
            long_press_actions,
            ..
        } => {
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?
                .unwrap_or(Expr::Bool(false));
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            let actions = lower_actions(actions, symbols, functions, false)?;
            let long_press_actions = lower_actions(long_press_actions, symbols, functions, false)?;
            Ok(Node::Pressable {
                disabled,
                children: lowered_children,
                actions,
                long_press_actions,
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
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            Ok(Node::NavigationLink {
                destination,
                children: lowered_children,
            })
        }
        ast::Node::Link {
            url,
            children,
            span,
        } => {
            let url = require_string_literal(&url, "Link URL")?;
            if !is_link_url(&url) {
                return Err(CompileError::new(
                    span,
                    "Link URL must include a valid absolute scheme (for example `https://` or `mailto:`)",
                ));
            }
            let children = lower_nodes(
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            Ok(Node::Link { url, children })
        }
        ast::Node::Accessibility {
            label,
            role,
            children,
            span,
        } => {
            let label = require_string_literal(&label, "Accessibility label")?;
            if label.is_empty() {
                return Err(CompileError::new(
                    span,
                    "Accessibility label cannot be empty",
                ));
            }
            let role = match role {
                None => AccessibilityRole::None,
                Some(ast::Expr::Name(name, role_span)) => match name.as_str() {
                    "None" => AccessibilityRole::None,
                    "Button" => AccessibilityRole::Button,
                    "Link" => AccessibilityRole::Link,
                    "Header" => AccessibilityRole::Header,
                    "Image" => AccessibilityRole::Image,
                    _ => {
                        return Err(CompileError::new(
                            role_span,
                            "Accessibility role must be `None`, `Button`, `Link`, `Header`, or `Image`",
                        ));
                    }
                },
                Some(expr) => {
                    return Err(CompileError::new(
                        expr.span(),
                        "Accessibility role must be a role name",
                    ));
                }
            };
            let children = lower_nodes(
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            if children.is_empty() {
                return Err(CompileError::new(
                    span,
                    "Accessibility requires at least one child",
                ));
            }
            Ok(Node::Accessibility {
                label,
                role,
                children,
            })
        }
        ast::Node::KeyboardAware { children, .. } => {
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, functions, false, target,
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
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            Ok(Node::BottomSheet {
                state,
                children: lowered_children,
            })
        }
        ast::Node::RefreshControl {
            is_refreshing,
            children,
            actions,
            span,
        } => {
            let state = require_mutable_binding(
                &is_refreshing,
                &Type::Bool,
                symbols,
                span,
                "RefreshControl",
            )?;
            let lowered_children = lower_nodes(
                children, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            Ok(Node::RefreshControl {
                state,
                children: lowered_children,
                actions: lower_actions(actions, symbols, functions, false)?,
            })
        }
        ast::Node::AppBottomBar {
            selected,
            tabs,
            span,
        } => {
            let state = require_mutable_binding(
                &selected,
                &Type::Numeric(NumericType::Int32),
                symbols,
                span,
                "AppBottomBar",
            )?;
            let mut lowered_tabs = Vec::with_capacity(tabs.len());
            for tab in tabs {
                let (index, index_span) = match tab.index {
                    ast::Expr::Number(raw, index_span) => (raw, index_span),
                    expr => {
                        return Err(CompileError::new(
                            expr.span(),
                            "Tab index must be a non-negative Int32 literal",
                        ));
                    }
                };
                let index = index.parse::<i32>().map_err(|_| {
                    CompileError::new(index_span, "Tab index must be a non-negative Int32 literal")
                })?;
                if index < 0
                    || lowered_tabs
                        .iter()
                        .any(|tab: &BottomBarTab| tab.index == index)
                {
                    return Err(CompileError::new(
                        index_span,
                        "AppBottomBar tab indexes must be unique and non-negative",
                    ));
                }
                let label = require_string_literal(&tab.label, "Tab label")?;
                let children = lower_nodes(
                    tab.children,
                    symbols,
                    screen_ids,
                    themes,
                    components,
                    functions,
                    false,
                    target,
                )?;
                lowered_tabs.push(BottomBarTab {
                    index,
                    label,
                    children,
                });
            }
            Ok(Node::AppBottomBar {
                state,
                tabs: lowered_tabs,
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
                    let count_value =
                        lower_expr(&count, Some(&row_index_type), symbols, functions, false)?;
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
                functions,
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
            let condition = lower_expr(&condition, Some(&Type::Bool), symbols, functions, false)?;
            let lowered_then = lower_nodes(
                then_body, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            let lowered_else = else_body
                .map(|body| {
                    lower_nodes(
                        body, symbols, screen_ids, themes, components, functions, false, target,
                    )
                })
                .transpose()?;
            Ok(Node::If {
                condition,
                then_body: lowered_then,
                else_body: lowered_else,
            })
        }
        ast::Node::When {
            value,
            cases,
            else_body,
            span,
        } => {
            let value_type = infer_expr_type(&value, symbols, functions)
                .ok_or_else(|| CompileError::new(span, "when requires a typed scalar value"))?;
            if !matches!(value_type, Type::String | Type::Bool | Type::Numeric(_)) {
                return Err(CompileError::new(
                    span,
                    "when supports String, Bool, and numeric values only",
                ));
            }
            let lowered_value = lower_expr(&value, Some(&value_type), symbols, functions, false)?;
            let mut seen = std::collections::HashSet::with_capacity(cases.len());
            let mut lowered_cases = Vec::with_capacity(cases.len());
            for case in cases {
                if !matches!(
                    &case.value,
                    ast::Expr::String(_, _) | ast::Expr::Number(_, _) | ast::Expr::Bool(_, _)
                ) {
                    return Err(CompileError::new(
                        case.span,
                        "when case values must be String, Bool, or numeric literals",
                    ));
                }
                let lowered_case =
                    lower_expr(&case.value, Some(&value_type), symbols, functions, false)?;
                let key = format!("{lowered_case:?}");
                if !seen.insert(key) {
                    return Err(CompileError::new(
                        case.span,
                        "when case values must be unique",
                    ));
                }
                let body = lower_nodes(
                    case.body, symbols, screen_ids, themes, components, functions, false, target,
                )?;
                lowered_cases.push(WhenCase {
                    value: lowered_case,
                    body,
                });
            }
            let lowered_else = lower_nodes(
                else_body, symbols, screen_ids, themes, components, functions, false, target,
            )?;
            Ok(Node::When {
                value: lowered_value,
                cases: lowered_cases,
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
                lowered_arguments.push((
                    parameter.clone(),
                    lower_expr(&argument, Some(ty), symbols, functions, false)?,
                ));
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

fn lower_direction(value: ast::Expr) -> Result<DirectionConfig, CompileError> {
    let style = match value {
        ast::Expr::Name(name, name_span) => match name.as_str() {
            "LTR" | "Ltr" => DirectionStyle::Ltr,
            "RTL" | "Rtl" => DirectionStyle::Rtl,
            _ => {
                return Err(CompileError::new(
                    name_span,
                    "Direction value must be `LTR` or `RTL`",
                ));
            }
        },
        expr => {
            return Err(CompileError::new(
                expr.span(),
                "Direction value must be `LTR` or `RTL`",
            ));
        }
    };
    Ok(DirectionConfig { style })
}

fn lower_font_weight(value: Option<ast::Expr>) -> Result<Option<FontWeight>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Text fontWeight must be Normal, Medium, Semibold, or Bold",
        ));
    };
    let weight = match name.as_str() {
        "Normal" | "Regular" => FontWeight::Normal,
        "Medium" => FontWeight::Medium,
        "Semibold" | "SemiBold" => FontWeight::Semibold,
        "Bold" => FontWeight::Bold,
        _ => {
            return Err(CompileError::new(
                span,
                "Text fontWeight must be Normal, Medium, Semibold, or Bold",
            ));
        }
    };
    Ok(Some(weight))
}

fn lower_line_limit(value: Option<ast::Expr>) -> Result<Option<i32>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Number(raw, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Text lineLimit must be a positive integer literal",
        ));
    };
    let limit = raw.parse::<i32>().map_err(|_| {
        CompileError::new(span, "Text lineLimit must be a positive integer literal")
    })?;
    if limit <= 0 {
        return Err(CompileError::new(
            span,
            "Text lineLimit must be greater than zero",
        ));
    }
    Ok(Some(limit))
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
    functions: &FunctionSignatures,
    allow_await: bool,
) -> Result<Vec<Action>, CompileError> {
    lower_actions_with_depth(actions, symbols, functions, allow_await, 0)
}

fn lower_actions_with_depth(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    loop_depth: usize,
) -> Result<Vec<Action>, CompileError> {
    let mut lowered = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            ast::Stmt::Let { span, .. } => {
                return Err(CompileError::new(
                    span,
                    "local `let` declarations are only allowed inside functions",
                ));
            }
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
                let value = lower_expr(&value, Some(ty), symbols, functions, allow_await)?;
                lowered.push(Action::Assign { name, value });
            }
            ast::Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    symbols,
                    functions,
                    allow_await,
                )?;
                lowered.push(Action::If {
                    condition,
                    then_branch: lower_actions_with_depth(
                        then_branch,
                        symbols,
                        functions,
                        allow_await,
                        loop_depth,
                    )?,
                    else_branch: else_branch
                        .map(|branch| {
                            lower_actions_with_depth(
                                branch,
                                symbols,
                                functions,
                                allow_await,
                                loop_depth,
                            )
                        })
                        .transpose()?,
                });
            }
            ast::Stmt::For {
                name,
                iterable,
                body,
                span,
            } => {
                if symbols.contains_key(&name) {
                    return Err(CompileError::new(
                        span,
                        format!("loop binding `{name}` shadows an existing binding"),
                    ));
                }
                let (iterable, element_type) = if let ast::Expr::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = &iterable
                {
                    let int32 = Type::Numeric(NumericType::Int32);
                    let start = lower_expr(start, Some(&int32), symbols, functions, allow_await)?;
                    let end = lower_expr(end, Some(&int32), symbols, functions, allow_await)?;
                    (
                        Expr::Range {
                            start: Box::new(start),
                            end: Box::new(end),
                            inclusive: *inclusive,
                        },
                        int32,
                    )
                } else {
                    let Some(iterable_type) = infer_expr_type(&iterable, symbols, functions) else {
                        return Err(CompileError::new(
                            span,
                            "for loops require an Array<T>, Set<T>, or an Int32 range",
                        ));
                    };
                    let element_type = match &iterable_type {
                        Type::Array(element) | Type::Set(element) => element.as_ref().clone(),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                "for loops require an Array<T>, Set<T>, or an Int32 range",
                            ));
                        }
                    };
                    let iterable = lower_expr(
                        &iterable,
                        Some(&iterable_type),
                        symbols,
                        functions,
                        allow_await,
                    )?;
                    (iterable, element_type)
                };
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(name.clone(), (element_type, false));
                let body = lower_actions_with_depth(
                    body,
                    &loop_symbols,
                    functions,
                    allow_await,
                    loop_depth + 1,
                )?;
                lowered.push(Action::For {
                    name,
                    iterable,
                    body,
                });
            }
            ast::Stmt::ForMap {
                key_name,
                value_name,
                iterable,
                body,
                span,
            } => {
                if key_name == value_name {
                    return Err(CompileError::new(
                        span,
                        "map loop key and value bindings must have different names",
                    ));
                }
                if symbols.contains_key(&key_name) || symbols.contains_key(&value_name) {
                    return Err(CompileError::new(
                        span,
                        "map loop bindings cannot shadow an existing binding",
                    ));
                }
                let Some(Type::Map(key_type, value_type)) =
                    infer_expr_type(&iterable, symbols, functions)
                else {
                    return Err(CompileError::new(
                        span,
                        "map destructuring loops require a Map<K, V> iterable",
                    ));
                };
                let iterable_type = Type::Map(key_type.clone(), value_type.clone());
                let iterable = lower_expr(
                    &iterable,
                    Some(&iterable_type),
                    symbols,
                    functions,
                    allow_await,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(key_name.clone(), ((*key_type).clone(), false));
                loop_symbols.insert(value_name.clone(), ((*value_type).clone(), false));
                let body = lower_actions_with_depth(
                    body,
                    &loop_symbols,
                    functions,
                    allow_await,
                    loop_depth + 1,
                )?;
                lowered.push(Action::ForMap {
                    key_name,
                    value_name,
                    iterable,
                    body,
                });
            }
            ast::Stmt::While {
                condition, body, ..
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    symbols,
                    functions,
                    allow_await,
                )?;
                let body = lower_actions_with_depth(
                    body,
                    symbols,
                    functions,
                    allow_await,
                    loop_depth + 1,
                )?;
                lowered.push(Action::While { condition, body });
            }
            ast::Stmt::Break { span } => {
                if loop_depth == 0 {
                    return Err(CompileError::new(
                        span,
                        "`break` is only allowed inside a loop",
                    ));
                }
                lowered.push(Action::Break);
            }
            ast::Stmt::Continue { span } => {
                if loop_depth == 0 {
                    return Err(CompileError::new(
                        span,
                        "`continue` is only allowed inside a loop",
                    ));
                }
                lowered.push(Action::Continue);
            }
            ast::Stmt::Return { span, .. } => {
                return Err(CompileError::new(
                    span,
                    "`return` is only valid inside a function",
                ));
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

fn is_link_url(value: &str) -> bool {
    let Some((scheme, remainder)) = value.split_once(':') else {
        return false;
    };
    if scheme.is_empty()
        || !scheme.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic()
            } else {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
            }
        })
    {
        return false;
    }
    !remainder.trim().is_empty() && !value.chars().any(char::is_whitespace)
}
