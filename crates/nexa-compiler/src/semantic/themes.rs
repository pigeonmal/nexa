use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::ColorValue;
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
    Metric {
        kind: ast::ThemeTokenKind,
        value: f32,
    },
}

impl ThemeSymbols {
    pub(super) fn default_font_family(&self) -> Option<&str> {
        match self.tokens.get("Default") {
            Some(ResolvedThemeToken::FontFamily(family)) => Some(family),
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
            (_, ast::ThemeTokenValue::AdaptiveColor { .. }) => {
                return Err(CompileError::new(
                    token.span,
                    "only `color` theme tokens accept `light` and `dark` values",
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
    }
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
