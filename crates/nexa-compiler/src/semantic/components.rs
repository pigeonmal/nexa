mod app_bottom_bar;
mod appearance;
mod bottom_sheet;
mod button;
mod confirmation_dialog;
mod content_unavailable;
mod dialog;
mod fast_list;
mod forms;
mod images;
mod keyboard_aware;
mod layout;
mod lifecycle;
mod navigation;
mod picker;
mod picker_date;
mod pressable;
mod progress_bar;
mod refresh_control;
mod scroll_view;
mod segmented_control;
mod shapes;
mod slider;
mod switch;
mod text;
mod text_input;

use std::collections::{BTreeMap, HashMap, HashSet};

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::walk::any_node;
use nexa_ir::{
    AccessibilityRole, Action, AnimationSpec, AutofillType, BottomBarTab, ButtonShape, ButtonSize,
    ButtonStyle, Capitalization, CollectionMutation, ColorExpression, ColorValue, DirectionConfig,
    DirectionStyle, Expr, FastListMove, FastListRefresh, FontWeight, GradientDirection,
    HapticStyle, ImageScale, ImageSource, KeyboardDismissMode, KeyboardType, LayoutKind, ListAxis,
    ListCommon, ListPlan, NativeComponentEventHandler, Node, NumberFlow, NumberFlowFormat,
    NumericType, PickerDateSelection, PickerDateStyle, PickerDateType, ReturnKeyType, ScreenId,
    SectionedListCommon, StatusBarConfig, StatusBarStyle, SystemIcon,
    TaskExecutor as IrTaskExecutor, TextFontDesign, TextFontStyle, TextInputFont, TextStyle,
    ToolbarPlacement, Type, ViewTransition as IrViewTransition, WhenCase,
    adaptive_layout::{LayoutEdge, SafeAreaInsetPlan, ScrollHeaderBehavior, ScrollHeaderPlan},
};
use nexa_syntax::{ast, catalog};

use super::{
    context::{ScreenSignatures, SemanticContext, TypeRegistries},
    expressions::{
        FunctionSignatures, functions_with_error_handling, infer_expr_type, lower_expr,
        lower_for_iterable, lower_map_iterable, plugin_error_variant, resolve_struct_type,
        resolve_value_type, type_name,
    },
    styles::{
        lower_opacity, lower_style, lower_view_effects, number_value, optional_color,
        optional_dimension, optional_tracking, parse_animation, parse_color, parse_color_literal,
    },
    themes::{ThemeSymbols, validate_font_family},
};
use crate::Target;

type AccessibilityOptions = (Expr, Option<Expr>, Option<Expr>, AccessibilityRole);

/// Move a RefreshControl callback onto a single FastList child, even when
/// FastList's own accessibility options have lowered to a one-child wrapper.
/// Keeping the refresh action on the native list lets each platform use its
/// native scroll container and keeps short lists refreshable on Android.
fn attach_refresh_to_fast_list(
    node: &mut Node,
    refresh: &FastListRefresh,
    span: Span,
) -> Result<bool, CompileError> {
    match node {
        Node::FastList { plan } => {
            if plan.reverse_layout() {
                return Err(CompileError::new(
                    span,
                    "`RefreshControl` cannot be combined with FastList `reverseLayout`",
                ));
            }
            *plan.refresh_slot() = Some(refresh.clone());
            Ok(true)
        }
        Node::Accessibility { children, .. } if children.len() == 1 => {
            attach_refresh_to_fast_list(&mut children[0], refresh, span)
        }
        _ => Ok(false),
    }
}

fn parse_shape(value: ast::Expr, subject: &str) -> Result<ButtonShape, CompileError> {
    match value {
        ast::Expr::String(value, span) => match value.to_ascii_lowercase().as_str() {
            "capsule" => Ok(ButtonShape::Capsule),
            "circle" => Ok(ButtonShape::Circle),
            other => {
                if let Some(radius) = other
                    .strip_prefix("rounded(")
                    .and_then(|radius| radius.strip_suffix(')'))
                    .and_then(|radius| radius.parse::<f32>().ok())
                {
                    return Ok(ButtonShape::Rounded(radius));
                }
                Err(CompileError::new(
                    span,
                    format!("unknown {subject} `{other}`"),
                ))
            }
        },
        ast::Expr::Name(value, span) => match value.as_str() {
            "Capsule" => Ok(ButtonShape::Capsule),
            "Circle" => Ok(ButtonShape::Circle),
            other => Err(CompileError::new(
                span,
                format!("unknown {subject} `{other}`"),
            )),
        },
        ast::Expr::Call(name, _, arguments, _span) if name == "Rounded" && arguments.len() == 1 => {
            Ok(ButtonShape::Rounded(number_value(
                &arguments[0],
                "shape radius",
            )?))
        }
        other => Err(CompileError::new(
            other.span(),
            format!("{subject} must be capsule, circle, or rounded"),
        )),
    }
}

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
                    && match &node {
                        ast::Node::ComponentInvocation(invocation) => {
                            invocation.name == "NavigationStack" || invocation.name == "Appearance"
                        }
                        ast::Node::If { .. } | ast::Node::When { .. } => true,
                        _ => false,
                    };
                let child_cx = cx.with_navigation(allow_navigation, cx.allow_navigation_back);
                lowered.push(lower_node(node, &child_cx)?);
            }
        }
    }
    Ok(lowered)
}

