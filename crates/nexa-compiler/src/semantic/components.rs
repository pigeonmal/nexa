use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{
    AccessibilityRole, Action, BottomBarTab, Capitalization, CollectionMutation, DirectionConfig,
    DirectionStyle, Expr, FastListRefresh, FontWeight, HapticStyle, ImageScale, ImageSource,
    KeyboardDismissMode, KeyboardType, LayoutKind, ListAxis, ListSource, Node, NumericType,
    ScreenId, StatusBarConfig, StatusBarStyle, TextStyle, Type, WhenCase,
};
use nexa_syntax::ast;

use super::{
    custom_components::ComponentSignatures,
    expressions::{FunctionSignatures, infer_expr_type, lower_expr, type_name},
    styles::{lower_style, optional_color, optional_dimension, parse_color_literal},
    themes::ThemeSymbols,
};
use crate::Target;

#[derive(Clone)]
pub(super) struct ScreenSignature {
    pub(super) id: ScreenId,
    pub(super) parameters: Vec<nexa_ir::FunctionParameter>,
}

pub(super) type ScreenSignatures = HashMap<String, ScreenSignature>;

pub(super) fn lower_nodes(
    nodes: Vec<ast::Node>,
    symbols: &HashMap<String, (Type, bool)>,
    screen_ids: &ScreenSignatures,
    themes: &ThemeSymbols,
    components: &ComponentSignatures,
    functions: &FunctionSignatures,
    allow_navigation_stack: bool,
    allow_navigation_back: bool,
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
                        allow_navigation_back,
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
                    allow_navigation_back,
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
    screen_ids: &ScreenSignatures,
    themes: &ThemeSymbols,
    components: &ComponentSignatures,
    functions: &FunctionSignatures,
    allow_navigation_stack: bool,
    allow_navigation_back: bool,
    target: Target,
) -> Result<Node, CompileError> {
    match node {
        ast::Node::Platform { .. } => {
            unreachable!("platform blocks are expanded by lower_nodes")
        }
        ast::Node::Content { .. } => Ok(Node::Content),
        ast::Node::StatusBar {
            style,
            hidden,
            background,
            ..
        } => Ok(Node::StatusBar {
            config: lower_status_bar(style, hidden, background)?,
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
        ast::Node::OnActive { actions, .. } => Ok(Node::OnActive {
            actions: lower_actions(actions, symbols, functions, false)?,
        }),
        ast::Node::OnInactive { actions, .. } => Ok(Node::OnInactive {
            actions: lower_actions(actions, symbols, functions, false)?,
        }),
        ast::Node::OnBackground { actions, .. } => Ok(Node::OnBackground {
            actions: lower_actions(actions, symbols, functions, false)?,
        }),
        ast::Node::Layout {
            kind,
            spacing,
            style,
            children,
            span,
        } => {
            if matches!(kind, ast::LayoutKind::Stack) && spacing.is_some() {
                return Err(CompileError::new(
                    spacing.as_ref().map(ast::Expr::span).unwrap_or(span),
                    "Stack does not accept `spacing`; use alignment or explicit child layout instead",
                ));
            }
            let spacing = optional_dimension(
                spacing,
                "spacing",
                Some(ast::ThemeTokenKind::Spacing),
                themes,
            )?
            .unwrap_or(0.0);
            let style = lower_style(style, themes)?;
            let lowered = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            let kind = match kind {
                ast::LayoutKind::Column => LayoutKind::Column,
                ast::LayoutKind::Row => LayoutKind::Row,
                ast::LayoutKind::Stack => LayoutKind::Stack,
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
            icon,
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
            let icon = icon
                .map(|value| require_string_literal(&value, "Button icon"))
                .transpose()?;
            let loading = loading
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?;
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?;
            let lowered = lower_actions(actions, symbols, functions, false)?;
            Ok(Node::Button {
                label,
                icon,
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
            max_length,
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
            let max_length = lower_positive_integer(max_length, "TextInput maxLength")?;
            Ok(Node::TextInput {
                state,
                placeholder,
                keyboard,
                secure,
                multiline,
                autocorrect,
                capitalization,
                focused,
                max_length,
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
                    let lowered_url =
                        lower_expr(&url, Some(&Type::String), symbols, functions, false)?;
                    if let ast::Expr::String(value, _) = &url {
                        if !is_https_url(value) {
                            return Err(CompileError::new(
                                span,
                                "Image URL must be an absolute HTTPS URL",
                            ));
                        }
                    }
                    ImageSource::RemoteUrl(lowered_url)
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
            haptic,
            children,
            actions,
            long_press_actions,
            ..
        } => {
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), symbols, functions, false))
                .transpose()?
                .unwrap_or(Expr::Bool(false));
            let haptic = lower_haptic(haptic)?;
            let lowered_children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            let actions = lower_actions(actions, symbols, functions, false)?;
            let long_press_actions = lower_actions(long_press_actions, symbols, functions, false)?;
            Ok(Node::Pressable {
                disabled,
                haptic,
                children: lowered_children,
                actions,
                long_press_actions,
            })
        }
        ast::Node::NavigationStack {
            root,
            arguments,
            span,
        } => {
            if !allow_navigation_stack {
                return Err(CompileError::new(
                    span,
                    "NavigationStack is only allowed as the app's top-level body",
                ));
            }
            let root = lower_screen_target(
                &root,
                arguments,
                screen_ids,
                symbols,
                functions,
                "NavigationStack root",
            )?;
            Ok(Node::NavigationStack {
                root: root.0,
                arguments: root.1,
            })
        }
        ast::Node::NavigationBack { label, span } => {
            if !allow_navigation_back {
                return Err(CompileError::new(
                    span,
                    "NavigationBack is only allowed inside a declared screen",
                ));
            }
            let label = label
                .map(|label| lower_expr(&label, Some(&Type::String), symbols, functions, false))
                .transpose()?
                .unwrap_or_else(|| Expr::String("Back".to_owned()));
            Ok(Node::NavigationBack { label })
        }
        ast::Node::NavigationLink {
            destination,
            arguments,
            guard,
            children,
            span,
        } => {
            let destination = lower_screen_target(
                &destination,
                arguments,
                screen_ids,
                symbols,
                functions,
                "NavigationLink destination",
            )
            .map_err(|error| {
                if screen_ids.is_empty() {
                    CompileError::new(span, "NavigationLink requires declared app screens")
                } else {
                    error
                }
            })?;
            let guard = guard
                .map(|guard| lower_expr(&guard, Some(&Type::Bool), symbols, functions, false))
                .transpose()?;
            let lowered_children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            Ok(Node::NavigationLink {
                destination: destination.0,
                arguments: destination.1,
                guard,
                children: lowered_children,
            })
        }
        ast::Node::Link {
            url,
            children,
            span,
        } => {
            let lowered_url = lower_expr(&url, Some(&Type::String), symbols, functions, false)?;
            if let ast::Expr::String(value, _) = &url {
                if !is_link_url(value) {
                    return Err(CompileError::new(
                        span,
                        "Link URL must include a valid absolute scheme (for example `https://` or `mailto:`)",
                    ));
                }
            }
            let children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            Ok(Node::Link {
                url: lowered_url,
                children,
            })
        }
        ast::Node::Accessibility {
            label,
            hint,
            role,
            children,
            span,
        } => {
            let lowered_label = lower_expr(&label, Some(&Type::String), symbols, functions, false)?;
            if let ast::Expr::String(value, _) = &label {
                if value.is_empty() {
                    return Err(CompileError::new(
                        span,
                        "Accessibility label cannot be empty",
                    ));
                }
            }
            let lowered_hint = hint
                .map(|hint| {
                    let lowered =
                        lower_expr(&hint, Some(&Type::String), symbols, functions, false)?;
                    if let ast::Expr::String(value, _) = &hint {
                        if value.is_empty() {
                            return Err(CompileError::new(
                                hint.span(),
                                "Accessibility hint cannot be empty",
                            ));
                        }
                    }
                    Ok(lowered)
                })
                .transpose()?;
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
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            if children.is_empty() {
                return Err(CompileError::new(
                    span,
                    "Accessibility requires at least one child",
                ));
            }
            Ok(Node::Accessibility {
                label: lowered_label,
                hint: lowered_hint,
                role,
                children,
            })
        }
        ast::Node::KeyboardAware {
            dismiss, children, ..
        } => {
            let lowered_children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            Ok(Node::KeyboardAware {
                dismiss: lower_keyboard_dismiss(dismiss)?,
                children: lowered_children,
            })
        }
        ast::Node::BottomSheet {
            is_presented,
            partial,
            children,
            span,
        } => {
            let state =
                require_mutable_binding(&is_presented, &Type::Bool, symbols, span, "BottomSheet")?;
            let lowered_children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            Ok(Node::BottomSheet {
                state,
                partial: optional_bool(partial, false, "BottomSheet partial")?,
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
            let mut lowered_children = lower_nodes(
                children,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            let actions = lower_actions(actions, symbols, functions, false)?;
            if lowered_children.len() == 1 {
                let mut child = lowered_children.pop().expect("one lowered refresh child");
                if let Node::FastList { refresh, .. } = &mut child {
                    *refresh = Some(FastListRefresh { state, actions });
                    return Ok(child);
                }
                lowered_children.push(child);
            }
            Ok(Node::RefreshControl {
                state,
                children: lowered_children,
                actions,
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
                let icon = tab
                    .icon
                    .as_ref()
                    .map(|icon| require_string_literal(icon, "Tab icon"))
                    .transpose()?;
                let badge = tab
                    .badge
                    .as_ref()
                    .map(|badge| require_string_literal(badge, "Tab badge"))
                    .transpose()?;
                let children = lower_nodes(
                    tab.children,
                    symbols,
                    screen_ids,
                    themes,
                    components,
                    functions,
                    false,
                    allow_navigation_back,
                    target,
                )?;
                lowered_tabs.push(BottomBarTab {
                    index,
                    label,
                    icon,
                    badge,
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
            axis,
            item_extent,
            section,
            index,
            item,
            key,
            scroll_position,
            children,
            on_end_reached,
            on_scroll,
            sticky_header,
            section_header,
            span,
        } => {
            let sections_source = matches!(&source, ast::ListSource::Sections(_));
            let axis = match axis {
                None => ListAxis::Vertical,
                Some(ast::Expr::Name(name, axis_span)) => match name.as_str() {
                    "Vertical" => ListAxis::Vertical,
                    "Horizontal" => ListAxis::Horizontal,
                    _ => {
                        return Err(CompileError::new(
                            axis_span,
                            "FastList `axis` must be `Vertical`, `Horizontal`, or `Grid(columns)`",
                        ));
                    }
                },
                Some(ast::Expr::Call(name, arguments, axis_span)) if name == "Grid" => {
                    match arguments.as_slice() {
                        [ast::Expr::Number(raw, columns_span)] => {
                            let Ok(columns) = raw.parse::<u32>() else {
                                return Err(CompileError::new(
                                    *columns_span,
                                    "FastList grid columns must be a positive integer literal",
                                ));
                            };
                            if columns == 0 {
                                return Err(CompileError::new(
                                    *columns_span,
                                    "FastList grid columns must be greater than zero",
                                ));
                            }
                            ListAxis::Grid { columns }
                        }
                        _ => {
                            return Err(CompileError::new(
                                axis_span,
                                "FastList `Grid` requires one positive integer column count",
                            ));
                        }
                    }
                }
                Some(expression) => {
                    return Err(CompileError::new(
                        expression.span(),
                        "FastList `axis` must be `Vertical`, `Horizontal`, or `Grid(columns)`",
                    ));
                }
            };
            if sticky_header.is_some() && !matches!(axis, ListAxis::Vertical) {
                return Err(CompileError::new(
                    span,
                    "FastList `stickyHeader` is supported only for vertical lists",
                ));
            }
            if sections_source && !matches!(axis, ListAxis::Vertical) {
                return Err(CompileError::new(
                    span,
                    "FastList `sections` is supported only for vertical lists",
                ));
            }
            if sections_source && sticky_header.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `sections` uses `sectionHeader` instead of `stickyHeader`",
                ));
            }
            if !sections_source && section.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `section` is only available with a `sections` source",
                ));
            }
            if !sections_source && section_header.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `sectionHeader` is only available with a `sections` source",
                ));
            }
            if sections_source
                && (scroll_position.is_some() || on_end_reached.is_some() || on_scroll.is_some())
            {
                return Err(CompileError::new(
                    span,
                    "FastList `sections` does not yet support scrollPosition, onEndReached, or onScroll",
                ));
            }
            let item_extent = optional_dimension(item_extent, "FastList rowHeight", None, themes)?;
            if item_extent.is_some_and(|value| value <= 0.0) {
                return Err(CompileError::new(
                    span,
                    "FastList `rowHeight` must be greater than zero",
                ));
            }
            let row_index_type = Type::Numeric(NumericType::Int32);
            let section_index_type = Type::Numeric(NumericType::Int32);
            let has_section_binding = section.is_some();
            let section_name = binding_name(section, "section", "FastList section")?;
            let index = binding_name(index, "index", "FastList index")?;
            let scroll_position = scroll_position
                .map(|position| {
                    require_mutable_binding(
                        &position,
                        &row_index_type,
                        symbols,
                        position.span(),
                        "FastList scrollPosition",
                    )
                })
                .transpose()?;
            let (source, section, item, item_type) = match source {
                ast::ListSource::Count(count) => {
                    if item.is_some() || has_section_binding {
                        return Err(CompileError::new(
                            span,
                            "FastList item and section bindings are only available with an array or `sections` source",
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
                    (ListSource::Count(count_value), None, None, None)
                }
                ast::ListSource::Items(collection) => {
                    if has_section_binding {
                        return Err(CompileError::new(
                            span,
                            "FastList `section` is only available with a `sections` source",
                        ));
                    }
                    let ast::Expr::Name(name, name_span) = collection else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList positional source must be an Array<T> binding",
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
                            format!("FastList collection `{name}` must have type Array<T>"),
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
                        None,
                        Some(item_name),
                        Some((**element_type).clone()),
                    )
                }
                ast::ListSource::Sections(collection) => {
                    let ast::Expr::Name(name, name_span) = collection else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList `sections` must be an Array<Array<T>> binding",
                        ));
                    };
                    let Some((ty, _)) = symbols.get(&name) else {
                        return Err(CompileError::new(
                            name_span,
                            format!("unknown state `{name}`"),
                        ));
                    };
                    let Type::Array(section_type) = ty else {
                        return Err(CompileError::new(
                            name_span,
                            format!("FastList sections `{name}` must have type Array<Array<T>>"),
                        ));
                    };
                    let Type::Array(element_type) = &**section_type else {
                        return Err(CompileError::new(
                            name_span,
                            format!("FastList sections `{name}` must have type Array<Array<T>>"),
                        ));
                    };
                    let item_name = binding_name(item, "item", "FastList item")?;
                    if item_name == index || item_name == section_name || index == section_name {
                        return Err(CompileError::new(
                            name_span,
                            "FastList section, item, and index bindings must have different names",
                        ));
                    }
                    (
                        ListSource::Sections {
                            collection: Expr::State(name, ty.clone()),
                            element_type: (**element_type).clone(),
                        },
                        Some(section_name.clone()),
                        Some(item_name),
                        Some((**element_type).clone()),
                    )
                }
            };
            let mut row_symbols = symbols.clone();
            row_symbols.insert(index.clone(), (row_index_type, false));
            if let Some(section_name) = &section {
                row_symbols.insert(section_name.clone(), (section_index_type, false));
            }
            if let (Some(item_name), Some(item_type)) = (&item, &item_type) {
                row_symbols.insert(item_name.clone(), (item_type.clone(), false));
            }
            let key = key
                .map(|key| {
                    let item_name = item.as_deref().ok_or_else(|| {
                        CompileError::new(
                            span,
                            "FastList `key` requires a collection source with an item binding",
                        )
                    })?;
                    item_type.as_ref().ok_or_else(|| {
                        CompileError::new(
                            span,
                            "FastList `key` requires a collection source with an item binding",
                        )
                    })?;
                    let key = match key {
                        ast::ListKey::SelfValue(id_span) => {
                            ast::Expr::Name(item_name.to_owned(), id_span)
                        }
                        ast::ListKey::Member {
                            name,
                            span: id_span,
                        } => ast::Expr::Member {
                            base: Box::new(ast::Expr::Name(item_name.to_owned(), id_span)),
                            name,
                            optional: false,
                            span: id_span,
                        },
                    };
                    let key_type =
                        super::expressions::infer_expr_type(&key, &row_symbols, functions)
                            .ok_or_else(|| {
                                CompileError::new(
                                    key.span(),
                                    "FastList `key` must resolve to a statically known scalar type",
                                )
                            })?;
                    super::expressions::require_hashable_key(
                        &key_type,
                        key.span(),
                        "FastList keys",
                    )?;
                    lower_expr(&key, Some(&key_type), &row_symbols, functions, false)
                })
                .transpose()?;
            let lowered_children = lower_nodes(
                children,
                &row_symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            if lowered_children.is_empty() {
                return Err(CompileError::new(
                    span,
                    "FastList requires at least one row component",
                ));
            }
            if sticky_header.as_ref().is_some_and(Vec::is_empty) {
                return Err(CompileError::new(
                    span,
                    "FastList `stickyHeader` requires at least one header component",
                ));
            }
            if section_header.as_ref().is_some_and(Vec::is_empty) {
                return Err(CompileError::new(
                    span,
                    "FastList `sectionHeader` requires at least one header component",
                ));
            }
            let on_end_reached = on_end_reached
                .map(|actions| lower_actions(actions, symbols, functions, false))
                .transpose()?;
            let on_scroll = on_scroll
                .map(|actions| lower_actions(actions, symbols, functions, false))
                .transpose()?;
            let sticky_header = sticky_header
                .map(|header| {
                    lower_nodes(
                        header,
                        symbols,
                        screen_ids,
                        themes,
                        components,
                        functions,
                        false,
                        allow_navigation_back,
                        target,
                    )
                })
                .transpose()?;
            let section_header = section_header
                .map(|header| {
                    let mut header_symbols = symbols.clone();
                    if let Some(section_name) = &section {
                        header_symbols.insert(
                            section_name.clone(),
                            (Type::Numeric(NumericType::Int32), false),
                        );
                    }
                    lower_nodes(
                        header,
                        &header_symbols,
                        screen_ids,
                        themes,
                        components,
                        functions,
                        false,
                        allow_navigation_back,
                        target,
                    )
                })
                .transpose()?;
            Ok(Node::FastList {
                source,
                axis,
                item_extent,
                index,
                item,
                key,
                scroll_position,
                section,
                children: lowered_children,
                on_end_reached,
                on_scroll,
                sticky_header,
                section_header,
                refresh: None,
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
                then_body,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
            )?;
            let lowered_else = else_body
                .map(|body| {
                    lower_nodes(
                        body,
                        symbols,
                        screen_ids,
                        themes,
                        components,
                        functions,
                        false,
                        allow_navigation_back,
                        target,
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
            if !matches!(
                value_type,
                Type::String | Type::Bool | Type::Numeric(_) | Type::Enum(_)
            ) {
                return Err(CompileError::new(
                    span,
                    "when supports String, Bool, numeric, and enum values only",
                ));
            }
            let lowered_value = lower_expr(&value, Some(&value_type), symbols, functions, false)?;
            let mut seen = std::collections::HashSet::with_capacity(cases.len());
            let mut lowered_cases = Vec::with_capacity(cases.len());
            for case in cases {
                if !matches!(
                    &case.value,
                    ast::Expr::String(_, _)
                        | ast::Expr::Number(_, _)
                        | ast::Expr::Bool(_, _)
                        | ast::Expr::EnumCase { .. }
                ) {
                    return Err(CompileError::new(
                        case.span,
                        "when case values must be String, Bool, numeric literals, or enum cases",
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
                    case.body,
                    symbols,
                    screen_ids,
                    themes,
                    components,
                    functions,
                    false,
                    allow_navigation_back,
                    target,
                )?;
                lowered_cases.push(WhenCase {
                    value: lowered_case,
                    body,
                });
            }
            let lowered_else = lower_nodes(
                else_body,
                symbols,
                screen_ids,
                themes,
                components,
                functions,
                false,
                allow_navigation_back,
                target,
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
            children,
            span,
        } => {
            let signature = components
                .get(&name)
                .ok_or_else(|| CompileError::new(span, format!("unknown component `{name}`")))?;
            if children.is_some() && !signature.has_content_slot {
                return Err(CompileError::new(
                    span,
                    format!("component `{name}` does not declare a Content() slot"),
                ));
            }
            if children.is_none() && signature.has_content_slot {
                return Err(CompileError::new(
                    span,
                    format!("component `{name}` requires a content block"),
                ));
            }
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
            let lowered_children = children
                .map(|children| {
                    let lowered = lower_nodes(
                        children,
                        symbols,
                        screen_ids,
                        themes,
                        components,
                        functions,
                        false,
                        allow_navigation_back,
                        target,
                    )?;
                    if lowered.iter().any(contains_content) {
                        return Err(CompileError::new(
                            span,
                            "Content() is only available inside a custom component declaration",
                        ));
                    }
                    Ok(lowered)
                })
                .transpose()?;
            Ok(Node::ComponentCall {
                name,
                arguments: lowered_arguments,
                children: lowered_children,
            })
        }
    }
}

pub(super) fn contains_content(node: &Node) -> bool {
    match node {
        Node::Content => true,
        Node::Layout { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. }
        | Node::Pressable { children, .. } => children.iter().any(contains_content),
        Node::FastList {
            children,
            sticky_header,
            section_header,
            ..
        } => {
            children.iter().any(contains_content)
                || sticky_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_content))
                || section_header
                    .as_deref()
                    .is_some_and(|header| header.iter().any(contains_content))
        }
        Node::ComponentCall { children, .. } => children
            .as_ref()
            .is_some_and(|children| children.iter().any(contains_content)),
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .any(|tab| tab.children.iter().any(contains_content)),
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_content)
                || else_body
                    .as_deref()
                    .is_some_and(|body| body.iter().any(contains_content))
        }
        Node::When {
            cases, else_body, ..
        } => {
            cases
                .iter()
                .any(|case| case.body.iter().any(contains_content))
                || else_body.iter().any(contains_content)
        }
        Node::Text { .. }
        | Node::Button { .. }
        | Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. }
        | Node::NavigationBack { .. } => false,
    }
}

fn lower_status_bar(
    style: Option<ast::Expr>,
    hidden: Option<ast::Expr>,
    background: Option<ast::Expr>,
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
    let background = background
        .map(|value| parse_color_literal(value, "StatusBar background"))
        .transpose()?
        .map(nexa_ir::ColorValue::Static);
    Ok(StatusBarConfig {
        style,
        hidden,
        background,
    })
}

fn lower_keyboard_dismiss(value: Option<ast::Expr>) -> Result<KeyboardDismissMode, CompileError> {
    let Some(value) = value else {
        return Ok(KeyboardDismissMode::Interactive);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "KeyboardAware dismiss must be `Interactive` or `Never`",
        ));
    };
    match name.as_str() {
        "Interactive" => Ok(KeyboardDismissMode::Interactive),
        "Never" => Ok(KeyboardDismissMode::Never),
        _ => Err(CompileError::new(
            span,
            "KeyboardAware dismiss must be `Interactive` or `Never`",
        )),
    }
}

fn lower_haptic(value: Option<ast::Expr>) -> Result<Option<HapticStyle>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Pressable haptic must be `Light`, `Medium`, or `Heavy`",
        ));
    };
    let style = match name.as_str() {
        "Light" => HapticStyle::Light,
        "Medium" => HapticStyle::Medium,
        "Heavy" => HapticStyle::Heavy,
        _ => {
            return Err(CompileError::new(
                span,
                "Pressable haptic must be `Light`, `Medium`, or `Heavy`",
            ));
        }
    };
    Ok(Some(style))
}

