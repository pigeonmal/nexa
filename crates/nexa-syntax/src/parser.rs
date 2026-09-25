use std::collections::BTreeMap;

use crate::{
    ast::*,
    catalog::{self, ChildModel, ParensModel, PositionalModel},
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
    app.structs = program.structs;
    app.functions.extend(program.functions);
    app.plugins = program.plugins;
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

pub fn parse_config(tokens: Vec<Token>) -> Result<Config, CompileError> {
    Parser {
        tokens,
        cursor: 0,
        enum_names: std::collections::BTreeSet::new(),
    }
    .config()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    enum_names: std::collections::BTreeSet<String>,
}

impl Parser {
    fn config(mut self) -> Result<Config, CompileError> {
        let span = self.peek().span;
        self.expect_word("config")?;
        self.expect(Kind::LBrace, "expected `{` after config")?;
        let mut app = None;
        let mut flavors = Vec::new();
        let mut has_flavors = false;
        let mut assets = None;
        let mut dependencies = Vec::new();
        let mut permissions = Vec::new();
        let mut has_permissions = false;
        let mut plugins = Vec::new();
        let mut has_plugins = false;
        let mut has_dependencies = false;
        let mut ios = None;
        let mut android = None;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("app") {
                if app.is_some() {
                    return self.error_here("a config can declare only one `app` block");
                }
                app = Some(self.config_app_decl()?);
            } else if self.word_is("flavors") {
                if has_flavors {
                    return self.error_here("a config can declare only one `flavors` block");
                }
                has_flavors = true;
                flavors = self.config_flavors_decl()?;
            } else if self.word_is("dependencies") {
                if has_dependencies {
                    return self.error_here("a config can declare only one `dependencies` block");
                }
                has_dependencies = true;
                dependencies = self.config_dependencies_decl()?;
            } else if self.word_is("assets") {
                if assets.is_some() {
                    return self.error_here("a config can declare only one `assets` block");
                }
                assets = Some(self.config_assets_decl()?);
            } else if self.word_is("permissions") {
                if has_permissions {
                    return self.error_here("a config can declare only one `permissions` block");
                }
                has_permissions = true;
                permissions = self.config_permissions_decl()?;
            } else if self.word_is("plugins") {
                if has_plugins {
                    return self.error_here("a config can declare only one `plugins` block");
                }
                has_plugins = true;
                plugins = self.config_plugins_decl()?;
            } else if self.word_is("ios") {
                if ios.is_some() {
                    return self.error_here("a config can declare only one `ios` block");
                }
                ios = Some(self.config_ios_decl()?);
            } else if self.word_is("android") {
                if android.is_some() {
                    return self.error_here("a config can declare only one `android` block");
                }
                android = Some(self.config_android_decl()?);
            } else {
                return self.error_here(
                    "expected an `app`, `flavors`, `dependencies`, `permissions`, `plugins`, `ios`, or `android` block in config",
                );
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close config")?;
        self.optional_semicolon();
        if !self.check(&Kind::Eof) {
            return self.error_here("unexpected content after config");
        }
        Ok(Config {
            app,
            flavors,
            assets,
            dependencies,
            permissions,
            plugins,
            ios,
            android,
            span,
        })
    }

    fn config_assets_decl(&mut self) -> Result<AssetsConfig, CompileError> {
        self.expect_word("assets")?;
        self.expect(Kind::LBrace, "expected `{` after `assets`")?;
        let mut assets = AssetsConfig {
            icon: None,
            splash: None,
        };
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (field, span) = self.ident()?;
            self.expect(Kind::Colon, "expected `:` after asset field")?;
            match field.as_str() {
                "icon" => assets.icon = Some(self.config_string("asset icon")?),
                "splash" => assets.splash = Some(self.config_string("splash asset")?),
                _ => {
                    return Err(CompileError::new(
                        span,
                        format!("unknown asset field `{field}`"),
                    ));
                }
            }
            self.config_field_separator("assets")?;
        }
        self.expect(Kind::RBrace, "expected `}` to close config assets")?;
        self.optional_semicolon();
        Ok(assets)
    }

    fn config_app_decl(&mut self) -> Result<AppConfig, CompileError> {
        self.expect_word("app")?;
        self.expect(Kind::LBrace, "expected `{` after `app`")?;
        let mut display_name = None;
        let mut version = None;
        let mut build_number = None;
        let mut staging_suffix = None;
        let mut deep_links = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (field, field_span) = self.ident()?;
            self.expect(Kind::Colon, "expected `:` after app metadata field")?;
            match field.as_str() {
                "displayName" => display_name = Some(self.config_string("app displayName")?),
                "version" => version = Some(self.config_string("app version")?),
                "buildNumber" => build_number = Some(self.config_u32("app buildNumber")?),
                "stagingSuffix" => staging_suffix = Some(self.config_string("app stagingSuffix")?),
                "deepLinks" => deep_links = self.config_string_array("app deepLinks")?,
                _ => {
                    return Err(CompileError::new(
                        field_span,
                        format!("unknown app metadata field `{field}`"),
                    ));
                }
            }
            self.config_field_separator("app metadata")?;
        }
        self.expect(Kind::RBrace, "expected `}` to close config app")?;
        Ok(AppConfig {
            display_name: display_name.unwrap_or_else(|| "NexaApp".to_owned()),
            version: version.unwrap_or_else(|| "1.0.0".to_owned()),
            build_number: build_number.unwrap_or(1),
            staging_suffix,
            deep_links,
        })
    }

    fn config_string_array(&mut self, context: &str) -> Result<Vec<String>, CompileError> {
        let token = self.advance().clone();
        let Kind::LBracket = token.kind else {
            return Err(CompileError::new(
                token.span,
                format!("{context} must be an array of quoted strings"),
            ));
        };
        let mut values = Vec::new();
        while !self.check(&Kind::RBracket) && !self.check(&Kind::Eof) {
            let item = self.advance().clone();
            let Kind::String(value) = item.kind else {
                return Err(CompileError::new(
                    item.span,
                    format!("{context} entries must be quoted strings"),
                ));
            };
            values.push(value);
            if !self.take(&Kind::Comma) && !self.check(&Kind::RBracket) {
                return self.error_here(format!("expected `,` or `]` in {context}"));
            }
        }
        self.expect(Kind::RBracket, "expected `]` after app deepLinks")?;
        Ok(values)
    }

    fn config_flavors_decl(&mut self) -> Result<Vec<FlavorConfig>, CompileError> {
        self.expect_word("flavors")?;
        self.expect(Kind::LBrace, "expected `{` after `flavors`")?;
        let mut flavors = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if flavors
                .iter()
                .any(|flavor: &FlavorConfig| flavor.name == name)
            {
                return Err(CompileError::new(
                    span,
                    format!("flavor `{name}` is declared more than once"),
                ));
            }
            self.expect(Kind::LBrace, "expected `{` after flavor name")?;
            let mut suffix = None;
            while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
                let (field, field_span) = self.ident()?;
                self.expect(Kind::Colon, "expected `:` after flavor field")?;
                match field.as_str() {
                    "suffix" => suffix = Some(self.config_string("flavor suffix")?),
                    _ => {
                        return Err(CompileError::new(
                            field_span,
                            format!("unknown flavor field `{field}`"),
                        ));
                    }
                }
                self.config_field_separator("flavor")?;
            }
            self.expect(Kind::RBrace, "expected `}` to close flavor")?;
            flavors.push(FlavorConfig { name, suffix });
            self.take(&Kind::Comma);
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close config flavors")?;
        self.optional_semicolon();
        Ok(flavors)
    }

    fn config_ios_decl(&mut self) -> Result<IosConfig, CompileError> {
        self.expect_word("ios")?;
        self.expect(Kind::LBrace, "expected `{` after `ios`")?;
        let mut config = IosConfig {
            min_version: None,
            bundle_identifier: None,
            icon: None,
        };
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (field, field_span) = self.ident()?;
            self.expect(Kind::Colon, "expected `:` after iOS config field")?;
            match field.as_str() {
                "minVersion" => config.min_version = Some(self.config_string("ios minVersion")?),
                "bundleIdentifier" => {
                    config.bundle_identifier = Some(self.config_string("ios bundleIdentifier")?)
                }
                "icon" => config.icon = Some(self.config_string("ios icon")?),
                _ => {
                    return Err(CompileError::new(
                        field_span,
                        format!("unknown iOS config field `{field}`"),
                    ));
                }
            }
            self.config_field_separator("iOS config")?;
        }
        self.expect(Kind::RBrace, "expected `}` to close config ios")?;
        Ok(config)
    }

    fn config_android_decl(&mut self) -> Result<AndroidConfig, CompileError> {
        self.expect_word("android")?;
        self.expect(Kind::LBrace, "expected `{` after `android`")?;
        let mut config = AndroidConfig {
            min_sdk: None,
            target_sdk: None,
            application_id: None,
            icon: None,
        };
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (field, field_span) = self.ident()?;
            self.expect(Kind::Colon, "expected `:` after Android config field")?;
            match field.as_str() {
                "minSdk" => config.min_sdk = Some(self.config_u32("android minSdk")?),
                "targetSdk" => config.target_sdk = Some(self.config_u32("android targetSdk")?),
                "applicationId" => {
                    config.application_id = Some(self.config_string("android applicationId")?)
                }
                "icon" => config.icon = Some(self.config_string("android icon")?),
                _ => {
                    return Err(CompileError::new(
                        field_span,
                        format!("unknown Android config field `{field}`"),
                    ));
                }
            }
            self.config_field_separator("Android config")?;
        }
        self.expect(Kind::RBrace, "expected `}` to close config android")?;
        Ok(config)
    }

    fn config_dependencies_decl(&mut self) -> Result<Vec<PluginDependencyConfig>, CompileError> {
        self.expect_word("dependencies")?;
        self.expect(Kind::LBrace, "expected `{` after `dependencies`")?;
        let mut dependencies = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (alias, span) = self.ident()?;
            if dependencies
                .iter()
                .any(|dependency: &PluginDependencyConfig| dependency.alias == alias)
            {
                return Err(CompileError::new(
                    span,
                    format!("plugin dependency alias `{alias}` is declared more than once"),
                ));
            }
            self.expect(Kind::LBrace, "expected `{` after plugin dependency alias")?;
            let mut package_id = None;
            let mut path = None;
            let mut git = None;
            let mut revision = None;
            let mut package_path = None;
            while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
                let (field, field_span) = self.ident()?;
                self.expect(Kind::Colon, "expected `:` after plugin dependency field")?;
                match field.as_str() {
                    "id" => package_id = Some(self.config_string("plugin dependency id")?),
                    "path" => path = Some(self.config_string("plugin dependency path")?),
                    "git" => git = Some(self.config_string("plugin dependency git URL")?),
                    "rev" => revision = Some(self.config_string("plugin dependency rev")?),
                    "package" => {
                        package_path = Some(self.config_string("plugin package subdirectory")?)
                    }
                    _ => {
                        return Err(CompileError::new(
                            field_span,
                            format!("unknown plugin dependency field `{field}`"),
                        ));
                    }
                }
                self.config_field_separator("plugin dependency")?;
            }
            self.expect(Kind::RBrace, "expected `}` to close plugin dependency")?;
            if path.is_some() == git.is_some() {
                return Err(CompileError::new(
                    span,
                    "plugin dependency must declare exactly one of `path` or `git`",
                ));
            }
            if git.is_some() && revision.is_none() {
                return Err(CompileError::new(
                    span,
                    "Git plugin dependency must pin a `rev`",
                ));
            }
            if path.is_some() && (revision.is_some() || package_path.is_some()) {
                return Err(CompileError::new(
                    span,
                    "`rev` and `package` are only valid for Git plugin dependencies",
                ));
            }
            dependencies.push(PluginDependencyConfig {
                alias,
                package_id: package_id
                    .ok_or_else(|| CompileError::new(span, "plugin dependency requires `id`"))?,
                path,
                git,
                revision,
                package_path,
                span,
            });
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close config dependencies")?;
        self.optional_semicolon();
        Ok(dependencies)
    }

    fn config_string(&mut self, context: &str) -> Result<String, CompileError> {
        let token = self.advance().clone();
        match token.kind {
            Kind::String(value) => Ok(value),
            _ => Err(CompileError::new(
                token.span,
                format!("{context} must be a quoted string"),
            )),
        }
    }

    fn config_u32(&mut self, context: &str) -> Result<u32, CompileError> {
        let token = self.advance().clone();
        match token.kind {
            Kind::Number(value) if !value.contains('.') => value.parse().map_err(|_| {
                CompileError::new(token.span, format!("{context} must be a positive integer"))
            }),
            _ => Err(CompileError::new(
                token.span,
                format!("{context} must be a positive integer"),
            )),
        }
    }

    fn config_field_separator(&mut self, context: &str) -> Result<(), CompileError> {
        if self.take(&Kind::Comma) {
            return Ok(());
        }
        self.optional_semicolon();
        if self.check(&Kind::RBrace) {
            return Ok(());
        }
        self.error_here(format!("expected `,` or `}}` after {context} field"))
    }

    fn config_permissions_decl(&mut self) -> Result<Vec<PermissionConfig>, CompileError> {
        self.expect_word("permissions")?;
        self.expect(Kind::LBrace, "expected `{` after `permissions`")?;
        let mut permissions = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if permissions
                .iter()
                .any(|permission: &PermissionConfig| permission.name == name)
            {
                return Err(CompileError::new(
                    span,
                    format!("permission `{name}` is declared more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after permission name")?;
            let token = self.advance().clone();
            let Kind::String(message) = token.kind else {
                return Err(CompileError::new(
                    token.span,
                    "permission message must be a quoted string",
                ));
            };
            if message.trim().is_empty() {
                return Err(CompileError::new(
                    token.span,
                    "permission message cannot be empty",
                ));
            }
            permissions.push(PermissionConfig {
                name,
                message,
                span,
            });
            if self.take(&Kind::Comma) {
                continue;
            }
            self.optional_semicolon();
            if !self.check(&Kind::RBrace) {
                return self.error_here("expected `,` or `}` after permission message");
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close config permissions")?;
        self.optional_semicolon();
        Ok(permissions)
    }

    fn config_plugins_decl(&mut self) -> Result<Vec<PluginConfigDecl>, CompileError> {
        self.expect_word("plugins")?;
        self.expect(Kind::LBrace, "expected `{` after `plugins`")?;
        let mut plugins = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if plugins
                .iter()
                .any(|plugin: &PluginConfigDecl| plugin.name == name)
            {
                return Err(CompileError::new(
                    span,
                    format!("plugin `{name}` is declared more than once"),
                ));
            }
            self.expect(Kind::LBrace, "expected `{` after plugin name")?;
            let mut options = Vec::new();
            while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
                let (option_name, option_span) = self.ident()?;
                if options
                    .iter()
                    .any(|option: &PluginOptionDecl| option.name == option_name)
                {
                    return Err(CompileError::new(
                        option_span,
                        format!("plugin option `{option_name}` is declared more than once"),
                    ));
                }
                self.expect(Kind::Colon, "expected `:` after plugin option name")?;
                let value = self.config_value()?;
                options.push(PluginOptionDecl {
                    name: option_name,
                    value,
                    span: option_span,
                });
                if self.take(&Kind::Comma) {
                    continue;
                }
                self.optional_semicolon();
                if !self.check(&Kind::RBrace) {
                    return self.error_here("expected `,` or `}` after plugin option");
                }
            }
            self.expect(Kind::RBrace, "expected `}` to close plugin configuration")?;
            self.optional_semicolon();
            plugins.push(PluginConfigDecl {
                name,
                options,
                span,
            });
        }
        self.expect(Kind::RBrace, "expected `}` to close config plugins")?;
        self.optional_semicolon();
        Ok(plugins)
    }

    fn config_value(&mut self) -> Result<ConfigValue, CompileError> {
        let token = self.advance().clone();
        match token.kind {
            Kind::String(value) => Ok(ConfigValue::String(value)),
            Kind::Number(value) => Ok(ConfigValue::Number(value)),
            Kind::Minus => {
                let number = self.advance().clone();
                let Kind::Number(value) = number.kind else {
                    return Err(CompileError::new(
                        number.span,
                        "expected a number after `-` in plugin option",
                    ));
                };
                Ok(ConfigValue::Number(format!("-{value}")))
            }
            Kind::Ident(value) if value == "true" => Ok(ConfigValue::Bool(true)),
            Kind::Ident(value) if value == "false" => Ok(ConfigValue::Bool(false)),
            Kind::Ident(value) if value == "null" => Ok(ConfigValue::Null),
            Kind::LBracket => {
                let mut values = Vec::new();
                while !self.check(&Kind::RBracket) && !self.check(&Kind::Eof) {
                    values.push(self.config_value()?);
                    if self.take(&Kind::Comma) {
                        continue;
                    }
                    if !self.check(&Kind::RBracket) {
                        return self.error_here("expected `,` or `]` in plugin option array");
                    }
                }
                self.expect(Kind::RBracket, "expected `]` after plugin option array")?;
                Ok(ConfigValue::Array(values))
            }
            _ => Err(CompileError::new(
                token.span,
                "plugin option value must be a string, number, Boolean, null, or array",
            )),
        }
    }

    fn program(mut self) -> Result<Program, CompileError> {
        let mut imports = Vec::new();
        let mut plugins = Vec::new();
        let mut components = Vec::new();
        let mut structs = Vec::new();
        let mut functions = Vec::new();
        let mut app = None;
        while !self.check(&Kind::Eof) {
            if self.word_is("import") {
                imports.push(self.import_decl()?);
            } else if self.word_is("plugin") {
                plugins.push(self.plugin_decl()?);
            } else if self.word_is("struct") {
                structs.push(self.struct_decl()?);
            } else if self.word_is("component") {
                components.push(self.component_decl()?);
            } else if self.word_is("async") {
                self.advance();
                self.expect_word("fn")?;
                functions.push(self.function_decl(true)?);
            } else if self.word_is("fn") {
                functions.push(self.function_decl(false)?);
            } else if self.word_is("app") {
                if app.is_some() {
                    return self.error_here("a source file can only declare one `app`");
                }
                app = Some(self.app_decl()?);
            } else {
                return self.error_here(
                    "expected an `import`, `plugin`, `struct`, `component`, `fn`, `async fn`, or `app` declaration",
                );
            }
        }
        Ok(Program {
            imports,
            plugins,
            components,
            structs,
            functions,
            app,
        })
    }

    fn struct_decl(&mut self) -> Result<StructDecl, CompileError> {
        let span = self.advance().span;
        let (name, _) = self.ident()?;
        self.expect(Kind::LBrace, "expected `{` after struct name")?;
        let mut fields = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            let (field_name, field_span) = self.ident()?;
            if fields
                .iter()
                .any(|field: &StructFieldDecl| field.name == field_name)
            {
                return Err(CompileError::new(
                    field_span,
                    format!("struct field `{field_name}` is declared more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after struct field name")?;
            let ty = self.type_syntax()?;
            fields.push(StructFieldDecl {
                name: field_name,
                ty,
                span: field_span,
            });
            if self.take(&Kind::Comma) {
                continue;
            }
            self.optional_semicolon();
            if !self.check(&Kind::RBrace) {
                return self.error_here("expected `,` or `}` after struct field");
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close struct")?;
        self.optional_semicolon();
        Ok(StructDecl {
            name,
            fields,
            span,
            source_file: None,
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

    fn plugin_decl(&mut self) -> Result<PluginDecl, CompileError> {
        let keyword = self.advance().span;
        let token = self.advance().clone();
        let Kind::String(path) = token.kind else {
            return Err(CompileError::new(
                token.span,
                "a plugin path must be a quoted directory or IDL path",
            ));
        };
        if path.is_empty() {
            return Err(CompileError::new(
                token.span,
                "a plugin path cannot be empty",
            ));
        }
        self.expect_word("as")?;
        let (namespace, _) = self.ident()?;
        self.optional_semicolon();
        Ok(PluginDecl {
            path,
            namespace,
            span: keyword,
            idl: None,
            pure: false,
            assets_path: None,
            ios_sources: Vec::new(),
            android_sources: Vec::new(),
            cpp_sources: Vec::new(),
            cpp_headers: Vec::new(),
            cpp_standard: None,
            ios_min_version: None,
            android_min_sdk: None,
            ios_frameworks: Vec::new(),
            ios_xcframeworks: Vec::new(),
            ios_resources: Vec::new(),
            ios_privacy_manifest: None,
            swift_packages: Vec::new(),
            maven_dependencies: Vec::new(),
            android_aars: Vec::new(),
            android_resources: Vec::new(),
            android_proguard_rules: Vec::new(),
            android_maven_repositories: Vec::new(),
            ios_usage_descriptions: Vec::new(),
            ios_entitlements: Vec::new(),
            ios_linker_flags: Vec::new(),
            android_permissions: Vec::new(),
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
                    "expected an `enum`, `state`, `fn`, `async fn`, `screen`, `theme`, or `body` declaration",
                );
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close app")?;
        let body = body.ok_or_else(|| CompileError::new(span, "app is missing a `body` block"))?;
        Ok(App {
            name,
            plugins: Vec::new(),
            enums,
            structs: Vec::new(),
            states,
            functions,
            screens,
            theme,
            components: Vec::new(),
            body,
            span,
        })
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
        let mut parameters = Vec::new();
        if self.take(&Kind::LParen) {
            while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
                let (parameter_name, parameter_span) = self.ident()?;
                if parameters
                    .iter()
                    .any(|parameter: &FunctionParameter| parameter.name == parameter_name)
                {
                    return Err(CompileError::new(
                        parameter_span,
                        format!("screen parameter `{parameter_name}` is declared more than once"),
                    ));
                }
                self.expect(Kind::Colon, "expected `:` after screen parameter name")?;
                let ty = self.type_syntax()?;
                parameters.push(FunctionParameter {
                    name: parameter_name,
                    ty,
                    span: parameter_span,
                });
                if !self.take(&Kind::Comma) {
                    break;
                }
            }
            self.expect(Kind::RParen, "expected `)` after screen parameters")?;
        }
        self.expect(Kind::LBrace, "expected `{` after screen name")?;
        let mut states = Vec::new();
        let mut body = Vec::new();
        let mut body_started = false;
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            if self.word_is("state") || self.word_is("let") {
                if body_started {
                    return self.error_here(
                        "screen state and let declarations must appear before screen UI nodes",
                    );
                }
                states.push(self.state_decl()?);
            } else {
                body_started = true;
                body.push(self.node()?);
                self.optional_semicolon();
            }
        }
        self.expect(Kind::RBrace, "expected `}` to close screen")?;
        Ok(ScreenDecl {
            name,
            parameters,
            states,
            body,
            span,
        })
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
        let mut qualified_name = name;
        let mut end = span.end;
        while self.take(&Kind::Dot) {
            let (part, part_span) = self.ident()?;
            qualified_name.push('.');
            qualified_name.push_str(&part);
            end = part_span.end;
        }
        let mut syntax = if !self.take(&Kind::Less) {
            TypeSyntax::Named(qualified_name.clone(), Span { end, ..span })
        } else {
            let mut arguments = vec![self.type_syntax()?];
            while self.take(&Kind::Comma) {
                arguments.push(self.type_syntax()?);
            }
            self.expect(
                Kind::Greater,
                "expected `>` to close generic type arguments",
            )?;
            TypeSyntax::Generic(qualified_name, arguments, Span { end, ..span })
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
                    "AppBottomBar accepts only `Tab(index: ..., label: ..., icon: ..., badge: ...)` entries",
                ));
            }
            let mut args = self.named_args(&["index", "label", "icon", "badge"])?;
            let index = self.required_arg(&mut args, "index", "Tab requires `index`")?;
            let label = self.required_arg(&mut args, "label", "Tab requires `label`")?;
            let icon = args.remove("icon");
            let badge = args.remove("badge");
            let children = self.block_nodes()?;
            tabs.push(TabDecl {
                index,
                label,
                icon,
                badge,
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
        // Plugin visual components use an explicit namespace so their export
        // is deterministic even when two packages expose the same type name.
        // Keep built-in and ordinary custom component parsing unchanged.
        if self.check(&Kind::Dot) {
            let saved = self.cursor;
            self.advance();
            if let Ok((component_name, component_span)) = self.ident() {
                if self.check(&Kind::LParen) || self.check(&Kind::LBrace) {
                    let arguments = if self.check(&Kind::LParen) {
                        self.named_args_any()?
                    } else {
                        BTreeMap::new()
                    };
                    let children = self
                        .check(&Kind::LBrace)
                        .then(|| self.block_nodes())
                        .transpose()?;
                    let mut event_handlers = Vec::new();
                    while self.take(&Kind::Dot) {
                        let (property, property_span) = self.ident()?;
                        if !property.starts_with("on") || property.len() <= 2 {
                            return Err(CompileError::new(
                                property_span,
                                "native component modifiers must name an event callback such as `.onTapped { ... }`",
                            ));
                        }
                        if event_handlers
                            .iter()
                            .any(|handler: &NativeComponentEventHandler| {
                                handler.property == property
                            })
                        {
                            return Err(CompileError::new(
                                property_span,
                                format!(
                                    "native component callback `{property}` is subscribed more than once"
                                ),
                            ));
                        }
                        let (parameters, actions) = self.native_event_handler()?;
                        event_handlers.push(NativeComponentEventHandler {
                            property,
                            parameters,
                            actions,
                            span: property_span,
                        });
                    }
                    return Ok(Node::NativeComponentCall {
                        namespace: name,
                        name: component_name,
                        arguments,
                        children,
                        event_handlers,
                        span: Span {
                            end: component_span.end,
                            ..span
                        },
                    });
                }
            }
            self.cursor = saved;
        }
        if name == "platform" {
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
            return Ok(Node::Platform {
                target,
                children: self.block_nodes()?,
                span,
            });
        }
        if let Some(schema) = catalog::component_schema(&name) {
            return self.component_invocation(name, span, schema);
        }
        if self.check(&Kind::LParen) {
            let arguments = self.named_args_any()?;
            let children = self
                .check(&Kind::LBrace)
                .then(|| self.block_nodes())
                .transpose()?;
            return Ok(Node::ComponentCall {
                name,
                arguments,
                children,
                span,
            });
        }
        if self.check(&Kind::LBrace) {
            return Ok(Node::ComponentCall {
                name,
                arguments: BTreeMap::new(),
                children: Some(self.block_nodes()?),
                span,
            });
        }
        Err(CompileError::new(
            span,
            format!("unknown component `{name}`; custom components must use `Name(...)` syntax"),
        ))
    }

    fn named_args(&mut self, allowed: &[&str]) -> Result<BTreeMap<String, Expr>, CompileError> {
        self.expect(Kind::LParen, "expected `(` before component options")?;
        let args = self.named_args_contents(allowed)?;
        self.expect(Kind::RParen, "expected `)` after component options")?;
        Ok(args)
    }

    /// Parse one built-in component invocation against its catalog schema:
    /// flags, parenthesized head, child block, and trailing dot-modifiers.
    /// The result is a generic syntactic invocation; semantic lowering
    /// converts it into the existing typed IR nodes.
    fn component_invocation(
        &mut self,
        name: String,
        span: Span,
        schema: &'static catalog::ComponentSchema,
    ) -> Result<Node, CompileError> {
        let mut flags = Vec::new();
        for flag in schema.flags {
            if self.word_is(flag) {
                self.advance();
                flags.push((*flag).to_owned());
            }
        }
        let mut positional = Vec::new();
        let mut arguments = BTreeMap::new();
        let mut list_source: Option<(ListSource, Option<ListKey>)> = None;
        match schema.positional {
            PositionalModel::None => match schema.parens {
                ParensModel::None => {}
                ParensModel::Optional => {
                    if self.check(&Kind::LParen) {
                        arguments = self.named_args(&catalog::schema_option_names(schema))?;
                    }
                }
                ParensModel::Required => {
                    arguments = self.named_args(&catalog::schema_option_names(schema))?;
                }
                ParensModel::Empty => {
                    self.expect(Kind::LParen, &format!("expected `(` after {}", schema.name))?;
                    self.expect(
                        Kind::RParen,
                        &format!("{} does not accept arguments", schema.name),
                    )?;
                }
            },
            PositionalModel::Single => {
                self.expect(Kind::LParen, &format!("expected `(` after {}", schema.name))?;
                positional.push(self.expr()?);
                if self.take(&Kind::Comma) {
                    arguments = self.named_args_contents(&catalog::schema_option_names(schema))?;
                }
                self.expect(
                    Kind::RParen,
                    &format!("expected `)` after {} options", schema.name),
                )?;
            }
            PositionalModel::ListSource => {
                let (source, args, key) = self.list_source_and_args(span, schema)?;
                arguments = args;
                list_source = Some((source, key));
            }
        }
        for group in schema.exclusive {
            let present = group
                .options
                .iter()
                .filter(|option| arguments.contains_key(**option))
                .count();
            if present == 0 {
                return self.error_here(group.missing_message);
            }
            if present > 1 {
                return Err(CompileError::new(span, group.both_message));
            }
        }
        for arg in schema.arguments.iter().filter(|arg| arg.required) {
            if !arguments.contains_key(arg.name) {
                return Err(CompileError::new(
                    self.peek().span,
                    catalog::required_message(schema, arg),
                ));
            }
        }
        let children = match schema.children {
            ChildModel::None => ChildBody::None,
            ChildModel::Nodes => ChildBody::Nodes(self.block_nodes()?),
            ChildModel::OptionalActions => {
                if self.check(&Kind::LBrace) {
                    ChildBody::Actions(self.block_stmts()?)
                } else {
                    ChildBody::Actions(Vec::new())
                }
            }
            ChildModel::RequiredActions => ChildBody::Actions(self.block_stmts()?),
            ChildModel::Tabs => ChildBody::Tabs(self.tab_declarations()?),
            ChildModel::ListRows => {
                let (source, key) =
                    list_source.expect("ListSource positional model parses a list source");
                let (item, index, section, children) = self.list_row_block(&source, span)?;
                ChildBody::Rows(ListRows {
                    source,
                    key,
                    item,
                    index,
                    section,
                    children,
                })
            }
        };
        let mut modifiers = Vec::new();
        while self.take(&Kind::Dot) {
            let (modifier, modifier_span) = self.ident()?;
            let Some(spec) = schema.modifiers.iter().find(|spec| spec.name == modifier) else {
                return Err(CompileError::new(
                    modifier_span,
                    catalog::unknown_modifier_message(schema, &modifier),
                ));
            };
            if modifiers
                .iter()
                .any(|existing: &DotModifier| existing.name == modifier)
            {
                return Err(CompileError::new(
                    modifier_span,
                    catalog::duplicate_modifier_message(schema, &modifier),
                ));
            }
            let body = match spec.body {
                catalog::ModifierBody::Actions => ModifierBody::Actions(self.block_stmts()?),
                catalog::ModifierBody::Nodes => ModifierBody::Nodes(self.block_nodes()?),
            };
            modifiers.push(DotModifier {
                name: modifier,
                span: modifier_span,
                body,
            });
        }
        if let Some(message) = schema.trailing_message {
            if self.check(&Kind::LBrace)
                || schema.modifiers.iter().any(|spec| self.word_is(spec.name))
            {
                return self.error_here(message);
            }
        }
        for spec in schema.modifiers.iter().filter(|spec| spec.required) {
            if !modifiers.iter().any(|modifier| modifier.name == spec.name) {
                return Err(CompileError::new(
                    span,
                    catalog::required_modifier_message(schema, spec.name),
                ));
            }
        }
        Ok(Node::ComponentInvocation(ComponentInvocation {
            name,
            span,
            positional,
            arguments,
            flags,
            children,
            modifiers,
        }))
    }

    fn list_source_and_args(
        &mut self,
        span: Span,
        schema: &'static catalog::ComponentSchema,
    ) -> Result<(ListSource, BTreeMap<String, Expr>, Option<ListKey>), CompileError> {
        self.expect(Kind::LParen, "expected `(` after FastList")?;
        // Allowed words come from the catalog schema plus the catalogued
        // source keys; the source-exclusivity diagnostics below stay
        // handwritten because they describe grammar structure, not names.
        let mut named_form: Vec<&str> = catalog::FASTLIST_SOURCE_KEYS.to_vec();
        named_form.extend(catalog::schema_option_names(schema));
        named_form.push(catalog::FASTLIST_KEY_OPTION);
        let mut option_form: Vec<&str> = catalog::schema_option_names(schema);
        option_form.push(catalog::FASTLIST_KEY_OPTION);
        let (source, args, key) = if self.next_is_named_argument() {
            let mut args = BTreeMap::new();
            let mut key = None;
            self.list_named_arguments(&mut args, &mut key, &named_form)?;
            let count = args.remove("count");
            let sections = args.remove("sections");
            let source = match (count, sections) {
                (Some(count), None) => ListSource::Count(count),
                (None, Some(sections)) => ListSource::Sections(sections),
                (None, None) => {
                    return self.error_here(
                        "FastList requires a positional collection, `count:`, or `sections:`",
                    );
                }
                (Some(_), Some(_)) => {
                    return Err(CompileError::new(
                        span,
                        "FastList accepts exactly one source: a positional collection, `count:`, or `sections:`",
                    ));
                }
            };
            (source, args, key)
        } else {
            let collection = self.expr()?;
            let mut args = BTreeMap::new();
            let mut key = None;
            if self.take(&Kind::Comma) {
                self.list_named_arguments(&mut args, &mut key, &option_form)?;
            }
            (ListSource::Items(collection), args, key)
        };
        self.expect(Kind::RParen, "expected `)` after FastList options")?;
        Ok((source, args, key))
    }

    fn list_named_arguments(
        &mut self,
        args: &mut BTreeMap<String, Expr>,
        key: &mut Option<ListKey>,
        allowed: &[&str],
    ) -> Result<(), CompileError> {
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            let (name, name_span) = self.ident()?;
            if !allowed.contains(&name.as_str()) {
                return Err(CompileError::new(
                    name_span,
                    format!(
                        "unknown FastList option `{name}`; use a positional collection and explicit row bindings"
                    ),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after FastList option name")?;
            if name == "key" {
                if key.is_some() {
                    return Err(CompileError::new(
                        name_span,
                        "FastList option `key` was provided more than once",
                    ));
                }
                *key = Some(self.parse_list_key()?);
            } else {
                if args.contains_key(&name) {
                    return Err(CompileError::new(
                        name_span,
                        format!("FastList option `{name}` was provided more than once"),
                    ));
                }
                args.insert(name, self.expr()?);
            }
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        Ok(())
    }

    fn parse_list_key(&mut self) -> Result<ListKey, CompileError> {
        let dot = self.expect(
            Kind::Dot,
            "FastList `key` must be `.self` or a member key path such as `.id`",
        )?;
        let (name, name_span) = self.ident()?;
        let span = Span {
            end: name_span.end,
            ..dot.span
        };
        if name == "self" {
            Ok(ListKey::SelfValue(span))
        } else {
            Ok(ListKey::Member { name, span })
        }
    }

    fn list_row_block(
        &mut self,
        source: &ListSource,
        span: Span,
    ) -> Result<(Option<Expr>, Option<Expr>, Option<Expr>, Vec<Node>), CompileError> {
        self.expect(Kind::LBrace, "expected `{` before FastList row bindings")?;
        let mut bindings = Vec::new();
        while !self.word_is("in") {
            let (name, name_span) = self.ident()?;
            if bindings.iter().any(|(existing, _)| existing == &name) {
                return Err(CompileError::new(
                    name_span,
                    format!("FastList row binding `{name}` is declared more than once"),
                ));
            }
            bindings.push((name, name_span));
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect_word("in")?;
        let mut children = Vec::new();
        while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
            children.push(self.node()?);
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` after FastList row body")?;
        let expected = match source {
            ListSource::Count(_) => 1,
            ListSource::Items(_) => 2,
            ListSource::Sections(_) => 3,
        };
        if bindings.len() != expected {
            return Err(CompileError::new(
                span,
                format!(
                    "FastList row closure requires {expected} bindings for this source; use `{}`",
                    match source {
                        ListSource::Count(_) => "index in",
                        ListSource::Items(_) => "item, index in",
                        ListSource::Sections(_) => "item, index, section in",
                    }
                ),
            ));
        }
        let binding = |position: usize| {
            let (name, span) = &bindings[position];
            Some(Expr::Name(name.clone(), *span))
        };
        Ok(match source {
            ListSource::Count(_) => (None, binding(0), None, children),
            ListSource::Items(_) => (binding(0), binding(1), None, children),
            ListSource::Sections(_) => (binding(0), binding(1), binding(2), children),
        })
    }

    fn next_is_named_argument(&self) -> bool {
        matches!(&self.peek().kind, Kind::Ident(_))
            && matches!(
                self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                Some(Kind::Colon)
            )
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
            if !self.next_is_named_argument() {
                return self.error_here(
                    "component options must use `name: value` syntax; positional values are only allowed in the documented primary-value position",
                );
            }
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
        self.statements_after_open(allow_return)
    }

    fn statements_after_open(&mut self, allow_return: bool) -> Result<Vec<Stmt>, CompileError> {
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
            if self.word_is("await") {
                let span = self.peek().span;
                let expression = self.expr()?;
                stmts.push(Stmt::Expression { expression, span });
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
            if self.word_is("try") {
                let span = self.advance().span;
                let body = self.statements(allow_return)?;
                if !self.word_is("catch") {
                    return self.error_here("a `try` block requires a `catch` recovery block");
                }
                self.advance();
                self.expect(Kind::LBrace, "expected `{` to open catch recovery")?;
                let (error_catches, catch_body, catch_close_consumed) = if self.word_is("case")
                    || self.word_is("else")
                {
                    let mut error_catches = Vec::new();
                    let mut catch_body = None;
                    while !self.check(&Kind::RBrace) && !self.check(&Kind::Eof) {
                        if self.word_is("case") {
                            if catch_body.is_some() {
                                return self.error_here("typed catch cases must precede `else`");
                            }
                            error_catches.push(self.error_catch_arm()?);
                        } else if self.word_is("else") {
                            if catch_body.is_some() {
                                return self
                                    .error_here("a catch block can declare only one `else`");
                            }
                            self.advance();
                            catch_body = Some(self.block_stmts()?);
                        } else {
                            return self.error_here("expected a typed `case` or catch-all `else`");
                        }
                        self.optional_semicolon();
                    }
                    if error_catches.is_empty() {
                        return self
                            .error_here("typed catch recovery requires at least one `case`");
                    }
                    (error_catches, catch_body, false)
                } else {
                    (
                        Vec::new(),
                        Some(self.statements_after_open(allow_return)?),
                        true,
                    )
                };
                if !catch_close_consumed {
                    self.expect(Kind::RBrace, "expected `}` to close catch recovery")?;
                }
                stmts.push(Stmt::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                    span,
                });
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
            if self.take(&Kind::Equal) {
                let value = self.expr()?;
                stmts.push(Stmt::Assign { name, value, span });
            } else if name.chars().next().is_some_and(char::is_uppercase)
                && self.check(&Kind::Dot)
                && matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(Kind::Ident(_))
                )
                && matches!(
                    self.tokens.get(self.cursor + 2).map(|token| &token.kind),
                    Some(Kind::LParen)
                )
            {
                self.cursor -= 1;
                let expression = self.expr()?;
                stmts.push(Stmt::Expression { expression, span });
            } else {
                self.expect(Kind::Dot, "expected `=` or `.` after state name")?;
                let (method, method_span) = self.ident()?;
                if self.check(&Kind::LBrace) {
                    let (parameters, actions) = self.native_event_handler()?;
                    stmts.push(Stmt::NativeEventSubscribe {
                        receiver: Expr::Name(name, span),
                        event: method,
                        parameters,
                        actions,
                        span: Span {
                            end: method_span.end,
                            ..span
                        },
                    });
                    self.optional_semicolon();
                    continue;
                }
                if self.take(&Kind::Equal) {
                    let value = self.expr()?;
                    stmts.push(Stmt::NativePropertyAssign {
                        receiver: Expr::Name(name, span),
                        property: method,
                        value,
                        span: Span {
                            end: method_span.end,
                            ..span
                        },
                    });
                    self.optional_semicolon();
                    continue;
                }
                self.expect(
                    Kind::LParen,
                    "expected `(` after collection mutation method",
                )?;
                let mut arguments = Vec::new();
                if !self.check(&Kind::RParen) {
                    arguments.push(self.expr()?);
                    while self.take(&Kind::Comma) {
                        arguments.push(self.expr()?);
                    }
                }
                self.expect(
                    Kind::RParen,
                    "expected `)` after collection mutation arguments",
                )?;
                let span = Span {
                    end: method_span.end,
                    ..span
                };
                stmts.push(Stmt::CollectionMutation {
                    name,
                    method,
                    arguments,
                    span,
                });
            }
            self.optional_semicolon();
        }
        self.expect(Kind::RBrace, "expected `}` to close button handler")?;
        Ok(stmts)
    }

    fn error_catch_arm(&mut self) -> Result<ErrorCatchArm, CompileError> {
        let span = self.advance().span;
        let (namespace, _) = self.ident()?;
        self.expect(Kind::Dot, "expected `.` after catch plugin alias")?;
        let (error_name, _) = self.ident()?;
        self.expect(Kind::Dot, "expected `.` after catch error type")?;
        let (variant, _) = self.ident()?;
        let mut bindings = Vec::new();
        if self.take(&Kind::LParen) {
            if !self.check(&Kind::RParen) {
                loop {
                    bindings.push(self.ident()?.0);
                    if !self.take(&Kind::Comma) {
                        break;
                    }
                }
            }
            self.expect(Kind::RParen, "expected `)` after catch payload bindings")?;
        }
        let body = self.block_stmts()?;
        Ok(ErrorCatchArm {
            namespace,
            error_name,
            variant,
            bindings,
            body,
            span,
        })
    }

    fn native_event_handler(&mut self) -> Result<(Vec<String>, Vec<Stmt>), CompileError> {
        self.expect(Kind::LBrace, "expected `{` to open native event handler")?;
        let parameter_start = self.cursor;
        let mut parameters = Vec::new();
        if matches!(&self.peek().kind, Kind::Ident(_)) {
            loop {
                parameters.push(self.ident()?.0);
                if !self.take(&Kind::Comma) {
                    break;
                }
            }
        }
        let has_arrow =
            !parameters.is_empty() && self.take(&Kind::Minus) && self.take(&Kind::Greater);
        if !has_arrow {
            self.cursor = parameter_start;
            parameters.clear();
        }
        let actions = self.statements_after_open(false)?;
        Ok((parameters, actions))
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
                if self.take(&Kind::LParen) {
                    let (mut arguments, named_arguments) =
                        self.call_arguments_after_open_with_names()?;
                    if self.check(&Kind::LBrace) {
                        arguments.push(self.closure_expression()?);
                    }
                    expression = Expr::MethodCall {
                        base: Box::new(expression),
                        name,
                        arguments,
                        named_arguments,
                        span,
                    };
                } else if self.check(&Kind::LBrace) && matches!(name.as_str(), "map" | "filter") {
                    expression = Expr::MethodCall {
                        base: Box::new(expression),
                        name,
                        arguments: vec![self.closure_expression()?],
                        named_arguments: BTreeMap::new(),
                        span,
                    };
                } else {
                    expression = Expr::Member {
                        base: Box::new(expression),
                        name,
                        optional: false,
                        span,
                    };
                }
            } else if self.take(&Kind::Question) {
                let question_span = self.previous_span();
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
                } else if self.take(&Kind::Dot) {
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
                } else {
                    let span = Span {
                        end: question_span.end,
                        ..expression.span()
                    };
                    expression = Expr::Try {
                        expr: Box::new(expression),
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
                if (value.chars().next().is_some_and(char::is_uppercase)
                    || matches!(value.as_str(), "Network" | "Path" | "File" | "Permissions"))
                    && self.check(&Kind::Dot)
                    && matches!(
                        self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                        Some(Kind::Ident(_))
                    )
                    && matches!(
                        self.tokens.get(self.cursor + 2).map(|token| &token.kind),
                        Some(Kind::LParen)
                    )
                {
                    self.advance();
                    let (name, name_span) = self.ident()?;
                    let span = Span {
                        end: name_span.end,
                        ..token.span
                    };
                    return self.qualified_call(value, name, span);
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
                        "Layout" if name == "isCompactWidth" => Ok(Expr::IsCompactWidth(span)),
                        "Layout" if name == "isRegularHeight" => Ok(Expr::IsRegularHeight(span)),
                        "Layout" if name == "isCompactHeight" => Ok(Expr::IsCompactHeight(span)),
                        "Layout" => Err(CompileError::new(
                            name_span,
                            format!(
                                "unknown layout property `{name}`; expected `isRegularWidth`, `isCompactWidth`, `isRegularHeight`, or `isCompactHeight`"
                            ),
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
        let arguments = self.call_arguments_after_open()?;
        Ok(Expr::Call(name, arguments, span))
    }

    fn call_arguments_after_open(&mut self) -> Result<Vec<Expr>, CompileError> {
        let mut arguments = Vec::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            arguments.push(self.expr()?);
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after function arguments")?;
        Ok(arguments)
    }

    fn call_arguments_after_open_with_names(
        &mut self,
    ) -> Result<(Vec<Expr>, BTreeMap<String, Expr>), CompileError> {
        let named = matches!(
            (
                &self.peek().kind,
                self.tokens.get(self.cursor + 1).map(|token| &token.kind)
            ),
            (Kind::Ident(_), Some(Kind::Colon))
        );
        if !named {
            return Ok((self.call_arguments_after_open()?, BTreeMap::new()));
        }
        let mut arguments = BTreeMap::new();
        while !self.check(&Kind::RParen) && !self.check(&Kind::Eof) {
            let (name, span) = self.ident()?;
            if arguments.contains_key(&name) {
                return Err(CompileError::new(
                    span,
                    format!("argument `{name}` was provided more than once"),
                ));
            }
            self.expect(Kind::Colon, "expected `:` after argument name")?;
            arguments.insert(name, self.expr()?);
            if !self.take(&Kind::Comma) {
                break;
            }
        }
        self.expect(Kind::RParen, "expected `)` after call arguments")?;
        Ok((Vec::new(), arguments))
    }

    fn closure_expression(&mut self) -> Result<Expr, CompileError> {
        let span = self.expect(Kind::LBrace, "expected a closure body")?.span;
        let mut parameters = Vec::new();
        if !self.check(&Kind::Minus) {
            parameters.push(self.ident()?.0);
            while self.take(&Kind::Comma) {
                parameters.push(self.ident()?.0);
            }
        }
        self.expect(Kind::Minus, "expected an arrow after closure parameters")?;
        self.expect(Kind::Greater, "expected an arrow after closure parameters")?;
        let body = self.expr()?;
        self.expect(Kind::RBrace, "expected a closing brace for closure")?;
        Ok(Expr::Closure {
            parameters,
            body: Box::new(body),
            span,
        })
    }

    fn qualified_call(
        &mut self,
        namespace: String,
        name: String,
        span: Span,
    ) -> Result<Expr, CompileError> {
        self.expect(Kind::LParen, "expected `(` after qualified function name")?;
        let (arguments, named_arguments) = self.call_arguments_after_open_with_names()?;
        Ok(Expr::QualifiedCall {
            namespace,
            name,
            arguments,
            named_arguments,
            span,
        })
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
        let inclusive = if self.take(&Kind::DotDot) {
            Some(true)
        } else if self.take(&Kind::DotDotLess) {
            Some(false)
        } else {
            None
        };
        let iterable = if let Some(inclusive) = inclusive {
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
                inclusive,
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
    fn previous_span(&self) -> Span {
        if self.cursor > 0 {
            self.tokens[self.cursor - 1].span
        } else {
            self.peek().span
        }
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
