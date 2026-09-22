//! Parser for the package-level `plugin.config.nx` manifest.
//!
//! The manifest is deliberately separate from the application `nexa.config.nx`
//! file. It is the package author's source of truth for identity and source
//! roots; no JSON convention or filename probing is needed after it is parsed.

use std::{fs, path::Path};

pub const MANIFEST_SCHEMA_VERSION: u16 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub schema: u16,
    pub id: String,
    pub version: String,
    pub nexa: Option<String>,
    pub native: Option<String>,
    pub ios: PlatformManifest,
    pub android: PlatformManifest,
    pub assets: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlatformManifest {
    pub min_version: Option<String>,
    pub min_sdk: Option<u32>,
    pub sources: Vec<String>,
    pub dependencies: Vec<String>,
}

impl Default for PluginManifest {
    fn default() -> Self {
        Self {
            schema: MANIFEST_SCHEMA_VERSION,
            id: String::new(),
            version: String::new(),
            nexa: None,
            native: None,
            ios: PlatformManifest::default(),
            android: PlatformManifest::default(),
            assets: Vec::new(),
        }
    }
}

pub fn parse_file(path: &Path) -> Result<PluginManifest, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn parse(source: &str) -> Result<PluginManifest, String> {
    let tokens = lex(source)?;
    Parser { tokens, cursor: 0 }.parse()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Identifier(String),
    String(String),
    Number(String),
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    let mut column = 1;
    while cursor < characters.len() {
        let character = characters[cursor];
        if character.is_whitespace() {
            if character == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            cursor += 1;
            continue;
        }
        if character == '#' || (character == '/' && characters.get(cursor + 1) == Some(&'/')) {
            if character == '/' {
                cursor += 2;
                column += 2;
            } else {
                cursor += 1;
                column += 1;
            }
            while cursor < characters.len() && characters[cursor] != '\n' {
                cursor += 1;
                column += 1;
            }
            continue;
        }
        let token_line = line;
        let token_column = column;
        let kind = match character {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '"' => TokenKind::String(read_string(
                &characters,
                &mut cursor,
                &mut line,
                &mut column,
            )?),
            c if c.is_ascii_digit() => {
                let start = cursor;
                while characters
                    .get(cursor)
                    .is_some_and(|value| value.is_ascii_digit() || *value == '.')
                {
                    cursor += 1;
                    column += 1;
                }
                TokenKind::Number(characters[start..cursor].iter().collect())
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = cursor;
                while characters.get(cursor).is_some_and(|value| {
                    value.is_ascii_alphanumeric() || *value == '_' || *value == '-'
                }) {
                    cursor += 1;
                    column += 1;
                }
                TokenKind::Identifier(characters[start..cursor].iter().collect())
            }
            _ => {
                return Err(format!(
                    "{}:{}: unexpected character `{character}`",
                    token_line, token_column
                ));
            }
        };
        let advances_cursor = !matches!(
            &kind,
            TokenKind::String(_) | TokenKind::Identifier(_) | TokenKind::Number(_)
        );
        tokens.push(Token {
            kind,
            line: token_line,
            column: token_column,
        });
        if advances_cursor {
            cursor += 1;
            column += 1;
        }
    }
    Ok(tokens)
}