pub(super) fn lower_node(mut node: ast::Node, cx: &SemanticContext) -> Result<Node, CompileError> {
    let mut padding = None;
    let mut margin = None;
    if let ast::Node::ComponentInvocation(invocation) = &mut node {
        padding = invocation.arguments.remove("padding");
        margin = invocation.arguments.remove("margin");
    }
    let padding = padding
        .map(|value| {
            super::styles::lower_edge_insets(value, "padding", cx.themes, &cx.exprs(false))
        })
        .transpose()?;
    let margin = margin
        .map(|value| super::styles::lower_edge_insets(value, "margin", cx.themes, &cx.exprs(false)))
        .transpose()?;
    lower_node_with_modifiers(node, cx, padding, margin)
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

fn lower_text_reveal(
    args: &mut BTreeMap<String, ast::Expr>,
    span: Span,
    cx: &SemanticContext<'_>,
) -> Result<nexa_ir::TextReveal, CompileError> {
    let component = "TextReveal";
    let segments_source = take_required_arg(args, component, "segments", span)?;
    let segments_type = infer_expr_type(&segments_source, cx.symbols, cx.functions)
        .map(|ty| resolve_struct_type(&ty, cx.structs))
        .ok_or_else(|| {
            CompileError::new(
                segments_source.span(),
                "`segments` must be a typed array of text timing structs",
            )
        })?;
    let Type::Array(segment_type) = &segments_type else {
        return Err(CompileError::new(
            segments_source.span(),
            "`segments` must be an array of structs with text and timing fields",
        ));
    };
    let Type::Struct { fields, .. } = segment_type.as_ref() else {
        return Err(CompileError::new(
            segments_source.span(),
            "`segments` elements must be declared structs with text and timing fields",
        ));
    };
    let field_type = |name: &str| {
        fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, ty)| ty)
    };
    if field_type("text") != Some(&Type::String) {
        return Err(CompileError::new(
            segments_source.span(),
            "`segments` elements require a `text: String` field",
        ));
    }
    for name in ["startTimeSeconds", "endTimeSeconds"] {
        if !matches!(field_type(name), Some(Type::Numeric(_))) {
            return Err(CompileError::new(
                segments_source.span(),
                format!("`segments` elements require a numeric `{name}` field"),
            ));
        }
    }
    if let Some(trailing_space) = field_type("hasTrailingSpace")
        && !(matches!(trailing_space, Type::Bool)
            || matches!(trailing_space, Type::Optional(inner) if matches!(inner.as_ref(), Type::Bool)))
    {
        return Err(CompileError::new(
            segments_source.span(),
            "`segments.hasTrailingSpace`, when present, must be Bool or Bool?",
        ));
    }
    let segments = lower_expr(&segments_source, Some(&segments_type), &cx.exprs(false))?;

    let position_source = take_required_arg(args, component, "position", span)?;
    let position = lower_expr(
        &position_source,
        Some(&Type::Numeric(NumericType::Float64)),
        &cx.exprs(false),
    )?;
    let bool_argument = |args: &mut BTreeMap<String, ast::Expr>, name: &str| {
        let source = take_required_arg(args, component, name, span)?;
        lower_expr(&source, Some(&Type::Bool), &cx.exprs(false))
    };
    let is_active = bool_argument(args, "isActive")?;
    let is_playing = bool_argument(args, "isPlaying")?;
    let reveal_enabled = bool_argument(args, "revealEnabled")?;
    let seek_revision_source = take_required_arg(args, component, "seekRevision", span)?;
    let seek_revision_type = infer_expr_type(&seek_revision_source, cx.symbols, cx.functions)
        .ok_or_else(|| {
            CompileError::new(
                seek_revision_source.span(),
                "`seekRevision` must be an integer value",
            )
        })?;
    if !matches!(
        seek_revision_type,
        Type::Numeric(
            NumericType::Int8
                | NumericType::Int16
                | NumericType::Int32
                | NumericType::Int64
                | NumericType::UInt8
                | NumericType::UInt16
                | NumericType::UInt32
                | NumericType::UInt64
        )
    ) {
        return Err(CompileError::new(
            seek_revision_source.span(),
            "`seekRevision` must be an integer value",
        ));
    }
    let seek_revision = lower_expr(&seek_revision_source, None, &cx.exprs(false))?;
    let dim_opacity = lower_opacity(args.remove("revealOpacity"))?.unwrap_or(0.38);
    let mut lower_timing = |name: &str, default: f32| -> Result<f32, CompileError> {
        args.remove(name)
            .map(|value| number_value(&value, name))
            .transpose()
            .map(|value| value.unwrap_or(default))
    };

    Ok(nexa_ir::TextReveal {
        segments,
        segment_type: segment_type.as_ref().clone(),
        position,
        is_active,
        is_playing,
        reveal_enabled,
        seek_revision,
        dim_opacity,
        lead_time: lower_timing("leadTime", 0.3)?,
        target_offset: lower_timing("targetOffset", 0.56)?,
        smoothing_duration: lower_timing("smoothingDuration", 0.25)?,
    })
}

