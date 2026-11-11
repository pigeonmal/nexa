//! Typed images component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_image_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
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
    let show_loading_indicator = optional_bool(
        args.remove("showLoadingIndicator"),
        false,
        "Image showLoadingIndicator",
    )?;
    let fill_max_size = optional_bool(args.remove("fillMaxSize"), false, "Image fillMaxSize")?;
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
        show_loading_indicator,
        max_height,
        fill_max_size,
        shared_element,
    })
}

#[inline(never)]
pub(super) fn lower_icon_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;

    let span = inv.span;
    let mut args = inv.arguments;
    let font_family_arg = args.remove("fontFamily");
    let glyph_arg = args.remove("glyph");
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
    if selection.is_some() && (font_family_arg.is_some() || glyph_arg.is_some()) {
        return Err(CompileError::new(
            span,
            "Icon accepts either a system symbol or `fontFamily` with `glyph`",
        ));
    }
    let font_family = font_family_arg
        .map(|value| lower_icon_font_family(value, themes))
        .transpose()?;
    let glyph = glyph_arg
        .map(|value| {
            lower_expr(
                &value,
                Some(&Type::Numeric(NumericType::Int32)),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    if glyph.is_some() != font_family.is_some() {
        return Err(CompileError::new(
            span,
            "font glyph icons require both `fontFamily` and `glyph`",
        ));
    }
    let description_expr = take_required_arg(&mut args, &inv.name, "description", span)?;
    let description = require_string_literal(&description_expr, "Icon description")?;
    let size = take_required_arg(&mut args, &inv.name, "size", span)?;
    let tint = lower_color_expression(
        take_required_arg(&mut args, &inv.name, "tint", span)?,
        "Icon tint",
        cx,
    )?;
    if let (Some(font_family), Some(glyph)) = (font_family, glyph) {
        validate_icon_size_literal_if_static(&size)?;
        let size = lower_expr(
            &size,
            Some(&Type::Numeric(NumericType::Float64)),
            &cx.exprs(false),
        )?;
        Ok(Node::FontIcon {
            font_family,
            glyph,
            description,
            size,
            tint,
        })
    } else {
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
                    "system Icon size must be a positive numeric literal",
                ));
            }
        };
        let icon = match selection {
                    Some((0, name)) => cx.resolve_icon(&name).ok_or_else(|| {
                        CompileError::new(
                            span,
                            format!("unknown icon `{name}`; use a Nexa shared icon, an alias from `assets.icons`, `sfsymbol`, or `materialsymbol` for a platform-specific name"),
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
                            "Icon requires one system symbol or a font glyph with `fontFamily`",
                        ));
                    }
                };
        Ok(Node::SystemIcon {
            icon,
            description,
            size,
            tint,
        })
    }
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
