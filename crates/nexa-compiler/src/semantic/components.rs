use std::collections::{HashMap, HashSet};

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::walk::any_node;
use nexa_ir::{
    AccessibilityRole, Action, BottomBarTab, Capitalization, CollectionMutation, DirectionConfig,
    DirectionStyle, Expr, FastListRefresh, FontWeight, HapticStyle, ImageScale, ImageSource,
    KeyboardDismissMode, KeyboardType, LayoutKind, ListAxis, ListSource,
    NativeComponentEventHandler, Node, NumericType, ScreenId, StatusBarConfig, StatusBarStyle,
    TextStyle, Type, WhenCase,
};
use nexa_syntax::ast;

use super::{
    custom_components::ComponentSignatures,
    expressions::{
        FunctionSignatures, functions_with_error_handling, infer_expr_type, lower_expr,
        plugin_error_variant, type_name,
    },
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
    native_aliases: &HashMap<String, String>,
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
                        native_aliases,
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
                    native_aliases,
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
    native_aliases: &HashMap<String, String>,
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
            actions: lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                asynchronous,
                native_aliases,
            )?,
            asynchronous,
        }),
        ast::Node::OnDisappear { actions, .. } => Ok(Node::OnDisappear {
            actions: lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
            )?,
        }),
        ast::Node::OnActive { actions, .. } => Ok(Node::OnActive {
            actions: lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
            )?,
        }),
        ast::Node::OnInactive { actions, .. } => Ok(Node::OnInactive {
            actions: lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
            )?,
        }),
        ast::Node::OnBackground { actions, .. } => Ok(Node::OnBackground {
            actions: lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
            )?,
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
                native_aliases,
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
            let lowered =
                lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)?;
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
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                )?,
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
                native_aliases,
                false,
                allow_navigation_back,
                target,
            )?;
            let actions =
                lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)?;
            let long_press_actions = lower_actions_with_aliases(
                long_press_actions,
                symbols,
                functions,
                false,
                native_aliases,
            )?;
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
                native_aliases,
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
                native_aliases,
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
                native_aliases,
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
                native_aliases,
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
                native_aliases,
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
                native_aliases,
                false,
                allow_navigation_back,
                target,
            )?;
            let actions =
                lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)?;
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
                    native_aliases,
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
                native_aliases,
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
                .map(|actions| {
                    lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)
                })
                .transpose()?;
            let on_scroll = on_scroll
                .map(|actions| {
                    lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)
                })
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
                        native_aliases,
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
                        native_aliases,
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
                native_aliases,
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
                        native_aliases,
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
                    native_aliases,
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
                native_aliases,
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
                        native_aliases,
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
        ast::Node::NativeComponentCall {
            namespace,
            name,
            mut arguments,
            children,
            event_handlers,
            span,
        } => {
            let qualified_name = format!("{namespace}.{name}");
            let signature = components.get(&qualified_name).ok_or_else(|| {
                CompileError::new(span, format!("unknown native component `{qualified_name}`"))
            })?;
            if !signature.native {
                return Err(CompileError::new(
                    span,
                    format!("`{qualified_name}` is not a native component"),
                ));
            }
            let lowered_children = match (signature.has_content_slot, children) {
                (true, Some(children)) => Some(lower_nodes(
                    children,
                    symbols,
                    screen_ids,
                    themes,
                    components,
                    functions,
                    native_aliases,
                    allow_navigation_stack,
                    allow_navigation_back,
                    target,
                )?),
                (true, None) => {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "native component `{qualified_name}` requires a child block for its content slot"
                        ),
                    ));
                }
                (false, Some(_)) => {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "native component `{qualified_name}` does not declare a content slot"
                        ),
                    ));
                }
                (false, None) => None,
            };
            for argument_name in arguments.keys() {
                if !signature
                    .parameters
                    .iter()
                    .any(|(parameter_name, _)| parameter_name == argument_name)
                {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "native component `{qualified_name}` has no property `{argument_name}`"
                        ),
                    ));
                }
            }
            let mut lowered_arguments = Vec::with_capacity(signature.parameters.len());
            for (parameter, ty) in &signature.parameters {
                let Some(argument) = arguments.remove(parameter) else {
                    if let Some(default) = signature.defaults.get(parameter) {
                        lowered_arguments.push((
                            parameter.clone(),
                            lower_expr(default, Some(ty), symbols, functions, false)?,
                        ));
                        continue;
                    }
                    return Err(CompileError::new(
                        span,
                        format!(
                            "native component `{qualified_name}` requires property `{parameter}`"
                        ),
                    ));
                };
                lowered_arguments.push((
                    parameter.clone(),
                    lower_expr(&argument, Some(ty), symbols, functions, false)?,
                ));
            }
            let mut lowered_events = Vec::with_capacity(event_handlers.len());
            let mut seen_events = HashSet::with_capacity(event_handlers.len());
            for handler in event_handlers {
                if !seen_events.insert(handler.property.clone()) {
                    return Err(CompileError::new(
                        handler.span,
                        format!(
                            "native component callback `{}` is subscribed more than once",
                            handler.property
                        ),
                    ));
                }
                let Some(event) = signature
                    .events
                    .iter()
                    .find(|event| event.property == handler.property)
                else {
                    return Err(CompileError::new(
                        handler.span,
                        format!(
                            "native component `{qualified_name}` has no event callback `{}`",
                            handler.property
                        ),
                    ));
                };
                if handler.parameters.len() != event.parameters.len() {
                    return Err(CompileError::new(
                        handler.span,
                        format!(
                            "native component event `{qualified_name}.{}` provides {} value(s), but handler binds {}",
                            handler.property,
                            event.parameters.len(),
                            handler.parameters.len()
                        ),
                    ));
                }
                let mut event_symbols = symbols.clone();
                let mut unique_parameters = HashSet::with_capacity(handler.parameters.len());
                for (parameter_name, (_, ty)) in handler.parameters.iter().zip(&event.parameters) {
                    if !unique_parameters.insert(parameter_name.as_str()) {
                        return Err(CompileError::new(
                            handler.span,
                            format!(
                                "native component event handler binds `{parameter_name}` more than once"
                            ),
                        ));
                    }
                    if symbols.contains_key(parameter_name) {
                        return Err(CompileError::new(
                            handler.span,
                            format!(
                                "native component event binding `{parameter_name}` shadows an existing value"
                            ),
                        ));
                    }
                    event_symbols.insert(parameter_name.clone(), (ty.clone(), false));
                }
                let actions =
                    lower_actions_with_depth(handler.actions, &event_symbols, functions, false, 0)?;
                lowered_events.push(NativeComponentEventHandler {
                    property: handler.property,
                    parameters: handler.parameters,
                    actions,
                });
            }
            Ok(Node::NativeComponentCall {
                namespace,
                name,
                arguments: lowered_arguments,
                children: lowered_children,
                event_handlers: lowered_events,
            })
        }
    }
}