fn lower_number_flow(
    args: &mut BTreeMap<String, ast::Expr>,
    span: Span,
) -> Result<NumberFlow, CompileError> {
    let format = match args.remove("format") {
        None => NumberFlowFormat::Number,
        Some(ast::Expr::Name(name, _)) => match name.as_str() {
            "Number" => NumberFlowFormat::Number,
            "Currency" => NumberFlowFormat::Currency,
            "Percent" => NumberFlowFormat::Percent,
            "Compact" => NumberFlowFormat::Compact,
            _ => {
                return Err(CompileError::new(
                    span,
                    "NumberFlow format must be Number, Currency, Percent, or Compact",
                ));
            }
        },
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "NumberFlow format must be Number, Currency, Percent, or Compact",
            ));
        }
    };
    let currency_code = match args.remove("currencyCode") {
        Some(ast::Expr::String(code, code_span)) => {
            if format != NumberFlowFormat::Currency {
                return Err(CompileError::new(
                    code_span,
                    "NumberFlow `currencyCode` is only valid with `format: Currency`",
                ));
            }
            let code = code.to_ascii_uppercase();
            if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                return Err(CompileError::new(
                    code_span,
                    "NumberFlow currencyCode must be a three-letter ISO 4217 code",
                ));
            }
            Some(code)
        }
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "NumberFlow currencyCode must be a static three-letter string",
            ));
        }
        None if format == NumberFlowFormat::Currency => {
            return Err(CompileError::new(
                span,
                "NumberFlow with `format: Currency` requires `currencyCode`",
            ));
        }
        None => None,
    };
    let minimum_fraction_digits = lower_number_flow_fraction_digits(
        args.remove("minimumFractionDigits"),
        "minimumFractionDigits",
    )?;
    let maximum_fraction_digits = lower_number_flow_fraction_digits(
        args.remove("maximumFractionDigits"),
        "maximumFractionDigits",
    )?;
    if let (Some(minimum), Some(maximum)) = (minimum_fraction_digits, maximum_fraction_digits)
        && minimum > maximum
    {
        return Err(CompileError::new(
            span,
            "NumberFlow minimumFractionDigits cannot exceed maximumFractionDigits",
        ));
    }
    let animation = args
        .remove("animation")
        .map(parse_animation)
        .transpose()?
        .unwrap_or(AnimationSpec::Spring {
            response: 0.35,
            damping: 0.82,
        });
    Ok(NumberFlow {
        format,
        currency_code,
        minimum_fraction_digits,
        maximum_fraction_digits,
        animation,
    })
}

fn lower_number_flow_fraction_digits(
    value: Option<ast::Expr>,
    name: &str,
) -> Result<Option<u8>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Number(raw, span) = value else {
        return Err(CompileError::new(
            value.span(),
            format!("NumberFlow `{name}` must be an integer literal from 0 to 20"),
        ));
    };
    let digits = raw.parse::<u8>().ok().filter(|digits| *digits <= 20);
    digits.map(Some).ok_or_else(|| {
        CompileError::new(
            span,
            format!("NumberFlow `{name}` must be an integer literal from 0 to 20"),
        )
    })
}

fn enum_argument(value: &ast::Expr, context: &str) -> Result<String, CompileError> {
    match value {
        ast::Expr::Name(name, _) | ast::Expr::String(name, _) => Ok(name.clone()),
        ast::Expr::EnumCase { case_name, .. } => Ok(case_name.clone()),
        _ => Err(CompileError::new(
            value.span(),
            format!("{context} must be a named enum value or string"),
        )),
    }
}

fn validate_icon_size_literal_if_static(size: &ast::Expr) -> Result<(), CompileError> {
    let ast::Expr::Number(value, span) = size else {
        return Ok(());
    };
    let valid = value
        .parse::<f32>()
        .ok()
        .is_some_and(|value| value.is_finite() && value > 0.0 && value <= 512.0);
    if valid {
        Ok(())
    } else {
        Err(CompileError::new(
            *span,
            "Icon size must be a positive numeric literal no greater than 512",
        ))
    }
}

fn bottom_sheet_detents(value: Option<ast::Expr>) -> Result<Vec<f32>, CompileError> {
    let Some(value) = value else {
        return Ok(vec![0.5, 1.0]);
    };
    let values = match &value {
        ast::Expr::Array(values, _) => values,
        _ => {
            return Err(CompileError::new(
                value.span(),
                "BottomSheet detents must be an array of fractions such as `[0.5, 1.0]`",
            ));
        }
    };
    if values.is_empty() {
        return Err(CompileError::new(
            value.span(),
            "BottomSheet requires at least one detent fraction",
        ));
    }
    let mut detents = Vec::with_capacity(values.len());
    for value in values {
        let detent = number_value(value, "BottomSheet detent")?;
        if !(detent.is_finite() && 0.0 < detent && detent <= 1.0) {
            return Err(CompileError::new(
                value.span(),
                "BottomSheet detents must be greater than 0 and at most 1.0",
            ));
        }
        if detents.last().is_some_and(|previous| *previous >= detent) {
            return Err(CompileError::new(
                value.span(),
                "BottomSheet detents must be unique and ordered from smallest to largest",
            ));
        }
        detents.push(detent);
    }
    Ok(detents)
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
    lower_expr(&value, Some(&Type::String), &cx.exprs(false))
        .map(|expression| ColorExpression::Dynamic(Box::new(expression)))
}

fn lower_progress_bar(
    progress: Option<Expr>,
    mut args: BTreeMap<String, ast::Expr>,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "ProgressBar tint", cx))
        .transpose()?;
    let track_color = args
        .remove("trackColor")
        .map(|value| lower_color_expression(value, "ProgressBar trackColor", cx))
        .transpose()?;
    let thickness = optional_dimension(
        args.remove("thickness"),
        "ProgressBar thickness",
        None,
        cx.themes,
    )?;
    let corner_radius = optional_dimension(
        args.remove("cornerRadius"),
        "ProgressBar cornerRadius",
        None,
        cx.themes,
    )?;
    let android_wave = args
        .remove("androidWave")
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let style = if tint.is_some()
        || track_color.is_some()
        || thickness.is_some()
        || corner_radius.is_some()
        || android_wave.is_some()
    {
        Some(Box::new(nexa_ir::ProgressBarStyle {
            tint,
            track_color,
            thickness,
            corner_radius,
            android_wave,
        }))
    } else {
        None
    };
    Ok(Node::ProgressBar { progress, style })
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