fn read_string(
    characters: &[char],
    cursor: &mut usize,
    line: &mut usize,
    column: &mut usize,
) -> Result<String, String> {
    *cursor += 1;
    *column += 1;
    let mut value = String::new();
    while let Some(character) = characters.get(*cursor).copied() {
        match character {
            '"' => {
                *cursor += 1;
                *column += 1;
                return Ok(value);
            }
            '\\' => {
                *cursor += 1;
                *column += 1;
                let escaped = characters
                    .get(*cursor)
                    .copied()
                    .ok_or_else(|| format!("{}:{}: unterminated string", *line, *column))?;
                value.push(match escaped {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => {
                        return Err(format!(
                            "{}:{}: unsupported string escape `\\{other}`",
                            *line, *column
                        ));
                    }
                });
                *cursor += 1;
                *column += 1;
            }
            '\n' => return Err(format!("{}:{}: unterminated string", *line, *column)),
            other => {
                value.push(other);
                *cursor += 1;
                *column += 1;
            }
        }
    }
    Err(format!("{}:{}: unterminated string", *line, *column))
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn parse(mut self) -> Result<PluginManifest, String> {
        let mut manifest = PluginManifest::default();
        let mut seen = std::collections::HashSet::new();
        self.expect_identifier("plugin")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close plugin manifest");
            }
            let field = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected a plugin manifest field"))?;
            if !seen.insert(field.clone()) {
                return Err(self.error_value(format!(
                    "plugin manifest field `{field}` is declared more than once"
                )));
            }
            match Some(field).as_deref() {
                Some("schema") => manifest.schema = self.parse_u16_field("schema")?,
                Some("id") => manifest.id = self.parse_string_field("id")?,
                Some("version") => manifest.version = self.parse_string_field("version")?,
                Some("nexa") => manifest.nexa = Some(self.parse_string_field("nexa")?),
                Some("assets") => manifest.assets = self.parse_string_array_field("assets")?,
                Some("sources") => self.parse_sources(&mut manifest)?,
                Some("ios") => self.parse_platform(&mut manifest.ios, false)?,
                Some("android") => self.parse_platform(&mut manifest.android, true)?,
                Some(name) => return self.error(format!("unknown plugin manifest field `{name}`")),
                None => return self.error("expected a plugin manifest field"),
            }
            self.consume(TokenKind::Comma);
        }
        self.validate(&manifest)?;
        Ok(manifest)
    }

    fn parse_sources(&mut self, manifest: &mut PluginManifest) -> Result<(), String> {
        self.expect_identifier("sources")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut seen = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            let field = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected a source field"))?;
            if !seen.insert(field.clone()) {
                return Err(
                    self.error_value(format!("source field `{field}` is declared more than once"))
                );
            }
            match Some(field).as_deref() {
                Some("nexa") => manifest.nexa = Some(self.parse_string_field("nexa")?),
                Some("native") => manifest.native = Some(self.parse_string_field("native")?),
                Some(name) => return self.error(format!("unknown source field `{name}`")),
                None => return self.error("expected a source field"),
            }
            self.consume(TokenKind::Comma);
        }
        Ok(())
    }

    fn parse_platform(
        &mut self,
        platform: &mut PlatformManifest,
        android: bool,
    ) -> Result<(), String> {
        self.expect_identifier(if android { "android" } else { "ios" })?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut seen = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            let field = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected a platform field"))?;
            if !seen.insert(field.clone()) {
                return Err(self.error_value(format!(
                    "platform field `{field}` is declared more than once"
                )));
            }
            match Some(field).as_deref() {
                Some("minVersion") => {
                    platform.min_version = Some(self.parse_string_field("minVersion")?)
                }
                Some("minSdk") if android => {
                    platform.min_sdk = Some(self.parse_u32_field("minSdk")?)
                }
                Some("sources") => platform.sources = self.parse_string_array_field("sources")?,
                Some("dependencies") => {
                    platform.dependencies = self.parse_string_array_field("dependencies")?
                }
                Some(name) => return self.error(format!("unknown platform field `{name}`")),
                None => return self.error("expected a platform field"),
            }
            self.consume(TokenKind::Comma);
        }
        Ok(())
    }

    fn parse_string_field(&mut self, name: &str) -> Result<String, String> {
        self.expect_identifier(name)?;
        self.expect(TokenKind::Colon, "`:`")?;
        self.expect_string()
    }

    fn parse_u16_field(&mut self, name: &str) -> Result<u16, String> {
        self.expect_identifier(name)?;
        self.expect(TokenKind::Colon, "`:`")?;
        self.expect_number()?
            .parse()
            .map_err(|_| self.error_value(format!("{name} must be a 16-bit integer")))
    }

    fn parse_u32_field(&mut self, name: &str) -> Result<u32, String> {
        self.expect_identifier(name)?;
        self.expect(TokenKind::Colon, "`:`")?;
        self.expect_number()?
            .parse()
            .map_err(|_| self.error_value(format!("{name} must be a 32-bit integer")))
    }

    fn parse_string_array_field(&mut self, name: &str) -> Result<Vec<String>, String> {
        self.expect_identifier(name)?;
        self.expect(TokenKind::Colon, "`:`")?;
        self.expect(TokenKind::LeftBracket, "`[`")?;
        let mut values = Vec::new();
        while !self.consume(TokenKind::RightBracket) {
            values.push(self.expect_string()?);
            if !self.consume(TokenKind::Comma) && !self.peek_kind(TokenKind::RightBracket) {
                return self.error("expected `,` or `]`");
            }
        }
        Ok(values)
    }

    fn validate(&self, manifest: &PluginManifest) -> Result<(), String> {
        if manifest.schema != MANIFEST_SCHEMA_VERSION {
            return Err(format!(
                "unsupported plugin manifest schema {}; expected {}",
                manifest.schema, MANIFEST_SCHEMA_VERSION
            ));
        }
        if manifest.id.is_empty()
            || manifest.id.split('.').any(|part| {
                part.is_empty()
                    || !part.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_-".contains(character)
                    })
            })
        {
            return Err(
                "plugin manifest `id` must be a non-empty dot-separated identifier".to_owned(),
            );
        }
        if manifest.version.is_empty() || manifest.version.chars().any(char::is_whitespace) {
            return Err(
                "plugin manifest `version` must be non-empty and contain no whitespace".to_owned(),
            );
        }
        for path in manifest
            .nexa
            .iter()
            .chain(manifest.native.iter())
            .chain(manifest.assets.iter())
            .chain(manifest.ios.sources.iter())
            .chain(manifest.android.sources.iter())
        {
            validate_relative_path(path)?;
        }
        if manifest.nexa.is_none() && manifest.native.is_none() && manifest.assets.is_empty() {
            return Err(
                "plugin manifest must declare Nexa source, native IDL, or assets".to_owned(),
            );
        }
        Ok(())
    }

    fn expect_string(&mut self) -> Result<String, String> {
        match self.tokens.get(self.cursor).cloned() {
            Some(Token {
                kind: TokenKind::String(value),
                ..
            }) => {
                self.cursor += 1;
                Ok(value)
            }
            _ => self.error("expected a string"),
        }
    }

    fn expect_number(&mut self) -> Result<String, String> {
        match self.tokens.get(self.cursor).cloned() {
            Some(Token {
                kind: TokenKind::Number(value),
                ..
            }) => {
                self.cursor += 1;
                Ok(value)
            }
            _ => self.error("expected a number"),
        }
    }

    fn expect_identifier(&mut self, value: &str) -> Result<(), String> {
        if self.peek_identifier().as_deref() == Some(value) {
            self.cursor += 1;
            Ok(())
        } else {
            self.error(format!("expected `{value}`"))
        }
    }

    fn peek_identifier(&self) -> Option<String> {
        match self.tokens.get(self.cursor).map(|token| &token.kind) {
            Some(TokenKind::Identifier(value)) => Some(value.clone()),
            _ => None,
        }
    }

    fn expect(&mut self, kind: TokenKind, description: &str) -> Result<(), String> {
        if self.consume(kind) {
            Ok(())
        } else {
            self.error(format!("expected {description}"))
        }
    }

    fn consume(&mut self, kind: TokenKind) -> bool {
        if self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
        {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn peek_kind(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn error<T, R>(&self, message: T) -> Result<R, String>
    where
        T: Into<String>,
    {
        let location = self
            .tokens
            .get(self.cursor)
            .map(|token| format!("{}:{}", token.line, token.column))
            .unwrap_or_else(|| "end of file".to_owned());
        Err(format!("{location}: {}", message.into()))
    }

    fn error_value(&self, message: impl Into<String>) -> String {
        let location = self
            .tokens
            .get(self.cursor)
            .map(|token| format!("{}:{}", token.line, token.column))
            .unwrap_or_else(|| "end of file".to_owned());
        format!("{location}: {}", message.into())
    }
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.is_absolute()
        || path
            .components()
            .any(|component| component == std::path::Component::ParentDir)
    {
        return Err(format!(
            "plugin manifest path `{}` must stay inside the package",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_native_manifest() {
        let manifest = parse(
            r#"
            plugin {
                schema: 2
                id: "dev.nexa.video"
                version: "1.0.0"
                sources {
                    native: "native.nxid"
                }
                ios { minVersion: "17.0", sources: ["ios/Sources/**"] }
                android { minSdk: 26, sources: ["android/src/main/kotlin/**"] }
                assets: ["assets/**"]
            }
            "#,
        )
        .expect("manifest should parse");
        assert_eq!(manifest.native.as_deref(), Some("native.nxid"));
        assert_eq!(manifest.android.min_sdk, Some(26));
        assert_eq!(manifest.ios.sources, vec!["ios/Sources/**"]);
    }

    #[test]
    fn rejects_package_escape() {
        let error =
            parse(r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" assets: ["../outside"] }"#)
                .expect_err("path traversal must be rejected");
        assert!(error.contains("must stay inside"));
    }
}
