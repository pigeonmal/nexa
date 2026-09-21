use std::collections::HashMap;

use nexa_diagnostics::{CompileError, Span};
use nexa_ir::ColorValue;
use nexa_syntax::ast;

use super::styles::{number_value, parse_color_literal};

#[derive(Default)]
pub(super) struct ThemeSymbols {
    tokens: HashMap<String, ResolvedThemeToken>,
}

enum ResolvedThemeToken {
    Color(ColorValue),
    Metric {
        kind: ast::ThemeTokenKind,
        value: f32,
    },
}

impl ThemeSymbols {
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
    }
}
