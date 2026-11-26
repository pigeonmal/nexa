//! Typed images component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_image_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let span = inv.span;
    let themes = cx.themes;
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
    let description = args.remove("description");
    let fit = args.remove("fit");
    let placeholder = args.remove("placeholder");
    let show_loading_indicator = optional_bool(
        args.remove("showLoadingIndicator"),
        false,
        "Image showLoadingIndicator",
    )?;
    let auto_replay = optional_bool(args.remove("autoReplay"), true, "Image autoReplay")?;
    let raw_style = super::layout::take_view_style_args(&mut args);
    let mut style = lower_style(raw_style, cx.target, themes, &cx.exprs(false))?;
    if style.effects.clip_rounded.is_none()
        && let Some(radius) = style.corner_radius
    {
        style.effects.clip_rounded = Some(radius);
    }
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
            let asset_span = asset.span();
            let asset = require_string_literal(&asset, "Image asset")?;
            ImageSource::Asset(validate_image_asset_filename(&asset, asset_span, "Image asset")?)
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
        ast::ImageSource::File(file) => {
            ImageSource::LocalFile(lower_expr(&file, Some(&Type::String), &cx.exprs(false))?)
        }
    };
    if show_loading_indicator && matches!(&source, ImageSource::Asset(_)) {
        return Err(CompileError::new(
            span,
            "Image `showLoadingIndicator` can only be used with a URL or file",
        ));
    }
    let description = match description {
        Some(expr) => require_string_literal(&expr, "Image description")?,
        None => String::new(),
    };
    let fit = match fit {
        Some(ast::Expr::Name(name, _)) if name == "Fit" => ImageScale::Fit,
        Some(ast::Expr::Name(name, _)) if name == "Fill" => ImageScale::Fill,
        Some(expr) => {
            return Err(CompileError::new(
                expr.span(),
                "Image fit must be `Fit` or `Fill`",
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
            let expr_span = expr.span();
            let placeholder = require_string_literal(&expr, "Image placeholder")?;
            Some(validate_image_asset_filename(
                &placeholder,
                expr_span,
                "Image placeholder",
            )?)
        }
        None => None,
    };
    Ok(Node::Image {
        source,
        description,
        fit,
        placeholder,
        show_loading_indicator,
        auto_replay,
        style,
        shared_element,
    })
}

fn validate_image_asset_filename(
    value: &str,
    span: Span,
    label: &str,
) -> Result<String, CompileError> {
    let Some((stem, ext)) = value.rsplit_once('.') else {
        return Err(CompileError::new(
            span,
            format!("{label} must include a supported file extension (e.g. 'hero.png')"),
        ));
    };
    if !is_resource_name(stem) {
        return Err(CompileError::new(
            span,
            format!("{label} base name must be an identifier suitable for an Android resource"),
        ));
    }
    let ext_lower = ext.to_ascii_lowercase();
    if !matches!(
        ext_lower.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif"
    ) {
        return Err(CompileError::new(
            span,
            format!("{label} has unsupported extension `.{ext}`; use .png, .jpg, .jpeg, .webp, or .gif"),
        ));
    }
    Ok(format!("{stem}.{ext_lower}"))
}

#[inline(never)]
pub(super) fn lower_icon_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let span = inv.span;
    let expr = super::super::expressions::lower_icon_constructor(
        &[],
        &inv.positional,
        Some(&inv.arguments),
        span,
        Some(&Type::Icon),
        &cx.exprs(false),
        false,
    )?;
    if let Expr::Icon(spec) = expr {
        let description = spec.description;
        let tint = spec.color;
        let size = spec.size;
        let layer_colors = spec.layer_colors;
        return match spec.source {
            nexa_ir::IconSource::System(icon) => Ok(Node::SystemIcon {
                icon,
                description,
                size,
                tint: tint.or_else(|| layer_colors.into_iter().next()),
            }),
            nexa_ir::IconSource::FontGlyph {
                font_family,
                font_resource,
                glyph,
                layers,
            } => Ok(Node::FontIcon {
                font_family,
                font_resource,
                glyph: Expr::Number {
                    raw: glyph.to_string(),
                    ty: NumericType::Int32,
                },
                layers,
                description,
                size: size.map(|size| Expr::Number {
                    raw: size.to_string(),
                    ty: NumericType::Float64,
                }),
                tint,
                layer_colors,
            }),
        };
    }
    Ok(Node::Icon { icon: expr })
}

#[inline(never)]
pub(super) fn lower_linear_gradient_component(
    inv: ast::ComponentInvocation,
    _cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
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
        Some(ast::Expr::Name(name, _)) if name == "TopToBottom" => GradientDirection::TopToBottom,
        Some(ast::Expr::Name(name, _)) if name == "BottomToTop" => GradientDirection::BottomToTop,
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
