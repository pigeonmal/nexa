use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::{ColorValue, FontWeight, TextFontStyle};
use nexa_syntax::ast;

use super::styles::{number_value, parse_color_literal};

#[derive(Clone, Default)]
pub(super) struct ThemeSymbols {
    tokens: HashMap<String, ResolvedThemeToken>,
}

#[derive(Clone)]
enum ResolvedThemeToken {
    Color(ColorValue),
    FontFamily(String),
    FontStyle(ThemeFontStyle),
    Metric {
        kind: ast::ThemeTokenKind,
        value: f32,
    },
}

#[derive(Clone)]
pub(super) struct ThemeFontStyle {
    pub(super) size: Option<f32>,
    pub(super) weight: Option<FontWeight>,
    pub(super) family: Option<String>,
}

impl ThemeSymbols {
    pub(super) fn default_font_family(&self) -> Option<&str> {
        match self.tokens.get("Default") {
            Some(ResolvedThemeToken::FontFamily(family)) => Some(family),
            _ => None,
        }
    }

    pub(super) fn font_style(&self, style: TextFontStyle) -> Option<&ThemeFontStyle> {
        match self.tokens.get(text_font_style_name(style)) {
            Some(ResolvedThemeToken::FontStyle(style)) => Some(style),
            _ => None,
        }
    }

    pub(super) fn font_family(&self, name: &str, span: Span) -> Result<String, CompileError> {
        match self.tokens.get(name) {
            Some(ResolvedThemeToken::FontFamily(family)) => Ok(family.clone()),
            Some(ResolvedThemeToken::Color(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a color token and cannot be used as a font family"),
            )),
            Some(ResolvedThemeToken::FontStyle(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a fontStyle token and cannot be used as a font family"),
            )),
            Some(ResolvedThemeToken::Metric { kind, .. }) => Err(CompileError::new(
                span,
                format!(
                    "`Theme.{name}` is a {} token and cannot be used as a font family",
                    token_kind_name(*kind)
                ),
            )),
            None => Err(unknown_token(name, span)),
        }
    }

    pub(super) fn color(
        &self,
        name: &str,
        span: Span,
        role: &str,
    ) -> Result<ColorValue, CompileError> {
        match self.tokens.get(name) {
            Some(ResolvedThemeToken::Color(color)) => Ok(*color),
            Some(ResolvedThemeToken::Metric { kind, .. }) => Err(CompileError::new(
                span,
                format!(
                    "`Theme.{name}` is a {} token and cannot be used as {role}",
                    token_kind_name(*kind)
                ),
            )),
            Some(ResolvedThemeToken::FontFamily(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a fontFamily token and cannot be used as {role}"),
            )),
            Some(ResolvedThemeToken::FontStyle(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a fontStyle token and cannot be used as {role}"),
            )),
            None => Err(unknown_token(name, span)),
        }
    }

    pub(super) fn metric(
        &self,
        name: &str,
        span: Span,
        expected_kind: ast::ThemeTokenKind,
        role: &str,
    ) -> Result<f32, CompileError> {
        match self.tokens.get(name) {
            Some(ResolvedThemeToken::Metric { kind, value }) if *kind == expected_kind => {
                Ok(*value)
            }
            Some(ResolvedThemeToken::Metric { kind, .. }) => Err(CompileError::new(
                span,
                format!(
                    "`Theme.{name}` is a {} token and cannot be used as {role}; expected a {} token",
                    token_kind_name(*kind),
                    token_kind_name(expected_kind)
                ),
            )),
            Some(ResolvedThemeToken::Color(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a color token and cannot be used as {role}"),
            )),
            Some(ResolvedThemeToken::FontFamily(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a fontFamily token and cannot be used as {role}"),
            )),
            Some(ResolvedThemeToken::FontStyle(_)) => Err(CompileError::new(
                span,
                format!("`Theme.{name}` is a fontStyle token and cannot be used as {role}"),
            )),
            None => Err(unknown_token(name, span)),
        }
    }
}

