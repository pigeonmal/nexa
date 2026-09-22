use std::collections::BTreeMap;

use crate::{
    ast::*,
    lexer::{self, Kind, Token},
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
    Parser {
        tokens,
        cursor: 0,
        enum_names: std::collections::BTreeSet::new(),
    }
    .program()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    enum_names: std::collections::BTreeSet<String>,
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
        let mut enums = Vec::new();
        let mut permissions = Vec::new();
        let mut has_permissions = false;
        let mut states = Vec::new();
        let mut screens = Vec::new();
        let mut functions = Vec::new();
        let mut theme = None;
        let mut body = None;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("enum") {
                let declaration = self.enum_decl()?;
                self.enum_names.insert(declaration.name.clone());
                enums.push(declaration);
            } else if self.word_is("permissions") {
                if has_permissions {
                    return self.error_here("an app can declare only one `permissions` block");
                }
                has_permissions = true;
                permissions = self.permissions_decl()?;
            } else if self.word_is("state") || self.word_is("let") {
                states.push(self.state_decl()?);
            } else if self.word_is("async") {
                functions.push(self.function_decl(true)?);
            } else if self.word_is("fn") {
                functions.push(self.function_decl(false)?);
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
                return self.error_here(
                    "expected an `enum`, `permissions`, `state`, `fn`, `async fn`, `screen`, `theme`, or `body` declaration",
                );
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close app")?;
        let body = body.ok_or_else(|| CompileError::new(span, "app is missing a `body` block"))?;
        Ok(App {
            name,
            enums,
            permissions,
            states,
            functions,
            screens,
            theme,
            components: Vec::new(),
            body,
            span,
        })
    }

    fn permissions_decl(&mut self) -> Result<Vec<PermissionDecl>, CompileError> {
        self.expect_word("permissions")?;
        self.expect(Kind::LBrace, "expected `{` after `permissions`")?;
        let mut permissions = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if permissions
                .iter()
                .any(|permission: &PermissionDecl| permission.name == name)
            {
                return Err(CompileError::new(
                    span,
                    format!("permission `{name}` is declared more than once"),
                ));
            }
            permissions.push(PermissionDecl { name, span });
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close `permissions`")?;
        self.optional_semicolon();
        Ok(permissions)
    }

    fn enum_decl(&mut self) -> Result<EnumDecl, CompileError> {
        let span = self.advance().span;
        let (name, name_span) = self.ident()?;
        self.expect(Kind::LBrace, "expected `{` after enum name")?;
        let mut cases = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (case_name, case_span) = self.ident()?;
            if cases
                .iter()
                .any(|case: &EnumCaseDecl| case.name == case_name)
            {
                return Err(CompileError::new(
                    case_span,
                    format!("enum case `{case_name}` is declared more than once"),
                ));
            }
            cases.push(EnumCaseDecl {
                name: case_name,
                span: case_span,
            });
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close enum")?;
        if cases.is_empty() {
            return Err(CompileError::new(
                name_span,
                format!("enum `{name}` must declare at least one case"),
            ));
        }
        self.optional_semicolon();
        Ok(EnumDecl { name, cases, span })
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
        let ty = if self.take(&Kind::Colon) {
            Some(self.type_syntax()?)
        } else {
            None
        };
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

    fn function_decl(&mut self, is_async: bool) -> Result<FunctionDecl, CompileError> {
        let keyword = self.advance().span;
        if is_async {
            self.expect_word("fn")?;
        }
        let (name, _) = self.ident()?;
        self.expect(Kind::LParen, "expected `(` after function name")?;
        let mut parameters = Vec::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            let (parameter_name, span) = self.ident()?;
            if parameters
                .iter()
                .any(|parameter: &FunctionParameter| parameter.name == parameter_name)
            {
                return Err(CompileError::new(
                    span,
                    format!("function parameter `{parameter_name}` is declared more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after function parameter name")?;
            let ty = self.type_syntax()?;
            parameters.push(FunctionParameter {
                name: parameter_name,
                ty,
                span,
            });
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after function parameters")?;
        self.expect(Kind::Minus, "expected `->` before function return type")?;
        self.expect(Kind::Greater, "expected `->` before function return type")?;
        let return_type = self.type_syntax()?;
        let body = self.function_body()?;
        Ok(FunctionDecl {
            name,
            is_async,
            parameters,
            return_type,
            body,
            span: keyword,
        })
    }

    fn type_syntax(&mut self) -> Result<TypeSyntax, CompileError> {
        let (name, span) = self.ident()?;
        let mut syntax = if !self.take(&Kind::Less) {
            TypeSyntax::Named(name, span)
        } else {
            let mut arguments = vec![self.type_syntax()?];
            while self.take(&Kind::Comma) {
                arguments.push(self.type_syntax()?);
            }
            self.expect(
                Kind::Greater,
                "expected `>` to close generic type arguments",
            )?;
            TypeSyntax::Generic(name, arguments, span)
        };
        if self.take(&Kind::Question) {
            syntax = TypeSyntax::Optional(Box::new(syntax), span);
        }
        Ok(syntax)
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

    fn tab_declarations(&mut self) -> Result<Vec<TabDecl>, CompileError> {
        self.expect(Kind::LBrace, "expected `{` to open AppBottomBar tabs")?;
        let mut tabs = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if name != "Tab" {
                return Err(CompileError::new(
                    span,
                    "AppBottomBar accepts only `Tab(index: ..., label: ...)` entries",
                ));
            }
            let mut args = self.named_args(&["index", "label"])?;
            let index = self.required_arg(&mut args, "index", "Tab requires `index`")?;
            let label = self.required_arg(&mut args, "label", "Tab requires `label`")?;
            let children = self.block_nodes()?;
            tabs.push(TabDecl {
                index,
                label,
                children,
                span,
            });
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close AppBottomBar tabs")?;
        if tabs.is_empty() {
            return self.error_here("AppBottomBar requires at least one `Tab`");
        }
        Ok(tabs)
    }

    fn node(&mut self) -> Result<Node, CompileError> {
        if self.word_is("if") {
            return self.if_node();
        }
        if self.word_is("when") {
            return self.when_node();
        }
        let (name, span) = self.ident()?;
        match name.as_str() {
            "platform" => {
                let (target, target_span) = self.ident()?;
                let target = match target.as_str() {
                    "ios" => PlatformTarget::Ios,
                    "android" => PlatformTarget::Android,
                    _ => {
                        return Err(CompileError::new(
                            target_span,
                            "unknown platform; expected `ios` or `android`",
                        ));
                    }
                };
                Ok(Node::Platform {
                    target,
                    children: self.block_nodes()?,
                    span,
                })
            }
            "StatusBar" => {
                let mut args = self.named_args(&["style", "hidden"])?;
                Ok(Node::StatusBar {
                    style: args.remove("style"),
                    hidden: args.remove("hidden"),
                    span,
                })
            }
            "Direction" => {
                let mut args = self.named_args(&["value"])?;
                let value = self.required_arg(
                    &mut args,
                    "value",
                    "Direction requires `value: LTR` or `value: RTL`",
                )?;
                Ok(Node::Direction { value, span })
            }
            "OnAppear" => {
                let asynchronous = if self.word_is("async") {
                    self.advance();
                    true
                } else {
                    false
                };
                Ok(Node::OnAppear {
                    actions: self.block_stmts()?,
                    asynchronous,
                    span,
                })
            }
            "OnDisappear" => Ok(Node::OnDisappear {
                actions: self.block_stmts()?,
                span,
            }),
            "Column" | "Row" => {
                let kind = match name.as_str() {
                    "Column" => LayoutKind::Column,
                    _ => LayoutKind::Row,
                };
                let mut args = if self.check(&Kind::LParen) {
                    self.named_args(&[
                        "spacing",
                        "alignment",
                        "padding",
                        "width",
                        "height",
                        "background",
                        "cornerRadius",
                        "borderColor",
                        "borderWidth",
                        "opacity",
                        "animation",
                    ])?
                } else {
                    BTreeMap::new()
                };
                let spacing = args.remove("spacing");
                let style = LayoutStyle {
                    alignment: args.remove("alignment"),
                    padding: args.remove("padding"),
                    width: args.remove("width"),
                    height: args.remove("height"),
                    background: args.remove("background"),
                    corner_radius: args.remove("cornerRadius"),
                    border_color: args.remove("borderColor"),
                    border_width: args.remove("borderWidth"),
                    opacity: args.remove("opacity"),
                    animation: args.remove("animation"),
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
                    self.named_args_contents(&[
                        "color",
                        "fontSize",
                        "fontWeight",
                        "lineLimit",
                        "lineHeight",
                        "letterSpacing",
                        "selectable",
                    ])?
                } else {
                    BTreeMap::new()
                };
                self.expect(Kind::RParen, "expected `)` after Text options")?;
                Ok(Node::Text {
                    value,
                    color: args.remove("color"),
                    font_size: args.remove("fontSize"),
                    font_weight: args.remove("fontWeight"),
                    line_limit: args.remove("lineLimit"),
                    line_height: args.remove("lineHeight"),
                    letter_spacing: args.remove("letterSpacing"),
                    selectable: args.remove("selectable"),
                    span,
                })
            }
            "Button" => {
                self.expect(Kind::LParen, "expected `(` after Button")?;
                let label = self.expr()?;
                let options = if self.take(&Kind::Comma) {
                    self.named_args_contents(&["loading", "disabled"])?
                } else {
                    BTreeMap::new()
                };
                self.expect(Kind::RParen, "expected `)` after Button options")?;
                let actions = if self.check(&Kind::LBrace) {
                    self.block_stmts()?
                } else {
                    Vec::new()
                };
                Ok(Node::Button {
                    label,
                    loading: options.get("loading").cloned(),
                    disabled: options.get("disabled").cloned(),
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
                    "focused",
                ])?;
                let value = self.required_arg(&mut args, "value", "TextInput requires `value`")?;
                let placeholder = self.required_arg(
                    &mut args,
                    "placeholder",
                    "TextInput requires `placeholder`",
                )?;
                let actions = if self.check(&Kind::LBrace) {
                    self.block_stmts()?
                } else {
                    Vec::new()
                };
                Ok(Node::TextInput {
                    value,
                    placeholder,
                    keyboard: args.remove("keyboard"),
                    secure: args.remove("secure"),
                    multiline: args.remove("multiline"),
                    autocorrect: args.remove("autocorrect"),
                    capitalization: args.remove("capitalization"),
                    focused: args.remove("focused"),
                    actions,
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
                let long_press_actions = if self.check(&Kind::LBrace) {
                    self.block_stmts()?
                } else {
                    Vec::new()
                };
                Ok(Node::Pressable {
                    disabled,
                    children,
                    actions,
                    long_press_actions,
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
            "Link" => {
                let mut args = self.named_args(&["url"])?;
                let url = self.required_arg(&mut args, "url", "Link requires `url`")?;
                let children = self.block_nodes()?;
                Ok(Node::Link {
                    url,
                    children,
                    span,
                })
            }
            "Accessibility" => {
                let mut args = self.named_args(&["label", "role"])?;
                let label =
                    self.required_arg(&mut args, "label", "Accessibility requires `label`")?;
                let role = args.remove("role");
                let children = self.block_nodes()?;
                Ok(Node::Accessibility {
                    label,
                    role,
                    children,
                    span,
                })
            }
            "KeyboardAware" => {
                let children = self.block_nodes()?;
                Ok(Node::KeyboardAware { children, span })
            }
            "BottomSheet" => {
                let mut args = self.named_args(&["isPresented"])?;
                let is_presented = self.required_arg(
                    &mut args,
                    "isPresented",
                    "BottomSheet requires `isPresented`",
                )?;
                let children = self.block_nodes()?;
                Ok(Node::BottomSheet {
                    is_presented,
                    children,
                    span,
                })
            }
            "RefreshControl" => {
                let mut args = self.named_args(&["isRefreshing"])?;
                let is_refreshing = self.required_arg(
                    &mut args,
                    "isRefreshing",
                    "RefreshControl requires `isRefreshing`",
                )?;
                let children = self.block_nodes()?;
                let actions = self.block_stmts()?;
                Ok(Node::RefreshControl {
                    is_refreshing,
                    children,
                    actions,
                    span,
                })
            }
            "AppBottomBar" => {
                let mut args = self.named_args(&["selected"])?;
                let selected =
                    self.required_arg(&mut args, "selected", "AppBottomBar requires `selected`")?;
                let tabs = self.tab_declarations()?;
                Ok(Node::AppBottomBar {
                    selected,
                    tabs,
                    span,
                })
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
        self.statements(false)
    }

    fn function_body(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.statements(true)
    }

    fn statements(&mut self, allow_return: bool) -> Result<Vec<Stmt>, CompileError> {
        self.expect(
            Kind::LBrace,
            if allow_return {
                "expected `{` to open function body"
            } else {
                "expected `{` to open event handler"
            },
        )?;
        let mut stmts = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if !allow_return && self.word_is("let") {
                return self
                    .error_here("local `let` declarations are only allowed inside functions");
            }
            if allow_return && self.word_is("let") {
                let span = self.advance().span;
                let (name, _) = self.ident()?;
                let ty = if self.take(&Kind::Colon) {
                    Some(self.type_syntax()?)
                } else {
                    None
                };
                self.expect(Kind::Equal, "expected `=` after local constant")?;
                let initial = self.expr()?;
                stmts.push(Stmt::Let {
                    name,
                    ty,
                    initial,
                    span,
                });
                self.optional_semicolon();
                continue;
            }
            if self.word_is("return") {
                if !allow_return {
                    return self.error_here("`return` is only allowed inside a function");
                }
                let span = self.advance().span;
                let value = self.expr()?;
                stmts.push(Stmt::Return { value, span });
                self.optional_semicolon();
                continue;
            }
            if self.word_is("if") {
                stmts.push(self.if_stmt()?);
                self.optional_semicolon();
                continue;
            }
            if self.word_is("for") {
                stmts.push(self.for_stmt()?);
                self.optional_semicolon();
                continue;
            }
            if self.word_is("while") {
                stmts.push(self.while_stmt()?);
                self.optional_semicolon();
                continue;
            }
            if self.word_is("break") {
                let span = self.advance().span;
                stmts.push(Stmt::Break { span });
                self.optional_semicolon();
                continue;
            }
            if self.word_is("continue") {
                let span = self.advance().span;
                stmts.push(Stmt::Continue { span });
                self.optional_semicolon();
                continue;
            }
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
        let mut left = self.logical_or()?;
        while self.take(&Kind::QuestionQuestion) {
            let span = left.span();
            let right = self.logical_or()?;
            left = Expr::Coalesce(Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn logical_or(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.logical_and()?;
        while self.take(&Kind::OrOr) {
            let span = left.span();
            let right = self.logical_and()?;
            left = Expr::Binary(Box::new(left), BinaryOp::Or, Box::new(right), span);
        }
        Ok(left)
    }

    fn logical_and(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.equality()?;
        while self.take(&Kind::AndAnd) {
            let span = left.span();
            let right = self.equality()?;
            left = Expr::Binary(Box::new(left), BinaryOp::And, Box::new(right), span);
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.comparison()?;
        loop {
            let operator = if self.take(&Kind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.take(&Kind::BangEqual) {
                Some(BinaryOp::NotEqual)
            } else if self.word_is("in") {
                self.advance();
                Some(BinaryOp::Contains)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let span = left.span();
            let right = self.comparison()?;
            left = Expr::Binary(Box::new(left), operator, Box::new(right), span);
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.addition()?;
        loop {
            let operator = if self.take(&Kind::Less) {
                Some(BinaryOp::Less)
            } else if self.take(&Kind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.take(&Kind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.take(&Kind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(operator) = operator else {
                break;
            };
            let span = left.span();
            let right = self.addition()?;
            left = Expr::Binary(Box::new(left), operator, Box::new(right), span);
        }
        Ok(left)
    }

    fn addition(&mut self) -> Result<Expr, CompileError> {
        let mut left = self.unary()?;
        while self.take(&Kind::Plus) {
            let span = left.span();
            let right = self.unary()?;
            left = Expr::Add(Box::new(left), Box::new(right), span);
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, CompileError> {
        if self.take(&Kind::Bang) {
            let span = self.tokens[self.cursor - 1].span;
            let expression = self.unary()?;
            return Ok(Expr::Not(Box::new(expression), span));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        let mut expression = self.primary_atom()?;
        loop {
            if self.take(&Kind::LBracket) {
                let span = expression.span();
                let index = self.expr()?;
                self.expect(Kind::RBracket, "expected `]` after collection index")?;
                expression = Expr::Index {
                    collection: Box::new(expression),
                    index: Box::new(index),
                    optional: false,
                    span,
                };
            } else if self.take(&Kind::Dot) {
                let (name, name_span) = self.ident()?;
                let span = Span {
                    end: name_span.end,
                    ..expression.span()
                };
                expression = Expr::Member {
                    base: Box::new(expression),
                    name,
                    optional: false,
                    span,
                };
            } else if self.take(&Kind::Question) {
                if self.take(&Kind::LBracket) {
                    let span = expression.span();
                    let index = self.expr()?;
                    self.expect(
                        Kind::RBracket,
                        "expected `]` after optional collection index",
                    )?;
                    expression = Expr::Index {
                        collection: Box::new(expression),
                        index: Box::new(index),
                        optional: true,
                        span,
                    };
                } else {
                    self.expect(
                        Kind::Dot,
                        "expected `.` or `[` after `?` in optional access",
                    )?;
                    let (name, name_span) = self.ident()?;
                    let span = Span {
                        end: name_span.end,
                        ..expression.span()
                    };
                    expression = Expr::Member {
                        base: Box::new(expression),
                        name,
                        optional: true,
                        span,
                    };
                }
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn primary_atom(&mut self) -> Result<Expr, CompileError> {
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
            if self.take(&Kind::Colon) {
                self.expect(Kind::RBracket, "expected `]` after empty map literal")?;
                return Ok(Expr::Map(Vec::new(), span));
            }
            if self.take(&Kind::RBracket) {
                return Ok(Expr::Array(Vec::new(), span));
            }

            let first = self.expr()?;
            if self.take(&Kind::Colon) {
                let mut entries = vec![(first, self.expr()?)];
                while self.take(&Kind::Comma) && !self.check(&Kind::RBracket) {
                    let key = self.expr()?;
                    self.expect(Kind::Colon, "expected `:` between map key and value")?;
                    entries.push((key, self.expr()?));
                }
                self.expect(Kind::RBracket, "expected `]` to close map literal")?;
                return Ok(Expr::Map(entries, span));
            }

            let mut items = vec![first];
            while self.take(&Kind::Comma) && !self.check(&Kind::RBracket) {
                items.push(self.expr()?);
            }
            self.expect(Kind::RBracket, "expected `]` to close collection literal")?;
            return Ok(Expr::Array(items, span));
        }
        if self.take(&Kind::LParen) {
            let expression = self.expr()?;
            self.expect(Kind::RParen, "expected `)` after expression")?;
            return Ok(expression);
        }
        let token = self.advance().clone();
        match token.kind {
            Kind::String(value) => self.string_expression(value, token.span),
            Kind::Number(value) => Ok(Expr::Number(value, token.span)),
            Kind::Ident(value) if value == "true" => Ok(Expr::Bool(true, token.span)),
            Kind::Ident(value) if value == "false" => Ok(Expr::Bool(false, token.span)),
            Kind::Ident(value) if value == "null" => Ok(Expr::Null(token.span)),
            Kind::Ident(value) => {
                if value == "await" {
                    let expression = self.unary()?;
                    return Ok(Expr::Await(Box::new(expression), token.span));
                }
                if (value == "Pair" || value == "Triple") && self.check(&Kind::LParen) {
                    return self.tuple_constructor(value, token.span);
                }
                if self.check(&Kind::LParen) {
                    return self.call_expression(value, token.span);
                }
                if value == "Theme" || value == "Layout" {
                    if !self.take(&Kind::Dot) {
                        return Ok(Expr::Name(value, token.span));
                    }
                    let (name, name_span) = self.ident()?;
                    let span = Span {
                        end: name_span.end,
                        ..token.span
                    };
                    return match value.as_str() {
                        "Theme" => Ok(Expr::ThemeToken(name, span)),
                        "Layout" if name == "isRegularWidth" => Ok(Expr::IsRegularWidth(span)),
                        "Layout" => Err(CompileError::new(
                            name_span,
                            format!("unknown layout property `{name}`; expected `isRegularWidth`"),
                        )),
                        _ => unreachable!("namespace checked above"),
                    };
                }
                if self.enum_names.contains(&value) && self.take(&Kind::Dot) {
                    let (case_name, case_span) = self.ident()?;
                    let span = Span {
                        end: case_span.end,
                        ..token.span
                    };
                    return Ok(Expr::EnumCase {
                        enum_name: value,
                        case_name,
                        span,
                    });
                }
                Ok(Expr::Name(value, token.span))
            }
            _ => Err(CompileError::new(
                token.span,
                "expected a string, number, boolean, or state name",
            )),
        }
    }

    fn string_expression(&self, value: String, span: Span) -> Result<Expr, CompileError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut chars = value.chars().peekable();

        while let Some(character) = chars.next() {
            let part = if character == '$' {
                if chars.peek().copied().is_some_and(is_ident_start) {
                    Some(StringPart::Name(self.take_interpolation_name(&mut chars)))
                } else {
                    None
                }
            } else if character == '\\' && chars.peek() == Some(&'(') {
                chars.next();
                let source = self.take_interpolation_expression(&mut chars, span)?;
                Some(StringPart::Expression(
                    self.parse_interpolation_expression(&source, span)?,
                ))
            } else {
                None
            };

            if let Some(part) = part {
                if !literal.is_empty() {
                    parts.push(StringPart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(part);
            } else {
                literal.push(character);
            }
        }
        if !literal.is_empty() {
            parts.push(StringPart::Literal(literal));
        }
        if parts
            .iter()
            .all(|part| matches!(part, StringPart::Literal(_)))
        {
            return Ok(Expr::String(value, span));
        }
        Ok(Expr::Interpolation(parts, span))
    }

    fn take_interpolation_expression(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
        span: Span,
    ) -> Result<String, CompileError> {
        let mut expression = String::new();
        let mut depth = 1usize;
        let mut in_string = false;
        let mut escaped = false;
        while let Some(character) = chars.next() {
            if in_string {
                expression.push(character);
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    in_string = false;
                }
                continue;
            }
            match character {
                '"' => {
                    in_string = true;
                    expression.push(character);
                }
                '(' => {
                    depth += 1;
                    expression.push(character);
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(expression);
                    }
                    expression.push(character);
                }
                _ => expression.push(character),
            }
        }
        Err(CompileError::new(
            span,
            "unterminated string interpolation; expected `)`",
        ))
    }

    fn parse_interpolation_expression(
        &self,
        source: &str,
        span: Span,
    ) -> Result<Expr, CompileError> {
        let tokens = lexer::lex(source)?;
        let mut parser = Parser {
            tokens,
            cursor: 0,
            enum_names: self.enum_names.clone(),
        };
        let expression = parser.expr()?;
        if !parser.check(&Kind::Eof) {
            return Err(CompileError::new(
                span,
                "string interpolation contains an incomplete expression",
            ));
        }
        Ok(expression)
    }

    fn take_interpolation_name(
        &self,
        chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    ) -> String {
        let mut name = String::new();
        while chars.peek().copied().is_some_and(is_ident_continue) {
            name.push(chars.next().expect("peeked character exists"));
        }
        name
    }

    fn tuple_constructor(&mut self, name: String, span: Span) -> Result<Expr, CompileError> {
        self.expect(Kind::LParen, "expected `(` after tuple type name")?;
        let arity = if name == "Pair" { 2 } else { 3 };
        let mut values = Vec::with_capacity(arity);
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            values.push(self.expr()?);
            if !self.take(&Kind::Comma) || self.check(&Kind::RParen) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after tuple values")?;
        if values.len() != arity {
            return Err(CompileError::new(
                span,
                format!("{name} requires exactly {arity} values"),
            ));
        }
        let mut values = values.into_iter();
        if name == "Pair" {
            Ok(Expr::Pair(
                Box::new(values.next().expect("pair arity checked")),
                Box::new(values.next().expect("pair arity checked")),
                span,
            ))
        } else {
            Ok(Expr::Triple(
                Box::new(values.next().expect("triple arity checked")),
                Box::new(values.next().expect("triple arity checked")),
                Box::new(values.next().expect("triple arity checked")),
                span,
            ))
        }
    }

    fn call_expression(&mut self, name: String, span: Span) -> Result<Expr, CompileError> {
        self.expect(Kind::LParen, "expected `(` after function name")?;
        let mut arguments = Vec::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            arguments.push(self.expr()?);
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after function arguments")?;
        Ok(Expr::Call(name, arguments, span))
    }

    fn if_node(&mut self) -> Result<Node, CompileError> {
        let span = self.advance().span;
        let condition = self.expr()?;
        let then_body = self.block_nodes()?;
        let else_body = if self.word_is("else") {
            self.advance();
            if self.word_is("if") {
                Some(vec![self.if_node()?])
            } else {
                Some(self.block_nodes()?)
            }
        } else {
            None
        };
        Ok(Node::If {
            condition,
            then_body,
            else_body,
            span,
        })
    }

    fn when_node(&mut self) -> Result<Node, CompileError> {
        let span = self.advance().span;
        let value = self.expr()?;
        self.expect(Kind::LBrace, "expected `{` to open when cases")?;
        let mut cases = Vec::new();
        let mut else_body = None;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("else") {
                let else_span = self.advance().span;
                if else_body.is_some() {
                    return Err(CompileError::new(
                        else_span,
                        "when can declare only one `else` case",
                    ));
                }
                self.expect(Kind::Colon, "expected `:` after `else`")?;
                else_body = Some(self.block_nodes()?);
            } else {
                let case_value = self.expr()?;
                let case_span = case_value.span();
                self.expect(Kind::Colon, "expected `:` after when case value")?;
                cases.push(WhenCase {
                    value: case_value,
                    body: self.block_nodes()?,
                    span: case_span,
                });
            }
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close when cases")?;
        let Some(else_body) = else_body else {
            return Err(CompileError::new(
                span,
                "when requires an `else` case for exhaustive native lowering",
            ));
        };
        if cases.is_empty() {
            return Err(CompileError::new(
                span,
                "when requires at least one value case",
            ));
        }
        Ok(Node::When {
            value,
            cases,
            else_body,
            span,
        })
    }

    fn if_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        let condition = self.expr()?;
        let then_branch = self.block_stmts()?;
        let else_branch = if self.word_is("else") {
            self.advance();
            if self.word_is("if") {
                Some(vec![self.if_stmt()?])
            } else {
                Some(self.block_stmts()?)
            }
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
        })
    }

    fn for_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        if self.take(&Kind::LParen) {
            let (key_name, _) = self.ident()?;
            self.expect(Kind::Comma, "expected `,` between map loop bindings")?;
            let (value_name, _) = self.ident()?;
            self.expect(Kind::RParen, "expected `)` after map loop bindings")?;
            self.expect_word("in")?;
            let iterable = self.expr()?;
            let body = self.block_stmts()?;
            return Ok(Stmt::ForMap {
                key_name,
                value_name,
                iterable,
                body,
                span,
            });
        }
        let (name, _) = self.ident()?;
        self.expect_word("in")?;
        let start = self.expr()?;
        let iterable = if self.take(&Kind::DotDot)
            || self.take(&Kind::DotDotDot)
            || self.take(&Kind::DotDotLess)
        {
            let operator = self.tokens[self.cursor - 1].kind.clone();
            let end = self.expr()?;
            let step = if self.word_is("step") {
                self.advance();
                Some(Box::new(self.expr()?))
            } else {
                None
            };
            let range_span = Span {
                end: step
                    .as_deref()
                    .map_or_else(|| end.span().end, |step| step.span().end),
                ..start.span()
            };
            Expr::Range {
                start: Box::new(start),
                end: Box::new(end),
                inclusive: !matches!(operator, Kind::DotDotLess),
                step,
                span: range_span,
            }
        } else {
            start
        };
        let body = self.block_stmts()?;
        Ok(Stmt::For {
            name,
            iterable,
            body,
            span,
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt, CompileError> {
        let span = self.advance().span;
        let condition = self.expr()?;
        let body = self.block_stmts()?;
        Ok(Stmt::While {
            condition,
            body,
            span,
        })
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

fn is_ident_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_ident_continue(character: char) -> bool {
    is_ident_start(character) || character.is_ascii_digit()
}
