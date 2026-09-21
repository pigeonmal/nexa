use std::collections::BTreeMap;

use crate::{
    ast::*,
    lexer::{Kind, Token},
};
use nexa_diagnostics::{CompileError, Span};

pub fn parse(tokens: Vec<Token>) -> Result<App, CompileError> {
    let mut program = parse_program(tokens)?;
    if let Some(import) = program.imports.first() {
        return Err(CompileError::new(
            import.span,
            "imports require compiling an entry file with the Nexa CLI",
        ));
    }
    let Some(mut app) = program.app.take() else {
        return Err(CompileError::new(
            Span::default(),
            "source file is missing an `app` declaration",
        ));
    };
    app.components = program.components;
    Ok(app)
}

pub fn parse_program(tokens: Vec<Token>) -> Result<Program, CompileError> {
    Parser { tokens, cursor: 0 }.program()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn program(mut self) -> Result<Program, CompileError> {
        let mut imports = Vec::new();
        let mut components = Vec::new();
        let mut app = None;
        while !self.check(&Kind::Eof) {
            if self.word_is("import") {
                imports.push(self.import_decl()?);
            } else if self.word_is("component") {
                components.push(self.component_decl()?);
            } else if self.word_is("app") {
                if app.is_some() {
                    return self.error_here("a source file can only declare one `app`");
                }
                app = Some(self.app_decl()?);
            } else {
                return self.error_here("expected an `import`, `component`, or `app` declaration");
            }
        }
        Ok(Program {
            imports,
            components,
            app,
        })
    }

    fn import_decl(&mut self) -> Result<ImportDecl, CompileError> {
        let keyword = self.advance().span;
        let token = self.advance().clone();
        let Kind::String(path) = token.kind else {
            return Err(CompileError::new(
                token.span,
                "an import path must be a quoted string",
            ));
        };
        if path.is_empty() {
            return Err(CompileError::new(
                token.span,
                "an import path cannot be empty",
            ));
        }
        self.optional_semicolon();
        Ok(ImportDecl {
            path,
            span: keyword,
        })
    }

    fn component_decl(&mut self) -> Result<ComponentDecl, CompileError> {
        let keyword = self.advance().span;
        let (name, _) = self.ident()?;
        self.expect(Kind::LParen, "expected `(` after component name")?;
        let mut parameters = Vec::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            let (parameter_name, span) = self.ident()?;
            if parameters
                .iter()
                .any(|parameter: &ComponentParameter| parameter.name == parameter_name)
            {
                return Err(CompileError::new(
                    span,
                    format!("component parameter `{parameter_name}` is declared more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after component parameter name")?;
            let ty = self.type_syntax()?;
            parameters.push(ComponentParameter {
                name: parameter_name,
                ty,
                span,
            });
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after component parameters")?;
        self.expect(Kind::LBrace, "expected `{` after component declaration")?;
        let mut states = Vec::new();
        let mut body = None;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("state") || self.word_is("let") {
                states.push(self.state_decl()?);
            } else if self.word_is("body") {
                if body.is_some() {
                    return self.error_here("a component can only declare one body");
                }
                self.advance();
                body = Some(self.block_nodes()?);
            } else {
                return self
                    .error_here("expected a component `state`, `let`, or `body` declaration");
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close component")?;
        let body = body.ok_or_else(|| {
            CompileError::new(
                keyword,
                format!("component `{name}` is missing a `body` block"),
            )
        })?;
        Ok(ComponentDecl {
            name,
            parameters,
            states,
            body,
            span: keyword,
            source_file: None,
        })
    }

    fn app_decl(&mut self) -> Result<App, CompileError> {
        self.expect_word("app")?;
        let (name, span) = self.ident()?;
        self.expect(Kind::LBrace, "expected `{` after app name")?;
        let mut states = Vec::new();
        let mut screens = Vec::new();
        let mut theme = None;
        let mut body = None;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("state") || self.word_is("let") {
                states.push(self.state_decl()?);
            } else if self.word_is("screen") {
                screens.push(self.screen_decl()?);
            } else if self.word_is("theme") {
                if theme.is_some() {
                    return self.error_here("an app can only declare one `theme` block");
                }
                theme = Some(self.theme_decl()?);
            } else if self.word_is("body") {
                if body.is_some() {
                    return self.error_here("an app can only declare one body");
                }
                self.advance();
                body = Some(self.block_nodes()?);
            } else {
                return self
                    .error_here("expected a `state`, `screen`, `theme`, or `body` declaration");
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close app")?;
        let body = body.ok_or_else(|| CompileError::new(span, "app is missing a `body` block"))?;
        Ok(App {
            name,
            states,
            screens,
            theme,
            components: Vec::new(),
            body,
            span,
        })
    }

    fn screen_decl(&mut self) -> Result<ScreenDecl, CompileError> {
        self.expect_word("screen")?;
        let (name, span) = self.ident()?;
        let body = self.block_nodes()?;
        Ok(ScreenDecl { name, body, span })
    }

    fn theme_decl(&mut self) -> Result<ThemeDecl, CompileError> {
        let keyword = self.advance().span;
        self.expect(Kind::LBrace, "expected `{` after `theme`")?;
        let mut tokens = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (kind_name, span) = self.ident()?;
            let kind = match kind_name.as_str() {
                "color" => ThemeTokenKind::Color,
                "spacing" => ThemeTokenKind::Spacing,
                "radius" => ThemeTokenKind::Radius,
                "fontSize" => ThemeTokenKind::FontSize,
                _ => {
                    return Err(CompileError::new(
                        span,
                        format!("unknown theme token type `{kind_name}`"),
                    ));
                }
            };
            let (name, _) = self.ident()?;
            let value = if kind == ThemeTokenKind::Color {
                let mut values = self.named_args(&["light", "dark"])?;
                let light = self.required_arg(
                    &mut values,
                    "light",
                    "theme colors require a `light` value",
                )?;
                let dark =
                    self.required_arg(&mut values, "dark", "theme colors require a `dark` value")?;
                ThemeTokenValue::AdaptiveColor { light, dark }
            } else {
                self.expect(Kind::Colon, "expected `:` after theme token name")?;
                ThemeTokenValue::Static(self.expr()?)
            };
            tokens.push(ThemeTokenDecl {
                kind,
                name,
                value,
                span,
            });
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close `theme` block")?;
        Ok(ThemeDecl {
            tokens,
            span: keyword,
        })
    }

    fn state_decl(&mut self) -> Result<StateDecl, CompileError> {
        let keyword = self.advance().clone();
        let mutable = matches!(&keyword.kind, Kind::Ident(word) if word == "state");
        let (name, span) = self.ident()?;
        self.expect(Kind::Colon, "expected `:` after state name")?;
        let ty = self.type_syntax()?;
        self.expect(Kind::Equal, "expected `=` before initial value")?;
        let initial = self.expr()?;
        self.optional_semicolon();
        Ok(StateDecl {
            name,
            ty,
            initial,
            mutable,
            span,
        })
    }

    fn type_syntax(&mut self) -> Result<TypeSyntax, CompileError> {
        let (name, span) = self.ident()?;
        if !self.take(&Kind::Less) {
            return Ok(TypeSyntax::Named(name, span));
        }
        let mut arguments = vec![self.type_syntax()?];
        while self.take(&Kind::Comma) {
            arguments.push(self.type_syntax()?);
        }
        self.expect(
            Kind::Greater,
            "expected `>` to close generic type arguments",
        )?;
        Ok(TypeSyntax::Generic(name, arguments, span))
    }

    fn block_nodes(&mut self) -> Result<Vec<Node>, CompileError> {
        self.expect(Kind::LBrace, "expected `{` to open block")?;
        let mut nodes = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            nodes.push(self.node()?);
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close block")?;
        Ok(nodes)
    }

    fn node(&mut self) -> Result<Node, CompileError> {
        let (name, span) = self.ident()?;
        match name.as_str() {
            "View" | "Column" | "Row" => {
                let kind = match name.as_str() {
                    "View" => LayoutKind::View,
                    "Column" => LayoutKind::Column,
                    _ => LayoutKind::Row,
                };
                let mut args = if self.check(&Kind::LParen) {
                    self.named_args(&[
                        "spacing",
                        "padding",
                        "width",
                        "height",
                        "background",
                        "cornerRadius",
                        "opacity",
                    ])?
                } else {
                    BTreeMap::new()
                };
                let spacing = args.remove("spacing");
                let style = LayoutStyle {
                    padding: args.remove("padding"),
                    width: args.remove("width"),
                    height: args.remove("height"),
                    background: args.remove("background"),
                    corner_radius: args.remove("cornerRadius"),
                    opacity: args.remove("opacity"),
                };
                let children = self.block_nodes()?;
                Ok(Node::Layout {
                    kind,
                    spacing,
                    style,
                    children,
                    span,
                })
            }
            "Text" => {
                self.expect(Kind::LParen, "expected `(` after Text")?;
                let value = self.expr()?;
                let mut args = if self.take(&Kind::Comma) {
                    self.named_args_contents(&["color", "fontSize"])?
                } else {
                    BTreeMap::new()
                };
                self.expect(Kind::RParen, "expected `)` after Text options")?;
                Ok(Node::Text {
                    value,
                    color: args.remove("color"),
                    font_size: args.remove("fontSize"),
                    span,
                })
            }
            "Button" => {
                self.expect(Kind::LParen, "expected `(` after Button")?;
                let label = self.expr()?;
                self.expect(Kind::RParen, "expected `)` after Button label")?;
                let actions = if self.check(&Kind::LBrace) {
                    self.block_stmts()?
                } else {
                    Vec::new()
                };
                Ok(Node::Button {
                    label,
                    actions,
                    span,
                })
            }
            "TextInput" => {
                let mut args = self.named_args(&[
                    "value",
                    "placeholder",
                    "keyboard",
                    "secure",
                    "multiline",
                    "autocorrect",
                    "capitalization",
                ])?;
                let value = self.required_arg(&mut args, "value", "TextInput requires `value`")?;
                let placeholder = self.required_arg(
                    &mut args,
                    "placeholder",
                    "TextInput requires `placeholder`",
                )?;
                Ok(Node::TextInput {
                    value,
                    placeholder,
                    keyboard: args.remove("keyboard"),
                    secure: args.remove("secure"),
                    multiline: args.remove("multiline"),
                    autocorrect: args.remove("autocorrect"),
                    capitalization: args.remove("capitalization"),
                    span,
                })
            }
            "Switch" => {
                let mut args = self.named_args(&["value", "label"])?;
                let value = self.required_arg(&mut args, "value", "Switch requires `value`")?;
                let label = self.required_arg(&mut args, "label", "Switch requires `label`")?;
                Ok(Node::Switch { value, label, span })
            }
            "Image" => {
                let mut args =
                    self.named_args(&["asset", "url", "description", "scale", "placeholder"])?;
                let asset = args.remove("asset");
                let url = args.remove("url");
                let source = match (asset, url) {
                    (Some(asset), None) => ImageSource::Asset(asset),
                    (None, Some(url)) => ImageSource::Url(url),
                    (None, None) => {
                        return self.error_here("Image requires exactly one of `asset` or `url`");
                    }
                    (Some(_), Some(_)) => {
                        return Err(CompileError::new(
                            span,
                            "Image accepts either `asset` or `url`, not both",
                        ));
                    }
                };
                let description =
                    self.required_arg(&mut args, "description", "Image requires `description`")?;
                Ok(Node::Image {
                    source,
                    description,
                    scale: args.remove("scale"),
                    placeholder: args.remove("placeholder"),
                    span,
                })
            }
            "Pressable" => {
                let args = self.named_args(&["disabled"])?;
                let disabled = args.into_iter().next().map(|(_, value)| value);
                let children = self.block_nodes()?;
                let actions = self.block_stmts()?;
                Ok(Node::Pressable {
                    disabled,
                    children,
                    actions,
                    span,
                })
            }
            "NavigationStack" => {
                let mut args = self.named_args(&["root"])?;
                let root = self.required_arg(
                    &mut args,
                    "root",
                    "NavigationStack requires a declared screen as `root`",
                )?;
                Ok(Node::NavigationStack { root, span })
            }
            "NavigationLink" => {
                let mut args = self.named_args(&["destination"])?;
                let destination = self.required_arg(
                    &mut args,
                    "destination",
                    "NavigationLink requires a declared screen as `destination`",
                )?;
                let children = self.block_nodes()?;
                Ok(Node::NavigationLink {
                    destination,
                    children,
                    span,
                })
            }
            "KeyboardAware" => {
                let children = self.block_nodes()?;
                Ok(Node::KeyboardAware { children, span })
            }
            "FastList" => {
                let mut args = self.named_args(&["count", "items", "index", "item"])?;
                let count = args.remove("count");
                let items = args.remove("items");
                let source = match (count, items) {
                    (Some(count), None) => ListSource::Count(count),
                    (None, Some(items)) => ListSource::Items(items),
                    (None, None) => {
                        return self.error_here("FastList requires `count` or `items`");
                    }
                    (Some(_), Some(_)) => {
                        return Err(CompileError::new(
                            span,
                            "FastList accepts `count` or `items`, not both",
                        ));
                    }
                };
                let index = args.remove("index");
                let item = args.remove("item");
                let children = self.block_nodes()?;
                Ok(Node::FastList {
                    source,
                    index,
                    item,
                    children,
                    span,
                })
            }
            _ if self.check(&Kind::LParen) => Ok(Node::ComponentCall {
                name,
                arguments: self.named_args_any()?,
                span,
            }),
            _ => Err(CompileError::new(
                span,
                format!(
                    "unknown component `{name}`; custom components must use `Name(...)` syntax"
                ),
            )),
        }
    }

    fn named_args(&mut self, allowed: &[&str]) -> Result<BTreeMap<String, Expr>, CompileError> {
        self.expect(Kind::LParen, "expected `(` before component options")?;
        let args = self.named_args_contents(allowed)?;
        self.expect(Kind::RParen, "expected `)` after component options")?;
        Ok(args)
    }

    fn named_args_any(&mut self) -> Result<BTreeMap<String, Expr>, CompileError> {
        self.expect(Kind::LParen, "expected `(` before component arguments")?;
        let args = self.parse_argument_contents(None)?;
        self.expect(Kind::RParen, "expected `)` after component arguments")?;
        Ok(args)
    }

    fn named_args_contents(
        &mut self,
        allowed: &[&str],
    ) -> Result<BTreeMap<String, Expr>, CompileError> {
        self.parse_argument_contents(Some(allowed))
    }

    fn parse_argument_contents(
        &mut self,
        allowed: Option<&[&str]>,
    ) -> Result<BTreeMap<String, Expr>, CompileError> {
        let mut args = BTreeMap::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if allowed.is_some_and(|allowed| !allowed.contains(&name.as_str())) {
                return Err(CompileError::new(span, format!("unknown option `{name}`")));
            }
            if args.contains_key(&name) {
                return Err(CompileError::new(
                    span,
                    format!("option `{name}` was provided more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after option name")?;
            args.insert(name, self.expr()?);
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn required_arg(
        &self,
        args: &mut BTreeMap<String, Expr>,
        name: &str,
        message: &str,
    ) -> Result<Expr, CompileError> {
        args.remove(name)
            .ok_or_else(|| CompileError::new(self.peek().span, message))
    }

    fn block_stmts(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.expect(Kind::LBrace, "expected `{` to open button handler")?;
        let mut stmts = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            self.expect(Kind::Equal, "expected `=` in state assignment")?;
            let value = self.expr()?;
            stmts.push(Stmt::Assign { name, value, span });
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close button handler")?;
        Ok(stmts)
    }

    fn expr(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.primary()?;
        while self.take(&Kind::Plus) {
            let span = left.span();
            let right = self.primary()?;
            left = Expr::Add(Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        if self.check(&Kind::Minus) {
            let minus_span = self.advance().span;
            let token = self.advance().clone();
            if let Kind::Number(raw) = token.kind {
                let value = format!("-{raw}");
                let span = Span {
                    end: token.span.end,
                    ..minus_span
                };
                return Ok(Expr::Number(value, span));
            }
            return Err(CompileError::new(
                token.span,
                "unary `-` requires a numeric literal",
            ));
        }
        if self.take(&Kind::LBracket) {
            let span = self.tokens[self.cursor - 1].span;
            let mut items = Vec::new();
            while !self.check(&Kind::RBracket) && !self.check(&Kind::Eof) {
                items.push(self.expr()?);
                if !self.take(&Kind::Comma) {
                    break;
                }
            }
            self.expect(Kind::RBracket, "expected `]` to close array literal")?;
            return Ok(Expr::Array(items, span));
        }
        let token = self.advance().clone();
        match token.kind {
            Kind::String(value) => Ok(Expr::String(value, token.span)),
            Kind::Number(value) => Ok(Expr::Number(value, token.span)),
            Kind::Ident(value) if value == "true" => Ok(Expr::Bool(true, token.span)),
            Kind::Ident(value) if value == "false" => Ok(Expr::Bool(false, token.span)),
            Kind::Ident(value) => {
                if !self.take(&Kind::Dot) {
                    return Ok(Expr::Name(value, token.span));
                }
                if value != "Theme" {
                    return Err(CompileError::new(
                        token.span,
                        "qualified references must use the `Theme` namespace",
                    ));
                }
                let (name, name_span) = self.ident()?;
                Ok(Expr::ThemeToken(
                    name,
                    Span {
                        end: name_span.end,
                        ..token.span
                    },
                ))
            }
            _ => Err(CompileError::new(
                token.span,
                "expected a string, number, boolean, or state name",
            )),
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), CompileError> {
        if self.word_is(word) {
            self.advance();
            Ok(())
        } else {
            self.error_here(format!("expected `{word}`"))
        }
    }
    fn word_is(&self, word: &str) -> bool {
        matches!(&self.peek().kind, Kind::Ident(value) if value == word)
    }
    fn ident(&mut self) -> Result<(String, Span), CompileError> {
        let token = self.advance().clone();
        if let Kind::Ident(value) = token.kind {
            Ok((value, token.span))
        } else {
            Err(CompileError::new(token.span, "expected an identifier"))
        }
    }
    fn expect(&mut self, kind: Kind, message: &str) -> Result<Token, CompileError> {
        if self.check(&kind) {
            Ok(self.advance().clone())
        } else {
            self.error_here(message)
        }
    }
    fn check(&self, kind: &Kind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }
    fn take(&mut self, kind: &Kind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn optional_semicolon(&mut self) {
        self.take(&Kind::Semicolon);
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len() - 1)]
    }
    fn advance(&mut self) -> &Token {
        let index = self.cursor;
        if !self.check(&Kind::Eof) {
            self.cursor += 1;
        }
        &self.tokens[index]
    }
    fn error_here<T>(&self, message: impl Into<String>) -> Result<T, CompileError> {
        Err(CompileError::new(self.peek().span, message))
    }
}
