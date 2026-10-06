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
    /// Optional sources compiled into the generated native host projects.
    pub cpp: CppManifest,
    pub assets: Vec<String>,
    /// Optional host-side compiler analyzer supplied by this plugin.
    pub compiler: Option<CompilerManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerManifest {
    /// Executable and arguments, launched without a shell from the plugin
    /// package root. A relative executable is resolved against that root.
    pub analyzer: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CppManifest {
    /// Minimum ISO C++ standard required by the plugin's implementation.
    pub standard: Option<u8>,
    /// C++ implementation sources, relative to the plugin package.
    pub sources: Vec<String>,
    /// C++ headers, relative to the plugin package.
    pub headers: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlatformManifest {
    pub min_version: Option<String>,
    pub min_sdk: Option<u32>,
    pub sources: Vec<String>,
    /// iOS implementation files that are safe to compile into app-extension
    /// targets. This is an explicit subset/selection, separate from app
    /// sources because extension targets have stricter API availability.
    pub extension_sources: Vec<String>,
    /// iOS system frameworks linked by the generated Xcode target.
    pub frameworks: Vec<String>,
    /// Local iOS XCFramework bundles copied into the generated Xcode project.
    pub xcframeworks: Vec<String>,
    pub swift_packages: Vec<SwiftPackage>,
    /// Local Android AAR files copied into the generated Gradle project.
    pub aars: Vec<String>,
    /// Platform resources bundled with the generated native host.
    pub resources: Vec<String>,
    /// iOS SDK privacy manifest copied into a plugin-owned resource bundle.
    pub privacy_manifest: Option<String>,
    /// Additional Android shrinker rules required by the plugin's native SDK.
    pub proguard_rules: Vec<String>,
    pub maven_dependencies: Vec<String>,
    /// Additional HTTPS Maven repositories for Android dependencies.
    pub repositories: Vec<String>,
    /// iOS Info.plist purpose strings keyed by native usage-description key.
    pub usage_descriptions: Vec<(String, String)>,
    /// iOS code-signing entitlements requested by this plugin.
    pub entitlements: Vec<(String, EntitlementValue)>,
    /// SwiftUI host app-delegate type supplied by this plugin.
    pub application_delegate: Option<String>,
    /// iOS background modes required by this plugin.
    pub background_modes: Vec<String>,
    /// Additional arguments passed to the iOS linker, each as one argument.
    pub linker_flags: Vec<String>,
    /// Android manifest permissions required by the plugin.
    pub permissions: Vec<String>,
    /// Android `<application>` metadata entries contributed by the plugin.
    /// Values may be literal strings or a complete `${ENV_VAR}` placeholder.
    pub application_metadata: Vec<(String, String)>,
    /// Whether an Android plugin needs the generated activity to support PiP.
    pub picture_in_picture: bool,
    /// Fully qualified media playback service declared by this plugin.
    pub media_playback_service: Option<String>,
    /// Fully qualified Firebase Cloud Messaging service declared by this plugin.
    pub firebase_messaging_service: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntitlementValue {
    String(String),
    Bool(bool),
    Strings(Vec<String>),
}

/// How a Swift package is versioned for resolution.
///
/// Exactly one requirement is declared. `from` is the usual semantic-version
/// range; `branch` and `revision` exist because a plugin's upstream may not
/// tag its releases, and an untagged upstream still has to resolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwiftPackageRequirement {
    From(String),
    Branch(String),
    Revision(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwiftPackage {
    pub url: String,
    pub requirement: SwiftPackageRequirement,
    pub products: Vec<String>,
    /// SwiftPM products linked by generated app-extension targets, never by
    /// the main application target unless also listed in `products`.
    pub extension_products: Vec<String>,
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
            cpp: CppManifest::default(),
            assets: Vec::new(),
            compiler: None,
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
                Some("cpp") => self.parse_cpp(&mut manifest.cpp)?,
                Some("compiler") => {
                    manifest.compiler = Some(self.parse_compiler()?);
                }
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

    fn parse_compiler(&mut self) -> Result<CompilerManifest, String> {
        self.expect_identifier("compiler")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut analyzer = None;
        let mut seen = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            let field = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected a compiler analyzer field"))?;
            if !seen.insert(field.clone()) {
                return Err(self.error_value(format!(
                    "compiler field `{field}` is declared more than once"
                )));
            }
            match field.as_str() {
                "analyzer" => analyzer = Some(self.parse_string_array_field("analyzer")?),
                name => return self.error(format!("unknown compiler field `{name}`")),
            }
            self.consume(TokenKind::Comma);
        }
        let analyzer = analyzer.ok_or_else(|| self.error_value("compiler requires `analyzer`"))?;
        let Some(program) = analyzer.first() else {
            return self.error("compiler analyzer command cannot be empty");
        };
        if program.is_empty() || program.contains('\0') {
            return self.error("compiler analyzer executable must be a non-empty path or command");
        }
        if program.contains('/') || program.contains('\\') || program.starts_with('.') {
            validate_relative_path(program)?;
        }
        if analyzer.iter().any(|argument| argument.contains('\0')) {
            return self.error("compiler analyzer arguments cannot contain NUL bytes");
        }
        Ok(CompilerManifest { analyzer })
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
                Some("extensionSources") if !android => {
                    platform.extension_sources =
                        self.parse_string_array_field("extensionSources")?;
                    validate_unique_paths(&platform.extension_sources, "iOS extension source")?;
                }
                Some("frameworks") if !android => {
                    platform.frameworks = self.parse_string_array_field("frameworks")?;
                    validate_ios_frameworks(&platform.frameworks)?;
                }
                Some("xcframeworks") if !android => {
                    platform.xcframeworks = self.parse_string_array_field("xcframeworks")?;
                    validate_native_artifacts(
                        &platform.xcframeworks,
                        "iOS XCFramework",
                        "xcframework",
                    )?;
                }
                Some("privacyManifest") if !android => {
                    platform.privacy_manifest = Some(self.parse_string_field("privacyManifest")?);
                }
                Some("resources") => {
                    platform.resources = self.parse_string_array_field("resources")?;
                    validate_unique_paths(&platform.resources, "platform resource")?;
                }
                Some("repositories") if android => {
                    platform.repositories = self.parse_string_array_field("repositories")?;
                    validate_maven_repositories(&platform.repositories)?;
                }
                Some("dependencies") if android => {
                    platform.maven_dependencies = self.parse_string_array_field("dependencies")?;
                    validate_maven_dependencies(&platform.maven_dependencies)?;
                }
                Some("aars") if android => {
                    platform.aars = self.parse_string_array_field("aars")?;
                    validate_native_artifacts(&platform.aars, "Android AAR", "aar")?;
                }
                Some("proguardRules") if android => {
                    platform.proguard_rules = self.parse_string_array_field("proguardRules")?;
                    validate_native_artifacts(
                        &platform.proguard_rules,
                        "Android ProGuard rule",
                        "pro",
                    )?;
                }
                Some("dependencies") => {
                    platform.swift_packages = self.parse_swift_packages()?;
                }
                Some("usageDescriptions") if !android => {
                    platform.usage_descriptions = self.parse_usage_descriptions()?;
                }
                Some("entitlements") if !android => {
                    platform.entitlements = self.parse_entitlements()?;
                    validate_ios_entitlements(&platform.entitlements)?;
                }
                Some("applicationDelegate") if !android => {
                    let delegate = self.parse_string_field("applicationDelegate")?;
                    validate_ios_application_delegate(&delegate)?;
                    platform.application_delegate = Some(delegate);
                }
                Some("backgroundModes") if !android => {
                    platform.background_modes = self.parse_string_array_field("backgroundModes")?;
                    validate_ios_background_modes(&platform.background_modes)?;
                }
                Some("linkerFlags") if !android => {
                    platform.linker_flags = self.parse_string_array_field("linkerFlags")?;
                    validate_ios_linker_flags(&platform.linker_flags)?;
                }
                Some("permissions") if android => {
                    platform.permissions = self.parse_string_array_field("permissions")?;
                    validate_android_permissions(&platform.permissions)?;
                }
                Some("applicationMetadata") if android => {
                    platform.application_metadata = self.parse_application_metadata()?;
                    validate_android_application_metadata(&platform.application_metadata)?;
                }
                Some("pictureInPicture") if android => {
                    platform.picture_in_picture = self.parse_bool_field("pictureInPicture")?;
                }
                Some("mediaPlaybackService") if android => {
                    platform.media_playback_service =
                        Some(self.parse_string_field("mediaPlaybackService")?);
                }
                Some("firebaseMessagingService") if android => {
                    let service = self.parse_string_field("firebaseMessagingService")?;
                    validate_android_class_name(&service, "Firebase messaging service")?;
                    platform.firebase_messaging_service = Some(service);
                }
                Some(name) => return self.error(format!("unknown platform field `{name}`")),
                None => return self.error("expected a platform field"),
            }
            self.consume(TokenKind::Comma);
        }
        if android && platform.min_sdk == Some(0) {
            return self.error("Android plugin minSdk must be greater than zero");
        }
        Ok(())
    }

    fn parse_cpp(&mut self, cpp: &mut CppManifest) -> Result<(), String> {
        self.expect_identifier("cpp")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut seen = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            let field = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected a C++ field"))?;
            if !seen.insert(field.clone()) {
                return Err(
                    self.error_value(format!("C++ field `{field}` is declared more than once"))
                );
            }
            match field.as_str() {
                "standard" => cpp.standard = Some(self.parse_cpp_standard_field()?),
                "sources" => cpp.sources = self.parse_string_array_field("sources")?,
                "headers" => cpp.headers = self.parse_string_array_field("headers")?,
                name => return self.error(format!("unknown C++ field `{name}`")),
            }
            self.consume(TokenKind::Comma);
        }
        Ok(())
    }

    fn parse_cpp_standard_field(&mut self) -> Result<u8, String> {
        self.expect_identifier("standard")?;
        self.expect(TokenKind::Colon, "`:`")?;
        match self.expect_string()?.as_str() {
            "c++17" => Ok(17),
            "c++20" => Ok(20),
            "c++23" => Ok(23),
            value => self.error(format!(
                "unsupported C++ standard `{value}`; expected `c++17`, `c++20`, or `c++23`"
            )),
        }
    }

    fn parse_swift_packages(&mut self) -> Result<Vec<SwiftPackage>, String> {
        self.expect_identifier("dependencies")?;
        self.expect(TokenKind::LeftBrace, "`{`")?;
        let mut packages = Vec::new();
        let mut urls = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close iOS dependencies");
            }
            self.expect_identifier("swiftPackage")?;
            self.expect(TokenKind::LeftBrace, "`{`")?;
            let mut seen = std::collections::HashSet::new();
            let mut url = None;
            let mut from = None;
            let mut branch = None;
            let mut revision = None;
            let mut products = None;
            let mut extension_products = Vec::new();
            while !self.consume(TokenKind::RightBrace) {
                let field = self
                    .peek_identifier()
                    .ok_or_else(|| self.error_value("expected a Swift package field"))?;
                if !seen.insert(field.clone()) {
                    return Err(self.error_value(format!(
                        "Swift package field `{field}` is declared more than once"
                    )));
                }
                match field.as_str() {
                    "url" => url = Some(self.parse_string_field("url")?),
                    "from" => from = Some(self.parse_string_field("from")?),
                    "branch" => branch = Some(self.parse_string_field("branch")?),
                    "revision" => revision = Some(self.parse_string_field("revision")?),
                    "products" => products = Some(self.parse_string_array_field("products")?),
                    "extensionProducts" => {
                        extension_products = self.parse_string_array_field("extensionProducts")?
                    }
                    name => return self.error(format!("unknown Swift package field `{name}`")),
                }
                self.consume(TokenKind::Comma);
            }
            let declared = [from.is_some(), branch.is_some(), revision.is_some()];
            if declared.iter().filter(|present| **present).count() != 1 {
                return Err(self.error_value(
                    "Swift package requires exactly one of `from`, `branch`, or `revision`",
                ));
            }
            let requirement = match (from, branch, revision) {
                (Some(from), None, None) => SwiftPackageRequirement::From(from),
                (None, Some(branch), None) => SwiftPackageRequirement::Branch(branch),
                (None, None, Some(revision)) => SwiftPackageRequirement::Revision(revision),
                _ => unreachable!("exactly one requirement is checked above"),
            };
            let package = SwiftPackage {
                url: url.ok_or_else(|| self.error_value("Swift package requires `url`"))?,
                requirement,
                products: products
                    .ok_or_else(|| self.error_value("Swift package requires `products`"))?,
                extension_products,
            };
            validate_swift_package(&package)?;
            if !urls.insert(package.url.clone()) {
                return self.error("an iOS Swift package URL may be declared only once");
            }
            packages.push(package);
            self.consume(TokenKind::Comma);
        }
        Ok(packages)
    }

    fn parse_usage_descriptions(&mut self) -> Result<Vec<(String, String)>, String> {
        self.expect_identifier("usageDescriptions")?;
        self.expect(TokenKind::LeftBrace, "`{")?;
        let mut values = Vec::new();
        let mut keys = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close iOS usage descriptions");
            }
            let key = self
                .peek_identifier()
                .ok_or_else(|| self.error_value("expected an Info.plist usage-description key"))?;
            if !keys.insert(key.clone()) {
                return self.error(format!(
                    "iOS usage-description key `{key}` is declared twice"
                ));
            }
            self.cursor += 1;
            self.expect(TokenKind::Colon, "`:`")?;
            values.push((key, self.expect_string()?));
            self.consume(TokenKind::Comma);
        }
        Ok(values)
    }

    fn parse_entitlements(&mut self) -> Result<Vec<(String, EntitlementValue)>, String> {
        self.expect_identifier("entitlements")?;
        self.expect(TokenKind::LeftBrace, "`{")?;
        let mut values = Vec::new();
        let mut keys = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close iOS entitlements");
            }
            let key = self.expect_string()?;
            if !keys.insert(key.clone()) {
                return self.error(format!("iOS entitlement `{key}` is declared twice"));
            }
            self.expect(TokenKind::Colon, "`:`")?;
            let value = match self.tokens.get(self.cursor).map(|token| &token.kind) {
                Some(TokenKind::String(_)) => EntitlementValue::String(self.expect_string()?),
                Some(TokenKind::Identifier(value)) if value == "true" || value == "false" => {
                    let value = value == "true";
                    self.cursor += 1;
                    EntitlementValue::Bool(value)
                }
                Some(TokenKind::LeftBracket) => {
                    self.cursor += 1;
                    let mut strings = Vec::new();
                    while !self.consume(TokenKind::RightBracket) {
                        strings.push(self.expect_string()?);
                        if !self.consume(TokenKind::Comma)
                            && !self.peek_kind(TokenKind::RightBracket)
                        {
                            return self.error("expected `,` or `]`");
                        }
                    }
                    EntitlementValue::Strings(strings)
                }
                _ => return self.error("expected a string, boolean, or string array entitlement"),
            };
            values.push((key, value));
            self.consume(TokenKind::Comma);
        }
        Ok(values)
    }

    fn parse_application_metadata(&mut self) -> Result<Vec<(String, String)>, String> {
        self.expect_identifier("applicationMetadata")?;
        self.expect(TokenKind::LeftBrace, "`{")?;
        let mut values = Vec::new();
        let mut keys = std::collections::HashSet::new();
        while !self.consume(TokenKind::RightBrace) {
            if self.at_end() {
                return self.error("expected `}` to close Android application metadata");
            }
            let key = self.expect_string()?;
            if !keys.insert(key.clone()) {
                return self.error(format!(
                    "Android application metadata key `{key}` is declared twice"
                ));
            }
            self.expect(TokenKind::Colon, "`:`")?;
            values.push((key, self.expect_string()?));
            self.consume(TokenKind::Comma);
        }
        Ok(values)
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

    fn parse_bool_field(&mut self, name: &str) -> Result<bool, String> {
        self.expect_identifier(name)?;
        self.expect(TokenKind::Colon, "`:`")?;
        match self.tokens.get(self.cursor).map(|token| &token.kind) {
            Some(TokenKind::Identifier(value)) if value == "true" || value == "false" => {
                let result = value == "true";
                self.cursor += 1;
                Ok(result)
            }
            _ => self.error("expected a boolean (`true` or `false`)"),
        }
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
            .chain(manifest.ios.extension_sources.iter())
            .chain(manifest.android.sources.iter())
            .chain(manifest.cpp.sources.iter())
            .chain(manifest.cpp.headers.iter())
            .chain(manifest.ios.xcframeworks.iter())
            .chain(manifest.android.aars.iter())
            .chain(manifest.ios.resources.iter())
            .chain(manifest.android.resources.iter())
            .chain(manifest.ios.privacy_manifest.iter())
            .chain(manifest.android.proguard_rules.iter())
        {
            validate_relative_path(path)?;
        }
        if let Some(version) = &manifest.ios.min_version {
            validate_numeric_version(version, "iOS minVersion", 2, 3)?;
        }
        validate_maven_dependencies(&manifest.android.maven_dependencies)?;
        validate_maven_repositories(&manifest.android.repositories)?;
        validate_ios_frameworks(&manifest.ios.frameworks)?;
        validate_usage_descriptions(&manifest.ios.usage_descriptions)?;
        validate_ios_entitlements(&manifest.ios.entitlements)?;
        if let Some(delegate) = &manifest.ios.application_delegate {
            validate_ios_application_delegate(delegate)?;
        }
        validate_ios_background_modes(&manifest.ios.background_modes)?;
        validate_ios_linker_flags(&manifest.ios.linker_flags)?;
        validate_android_permissions(&manifest.android.permissions)?;
        validate_android_application_metadata(&manifest.android.application_metadata)?;
        if let Some(service) = &manifest.android.firebase_messaging_service {
            validate_android_class_name(service, "Firebase messaging service")?;
        }
        if let Some(service) = &manifest.android.media_playback_service {
            validate_android_class_name(service, "Android media playback service")?;
        }
        validate_native_artifacts(&manifest.ios.xcframeworks, "iOS XCFramework", "xcframework")?;
        validate_native_artifacts(&manifest.android.aars, "Android AAR", "aar")?;
        validate_unique_paths(&manifest.ios.resources, "iOS platform resource")?;
        validate_unique_paths(&manifest.ios.extension_sources, "iOS extension source")?;
        validate_unique_paths(&manifest.android.resources, "Android platform resource")?;
        validate_unique_paths(&manifest.cpp.sources, "C++ source")?;
        validate_unique_paths(&manifest.cpp.headers, "C++ header")?;
        if let Some(path) = &manifest.ios.privacy_manifest
            && Path::new(path).file_name().and_then(|value| value.to_str())
                != Some("PrivacyInfo.xcprivacy")
        {
            return Err("iOS privacy manifest path must end in `PrivacyInfo.xcprivacy`".to_owned());
        }
        validate_native_artifacts(
            &manifest.android.proguard_rules,
            "Android ProGuard rule",
            "pro",
        )?;
        for package in &manifest.ios.swift_packages {
            validate_swift_package(package)?;
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

fn validate_swift_package(package: &SwiftPackage) -> Result<(), String> {
    if !(package.url.starts_with("https://") || package.url.starts_with("ssh://"))
        || package
            .url
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '"' | '\\'))
    {
        return Err(format!(
            "Swift package URL `{}` must use https:// or ssh:// and contain no whitespace or quoting characters",
            package.url
        ));
    }
    match &package.requirement {
        SwiftPackageRequirement::From(from) => {
            validate_numeric_version(from, "Swift package `from`", 3, 3)?
        }
        SwiftPackageRequirement::Branch(branch) => {
            validate_requirement_name(branch, "Swift package `branch`")?
        }
        SwiftPackageRequirement::Revision(revision) => {
            if revision.len() != 40
                || !revision
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                return Err(
                    "Swift package `revision` must be a full 40-character commit hash".to_owned(),
                );
            }
        }
    }
    if package.products.is_empty() {
        return Err(format!(
            "Swift package `{}` must declare at least one product",
            package.url
        ));
    }
    let mut products = std::collections::HashSet::new();
    for product in package
        .products
        .iter()
        .chain(package.extension_products.iter())
    {
        let mut characters = product.chars();
        let valid = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
        if !valid {
            return Err(format!(
                "Swift package product `{product}` must be a Swift identifier"
            ));
        }
        if !products.insert(product) {
            return Err(format!("duplicate Swift package product `{product}`"));
        }
    }
    Ok(())
}

