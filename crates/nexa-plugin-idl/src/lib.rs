//! Small, dependency-free parser for the public plugin interface description.
//!
//! The plugin IDL is deliberately separate from the application language. It
//! describes typed native boundaries and compile-time plugin options without
//! adding plugin concepts to the core `.nx` parser or runtime.

use std::{fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginIdl {
    pub types: Vec<NamedType>,
    pub interfaces: Vec<Interface>,
    pub config: Vec<ConfigOption>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedType {
    pub name: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub methods: Vec<Method>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRef {
    pub name: String,
    pub arguments: Vec<TypeRef>,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigOption {
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<Literal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    String(String),
    Number(String),
    Bool(bool),
    Null,
}

impl TypeRef {
    fn named(name: String) -> Self {
        Self {
            name,
            arguments: Vec::new(),
            optional: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Location {
    line: usize,
    column: usize,
}

impl Location {
    fn describe(self) -> String {
        format!("{}:{}", self.line, self.column)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    Colon,
    Comma,
    Semicolon,
    Arrow,
    Less,
    Greater,
    Question,
    Equal,
    Minus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    location: Location,
}

pub fn parse_file(path: &Path) -> Result<PluginIdl, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn parse(source: &str) -> Result<PluginIdl, String> {
    let tokens = lex(source)?;
    Parser { tokens, cursor: 0 }.parse()
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
        if character == '/' && characters.get(cursor + 1) == Some(&'/') {
            cursor += 2;
            column += 2;
            while cursor < characters.len() && characters[cursor] != '\n' {
                cursor += 1;
                column += 1;
            }
            continue;
        }
        let location = Location { line, column };
        let kind = match character {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '<' => TokenKind::Less,
            '>' => TokenKind::Greater,
            '?' => TokenKind::Question,
            '=' => TokenKind::Equal,
            '-' if characters.get(cursor + 1) != Some(&'>') => TokenKind::Minus,
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
                tokens.push(Token {
                    kind: TokenKind::Number(characters[start..cursor].iter().collect()),
                    location,
                });
                continue;
            }
            '-' if characters.get(cursor + 1) == Some(&'>') => {
                cursor += 1;
                column += 1;
                TokenKind::Arrow
            }
            character if character.is_ascii_alphabetic() || character == '_' => {
                let start = cursor;
                while characters
                    .get(cursor)
                    .is_some_and(|value| value.is_ascii_alphanumeric() || *value == '_')
                {
                    cursor += 1;
                    column += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Identifier(characters[start..cursor].iter().collect()),
                    location,
                });
                continue;
            }
            _ => {
                return Err(format!(
                    "{}: unexpected character `{character}`",
                    location.describe()
                ));
            }
        };
        tokens.push(Token { kind, location });
        cursor += 1;
        column += 1;
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
                return Ok(value);
            }
            '\\' => {
                *cursor += 1;
                *column += 1;
                let escaped = characters
                    .get(*cursor)
                    .copied()
                    .ok_or_else(|| "unterminated string literal".to_owned())?;
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
            '\n' => {
                return Err(format!(
                    "{}:{}: unterminated string literal",
                    *line, *column
                ));
            }
            other => {
                value.push(other);
                *cursor += 1;
                *column += 1;
            }
        }
    }
    Err(format!(
        "{}:{}: unterminated string literal",
        *line, *column
    ))
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn parse(mut self) -> Result<PluginIdl, String> {
        let mut result = PluginIdl {
            types: Vec::new(),
            interfaces: Vec::new(),
            config: Vec::new(),
        };
        let mut has_config = false;
        while !self.at_end() {
            match self.peek_identifier().as_deref() {
                Some("type") => result.types.push(self.parse_named_type()?),
                Some("interface") => result.interfaces.push(self.parse_interface()?),
                Some("config") => {
                    if has_config {
                        return self.error("plugin IDL can declare only one `config` block");
                    }
                    has_config = true;
                    result.config = self.parse_config()?;
                }
                _ => return self.error("expected `type`, `interface`, or `config`"),
            }
        }
        self.validate_names(&result)?;
        Ok(result)
    }

    fn parse_named_type(&mut self) -> Result<NamedType, String> {
        self.expect_identifier("type")?;
        let name = self.expect_name("type name")?;
        let is_error = if self.consume(TokenKind::Colon) {
            self.expect_identifier("Error")?;
            true
        } else {
            false
        };
        self.consume(TokenKind::Semicolon);
        Ok(NamedType { name, is_error })
    }

    fn parse_interface(&mut self) -> Result<Interface, String> {
        self.expect_identifier("interface")?;
        let name = self.expect_name("interface name")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut methods = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close interface");
            }
            methods.push(self.parse_method()?);
        }
        Ok(Interface { name, methods })
    }

    fn parse_config(&mut self) -> Result<Vec<ConfigOption>, String> {
        self.expect_identifier("config")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut options = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close config");
            }
            let name = self.expect_name("config option name")?;
            self.expect(TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            let default = if self.consume(TokenKind::Equal) {
                Some(self.parse_literal()?)
            } else {
                None
            };
            let separated = self.consume(TokenKind::Comma) || self.consume(TokenKind::Semicolon);
            if !separated && !self.peek_kind(TokenKind::RightBrace) && !self.peek_is_identifier() {
                return self.error("expected `,` or `}` after config option");
            }
            options.push(ConfigOption { name, ty, default });
        }
        self.consume(TokenKind::Semicolon);
        Ok(options)
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        let token = self.tokens.get(self.cursor).cloned();
        match token.map(|token| token.kind) {
            Some(TokenKind::String(value)) => {
                self.cursor += 1;
                Ok(Literal::String(value))
            }
            Some(TokenKind::Number(value)) => {
                self.cursor += 1;
                Ok(Literal::Number(value))
            }
            Some(TokenKind::Minus) => {
                self.cursor += 1;
                let Some(Token {
                    kind: TokenKind::Number(value),
                    ..
                }) = self.tokens.get(self.cursor).cloned()
                else {
                    return self.error("expected a number after `-`");
                };
                self.cursor += 1;
                Ok(Literal::Number(format!("-{value}")))
            }
            Some(TokenKind::Identifier(value)) if value == "true" => {
                self.cursor += 1;
                Ok(Literal::Bool(true))
            }
            Some(TokenKind::Identifier(value)) if value == "false" => {
                self.cursor += 1;
                Ok(Literal::Bool(false))
            }
            Some(TokenKind::Identifier(value)) if value == "null" => {
                self.cursor += 1;
                Ok(Literal::Null)
            }
            _ => self.error("expected a string, number, Boolean, or null default"),
        }
    }

    fn parse_method(&mut self) -> Result<Method, String> {
        let is_async = self.consume_identifier("async");
        self.expect_identifier("fn")?;
        let name = self.expect_name("method name")?;
        self.expect(TokenKind::LeftParen, "`(`")?;
        let mut parameters = Vec::new();
        if !self.consume(TokenKind::RightParen) {
            loop {
                let parameter_name = self.expect_name("parameter name")?;
                self.expect(TokenKind::Colon, "`:`")?;
                parameters.push(Parameter {
                    name: parameter_name,
                    ty: self.parse_type()?,
                });
                if self.consume(TokenKind::RightParen) {
                    break;
                }
                self.expect(TokenKind::Comma, "`,` or `)`")?;
            }
        }
        self.expect(TokenKind::Arrow, "`->`")?;
        let return_type = self.parse_type()?;
        self.consume(TokenKind::Semicolon);
        if return_type.name == "Result" && !is_async {
            return self.error("`Result<Success, Failure>` methods must be async");
        }
        Ok(Method {
            name,
            is_async,
            parameters,
            return_type,
        })
    }

    fn parse_type(&mut self) -> Result<TypeRef, String> {
        let name = self.expect_name("type")?;
        let mut ty = TypeRef::named(name);
        if self.consume(TokenKind::Less) {
            if self.consume(TokenKind::Greater) {
                return self.error("generic type arguments cannot be empty");
            }
            loop {
                ty.arguments.push(self.parse_type()?);
                if self.consume(TokenKind::Greater) {
                    break;
                }
                self.expect(TokenKind::Comma, "`,` or `>`")?;
            }
        }
        ty.optional = self.consume(TokenKind::Question);
        Ok(ty)
    }

    fn validate_names(&self, idl: &PluginIdl) -> Result<(), String> {
        let mut types = std::collections::HashSet::new();
        for ty in &idl.types {
            if is_builtin_type_name(&ty.name) {
                return Err(format!(
                    "type `{}` uses a reserved built-in type name",
                    ty.name
                ));
            }
            if !types.insert(ty.name.as_str()) {
                return Err(format!("duplicate type `{}`", ty.name));
            }
        }
        let mut interfaces = std::collections::HashSet::new();
        for interface in &idl.interfaces {
            if is_builtin_type_name(&interface.name) || types.contains(interface.name.as_str()) {
                return Err(format!(
                    "interface `{}` conflicts with a declared type or built-in type",
                    interface.name
                ));
            }
            if !interfaces.insert(interface.name.as_str()) {
                return Err(format!("duplicate interface `{}`", interface.name));
            }
            let mut methods = std::collections::HashSet::new();
            for method in &interface.methods {
                if !methods.insert(method.name.as_str()) {
                    return Err(format!(
                        "duplicate method `{}` in interface `{}`",
                        method.name, interface.name
                    ));
                }
                let mut parameters = std::collections::HashSet::new();
                for parameter in &method.parameters {
                    if !parameters.insert(parameter.name.as_str()) {
                        return Err(format!(
                            "duplicate parameter `{}` in method `{}`",
                            parameter.name, method.name
                        ));
                    }
                }
                if method.return_type.name == "Result" && method.return_type.arguments.len() != 2 {
                    return Err(format!(
                        "method `{}` uses `Result` with {}, expected two type arguments",
                        method.name,
                        method.return_type.arguments.len()
                    ));
                }
                if method.return_type.name == "Result" && method.return_type.optional {
                    return Err(format!(
                        "method `{}` cannot make `Result` optional; make its success type optional instead",
                        method.name
                    ));
                }
                validate_type_ref(&method.return_type, &method.name, &types, true)?;
                for parameter in &method.parameters {
                    validate_type_ref(&parameter.ty, &parameter.name, &types, false)?;
                }
            }
        }
        let mut options = std::collections::HashSet::new();
        for option in &idl.config {
            if !options.insert(option.name.as_str()) {
                return Err(format!("duplicate config option `{}`", option.name));
            }
            validate_config_type(&option.ty, &option.name)?;
            if let Some(default) = &option.default {
                if !literal_matches_type(default, &option.ty) {
                    return Err(format!(
                        "default for config option `{}` does not match `{}`",
                        option.name, option.ty.name
                    ));
                }
            }
        }
        Ok(())
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn peek_identifier(&self) -> Option<&str> {
        match self.tokens.get(self.cursor).map(|token| &token.kind) {
            Some(TokenKind::Identifier(value)) => Some(value),
            _ => None,
        }
    }

    fn expect_name(&mut self, description: &str) -> Result<String, String> {
        match self.tokens.get(self.cursor).cloned() {
            Some(Token {
                kind: TokenKind::Identifier(value),
                ..
            }) => {
                self.cursor += 1;
                Ok(value)
            }
            _ => self.error(format!("expected {description}")),
        }
    }

    fn expect_identifier(&mut self, value: &str) -> Result<(), String> {
        if self.consume_identifier(value) {
            Ok(())
        } else {
            self.error(format!("expected `{value}`"))
        }
    }

    fn consume_identifier(&mut self, value: &str) -> bool {
        if self.peek_identifier() == Some(value) {
            self.cursor += 1;
            true
        } else {
            false
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

    fn peek_is_identifier(&self) -> bool {
        matches!(
            self.tokens.get(self.cursor).map(|token| &token.kind),
            Some(TokenKind::Identifier(_))
        )
    }

    fn error<T, R>(&self, message: T) -> Result<R, String>
    where
        T: Into<String>,
    {
        let location = self
            .tokens
            .get(self.cursor)
            .map(|token| token.location.describe())
            .unwrap_or_else(|| "end of file".to_owned());
        Err(format!("{location}: {}", message.into()))
    }
}

fn validate_type_ref(
    ty: &TypeRef,
    context: &str,
    declared_types: &std::collections::HashSet<&str>,
    allow_result: bool,
) -> Result<(), String> {
    let expected = match ty.name.as_str() {
        "Array" | "Set" => Some(1),
        "Map" | "Pair" | "Result" => Some(2),
        "Triple" => Some(3),
        _ => None,
    };
    if let Some(expected) = expected {
        if ty.arguments.len() != expected {
            return Err(format!(
                "{context} uses `{}` with {}, expected {} type argument(s)",
                ty.name,
                ty.arguments.len(),
                expected
            ));
        }
        if ty.name == "Result" && !allow_result {
            return Err(format!(
                "{context} uses `Result` outside a method return type"
            ));
        }
    } else if is_builtin_type_name(&ty.name) {
        if !ty.arguments.is_empty() {
            return Err(format!(
                "{context} uses built-in `{}` with unexpected type arguments",
                ty.name
            ));
        }
    } else if !declared_types.contains(ty.name.as_str()) {
        return Err(format!(
            "{context} references undeclared plugin type `{}`",
            ty.name
        ));
    } else if !ty.arguments.is_empty() {
        return Err(format!(
            "{context} uses declared type `{}` with unsupported type arguments",
            ty.name
        ));
    }
    for argument in &ty.arguments {
        validate_type_ref(argument, context, declared_types, false)?;
    }
    Ok(())
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Void"
            | "String"
            | "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
            | "Bytes"
            | "Array"
            | "Set"
            | "Map"
            | "Pair"
            | "Triple"
            | "Result"
    )
}

fn validate_config_type(ty: &TypeRef, context: &str) -> Result<(), String> {
    let supported = matches!(
        ty.name.as_str(),
        "String"
            | "Bool"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
    );
    if !supported || !ty.arguments.is_empty() {
        return Err(format!(
            "config option `{context}` must use a scalar String, Bool, integer, or floating-point type"
        ));
    }
    Ok(())
}

fn literal_matches_type(literal: &Literal, ty: &TypeRef) -> bool {
    match (literal, ty.name.as_str()) {
        (Literal::String(_), "String") => true,
        (Literal::Bool(_), "Bool") => true,
        (Literal::Number(value), name) if is_integer_type(name) => integer_value_fits(value, name),
        (Literal::Number(value), "Float32") => value.parse::<f32>().is_ok(),
        (Literal::Number(value), "Float64") => value.parse::<f64>().is_ok(),
        (Literal::Null, _) => ty.optional,
        _ => false,
    }
}

fn is_integer_type(name: &str) -> bool {
    matches!(
        name,
        "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64"
    )
}

fn integer_value_fits(value: &str, name: &str) -> bool {
    if value.contains('.') {
        return false;
    }
    match name {
        "Int8" => value.parse::<i8>().is_ok(),
        "Int16" => value.parse::<i16>().is_ok(),
        "Int32" => value.parse::<i32>().is_ok(),
        "Int64" => value.parse::<i64>().is_ok(),
        "UInt8" => value.parse::<u8>().is_ok(),
        "UInt16" => value.parse::<u16>().is_ok(),
        "UInt32" => value.parse::<u32>().is_ok(),
        "UInt64" => value.parse::<u64>().is_ok(),
        _ => false,
    }
}