fn lower_direction(value: ast::Expr) -> Result<DirectionConfig, CompileError> {
    let style = match value {
        ast::Expr::Name(name, name_span) => match name.as_str() {
            "LTR" => DirectionStyle::Ltr,
            "RTL" => DirectionStyle::Rtl,
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
        "Normal" => FontWeight::Normal,
        "Medium" => FontWeight::Medium,
        "Semibold" => FontWeight::Semibold,
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

fn lower_positive_integer(
    value: Option<ast::Expr>,
    field: &str,
) -> Result<Option<i32>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Number(raw, span) = value else {
        return Err(CompileError::new(
            value.span(),
            format!("{field} must be a positive integer literal"),
        ));
    };
    let limit = raw.parse::<i32>().map_err(|_| {
        CompileError::new(span, format!("{field} must be a positive integer literal"))
    })?;
    if limit <= 0 {
        return Err(CompileError::new(
            span,
            format!("{field} must be greater than zero"),
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
    screen_ids: &ScreenSignatures,
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
        .map(|signature| signature.id)
        .ok_or_else(|| CompileError::new(*span, format!("unknown screen `{name}` in {role}")))
}

fn lower_screen_target(
    screen: &ast::Expr,
    arguments: Vec<ast::Expr>,
    screen_ids: &ScreenSignatures,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    role: &str,
) -> Result<(ScreenId, Vec<Expr>), CompileError> {
    let id = resolve_screen(screen, screen_ids, role)?;
    let ast::Expr::Name(name, _) = screen else {
        unreachable!("resolve_screen validated navigation target name")
    };
    let signature = screen_ids
        .get(name)
        .expect("screen signature exists after screen resolution");
    if arguments.len() != signature.parameters.len() {
        return Err(CompileError::new(
            screen.span(),
            format!(
                "{role} `{name}` expects {} route argument(s), found {}",
                signature.parameters.len(),
                arguments.len()
            ),
        ));
    }
    let lowered = arguments
        .iter()
        .zip(&signature.parameters)
        .map(|(argument, parameter)| {
            lower_expr(argument, Some(&parameter.ty), symbols, functions, false)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((id, lowered))
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
            ast::Stmt::CollectionMutation {
                name,
                method,
                arguments,
                span,
            } => {
                let Some((ty, mutable)) = symbols.get(&name) else {
                    return Err(CompileError::new(span, format!("unknown state `{name}`")));
                };
                if !mutable {
                    return Err(CompileError::new(
                        span,
                        format!("`{name}` is immutable and cannot be mutated"),
                    ));
                }
                let (operation, expected_arguments): (CollectionMutation, Vec<&Type>) = match ty {
                    Type::Array(element) => match method.as_str() {
                        "append" => (CollectionMutation::ArrayAppend, vec![element.as_ref()]),
                        "remove" => (
                            CollectionMutation::ArrayRemoveAt,
                            vec![&Type::Numeric(NumericType::Int32)],
                        ),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "Array state `{name}` supports `append(value)` and `remove(index)`"
                                ),
                            ));
                        }
                    },
                    Type::Set(element) => match method.as_str() {
                        "insert" => (CollectionMutation::SetInsert, vec![element.as_ref()]),
                        "remove" => (CollectionMutation::SetRemove, vec![element.as_ref()]),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "Set state `{name}` supports `insert(value)` and `remove(value)`"
                                ),
                            ));
                        }
                    },
                    Type::Map(key, value) => match method.as_str() {
                        "set" => (
                            CollectionMutation::MapSet,
                            vec![key.as_ref(), value.as_ref()],
                        ),
                        "remove" => (CollectionMutation::MapRemove, vec![key.as_ref()]),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "Map state `{name}` supports `set(key, value)` and `remove(key)`"
                                ),
                            ));
                        }
                    },
                    _ => {
                        return Err(CompileError::new(
                            span,
                            format!("`{name}` is not a mutable collection"),
                        ));
                    }
                };
                if arguments.len() != expected_arguments.len() {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "collection method `{method}` expects {} argument(s), got {}",
                            expected_arguments.len(),
                            arguments.len()
                        ),
                    ));
                }
                let arguments = arguments
                    .iter()
                    .zip(expected_arguments)
                    .map(|(argument, expected)| {
                        lower_expr(argument, Some(expected), symbols, functions, allow_await)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                lowered.push(Action::CollectionMutation {
                    name,
                    operation,
                    arguments,
                });
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
                    step,
                    ..
                } = &iterable
                {
                    let int32 = Type::Numeric(NumericType::Int32);
                    let start = lower_expr(start, Some(&int32), symbols, functions, allow_await)?;
                    let end = lower_expr(end, Some(&int32), symbols, functions, allow_await)?;
                    let step = step
                        .as_deref()
                        .map(|step| lower_range_step(step, symbols, functions, allow_await))
                        .transpose()?
                        .map(Box::new);
                    (
                        Expr::Range {
                            start: Box::new(start),
                            end: Box::new(end),
                            inclusive: *inclusive,
                            step,
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

fn lower_range_step(
    step: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
) -> Result<Expr, CompileError> {
    let ast::Expr::Number(raw, span) = step else {
        return Err(CompileError::new(
            step.span(),
            "range step must be a positive Int32 literal",
        ));
    };
    let value = raw
        .parse::<i32>()
        .map_err(|_| CompileError::new(*span, "range step must be a positive Int32 literal"))?;
    if value <= 0 {
        return Err(CompileError::new(
            *span,
            "range step must be a positive Int32 literal",
        ));
    }
    lower_expr(
        step,
        Some(&Type::Numeric(NumericType::Int32)),
        symbols,
        functions,
        allow_await,
    )
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