fn validate_maven_dependencies(dependencies: &[String]) -> Result<(), String> {
    let mut artifacts = std::collections::HashSet::new();
    for dependency in dependencies {
        let parts = dependency.split(':').collect::<Vec<_>>();
        if parts.len() != 3
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.chars().all(is_maven_coordinate_character))
        {
            return Err(format!(
                "Android Maven dependency `{dependency}` must use `group:artifact:version` with non-empty coordinate parts"
            ));
        }
        let artifact = format!("{}:{}", parts[0], parts[1]);
        if !artifacts.insert(artifact.clone()) {
            return Err(format!(
                "Android Maven artifact `{artifact}` is declared more than once"
            ));
        }
    }
    Ok(())
}

fn validate_native_artifacts(paths: &[String], kind: &str, extension: &str) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        let artifact = Path::new(path);
        if artifact.extension().and_then(|value| value.to_str()) != Some(extension) {
            return Err(format!(
                "{kind} path `{path}` must have the `.{extension}` extension"
            ));
        }
        if !seen.insert(path) {
            return Err(format!("{kind} `{path}` is declared more than once"));
        }
    }
    Ok(())
}

fn validate_unique_paths(paths: &[String], kind: &str) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for path in paths {
        if path.trim().is_empty() {
            return Err(format!("{kind} path must not be empty"));
        }
        if !seen.insert(path) {
            return Err(format!("{kind} `{path}` is declared more than once"));
        }
    }
    Ok(())
}

