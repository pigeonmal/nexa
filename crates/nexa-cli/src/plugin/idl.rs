//! Small, dependency-free parser for the public plugin interface description.
//!
//! The plugin IDL is deliberately separate from the application language. It
//! describes typed native boundaries without adding plugin concepts to the
//! core `.nx` parser or runtime.

use std::{fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginIdl {
    pub(crate) types: Vec<NamedType>,
    pub(crate) interfaces: Vec<Interface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NamedType {
    pub(crate) name: String,
    pub(crate) is_error: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Interface {
    pub(crate) name: String,
    pub(crate) methods: Vec<Method>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Method {
    pub(crate) name: String,
    pub(crate) is_async: bool,
    pub(crate) parameters: Vec<Parameter>,
    pub(crate) return_type: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parameter {
    pub(crate) name: String,
    pub(crate) ty: TypeRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypeRef {
    pub(crate) name: String,
    pub(crate) arguments: Vec<TypeRef>,
    pub(crate) optional: bool,
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    location: Location,
}

pub(crate) fn parse_file(path: &Path) -> Result<PluginIdl, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    parse(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn parse(source: &str) -> Result<PluginIdl, String> {
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

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn parse(mut self) -> Result<PluginIdl, String> {
        let mut result = PluginIdl {
            types: Vec::new(),
            interfaces: Vec::new(),
        };
        while !self.at_end() {
            match self.peek_identifier().as_deref() {
                Some("type") => result.types.push(self.parse_named_type()?),
                Some("interface") => result.interfaces.push(self.parse_interface()?),
                _ => return self.error("expected `type` or `interface`"),
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
            if !types.insert(ty.name.as_str()) {
                return Err(format!("duplicate type `{}`", ty.name));
            }
        }
        let mut interfaces = std::collections::HashSet::new();
        for interface in &idl.interfaces {
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
                validate_type_shape(&method.return_type, &method.name)?;
                for parameter in &method.parameters {
                    validate_type_shape(&parameter.ty, &parameter.name)?;
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

fn validate_type_shape(ty: &TypeRef, context: &str) -> Result<(), String> {
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
    }
    for argument in &ty.arguments {
        validate_type_shape(argument, context)?;
    }
    Ok(())
}