fn lower_node_with_modifiers(
    mut node: ast::Node,
    cx: &SemanticContext,
    padding: Option<nexa_ir::EdgeInsets>,
    margin: Option<nexa_ir::EdgeInsets>,
) -> Result<Node, CompileError> {
    let shared_modifiers = match &mut node {
        ast::Node::ComponentInvocation(invocation) => {
            let mut shared = Vec::new();
            let mut index = 0;
            while index < invocation.modifiers.len() {
                if catalog::SHARED_VIEW_MODIFIERS
                    .iter()
                    .any(|entry| entry.modifier.name == invocation.modifiers[index].name)
                {
                    shared.push(invocation.modifiers.remove(index));
                } else {
                    index += 1;
                }
            }
            shared
        }
        _ => Vec::new(),
    };
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
    let mut lowered = lower_node_raw(node, cx)?;
    if padding.is_some() || margin.is_some() {
        lowered = Node::ViewInsets {
            padding,
            margin,
            child: Box::new(lowered),
        };
    }
    lowered = apply_shared_view_modifiers(lowered, shared_modifiers, cx)?;
    match accessibility {
        Some((label, hint, value, role)) => {
            if let Node::Button {
                accessibility_label,
                ..
            } = &mut lowered
            {
                *accessibility_label = Some(Box::new(label.clone()));
            }
            if matches!(lowered, Node::Button { .. })
                && hint.is_none()
                && value.is_none()
                && role == AccessibilityRole::None
            {
                Ok(lowered)
            } else {
                Ok(Node::Accessibility {
                    label,
                    hint,
                    value,
                    role,
                    children: vec![lowered],
                })
            }
        }
        None => Ok(lowered),
    }
}

fn apply_shared_view_modifiers(
    mut node: Node,
    modifiers: Vec<ast::DotModifier>,
    cx: &SemanticContext,
) -> Result<Node, CompileError> {
    for mut modifier in modifiers {
        match modifier.name.as_str() {
            "safeAreaInset" => {
                let edge = modifier
                    .arguments
                    .get("edge")
                    .ok_or_else(|| {
                        CompileError::new(
                            modifier.span,
                            "modifier `.safeAreaInset` requires `edge`",
                        )
                    })
                    .and_then(|value| parse_safe_area_edge(value, "edge"))?;
                let ast::ModifierBody::Nodes(content) = modifier.body else {
                    return Err(CompileError::new(
                        modifier.span,
                        "modifier `.safeAreaInset` requires a content block",
                    ));
                };
                let content = lower_nodes(content, cx)?;
                let scroll_owner = if contains_navigation_host(std::slice::from_ref(&node)) {
                    // A navigation host renders one destination at a time.
                    // Route-local scroll owners therefore share the attached
                    // endpoint reservation instead of making it ambiguous.
                    nexa_ir::adaptive_layout::ScrollOwnerResolution::RouteHost
                } else {
                    nexa_ir::walk::resolve_scroll_owner(std::slice::from_ref(&node), edge, None)
                };
                node = Node::SafeAreaInset {
                    plan: SafeAreaInsetPlan { edge, scroll_owner },
                    content,
                    children: vec![node],
                };
            }
            "scrollHeader" => {
                let behavior = lower_scroll_header_behavior(modifier.arguments.remove("behavior"))?;
                let collapsed_height = modifier
                    .arguments
                    .remove("collapsedHeight")
                    .map(|value| number_value(&value, "collapsedHeight"))
                    .transpose()?;
                if collapsed_height.is_some_and(|height| height <= 0.0) {
                    return Err(CompileError::new(
                        modifier.span,
                        "`collapsedHeight` must be greater than zero",
                    ));
                }
                let collapsed_height = match (behavior, collapsed_height) {
                    (ScrollHeaderBehavior::Collapse, Some(height)) => Some(height),
                    (ScrollHeaderBehavior::Collapse, None) => Some(56.0),
                    (_, Some(_)) => {
                        return Err(CompileError::new(
                            modifier.span,
                            "`collapsedHeight` is only valid with `behavior: Collapse`",
                        ));
                    }
                    (_, None) => None,
                };
                let compatible = nexa_ir::walk::scroll_owners(std::slice::from_ref(&node))
                    .into_iter()
                    .filter(|owner| owner.axes.supports_edge(LayoutEdge::Top))
                    .collect::<Vec<_>>();
                if compatible.is_empty() {
                    return Err(CompileError::new(
                        modifier.span,
                        "modifier `.scrollHeader` requires a vertical scroll owner such as `ScrollView`, `FastList`, `Form`, or `KeyboardAware`",
                    ));
                }
                if compatible.len() > 1 {
                    return Err(CompileError::new(
                        modifier.span,
                        "modifier `.scrollHeader` is ambiguous with multiple vertical scroll owners; attach it directly to one owner",
                    ));
                }
                let ast::ModifierBody::Nodes(header) = modifier.body else {
                    return Err(CompileError::new(
                        modifier.span,
                        "modifier `.scrollHeader` requires a header content block",
                    ));
                };
                node = Node::ScrollHeader {
                    plan: ScrollHeaderPlan {
                        behavior,
                        collapsed_height,
                    },
                    header: lower_nodes(header, cx)?,
                    children: vec![node],
                };
            }
            _ => {
                return Err(CompileError::new(
                    modifier.span,
                    format!(
                        "shared modifier `.{}` has no semantic lowering",
                        modifier.name
                    ),
                ));
            }
        }
    }
    Ok(node)
}

