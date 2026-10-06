use std::collections::BTreeMap;

use nexa_diagnostics::Span;

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub plugins: Vec<PluginDecl>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    /// User-defined reference classes with immutable state and instance methods.
    pub classes: Vec<ClassDecl>,
    pub states: Vec<StateDecl>,
    /// Immutable module-level values, emitted once outside UI identity/lifecycle.
    pub globals: Vec<StateDecl>,
    pub functions: Vec<FunctionDecl>,
    pub background_tasks: Vec<BackgroundTaskDecl>,
    pub screens: Vec<ScreenDecl>,
    /// Platform-neutral widget declarations collected from imported source modules.
    pub widgets: Vec<WidgetDecl>,
    pub theme: Option<ThemeDecl>,
    pub components: Vec<ComponentDecl>,
    /// Host-evaluated unit tests. These declarations never enter generated app code.
    pub tests: Vec<TestDecl>,
    pub body: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PluginDecl {
    pub path: String,
    /// Plugin identity from `plugin.config.nx`, when loaded from a package.
    pub package_id: Option<String>,
    /// Canonical package root for generic compiler analyzer invocation.
    pub package_root: Option<String>,
    /// Canonical executable path for a plugin-owned compiler analyzer.
    pub compiler_analyzer: Vec<String>,
    pub namespace: String,
    pub span: Span,
    pub idl: Option<nexa_plugin_idl::PluginIdl>,
    pub pure: bool,
    pub assets_path: Option<String>,
    /// Absolute source globs declared by the plugin manifest. Keeping these
    /// on the resolved declaration lets project generation honor a plugin's
    /// package layout without rediscovering or guessing conventional folders.
    pub ios_sources: Vec<String>,
    /// Absolute Swift sources explicitly declared safe for app-extension targets.
    pub ios_extension_sources: Vec<String>,
    pub android_sources: Vec<String>,
    /// Absolute C++ source and header globs declared by `plugin.config.nx`.
    pub cpp_sources: Vec<String>,
    pub cpp_headers: Vec<String>,
    /// Minimum C++ language standard required by native C++ sources.
    pub cpp_standard: Option<u8>,
    pub ios_min_version: Option<String>,
    pub android_min_sdk: Option<u32>,
    pub ios_frameworks: Vec<String>,
    pub ios_xcframeworks: Vec<String>,
    pub ios_resources: Vec<String>,
    pub ios_privacy_manifest: Option<String>,
    pub swift_packages: Vec<nexa_plugin_idl::manifest::SwiftPackage>,
    pub maven_dependencies: Vec<String>,
    pub android_aars: Vec<String>,
    pub android_resources: Vec<String>,
    pub android_proguard_rules: Vec<String>,
    pub android_maven_repositories: Vec<String>,
    pub ios_usage_descriptions: Vec<(String, String)>,
    pub ios_entitlements: Vec<(String, PluginEntitlementValue)>,
    pub ios_application_delegate: Option<String>,
    pub ios_background_modes: Vec<String>,
    pub ios_linker_flags: Vec<String>,
    pub android_permissions: Vec<String>,
    pub android_application_metadata: Vec<(String, String)>,
    pub android_firebase_messaging_service: Option<String>,
    /// The generated Android activity opts into PiP for video playback plugins.
    pub android_picture_in_picture: bool,
    /// Android MediaSessionService class contributed to the generated host.
    pub android_media_playback_service: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginEntitlementValue {
    String(String),
    Bool(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructFieldDecl>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StructFieldDecl {
    pub name: String,
    pub ty: TypeSyntax,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub name: String,
    pub constructor_parameters: Vec<ClassParameterDecl>,
    pub fields: Vec<ClassFieldDecl>,
    pub static_fields: Vec<ClassFieldDecl>,
    pub methods: Vec<FunctionDecl>,
    pub static_methods: Vec<FunctionDecl>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ClassParameterDecl {
    pub name: String,
    pub ty: TypeSyntax,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ClassFieldDecl {
    pub name: String,
    pub ty: Option<TypeSyntax>,
    pub initial: Expr,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub cases: Vec<EnumCaseDecl>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct EnumCaseDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub plugins: Vec<PluginDecl>,
    pub enums: Vec<EnumDecl>,
    pub components: Vec<ComponentDecl>,
    pub structs: Vec<StructDecl>,
    pub classes: Vec<ClassDecl>,
    pub functions: Vec<FunctionDecl>,
    /// Immutable file-scope values. These are suitable for long-lived native handles.
    pub globals: Vec<StateDecl>,
    /// Named screens declared in a standalone source module.
    pub screens: Vec<ScreenDecl>,
    /// Standalone native widget declarations.
    pub widgets: Vec<WidgetDecl>,
    pub tests: Vec<TestDecl>,
    pub app: Option<App>,
}

#[derive(Clone, Debug)]
pub struct TestDecl {
    pub name: String,
    /// Optional custom component instantiated by a headless behavior test.
    pub component: Option<Expr>,
    pub statements: Vec<TestStatement>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub enum TestStatement {
    Let {
        name: String,
        ty: Option<TypeSyntax>,
        initial: Expr,
        span: Span,
    },
    Assert {
        condition: Expr,
        message: Option<Expr>,
        span: Span,
    },
    Tap {
        label: Expr,
        span: Span,
    },
    AssertText {
        value: Expr,
        span: Span,
    },
    TypeText {
        placeholder: Expr,
        value: Expr,
        span: Span,
    },
    Toggle {
        label: Expr,
        span: Span,
    },
    Slide {
        state: Expr,
        value: Expr,
        span: Span,
    },
    Select {
        state: Expr,
        value: Expr,
        span: Span,
    },
    Submit {
        placeholder: Expr,
        span: Span,
    },
    AssertComponent {
        name: Expr,
        span: Span,
    },
    Emit {
        component: Expr,
        event: Expr,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct Config {
    pub app: Option<AppConfig>,
    pub flavors: Vec<FlavorConfig>,
    pub assets: Option<AssetsConfig>,
    pub dependencies: Vec<PluginDependencyConfig>,
    pub permissions: Vec<PermissionConfig>,
    pub plugins: Vec<PluginConfigDecl>,
    pub ios: Option<IosConfig>,
    pub android: Option<AndroidConfig>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct AssetsConfig {
    pub icon: Option<String>,
    pub splash: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub display_name: String,
    pub version: String,
    pub build_number: u32,
    pub staging_suffix: Option<String>,
    pub deep_links: Vec<String>,
    pub orientation: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FlavorConfig {
    pub name: String,
    pub suffix: Option<String>,
}

#[derive(Clone, Debug)]
pub struct IosConfig {
    pub min_version: Option<String>,
    pub bundle_identifier: Option<String>,
    pub app_group_identifier: Option<String>,
    pub icon: Option<String>,
    pub alternate_icons: Vec<String>,
    pub arch: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct AndroidConfig {
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub application_id: Option<String>,
    pub icon: Option<String>,
    pub alternate_icons: Vec<String>,
    pub arch: Option<Vec<String>>,
    pub cronet: Option<AndroidCronetConfig>,
}

#[derive(Clone, Debug)]
pub struct AndroidCronetConfig {
    pub provider: Option<String>,
    pub disk_cache_size_mb: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct PluginDependencyConfig {
    pub alias: String,
    pub package_id: String,
    pub path: Option<String>,
    pub git: Option<String>,
    pub revision: Option<String>,
    pub package_path: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PermissionConfig {
    pub name: String,
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PluginConfigDecl {
    pub name: String,
    pub options: Vec<PluginOptionDecl>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PluginOptionDecl {
    pub name: String,
    pub value: ConfigValue,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ConfigValue {
    String(String),
    Number(String),
    Bool(bool),
    Null,
    Array(Vec<ConfigValue>),
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ComponentDecl {
    pub name: String,
    pub parameters: Vec<ComponentParameter>,
    pub states: Vec<StateDecl>,
    pub body: Vec<Node>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ComponentParameter {
    pub name: String,
    pub ty: TypeSyntax,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub name: String,
    pub is_async: bool,
    pub is_throwing: bool,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: TypeSyntax,
    pub body: Vec<Stmt>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FunctionParameter {
    pub name: String,
    pub ty: TypeSyntax,
    pub span: Span,
}

/// A periodic app task that the native host can resume after process death.
#[derive(Clone, Debug)]
pub struct BackgroundTaskDecl {
    pub name: String,
    pub identifier: String,
    pub interval_minutes: u32,
    pub body: Vec<Stmt>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ThemeDecl {
    pub tokens: Vec<ThemeTokenDecl>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeTokenKind {
    Color,
    Spacing,
    Radius,
    FontSize,
}

#[derive(Clone, Debug)]
pub struct ThemeTokenDecl {
    pub kind: ThemeTokenKind,
    pub name: String,
    pub value: ThemeTokenValue,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ThemeTokenValue {
    Static(Expr),
    AdaptiveColor { light: Expr, dark: Expr },
}

#[derive(Clone, Debug)]
pub struct ScreenDecl {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub states: Vec<StateDecl>,
    pub body: Vec<Node>,
    pub span: Span,
    pub source_file: Option<String>,
}

/// A cross-platform widget declaration with an immutable, typed timeline entry.
/// The optional configuration expression supplies the default value for the
/// widget's user-configurable settings.
#[derive(Clone, Debug)]
pub struct WidgetDecl {
    pub name: String,
    /// Source text for the native widget's optional localized metadata.
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub configuration_title: Option<String>,
    pub configuration_description: Option<String>,
    pub configuration: Option<Expr>,
    pub entry_provider: Expr,
    /// Synchronous value used by native widget hosts while async entry data
    /// loads and when a provider fails.
    pub placeholder_provider: Option<Expr>,
    pub families: Vec<String>,
    pub refresh_seconds: Expr,
    pub body: Vec<Node>,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct StateDecl {
    pub name: String,
    pub ty: Option<TypeSyntax>,
    pub initial: Expr,
    pub mutable: bool,
    pub span: Span,
    pub source_file: Option<String>,
}

#[derive(Clone, Debug)]
pub enum TypeSyntax {
    Named(String, Span),
    Generic(String, Vec<TypeSyntax>, Span),
    Optional(Box<TypeSyntax>, Span),
}

impl TypeSyntax {
    pub fn span(&self) -> Span {
        match self {
            Self::Named(_, span) | Self::Generic(_, _, span) | Self::Optional(_, span) => *span,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Platform {
        target: PlatformTarget,
        children: Vec<Node>,
        span: Span,
    },
    /// A built-in component validated against the language catalog
    /// (`catalog::ComponentSchema`). Semantic lowering converts each
    /// invocation into the existing typed IR nodes, so generated code stays
    /// fully specialized with no runtime maps or reflection.
    ComponentInvocation(ComponentInvocation),
    If {
        condition: Expr,
        then_body: Vec<Node>,
        else_body: Option<Vec<Node>>,
        transition: Option<ViewTransition>,
        span: Span,
    },
    When {
        value: Expr,
        cases: Vec<WhenCase>,
        else_body: Vec<Node>,
        transition: Option<ViewTransition>,
        span: Span,
    },
    ComponentCall {
        name: String,
        arguments: BTreeMap<String, Expr>,
        children: Option<Vec<Node>>,
        span: Span,
    },
    /// A statically qualified plugin visual component, for example
    /// `Video.VideoView(player: player)`.
    NativeComponentCall {
        namespace: String,
        name: String,
        arguments: BTreeMap<String, Expr>,
        children: Option<Vec<Node>>,
        event_handlers: Vec<NativeComponentEventHandler>,
        span: Span,
    },
}

/// A single built-in component use: head arguments plus child block plus
/// trailing dot-modifiers, exactly as validated against its catalog schema.
#[derive(Clone, Debug)]
pub struct ComponentInvocation {
    pub name: String,
    pub span: Span,
    /// Leading positional expressions (`Text("label")`, FastList collection).
    pub positional: Vec<Expr>,
    /// Named options keyed by source spelling.
    pub arguments: BTreeMap<String, Expr>,
    /// Leading keywords accepted by the schema (`OnAppear async`).
    pub flags: Vec<String>,
    pub children: ChildBody,
    pub modifiers: Vec<DotModifier>,
}

/// The child block a component invocation carries.
#[derive(Clone, Debug)]
pub enum ChildBody {
    /// No trailing block.
    None,
    /// A `{ ... }` node block.
    Nodes(Vec<Node>),
    /// A trailing action block (`Button { ... }`).
    Actions(Vec<Stmt>),
    /// `AppBottomBar` tab declarations.
    Tabs(Vec<TabDecl>),
    /// `NavigationSplitView` sidebar and detail columns.
    SplitPanes {
        sidebar: Vec<Node>,
        detail: Vec<Node>,
    },
    /// FastList row bindings plus row body.
    Rows(ListRows),
}

/// FastList row closure: validated source, bindings, and row body.
#[derive(Clone, Debug)]
pub struct ListRows {
    pub source: ListSource,
    pub key: Option<ListKey>,
    pub item: Option<Expr>,
    pub index: Option<Expr>,
    pub section: Option<Expr>,
    pub children: Vec<Node>,
}

/// One trailing `.name { ... }` modifier on an invocation.
#[derive(Clone, Debug)]
pub struct DotModifier {
    pub name: String,
    pub span: Span,
    pub arguments: BTreeMap<String, Expr>,
    pub body: ModifierBody,
}

/// A dot-modifier payload: action statements or child nodes.
#[derive(Clone, Debug)]
pub enum ModifierBody {
    None,
    Actions(Vec<Stmt>),
    EventActions {
        parameters: Vec<String>,
        actions: Vec<Stmt>,
    },
    Nodes(Vec<Node>),
}

#[derive(Clone, Debug)]
pub struct NativeComponentEventHandler {
    pub property: String,
    pub parameters: Vec<String>,
    pub actions: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ListKey {
    SelfValue(Span),
    Member { name: String, span: Span },
}

#[derive(Clone, Debug)]
pub struct WhenCase {
    pub value: Expr,
    pub body: Vec<Node>,
    pub span: Span,
}

/// Native appearance/removal effect attached to a conditional view block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewTransition {
    Fade,
    SlideFromBottom,
    SlideFromLeft,
    SlideFromRight,
    Scale,
}

#[derive(Clone, Debug)]
pub enum ListSource {
    Count(Expr),
    Items(Expr),
    Sections(Expr),
}

#[derive(Clone, Debug)]
pub struct TabDecl {
    pub index: Expr,
    pub label: Expr,
    pub comment: Option<String>,
    pub icon: Option<Expr>,
    pub badge: Option<Expr>,
    pub role: Option<Expr>,
    /// Native title for the destination's navigation stack.
    pub navigation_title: Option<Expr>,
    /// Requests the platform's large navigation title presentation.
    pub large_title: Option<Expr>,
    /// Optional text state that a native navigation search field edits.
    pub searchable: Option<Expr>,
    /// Native search field prompt. Used together with `searchable`.
    pub search_prompt: Option<Expr>,
    pub children: Vec<Node>,
    pub span: Span,
}

/// A layout style bundle assembled by semantic lowering from an invocation's
/// named options. The parser no longer builds this; it stays here as the
/// syntactic shape `styles::lower_style` consumes.
#[derive(Clone, Debug, Default)]
pub struct LayoutStyle {
    pub alignment: Option<Expr>,
    pub padding: Option<Expr>,
    pub width: Option<Expr>,
    pub height: Option<Expr>,
    pub min_width: Option<Expr>,
    pub max_width: Option<Expr>,
    pub min_height: Option<Expr>,
    pub max_height: Option<Expr>,
    pub background: Option<Expr>,
    pub corner_radius: Option<Expr>,
    pub border_color: Option<Expr>,
    pub border_width: Option<Expr>,
    pub opacity: Option<Expr>,
    pub scale: Option<Expr>,
    pub rotation: Option<Expr>,
    pub shadow: Option<Expr>,
    pub blur: Option<Expr>,
    pub clip: Option<Expr>,
    pub z_index: Option<Expr>,
    pub glass: Option<Expr>,
    pub animation: Option<Expr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformTarget {
    Ios,
    Android,
}

/// An image source assembled by semantic lowering from an invocation's
/// `asset:`/`url:`/`file:` options (exactly one is enforced by the catalog schema).
#[derive(Clone, Debug)]
pub enum ImageSource {
    Asset(Expr),
    Url(Expr),
    File(Expr),
}

/// Split a navigation target into its screen expression and call arguments.
/// `Home` stays a bare name; `Home(id: 1)` splits into `Home` plus `[1]`.
/// Semantic lowering applies this to `root:`/`destination:` values so the
/// parser remains purely syntactic.
pub fn split_navigation_target(target: Expr) -> (Expr, Vec<Expr>) {
    match target {
        Expr::Call(name, _, arguments, span) => (Expr::Name(name, span), arguments),
        target => (target, Vec::new()),
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    String(String, Span),
    Interpolation(Vec<StringPart>, Span),
    Number(String, Span),
    Bool(bool, Span),
    Name(String, Span),
    EnumCase {
        enum_name: String,
        case_name: String,
        span: Span,
    },
    ThemeToken(String, Span),
    IsRegularWidth(Span),
    IsCompactWidth(Span),
    IsRegularHeight(Span),
    IsCompactHeight(Span),
    Add(Box<Expr>, Box<Expr>, Span),
    Arithmetic(Box<Expr>, ArithmeticOp, Box<Expr>, Span),
    Negate(Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Array(Vec<Expr>, Span),
    Map(Vec<(Expr, Expr)>, Span),
    Pair(Box<Expr>, Box<Expr>, Span),
    Triple(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    /// `Call(String, Vec<Expr>, Span)` plus explicit value type arguments,
    /// as in `getObject<PlayerOptions>("key")`.
    Call(String, Vec<TypeSyntax>, Vec<Expr>, Span),
    CallNamed {
        name: String,
        type_arguments: Vec<TypeSyntax>,
        arguments: BTreeMap<String, Expr>,
        span: Span,
    },
    MethodCall {
        base: Box<Expr>,
        name: String,
        /// Explicit value type arguments on a plugin method call.
        type_arguments: Vec<TypeSyntax>,
        arguments: Vec<Expr>,
        named_arguments: BTreeMap<String, Expr>,
        span: Span,
    },
    Closure {
        parameters: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },
    QualifiedCall {
        namespace: String,
        name: String,
        /// Explicit value type arguments on a plugin service call.
        type_arguments: Vec<TypeSyntax>,
        arguments: Vec<Expr>,
        named_arguments: BTreeMap<String, Expr>,
        span: Span,
    },
    Index {
        collection: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
        span: Span,
    },
    Member {
        base: Box<Expr>,
        name: String,
        optional: bool,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        step: Option<Box<Expr>>,
        span: Span,
    },
    Conditional {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
        span: Span,
    },
    Null(Span),
    Coalesce(Box<Expr>, Box<Expr>, Span),
    Await(Box<Expr>, Span),
    Try {
        expr: Box<Expr>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum StringPart {
    Literal(String),
    Name(String),
    Expression(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    And,
    Or,
    Contains,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithmeticOp {
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::String(_, s)
            | Self::Interpolation(_, s)
            | Self::Number(_, s)
            | Self::Bool(_, s)
            | Self::Name(_, s)
            | Self::EnumCase { span: s, .. }
            | Self::ThemeToken(_, s)
            | Self::IsRegularWidth(s)
            | Self::IsCompactWidth(s)
            | Self::IsRegularHeight(s)
            | Self::IsCompactHeight(s)
            | Self::Add(_, _, s)
            | Self::Arithmetic(_, _, _, s)
            | Self::Negate(_, s)
            | Self::Not(_, s)
            | Self::Binary(_, _, _, s)
            | Self::Array(_, s)
            | Self::Map(_, s)
            | Self::Pair(_, _, s)
            | Self::Triple(_, _, _, s)
            | Self::Call(_, _, _, s)
            | Self::CallNamed { span: s, .. }
            | Self::MethodCall { span: s, .. }
            | Self::Closure { span: s, .. }
            | Self::QualifiedCall { span: s, .. }
            | Self::Index { span: s, .. }
            | Self::Member { span: s, .. }
            | Self::Range { span: s, .. }
            | Self::Conditional { span: s, .. }
            | Self::Null(s)
            | Self::Coalesce(_, _, s)
            | Self::Await(_, s)
            | Self::Try { span: s, .. } => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Expression {
        expression: Expr,
        span: Span,
    },
    Let {
        name: String,
        ty: Option<TypeSyntax>,
        initial: Expr,
        span: Span,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    NativePropertyAssign {
        receiver: Expr,
        property: String,
        value: Expr,
        span: Span,
    },
    NativeEventSubscribe {
        receiver: Expr,
        event: String,
        parameters: Vec<String>,
        actions: Vec<Stmt>,
        span: Span,
    },
    CollectionMutation {
        name: String,
        method: String,
        /// Explicit value type arguments, as in `store.getObject<PlayerOptions>(k)`.
        /// Only a plugin method call can carry them; the compiler reports it.
        type_arguments: Vec<TypeSyntax>,
        arguments: Vec<Expr>,
        span: Span,
    },
    /// Starts a view-scoped native task and stores its cancellation handle in
    /// a mutable app, screen, or component state binding.
    TaskLaunch {
        handle: Option<String>,
        executor: TaskExecutor,
        body: Vec<Stmt>,
        span: Span,
    },
    /// Requests cooperative cancellation of a task handle.
    TaskCancel {
        handle: String,
        span: Span,
    },
    /// Applies a native animation transaction to floating-point state writes.
    WithAnimation {
        animation: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    For {
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    ForMap {
        key_name: String,
        value_name: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    TryCatch {
        body: Vec<Stmt>,
        error_catches: Vec<ErrorCatchArm>,
        catch_body: Option<Vec<Stmt>>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Return {
        value: Expr,
        span: Span,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskExecutor {
    Main,
    Background,
}

#[derive(Clone, Debug)]
pub struct ErrorCatchArm {
    pub namespace: String,
    pub error_name: String,
    pub variant: String,
    pub bindings: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}
