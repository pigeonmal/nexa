//! Small, dependency-free parser for the public plugin interface description.
//!
//! The plugin IDL is deliberately separate from the application language. It
//! describes typed native boundaries and compile-time plugin options without
//! adding plugin concepts to the core `.nx` parser or runtime.

use std::{fs, path::Path};

pub mod manifest;

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
    pub kind: NamedTypeKind,
    pub fields: Vec<Field>,
    pub cases: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedTypeKind {
    Struct,
    Enum,
    Error,
    Opaque,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeRef,
    pub default: Option<Literal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub kind: InterfaceKind,
    pub constructors: Vec<Constructor>,
    pub methods: Vec<Method>,
    pub properties: Vec<Property>,
    pub events: Vec<Event>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceKind {
    Interface,
    Service,
    NativeClass,
    NativeComponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constructor {
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub ty: TypeRef,
    pub mutable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: TypeRef,
    pub throws: Option<TypeRef>,
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
                Some("struct") => result.types.push(self.parse_struct_type()?),
                Some("enum") => result.types.push(self.parse_enum_type()?),
                Some("error") => result.types.push(self.parse_error_type()?),
                Some("type") => result.types.push(self.parse_opaque_type()?),
                Some("interface") => result.interfaces.push(self.parse_interface()?),
                Some("service") => result.interfaces.push(self.parse_service()?),
                Some("native") => result.interfaces.push(self.parse_native_declaration()?),
                Some("config") => {
                    if has_config {
                        return self.error("plugin IDL can declare only one `config` block");
                    }
                    has_config = true;
                    result.config = self.parse_config()?;
                }
                _ => return self.error(
                    "expected `struct`, `enum`, `error`, `interface`, `service`, `native`, or `config`",
                ),
            }
        }
        self.validate_names(&result)?;
        Ok(result)
    }

    fn parse_opaque_type(&mut self) -> Result<NamedType, String> {
        self.expect_identifier("type")?;
        let name = self.expect_name("type name")?;
        self.consume(TokenKind::Semicolon);
        Ok(NamedType {
            name,
            is_error: false,
            kind: NamedTypeKind::Opaque,
            fields: Vec::new(),
            cases: Vec::new(),
        })
    }

    fn parse_struct_type(&mut self) -> Result<NamedType, String> {
        self.expect_identifier("struct")?;
        let name = self.expect_name("struct name")?;
        let fields = self.parse_fields()?;
        Ok(NamedType {
            name,
            is_error: false,
            kind: NamedTypeKind::Struct,
            fields,
            cases: Vec::new(),
        })
    }

    fn parse_enum_type(&mut self) -> Result<NamedType, String> {
        self.expect_identifier("enum")?;
        let name = self.expect_name("enum name")?;
        let cases = self.parse_cases()?;
        Ok(NamedType {
            name,
            is_error: false,
            kind: NamedTypeKind::Enum,
            fields: Vec::new(),
            cases,
        })
    }

    fn parse_error_type(&mut self) -> Result<NamedType, String> {
        self.expect_identifier("error")?;
        let name = self.expect_name("error name")?;
        let cases = if self.peek_kind(TokenKind::LeftBrace) {
            self.parse_cases()?
        } else {
            self.consume(TokenKind::Semicolon);
            Vec::new()
        };
        Ok(NamedType {
            name,
            is_error: true,
            kind: NamedTypeKind::Error,
            fields: Vec::new(),
            cases,
        })
    }

    fn parse_fields(&mut self) -> Result<Vec<Field>, String> {
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            let name = self.expect_name("field name")?;
            self.expect(TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            let default = self
                .consume(TokenKind::Equal)
                .then(|| self.parse_literal())
                .transpose()?;
            if !self.consume(TokenKind::Comma) && !self.consume(TokenKind::Semicolon) {
                if !self.peek_kind(TokenKind::RightBrace) && !self.peek_is_identifier() {
                    return self.error("expected `,`, `;`, or `}` after field");
                }
            }
            fields.push(Field { name, ty, default });
        }
        Ok(fields)
    }

    fn parse_cases(&mut self) -> Result<Vec<String>, String> {
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut cases = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            cases.push(self.expect_name("case name")?);
            if !self.consume(TokenKind::Comma) && !self.consume(TokenKind::Semicolon) {
                if !self.peek_kind(TokenKind::RightBrace) && !self.peek_is_identifier() {
                    return self.error("expected `,`, `;`, or `}` after case");
                }
            }
        }
        Ok(cases)
    }

    fn parse_interface(&mut self) -> Result<Interface, String> {
        self.expect_identifier("interface")?;
        let name = self.expect_name("interface name")?;
        self.parse_interface_body(name, InterfaceKind::Interface)
    }

    fn parse_service(&mut self) -> Result<Interface, String> {
        self.expect_identifier("service")?;
        let name = self.expect_name("service name")?;
        self.parse_interface_body(name, InterfaceKind::Service)
    }

    fn parse_native_declaration(&mut self) -> Result<Interface, String> {
        self.expect_identifier("native")?;
        let kind = match self.peek_identifier().as_deref() {
            Some("class") => {
                self.cursor += 1;
                InterfaceKind::NativeClass
            }
            Some("component") => {
                self.cursor += 1;
                InterfaceKind::NativeComponent
            }
            _ => return self.error("expected `class` or `component` after `native`"),
        };
        let name = self.expect_name("native declaration name")?;
        self.parse_interface_body(name, kind)
    }

    fn parse_interface_body(
        &mut self,
        name: String,
        kind: InterfaceKind,
    ) -> Result<Interface, String> {
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut constructors = Vec::new();
        let mut methods = Vec::new();
        let mut properties = Vec::new();
        let mut events = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close native declaration");
            }
            match self.peek_identifier().as_deref() {
                Some("init") => constructors.push(self.parse_constructor()?),
                Some("readonly") | Some("property") | Some("prop") => {
                    properties.push(self.parse_property()?)
                }
                Some("event") => events.push(self.parse_event()?),
                _ => methods.push(self.parse_method()?),
            }
        }
        Ok(Interface {
            name,
            kind,
            constructors,
            methods,
            properties,
            events,
        })
    }

    fn parse_constructor(&mut self) -> Result<Constructor, String> {
        self.expect_identifier("init")?;
        let parameters = self.parse_parameters()?;
        self.consume(TokenKind::Semicolon);
        Ok(Constructor { parameters })
    }

    fn parse_property(&mut self) -> Result<Property, String> {
        let mutable = if self.consume_identifier("readonly") {
            self.expect_identifier("property")?;
            false
        } else {
            if !self.consume_identifier("property") && !self.consume_identifier("prop") {
                return self.error("expected `property` or `prop`");
            }
            true
        };
        let name = self.expect_name("property name")?;
        self.expect(TokenKind::Colon, "`:`")?;
        let ty = self.parse_type()?;
        self.consume(TokenKind::Semicolon);
        Ok(Property { name, ty, mutable })
    }

    fn parse_event(&mut self) -> Result<Event, String> {
        self.expect_identifier("event")?;
        let name = self.expect_name("event name")?;
        let parameters = self.parse_parameters()?;
        self.consume(TokenKind::Semicolon);
        Ok(Event { name, parameters })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        self.expect(TokenKind::LeftParen, "`(`")?;
        let mut parameters = Vec::new();
        if !self.consume(TokenKind::RightParen) {
            loop {
                let name = self.expect_name("parameter name")?;
                self.expect(TokenKind::Colon, "`:`")?;
                parameters.push(Parameter {
                    name,
                    ty: self.parse_type()?,
                });
                if self.consume(TokenKind::RightParen) {
                    break;
                }
                self.expect(TokenKind::Comma, "`,` or `)`")?;
            }
        }
        Ok(parameters)
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
        let parameters = self.parse_parameters()?;
        let return_type = if self.consume(TokenKind::Arrow) {
            self.parse_type()?
        } else {
            TypeRef::named("Void".to_owned())
        };
        let throws = if self.consume_identifier("throws") {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.consume(TokenKind::Semicolon);
        if (return_type.name == "Result" || throws.is_some()) && !is_async {
            return self.error("`Result<Success, Failure>` methods must be async");
        }
        Ok(Method {
            name,
            is_async,
            parameters,
            return_type,
            throws,
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
            let mut names = std::collections::HashSet::new();
            for field in &ty.fields {
                if !names.insert(field.name.as_str()) {
                    return Err(format!(
                        "duplicate field `{}` in type `{}`",
                        field.name, ty.name
                    ));
                }
            }
            let mut cases = std::collections::HashSet::new();
            for case in &ty.cases {
                if !cases.insert(case.as_str()) {
                    return Err(format!("duplicate case `{case}` in type `{}`", ty.name));
                }
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
            let mut properties = std::collections::HashSet::new();
            for property in &interface.properties {
                if !properties.insert(property.name.as_str()) {
                    return Err(format!(
                        "duplicate property `{}` in interface `{}`",
                        property.name, interface.name
                    ));
                }
                validate_type_ref(&property.ty, &property.name, &types, &interfaces, false)?;
            }
            let mut events = std::collections::HashSet::new();
            for event in &interface.events {
                if !events.insert(event.name.as_str()) {
                    return Err(format!(
                        "duplicate event `{}` in interface `{}`",
                        event.name, interface.name
                    ));
                }
                validate_parameters(&event.parameters, &event.name, &types, &interfaces)?;
            }
            for constructor in &interface.constructors {
                validate_parameters(
                    &constructor.parameters,
                    &interface.name,
                    &types,
                    &interfaces,
                )?;
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
                validate_type_ref(&method.return_type, &method.name, &types, &interfaces, true)?;
                for parameter in &method.parameters {
                    validate_type_ref(&parameter.ty, &parameter.name, &types, &interfaces, false)?;
                }
                if let Some(error) = &method.throws {
                    validate_type_ref(error, &method.name, &types, &interfaces, false)?;
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
    declared_interfaces: &std::collections::HashSet<&str>,
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
    } else if !declared_types.contains(ty.name.as_str())
        && !declared_interfaces.contains(ty.name.as_str())
    {
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
        validate_type_ref(
            argument,
            context,
            declared_types,
            declared_interfaces,
            false,
        )?;
    }
    Ok(())
}

fn validate_parameters(
    parameters: &[Parameter],
    context: &str,
    declared_types: &std::collections::HashSet<&str>,
    declared_interfaces: &std::collections::HashSet<&str>,
) -> Result<(), String> {
    let mut names = std::collections::HashSet::new();
    for parameter in parameters {
        if !names.insert(parameter.name.as_str()) {
            return Err(format!(
                "duplicate parameter `{}` in `{context}`",
                parameter.name
            ));
        }
        validate_type_ref(
            &parameter.ty,
            &parameter.name,
            declared_types,
            declared_interfaces,
            false,
        )?;
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

#[cfg(test)]
mod tests {
    use super::{InterfaceKind, NamedTypeKind, parse};

    #[test]
    fn parses_structs_services_classes_and_components() {
        let idl = parse(
            r#"
            struct PlayerOptions {
                quality: Float64,
                saveToGallery: Bool = false
            }
            enum PlayerState { idle, ready, ended }
            error PlayerError { invalidUrl, decodingFailed }
            service Clipboard {
                fn copy(text: String)
            }
            native class VideoPlayer {
                init(options: PlayerOptions)
                readonly property state: PlayerState
                property volume: Float64
                async fn prepare(url: String) throws PlayerError
                fn play()
                event ended()
            }
            native component VideoView {
                prop player: VideoPlayer
                prop controls: Bool
                event tapped()
            }
            "#,
        )
        .expect("native IDL should parse");

        assert_eq!(idl.types[0].kind, NamedTypeKind::Struct);
        assert_eq!(idl.types[0].fields.len(), 2);
        assert_eq!(idl.interfaces[0].kind, InterfaceKind::Service);
        assert_eq!(idl.interfaces[1].kind, InterfaceKind::NativeClass);
        assert_eq!(idl.interfaces[1].constructors.len(), 1);
        assert_eq!(idl.interfaces[1].properties.len(), 2);
        assert_eq!(idl.interfaces[1].events.len(), 1);
        assert_eq!(idl.interfaces[2].kind, InterfaceKind::NativeComponent);
    }
}