fn contains_navigation_host(nodes: &[Node]) -> bool {
    if nodes.len() != 1 {
        return false;
    }
    match &nodes[0] {
        Node::NavigationStack { .. } => true,
        Node::Layout { children, .. }
        | Node::Form { children, .. }
        | Node::Toolbar { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::SafeAreaInset { children, .. } => contains_navigation_host(children),
        _ => false,
    }
}

fn parse_safe_area_edge(value: &ast::Expr, argument: &str) -> Result<LayoutEdge, CompileError> {
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            format!("`{argument}` must be Top, Bottom, Leading, or Trailing"),
        ));
    };
    match name.as_str() {
        "Top" => Ok(LayoutEdge::Top),
        "Bottom" => Ok(LayoutEdge::Bottom),
        "Leading" => Ok(LayoutEdge::Leading),
        "Trailing" => Ok(LayoutEdge::Trailing),
        _ => Err(CompileError::new(
            *span,
            format!("unknown safe-area edge `{name}`; expected Top, Bottom, Leading, or Trailing"),
        )),
    }
}

fn lower_scroll_header_behavior(
    value: Option<ast::Expr>,
) -> Result<ScrollHeaderBehavior, CompileError> {
    let Some(value) = value else {
        return Ok(ScrollHeaderBehavior::Pinned);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "scrollHeader behavior must be `Pinned`, `Collapse`, or `HideOnScroll`",
        ));
    };
    match name.as_str() {
        "Pinned" => Ok(ScrollHeaderBehavior::Pinned),
        "Collapse" => Ok(ScrollHeaderBehavior::Collapse),
        "HideOnScroll" => Ok(ScrollHeaderBehavior::HideOnScroll),
        _ => Err(CompileError::new(
            span,
            "scrollHeader behavior must be `Pinned`, `Collapse`, or `HideOnScroll`",
        )),
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

fn lower_node_raw(node: ast::Node, cx: &SemanticContext) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let components = cx.components;
    let functions = cx.functions;

    let allow_navigation_stack = cx.allow_navigation_stack;
    let allow_navigation_back = cx.allow_navigation_back;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);
    match node {
        ast::Node::Platform { span, .. } => Err(CompileError::new(
            span,
            "platform blocks must be expanded before component lowering",
        )),
        ast::Node::ComponentInvocation(inv) if inv.name == "Content" => Ok(Node::Content),
        ast::Node::ComponentInvocation(inv) if inv.name == "Toolbar" => {
            navigation::lower_toolbar_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "StatusBar" => {
            appearance::lower_status_bar_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Appearance" => {
            appearance::lower_appearance_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Direction" => {
            appearance::lower_direction_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnAppear" => {
            lifecycle::lower_on_appear_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnDisappear" => {
            lifecycle::lower_on_disappear_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnActive" => {
            lifecycle::lower_on_active_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnInactive" => {
            lifecycle::lower_on_inactive_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "OnBackground" => {
            lifecycle::lower_on_background_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv)
            if inv.name == "Column" || inv.name == "Row" || inv.name == "Stack" =>
        {
            layout::lower_column_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "ScrollView" => {
            scroll_view::lower_scroll_view_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Form" => {
            forms::lower_form_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Section" => {
            forms::lower_section_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "ContentUnavailable" => {
            content_unavailable::lower_content_unavailable_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv)
            if matches!(inv.name.as_str(), "Text" | "TextReveal" | "NumberFlow") =>
        {
            text::lower_text_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Spacer" => {
            shapes::lower_spacer_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Divider" => {
            shapes::lower_divider_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Shape" => {
            shapes::lower_shape_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Slider" => {
            slider::lower_slider_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv)
            if inv.name == "ProgressBar" || inv.name == "ProgressRing" =>
        {
            progress_bar::lower_progress_bar_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "SegmentedControl" => {
            segmented_control::lower_segmented_control_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Picker" => {
            picker::lower_picker_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "PickerDate" => {
            picker_date::lower_picker_date_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Button" => {
            button::lower_button_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "TextInput" => {
            text_input::lower_text_input_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Switch" => {
            switch::lower_switch_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Image" => {
            images::lower_image_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Icon" => {
            images::lower_icon_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "LinearGradient" => {
            images::lower_linear_gradient_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Pressable" => {
            pressable::lower_pressable_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationStack" => {
            navigation::lower_navigation_stack_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationSplitView" => {
            navigation::lower_navigation_split_view_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationBack" => {
            navigation::lower_navigation_back_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "NavigationLink" => {
            navigation::lower_navigation_link_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Link" => {
            navigation::lower_link_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "KeyboardAware" => {
            keyboard_aware::lower_keyboard_aware_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "BottomSheet" => {
            bottom_sheet::lower_bottom_sheet_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "Dialog" => {
            dialog::lower_dialog_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "ConfirmationDialog" => {
            confirmation_dialog::lower_confirmation_dialog_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "RefreshControl" => {
            refresh_control::lower_refresh_control_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv)
            if inv.name == "AppBottomBar" || inv.name == "PagePager" =>
        {
            app_bottom_bar::lower_app_bottom_bar_component(inv, cx)
        }
        ast::Node::ComponentInvocation(inv) if inv.name == "FastList" => {
            fast_list::lower_fast_list_component(inv, cx)
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
            transition_animation,
            ..
        } => {
            let condition = lower_expr(&condition, Some(&Type::Bool), &cx.exprs(false))?;
            let branch_cx = cx.with_navigation(allow_navigation_stack, allow_navigation_back);
            let lowered_then = lower_nodes(then_body, &branch_cx)?;
            let lowered_else = else_body
                .map(|body| lower_nodes(body, &branch_cx))
                .transpose()?;
            Ok(Node::If {
                condition,
                then_body: lowered_then,
                else_body: lowered_else,
                transition: transition.map(|transition| match transition {
                    ast::ViewTransition::Fade => IrViewTransition::Fade,
                    ast::ViewTransition::FadeSlideFromTop => IrViewTransition::FadeSlideFromTop,
                    ast::ViewTransition::SlideFromBottom => IrViewTransition::SlideFromBottom,
                    ast::ViewTransition::SlideFromLeft => IrViewTransition::SlideFromLeft,
                    ast::ViewTransition::SlideFromRight => IrViewTransition::SlideFromRight,
                    ast::ViewTransition::Scale => IrViewTransition::Scale,
                }),
                transition_animation: transition_animation.map(parse_animation).transpose()?,
            })
        }
        ast::Node::When {
            value,
            cases,
            else_body,
            has_else,
            transition,
            transition_animation,
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
            let branch_cx = cx.with_navigation(allow_navigation_stack, allow_navigation_back);
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
                let body = lower_nodes(case.body, &branch_cx)?;
                lowered_cases.push(WhenCase {
                    value: lowered_case,
                    body,
                });
            }
            if !has_else && !when_is_exhaustive(&value_type, &lowered_cases, cx.symbols) {
                return Err(CompileError::new(
                    span,
                    "when needs an `else` case unless it covers every Bool or enum case",
                ));
            }
            let lowered_else = has_else
                .then(|| lower_nodes(else_body, &branch_cx))
                .transpose()?;
            Ok(Node::When {
                value: lowered_value,
                cases: lowered_cases,
                else_body: lowered_else,
                transition: transition.map(|transition| match transition {
                    ast::ViewTransition::Fade => IrViewTransition::Fade,
                    ast::ViewTransition::FadeSlideFromTop => IrViewTransition::FadeSlideFromTop,
                    ast::ViewTransition::SlideFromBottom => IrViewTransition::SlideFromBottom,
                    ast::ViewTransition::SlideFromLeft => IrViewTransition::SlideFromLeft,
                    ast::ViewTransition::SlideFromRight => IrViewTransition::SlideFromRight,
                    ast::ViewTransition::Scale => IrViewTransition::Scale,
                }),
                transition_animation: transition_animation.map(parse_animation).transpose()?,
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
            let component_ref = arguments.remove("ref");
            if component_ref.is_some() && !signature.supports_ref {
                return Err(CompileError::new(
                    span,
                    format!(
                        "component `{name}` has no ref methods; declare a `fn` inside the component before passing `ref:`"
                    ),
                ));
            }
            let component_ref = component_ref
                .map(|reference| {
                    lower_expr(
                        &reference,
                        Some(&Type::ComponentRef {
                            namespace: "Nexa".to_owned(),
                            name: name.clone(),
                        }),
                        &cx.exprs(false),
                    )
                })
                .transpose()?;
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
                component_ref,
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
            if arguments.contains_key("ref") && !signature.supports_ref {
                return Err(CompileError::new(
                    span,
                    format!("native component `{qualified_name}` does not expose ref methods"),
                ));
            }
            let component_ref = arguments
                .remove("ref")
                .map(|reference| {
                    lower_expr(
                        &reference,
                        Some(&Type::ComponentRef {
                            namespace: namespace.clone(),
                            name: name.clone(),
                        }),
                        &cx.exprs(false),
                    )
                })
                .transpose()?;
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
                component_ref,
                children: lowered_children,
                event_handlers: lowered_events,
            })
        }
    }
}

/// A missing `else` is valid only for closed value sets whose complete case
/// list is known to the compiler. Strings and numbers remain open-ended.
fn when_is_exhaustive(
    value_type: &Type,
    cases: &[WhenCase],
    symbols: &HashMap<String, (Type, bool)>,
) -> bool {
    match value_type {
        Type::Bool => {
            let mut true_case = false;
            let mut false_case = false;
            for case in cases {
                match case.value {
                    Expr::Bool(true) => true_case = true,
                    Expr::Bool(false) => false_case = true,
                    _ => {}
                }
            }
            true_case && false_case
        }
        Type::Enum(enum_name) => {
            let prefix = format!("{enum_name}.");
            let expected = symbols
                .iter()
                .filter_map(|(name, (ty, _))| {
                    (ty == value_type)
                        .then(|| name.strip_prefix(&prefix))
                        .flatten()
                })
                .collect::<HashSet<_>>();
            if expected.is_empty() {
                return false;
            }
            let covered = cases
                .iter()
                .filter_map(|case| match &case.value {
                    Expr::EnumValue {
                        enum_name: case_enum,
                        case_name,
                    } if case_enum == enum_name => Some(case_name.as_str()),
                    _ => None,
                })
                .collect::<HashSet<_>>();
            expected == covered
        }
        _ => false,
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
    navigation_bar_background: Option<ast::Expr>,
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
    let navigation_bar_background = navigation_bar_background
        .map(|value| parse_color_literal(value, "StatusBar navigationBarBackground"))
        .transpose()?
        .map(nexa_ir::ColorValue::Static);
    Ok(StatusBarConfig {
        style,
        hidden,
        background,
        navigation_bar_background,
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

fn lower_keyboard_behavior(
    value: Option<ast::Expr>,
) -> Result<nexa_ir::adaptive_layout::KeyboardBehavior, CompileError> {
    let Some(value) = value else {
        return Ok(nexa_ir::adaptive_layout::KeyboardBehavior::Stay);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "KeyboardAware behavior must be `Stay` or `Avoid`",
        ));
    };
    match name.as_str() {
        "Stay" => Ok(nexa_ir::adaptive_layout::KeyboardBehavior::Stay),
        "Avoid" => Ok(nexa_ir::adaptive_layout::KeyboardBehavior::Avoid),
        _ => Err(CompileError::new(
            span,
            "KeyboardAware behavior must be `Stay` or `Avoid`",
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

fn lower_text_font_design(
    value: Option<ast::Expr>,
) -> Result<Option<TextFontDesign>, CompileError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ast::Expr::Name(name, span) = value else {
        return Err(CompileError::new(
            value.span(),
            "Text fontDesign must be Default, Serif, Rounded, or Monospaced",
        ));
    };
    let design = match name.as_str() {
        "Default" => TextFontDesign::Default,
        "Serif" => TextFontDesign::Serif,
        "Rounded" => TextFontDesign::Rounded,
        "Monospaced" => TextFontDesign::Monospaced,
        _ => {
            return Err(CompileError::new(
                span,
                "Text fontDesign must be Default, Serif, Rounded, or Monospaced",
            ));
        }
    };
    Ok(Some(design))
}

fn lower_text_font_family(
    value: Option<ast::Expr>,
    themes: &ThemeSymbols,
) -> Result<Option<String>, CompileError> {
    match value {
        None => Ok(themes.default_font_family().map(str::to_owned)),
        Some(ast::Expr::String(family, span)) => {
            validate_font_family(&family, span)?;
            Ok(Some(family))
        }
        Some(ast::Expr::ThemeToken(name, span)) => themes.font_family(&name, span).map(Some),
        Some(value) => Err(CompileError::new(
            value.span(),
            "Text fontFamily must be a static string or a fontFamily theme token",
        )),
    }
}

fn lower_icon_font_family(value: ast::Expr, themes: &ThemeSymbols) -> Result<String, CompileError> {
    match value {
        ast::Expr::String(family, span) => {
            validate_font_family(&family, span)?;
            Ok(family)
        }
        ast::Expr::ThemeToken(name, span) => themes.font_family(&name, span),
        value => Err(CompileError::new(
            value.span(),
            "Icon fontFamily must be a static string or a fontFamily theme token",
        )),
    }
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

pub(super) fn lower_screen_target(
    screen: &ast::Expr,
    arguments: Vec<ast::Expr>,
    screen_ids: &ScreenSignatures,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    registries: TypeRegistries<'_>,
    role: &str,
) -> Result<(ScreenId, Vec<Expr>), CompileError> {
    let id = resolve_screen(screen, screen_ids, role)?;
    let name = match screen {
        ast::Expr::Name(name, _) => name,
        _ => {
            return Err(CompileError::new(
                screen.span(),
                format!("{role} must be a declared screen name"),
            ));
        }
    };
    let Some(signature) = screen_ids.get(name) else {
        return Err(CompileError::new(
            screen.span(),
            format!("unknown screen `{name}` in {role}"),
        ));
    };
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

#[allow(clippy::too_many_arguments)]
fn lower_slider_event(
    body: Option<ast::ModifierBody>,
    span: Span,
    modifier_name: &str,
    parameter_type: Type,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    native_aliases: &HashMap<String, String>,
    registries: TypeRegistries<'_>,
) -> Result<Option<(String, Vec<Action>)>, CompileError> {
    match body {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => {
            if parameters.len() != 1 {
                return Err(CompileError::new(
                    span,
                    format!(".{modifier_name} requires one binding"),
                ));
            }
            let Some(parameter) = parameters.into_iter().next() else {
                return Err(CompileError::new(
                    span,
                    format!(".{modifier_name} requires one binding"),
                ));
            };
            if symbols.contains_key(&parameter) {
                return Err(CompileError::new(
                    span,
                    format!(".{modifier_name} binding `{parameter}` shadows an existing value"),
                ));
            }
            let mut callback_symbols = symbols.clone();
            callback_symbols.insert(parameter.clone(), (parameter_type, false));
            let actions = lower_actions_with_aliases(
                actions,
                &callback_symbols,
                functions,
                false,
                native_aliases,
                registries,
            )?;
            Ok(Some((parameter, actions)))
        }
        Some(_) => Err(child_mismatch(span)),
        None => Ok(None),
    }
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
                if let Some(action) =
                    lower_navigation_action(&expression, symbols, functions, registries)?
                {
                    lowered.push(action);
                    continue;
                }
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
                let (animated_states, has_state_assignment) =
                    collect_animation_targets(&actions, symbols, span)?;
                if !has_state_assignment {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` must contain at least one mutable state assignment",
                    ));
                }
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

fn lower_navigation_action(
    expression: &ast::Expr,
    symbols: &HashMap<String, (Type, bool)>,
    functions: &FunctionSignatures,
    registries: TypeRegistries<'_>,
) -> Result<Option<Action>, CompileError> {
    let ast::Expr::QualifiedCall {
        namespace,
        name,
        arguments,
        named_arguments,
        span: call_span,
        ..
    } = expression
    else {
        return Ok(None);
    };
    if namespace != "Navigation" || !matches!(name.as_str(), "push" | "back") {
        return Ok(None);
    }
    if !registries.allow_navigation_actions {
        return Err(CompileError::new(
            *call_span,
            "Navigation.push and Navigation.back are only allowed inside a declared screen",
        ));
    }
    if !named_arguments.is_empty() {
        return Err(CompileError::new(
            *call_span,
            format!("Navigation.{name} does not accept named arguments"),
        ));
    }
    match name.as_str() {
        "back" => {
            if !arguments.is_empty() {
                return Err(CompileError::new(
                    *call_span,
                    "Navigation.back() does not accept arguments",
                ));
            }
            Ok(Some(Action::NavigationBack))
        }
        "push" => {
            if arguments.len() != 1 {
                return Err(CompileError::new(
                    *call_span,
                    "Navigation.push expects one screen target, such as `OrderDetail(orderID)`",
                ));
            }
            let Some(screen_ids) = registries.screen_ids else {
                return Err(CompileError::new(
                    *call_span,
                    "Navigation.push requires declared app screens",
                ));
            };
            let Some(target) = arguments.first() else {
                return Err(CompileError::new(
                    *call_span,
                    "Navigation.push expects one screen target",
                ));
            };
            let (screen, route_arguments) = ast::split_navigation_target(target.clone());
            let (destination, arguments) = lower_screen_target(
                &screen,
                route_arguments,
                screen_ids,
                symbols,
                functions,
                registries,
                "Navigation.push destination",
            )?;
            Ok(Some(Action::NavigationPush {
                destination,
                arguments,
            }))
        }
        _ => Ok(None),
    }
}

fn collect_animation_targets(
    actions: &[Action],
    symbols: &HashMap<String, (Type, bool)>,
    span: Span,
) -> Result<(Vec<String>, bool), CompileError> {
    fn collect(
        actions: &[Action],
        symbols: &HashMap<String, (Type, bool)>,
        span: Span,
        targets: &mut Vec<String>,
        seen: &mut HashSet<String>,
        has_state_assignment: &mut bool,
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
                    let is_float = matches!(
                        ty,
                        Type::Numeric(NumericType::Float32 | NumericType::Float64)
                    );
                    if !mutable || !is_float {
                        return Err(CompileError::new(
                            span,
                            format!(
                                "`withAnimation` supports mutable Float32 or Float64 state; use a layout animation or view transition for collection changes. `{name}` has type `{}`",
                                type_name(ty)
                            ),
                        ));
                    }
                    *has_state_assignment = true;
                    if is_float && seen.insert(name.clone()) {
                        targets.push(name.clone());
                    }
                }
                Action::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    collect(
                        then_branch,
                        symbols,
                        span,
                        targets,
                        seen,
                        has_state_assignment,
                    )?;
                    if let Some(else_branch) = else_branch {
                        collect(
                            else_branch,
                            symbols,
                            span,
                            targets,
                            seen,
                            has_state_assignment,
                        )?;
                    }
                }
                Action::For { body, .. }
                | Action::ForMap { body, .. }
                | Action::While { body, .. } => {
                    collect(body, symbols, span, targets, seen, has_state_assignment)?
                }
                Action::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                } => {
                    collect(body, symbols, span, targets, seen, has_state_assignment)?;
                    for arm in error_catches {
                        collect(
                            &arm.body,
                            symbols,
                            span,
                            targets,
                            seen,
                            has_state_assignment,
                        )?;
                    }
                    if let Some(catch_body) = catch_body {
                        collect(
                            catch_body,
                            symbols,
                            span,
                            targets,
                            seen,
                            has_state_assignment,
                        )?;
                    }
                }
                Action::WithAnimation { actions, .. } => {
                    collect(actions, symbols, span, targets, seen, has_state_assignment)?
                }
                Action::TaskLaunch { .. }
                | Action::TaskCancel { .. }
                | Action::NativeEventSubscribe { .. }
                | Action::NetworkStatusSubscribe { .. }
                | Action::NavigationPush { .. }
                | Action::NavigationBack
                | Action::NavigationCommand(_) => {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` only supports mutable Float32 or Float64 state assignments and control flow around them; use a layout animation or view transition for collection changes",
                    ));
                }
                Action::CollectionMutation { name, .. } => {
                    let Some((ty, _)) = symbols.get(name) else {
                        return Err(CompileError::new(
                            span,
                            format!("unknown state `{name}` in `withAnimation`"),
                        ));
                    };
                    return Err(CompileError::new(
                        span,
                        format!(
                            "`withAnimation` cannot animate collection mutations of `{name}` ({}); use a layout animation or view transition",
                            type_name(ty)
                        ),
                    ));
                }
                Action::Expression(_)
                | Action::Let { .. }
                | Action::Return { .. }
                | Action::NativePropertyAssign { .. } => {
                    return Err(CompileError::new(
                        span,
                        "`withAnimation` only supports mutable Float32 or Float64 state assignments and control flow around them; use a layout animation or view transition for collection changes",
                    ));
                }
                Action::Break | Action::Continue => {}
            }
        }
        Ok(())
    }

    let mut targets = Vec::new();
    let mut has_state_assignment = false;
    collect(
        actions,
        symbols,
        span,
        &mut targets,
        &mut HashSet::new(),
        &mut has_state_assignment,
    )?;
    Ok((targets, has_state_assignment))
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
        // These nodes preserve source/await behavior but do not declare a
        // separate error type. The visitor also reaches the wrapped call,
        // which owns the actual typed plugin error contract.
        if matches!(expression, Expr::Located { .. } | Expr::TryAwait(_)) {
            return;
        }
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
        | Action::TaskCancel { .. }
        | Action::NavigationPush { .. }
        | Action::NavigationBack
        | Action::NavigationCommand(_) => true,
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
            Action::NavigationPush { arguments, .. } => {
                for argument in arguments {
                    nexa_ir::walk::walk_expression(argument, visit);
                }
            }
            Action::NavigationBack => {}
            Action::NavigationCommand(cmd) => match cmd.as_ref() {
                nexa_ir::NavigationCommand::Push { arguments, .. }
                | nexa_ir::NavigationCommand::ReplaceTop { arguments, .. }
                | nexa_ir::NavigationCommand::Reset { arguments, .. } => {
                    for argument in arguments {
                        nexa_ir::walk::walk_expression(argument, visit);
                    }
                }
                _ => {}
            },
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

fn lower_picker_item_icons(
    expr: ast::Expr,
    cx: &SemanticContext<'_>,
) -> Result<Vec<SystemIcon>, CompileError> {
    let ast::Expr::Array(items, _) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "Picker `itemIcons` must be an array literal of shared system icon names",
        ));
    };
    items
        .iter()
        .map(|item| {
            let name = require_string_literal(item, "Picker `itemIcons` entry")?;
            cx.resolve_icon(&name).ok_or_else(|| {
                CompileError::new(
                    item.span(),
                    format!("unknown icon `{name}` in Picker `itemIcons`; use a Nexa shared icon or an alias from `assets.icons`"),
                )
            })
        })
        .collect()
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