pub(super) fn contains_content(node: &Node) -> bool {
    any_node(std::slice::from_ref(node), |node| {
        matches!(node, Node::Content)
    })
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

fn lower_actions_with_aliases(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
) -> Result<Vec<Action>, CompileError> {
    lower_actions_with_aliases_depth(actions, symbols, functions, allow_await, native_aliases, 0)
}

fn lower_actions_with_depth(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    loop_depth: usize,
) -> Result<Vec<Action>, CompileError> {
    lower_actions_with_aliases_depth(
        actions,
        symbols,
        functions,
        allow_await,
        &HashMap::new(),
        loop_depth,
    )
}

fn lower_actions_with_aliases_depth(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
    loop_depth: usize,
) -> Result<Vec<Action>, CompileError> {
    let mut disposed_instances = HashSet::new();
    lower_actions_with_disposal_state(
        actions,
        symbols,
        functions,
        allow_await,
        native_aliases,
        loop_depth,
        &mut disposed_instances,
    )
}

fn lower_actions_with_disposal_state(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
    loop_depth: usize,
    disposed_instances: &mut HashSet<String>,
) -> Result<Vec<Action>, CompileError> {
    let mut lowered = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            ast::Stmt::Expression { expression, span } => {
                let expression = lower_expr(&expression, None, symbols, functions, allow_await)?;
                validate_and_record_native_disposal(
                    &expression,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                lowered.push(Action::Expression(expression));
            }
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
                validate_and_record_native_disposal(
                    &value,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                if matches!(ty, Type::Plugin { .. }) && class_has_dispose_method(ty, functions) {
                    if matches!(&value, Expr::State(existing, _) if existing == &name) {
                        // Reassigning a binding to itself preserves its identity.
                    } else if is_native_constructor_for(&value, ty) {
                        let identity = native_object_identity(&name, native_aliases);
                        if let Some((alias, _)) = native_aliases
                            .iter()
                            .find(|(_, root)| root.as_str() == identity)
                        {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "native class binding `{name}` cannot be replaced while alias `{alias}` may still refer to its disposed instance"
                                ),
                            ));
                        }
                        if !disposed_instances.remove(&identity) {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "native class instance `{name}` must be disposed before it is replaced"
                                ),
                            ));
                        }
                    } else {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "native class instance `{name}` can only be reset with a fresh constructor after disposal"
                            ),
                        ));
                    }
                }
                lowered.push(Action::Assign { name, value });
            }
            ast::Stmt::NativePropertyAssign {
                receiver,
                property,
                value,
                span,
            } => {
                let Some(receiver_type) = infer_expr_type(&receiver, symbols, functions) else {
                    return Err(CompileError::new(
                        span,
                        "native property assignment requires a native class instance",
                    ));
                };
                let Type::Plugin { name: class, .. } = &receiver_type else {
                    return Err(CompileError::new(
                        span,
                        "native property assignment requires a native class instance",
                    ));
                };
                let Some(signature) = functions.get(&format!("{class}.#property.{property}"))
                else {
                    return Err(CompileError::new(
                        span,
                        format!("native class `{class}` has no property `{property}`"),
                    ));
                };
                if signature.receiver.as_ref() != Some(&receiver_type) {
                    return Err(CompileError::new(
                        span,
                        format!("property `{property}` is not available on `{class}`"),
                    ));
                }
                if !signature.is_mutable_property {
                    return Err(CompileError::new(
                        span,
                        format!("native property `{class}.{property}` is read-only"),
                    ));
                }
                let receiver = lower_expr(
                    &receiver,
                    Some(&receiver_type),
                    symbols,
                    functions,
                    allow_await,
                )?;
                validate_and_record_native_disposal(
                    &receiver,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let value = lower_expr(
                    &value,
                    Some(&signature.return_type),
                    symbols,
                    functions,
                    allow_await,
                )?;
                validate_and_record_native_disposal(
                    &value,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                lowered.push(Action::NativePropertyAssign {
                    receiver,
                    property,
                    value,
                });
            }
            ast::Stmt::NativeEventSubscribe {
                receiver,
                event,
                parameters,
                actions,
                span,
            } => {
                let Some(receiver_type) = infer_expr_type(&receiver, symbols, functions) else {
                    return Err(CompileError::new(
                        span,
                        "native event subscription requires a native class instance",
                    ));
                };
                let Type::Plugin { name: class, .. } = &receiver_type else {
                    return Err(CompileError::new(
                        span,
                        "native event subscription requires a native class instance",
                    ));
                };
                let Some(signature) = functions.get(&format!("{class}.#event.{event}")) else {
                    return Err(CompileError::new(
                        span,
                        format!("native class `{class}` has no event `{event}`"),
                    ));
                };
                if signature.receiver.as_ref() != Some(&receiver_type) {
                    return Err(CompileError::new(
                        span,
                        format!("event `{event}` is not available on `{class}`"),
                    ));
                }
                if parameters.len() != signature.parameters.len() {
                    return Err(CompileError::new(
                        span,
                        format!(
                            "native event `{class}.{event}` provides {} value(s), but handler binds {}",
                            signature.parameters.len(),
                            parameters.len()
                        ),
                    ));
                }
                let mut event_symbols = symbols.clone();
                let mut unique_parameters =
                    std::collections::HashSet::with_capacity(parameters.len());
                for (name, (_, ty)) in parameters.iter().zip(&signature.parameters) {
                    if !unique_parameters.insert(name.as_str()) {
                        return Err(CompileError::new(
                            span,
                            format!("native event handler binds `{name}` more than once"),
                        ));
                    }
                    if symbols.contains_key(name) {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "native event handler binding `{name}` shadows an existing value"
                            ),
                        ));
                    }
                    event_symbols.insert(name.clone(), (ty.clone(), false));
                }
                let receiver = lower_expr(
                    &receiver,
                    Some(&receiver_type),
                    symbols,
                    functions,
                    allow_await,
                )?;
                validate_and_record_native_disposal(
                    &receiver,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let callback_functions = functions_with_error_handling(functions, false);
                let actions = lower_actions_with_aliases_depth(
                    actions,
                    &event_symbols,
                    &callback_functions,
                    false,
                    native_aliases,
                    0,
                )?;
                lowered.push(Action::NativeEventSubscribe {
                    receiver,
                    property: nexa_plugin_idl::event_callback_property(&event),
                    parameters,
                    actions,
                });
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
                if matches!(ty, Type::Plugin { .. }) {
                    let expression = ast::Expr::MethodCall {
                        base: Box::new(ast::Expr::Name(name, span)),
                        name: method,
                        arguments,
                        named_arguments: std::collections::BTreeMap::new(),
                        span,
                    };
                    let expression =
                        lower_expr(&expression, None, symbols, functions, allow_await)?;
                    validate_and_record_native_disposal(
                        &expression,
                        disposed_instances,
                        native_aliases,
                        functions,
                        span,
                    )?;
                    lowered.push(Action::Expression(expression));
                    continue;
                }
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
                for argument in &arguments {
                    validate_and_record_native_disposal(
                        argument,
                        disposed_instances,
                        native_aliases,
                        functions,
                        span,
                    )?;
                }
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
                span,
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    symbols,
                    functions,
                    allow_await,
                )?;
                validate_and_record_native_disposal(
                    &condition,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let mut then_disposed = disposed_instances.clone();
                let then_branch = lower_actions_with_disposal_state(
                    then_branch,
                    symbols,
                    functions,
                    allow_await,
                    native_aliases,
                    loop_depth,
                    &mut then_disposed,
                )?;
                let mut else_disposed = disposed_instances.clone();
                let else_branch = else_branch
                    .map(|branch| {
                        lower_actions_with_disposal_state(
                            branch,
                            symbols,
                            functions,
                            allow_await,
                            native_aliases,
                            loop_depth,
                            &mut else_disposed,
                        )
                    })
                    .transpose()?;
                disposed_instances.extend(then_disposed);
                disposed_instances.extend(else_disposed);
                lowered.push(Action::If {
                    condition,
                    then_branch,
                    else_branch,
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
                validate_and_record_native_disposal(
                    &iterable,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(name.clone(), (element_type, false));
                let mut body_disposed = disposed_instances.clone();
                let body = lower_actions_with_disposal_state(
                    body,
                    &loop_symbols,
                    functions,
                    allow_await,
                    native_aliases,
                    loop_depth + 1,
                    &mut body_disposed,
                )?;
                disposed_instances.extend(body_disposed);
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
                validate_and_record_native_disposal(
                    &iterable,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(key_name.clone(), ((*key_type).clone(), false));
                loop_symbols.insert(value_name.clone(), ((*value_type).clone(), false));
                let mut body_disposed = disposed_instances.clone();
                let body = lower_actions_with_disposal_state(
                    body,
                    &loop_symbols,
                    functions,
                    allow_await,
                    native_aliases,
                    loop_depth + 1,
                    &mut body_disposed,
                )?;
                disposed_instances.extend(body_disposed);
                lowered.push(Action::ForMap {
                    key_name,
                    value_name,
                    iterable,
                    body,
                });
            }
            ast::Stmt::While {
                condition,
                body,
                span,
            } => {
                let condition = lower_expr(
                    &condition,
                    Some(&Type::Bool),
                    symbols,
                    functions,
                    allow_await,
                )?;
                validate_and_record_native_disposal(
                    &condition,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let mut body_disposed = disposed_instances.clone();
                let body = lower_actions_with_disposal_state(
                    body,
                    symbols,
                    functions,
                    allow_await,
                    native_aliases,
                    loop_depth + 1,
                    &mut body_disposed,
                )?;
                disposed_instances.extend(body_disposed);
                lowered.push(Action::While { condition, body });
            }
            ast::Stmt::TryCatch {
                body,
                error_catches,
                catch_body,
                span,
            } => {
                let handled_functions = functions_with_error_handling(functions, true);
                let mut body_disposed = disposed_instances.clone();
                let body = lower_actions_with_disposal_state(
                    body,
                    symbols,
                    &handled_functions,
                    allow_await,
                    native_aliases,
                    loop_depth,
                    &mut body_disposed,
                )?;
                disposed_instances.extend(body_disposed);
                let mut seen_error_variants = HashSet::with_capacity(error_catches.len());
                let mut lowered_error_catches = Vec::with_capacity(error_catches.len());
                for arm in error_catches {
                    let key = format!("{}.{}.{}", arm.namespace, arm.error_name, arm.variant);
                    if !seen_error_variants.insert(key.clone()) {
                        return Err(CompileError::new(
                            arm.span,
                            format!("error variant `{key}` is caught more than once"),
                        ));
                    }
                    let variant = plugin_error_variant(
                        functions,
                        &arm.namespace,
                        &arm.error_name,
                        &arm.variant,
                    )
                    .ok_or_else(|| {
                        CompileError::new(
                            arm.span,
                            format!(
                                "unknown plugin error variant `{}`",
                                format!("{}.{}.{}", arm.namespace, arm.error_name, arm.variant)
                            ),
                        )
                    })?;
                    if variant.parameters.len() != arm.bindings.len() {
                        return Err(CompileError::new(
                            arm.span,
                            format!(
                                "error variant `{}.{}` provides {} payload value(s), but the catch case binds {}",
                                arm.error_name,
                                arm.variant,
                                variant.parameters.len(),
                                arm.bindings.len()
                            ),
                        ));
                    }
                    let mut arm_symbols = symbols.clone();
                    let catch_parameters = variant
                        .parameters
                        .iter()
                        .zip(&arm.bindings)
                        .map(|((payload_name, payload_type), binding)| {
                            (binding.clone(), payload_name.clone(), payload_type.clone())
                        })
                        .collect::<Vec<_>>();
                    for (binding, _, payload_type) in &catch_parameters {
                        if arm_symbols
                            .insert(binding.clone(), (payload_type.clone(), false))
                            .is_some()
                        {
                            return Err(CompileError::new(
                                arm.span,
                                format!(
                                    "catch payload `{binding}` conflicts with an existing value"
                                ),
                            ));
                        }
                    }
                    let mut arm_disposed = disposed_instances.clone();
                    let actions = lower_actions_with_disposal_state(
                        arm.body,
                        &arm_symbols,
                        functions,
                        allow_await,
                        native_aliases,
                        loop_depth,
                        &mut arm_disposed,
                    )?;
                    disposed_instances.extend(arm_disposed);
                    lowered_error_catches.push(nexa_ir::ErrorCatchArm {
                        namespace: arm.namespace,
                        error_type: arm.error_name,
                        variant: arm.variant,
                        parameters: catch_parameters,
                        body: actions,
                    });
                }
                let catch_body = catch_body
                    .map(|catch_body| {
                        let mut catch_disposed = disposed_instances.clone();
                        let lowered = lower_actions_with_disposal_state(
                            catch_body,
                            symbols,
                            functions,
                            allow_await,
                            native_aliases,
                            loop_depth,
                            &mut catch_disposed,
                        )?;
                        disposed_instances.extend(catch_disposed);
                        Ok(lowered)
                    })
                    .transpose()?;
                validate_typed_error_recovery(
                    &body,
                    &lowered_error_catches,
                    catch_body.is_some(),
                    functions,
                    span,
                )?;
                lowered.push(Action::TryCatch {
                    body,
                    error_catches: lowered_error_catches,
                    catch_body,
                });
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

pub(super) fn class_has_dispose_method(ty: &Type, functions: &FunctionSignatures) -> bool {
    let Type::Plugin { name, .. } = ty else {
        return false;
    };
    functions
        .get(&format!("{name}.dispose"))
        .is_some_and(|signature| {
            signature.receiver.as_ref() == Some(ty)
                && signature.parameters.is_empty()
                && signature.return_type == Type::Void
                && !signature.is_async
                && !signature.is_throwing
        })
}

fn validate_typed_error_recovery(
    actions: &[Action],
    catches: &[nexa_ir::ErrorCatchArm],
    has_catch_all: bool,
    functions: &FunctionSignatures,
    span: Span,
) -> Result<(), CompileError> {
    if has_catch_all {
        return Ok(());
    }

    let mut typed_errors = HashMap::new();
    let mut has_untyped_throw = false;
    visit_action_expressions(actions, &mut |expression| {
        let Expr::NativeCall {
            receiver,
            namespace,
            name,
            is_throwing: true,
            ..
        } = expression
        else {
            return;
        };
        let key = match receiver.as_deref() {
            Some(Expr::State(
                _,
                Type::Plugin {
                    name: class_name, ..
                },
            )) => {
                format!("{class_name}.{name}")
            }
            _ => format!("{namespace}.{name}"),
        };
        let Some(error_type) = functions
            .get(&key)
            .and_then(|signature| signature.error_type.as_ref())
        else {
            has_untyped_throw = true;
            return;
        };
        let name = format!("{}.{}", error_type.namespace, error_type.name);
        typed_errors
            .entry(name)
            .or_insert_with(|| error_type.clone());
    });

    if has_untyped_throw {
        return Err(CompileError::new(
            span,
            "a catch-all `else` block is required when the try body can throw an untyped error",
        ));
    }
    if typed_errors.len() > 1 {
        return Err(CompileError::new(
            span,
            "a catch-all `else` block is required when the try body can throw more than one error type",
        ));
    }

    for (error_name, error_type) in typed_errors {
        let caught = catches
            .iter()
            .filter(|arm| {
                arm.namespace == error_type.namespace && arm.error_type == error_type.name
            })
            .map(|arm| arm.variant.as_str())
            .collect::<HashSet<_>>();
        if caught.len() != error_type.variants.len() {
            return Err(CompileError::new(
                span,
                format!(
                    "catch every variant of `{error_name}` or add an `else` block to handle the remaining errors"
                ),
            ));
        }
    }

    Ok(())
}

fn visit_action_expressions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Expression(expression)
            | Action::Assign {
                value: expression, ..
            } => {
                nexa_ir::walk::walk_expression(expression, visit);
            }
            Action::NativePropertyAssign {
                receiver, value, ..
            } => {
                nexa_ir::walk::walk_expression(receiver, visit);
                nexa_ir::walk::walk_expression(value, visit);
            }
            Action::NativeEventSubscribe { receiver, .. } => {
                nexa_ir::walk::walk_expression(receiver, visit);
            }
            Action::CollectionMutation { arguments, .. } => {
                for argument in arguments {
                    nexa_ir::walk::walk_expression(argument, visit);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                nexa_ir::walk::walk_expression(condition, visit);
                visit_action_expressions(then_branch, visit);
                if let Some(else_branch) = else_branch {
                    visit_action_expressions(else_branch, visit);
                }
            }
            Action::For { iterable, body, .. } | Action::ForMap { iterable, body, .. } => {
                nexa_ir::walk::walk_expression(iterable, visit);
                visit_action_expressions(body, visit);
            }
            Action::While { condition, body } => {
                nexa_ir::walk::walk_expression(condition, visit);
                visit_action_expressions(body, visit);
            }
            // Nested try/catch actions own their own error handling scope.
            Action::TryCatch { .. } | Action::Break | Action::Continue => {}
        }
    }
}

fn is_native_constructor_for(expression: &Expr, ty: &Type) -> bool {
    matches!(
        (expression, ty),
        (
            Expr::Call {
                is_constructor: true,
                return_type,
                ..
            },
            Type::Plugin { .. }
        ) if return_type == ty
    )
}

fn validate_and_record_native_disposal(
    expression: &Expr,
    disposed_instances: &mut HashSet<String>,
    native_aliases: &HashMap<String, String>,
    functions: &FunctionSignatures,
    span: Span,
) -> Result<(), CompileError> {
    let mut referenced_disposed = HashSet::new();
    let mut dispose_calls = HashSet::new();
    nexa_ir::walk::walk_expression(expression, &mut |expression| match expression {
        Expr::State(name, _) => {
            let identity = native_object_identity(name, native_aliases);
            if disposed_instances.contains(&identity) {
                referenced_disposed.insert(identity);
            }
        }
        Expr::NativeCall {
            receiver: Some(receiver),
            name: method,
            ..
        } if method == "dispose" => {
            if let Expr::State(name, ty) = receiver.as_ref()
                && class_has_dispose_method(ty, functions)
            {
                dispose_calls.insert(native_object_identity(name, native_aliases));
            }
        }
        _ => {}
    });

    for identity in &dispose_calls {
        if let Some(parameter) = identity.strip_prefix(BORROWED_NATIVE_IDENTITY_PREFIX) {
            return Err(CompileError::new(
                span,
                format!(
                    "native class component parameter `{parameter}` is borrowed and cannot be disposed here; dispose it from its owning screen or app"
                ),
            ));
        }
    }

    for name in referenced_disposed {
        let message = if dispose_calls.contains(&name) {
            format!("native class instance `{name}` may be disposed more than once")
        } else {
            format!("native class instance `{name}` may be used after disposal")
        };
        return Err(CompileError::new(span, message));
    }
    disposed_instances.extend(dispose_calls);
    Ok(())
}

const BORROWED_NATIVE_IDENTITY_PREFIX: &str = "@borrowed-native-parameter:";

fn native_object_identity(name: &str, aliases: &HashMap<String, String>) -> String {
    aliases
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_owned())
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use nexa_diagnostics::Span;
    use nexa_ir::{Action, Expr, NativeComponentEventHandler, NumericType, Type};
    use nexa_syntax::ast;

    use super::{
        FunctionSignatures, lower_actions_with_aliases, lower_actions_with_depth, lower_node,
    };
    use crate::Target;
    use crate::semantic::custom_components::{
        ComponentEventSignature, ComponentSignature, ComponentSignatures,
    };
    use crate::semantic::expressions::{
        FunctionSignature, PluginErrorType, PluginErrorVariant, record_native_alias,
    };
    use crate::semantic::themes::ThemeSymbols;

    fn fixture(mutable: bool) -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
        let signature = FunctionSignature {
            parameters: Vec::new(),
            return_type: Type::Numeric(NumericType::Float64),
            is_async: false,
            is_throwing: false,
            receiver: Some(player_type.clone()),
            is_constructor: false,
            is_mutable_property: mutable,
            error_handling_allowed: false,
            error_type: None,
        };
        let functions = HashMap::from([("VideoPlayer.#property.volume".to_owned(), signature)]);
        (symbols, functions)
    }

    fn assignment() -> ast::Stmt {
        ast::Stmt::NativePropertyAssign {
            receiver: ast::Expr::Name("player".to_owned(), Span::default()),
            property: "volume".to_owned(),
            value: ast::Expr::Number("0.5".to_owned(), Span::default()),
            span: Span::default(),
        }
    }

    fn assignment_with_value(value: ast::Expr) -> ast::Stmt {
        ast::Stmt::NativePropertyAssign {
            receiver: ast::Expr::Name("player".to_owned(), Span::default()),
            property: "volume".to_owned(),
            value,
            span: Span::default(),
        }
    }

    fn event_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
        let functions = HashMap::from([(
            "VideoPlayer.#event.progressChanged".to_owned(),
            FunctionSignature {
                parameters: vec![
                    ("position".to_owned(), Type::Numeric(NumericType::Float64)),
                    ("duration".to_owned(), Type::Numeric(NumericType::Float64)),
                ],
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
                receiver: Some(player_type),
                is_constructor: false,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        )]);
        (symbols, functions)
    }

    fn progress_event(parameters: Vec<String>) -> ast::Stmt {
        ast::Stmt::NativeEventSubscribe {
            receiver: ast::Expr::Name("player".to_owned(), Span::default()),
            event: "progressChanged".to_owned(),
            parameters,
            actions: vec![ast::Stmt::Expression {
                expression: ast::Expr::Name("position".to_owned(), Span::default()),
                span: Span::default(),
            }],
            span: Span::default(),
        }
    }

    fn native_component_event_node(property: &str, parameters: Vec<String>) -> ast::Node {
        ast::Node::NativeComponentCall {
            namespace: "Video".to_owned(),
            name: "VideoView".to_owned(),
            arguments: BTreeMap::new(),
            children: None,
            event_handlers: vec![ast::NativeComponentEventHandler {
                property: property.to_owned(),
                parameters,
                actions: vec![ast::Stmt::Expression {
                    expression: ast::Expr::Name("position".to_owned(), Span::default()),
                    span: Span::default(),
                }],
                span: Span::default(),
            }],
            span: Span::default(),
        }
    }

    fn native_component_signatures() -> ComponentSignatures {
        ComponentSignatures::from([(
            "Video.VideoView".to_owned(),
            ComponentSignature {
                parameters: Vec::new(),
                defaults: HashMap::new(),
                events: vec![ComponentEventSignature {
                    property: "onProgressChanged".to_owned(),
                    parameters: vec![
                        ("position".to_owned(), Type::Numeric(NumericType::Float64)),
                        ("duration".to_owned(), Type::Numeric(NumericType::Float64)),
                    ],
                }],
                has_content_slot: false,
                native: true,
            },
        )])
    }

    fn lower_native_component_event(
        property: &str,
        parameters: Vec<String>,
    ) -> Result<nexa_ir::Node, nexa_diagnostics::CompileError> {
        lower_node(
            native_component_event_node(property, parameters),
            &HashMap::new(),
            &HashMap::new(),
            &ThemeSymbols::default(),
            &native_component_signatures(),
            &HashMap::new(),
            &HashMap::new(),
            false,
            false,
            Target::Swift,
        )
    }

    fn disposal_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let symbols = HashMap::from([("player".to_owned(), (player_type.clone(), false))]);
        let method = |return_type| FunctionSignature {
            parameters: Vec::new(),
            return_type,
            is_async: false,
            is_throwing: false,
            receiver: Some(player_type.clone()),
            is_constructor: false,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: None,
        };
        let functions = HashMap::from([
            ("VideoPlayer.dispose".to_owned(), method(Type::Void)),
            ("VideoPlayer.play".to_owned(), method(Type::Void)),
        ]);
        (symbols, functions)
    }

    fn native_method_call(method: &str) -> ast::Stmt {
        native_method_call_on("player", method)
    }

    fn native_method_call_on(name: &str, method: &str) -> ast::Stmt {
        ast::Stmt::CollectionMutation {
            name: name.to_owned(),
            method: method.to_owned(),
            arguments: Vec::new(),
            span: Span::default(),
        }
    }

    fn throwing_prepare() -> ast::Expr {
        ast::Expr::Await(
            Box::new(ast::Expr::MethodCall {
                base: Box::new(ast::Expr::Name("player".to_owned(), Span::default())),
                name: "prepare".to_owned(),
                arguments: Vec::new(),
                named_arguments: BTreeMap::new(),
                span: Span::default(),
            }),
            Span::default(),
        )
    }

    fn throwing_file_read() -> ast::Expr {
        let span = Span::default();
        ast::Expr::Await(
            Box::new(ast::Expr::QualifiedCall {
                namespace: "File".to_owned(),
                name: "readText".to_owned(),
                arguments: BTreeMap::from([(
                    "path".to_owned(),
                    ast::Expr::String("notes.txt".to_owned(), span),
                )]),
                span,
            }),
            span,
        )
    }

    fn throwing_fixture() -> (HashMap<String, (Type, bool)>, FunctionSignatures) {
        let player_type = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let error_type = PluginErrorType {
            namespace: "Video".to_owned(),
            name: "PlayerError".to_owned(),
            variants: vec![
                PluginErrorVariant {
                    name: "invalidUrl".to_owned(),
                    parameters: Vec::new(),
                },
                PluginErrorVariant {
                    name: "decodingFailed".to_owned(),
                    parameters: vec![("message".to_owned(), Type::String)],
                },
            ],
        };
        let signature = |receiver| FunctionSignature {
            parameters: Vec::new(),
            return_type: Type::Void,
            is_async: true,
            is_throwing: true,
            receiver,
            is_constructor: false,
            is_mutable_property: false,
            error_handling_allowed: false,
            error_type: Some(error_type.clone()),
        };
        (
            HashMap::from([
                ("player".to_owned(), (player_type.clone(), false)),
                ("failed".to_owned(), (Type::Bool, true)),
            ]),
            HashMap::from([(
                "VideoPlayer.prepare".to_owned(),
                signature(Some(player_type)),
            )]),
        )
    }

    #[test]
    fn lowers_mutable_native_property_assignment() {
        let (symbols, functions) = fixture(true);
        let actions = lower_actions_with_depth(vec![assignment()], &symbols, &functions, false, 0)
            .expect("mutable plugin property should be assignable");

        assert!(matches!(
            &actions[0],
            Action::NativePropertyAssign {
                receiver: Expr::State(name, Type::Plugin { name: class, .. }),
                property,
                value: Expr::Number { raw, ty: NumericType::Float64 },
            } if name == "player" && class == "VideoPlayer" && property == "volume" && raw == "0.5"
        ));
    }

    #[test]
    fn rejects_assignment_to_readonly_native_property() {
        let (symbols, functions) = fixture(false);
        let error = lower_actions_with_depth(vec![assignment()], &symbols, &functions, false, 0)
            .expect_err("readonly plugin property must not be assignable");
        assert!(error.to_string().contains("read-only"));
    }

    #[test]
    fn rejects_native_property_assignment_with_the_wrong_type() {
        let (symbols, functions) = fixture(true);
        let statement = assignment_with_value(ast::Expr::Bool(true, Span::default()));
        let error = lower_actions_with_depth(vec![statement], &symbols, &functions, false, 0)
            .expect_err("a Bool must not be assigned to a Float64 property");
        assert!(error.to_string().contains("Bool"));
        assert!(error.to_string().contains("Float64"));
    }

    #[test]
    fn lowers_instance_event_with_typed_payload_bindings() {
        let (symbols, functions) = event_fixture();
        let actions = lower_actions_with_depth(
            vec![progress_event(vec![
                "position".to_owned(),
                "duration".to_owned(),
            ])],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect("declared native event should lower");

        assert!(matches!(
            &actions[0],
            Action::NativeEventSubscribe {
                receiver: Expr::State(receiver, Type::Plugin { name: class, .. }),
                property,
                parameters,
                actions: handler,
            } if receiver == "player" && class == "VideoPlayer"
                && property == "onProgressChanged"
                && parameters == &["position", "duration"]
                && matches!(
                    &handler[0],
                    Action::Expression(Expr::State(name, Type::Numeric(NumericType::Float64)))
                        if name == "position"
                )
        ));
    }

    #[test]
    fn rejects_event_handlers_with_the_wrong_payload_arity() {
        let (symbols, functions) = event_fixture();
        let error = lower_actions_with_depth(
            vec![progress_event(vec!["position".to_owned()])],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect_err("a two-value event must not accept one handler binding");
        assert!(error.to_string().contains("provides 2 value(s)"));
    }

    #[test]
    fn rejects_event_payload_bindings_that_shadow_existing_values() {
        let (mut symbols, functions) = event_fixture();
        symbols.insert("position".to_owned(), (Type::String, false));
        let error = lower_actions_with_depth(
            vec![progress_event(vec![
                "position".to_owned(),
                "duration".to_owned(),
            ])],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect_err("event bindings must not capture an existing value ambiguously");
        assert!(error.to_string().contains("shadows an existing value"));
    }

    #[test]
    fn lowers_native_component_event_with_typed_payload_bindings() {
        let node = lower_native_component_event(
            "onProgressChanged",
            vec!["position".to_owned(), "duration".to_owned()],
        )
        .expect("declared native component event should lower");

        assert!(matches!(
            node,
            nexa_ir::Node::NativeComponentCall { event_handlers, .. }
                if matches!(
                    event_handlers.as_slice(),
                    [NativeComponentEventHandler {
                        property,
                        parameters,
                        actions,
                    }] if property == "onProgressChanged"
                        && parameters == &["position", "duration"]
                        && matches!(
                            actions.as_slice(),
                            [Action::Expression(Expr::State(name, Type::Numeric(NumericType::Float64)))]
                                if name == "position"
                        )
                )
        ));
    }

    #[test]
    fn rejects_native_component_event_with_unknown_callback() {
        let error = lower_native_component_event("onMissing", Vec::new())
            .expect_err("unknown native component events must not be ignored");
        assert!(
            error
                .to_string()
                .contains("has no event callback `onMissing`")
        );
    }

    #[test]
    fn rejects_native_component_event_with_wrong_payload_arity() {
        let error = lower_native_component_event("onProgressChanged", vec!["position".to_owned()])
            .expect_err("component event handler must bind every payload value");
        assert!(error.to_string().contains("provides 2 value(s)"));
    }

    #[test]
    fn throwing_native_calls_need_and_use_an_explicit_recovery_block() {
        let (symbols, functions) = throwing_fixture();
        let actions = lower_actions_with_depth(
            vec![ast::Stmt::TryCatch {
                body: vec![ast::Stmt::Expression {
                    expression: throwing_prepare(),
                    span: Span::default(),
                }],
                error_catches: Vec::new(),
                catch_body: Some(vec![ast::Stmt::Assign {
                    name: "failed".to_owned(),
                    value: ast::Expr::Bool(true, Span::default()),
                    span: Span::default(),
                }]),
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect("a try block should lower a throwing native call");

        assert!(matches!(
            actions.as_slice(),
            [Action::TryCatch {
                body,
                error_catches,
                catch_body,
            }] if matches!(
                body.as_slice(),
                [Action::Expression(Expr::TryAwait(call))]
                    if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
            ) && error_catches.is_empty()
                && matches!(catch_body.as_deref(), Some([Action::Assign { name, .. }]) if name == "failed")
        ));

        let unhandled = lower_actions_with_depth(
            vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("a throwing call outside try/catch must remain a semantic error");
        assert!(unhandled.to_string().contains("may throw"));
    }

    #[test]
    fn typed_catch_cases_bind_declared_error_payloads_with_native_types() {
        let (mut symbols, functions) = throwing_fixture();
        symbols.insert("messageOut".to_owned(), (Type::String, true));
        let actions = lower_actions_with_depth(
            vec![ast::Stmt::TryCatch {
                body: vec![ast::Stmt::Expression {
                    expression: throwing_prepare(),
                    span: Span::default(),
                }],
                error_catches: vec![
                    ast::ErrorCatchArm {
                        namespace: "Video".to_owned(),
                        error_name: "PlayerError".to_owned(),
                        variant: "invalidUrl".to_owned(),
                        bindings: Vec::new(),
                        body: vec![ast::Stmt::Assign {
                            name: "failed".to_owned(),
                            value: ast::Expr::Bool(true, Span::default()),
                            span: Span::default(),
                        }],
                        span: Span::default(),
                    },
                    ast::ErrorCatchArm {
                        namespace: "Video".to_owned(),
                        error_name: "PlayerError".to_owned(),
                        variant: "decodingFailed".to_owned(),
                        bindings: vec!["message".to_owned()],
                        body: vec![ast::Stmt::Assign {
                            name: "messageOut".to_owned(),
                            value: ast::Expr::Name("message".to_owned(), Span::default()),
                            span: Span::default(),
                        }],
                        span: Span::default(),
                    },
                ],
                catch_body: None,
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect("declared plugin error cases and payloads should lower");

        assert!(matches!(
            actions.as_slice(),
            [Action::TryCatch {
                error_catches,
                catch_body: None,
                ..
            }] if matches!(error_catches.as_slice(), [
                nexa_ir::ErrorCatchArm { variant, parameters, .. },
                nexa_ir::ErrorCatchArm { variant: payload_variant, parameters: payload_parameters, body, .. }
            ] if variant == "invalidUrl"
                && parameters.is_empty()
                && payload_variant == "decodingFailed"
                && payload_parameters == &[("message".to_owned(), "message".to_owned(), Type::String)]
                && matches!(body.as_slice(), [Action::Assign {
                    name,
                    value: Expr::State(payload, Type::String),
                }] if name == "messageOut" && payload == "message"))
        ));
    }

    #[test]
    fn typed_catch_cases_must_be_exhaustive_without_an_else_branch() {
        let (symbols, functions) = throwing_fixture();
        let error = lower_actions_with_depth(
            vec![ast::Stmt::TryCatch {
                body: vec![ast::Stmt::Expression {
                    expression: throwing_prepare(),
                    span: Span::default(),
                }],
                error_catches: vec![ast::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_name: "PlayerError".to_owned(),
                    variant: "invalidUrl".to_owned(),
                    bindings: Vec::new(),
                    body: Vec::new(),
                    span: Span::default(),
                }],
                catch_body: None,
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("unmatched typed error variants need an explicit fallback");

        assert!(
            error
                .to_string()
                .contains("catch every variant of `Video.PlayerError`")
        );
    }

    #[test]
    fn typed_catch_cases_require_else_for_untyped_failures() {
        let (symbols, functions) = throwing_fixture();
        let error = lower_actions_with_depth(
            vec![ast::Stmt::TryCatch {
                body: vec![
                    ast::Stmt::Expression {
                        expression: throwing_prepare(),
                        span: Span::default(),
                    },
                    ast::Stmt::Expression {
                        expression: throwing_file_read(),
                        span: Span::default(),
                    },
                ],
                error_catches: vec![
                    ast::ErrorCatchArm {
                        namespace: "Video".to_owned(),
                        error_name: "PlayerError".to_owned(),
                        variant: "invalidUrl".to_owned(),
                        bindings: Vec::new(),
                        body: Vec::new(),
                        span: Span::default(),
                    },
                    ast::ErrorCatchArm {
                        namespace: "Video".to_owned(),
                        error_name: "PlayerError".to_owned(),
                        variant: "decodingFailed".to_owned(),
                        bindings: vec!["message".to_owned()],
                        body: Vec::new(),
                        span: Span::default(),
                    },
                ],
                catch_body: None,
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("typed cases cannot handle a built-in untyped error");

        assert!(
            error
                .to_string()
                .contains("catch-all `else` block is required")
        );
    }

    #[test]
    fn typed_catch_cases_reject_unknown_variants_and_wrong_payload_arity() {
        let (symbols, functions) = throwing_fixture();
        let catch = |variant: &str, bindings: Vec<String>| ast::Stmt::TryCatch {
            body: vec![ast::Stmt::Expression {
                expression: throwing_prepare(),
                span: Span::default(),
            }],
            error_catches: vec![ast::ErrorCatchArm {
                namespace: "Video".to_owned(),
                error_name: "PlayerError".to_owned(),
                variant: variant.to_owned(),
                bindings,
                body: Vec::new(),
                span: Span::default(),
            }],
            catch_body: None,
            span: Span::default(),
        };

        let unknown = lower_actions_with_depth(
            vec![catch("notAPlayerError", Vec::new())],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("error patterns must reference a declared error variant");
        assert!(unknown.to_string().contains("unknown plugin error variant"));

        let missing_payload = lower_actions_with_depth(
            vec![catch("decodingFailed", Vec::new())],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("payload variants must bind their declared values");
        assert!(
            missing_payload
                .to_string()
                .contains("provides 1 payload value(s)")
        );
    }

    #[test]
    fn throwing_file_calls_preserve_failures_only_inside_recovery_blocks() {
        let symbols = HashMap::new();
        let functions = FunctionSignatures::new();
        let actions = lower_actions_with_depth(
            vec![ast::Stmt::TryCatch {
                body: vec![ast::Stmt::Expression {
                    expression: throwing_file_read(),
                    span: Span::default(),
                }],
                error_catches: Vec::new(),
                catch_body: Some(Vec::new()),
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect("File.readText should preserve its native error inside try/catch");

        assert!(matches!(
            actions.as_slice(),
            [Action::TryCatch { body, .. }]
                if matches!(
                    body.as_slice(),
                    [Action::Expression(Expr::TryAwait(call))]
                        if matches!(call.as_ref(), Expr::NativeCall {
                            namespace,
                            name,
                            is_throwing: true,
                            ..
                        } if namespace == "File" && name == "readText")
                )
        ));

        let unhandled = lower_actions_with_depth(
            vec![ast::Stmt::Expression {
                expression: throwing_file_read(),
                span: Span::default(),
            }],
            &symbols,
            &functions,
            true,
            0,
        )
        .expect_err("File.readText must not silently discard native failures");
        assert!(unhandled.to_string().contains("may throw"));
    }

    #[test]
    fn try_catch_does_not_handle_errors_from_later_native_callbacks() {
        let (symbols, mut functions) = throwing_fixture();
        let player_type = symbols["player"].0.clone();
        functions.insert(
            "VideoPlayer.#event.ended".to_owned(),
            FunctionSignature {
                parameters: Vec::new(),
                return_type: Type::Void,
                is_async: false,
                is_throwing: false,
                receiver: Some(player_type),
                is_constructor: false,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        );
        let actions = vec![ast::Stmt::TryCatch {
            body: vec![ast::Stmt::NativeEventSubscribe {
                receiver: ast::Expr::Name("player".to_owned(), Span::default()),
                event: "ended".to_owned(),
                parameters: Vec::new(),
                actions: vec![ast::Stmt::Expression {
                    expression: throwing_prepare(),
                    span: Span::default(),
                }],
                span: Span::default(),
            }],
            error_catches: Vec::new(),
            catch_body: Some(Vec::new()),
            span: Span::default(),
        }];
        let error = lower_actions_with_depth(actions, &symbols, &functions, true, 0)
            .expect_err("an outer try block cannot catch a later callback failure");
        assert!(
            error
                .to_string()
                .contains("`await` is only allowed in an async function")
        );
    }

    #[test]
    fn native_component_content_blocks_must_match_the_declared_slot() {
        let mut signatures = native_component_signatures();
        signatures
            .get_mut("Video.VideoView")
            .expect("component fixture should exist")
            .has_content_slot = true;
        let error = lower_node(
            native_component_event_node(
                "onProgressChanged",
                vec!["position".to_owned(), "duration".to_owned()],
            ),
            &HashMap::new(),
            &HashMap::new(),
            &ThemeSymbols::default(),
            &signatures,
            &HashMap::new(),
            &HashMap::new(),
            false,
            false,
            Target::Swift,
        )
        .expect_err("a declared content slot requires a child block");
        assert!(error.to_string().contains("requires a child block"));

        let mut node = native_component_event_node(
            "onProgressChanged",
            vec!["position".to_owned(), "duration".to_owned()],
        );
        if let ast::Node::NativeComponentCall { children, .. } = &mut node {
            *children = Some(Vec::new());
        }
        let error = lower_node(
            node,
            &HashMap::new(),
            &HashMap::new(),
            &ThemeSymbols::default(),
            &native_component_signatures(),
            &HashMap::new(),
            &HashMap::new(),
            false,
            false,
            Target::Swift,
        )
        .expect_err("a child block without a declared slot must be rejected");
        assert!(
            error
                .to_string()
                .contains("does not declare a content slot")
        );
    }

    #[test]
    fn rejects_native_instance_use_after_disposal() {
        let (symbols, functions) = disposal_fixture();
        let error = lower_actions_with_depth(
            vec![native_method_call("dispose"), native_method_call("play")],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect_err("native methods must not be called after dispose");

        assert!(error.to_string().contains("used after disposal"));
    }

    #[test]
    fn disposal_analysis_tracks_aliases_to_the_same_native_instance() {
        let (mut symbols, functions) = disposal_fixture();
        let player_type = symbols["player"].0.clone();
        symbols.insert("playerAlias".to_owned(), (player_type, false));
        let mut aliases = HashMap::new();
        let player = Expr::State("player".to_owned(), symbols["player"].0.clone());
        record_native_alias("playerAlias", &symbols["player"].0, &player, &mut aliases);
        let alias = Expr::State("playerAlias".to_owned(), symbols["playerAlias"].0.clone());
        record_native_alias(
            "playerAlias2",
            &symbols["playerAlias"].0,
            &alias,
            &mut aliases,
        );
        symbols.insert(
            "playerAlias2".to_owned(),
            (symbols["player"].0.clone(), false),
        );
        assert_eq!(
            aliases.get("playerAlias2").map(String::as_str),
            Some("player")
        );

        let use_after_dispose = lower_actions_with_aliases(
            vec![
                native_method_call("dispose"),
                native_method_call_on("playerAlias", "play"),
            ],
            &symbols,
            &functions,
            false,
            &aliases,
        )
        .expect_err("disposing an object must invalidate every binding alias");
        assert!(
            use_after_dispose
                .to_string()
                .contains("native class instance `player` may be used after disposal")
        );

        let double_dispose = lower_actions_with_aliases(
            vec![
                native_method_call("dispose"),
                native_method_call_on("playerAlias2", "dispose"),
            ],
            &symbols,
            &functions,
            false,
            &aliases,
        )
        .expect_err("disposing through an alias must count as a second disposal");
        assert!(
            double_dispose
                .to_string()
                .contains("native class instance `player` may be disposed more than once")
        );
    }

    #[test]
    fn native_binding_with_alias_cannot_be_replaced_after_disposal() {
        let (mut symbols, mut functions) = disposal_fixture();
        symbols.get_mut("player").expect("fixture state").1 = true;
        let player_type = symbols["player"].0.clone();
        symbols.insert("playerAlias".to_owned(), (player_type.clone(), false));
        let mut aliases = HashMap::new();
        let player = Expr::State("player".to_owned(), player_type.clone());
        record_native_alias("playerAlias", &player_type, &player, &mut aliases);
        functions.insert(
            "Video.VideoPlayer".to_owned(),
            FunctionSignature {
                parameters: Vec::new(),
                return_type: player_type,
                is_async: false,
                is_throwing: false,
                receiver: None,
                is_constructor: true,
                is_mutable_property: false,
                error_handling_allowed: false,
                error_type: None,
            },
        );
        let fresh_player = ast::Expr::QualifiedCall {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
            arguments: BTreeMap::new(),
            span: Span::default(),
        };
        let reset = ast::Stmt::Assign {
            name: "player".to_owned(),
            value: fresh_player,
            span: Span::default(),
        };
        let error = lower_actions_with_aliases(
            vec![native_method_call("dispose"), reset],
            &symbols,
            &functions,
            false,
            &aliases,
        )
        .expect_err("replacing the source binding would leave its alias stale");
        assert!(error.to_string().contains("cannot be replaced while alias"));
    }

    #[test]
    fn rejects_disposing_the_same_native_instance_twice() {
        let (symbols, functions) = disposal_fixture();
        let error = lower_actions_with_depth(
            vec![native_method_call("dispose"), native_method_call("dispose")],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect_err("native instances must not be disposed twice");

        assert!(error.to_string().contains("disposed more than once"));
    }

    #[test]
    fn propagates_possible_disposal_from_conditional_branches() {
        let (symbols, functions) = disposal_fixture();
        let conditional_dispose = ast::Stmt::If {
            condition: ast::Expr::Bool(true, Span::default()),
            then_branch: vec![native_method_call("dispose")],
            else_branch: None,
            span: Span::default(),
        };
        let error = lower_actions_with_depth(
            vec![conditional_dispose, native_method_call("play")],
            &symbols,
            &functions,
            false,
            0,
        )
        .expect_err("use after a conditional dispose must be rejected");

        assert!(error.to_string().contains("used after disposal"));
    }
}
