use std::collections::{BTreeMap, HashMap, HashSet};

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::walk::any_node;
use nexa_ir::{
    AccessibilityRole, Action, AutofillType, BottomBarTab, ButtonShape, ButtonSize, ButtonStyle,
    Capitalization, CollectionMutation, ColorExpression, ColorValue, DirectionConfig,
    DirectionStyle, Expr, FastListMove, FastListRefresh, FontWeight, GradientDirection,
    HapticStyle, ImageScale, ImageSource, KeyboardDismissMode, KeyboardType, LayoutKind, ListAxis,
    ListCommon, ListPlan, NativeComponentEventHandler, Node, NumericType, ReturnKeyType, ScreenId,
    SectionedListCommon, StatusBarConfig, StatusBarStyle, SystemIcon,
    TaskExecutor as IrTaskExecutor, TextFontStyle, TextInputFont, TextStyle, ToolbarPlacement,
    Type, ViewTransition as IrViewTransition, WhenCase,
};
use nexa_syntax::{ast, catalog};

use super::{
    context::{ScreenSignatures, SemanticContext, TypeRegistries},
    expressions::{
        FunctionSignatures, functions_with_error_handling, infer_expr_type, lower_expr,
        lower_for_iterable, lower_map_iterable, plugin_error_variant, resolve_value_type,
        type_name,
    },
    styles::{
        lower_opacity, lower_style, lower_view_effects, number_value, optional_color,
        optional_dimension, parse_animation, parse_color, parse_color_literal,
    },
};
use crate::Target;

type AccessibilityOptions = (Expr, Option<Expr>, Option<Expr>, AccessibilityRole);

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

fn take_translator_comment(
    args: &mut BTreeMap<String, ast::Expr>,
    component: &str,
) -> Result<Option<String>, CompileError> {
    args.remove("comment")
        .map(|value| require_string_literal(&value, &format!("{component} comment")))
        .transpose()
}

