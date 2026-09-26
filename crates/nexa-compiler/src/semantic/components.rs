use std::collections::{BTreeMap, HashMap, HashSet};

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::walk::any_node;
use nexa_ir::{
    AccessibilityRole, Action, BottomBarTab, Capitalization, CollectionMutation, DirectionConfig,
    DirectionStyle, Expr, FastListRefresh, FontWeight, HapticStyle, ImageScale, ImageSource,
    KeyboardDismissMode, KeyboardType, LayoutKind, ListAxis, ListCommon, ListPlan,
    NativeComponentEventHandler, Node, NumericType, ScreenId, SectionedListCommon, StatusBarConfig,
    StatusBarStyle, TextStyle, Type, WhenCase,
};
use nexa_syntax::{ast, catalog};

use super::{
    context::{ExprContext, ScreenSignatures, SemanticContext},
    expressions::{
        FunctionSignatures, functions_with_error_handling, infer_expr_type, lower_expr,
        plugin_error_variant, type_name,
    },
    styles::{lower_style, optional_color, optional_dimension, parse_color_literal},
};
use crate::Target;

pub(super) fn lower_nodes(
    nodes: Vec<ast::Node>,
    cx: &SemanticContext,
) -> Result<Vec<Node>, CompileError> {
    let mut lowered = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            ast::Node::Platform {
                target: platform,
                children,
                ..
            } => {
                if cx.target == Target::All || platform_matches(platform, cx.target) {
                    lowered.extend(lower_nodes(children, cx)?);
                }
            }
            node => {
                let allow_navigation = cx.allow_navigation_stack
                    && if let ast::Node::ComponentInvocation(invocation) = &node {
                        invocation.name == "NavigationStack"
                    } else {
                        false
                    };
                let child_cx = cx.with_navigation(allow_navigation, cx.allow_navigation_back);
                lowered.push(lower_node(node, &child_cx)?);
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

/// Look up the catalog schema for a parsed invocation. The parser only
/// produces invocations for catalogued names, so this never fails on parser
/// output; it keeps lowering total over programmatically built syntax trees.
fn invocation_schema(
    inv: &ast::ComponentInvocation,
) -> Result<&'static catalog::ComponentSchema, CompileError> {
    catalog::component_schema(&inv.name)
        .ok_or_else(|| CompileError::new(inv.span, format!("unknown component `{}`", inv.name)))
}

/// Remove a required named option, reporting the catalog's missing-option
/// diagnostic when absent.
fn take_required_arg(
    args: &mut BTreeMap<String, ast::Expr>,
    component: &str,
    name: &str,
    span: Span,
) -> Result<ast::Expr, CompileError> {
    match args.remove(name) {
        Some(value) => Ok(value),
        None => Err(CompileError::new(
            span,
            catalog::component_schema(component)
                .and_then(|schema| {
                    schema
                        .arguments
                        .iter()
                        .find(|arg| arg.name == name)
                        .map(|arg| catalog::required_message(schema, arg))
                })
                .unwrap_or_else(|| format!("{component} requires `{name}`")),
        )),
    }
}

/// Take the leading positional expression of a `Single` invocation.
fn take_positional(
    positional: &mut Vec<ast::Expr>,
    span: Span,
    component: &str,
) -> Result<ast::Expr, CompileError> {
    if positional.is_empty() {
        return Err(CompileError::new(
            span,
            format!("{component} requires a primary value expression"),
        ));
    }
    Ok(positional.remove(0))
}

/// Remove one trailing dot-modifier body by name.
fn take_modifier(modifiers: &mut Vec<ast::DotModifier>, name: &str) -> Option<ast::ModifierBody> {
    modifiers
        .iter()
        .position(|modifier| modifier.name == name)
        .map(|index| modifiers.remove(index).body)
}

/// Take an optional action-block modifier body.
fn take_modifier_actions(
    modifiers: &mut Vec<ast::DotModifier>,
    span: Span,
    name: &str,
) -> Result<Option<Vec<ast::Stmt>>, CompileError> {
    match take_modifier(modifiers, name) {
        Some(ast::ModifierBody::Actions(actions)) => Ok(Some(actions)),
        Some(_) => Err(child_mismatch(span)),
        None => Ok(None),
    }
}

/// Take an optional node-block modifier body.
fn take_modifier_nodes(
    modifiers: &mut Vec<ast::DotModifier>,
    span: Span,
    name: &str,
) -> Result<Option<Vec<ast::Node>>, CompileError> {
    match take_modifier(modifiers, name) {
        Some(ast::ModifierBody::Nodes(nodes)) => Ok(Some(nodes)),
        Some(_) => Err(child_mismatch(span)),
        None => Ok(None),
    }
}

/// Defensive mismatch error for child-block shapes the parser never produces
/// for a given schema; keeps lowering total without panicking.
fn child_mismatch(span: Span) -> CompileError {
    CompileError::new(
        span,
        "malformed component invocation: unexpected child-block shape",
    )
}

/// Validated FastList source bindings collected during lowering, before the
/// row children are lowered. Each variant carries exactly the bindings its
/// source provides; the final assembly wraps them in a [`ListPlan`].
enum ListSourceParts {
    Count {
        count: Expr,
    },
    Items {
        collection: Expr,
        element_type: Type,
        item: String,
    },
    Sections {
        collection: Expr,
        element_type: Type,
        section: String,
        item: String,
    },
}

pub(super) fn lower_node(node: ast::Node, cx: &SemanticContext) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let screen_ids = cx.screen_ids;
    let themes = cx.themes;
    let components = cx.components;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;
    let allow_navigation_stack = cx.allow_navigation_stack;
    let allow_navigation_back = cx.allow_navigation_back;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);
    match node {
        ast::Node::Platform { .. } => {
            unreachable!("platform blocks are expanded by lower_nodes")
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Content" => Ok(Node::Content),
        ast::Node::ComponentInvocation(inv) if inv.name == "StatusBar" => {
            let mut args = inv.arguments;
            let style = args.remove("style");
            let hidden = args.remove("hidden");
            let background = args.remove("background");
            Ok(Node::StatusBar {
                config: lower_status_bar(style, hidden, background)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Direction" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let value = take_required_arg(&mut args, &inv.name, "value", span)?;
            Ok(Node::Direction {
                config: lower_direction(value)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnAppear" => {
            let span = inv.span;
            let asynchronous = inv.flags.iter().any(|flag| flag == "async");
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::OnAppear {
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    asynchronous,
                    native_aliases,
                )?,
                asynchronous,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnDisappear" => {
            let span = inv.span;
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::OnDisappear {
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                )?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnActive" => {
            let span = inv.span;
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::OnActive {
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                )?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnInactive" => {
            let span = inv.span;
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::OnInactive {
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                )?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnBackground" => {
            let span = inv.span;
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::OnBackground {
                actions: lower_actions_with_aliases(
                    actions,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                )?,
            })
        }
        ast::Node::ComponentInvocation(inv)
            if inv.name == "Column" || inv.name == "Row" || inv.name == "Stack" =>
        {
            let span = inv.span;
            let stacked = inv.name == "Stack";
            let kind = match inv.name.as_str() {
                "Column" => LayoutKind::Column,
                "Row" => LayoutKind::Row,
                _ => LayoutKind::Stack,
            };
            let mut args = inv.arguments;
            let spacing = args.remove("spacing");
            let style = ast::LayoutStyle {
                alignment: args.remove("alignment"),
                padding: args.remove("padding"),
                width: args.remove("width"),
                height: args.remove("height"),
                min_width: args.remove("minWidth"),
                max_width: args.remove("maxWidth"),
                min_height: args.remove("minHeight"),
                max_height: args.remove("maxHeight"),
                background: args.remove("background"),
                corner_radius: args.remove("cornerRadius"),
                border_color: args.remove("borderColor"),
                border_width: args.remove("borderWidth"),
                opacity: args.remove("opacity"),
                animation: args.remove("animation"),
            };
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            if stacked && spacing.is_some() {
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
            let lowered = lower_nodes(children, &child_cx)?;
            Ok(Node::Layout {
                kind,
                spacing,
                style,
                children: lowered,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Text" => {
            let mut positional = inv.positional;
            let value = take_positional(&mut positional, inv.span, "Text")?;
            let mut args = inv.arguments;
            let color = args.remove("color");
            let font_size = args.remove("fontSize");
            let font_weight = args.remove("fontWeight");
            let line_limit = args.remove("lineLimit");
            let line_height = args.remove("lineHeight");
            let letter_spacing = args.remove("letterSpacing");
            let selectable = args.remove("selectable");
            let value = lower_expr(&value, None, &cx.exprs(false))?;
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
        ast::Node::ComponentInvocation(inv) if inv.name == "Button" => {
            let span = inv.span;
            let mut positional = inv.positional;
            let label = take_positional(&mut positional, span, "Button")?;
            let options = inv.arguments;
            let icon = options.get("icon").cloned();
            let loading = options.get("loading").cloned();
            let disabled = options.get("disabled").cloned();
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            let label = lower_expr(&label, Some(&Type::String), &cx.exprs(false))?;
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
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?;
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
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
        ast::Node::ComponentInvocation(inv) if inv.name == "TextInput" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let value = take_required_arg(&mut args, &inv.name, "value", span)?;
            let placeholder = take_required_arg(&mut args, &inv.name, "placeholder", span)?;
            let keyboard = args.remove("keyboard");
            let secure = args.remove("secure");
            let multiline = args.remove("multiline");
            let autocorrect = args.remove("autocorrect");
            let capitalization = args.remove("capitalization");
            let focused = args.remove("focused");
            let max_length = args.remove("maxLength");
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
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
        ast::Node::ComponentInvocation(inv) if inv.name == "Switch" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let value = take_required_arg(&mut args, &inv.name, "value", span)?;
            let label = take_required_arg(&mut args, &inv.name, "label", span)?;
            let state = require_mutable_binding(&value, &Type::Bool, symbols, span, "Switch")?;
            let label = require_string_literal(&label, "Switch label")?;
            Ok(Node::Switch { state, label })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Image" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let source = match (args.remove("asset"), args.remove("url")) {
                (Some(asset), None) => ast::ImageSource::Asset(asset),
                (None, Some(url)) => ast::ImageSource::Url(url),
                _ => {
                    return Err(CompileError::new(
                        span,
                        "Image requires exactly one of `asset` or `url`",
                    ));
                }
            };
            let description = take_required_arg(&mut args, &inv.name, "description", span)?;
            let scale = args.remove("scale");
            let placeholder = args.remove("placeholder");
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
                    let lowered_url = lower_expr(&url, Some(&Type::String), &cx.exprs(false))?;
                    if let ast::Expr::String(value, _) = &url
                        && !is_https_url(value)
                    {
                        return Err(CompileError::new(
                            span,
                            "Image URL must be an absolute HTTPS URL",
                        ));
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
        ast::Node::ComponentInvocation(inv) if inv.name == "Pressable" => {
            let span = inv.span;
            let schema = invocation_schema(&inv)?;
            let mut args = inv.arguments;
            let disabled = args.remove("disabled");
            let haptic = args.remove("haptic");
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let mut modifiers = inv.modifiers;
            let actions = match take_modifier(&mut modifiers, "onPress") {
                Some(ast::ModifierBody::Actions(actions)) => actions,
                _ => {
                    return Err(CompileError::new(
                        span,
                        catalog::required_modifier_message(schema, "onPress"),
                    ));
                }
            };
            let long_press_actions = match take_modifier(&mut modifiers, "onLongPress") {
                Some(ast::ModifierBody::Actions(actions)) => actions,
                Some(_) => return Err(child_mismatch(span)),
                None => Vec::new(),
            };
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?
                .unwrap_or(Expr::Bool(false));
            let haptic = lower_haptic(haptic)?;
            let lowered_children = lower_nodes(children, &child_cx)?;
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
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationStack" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let root = take_required_arg(&mut args, &inv.name, "root", span)?;
            let (root, arguments) = ast::split_navigation_target(root);
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
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationBack" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let label = args.remove("label");
            if !allow_navigation_back {
                return Err(CompileError::new(
                    span,
                    "NavigationBack is only allowed inside a declared screen",
                ));
            }
            let label = label
                .map(|label| lower_expr(&label, Some(&Type::String), &cx.exprs(false)))
                .transpose()?
                .unwrap_or_else(|| Expr::String("Back".to_owned()));
            Ok(Node::NavigationBack { label })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationLink" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let destination = take_required_arg(&mut args, &inv.name, "destination", span)?;
            let guard = args.remove("when");
            let (destination, arguments) = ast::split_navigation_target(destination);
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
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
                .map(|guard| lower_expr(&guard, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?;
            let lowered_children = lower_nodes(children, &child_cx)?;
            Ok(Node::NavigationLink {
                destination: destination.0,
                arguments: destination.1,
                guard,
                children: lowered_children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Link" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let url = take_required_arg(&mut args, &inv.name, "url", span)?;
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let lowered_url = lower_expr(&url, Some(&Type::String), &cx.exprs(false))?;
            if let ast::Expr::String(value, _) = &url
                && !is_link_url(value)
            {
                return Err(CompileError::new(
                    span,
                    "Link URL must include a valid absolute scheme (for example `https://` or `mailto:`)",
                ));
            }
            let children = lower_nodes(children, &child_cx)?;
            Ok(Node::Link {
                url: lowered_url,
                children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Accessibility" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let label = take_required_arg(&mut args, &inv.name, "label", span)?;
            let hint = args.remove("hint");
            let role = args.remove("role");
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let lowered_label = lower_expr(&label, Some(&Type::String), &cx.exprs(false))?;
            if let ast::Expr::String(value, _) = &label
                && value.is_empty()
            {
                return Err(CompileError::new(
                    span,
                    "Accessibility label cannot be empty",
                ));
            }
            let lowered_hint = hint
                .map(|hint| {
                    let lowered = lower_expr(&hint, Some(&Type::String), &cx.exprs(false))?;
                    if let ast::Expr::String(value, _) = &hint
                        && value.is_empty()
                    {
                        return Err(CompileError::new(
                            hint.span(),
                            "Accessibility hint cannot be empty",
                        ));
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
            let children = lower_nodes(children, &child_cx)?;
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
        ast::Node::ComponentInvocation(inv) if inv.name == "KeyboardAware" => {
            let mut args = inv.arguments;
            let dismiss = args.remove("dismiss");
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(inv.span)),
            };
            let lowered_children = lower_nodes(children, &child_cx)?;
            Ok(Node::KeyboardAware {
                dismiss: lower_keyboard_dismiss(dismiss)?,
                children: lowered_children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "BottomSheet" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let is_presented = take_required_arg(&mut args, &inv.name, "isPresented", span)?;
            let partial = args.remove("partial");
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let state =
                require_mutable_binding(&is_presented, &Type::Bool, symbols, span, "BottomSheet")?;
            let lowered_children = lower_nodes(children, &child_cx)?;
            Ok(Node::BottomSheet {
                state,
                partial: optional_bool(partial, false, "BottomSheet partial")?,
                children: lowered_children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "RefreshControl" => {
            let span = inv.span;
            let schema = invocation_schema(&inv)?;
            let mut args = inv.arguments;
            let is_refreshing = take_required_arg(&mut args, &inv.name, "isRefreshing", span)?;
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let mut modifiers = inv.modifiers;
            let actions = match take_modifier(&mut modifiers, "onRefresh") {
                Some(ast::ModifierBody::Actions(actions)) => actions,
                _ => {
                    return Err(CompileError::new(
                        span,
                        catalog::required_modifier_message(schema, "onRefresh"),
                    ));
                }
            };
            let state = require_mutable_binding(
                &is_refreshing,
                &Type::Bool,
                symbols,
                span,
                "RefreshControl",
            )?;
            let mut lowered_children = lower_nodes(children, &child_cx)?;
            let actions =
                lower_actions_with_aliases(actions, symbols, functions, false, native_aliases)?;
            if lowered_children.len() == 1 {
                let mut child = lowered_children.pop().expect("one lowered refresh child");
                if let Node::FastList { plan } = &mut child {
                    *plan.refresh_slot() = Some(FastListRefresh { state, actions });
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
        ast::Node::ComponentInvocation(inv) if inv.name == "AppBottomBar" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
            let tabs = match inv.children {
                ast::ChildBody::Tabs(tabs) => tabs,
                _ => return Err(child_mismatch(span)),
            };
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
                let children = lower_nodes(tab.children, &child_cx)?;
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
        ast::Node::ComponentInvocation(inv) if inv.name == "FastList" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let rows = match inv.children {
                ast::ChildBody::Rows(rows) => rows,
                _ => return Err(child_mismatch(span)),
            };
            let ast::ListRows {
                source,
                key,
                item,
                index,
                section,
                children,
            } = rows;
            let axis = args.remove("axis");
            let item_extent = args.remove("rowHeight");
            let scroll_position = args.remove("scrollPosition");
            let mut modifiers = inv.modifiers;
            let on_end_reached = take_modifier_actions(&mut modifiers, span, "onEndReached")?;
            let on_scroll = take_modifier_actions(&mut modifiers, span, "onScroll")?;
            let sticky_header = take_modifier_nodes(&mut modifiers, span, "stickyHeader")?;
            let section_header = take_modifier_nodes(&mut modifiers, span, "sectionHeader")?;
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
            let (parts, section, item, item_type) = match source {
                ast::ListSource::Count(count) => {
                    if item.is_some() || has_section_binding {
                        return Err(CompileError::new(
                            span,
                            "FastList item and section bindings are only available with an array or `sections` source",
                        ));
                    }
                    let count_value = lower_expr(&count, Some(&row_index_type), &cx.exprs(false))?;
                    if let ast::Expr::Number(raw, count_span) = &count
                        && raw.parse::<i32>().is_ok_and(|value| value < 0)
                    {
                        return Err(CompileError::new(
                            *count_span,
                            "FastList count must be non-negative",
                        ));
                    }
                    (
                        ListSourceParts::Count { count: count_value },
                        None,
                        None,
                        None,
                    )
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
                        ListSourceParts::Items {
                            collection: Expr::State(name, ty.clone()),
                            element_type: (**element_type).clone(),
                            item: item_name.clone(),
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
                        ListSourceParts::Sections {
                            collection: Expr::State(name, ty.clone()),
                            element_type: (**element_type).clone(),
                            section: section_name.clone(),
                            item: item_name.clone(),
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
                    lower_expr(&key, Some(&key_type), &cx.exprs(false))
                })
                .transpose()?;
            let lowered_children = lower_nodes(children, &child_cx.with_symbols(&row_symbols))?;
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
                .map(|header| lower_nodes(header, &child_cx))
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
                    lower_nodes(header, &child_cx.with_symbols(&header_symbols))
                })
                .transpose()?;
            Ok(Node::FastList {
                plan: match parts {
                    ListSourceParts::Count { count } => ListPlan::Count {
                        count,
                        common: ListCommon {
                            axis,
                            item_extent,
                            index,
                            key,
                            scroll_position,
                            children: lowered_children,
                            on_end_reached,
                            on_scroll,
                            sticky_header,
                            refresh: None,
                        },
                    },
                    ListSourceParts::Items {
                        collection,
                        element_type,
                        item,
                    } => ListPlan::Items {
                        collection,
                        element_type,
                        item,
                        common: ListCommon {
                            axis,
                            item_extent,
                            index,
                            key,
                            scroll_position,
                            children: lowered_children,
                            on_end_reached,
                            on_scroll,
                            sticky_header,
                            refresh: None,
                        },
                    },
                    ListSourceParts::Sections {
                        collection,
                        element_type,
                        section,
                        item,
                    } => {
                        // Lowering rejects scroll observers and sticky headers
                        // for sectioned lists above, so only the sectioned
                        // options can be populated here.
                        debug_assert!(
                            scroll_position.is_none()
                                && on_end_reached.is_none()
                                && on_scroll.is_none()
                                && sticky_header.is_none()
                        );
                        ListPlan::Sections {
                            collection,
                            element_type,
                            section,
                            item,
                            common: SectionedListCommon {
                                item_extent,
                                index,
                                key,
                                children: lowered_children,
                                section_header,
                                refresh: None,
                            },
                        }
                    }
                },
            })
        }
        ast::Node::ComponentInvocation(inv) => Err(CompileError::new(
            inv.span,
            format!("unknown component `{}`", inv.name),
        )),
        ast::Node::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            let condition = lower_expr(&condition, Some(&Type::Bool), &cx.exprs(false))?;
            let lowered_then = lower_nodes(then_body, &child_cx)?;
            let lowered_else = else_body
                .map(|body| lower_nodes(body, &child_cx))
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
            let lowered_value = lower_expr(&value, Some(&value_type), &cx.exprs(false))?;
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
                let lowered_case = lower_expr(&case.value, Some(&value_type), &cx.exprs(false))?;
                let key = format!("{lowered_case:?}");
                if !seen.insert(key) {
                    return Err(CompileError::new(
                        case.span,
                        "when case values must be unique",
                    ));
                }
                let body = lower_nodes(case.body, &child_cx)?;
                lowered_cases.push(WhenCase {
                    value: lowered_case,
                    body,
                });
            }
            let lowered_else = lower_nodes(else_body, &child_cx)?;
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
                    lower_expr(&argument, Some(ty), &cx.exprs(false))?,
                ));
            }
            let lowered_children = children
                .map(|children| {
                    let lowered = lower_nodes(children, &child_cx)?;
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
                (true, Some(children)) => Some(lower_nodes(children, cx)?),
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
                            lower_expr(default, Some(ty), &cx.exprs(false))?,
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
                    lower_expr(&argument, Some(ty), &cx.exprs(false))?,
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
            lower_expr(
                argument,
                Some(&parameter.ty),
                &ExprContext::new(symbols, functions, false),
            )
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
                let expression = lower_expr(
                    &expression,
                    None,
                    &ExprContext::new(symbols, functions, allow_await),
                )?;
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
                let value = lower_expr(
                    &value,
                    Some(ty),
                    &ExprContext::new(symbols, functions, allow_await),
                )?;
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                    let expression = lower_expr(
                        &expression,
                        None,
                        &ExprContext::new(symbols, functions, allow_await),
                    )?;
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
                        lower_expr(
                            argument,
                            Some(expected),
                            &ExprContext::new(symbols, functions, allow_await),
                        )
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                    let start = lower_expr(
                        start,
                        Some(&int32),
                        &ExprContext::new(symbols, functions, allow_await),
                    )?;
                    let end = lower_expr(
                        end,
                        Some(&int32),
                        &ExprContext::new(symbols, functions, allow_await),
                    )?;
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
                        &ExprContext::new(symbols, functions, allow_await),
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                    &ExprContext::new(symbols, functions, allow_await),
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
                                "unknown plugin error variant `{}.{}.{}`",
                                arm.namespace, arm.error_name, arm.variant
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
        if !expression.is_throwing_call() {
            return;
        }
        let (receiver, namespace, name) = match expression {
            Expr::NativeCall {
                receiver,
                namespace,
                name,
                ..
            } => (receiver, namespace, name),
            // Validated core throwing calls (`Network`, `File`) declare no
            // error type, so they always require a catch-all `else` — exactly
            // as when they were unresolvable `NativeCall`s.
            _ => {
                has_untyped_throw = true;
                return;
            }
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

    if let Some(name) = referenced_disposed.into_iter().next() {
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
        &ExprContext::new(symbols, functions, allow_await),
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
#[path = "components_tests.rs"]
mod tests;
