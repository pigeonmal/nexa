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
    pub cases: Vec<Variant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NamedTypeKind {
    Struct,
    Enum,
    Error,
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
    pub has_content_slot: bool,
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
    pub default: Option<Literal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

/// Returns the generated native callback property name for an IDL event.
/// Both contract generation and compiler lowering use this mapping.
pub fn event_callback_property(event_name: &str) -> String {
    let mut property = String::from("on");
    let mut uppercase = true;
    for character in event_name.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase {
                property.extend(character.to_uppercase());
                uppercase = false;
            } else {
                property.push(character);
            }
        } else {
            uppercase = true;
        }
    }
    property
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
            match self.peek_identifier() {
                Some("struct") => result.types.push(self.parse_struct_type()?),
                Some("enum") => result.types.push(self.parse_enum_type()?),
                Some("error") => result.types.push(self.parse_error_type()?),
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
        let cases = self.parse_cases(false)?;
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
            self.parse_cases(true)?
        } else {
            self.consume(TokenKind::Semicolon);
            Vec::new()
        };
        if cases.is_empty() {
            return self.error("error declarations must contain at least one case");
        }
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
            if !self.consume(TokenKind::Comma)
                && !self.consume(TokenKind::Semicolon)
                && !self.peek_kind(TokenKind::RightBrace)
                && !self.peek_is_identifier()
            {
                return self.error("expected `,`, `;`, or `}` after field");
            }
            fields.push(Field { name, ty, default });
        }
        Ok(fields)
    }

    fn parse_cases(&mut self, allow_payloads: bool) -> Result<Vec<Variant>, String> {
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut cases = Vec::new();
        while !self.consume(TokenKind::RightBrace) {
            let name = self.expect_name("case name")?;
            let parameters = if allow_payloads && self.peek_kind(TokenKind::LeftParen) {
                self.parse_parameters()?
            } else {
                Vec::new()
            };
            cases.push(Variant { name, parameters });
            if !self.consume(TokenKind::Comma)
                && !self.consume(TokenKind::Semicolon)
                && !self.peek_kind(TokenKind::RightBrace)
                && !self.peek_is_identifier()
            {
                return self.error("expected `,`, `;`, or `}` after case");
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
        let kind = match self.peek_identifier() {
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
        let mut has_content_slot = false;
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close native declaration");
            }
            match self.peek_identifier() {
                Some("init") => constructors.push(self.parse_constructor()?),
                Some("readonly") | Some("property") | Some("prop") => {
                    properties.push(self.parse_property()?)
                }
                Some("event") => events.push(self.parse_event()?),
                Some("content") => {
                    if kind != InterfaceKind::NativeComponent {
                        return self.error("content slots are supported only by native components");
                    }
                    if has_content_slot {
                        return self.error("native components can declare only one `content` slot");
                    }
                    self.cursor += 1;
                    self.consume(TokenKind::Semicolon);
                    has_content_slot = true;
                }
                _ => methods.push(self.parse_method()?),
            }
        }
        Ok(Interface {
            name,
            kind,
            has_content_slot,
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
        let default = if self.consume(TokenKind::Equal) {
            Some(self.parse_literal()?)
        } else {
            None
        };
        self.consume(TokenKind::Semicolon);
        Ok(Property {
            name,
            ty,
            mutable,
            default,
        })
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
                if !cases.insert(case.name.as_str()) {
                    return Err(format!(
                        "duplicate case `{}` in type `{}`",
                        case.name, ty.name
                    ));
                }
            }
            if ty.kind == NamedTypeKind::Enum
                && ty.cases.iter().any(|case| !case.parameters.is_empty())
            {
                return Err(format!("enum `{}` cases cannot have payloads", ty.name));
            }
            if ty.kind == NamedTypeKind::Error && ty.cases.is_empty() {
                return Err(format!(
                    "error `{}` must declare at least one case",
                    ty.name
                ));
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
        }
        for ty in &idl.types {
            for field in &ty.fields {
                validate_type_ref(&field.ty, &field.name, &types, &interfaces, false)?;
                if let Some(default) = &field.default
                    && !literal_matches_type(default, &field.ty)
                {
                    return Err(format!(
                        "default for field `{}.{}` does not match `{}`",
                        ty.name, field.name, field.ty.name
                    ));
                }
            }
            for case in &ty.cases {
                validate_parameters(&case.parameters, &case.name, &types, &interfaces)?;
            }
        }
        validate_recursive_value_layouts(&idl.types)?;

        let error_types = idl
            .types
            .iter()
            .filter(|ty| ty.kind == NamedTypeKind::Error)
            .map(|ty| ty.name.as_str())
            .collect::<std::collections::HashSet<_>>();

        for interface in &idl.interfaces {
            if !interface.events.is_empty()
                && !matches!(
                    interface.kind,
                    InterfaceKind::NativeClass | InterfaceKind::NativeComponent
                )
            {
                return Err(format!(
                    "events are supported only by native classes and native components; `{}` is a stateless interface",
                    interface.name
                ));
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
                if let Some(default) = &property.default {
                    if interface.kind != InterfaceKind::NativeComponent {
                        return Err(format!(
                            "default values are supported only for native component properties; `{}.{}` is not a component property",
                            interface.name, property.name
                        ));
                    }
                    if !literal_matches_type(default, &property.ty) {
                        return Err(format!(
                            "default for property `{}.{}` does not match `{}`",
                            interface.name, property.name, property.ty.name
                        ));
                    }
                }
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
                if method.return_type.name == "Result" {
                    if method.throws.is_some() {
                        return Err(format!(
                            "method `{}` cannot combine `Result` with `throws`; declare one error type",
                            method.name
                        ));
                    }
                    let failure = &method.return_type.arguments[1];
                    if failure.optional || !error_types.contains(failure.name.as_str()) {
                        return Err(format!(
                            "failure type `{}` in method `{}` must be a declared error type",
                            failure.name, method.name
                        ));
                    }
                }
                for parameter in &method.parameters {
                    validate_type_ref(&parameter.ty, &parameter.name, &types, &interfaces, false)?;
                }
                if let Some(error) = &method.throws {
                    validate_type_ref(error, &method.name, &types, &interfaces, false)?;
                    if error.optional || !error_types.contains(error.name.as_str()) {
                        return Err(format!(
                            "throws type `{}` in method `{}` must be a declared error type",
                            error.name, method.name
                        ));
                    }
                }
            }
        }
        let mut options = std::collections::HashSet::new();
        for option in &idl.config {
            if !options.insert(option.name.as_str()) {
                return Err(format!("duplicate config option `{}`", option.name));
            }
            validate_config_type(&option.ty, &option.name)?;
            if let Some(default) = &option.default
                && !literal_matches_type(default, &option.ty)
            {
                return Err(format!(
                    "default for config option `{}` does not match `{}`",
                    option.name, option.ty.name
                ));
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

fn validate_recursive_value_layouts(types: &[NamedType]) -> Result<(), String> {
    let value_types = types
        .iter()
        .filter(|ty| matches!(ty.kind, NamedTypeKind::Struct | NamedTypeKind::Error))
        .map(|ty| ty.name.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut graph = std::collections::BTreeMap::<String, Vec<String>>::new();
    for ty in types
        .iter()
        .filter(|ty| matches!(ty.kind, NamedTypeKind::Struct | NamedTypeKind::Error))
    {
        let mut dependencies = Vec::new();
        for field in &ty.fields {
            collect_inline_value_refs(&field.ty, &value_types, &mut dependencies);
        }
        for case in &ty.cases {
            for parameter in &case.parameters {
                collect_inline_value_refs(&parameter.ty, &value_types, &mut dependencies);
            }
        }
        graph.insert(ty.name.clone(), dependencies);
    }

    let mut visiting = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for name in graph.keys() {
        visit_value_layout(name, &graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn collect_inline_value_refs(
    ty: &TypeRef,
    value_types: &std::collections::BTreeSet<String>,
    output: &mut Vec<String>,
) {
    if value_types.contains(&ty.name) {
        output.push(ty.name.clone());
        return;
    }
    if matches!(ty.name.as_str(), "Pair" | "Triple") {
        for argument in &ty.arguments {
            collect_inline_value_refs(argument, value_types, output);
        }
    }
}

fn visit_value_layout(
    name: &str,
    graph: &std::collections::BTreeMap<String, Vec<String>>,
    visiting: &mut Vec<String>,
    visited: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    if let Some(cycle_start) = visiting.iter().position(|value| value == name) {
        let mut cycle = visiting[cycle_start..].to_vec();
        cycle.push(name.to_owned());
        return Err(format!(
            "recursive plugin value layout is unsupported: {}",
            cycle.join(" -> ")
        ));
    }
    if !visited.insert(name.to_owned()) {
        return Ok(());
    }
    visiting.push(name.to_owned());
    if let Some(dependencies) = graph.get(name) {
        for dependency in dependencies {
            visit_value_layout(dependency, graph, visiting, visited)?;
        }
    }
    visiting.pop();
    Ok(())
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
    use super::{InterfaceKind, Literal, NamedTypeKind, event_callback_property, parse};

    #[test]
    fn event_callback_property_matches_generated_contract_naming() {
        assert_eq!(event_callback_property("ended"), "onEnded");
        assert_eq!(
            event_callback_property("progress_changed"),
            "onProgressChanged"
        );
    }

    #[test]
    fn parses_structs_services_classes_and_components() {
        let idl = parse(
            r#"
            struct PlayerOptions {
                quality: Float64,
                saveToGallery: Bool = false
            }
            enum PlayerState { idle, ready, ended }
            error PlayerError { invalidUrl, decodingFailed(message: String) }
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
                content
                prop player: VideoPlayer
                prop controls: Bool = true
                event tapped()
            }
            "#,
        )
        .expect("native IDL should parse");

        assert_eq!(idl.types[0].kind, NamedTypeKind::Struct);
        assert_eq!(idl.types[0].fields.len(), 2);
        assert_eq!(idl.types[2].kind, NamedTypeKind::Error);
        assert_eq!(idl.types[2].cases[1].name, "decodingFailed");
        assert_eq!(idl.types[2].cases[1].parameters[0].name, "message");
        assert_eq!(idl.types[2].cases[1].parameters[0].ty.name, "String");
        assert_eq!(idl.interfaces[0].kind, InterfaceKind::Service);
        assert_eq!(idl.interfaces[1].kind, InterfaceKind::NativeClass);
        assert_eq!(idl.interfaces[1].constructors.len(), 1);
        assert_eq!(idl.interfaces[1].properties.len(), 2);
        assert_eq!(idl.interfaces[1].events.len(), 1);
        assert_eq!(
            idl.interfaces[1].methods[0].throws.as_ref().unwrap().name,
            "PlayerError"
        );
        assert_eq!(idl.interfaces[2].kind, InterfaceKind::NativeComponent);
        assert!(idl.interfaces[2].has_content_slot);
        assert_eq!(
            idl.interfaces[2].properties[1].default,
            Some(Literal::Bool(true))
        );
    }

    #[test]
    fn throws_and_result_failures_require_declared_error_types() {
        let invalid_throws =
            parse("struct Failure { code: Int32 } service Api { async fn load() throws Failure }")
                .expect_err("throws must refer to an error declaration");
        assert!(invalid_throws.contains("must be a declared error type"));

        let invalid_result = parse(
            "struct Failure { code: Int32 } service Api { async fn load() -> Result<String, Failure> }",
        )
        .expect_err("Result failure types must be error declarations");
        assert!(invalid_result.contains("must be a declared error type"));

        let conflicting = parse(
            "error Failure { failed } service Api { async fn load() -> Result<String, Failure> throws Failure }",
        )
        .expect_err("Result and throws must not describe the same failure twice");
        assert!(conflicting.contains("cannot combine `Result` with `throws`"));
    }

    #[test]
    fn validates_error_payload_types_and_recursive_layouts() {
        let unknown = parse("error Failure { failed(reason: MissingType) }")
            .expect_err("error payloads must use declared types");
        assert!(unknown.contains("references undeclared plugin type `MissingType`"));

        let recursive = parse("error Failure { causedBy(other: Failure) }")
            .expect_err("inline recursive error payloads must be rejected");
        assert!(recursive.contains("recursive plugin value layout"));

        parse("error Failure { causedBy(other: Array<Failure>) }")
            .expect("collection indirection permits recursive error payloads");
    }

    #[test]
    fn rejects_layout_less_opaque_types() {
        let error = parse("type PlayerHandle")
            .expect_err("plugin values must describe their boundary layout or be native classes");
        assert!(error.contains("expected `struct`, `enum`, `error`"));
    }

    #[test]
    fn validates_struct_field_types_and_rejects_inline_recursion() {
        let unknown = parse("struct Player { options: MissingOptions }")
            .expect_err("unknown field types must be rejected");
        assert!(unknown.contains("references undeclared plugin type `MissingOptions`"));

        let recursive = parse("struct Node { parent: Node? }")
            .expect_err("optional fields still have inline value layout");
        assert!(recursive.contains("recursive plugin value layout"));

        parse("struct Node { children: Array<Node> }")
            .expect("collection indirection permits recursive tree models");
    }

    #[test]
    fn validates_struct_field_defaults() {
        let error = parse("struct PlayerOptions { autoplay: Bool = 1 }")
            .expect_err("field defaults must match the declared type");
        assert!(error.contains("default for field `PlayerOptions.autoplay`"));
    }

    #[test]
    fn validates_native_component_property_defaults() {
        parse("native component VideoView { prop controls: Bool = true }")
            .expect("native component defaults should be valid when their types match");

        let mismatch = parse("native component VideoView { prop controls: Bool = 1 }")
            .expect_err("native component defaults must match their property type");
        assert!(mismatch.contains("default for property `VideoView.controls`"));

        let wrong_kind = parse("native class Player { property enabled: Bool = true }")
            .expect_err("only native component properties may declare defaults");
        assert!(wrong_kind.contains("supported only for native component properties"));
    }

    #[test]
    fn content_slots_are_unique_and_component_only() {
        parse("native component Container { content }")
            .expect("a native component can declare one content slot");

        let duplicate = parse("native component Container { content content }")
            .expect_err("a native component can declare only one content slot");
        assert!(duplicate.contains("only one `content` slot"));

        let wrong_kind = parse("service Container { content }")
            .expect_err("stateless services cannot accept visual content");
        assert!(wrong_kind.contains("only by native components"));
    }

    #[test]
    fn validates_types_used_by_events_and_native_component_properties() {
        let event_error = parse("native class Player { event changed(state: MissingState) }")
            .expect_err("event payloads must use declared boundary types");
        assert!(event_error.contains("undeclared plugin type `MissingState`"));

        let component_error = parse("native component PlayerView { prop player: MissingPlayer }")
            .expect_err("component properties must use declared boundary types");
        assert!(component_error.contains("undeclared plugin type `MissingPlayer`"));
    }

    #[test]
    fn rejects_events_on_stateless_services() {
        let error = parse("service Clipboard { event copied(text: String) }")
            .expect_err("events on stateless services would require a global event bus");
        assert!(error.contains("events are supported only by native classes"));
    }
}