fn localize_ui_text(value: Expr, comment: Option<String>) -> Expr {
    match value {
        Expr::String(_) | Expr::Interpolation(_) => {
            let key = nexa_ir::localization::source_key(&value);
            match key {
                Some(key) => Expr::LocalizedText {
                    key,
                    value: Box::new(value),
                    comment,
                },
                None => value,
            }
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            value_type,
        } => Expr::Conditional {
            condition,
            then_value: Box::new(localize_ui_text(*then_value, comment.clone())),
            else_value: Box::new(localize_ui_text(*else_value, comment)),
            value_type,
        },
        Expr::Concat(left, right) => Expr::Concat(
            Box::new(localize_ui_text(*left, comment.clone())),
            Box::new(localize_ui_text(*right, comment)),
        ),
        Expr::Coalesce(left, right) => {
            Expr::Coalesce(left, Box::new(localize_ui_text(*right, comment)))
        }
        Expr::Array(values) => Expr::Array(
            values
                .into_iter()
                .map(|value| localize_ui_text(value, comment.clone()))
                .collect(),
        ),
        value => value,
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
    take_modifier_decl(modifiers, name).map(|modifier| modifier.body)
}

/// Remove one complete trailing dot-modifier declaration by name.
fn take_modifier_decl(
    modifiers: &mut Vec<ast::DotModifier>,
    name: &str,
) -> Option<ast::DotModifier> {
    modifiers
        .iter()
        .position(|modifier| modifier.name == name)
        .map(|index| modifiers.remove(index))
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

fn lower_color_expression(
    value: ast::Expr,
    role: &str,
    cx: &SemanticContext<'_>,
) -> Result<ColorExpression, CompileError> {
    if matches!(
        &value,
        ast::Expr::String(_, _) | ast::Expr::ThemeToken(_, _)
    ) {
        return parse_color(value, cx.themes, role).map(ColorExpression::Static);
    }
    lower_expr(&value, Some(&Type::String), &cx.exprs(false)).map(ColorExpression::Dynamic)
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

pub(super) fn lower_node(mut node: ast::Node, cx: &SemanticContext) -> Result<Node, CompileError> {
    let accessibility = match &mut node {
        ast::Node::ComponentInvocation(invocation) => {
            take_accessibility_options(&mut invocation.arguments, invocation.span, cx)?
        }
        ast::Node::ComponentCall {
            arguments, span, ..
        }
        | ast::Node::NativeComponentCall {
            arguments, span, ..
        } => take_accessibility_options(arguments, *span, cx)?,
        _ => None,
    };
    let lowered = lower_node_inner(node, cx)?;
    match accessibility {
        Some((label, hint, value, role)) => Ok(Node::Accessibility {
            label,
            hint,
            value,
            role,
            children: vec![lowered],
        }),
        None => Ok(lowered),
    }
}

fn take_accessibility_options(
    arguments: &mut BTreeMap<String, ast::Expr>,
    span: Span,
    cx: &SemanticContext,
) -> Result<Option<AccessibilityOptions>, CompileError> {
    let label = arguments.remove("accessibilityLabel");
    let hint = arguments.remove("accessibilityHint");
    let value = arguments.remove("accessibilityValue");
    let role = arguments.remove("accessibilityRole");
    if label.is_none() && hint.is_none() && value.is_none() && role.is_none() {
        return Ok(None);
    }
    let Some(label) = label else {
        let option_span = hint
            .as_ref()
            .or(value.as_ref())
            .or(role.as_ref())
            .map(ast::Expr::span)
            .unwrap_or(span);
        return Err(CompileError::new(
            option_span,
            "accessibilityLabel is required when accessibility options are supplied",
        ));
    };
    let lowered_label = localize_ui_text(
        lower_expr(&label, Some(&Type::String), &cx.exprs(false))?,
        None,
    );
    if matches!(&label, ast::Expr::String(value, _) if value.is_empty()) {
        return Err(CompileError::new(
            label.span(),
            "Accessibility label cannot be empty",
        ));
    }
    let lowered_hint = hint
        .map(|hint| {
            let lowered = localize_ui_text(
                lower_expr(&hint, Some(&Type::String), &cx.exprs(false))?,
                None,
            );
            if matches!(&hint, ast::Expr::String(value, _) if value.is_empty()) {
                return Err(CompileError::new(
                    hint.span(),
                    "Accessibility hint cannot be empty",
                ));
            }
            Ok(lowered)
        })
        .transpose()?;
    let lowered_value = value
        .map(|value| {
            lower_expr(&value, Some(&Type::String), &cx.exprs(false))
                .map(|value| localize_ui_text(value, None))
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
    Ok(Some((lowered_label, lowered_hint, lowered_value, role)))
}

fn lower_node_inner(node: ast::Node, cx: &SemanticContext) -> Result<Node, CompileError> {
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
        ast::Node::ComponentInvocation(inv) if inv.name == "Toolbar" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let placement = match args.remove("placement") {
                Some(ast::Expr::Name(name, _)) if name == "Leading" => ToolbarPlacement::Leading,
                Some(ast::Expr::Name(name, _)) if name == "Trailing" => ToolbarPlacement::Trailing,
                Some(value) => {
                    return Err(CompileError::new(
                        value.span(),
                        "Toolbar placement must be `Leading` or `Trailing`",
                    ));
                }
                None => ToolbarPlacement::Trailing,
            };
            let children = match inv.children {
                ast::ChildBody::Nodes(nodes) => lower_nodes(nodes, &child_cx)?,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::Toolbar {
                placement,
                children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "StatusBar" => {
            let mut args = inv.arguments;
            let style = args.remove("style");
            let hidden = args.remove("hidden");
            let background = args.remove("background");
            Ok(Node::StatusBar {
                config: lower_status_bar(style, hidden, background)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Appearance" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let mode = take_required_arg(&mut args, &inv.name, "mode", span)?;
            let mode = lower_expr(&mode, Some(&Type::String), &cx.exprs(false))?;
            if let Expr::String(value) = &mode
                && !matches!(value.as_str(), "system" | "light" | "dark")
            {
                return Err(CompileError::new(
                    span,
                    "Appearance mode must be `system`, `light`, or `dark`",
                ));
            }
            let children = match inv.children {
                ast::ChildBody::Nodes(nodes) => lower_nodes(nodes, &child_cx)?,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::Appearance { mode, children })
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
                    cx.type_registries(),
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
                    cx.type_registries(),
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
                    cx.type_registries(),
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
                    cx.type_registries(),
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
                    cx.type_registries(),
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
                scale: args.remove("scale"),
                rotation: args.remove("rotation"),
                shadow: args.remove("shadow"),
                blur: args.remove("blur"),
                clip: args.remove("clip"),
                z_index: args.remove("zIndex"),
                glass: args.remove("glass"),
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
        ast::Node::ComponentInvocation(inv) if inv.name == "Form" => {
            let span = inv.span;
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::Form {
                children: lower_nodes(children, &child_cx)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Section" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "Section")?;
            let title = args
                .remove("title")
                .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
                .transpose()?
                .map(|value| localize_ui_text(value, comment.clone()));
            let footer = args
                .remove("footer")
                .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
                .transpose()?
                .map(|value| localize_ui_text(value, comment.clone()));
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            Ok(Node::FormSection {
                title,
                footer,
                children: lower_nodes(children, &child_cx)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "ContentUnavailable" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "ContentUnavailable")?;
            let title = lower_expr(
                &take_required_arg(&mut args, &inv.name, "title", span)?,
                Some(&Type::String),
                &cx.exprs(false),
            )?;
            let icon_name = require_string_literal(
                &take_required_arg(&mut args, &inv.name, "icon", span)?,
                "ContentUnavailable icon",
            )?;
            let icon = SystemIcon::shared(&icon_name).ok_or_else(|| {
                CompileError::new(
                    span,
                    format!(
                        "unknown shared system icon `{icon_name}`; ContentUnavailable uses `icon: \"shared_name\"`"
                    ),
                )
            })?;
            let description = lower_expr(
                &take_required_arg(&mut args, &inv.name, "description", span)?,
                Some(&Type::String),
                &cx.exprs(false),
            )?;
            Ok(Node::ContentUnavailable {
                title: localize_ui_text(title, comment.clone()),
                icon,
                description: localize_ui_text(description, comment),
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Text" => {
            let mut positional = inv.positional;
            let value = take_positional(&mut positional, inv.span, "Text")?;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "Text")?;
            let color = args.remove("color");
            let alignment = lower_text_alignment(args.remove("alignment"))?;
            let font_size = args.remove("fontSize");
            let font_style = lower_text_font_style(args.remove("fontStyle"))?;
            let font_weight = args.remove("fontWeight");
            let padding = args.remove("padding");
            let line_limit = args.remove("lineLimit");
            let line_height = args.remove("lineHeight");
            let letter_spacing = args.remove("letterSpacing");
            let strikethrough = args.remove("strikethrough");
            let selectable = args.remove("selectable");
            let opacity = args.remove("opacity");
            let value =
                localize_ui_text(lower_expr(&value, None, &cx.exprs(false))?, comment.clone());
            let color = optional_color(color, "text color", themes)?;
            let font_size = optional_dimension(
                font_size,
                "fontSize",
                Some(ast::ThemeTokenKind::FontSize),
                themes,
            )?;
            let font_weight = lower_font_weight(font_weight)?;
            let padding = optional_dimension(
                padding,
                "padding",
                Some(ast::ThemeTokenKind::Spacing),
                themes,
            )?;
            let line_limit = lower_line_limit(line_limit)?;
            let line_height = optional_dimension(line_height, "lineHeight", None, themes)?;
            let letter_spacing = optional_dimension(letter_spacing, "letterSpacing", None, themes)?;
            let strikethrough = optional_bool(strikethrough, false, "strikethrough")?;
            let selectable = optional_bool(selectable, false, "selectable")?;
            let opacity = lower_opacity(opacity)?;
            let effects = lower_view_effects(
                args.remove("scale"),
                args.remove("rotation"),
                args.remove("shadow"),
                args.remove("blur"),
                args.remove("clip"),
                args.remove("zIndex"),
                args.remove("glass"),
                themes,
            )?;
            Ok(Node::Text {
                value,
                style: TextStyle {
                    alignment,
                    color,
                    font_size,
                    font_style,
                    font_weight,
                    padding,
                    opacity,
                    effects,
                    line_limit,
                    line_height,
                    letter_spacing,
                    strikethrough,
                    selectable,
                },
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Spacer" => Ok(Node::Spacer),
        ast::Node::ComponentInvocation(inv) if inv.name == "Divider" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let color = optional_color(args.remove("color"), "divider color", themes)?
                .ok_or_else(|| CompileError::new(span, "Divider requires a `color` value"))?;
            let thickness = optional_dimension(
                args.remove("thickness"),
                "divider thickness",
                Some(ast::ThemeTokenKind::Spacing),
                themes,
            )?
            .ok_or_else(|| CompileError::new(span, "Divider requires a `thickness` value"))?;
            Ok(Node::Divider { color, thickness })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Slider" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let value = take_required_arg(&mut args, &inv.name, "value", span)?;
            let state = require_mutable_binding(
                &value,
                &Type::Numeric(NumericType::Float64),
                symbols,
                span,
                "Slider",
            )?;
            let min = require_f64_literal(
                &take_required_arg(&mut args, &inv.name, "min", span)?,
                "Slider min",
            )?;
            let max = require_f64_literal(
                &take_required_arg(&mut args, &inv.name, "max", span)?,
                "Slider max",
            )?;
            let step = require_f64_literal(
                &take_required_arg(&mut args, &inv.name, "step", span)?,
                "Slider step",
            )?;
            if min >= max {
                return Err(CompileError::new(span, "Slider min must be less than max"));
            }
            if step <= 0.0 {
                return Err(CompileError::new(
                    span,
                    "Slider step must be greater than zero",
                ));
            }
            let intervals = (max - min) / step;
            let rounded_intervals = intervals.round();
            let tolerance = intervals.abs().max(1.0) * 1e-9;
            if !intervals.is_finite()
                || rounded_intervals < 1.0
                || rounded_intervals > i32::MAX as f64
                || (intervals - rounded_intervals).abs() > tolerance
            {
                return Err(CompileError::new(
                    span,
                    "Slider range must contain a whole number of steps",
                ));
            }
            if min.abs() > f32::MAX as f64 || max.abs() > f32::MAX as f64 {
                return Err(CompileError::new(
                    span,
                    "Slider min and max must fit the native Android Float range",
                ));
            }
            Ok(Node::Slider {
                state,
                animated: false,
                min,
                max,
                step,
            })
        }
        ast::Node::ComponentInvocation(inv)
            if inv.name == "ProgressBar" || inv.name == "ProgressRing" =>
        {
            let span = inv.span;
            let mut args = inv.arguments;
            let progress = take_required_arg(&mut args, &inv.name, "progress", span)?;
            let progress = lower_expr(
                &progress,
                Some(&Type::Numeric(NumericType::Float64)),
                &cx.exprs(false),
            )?;
            if inv.name == "ProgressBar" {
                Ok(Node::ProgressBar { progress })
            } else {
                Ok(Node::ProgressRing { progress })
            }
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "SegmentedControl" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "SegmentedControl")?;
            let items = take_required_arg(&mut args, &inv.name, "items", span)?;
            let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
            let state = require_mutable_binding(
                &selected,
                &Type::String,
                symbols,
                span,
                "SegmentedControl",
            )?;
            let items = localize_ui_text(
                lower_expr(
                    &items,
                    Some(&Type::Array(Box::new(Type::String))),
                    &cx.exprs(false),
                )?,
                comment.clone(),
            );
            Ok(Node::SegmentedControl { items, state })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Picker" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "Picker")?;
            let items = take_required_arg(&mut args, &inv.name, "items", span)?;
            let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
            let icon = args
                .remove("icon")
                .map(|value| {
                    let name = require_string_literal(&value, "Picker icon")?;
                    SystemIcon::shared(&name).ok_or_else(|| {
                        CompileError::new(
                            value.span(),
                            format!("unknown shared system icon `{name}` in Picker `icon`"),
                        )
                    })
                })
                .transpose()?;
            let state = require_mutable_binding(&selected, &Type::String, symbols, span, "Picker")?;
            let items = localize_ui_text(
                lower_expr(
                    &items,
                    Some(&Type::Array(Box::new(Type::String))),
                    &cx.exprs(false),
                )?,
                comment.clone(),
            );
            let label = args
                .remove("label")
                .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
                .transpose()?
                .map(|value| localize_ui_text(value, comment.clone()));
            Ok(Node::Picker {
                items,
                state,
                icon,
                label,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "DatePicker" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let timestamp = take_required_arg(&mut args, &inv.name, "timestamp", span)?;
            let has_time = take_required_arg(&mut args, &inv.name, "hasTime", span)?;
            let timestamp_state = require_mutable_binding(
                &timestamp,
                &Type::Numeric(NumericType::Int64),
                symbols,
                span,
                "DatePicker timestamp",
            )?;
            let has_time_state = require_mutable_binding(
                &has_time,
                &Type::Bool,
                symbols,
                span,
                "DatePicker hasTime",
            )?;
            Ok(Node::DatePicker {
                timestamp_state,
                has_time_state,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Button" => {
            let span = inv.span;
            let mut positional = inv.positional;
            let label = take_positional(&mut positional, span, "Button")?;
            let mut options = inv.arguments;
            let comment = take_translator_comment(&mut options, "Button")?;
            let icon = options.get("icon").cloned();
            let loading = options.get("loading").cloned();
            let disabled = options.get("disabled").cloned();
            let actions = match inv.children {
                ast::ChildBody::Actions(actions) => actions,
                _ => return Err(child_mismatch(span)),
            };
            let label = localize_ui_text(
                lower_expr(&label, Some(&Type::String), &cx.exprs(false))?,
                comment.clone(),
            );
            if !matches!(
                label,
                Expr::String(_)
                    | Expr::Interpolation(_)
                    | Expr::LocalizedText { .. }
                    | Expr::State(_, Type::String)
                    | Expr::NativeCall {
                        return_type: Type::String,
                        ..
                    }
                    | Expr::Call {
                        return_type: Type::String,
                        ..
                    }
                    | Expr::Conditional {
                        value_type: Type::String,
                        ..
                    }
            ) {
                return Err(CompileError::new(span, "Button label must be a String"));
            }
            let icon = icon
                .map(|value| {
                    let name = require_string_literal(&value, "Button icon")?;
                    SystemIcon::shared(&name).ok_or_else(|| {
                        CompileError::new(
                            value.span(),
                            format!("unknown shared system icon `{name}` in Button `icon`; use the `Icon` component for platform-specific symbols"),
                        )
                    })
                })
                .transpose()?;
            let loading = loading
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?;
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?;
            let style = options
                .get("style")
                .cloned()
                .map(|v| match v {
                    ast::Expr::String(s, span) => match s.as_str() {
                        "prominent" | "borderedProminent" => Ok(ButtonStyle::BorderedProminent),
                        "bordered" => Ok(ButtonStyle::Bordered),
                        "borderless" => Ok(ButtonStyle::Borderless),
                        "plain" => Ok(ButtonStyle::Plain),
                        other => Err(CompileError::new(
                            span,
                            format!("unknown button style `{other}`"),
                        )),
                    },
                    ast::Expr::Name(s, span) => match s.as_str() {
                        "BorderedProminent" | "Prominent" => Ok(ButtonStyle::BorderedProminent),
                        "Bordered" => Ok(ButtonStyle::Bordered),
                        "Borderless" => Ok(ButtonStyle::Borderless),
                        "Plain" => Ok(ButtonStyle::Plain),
                        other => Err(CompileError::new(
                            span,
                            format!("unknown button style `{other}`"),
                        )),
                    },
                    _ => Err(CompileError::new(
                        v.span(),
                        "button style must be a String or enum",
                    )),
                })
                .transpose()?;
            let size = options
                .get("size")
                .cloned()
                .map(|v| match v {
                    ast::Expr::String(s, span) => match s.as_str() {
                        "small" => Ok(ButtonSize::Small),
                        "regular" => Ok(ButtonSize::Regular),
                        "large" => Ok(ButtonSize::Large),
                        other => Err(CompileError::new(
                            span,
                            format!("unknown button size `{other}`"),
                        )),
                    },
                    ast::Expr::Name(s, span) => match s.as_str() {
                        "Small" => Ok(ButtonSize::Small),
                        "Regular" => Ok(ButtonSize::Regular),
                        "Large" => Ok(ButtonSize::Large),
                        other => Err(CompileError::new(
                            span,
                            format!("unknown button size `{other}`"),
                        )),
                    },
                    _ => Err(CompileError::new(
                        v.span(),
                        "button size must be a String or enum",
                    )),
                })
                .transpose()?;
            let shape = options
                .get("shape")
                .cloned()
                .map(|v| match v {
                    ast::Expr::String(s, span) => match s.to_ascii_lowercase().as_str() {
                        "capsule" => Ok(ButtonShape::Capsule),
                        "circle" => Ok(ButtonShape::Circle),
                        other => {
                            if let Some(radius) = other
                                .strip_prefix("rounded(")
                                .and_then(|r| r.strip_suffix(")"))
                                .and_then(|n| n.parse::<f32>().ok())
                            {
                                return Ok(ButtonShape::Rounded(radius));
                            }
                            Err(CompileError::new(
                                span,
                                format!("unknown button shape `{other}`"),
                            ))
                        }
                    },
                    ast::Expr::Name(s, span) => match s.as_str() {
                        "Capsule" => Ok(ButtonShape::Capsule),
                        "Circle" => Ok(ButtonShape::Circle),
                        other => Err(CompileError::new(
                            span,
                            format!("unknown button shape `{other}`"),
                        )),
                    },
                    ast::Expr::Call(name, _, args, _span)
                        if name == "Rounded" && args.len() == 1 =>
                    {
                        let r = number_value(&args[0], "shape radius")?;
                        Ok(ButtonShape::Rounded(r))
                    }
                    _ => Err(CompileError::new(
                        v.span(),
                        "button shape must be capsule, circle, or rounded",
                    )),
                })
                .transpose()?;
            let tint = options
                .get("tint")
                .cloned()
                .map(|value| lower_color_expression(value, "button tint", cx))
                .transpose()?;
            let glass = options
                .get("glass")
                .map(|v| match v {
                    ast::Expr::Bool(b, _) => *b,
                    _ => true,
                })
                .unwrap_or(false);
            let lowered = lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            Ok(Node::Button {
                label,
                icon,
                loading,
                disabled,
                style,
                size,
                shape,
                tint,
                glass,
                actions: lowered,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "TextInput" => {
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
            let max_length = args.remove("maxLength");
            let font = lower_text_input_font(args.remove("font"))?;
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
                .map(|value| {
                    require_mutable_binding(&value, &Type::Bool, symbols, span, "TextInput focus")
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
                max_length,
                font,
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
                on_change,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Switch" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "Switch")?;
            let value = take_required_arg(&mut args, &inv.name, "value", span)?;
            let label = take_required_arg(&mut args, &inv.name, "label", span)?;
            let state = require_mutable_binding(&value, &Type::Bool, symbols, span, "Switch")?;
            let label = localize_ui_text(
                lower_expr(&label, Some(&Type::String), &cx.exprs(false))?,
                comment.clone(),
            );
            Ok(Node::Switch { state, label })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Image" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let mut modifiers = inv.modifiers;
            let asset = args.remove("asset");
            let url = args.remove("url");
            let file = args.remove("file");
            let source = match (asset, url, file) {
                (Some(asset), None, None) => ast::ImageSource::Asset(asset),
                (None, Some(url), None) => ast::ImageSource::Url(url),
                (None, None, Some(file)) => ast::ImageSource::File(file),
                _ => {
                    return Err(CompileError::new(
                        span,
                        "Image requires exactly one of `asset`, `url`, or `file`",
                    ));
                }
            };
            let description = take_required_arg(&mut args, &inv.name, "description", span)?;
            let scale = args.remove("scale");
            let placeholder = args.remove("placeholder");
            let max_height = args
                .remove("maxHeight")
                .map(|value| {
                    let height = number_value(&value, "Image maxHeight")?;
                    if !height.is_finite() || height <= 0.0 || height > 4096.0 {
                        return Err(CompileError::new(
                            value.span(),
                            "Image maxHeight must be greater than 0 and no greater than 4096",
                        ));
                    }
                    Ok(height)
                })
                .transpose()?;
            let shared_element = take_modifier_decl(&mut modifiers, "sharedElement")
                .map(|modifier| {
                    let id = modifier.arguments.get("id").ok_or_else(|| {
                        CompileError::new(modifier.span, "`.sharedElement` requires `id`")
                    })?;
                    if !matches!(modifier.body, ast::ModifierBody::None) {
                        return Err(CompileError::new(
                            modifier.span,
                            "`.sharedElement(id: ...)` does not accept a trailing block",
                        ));
                    }
                    lower_expr(id, Some(&Type::String), &cx.exprs(false))
                })
                .transpose()?;
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
                ast::ImageSource::File(file) => ImageSource::LocalFile(lower_expr(
                    &file,
                    Some(&Type::String),
                    &cx.exprs(false),
                )?),
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
                max_height,
                shared_element,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Icon" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let selectors = [("system", 0), ("sfsymbol", 1), ("materialsymbol", 2)];
            let mut selection = None;
            for (name, kind) in selectors {
                if let Some(expression) = args.remove(name) {
                    if selection.is_some() {
                        return Err(CompileError::new(
                            expression.span(),
                            "Icon accepts exactly one of `system`, `sfsymbol`, or `materialsymbol`",
                        ));
                    }
                    selection = Some((
                        kind,
                        require_string_literal(&expression, "Icon symbol name")?,
                    ));
                }
            }
            let icon = match selection {
                Some((0, name)) => SystemIcon::shared(&name).ok_or_else(|| {
                    CompileError::new(
                        span,
                        format!("unknown shared system icon `{name}`; use `sfsymbol` or `materialsymbol` for a platform-specific name"),
                    )
                })?,
                Some((1, name)) => SystemIcon::sf_symbol(name).ok_or_else(|| {
                    CompileError::new(span, "Icon `sfsymbol` must be a non-empty symbol name")
                })?,
                Some((2, name)) => SystemIcon::material_symbol(name).ok_or_else(|| {
                    CompileError::new(span, "Icon `materialsymbol` must be a non-empty Compose icon name")
                })?,
                _ => {
                    return Err(CompileError::new(
                        span,
                        "Icon requires exactly one of `system`, `sfsymbol`, or `materialsymbol`",
                    ));
                }
            };
            let description_expr = take_required_arg(&mut args, &inv.name, "description", span)?;
            let description = require_string_literal(&description_expr, "Icon description")?;
            let size = take_required_arg(&mut args, &inv.name, "size", span)?;
            let size = match size {
                ast::Expr::Number(value, number_span) => value
                    .parse::<f32>()
                    .ok()
                    .filter(|value| value.is_finite() && *value > 0.0 && *value <= 512.0)
                    .ok_or_else(|| {
                        CompileError::new(
                            number_span,
                            "Icon size must be a positive numeric literal no greater than 512",
                        )
                    })?,
                expr => {
                    return Err(CompileError::new(
                        expr.span(),
                        "Icon size must be a positive numeric literal",
                    ));
                }
            };
            let tint = lower_color_expression(
                take_required_arg(&mut args, &inv.name, "tint", span)?,
                "Icon tint",
                cx,
            )?;
            Ok(Node::SystemIcon {
                icon,
                description,
                size,
                tint,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "LinearGradient" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let start_color = parse_color_literal(
                take_required_arg(&mut args, &inv.name, "startColor", span)?,
                "LinearGradient startColor",
            )?;
            let end_color = parse_color_literal(
                take_required_arg(&mut args, &inv.name, "endColor", span)?,
                "LinearGradient endColor",
            )?;
            let direction = match args.remove("direction") {
                None => GradientDirection::TopToBottom,
                Some(ast::Expr::Name(name, _)) if name == "TopToBottom" => {
                    GradientDirection::TopToBottom
                }
                Some(ast::Expr::Name(name, _)) if name == "BottomToTop" => {
                    GradientDirection::BottomToTop
                }
                Some(ast::Expr::Name(name, _)) if name == "LeadingToTrailing" => {
                    GradientDirection::LeadingToTrailing
                }
                Some(ast::Expr::Name(name, _)) if name == "TrailingToLeading" => {
                    GradientDirection::TrailingToLeading
                }
                Some(expr) => {
                    return Err(CompileError::new(
                        expr.span(),
                        "LinearGradient direction must be TopToBottom, BottomToTop, LeadingToTrailing, or TrailingToLeading",
                    ));
                }
            };
            let height = match args.remove("height") {
                None => 180.0,
                Some(expr) => {
                    let value = number_value(&expr, "LinearGradient height")?;
                    if !value.is_finite() || value <= 0.0 || value > 4096.0 {
                        return Err(CompileError::new(
                            expr.span(),
                            "LinearGradient height must be greater than 0 and no greater than 4096",
                        ));
                    }
                    value
                }
            };
            Ok(Node::LinearGradient {
                start_color: ColorValue::Static(start_color),
                end_color: ColorValue::Static(end_color),
                direction,
                height,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Pressable" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let disabled = args.remove("disabled");
            let haptic = args.remove("haptic");
            let fill_max_size = match args.remove("fillMaxSize") {
                Some(ast::Expr::Bool(value, _)) => value,
                Some(value) => {
                    return Err(CompileError::new(
                        value.span(),
                        "Pressable `fillMaxSize` must be a Boolean literal",
                    ));
                }
                None => false,
            };
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let mut modifiers = inv.modifiers;
            let actions =
                match take_modifier_decl(&mut modifiers, "onTap").map(|modifier| modifier.body) {
                    Some(ast::ModifierBody::Actions(actions)) => actions,
                    _ => {
                        return Err(CompileError::new(
                            span,
                            "Pressable requires a tap handler using `.onTap { ... }`",
                        ));
                    }
                };
            let (long_press_duration_ms, long_press_actions) =
                match take_modifier_decl(&mut modifiers, "onLongPress") {
                    Some(modifier) => {
                        let duration = modifier.arguments.get("durationMs");
                        let duration = duration
                            .map(|value| {
                                lower_expr(
                                    value,
                                    Some(&Type::Numeric(NumericType::Int32)),
                                    &cx.exprs(false),
                                )
                            })
                            .transpose()?
                            .unwrap_or(Expr::Number {
                                raw: "500".to_owned(),
                                ty: NumericType::Int32,
                            });
                        let actions = match modifier.body {
                            ast::ModifierBody::Actions(actions) => actions,
                            _ => return Err(child_mismatch(span)),
                        };
                        (duration, actions)
                    }
                    None => (
                        Expr::Number {
                            raw: "500".to_owned(),
                            ty: NumericType::Int32,
                        },
                        Vec::new(),
                    ),
                };
            let context_menu = take_modifier_nodes(&mut modifiers, span, "contextMenu")?;
            let double_tap_actions = match take_modifier(&mut modifiers, "onDoubleTap") {
                Some(ast::ModifierBody::Actions(actions)) => actions,
                Some(_) => return Err(child_mismatch(span)),
                None => Vec::new(),
            };
            let (drag_parameters, drag_actions) = match take_modifier(&mut modifiers, "onDrag") {
                Some(ast::ModifierBody::EventActions {
                    parameters,
                    actions,
                }) => (parameters, actions),
                Some(_) => return Err(child_mismatch(span)),
                None => (Vec::new(), Vec::new()),
            };
            let (pinch_parameter, pinch_actions) = match take_modifier(&mut modifiers, "onPinch") {
                Some(ast::ModifierBody::EventActions {
                    parameters,
                    actions,
                }) => {
                    if parameters.len() != 1 {
                        return Err(CompileError::new(
                            span,
                            ".onPinch requires one binding for its scaleFactor delta",
                        ));
                    }
                    let Some(parameter) = parameters.into_iter().next() else {
                        return Err(CompileError::new(
                            span,
                            ".onPinch requires one binding for its scaleFactor delta",
                        ));
                    };
                    if symbols.contains_key(&parameter) {
                        return Err(CompileError::new(
                            span,
                            format!(".onPinch binding `{parameter}` shadows an existing value"),
                        ));
                    }
                    (Some(parameter), actions)
                }
                Some(_) => return Err(child_mismatch(span)),
                None => (None, Vec::new()),
            };
            let mut drag_symbols = symbols.clone();
            if !drag_parameters.is_empty() {
                if drag_parameters.len() != 4 {
                    return Err(CompileError::new(
                        span,
                        ".onDrag requires four bindings: translationX, translationY, velocityX, and velocityY",
                    ));
                }
                let mut unique_parameters = HashSet::with_capacity(drag_parameters.len());
                for parameter in &drag_parameters {
                    if !unique_parameters.insert(parameter.as_str()) {
                        return Err(CompileError::new(
                            span,
                            format!(".onDrag binds `{parameter}` more than once"),
                        ));
                    }
                    if symbols.contains_key(parameter) {
                        return Err(CompileError::new(
                            span,
                            format!(".onDrag binding `{parameter}` shadows an existing value"),
                        ));
                    }
                    drag_symbols.insert(
                        parameter.clone(),
                        (Type::Numeric(NumericType::Float64), false),
                    );
                }
            }
            let mut pinch_symbols = symbols.clone();
            if let Some(parameter) = &pinch_parameter {
                pinch_symbols.insert(
                    parameter.clone(),
                    (Type::Numeric(NumericType::Float64), false),
                );
            }
            let disabled = disabled
                .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
                .transpose()?
                .unwrap_or(Expr::Bool(false));
            let haptic = lower_haptic(haptic)?;
            let lowered_children = lower_nodes(children, &child_cx)?;
            let context_menu = context_menu
                .map(|nodes| {
                    if nodes.is_empty()
                        || !nodes
                            .iter()
                            .any(|node| matches!(node, ast::Node::ComponentInvocation(inv) if inv.name == "Button"))
                    {
                        return Err(CompileError::new(
                            span,
                            "Pressable `contextMenu` requires at least one Button action",
                        ));
                    }
                    let lowered = lower_nodes(nodes, &child_cx)?;
                    if lowered
                        .iter()
                        .any(|node| !matches!(node, Node::Button { .. }))
                    {
                        return Err(CompileError::new(
                            span,
                            "Pressable `contextMenu` currently accepts only Button actions",
                        ));
                    }
                    Ok(lowered)
                })
                .transpose()?
                .unwrap_or_default();
            let actions = lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            let long_press_actions = lower_actions_with_aliases(
                long_press_actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            let double_tap_actions = lower_actions_with_aliases(
                double_tap_actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            let drag_actions = lower_actions_with_aliases(
                drag_actions,
                &drag_symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            let pinch_actions = lower_actions_with_aliases(
                pinch_actions,
                &pinch_symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            Ok(Node::Pressable {
                disabled,
                haptic,
                fill_max_size,
                children: lowered_children,
                actions,
                double_tap_actions,
                long_press_duration_ms,
                long_press_actions,
                context_menu,
                drag_parameters,
                drag_actions,
                pinch_parameter,
                pinch_actions,
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
                cx.type_registries(),
                "NavigationStack root",
            )?;
            Ok(Node::NavigationStack {
                root: root.0,
                arguments: root.1,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationSplitView" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let detail_visible = take_required_arg(&mut args, &inv.name, "detailVisible", span)?;
            let detail_visible = require_mutable_binding(
                &detail_visible,
                &Type::Bool,
                symbols,
                span,
                "NavigationSplitView detailVisible",
            )?;
            let ast::ChildBody::SplitPanes { sidebar, detail } = inv.children else {
                return Err(child_mismatch(span));
            };
            Ok(Node::NavigationSplitView {
                detail_visible,
                sidebar: lower_nodes(sidebar, &child_cx)?,
                detail: lower_nodes(detail, &child_cx)?,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationBack" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "NavigationBack")?;
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
            Ok(Node::NavigationBack {
                label: localize_ui_text(label, comment.clone()),
            })
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
                cx.type_registries(),
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
            let large_only = args.remove("largeOnly");
            let title = args
                .remove("title")
                .map(|title| {
                    let title = lower_expr(&title, Some(&Type::String), &cx.exprs(false))?;
                    Ok::<_, CompileError>(localize_ui_text(title, None))
                })
                .transpose()?;
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
                large_only: optional_bool(large_only, false, "BottomSheet largeOnly")?,
                title,
                children: lowered_children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Dialog" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "Dialog")?;
            let is_presented = take_required_arg(&mut args, &inv.name, "isPresented", span)?;
            let title = take_required_arg(&mut args, &inv.name, "title", span)?;
            let message = take_required_arg(&mut args, &inv.name, "message", span)?;
            let state =
                require_mutable_binding(&is_presented, &Type::Bool, symbols, span, "Dialog")?;
            let title = localize_ui_text(
                lower_expr(&title, Some(&Type::String), &cx.exprs(false))?,
                comment.clone(),
            );
            let message = localize_ui_text(
                lower_expr(&message, Some(&Type::String), &cx.exprs(false))?,
                comment.clone(),
            );
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let lowered_children = lower_nodes(children, &child_cx)?;
            let mut text_input_count = 0;
            for child in &lowered_children {
                match child {
                    Node::TextInput {
                        secure: false,
                        multiline: false,
                        searchable: false,
                        actions,
                        ..
                    } if actions.is_empty() => text_input_count += 1,
                    Node::Button { .. } => {}
                    Node::TextInput { .. } => {
                        return Err(CompileError::new(
                            span,
                            "Dialog text input must be a single-line, plain TextInput without submit actions",
                        ));
                    }
                    _ => {
                        return Err(CompileError::new(
                            span,
                            "Dialog children must be Button actions and at most one TextInput",
                        ));
                    }
                }
            }
            if text_input_count > 1 {
                return Err(CompileError::new(
                    span,
                    "Dialog supports at most one TextInput",
                ));
            }
            Ok(Node::Dialog {
                state,
                title,
                message,
                children: lowered_children,
            })
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "ConfirmationDialog" => {
            let span = inv.span;
            let mut args = inv.arguments;
            let comment = take_translator_comment(&mut args, "ConfirmationDialog")?;
            let is_presented = take_required_arg(&mut args, &inv.name, "isPresented", span)?;
            let title = take_required_arg(&mut args, &inv.name, "title", span)?;
            let state = require_mutable_binding(
                &is_presented,
                &Type::Bool,
                symbols,
                span,
                "ConfirmationDialog",
            )?;
            let title = localize_ui_text(
                lower_expr(&title, Some(&Type::String), &cx.exprs(false))?,
                comment,
            );
            let children = match inv.children {
                ast::ChildBody::Nodes(children) => children,
                _ => return Err(child_mismatch(span)),
            };
            let children = lower_nodes(children, &child_cx)?;
            if children.is_empty()
                || children
                    .iter()
                    .any(|child| !matches!(child, Node::Button { .. }))
            {
                return Err(CompileError::new(
                    span,
                    "ConfirmationDialog children must be Button actions",
                ));
            }
            Ok(Node::ConfirmationDialog {
                state,
                title,
                children,
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
            let actions = lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )?;
            if lowered_children.len() == 1 {
                let mut child = lowered_children.pop().expect("one lowered refresh child");
                if let Node::FastList { plan } = &mut child {
                    if plan.reverse_layout() {
                        return Err(CompileError::new(
                            span,
                            "`RefreshControl` cannot be combined with FastList `reverseLayout`",
                        ));
                    }
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
        ast::Node::ComponentInvocation(inv)
            if inv.name == "AppBottomBar" || inv.name == "PagePager" =>
        {
            let span = inv.span;
            let is_pager = inv.name == "PagePager";
            let mut args = inv.arguments;
            let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
            let tint = args
                .remove("tint")
                .map(|value| lower_color_expression(value, "AppBottomBar tint", cx))
                .transpose()?;
            let tabs = match inv.children {
                ast::ChildBody::Tabs(tabs) => tabs,
                _ => return Err(child_mismatch(span)),
            };
            let state = require_mutable_binding(
                &selected,
                &Type::Numeric(NumericType::Int32),
                symbols,
                span,
                &inv.name,
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
                let comment = tab.comment;
                let icon = tab
                    .icon
                    .as_ref()
                    .map(|icon| {
                        let name = require_string_literal(icon, "Tab icon")?;
                        SystemIcon::shared(&name).ok_or_else(|| {
                            CompileError::new(
                                icon.span(),
                                format!("unknown shared tab icon `{name}`"),
                            )
                        })
                    })
                    .transpose()?;
                let badge = tab
                    .badge
                    .as_ref()
                    .map(|badge| require_string_literal(badge, "Tab badge"))
                    .transpose()?;
                let role = tab
                    .role
                    .as_ref()
                    .map(|role| require_string_literal(role, "Tab role"))
                    .transpose()?;
                let navigation_title = tab
                    .navigation_title
                    .as_ref()
                    .map(|title| require_string_literal(title, "Tab title"))
                    .transpose()?;
                let large_title = tab
                    .large_title
                    .as_ref()
                    .map(|value| optional_bool(Some(value.clone()), false, "largeTitle"))
                    .transpose()?
                    .unwrap_or(false);
                let search_state = tab
                    .searchable
                    .as_ref()
                    .map(|value| {
                        require_mutable_binding(
                            value,
                            &Type::String,
                            symbols,
                            span,
                            "Tab searchable",
                        )
                    })
                    .transpose()?;
                let search_prompt = tab
                    .search_prompt
                    .as_ref()
                    .map(|prompt| require_string_literal(prompt, "Tab searchPrompt"))
                    .transpose()?;
                let children = lower_nodes(tab.children, &child_cx)?;
                lowered_tabs.push(BottomBarTab {
                    index,
                    label,
                    comment,
                    icon,
                    badge,
                    role,
                    navigation_title,
                    large_title,
                    search_state,
                    search_prompt,
                    children,
                });
            }
            if is_pager {
                if lowered_tabs
                    .iter()
                    .enumerate()
                    .any(|(index, tab)| tab.index != index as i32)
                {
                    return Err(CompileError::new(
                        span,
                        "PagePager indexes must be contiguous, starting at zero",
                    ));
                }
                return Ok(Node::PagePager {
                    state,
                    pages: lowered_tabs.into_iter().map(|tab| tab.children).collect(),
                });
            }
            Ok(Node::AppBottomBar {
                state,
                tint,
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
            let reverse_layout = optional_bool(
                args.remove("reverseLayout"),
                false,
                "FastList reverseLayout",
            )?;
            let page_snap = optional_bool(args.remove("pageSnap"), false, "FastList pageSnap")?;
            let native = optional_bool(args.remove("native"), false, "FastList native")?;
            let mut modifiers = inv.modifiers;
            let on_end_reached = take_modifier_actions(&mut modifiers, span, "onEndReached")?;
            let on_scroll = take_modifier_actions(&mut modifiers, span, "onScroll")?;
            let on_move = match take_modifier_decl(&mut modifiers, "onMove") {
                Some(mut modifier) => {
                    let enabled = match modifier.arguments.remove("enabled") {
                        Some(expression) => {
                            lower_expr(&expression, Some(&Type::Bool), &cx.exprs(false))?
                        }
                        None => Expr::Bool(true),
                    };
                    if let Some(unknown) = modifier.arguments.keys().next() {
                        return Err(CompileError::new(
                            modifier.span,
                            format!("unknown `.onMove` argument `{unknown}`; expected `enabled`"),
                        ));
                    }
                    match modifier.body {
                        ast::ModifierBody::EventActions {
                            parameters,
                            actions,
                        } => {
                            if parameters.len() != 2 {
                                return Err(CompileError::new(
                                    span,
                                    ".onMove requires two bindings: from and to",
                                ));
                            }
                            let from = parameters[0].clone();
                            let to = parameters[1].clone();
                            if from == to {
                                return Err(CompileError::new(
                                    span,
                                    ".onMove bindings must have different names",
                                ));
                            }
                            let mut move_symbols = symbols.clone();
                            for parameter in [&from, &to] {
                                if symbols.contains_key(parameter) {
                                    return Err(CompileError::new(
                                        span,
                                        format!(
                                            ".onMove binding `{parameter}` shadows an existing value"
                                        ),
                                    ));
                                }
                                move_symbols.insert(
                                    parameter.clone(),
                                    (Type::Numeric(NumericType::Int32), false),
                                );
                            }
                            let actions = lower_actions_with_aliases(
                                actions,
                                &move_symbols,
                                functions,
                                false,
                                native_aliases,
                                cx.type_registries(),
                            )?;
                            Some(FastListMove {
                                from,
                                to,
                                enabled,
                                actions,
                            })
                        }
                        _ => return Err(child_mismatch(span)),
                    }
                }
                None => None,
            };
            let sticky_header = take_modifier_nodes(&mut modifiers, span, "stickyHeader")?;
            let section_header = take_modifier_nodes(&mut modifiers, span, "sectionHeader")?;
            let swipe_actions = take_modifier_nodes(&mut modifiers, span, "swipeActions")?;
            let sections_source = matches!(&source, ast::ListSource::Sections(_));
            if swipe_actions.is_some() && !native {
                return Err(CompileError::new(
                    span,
                    "FastList `.swipeActions` currently requires `native: true`",
                ));
            }
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
                Some(ast::Expr::Call(name, _, arguments, axis_span)) if name == "Grid" => {
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
            if native
                && (!matches!(axis, ListAxis::Vertical)
                    || reverse_layout
                    || page_snap
                    || item_extent.is_some()
                    || scroll_position.is_some()
                    || on_end_reached.is_some()
                    || on_scroll.is_some()
                    || sticky_header.is_some())
            {
                return Err(CompileError::new(
                    span,
                    "FastList `native` requires a vertical list without custom scroll behavior",
                ));
            }
            let array_source = matches!(&source, ast::ListSource::Items(_));
            if on_move.is_some() && (!native || !array_source) {
                return Err(CompileError::new(
                    span,
                    "FastList `.onMove` requires `native: true` and an array source",
                ));
            }
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
            if reverse_layout && !matches!(axis, ListAxis::Vertical) {
                return Err(CompileError::new(
                    span,
                    "FastList `reverseLayout` is supported only for vertical lists",
                ));
            }
            if reverse_layout && sections_source {
                return Err(CompileError::new(
                    span,
                    "FastList `reverseLayout` is supported only for flat count or collection sources",
                ));
            }
            if reverse_layout && sticky_header.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `reverseLayout` cannot be combined with `stickyHeader`",
                ));
            }
            if page_snap && !matches!(axis, ListAxis::Vertical) {
                return Err(CompileError::new(
                    span,
                    "FastList `pageSnap` is supported only for vertical lists",
                ));
            }
            if page_snap && sections_source {
                return Err(CompileError::new(
                    span,
                    "FastList `pageSnap` is supported only for flat count or collection sources",
                ));
            }
            if page_snap && reverse_layout {
                return Err(CompileError::new(
                    span,
                    "FastList `pageSnap` cannot be combined with `reverseLayout`",
                ));
            }
            if page_snap && sticky_header.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `pageSnap` cannot be combined with `stickyHeader`",
                ));
            }
            if page_snap && item_extent.is_some() {
                return Err(CompileError::new(
                    span,
                    "FastList `pageSnap` sets each row to the viewport height; omit `rowHeight`",
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
                    let collection_type = infer_expr_type(&collection, symbols, functions)
                        .ok_or_else(|| {
                            CompileError::new(
                                collection.span(),
                                "FastList positional source must have a statically known Array<T> type",
                            )
                        })?;
                    let Type::Array(element_type) = &collection_type else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList collection must have type Array<T>",
                        ));
                    };
                    let item_name = binding_name(item, "item", "FastList item")?;
                    if item_name == index {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList item and index bindings must have different names",
                        ));
                    }
                    let collection =
                        lower_expr(&collection, Some(&collection_type), &cx.exprs(false))?;
                    (
                        ListSourceParts::Items {
                            collection,
                            element_type: (**element_type).clone(),
                            item: item_name.clone(),
                        },
                        None,
                        Some(item_name),
                        Some((**element_type).clone()),
                    )
                }
                ast::ListSource::Sections(collection) => {
                    let collection_type = infer_expr_type(&collection, symbols, functions)
                        .ok_or_else(|| {
                            CompileError::new(
                                collection.span(),
                                "FastList `sections` must have a statically known Array<Array<T>> type",
                            )
                        })?;
                    let Type::Array(section_type) = &collection_type else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList `sections` must have type Array<Array<T>>",
                        ));
                    };
                    let Type::Array(element_type) = &**section_type else {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList `sections` must have type Array<Array<T>>",
                        ));
                    };
                    let item_name = binding_name(item, "item", "FastList item")?;
                    if item_name == index || item_name == section_name || index == section_name {
                        return Err(CompileError::new(
                            collection.span(),
                            "FastList section, item, and index bindings must have different names",
                        ));
                    }
                    let collection =
                        lower_expr(&collection, Some(&collection_type), &cx.exprs(false))?;
                    (
                        ListSourceParts::Sections {
                            collection,
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
            let row_cx = child_cx.with_symbols(&row_symbols);
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
                    lower_expr(&key, Some(&key_type), &row_cx.exprs(false))
                })
                .transpose()?;
            let lowered_children = lower_nodes(children, &row_cx)?;
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
            if swipe_actions.as_ref().is_some_and(Vec::is_empty) {
                return Err(CompileError::new(
                    span,
                    "FastList `swipeActions` requires at least one action component",
                ));
            }
            if swipe_actions.is_some() && lowered_children.len() != 1 {
                return Err(CompileError::new(
                    span,
                    "FastList `swipeActions` requires exactly one row root component",
                ));
            }
            let swipe_actions = swipe_actions
                .map(|actions| lower_nodes(actions, &row_cx))
                .transpose()?;
            if swipe_actions.as_ref().is_some_and(|actions| {
                actions.len() != 1 || !matches!(actions.first(), Some(Node::Button { .. }))
            }) {
                return Err(CompileError::new(
                    span,
                    "FastList `swipeActions` currently requires one Button action",
                ));
            }
            let on_end_reached = on_end_reached
                .map(|actions| {
                    lower_actions_with_aliases(
                        actions,
                        symbols,
                        functions,
                        false,
                        native_aliases,
                        cx.type_registries(),
                    )
                })
                .transpose()?;
            let on_scroll = on_scroll
                .map(|actions| {
                    lower_actions_with_aliases(
                        actions,
                        symbols,
                        functions,
                        false,
                        native_aliases,
                        cx.type_registries(),
                    )
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
                    if sections_source && let Some(item_type) = &item_type {
                        header_symbols.insert(
                            "sectionItems".to_owned(),
                            (Type::Array(Box::new(item_type.clone())), false),
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
                            native,
                            reverse_layout,
                            page_snap,
                            item_extent,
                            index,
                            key,
                            scroll_position,
                            children: lowered_children,
                            on_end_reached,
                            on_scroll,
                            on_move: None,
                            swipe_actions,
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
                            native,
                            reverse_layout,
                            page_snap,
                            item_extent,
                            index,
                            key,
                            scroll_position,
                            children: lowered_children,
                            on_end_reached,
                            on_scroll,
                            on_move,
                            swipe_actions,
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
                                native,
                                item_extent,
                                index,
                                key,
                                children: lowered_children,
                                section_header,
                                swipe_actions,
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
            transition,
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
                transition: transition.map(|transition| match transition {
                    ast::ViewTransition::Fade => IrViewTransition::Fade,
                    ast::ViewTransition::SlideFromBottom => IrViewTransition::SlideFromBottom,
                    ast::ViewTransition::SlideFromLeft => IrViewTransition::SlideFromLeft,
                    ast::ViewTransition::SlideFromRight => IrViewTransition::SlideFromRight,
                    ast::ViewTransition::Scale => IrViewTransition::Scale,
                }),
            })
        }
        ast::Node::When {
            value,
            cases,
            else_body,
            transition,
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
                transition: transition.map(|transition| match transition {
                    ast::ViewTransition::Fade => IrViewTransition::Fade,
                    ast::ViewTransition::SlideFromBottom => IrViewTransition::SlideFromBottom,
                    ast::ViewTransition::SlideFromLeft => IrViewTransition::SlideFromLeft,
                    ast::ViewTransition::SlideFromRight => IrViewTransition::SlideFromRight,
                    ast::ViewTransition::Scale => IrViewTransition::Scale,
                }),
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
                let actions = lower_actions_with_depth(
                    handler.actions,
                    &event_symbols,
                    functions,
                    false,
                    0,
                    cx.type_registries(),
                )?;
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

fn lower_text_alignment(
    value: Option<ast::Expr>,
) -> Result<Option<nexa_ir::TextAlignment>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Text alignment must be Leading, Center, or Trailing",
        ));
    };
    let alignment = match name.as_str() {
        "Leading" => nexa_ir::TextAlignment::Leading,
        "Center" => nexa_ir::TextAlignment::Center,
        "Trailing" => nexa_ir::TextAlignment::Trailing,
        _ => {
            return Err(CompileError::new(
                span,
                "Text alignment must be Leading, Center, or Trailing",
            ));
        }
    };
    Ok(Some(alignment))
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

fn lower_text_font_style(value: Option<ast::Expr>) -> Result<Option<TextFontStyle>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Text fontStyle must be LargeTitle, Title, Title2, Title3, Headline, Subheadline, Body, Callout, Footnote, Caption, or Caption2",
        ));
    };
    let style = match name.as_str() {
        "LargeTitle" => TextFontStyle::LargeTitle,
        "Title" => TextFontStyle::Title,
        "Title2" => TextFontStyle::Title2,
        "Title3" => TextFontStyle::Title3,
        "Headline" => TextFontStyle::Headline,
        "Subheadline" => TextFontStyle::Subheadline,
        "Body" => TextFontStyle::Body,
        "Callout" => TextFontStyle::Callout,
        "Footnote" => TextFontStyle::Footnote,
        "Caption" => TextFontStyle::Caption,
        "Caption2" => TextFontStyle::Caption2,
        _ => {
            return Err(CompileError::new(
                span,
                "Text fontStyle must be LargeTitle, Title, Title2, Title3, Headline, Subheadline, Body, Callout, Footnote, Caption, or Caption2",
            ));
        }
    };
    Ok(Some(style))
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

fn lower_text_input_font(value: Option<ast::Expr>) -> Result<Option<TextInputFont>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "TextInput font must be `Body` or `Title3`",
        ));
    };
    match name.as_str() {
        "Body" => Ok(Some(TextInputFont::Body)),
        "Title3" => Ok(Some(TextInputFont::Title3)),
        _ => Err(CompileError::new(
            span,
            "TextInput font must be `Body` or `Title3`",
        )),
    }
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
    registries: TypeRegistries<'_>,
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
                &registries.expr_context(symbols, functions, false),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((id, lowered))
}

pub(super) fn lower_actions_with_aliases(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
    registries: TypeRegistries<'_>,
) -> Result<Vec<Action>, CompileError> {
    lower_actions_with_aliases_depth(
        actions,
        symbols,
        functions,
        allow_await,
        native_aliases,
        0,
        registries,
    )
}

fn lower_actions_with_depth(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    loop_depth: usize,
    registries: TypeRegistries<'_>,
) -> Result<Vec<Action>, CompileError> {
    lower_actions_with_aliases_depth(
        actions,
        symbols,
        functions,
        allow_await,
        &HashMap::new(),
        loop_depth,
        registries,
    )
}

fn lower_actions_with_aliases_depth(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
    loop_depth: usize,
    registries: TypeRegistries<'_>,
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
        registries,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_actions_with_disposal_state(
    actions: Vec<ast::Stmt>,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    allow_await: bool,
    native_aliases: &HashMap<String, String>,
    loop_depth: usize,
    disposed_instances: &mut HashSet<String>,
    registries: TypeRegistries<'_>,
) -> Result<Vec<Action>, CompileError> {
    let mut symbol_scope = (*symbols).clone();
    let symbols = &mut symbol_scope;
    let mut lowered = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            ast::Stmt::Expression { expression, span } => {
                let expression = lower_expr(
                    &expression,
                    None,
                    &registries.expr_context(symbols, functions, allow_await),
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
            ast::Stmt::Let {
                name,
                ty,
                initial,
                span,
            } => {
                if symbols.contains_key(&name) {
                    return Err(CompileError::new(
                        span,
                        format!("local constant `{name}` shadows an existing value"),
                    ));
                }
                let local_type = resolve_value_type(
                    &name,
                    ty.as_ref(),
                    &initial,
                    symbols,
                    functions,
                    registries.structs,
                )?;
                let value = lower_expr(
                    &initial,
                    Some(&local_type),
                    &registries.expr_context(symbols, functions, allow_await),
                )?;
                symbols.insert(name.clone(), (local_type.clone(), false));
                lowered.push(Action::Let {
                    name,
                    ty: local_type,
                    value,
                });
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                // A collection state is a snapshot-state collection on Kotlin,
                // where the binding itself is a `val`: replacing the value means
                // replacing the contents. The IR records that so the backend and
                // the hot-reload interpreter agree with Swift.
                if matches!(ty, Type::Array(_) | Type::Set(_) | Type::Map(..)) {
                    lowered.push(Action::CollectionMutation {
                        name,
                        operation: CollectionMutation::Replace,
                        arguments: vec![value],
                    });
                } else {
                    lowered.push(Action::Assign { name, value });
                }
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                if matches!(&receiver, ast::Expr::Name(namespace, _) if namespace == "Network") {
                    if event != "onStatusChange" {
                        return Err(CompileError::new(
                            span,
                            format!("Network has no event `{event}`"),
                        ));
                    }
                    if parameters.len() != 1 {
                        return Err(CompileError::new(
                            span,
                            "Network.onStatusChange requires exactly one Bool binding",
                        ));
                    }
                    let parameter = &parameters[0];
                    if symbols.contains_key(parameter) {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "network event binding `{parameter}` shadows an existing value"
                            ),
                        ));
                    }
                    let mut event_symbols = symbols.clone();
                    event_symbols.insert(parameter.clone(), (Type::Bool, false));
                    let callback_functions = functions_with_error_handling(functions, false);
                    let actions = lower_actions_with_aliases_depth(
                        actions,
                        &event_symbols,
                        &callback_functions,
                        false,
                        native_aliases,
                        0,
                        registries,
                    )?;
                    lowered.push(Action::NetworkStatusSubscribe {
                        parameter: parameter.clone(),
                        actions,
                    });
                    continue;
                }
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                    registries,
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
                type_arguments,
                arguments,
                span,
            } => {
                let Some((ty, mutable)) = symbols.get(&name) else {
                    return Err(CompileError::new(span, format!("unknown state `{name}`")));
                };
                if matches!(ty, Type::Plugin { .. } | Type::Class { .. }) {
                    let expression = ast::Expr::MethodCall {
                        base: Box::new(ast::Expr::Name(name, span)),
                        name: method,
                        type_arguments: type_arguments.clone(),
                        arguments,
                        named_arguments: std::collections::BTreeMap::new(),
                        span,
                    };
                    let expression = lower_expr(
                        &expression,
                        None,
                        &registries.expr_context(symbols, functions, allow_await),
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
                if !type_arguments.is_empty() {
                    return Err(CompileError::new(
                        span,
                        "type arguments are only supported on plugin methods",
                    ));
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
                        "move" => (
                            CollectionMutation::ArrayMove,
                            vec![
                                &Type::Numeric(NumericType::Int32),
                                &Type::Numeric(NumericType::Int32),
                            ],
                        ),
                        "moveSubset" => (
                            CollectionMutation::ArrayMoveSubset,
                            vec![
                                &Type::Numeric(NumericType::Int32),
                                &Type::Numeric(NumericType::Int32),
                                ty,
                            ],
                        ),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "Array state `{name}` supports `append(value)`, `remove(index)`, `move(from, to)`, and `moveSubset(from, to, orderedSubset)`"
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
                        "clear" => (CollectionMutation::MapClear, vec![]),
                        _ => {
                            return Err(CompileError::new(
                                span,
                                format!(
                                    "Map state `{name}` supports `set(key, value)`, `remove(key)`, and `clear()`"
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
                            &registries.expr_context(symbols, functions, allow_await),
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
            ast::Stmt::TaskLaunch {
                handle,
                executor,
                body,
                span,
            } => {
                if let Some(handle) = &handle {
                    let Some((handle_type, mutable)) = symbols.get(handle) else {
                        return Err(CompileError::new(
                            span,
                            format!("unknown task handle state `{handle}`"),
                        ));
                    };
                    if !mutable || handle_type != &Type::Optional(Box::new(Type::TaskHandle)) {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "Task.launch requires `{handle}` to be a mutable `TaskHandle?` state initialized to `null`"
                            ),
                        ));
                    }
                }
                let actions = lower_actions_with_aliases(
                    body,
                    symbols,
                    functions,
                    true,
                    native_aliases,
                    registries,
                )?;
                if executor == ast::TaskExecutor::Background && task_actions_touch_state(&actions) {
                    return Err(CompileError::new(
                        span,
                        "background tasks cannot read or mutate UI state; use `TaskExecutor.Main` for stateful work",
                    ));
                }
                lowered.push(Action::TaskLaunch {
                    handle,
                    executor: match executor {
                        ast::TaskExecutor::Main => IrTaskExecutor::Main,
                        ast::TaskExecutor::Background => IrTaskExecutor::Background,
                    },
                    actions,
                });
            }
            ast::Stmt::TaskCancel { handle, span } => {
                let Some((handle_type, _)) = symbols.get(&handle) else {
                    return Err(CompileError::new(
                        span,
                        format!("unknown task handle state `{handle}`"),
                    ));
                };
                if handle_type != &Type::Optional(Box::new(Type::TaskHandle)) {
                    return Err(CompileError::new(
                        span,
                        format!("Task.cancel requires `{handle}` to have type `TaskHandle?`"),
                    ));
                }
                lowered.push(Action::TaskCancel { handle });
            }
            ast::Stmt::WithAnimation {
                animation,
                body,
                span,
            } => {
                let animation = parse_animation(animation)?;
                let actions = lower_actions_with_aliases(
                    body,
                    symbols,
                    functions,
                    false,
                    native_aliases,
                    registries,
                )?;
                let animated_states = collect_animation_targets(&actions, symbols, span)?;
                lowered.push(Action::WithAnimation {
                    animation,
                    animated_states,
                    actions,
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                    registries,
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
                            registries,
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
                let (iterable, element_type) = lower_for_iterable(
                    &iterable,
                    symbols,
                    functions,
                    allow_await,
                    registries,
                    span,
                )?;
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
                    registries,
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
                let (iterable, key_type, value_type) = lower_map_iterable(
                    &iterable,
                    symbols,
                    functions,
                    allow_await,
                    registries,
                    span,
                )?;
                validate_and_record_native_disposal(
                    &iterable,
                    disposed_instances,
                    native_aliases,
                    functions,
                    span,
                )?;
                let mut loop_symbols = symbols.clone();
                loop_symbols.insert(key_name.clone(), (key_type, false));
                loop_symbols.insert(value_name.clone(), (value_type, false));
                let mut body_disposed = disposed_instances.clone();
                let body = lower_actions_with_disposal_state(
                    body,
                    &loop_symbols,
                    functions,
                    allow_await,
                    native_aliases,
                    loop_depth + 1,
                    &mut body_disposed,
                    registries,
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
                    &registries.expr_context(symbols, functions, allow_await),
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
                    registries,
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
                    registries,
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
                        registries,
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
                            registries,
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

fn collect_animation_targets(
    actions: &[Action],
    symbols: &HashMap<String, (Type, bool)>,
    span: Span,
) -> Result<Vec<String>, CompileError> {
    fn collect(
        actions: &[Action],
        symbols: &HashMap<String, (Type, bool)>,
        span: Span,
        targets: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) -> Result<(), CompileError> {
        for action in actions {
            match action {
                Action::Assign { name, .. } => {
                    let Some((ty, mutable)) = symbols.get(name) else {
                        return Err(CompileError::new(
                            span,
                            format!("unknown state `{name}` in `withAnimation`"),
                        ));
                    };
                    if !mutable
                        || !matches!(
                            ty,
                            Type::Numeric(NumericType::Float32 | NumericType::Float64)
                        )
                    {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "`withAnimation` currently supports assignments to mutable Float32 or Float64 state; `{name}` has type `{}`",
                                type_name(ty)
                            ),
                        ));
                    }
                    if seen.insert(name.clone()) {
                        targets.push(name.clone());
                    }
                }
                Action::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    collect(then_branch, symbols, span, targets, seen)?;
                    if let Some(else_branch) = else_branch {
                        collect(else_branch, symbols, span, targets, seen)?;
                    }
                }
                Action::For { body, .. }
                | Action::ForMap { body, .. }
                | Action::While { body, .. } => collect(body, symbols, span, targets, seen)?,
                Action::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                } => {
                    collect(body, symbols, span, targets, seen)?;
                    for arm in error_catches {
                        collect(&arm.body, symbols, span, targets, seen)?;
                    }
                    if let Some(catch_body) = catch_body {
                        collect(catch_body, symbols, span, targets, seen)?;
                    }
                }
                Action::WithAnimation { actions, .. } => {
                    collect(actions, symbols, span, targets, seen)?;
                }
                Action::TaskLaunch { .. }
                | Action::TaskCancel { .. }
                | Action::NativeEventSubscribe { .. }
                | Action::NetworkStatusSubscribe { .. } => {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` only supports floating-point state assignments and control flow around them",
                    ));
                }
                Action::CollectionMutation { .. } => {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` supports floating-point state assignments, not collection mutations",
                    ));
                }
                Action::Expression(_)
                | Action::Let { .. }
                | Action::Return { .. }
                | Action::NativePropertyAssign { .. } => {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` only supports floating-point state assignments and control flow around them",
                    ));
                }
                Action::Break | Action::Continue => {}
            }
        }
        Ok(())
    }

    let mut targets = Vec::new();
    collect(actions, symbols, span, &mut targets, &mut HashSet::new())?;
    if targets.is_empty() {
        return Err(CompileError::new(
            span,
            "`withAnimation` must contain at least one mutable Float32 or Float64 state assignment",
        ));
    }
    Ok(targets)
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

pub(super) fn validate_typed_error_recovery(
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

fn task_actions_touch_state(actions: &[Action]) -> bool {
    let mut reads_state = false;
    nexa_ir::walk::walk_actions(actions, &mut |expression| {
        reads_state |= matches!(expression, Expr::State(_, _) | Expr::AnimatedState(_, _));
    });
    reads_state || actions_mutate_or_subscribe(actions)
}

fn actions_mutate_or_subscribe(actions: &[Action]) -> bool {
    actions.iter().any(|action| match action {
        Action::Assign { .. }
        | Action::NativePropertyAssign { .. }
        | Action::NativeEventSubscribe { .. }
        | Action::NetworkStatusSubscribe { .. }
        | Action::CollectionMutation { .. }
        | Action::TaskLaunch { .. }
        | Action::TaskCancel { .. } => true,
        Action::WithAnimation { actions, .. } => actions_mutate_or_subscribe(actions),
        Action::If {
            then_branch,
            else_branch,
            ..
        } => {
            actions_mutate_or_subscribe(then_branch)
                || else_branch
                    .as_deref()
                    .is_some_and(actions_mutate_or_subscribe)
        }
        Action::For { body, .. } | Action::ForMap { body, .. } | Action::While { body, .. } => {
            actions_mutate_or_subscribe(body)
        }
        Action::TryCatch {
            body,
            error_catches,
            catch_body,
        } => {
            actions_mutate_or_subscribe(body)
                || error_catches
                    .iter()
                    .any(|arm| actions_mutate_or_subscribe(&arm.body))
                || catch_body
                    .as_deref()
                    .is_some_and(actions_mutate_or_subscribe)
        }
        Action::Expression(_)
        | Action::Let { .. }
        | Action::Return { .. }
        | Action::Break
        | Action::Continue => false,
    })
}

fn visit_action_expressions(actions: &[Action], visit: &mut impl FnMut(&Expr)) {
    for action in actions {
        match action {
            Action::Expression(expression)
            | Action::Assign {
                value: expression, ..
            }
            | Action::Let {
                value: expression, ..
            }
            | Action::Return { value: expression } => {
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
            Action::TaskLaunch { actions, .. } | Action::WithAnimation { actions, .. } => {
                visit_action_expressions(actions, visit)
            }
            Action::TaskCancel { .. } => {}
            Action::NetworkStatusSubscribe { .. } => {}
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

fn require_f64_literal(expr: &ast::Expr, field: &str) -> Result<f64, CompileError> {
    let (raw, span, sign) = match expr {
        ast::Expr::Number(raw, span) => (raw, *span, 1.0),
        ast::Expr::Negate(inner, span) => match inner.as_ref() {
            ast::Expr::Number(raw, _) => (raw, *span, -1.0),
            _ => {
                return Err(CompileError::new(
                    expr.span(),
                    format!("{field} must be a Float64 literal"),
                ));
            }
        },
        _ => {
            return Err(CompileError::new(
                expr.span(),
                format!("{field} must be a Float64 literal"),
            ));
        }
    };
    let value = raw
        .parse::<f64>()
        .map_err(|_| CompileError::new(span, format!("{field} must be a Float64 literal")))?
        * sign;
    if !value.is_finite() {
        return Err(CompileError::new(span, format!("{field} must be finite")));
    }
    Ok(value)
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