pub(super) fn lower_theme(theme: Option<&ast::ThemeDecl>) -> Result<ThemeSymbols, CompileError> {
    let mut symbols = ThemeSymbols::default();
    let Some(theme) = theme else {
        return Ok(symbols);
    };

    for token in &theme.tokens {
        if symbols.tokens.contains_key(&token.name) {
            return Err(CompileError::new(
                token.span,
                format!("theme token `{}` is already declared", token.name),
            ));
        }

        let resolved = match (&token.kind, &token.value) {
            (ast::ThemeTokenKind::Color, ast::ThemeTokenValue::AdaptiveColor { light, dark }) => {
                let light = parse_color_literal(light.clone(), "light theme color")?;
                let dark = parse_color_literal(dark.clone(), "dark theme color")?;
                ResolvedThemeToken::Color(ColorValue::Adaptive { light, dark })
            }
            (ast::ThemeTokenKind::Color, ast::ThemeTokenValue::Static(_)) => {
                return Err(CompileError::new(
                    token.span,
                    "theme colors must provide both `light` and `dark` values",
                ));
            }
            (
                kind @ (ast::ThemeTokenKind::Spacing
                | ast::ThemeTokenKind::Radius
                | ast::ThemeTokenKind::FontSize),
                ast::ThemeTokenValue::Static(value),
            ) => ResolvedThemeToken::Metric {
                kind: *kind,
                value: number_value(value, token_kind_name(*kind))?,
            },
            (
                ast::ThemeTokenKind::FontFamily,
                ast::ThemeTokenValue::Static(ast::Expr::String(family, span)),
            ) => {
                validate_font_family(family, *span)?;
                ResolvedThemeToken::FontFamily(family.clone())
            }
            (ast::ThemeTokenKind::FontFamily, ast::ThemeTokenValue::Static(value)) => {
                return Err(CompileError::new(
                    value.span(),
                    "fontFamily theme values must be a static font family string",
                ));
            }
            (
                ast::ThemeTokenKind::FontStyle,
                ast::ThemeTokenValue::FontStyle {
                    size,
                    weight,
                    family,
                },
            ) => {
                parse_font_style_role(&token.name, token.span)?;
                if size.is_none() && weight.is_none() && family.is_none() {
                    return Err(CompileError::new(
                        token.span,
                        "fontStyle theme tokens require `size`, `weight`, or `family`",
                    ));
                }
                let size = size
                    .as_ref()
                    .map(|value| number_value(value, "fontStyle size"))
                    .transpose()?;
                let weight = weight.as_ref().map(parse_font_weight).transpose()?;
                let family = family.as_ref().map(parse_font_style_family).transpose()?;
                ResolvedThemeToken::FontStyle(ThemeFontStyle {
                    size,
                    weight,
                    family,
                })
            }
            (_, ast::ThemeTokenValue::AdaptiveColor { .. }) => {
                return Err(CompileError::new(
                    token.span,
                    "only `color` theme tokens accept `light` and `dark` values",
                ));
            }
            (_, ast::ThemeTokenValue::FontStyle { .. }) => {
                return Err(CompileError::new(
                    token.span,
                    "only `fontStyle` theme tokens accept `size`, `weight`, and `family` values",
                ));
            }
            (_, ast::ThemeTokenValue::Static(value)) => {
                return Err(CompileError::new(
                    value.span(),
                    format!(
                        "{} theme values use the wrong value form",
                        token_kind_name(token.kind)
                    ),
                ));
            }
        };

        symbols.tokens.insert(token.name.clone(), resolved);
    }

    Ok(symbols)
}

fn unknown_token(name: &str, span: Span) -> CompileError {
    CompileError::new(span, format!("unknown theme token `Theme.{name}`"))
}

fn token_kind_name(kind: ast::ThemeTokenKind) -> &'static str {
    match kind {
        ast::ThemeTokenKind::Color => "color",
        ast::ThemeTokenKind::Spacing => "spacing",
        ast::ThemeTokenKind::Radius => "radius",
        ast::ThemeTokenKind::FontSize => "fontSize",
        ast::ThemeTokenKind::FontFamily => "fontFamily",
        ast::ThemeTokenKind::FontStyle => "fontStyle",
    }
}

pub(super) fn parse_font_style_role(name: &str, span: Span) -> Result<TextFontStyle, CompileError> {
    match name {
        "LargeTitle" => Ok(TextFontStyle::LargeTitle),
        "Title" => Ok(TextFontStyle::Title),
        "Title2" => Ok(TextFontStyle::Title2),
        "Title3" => Ok(TextFontStyle::Title3),
        "Headline" => Ok(TextFontStyle::Headline),
        "Subheadline" => Ok(TextFontStyle::Subheadline),
        "Body" => Ok(TextFontStyle::Body),
        "Callout" => Ok(TextFontStyle::Callout),
        "Footnote" => Ok(TextFontStyle::Footnote),
        "Caption" => Ok(TextFontStyle::Caption),
        "Caption2" => Ok(TextFontStyle::Caption2),
        _ => Err(CompileError::new(
            span,
            "fontStyle must be LargeTitle, Title, Title2, Title3, Headline, Subheadline, Body, Callout, Footnote, Caption, or Caption2",
        )),
    }
}

fn text_font_style_name(style: TextFontStyle) -> &'static str {
    match style {
        TextFontStyle::LargeTitle => "LargeTitle",
        TextFontStyle::Title => "Title",
        TextFontStyle::Title2 => "Title2",
        TextFontStyle::Title3 => "Title3",
        TextFontStyle::Headline => "Headline",
        TextFontStyle::Subheadline => "Subheadline",
        TextFontStyle::Body => "Body",
        TextFontStyle::Callout => "Callout",
        TextFontStyle::Footnote => "Footnote",
        TextFontStyle::Caption => "Caption",
        TextFontStyle::Caption2 => "Caption2",
    }
}

fn parse_font_weight(expr: &ast::Expr) -> Result<FontWeight, CompileError> {
    let ast::Expr::Name(name, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "fontStyle weight must be Normal, Medium, Semibold, or Bold",
        ));
    };
    match name.as_str() {
        "Normal" => Ok(FontWeight::Normal),
        "Medium" => Ok(FontWeight::Medium),
        "Semibold" => Ok(FontWeight::Semibold),
        "Bold" => Ok(FontWeight::Bold),
        _ => Err(CompileError::new(
            *span,
            "fontStyle weight must be Normal, Medium, Semibold, or Bold",
        )),
    }
}

fn parse_font_style_family(expr: &ast::Expr) -> Result<String, CompileError> {
    let ast::Expr::String(family, span) = expr else {
        return Err(CompileError::new(
            expr.span(),
            "fontStyle family must be a static font family string",
        ));
    };
    validate_font_family(family, *span)?;
    Ok(family.clone())
}

pub(super) fn validate_font_family(value: &str, span: Span) -> Result<(), CompileError> {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return Err(CompileError::new(span, "font family cannot be empty"));
    };
    if !first.is_ascii_alphabetic()
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '_' | '-')
        })
        || value.ends_with([' ', '_', '-'])
    {
        return Err(CompileError::new(
            span,
            "font family must start with a letter and contain only letters, numbers, spaces, `_`, or `-`",
        ));
    }
    Ok(())
}