fn validate_ios_frameworks(frameworks: &[String]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for framework in frameworks {
        let mut characters = framework.chars();
        let valid = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
        if !valid {
            return Err(format!(
                "iOS framework `{framework}` must be a system framework name"
            ));
        }
        if !seen.insert(framework) {
            return Err(format!(
                "iOS framework `{framework}` is declared more than once"
            ));
        }
    }
    Ok(())
}

fn validate_maven_repositories(repositories: &[String]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for repository in repositories {
        let Some(authority_and_path) = repository.strip_prefix("https://") else {
            return Err(format!(
                "Android Maven repository `{repository}` must use HTTPS"
            ));
        };
        let authority = authority_and_path.split('/').next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || repository
                .chars()
                .any(|character| character.is_whitespace() || matches!(character, '"' | '$' | '\\'))
            || repository.contains(['?', '#'])
        {
            return Err(format!(
                "Android Maven repository `{repository}` must be a credential-free HTTPS URL without query or fragment"
            ));
        }
        if !seen.insert(repository) {
            return Err(format!(
                "Android Maven repository `{repository}` is declared more than once"
            ));
        }
    }
    Ok(())
}

fn is_maven_coordinate_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '+')
}

fn validate_numeric_version(
    version: &str,
    field: &str,
    min_parts: usize,
    max_parts: usize,
) -> Result<(), String> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() < min_parts
        || parts.len() > max_parts
        || parts.iter().any(|part| {
            part.is_empty() || !part.chars().all(|character| character.is_ascii_digit())
        })
    {
        return Err(format!(
            "{field} must be a numeric dotted version with {min_parts} to {max_parts} parts"
        ));
    }
    Ok(())
}

/// A branch name is a git ref: no whitespace, no control characters, and no
/// traversal, so a plugin cannot smuggle a path into the project file.
fn validate_requirement_name(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('-')
        || value.contains("..")
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(format!("{field} must be a single git ref name"));
    }
    Ok(())
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

fn validate_usage_descriptions(values: &[(String, String)]) -> Result<(), String> {
    let mut keys = std::collections::HashSet::new();
    for (key, message) in values {
        if !key.starts_with("NS")
            || !key.ends_with("UsageDescription")
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            return Err(format!(
                "iOS usage-description key `{key}` must start with `NS` and end with `UsageDescription`"
            ));
        }
        if message.trim().is_empty() {
            return Err(format!(
                "iOS usage-description `{key}` must have a non-empty purpose message"
            ));
        }
        if !keys.insert(key) {
            return Err(format!(
                "iOS usage-description key `{key}` is declared twice"
            ));
        }
    }
    Ok(())
}

fn validate_ios_entitlements(values: &[(String, EntitlementValue)]) -> Result<(), String> {
    let mut keys = std::collections::HashSet::new();
    for (key, value) in values {
        let segments = key.split('.').collect::<Vec<_>>();
        if key.is_empty()
            || key.starts_with('.')
            || key.ends_with('.')
            || segments.iter().any(|segment| {
                segment.is_empty()
                    || !segment
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '-')
            })
        {
            return Err(format!(
                "iOS entitlement key `{key}` must contain only letters, digits, dots, or hyphens"
            ));
        }
        if !keys.insert(key) {
            return Err(format!("iOS entitlement `{key}` is declared twice"));
        }
        match value {
            EntitlementValue::String(value) if value.trim().is_empty() => {
                return Err(format!(
                    "iOS entitlement `{key}` must not be an empty string"
                ));
            }
            EntitlementValue::Strings(values) if values.is_empty() => {
                return Err(format!(
                    "iOS entitlement `{key}` must not be an empty array"
                ));
            }
            EntitlementValue::Strings(values)
                if values.iter().any(|value| value.trim().is_empty()) =>
            {
                return Err(format!(
                    "iOS entitlement `{key}` must not contain empty strings"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_ios_application_delegate(name: &str) -> Result<(), String> {
    let mut characters = name.chars();
    let valid = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
    if !valid {
        return Err(format!(
            "iOS application delegate `{name}` must be a Swift type name"
        ));
    }
    Ok(())
}

fn validate_ios_linker_flags(flags: &[String]) -> Result<(), String> {
    for flag in flags {
        if flag.is_empty() || flag.chars().any(char::is_control) {
            return Err(
                "iOS linker flags must be non-empty single arguments without control characters"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn validate_ios_background_modes(modes: &[String]) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for mode in modes {
        if mode != "audio" {
            return Err(format!(
                "unsupported iOS background mode `{mode}`; currently supported mode is `audio`"
            ));
        }
        if !seen.insert(mode) {
            return Err(format!("iOS background mode `{mode}` is declared twice"));
        }
    }
    Ok(())
}

fn validate_android_class_name(name: &str, kind: &str) -> Result<(), String> {
    let valid = name.split('.').count() > 1
        && name.split('.').all(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        });
    if !valid {
        return Err(format!(
            "{kind} name `{name}` must be a fully qualified Java/Kotlin class name"
        ));
    }
    Ok(())
}

fn validate_android_permissions(values: &[String]) -> Result<(), String> {
    let mut permissions = std::collections::HashSet::new();
    for permission in values {
        let valid_name = permission
            .strip_prefix("android.permission.")
            .is_some_and(|name| {
                !name.is_empty()
                    && name.split('.').all(|part| {
                        !part.is_empty()
                            && part.chars().all(|character| {
                                character.is_ascii_uppercase()
                                    || character.is_ascii_digit()
                                    || character == '_'
                            })
                    })
            });
        if !valid_name {
            return Err(format!(
                "Android permission `{permission}` must use the `android.permission.NAME` form"
            ));
        }
        if !permissions.insert(permission) {
            return Err(format!(
                "Android permission `{permission}` is declared twice"
            ));
        }
    }
    Ok(())
}

fn validate_android_application_metadata(values: &[(String, String)]) -> Result<(), String> {
    let mut keys = std::collections::HashSet::new();
    for (key, value) in values {
        let valid_key = !key.is_empty()
            && key.split('.').all(|part| {
                let mut characters = part.chars();
                characters
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                    && characters
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
            });
        if !valid_key {
            return Err(format!(
                "Android application metadata name `{key}` must be a dotted identifier"
            ));
        }
        if !keys.insert(key) {
            return Err(format!(
                "Android application metadata key `{key}` is declared twice"
            ));
        }
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(format!(
                "Android application metadata value for `{key}` must be non-empty and contain no control characters"
            ));
        }
        if value.contains("${") {
            let Some(variable) = value
                .strip_prefix("${")
                .and_then(|value| value.strip_suffix('}'))
            else {
                return Err(format!(
                    "Android application metadata placeholder for `{key}` must be a complete environment-variable placeholder"
                ));
            };
            let mut characters = variable.chars();
            let valid_variable = characters
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                && characters
                    .all(|character| character.is_ascii_alphanumeric() || character == '_');
            if !valid_variable {
                return Err(format!(
                    "Android application metadata placeholder `{variable}` for `{key}` must be an environment-variable name"
                ));
            }
        }
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
                ios {
                    minVersion: "17.0"
                    backgroundModes: ["audio"]
                    sources: ["ios/Sources/**"]
                    extensionSources: ["ios/Sources/MMKVStoreImpl.swift"]
                    frameworks: ["AVFoundation"]
                    xcframeworks: ["ios/Vendor.xcframework"]
                    privacyManifest: "ios/PrivacyInfo.xcprivacy"
                    resources: ["ios/Resources/model.dat"]
                    usageDescriptions {
                        NSCameraUsageDescription: "Record video clips."
                    }
                    entitlements {
                        "aps-environment": "development"
                        "com.apple.developer.associated-domains": ["applinks:example.com"]
                        "com.apple.developer.networking.wifi-info": true
                    }
                    linkerFlags: ["-ObjC", "-force_load", "$(PROJECT_DIR)/Vendor/lib.a"]
                    dependencies {
                        swiftPackage {
                            url: "https://github.com/example/video-sdk.git"
                            from: "2.1.0"
                            products: ["VideoSDK"]
                            extensionProducts: ["VideoSDKExtension"]
                        }
                    }
                }
                android {
                    minSdk: 28
                    pictureInPicture: true
                    mediaPlaybackService: "dev.example.audio.AudioPlaybackService"
                    sources: ["android/src/main/kotlin/**"]
                    dependencies: ["androidx.media3:media3-exoplayer:1.5.1"]
                    aars: ["android/libs/vendor.aar"]
                    resources: ["android/resources/model.dat"]
                    proguardRules: ["android/rules/vendor.pro"]
                    repositories: ["https://maven.example.com/releases"]
                    permissions: ["android.permission.INTERNET", "android.permission.CAMERA"]
                    applicationMetadata {
                        "com.example.sdk.API_KEY": "${EXAMPLE_SDK_API_KEY}"
                    }
                }
                cpp {
                    standard: "c++23"
                    sources: ["cpp/Sources/**"]
                    headers: ["cpp/include/**"]
                }
                compiler {
                    analyzer: ["python3", "compiler/analyze.py"]
                }
                assets: ["assets/**"]
            }
            "#,
        )
        .expect("manifest should parse");
        assert_eq!(manifest.native.as_deref(), Some("native.nxid"));
        assert_eq!(
            manifest
                .compiler
                .as_ref()
                .map(|compiler| &compiler.analyzer),
            Some(&vec![
                "python3".to_owned(),
                "compiler/analyze.py".to_owned()
            ])
        );
        assert_eq!(manifest.android.min_sdk, Some(28));
        assert!(manifest.android.picture_in_picture);
        assert_eq!(
            manifest.android.media_playback_service.as_deref(),
            Some("dev.example.audio.AudioPlaybackService")
        );
        assert_eq!(manifest.ios.background_modes, vec!["audio"]);
        assert_eq!(manifest.ios.sources, vec!["ios/Sources/**"]);
        assert_eq!(
            manifest.ios.extension_sources,
            vec!["ios/Sources/MMKVStoreImpl.swift"]
        );
        assert_eq!(manifest.ios.frameworks, vec!["AVFoundation"]);
        assert_eq!(manifest.ios.xcframeworks, vec!["ios/Vendor.xcframework"]);
        assert_eq!(
            manifest.ios.privacy_manifest.as_deref(),
            Some("ios/PrivacyInfo.xcprivacy")
        );
        assert_eq!(manifest.ios.resources, vec!["ios/Resources/model.dat"]);
        assert_eq!(manifest.ios.swift_packages.len(), 1);
        assert_eq!(
            manifest.ios.swift_packages[0].requirement,
            SwiftPackageRequirement::From("2.1.0".to_owned())
        );
        assert_eq!(manifest.ios.swift_packages[0].products, vec!["VideoSDK"]);
        assert_eq!(
            manifest.ios.swift_packages[0].extension_products,
            vec!["VideoSDKExtension"]
        );
        assert_eq!(
            manifest.android.maven_dependencies,
            vec!["androidx.media3:media3-exoplayer:1.5.1"]
        );
        assert_eq!(manifest.android.aars, vec!["android/libs/vendor.aar"]);
        assert_eq!(
            manifest.android.resources,
            vec!["android/resources/model.dat"]
        );
        assert_eq!(
            manifest.android.proguard_rules,
            vec!["android/rules/vendor.pro"]
        );
        assert_eq!(
            manifest.android.permissions,
            vec!["android.permission.INTERNET", "android.permission.CAMERA"]
        );
        assert_eq!(
            manifest.android.application_metadata,
            vec![(
                "com.example.sdk.API_KEY".to_owned(),
                "${EXAMPLE_SDK_API_KEY}".to_owned()
            )]
        );
        assert_eq!(
            manifest.android.repositories,
            vec!["https://maven.example.com/releases"]
        );
        assert_eq!(manifest.cpp.sources, vec!["cpp/Sources/**"]);
        assert_eq!(manifest.cpp.headers, vec!["cpp/include/**"]);
        assert_eq!(manifest.cpp.standard, Some(23));
        assert_eq!(
            manifest.ios.usage_descriptions,
            vec![(
                "NSCameraUsageDescription".to_owned(),
                "Record video clips.".to_owned()
            )]
        );
        assert_eq!(
            manifest.ios.entitlements,
            vec![
                (
                    "aps-environment".to_owned(),
                    EntitlementValue::String("development".to_owned())
                ),
                (
                    "com.apple.developer.associated-domains".to_owned(),
                    EntitlementValue::Strings(vec!["applinks:example.com".to_owned()])
                ),
                (
                    "com.apple.developer.networking.wifi-info".to_owned(),
                    EntitlementValue::Bool(true)
                ),
            ]
        );
        assert_eq!(
            manifest.ios.linker_flags,
            vec!["-ObjC", "-force_load", "$(PROJECT_DIR)/Vendor/lib.a"]
        );
    }

    #[test]
    fn rejects_package_escape() {
        let error =
            parse(r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" assets: ["../outside"] }"#)
                .expect_err("path traversal must be rejected");
        assert!(error.contains("must stay inside"));
    }

    #[test]
    fn extension_sources_are_ios_only_and_confined_to_package() {
        let escaped = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { extensionSources: ["../outside.swift"] } }"#,
        )
        .expect_err("extension source paths must stay inside the package");
        assert!(escaped.contains("stay inside"));

        let android_field = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { extensionSources: ["android/Source.kt"] } }"#,
        )
        .expect_err("extensionSources is an iOS target declaration");
        assert!(android_field.contains("unknown platform field `extensionSources`"));

        let duplicate = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { extensionSources: ["ios/Safe.swift", "ios/Safe.swift"] } }"#,
        )
        .expect_err("extension source patterns must be unique");
        assert!(duplicate.contains("declared more than once"));
    }

    #[test]
    fn validates_local_native_artifact_paths_and_extensions() {
        let wrong_ios_extension = parse(
            r#"plugin { schema: 2 id: "dev.nexa.sample" version: "1.0.0" sources { native: "native.nxid" } ios { xcframeworks: ["ios/SDK.framework"] } }"#,
        )
        .expect_err("an iOS binary dependency must be an XCFramework");
        assert!(wrong_ios_extension.contains(".xcframework"));

        let wrong_android_extension = parse(
            r#"plugin { schema: 2 id: "dev.nexa.sample" version: "1.0.0" sources { native: "native.nxid" } android { aars: ["android/SDK.jar"] } }"#,
        )
        .expect_err("an Android local binary dependency must be an AAR");
        assert!(wrong_android_extension.contains(".aar"));

        let escaping_path = parse(
            r#"plugin { schema: 2 id: "dev.nexa.sample" version: "1.0.0" sources { native: "native.nxid" } ios { xcframeworks: ["../SDK.xcframework"] } }"#,
        )
        .expect_err("native binary paths must stay within the package");
        assert!(escaping_path.contains("stay inside the package"));

        let bad_privacy_manifest = parse(
            r#"plugin { schema: 2 id: "dev.nexa.sample" version: "1.0.0" sources { native: "native.nxid" } ios { privacyManifest: "ios/privacy.plist" } }"#,
        )
        .expect_err("privacy manifest must use Apple's recognized filename");
        assert!(bad_privacy_manifest.contains("PrivacyInfo.xcprivacy"));

        let bad_rule_extension = parse(
            r#"plugin { schema: 2 id: "dev.nexa.sample" version: "1.0.0" sources { native: "native.nxid" } android { proguardRules: ["android/rules/keep.txt"] } }"#,
        )
        .expect_err("shrinker rules must use the ProGuard file extension");
        assert!(bad_rule_extension.contains(".pro"));
    }

    #[test]
    fn rejects_unsafe_native_dependency_coordinates() {
        let invalid_cpp_standard = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } cpp { standard: "gnu++20" } }"#,
        )
        .expect_err("C++ standards must use an explicit ISO language level");
        assert!(invalid_cpp_standard.contains("expected `c++17`, `c++20`, or `c++23`"));

        let swift_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { dependencies { swiftPackage { url: "https://example.com/sdk.git" from: "latest" products: ["SDK"] } } } }"#,
        )
        .expect_err("Swift package versions must be pinned to a numeric lower bound");
        assert!(swift_error.contains("numeric dotted version"));

        let maven_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { dependencies: ["group:artifact:$version"] } }"#,
        )
        .expect_err("Gradle code injection must not pass manifest validation");
        assert!(maven_error.contains("group:artifact:version"));

        let repository_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { repositories: ["http://repo.example.com/releases"] } }"#,
        )
        .expect_err("insecure Maven repositories must be rejected");
        assert!(repository_error.contains("must use HTTPS"));

        let framework_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { frameworks: ["AVFoundation; OTHER_LDFLAGS = evil"] } }"#,
        )
        .expect_err("framework names must not inject Xcode settings");
        assert!(framework_error.contains("system framework name"));
    }

    #[test]
    fn rejects_invalid_platform_permissions_and_empty_purpose_messages() {
        let ios_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { usageDescriptions { NSCameraUsageDescription: "   " } } }"#,
        )
        .expect_err("iOS purpose messages must not be empty");
        assert!(ios_error.contains("non-empty purpose message"));

        let android_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { permissions: ["android.permission.CAMERA\n"] } }"#,
        )
        .expect_err("Android permission names must be validated");
        assert!(android_error.contains("android.permission.NAME"));

        let min_sdk_error = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { minSdk: 0 } }"#,
        )
        .expect_err("Android plugin minSdk must be a valid API level");
        assert!(min_sdk_error.contains("Android plugin minSdk must be greater than zero"));
    }

    #[test]
    fn validates_android_application_metadata_and_environment_placeholders() {
        let malformed_key = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { applicationMetadata { "com..example.KEY": "value" } } }"#,
        )
        .expect_err("metadata keys must be dotted identifiers");
        assert!(malformed_key.contains("must be a dotted identifier"));

        let malformed_placeholder = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { applicationMetadata { "com.example.KEY": "prefix-${KEY}" } } }"#,
        )
        .expect_err("placeholders must occupy the full metadata value");
        assert!(malformed_placeholder.contains("complete environment-variable placeholder"));

        let invalid_variable = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } android { applicationMetadata { "com.example.KEY": "${bad-key}" } } }"#,
        )
        .expect_err("environment variable names must be valid identifiers");
        assert!(invalid_variable.contains("environment-variable name"));
    }

    #[test]
    fn validates_ios_entitlement_keys_and_values() {
        let malformed_key = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { entitlements { "com..apple.entitlement": true } } }"#,
        )
        .expect_err("malformed entitlement keys must be rejected");
        assert!(malformed_key.contains("entitlement key"));

        let empty_value = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { entitlements { "aps-environment": "  " } } }"#,
        )
        .expect_err("empty entitlement values must be rejected");
        assert!(empty_value.contains("must not be an empty string"));

        let duplicate = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } ios { entitlements { "aps-environment": "development" "aps-environment": "production" } } }"#,
        )
        .expect_err("duplicate entitlement keys must be rejected");
        assert!(duplicate.contains("declared twice"));
    }

    #[test]
    fn rejects_empty_or_multiline_ios_linker_arguments() {
        let multiline = format!("-ObjC{}OTHER_LDFLAGS = evil", '\n');
        let error = validate_ios_linker_flags(&[multiline])
            .expect_err("control characters must not enter Xcode build settings");
        assert!(error.contains("without control characters"));

        let empty = validate_ios_linker_flags(&[String::new()])
            .expect_err("empty linker arguments must be rejected");
        assert!(empty.contains("non-empty single arguments"));
    }

    #[test]
    fn compiler_analyzer_command_must_be_non_empty_and_package_relative_when_local() {
        let empty = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } compiler { analyzer: [] } }"#,
        )
        .expect_err("an empty analyzer command is invalid");
        assert!(empty.contains("cannot be empty"));

        let traversal = parse(
            r#"plugin { schema: 2 id: "dev.nexa.bad" version: "1" sources { native: "native.nxid" } compiler { analyzer: ["../outside"] } }"#,
        )
        .expect_err("local analyzer executables must stay in the package");
        assert!(traversal.contains("stay inside the package"));
    }
}
