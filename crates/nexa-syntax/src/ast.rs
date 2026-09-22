use std::collections::BTreeMap;

use nexa_diagnostics::Span;

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub plugins: Vec<PluginDecl>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub states: Vec<StateDecl>,
    pub functions: Vec<FunctionDecl>,
    pub screens: Vec<ScreenDecl>,
    pub theme: Option<ThemeDecl>,
    pub components: Vec<ComponentDecl>,
    pub body: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct PluginDecl {
    pub path: String,
    pub namespace: String,
    pub span: Span,
    pub idl: Option<nexa_plugin_idl::PluginIdl>,
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
    pub components: Vec<ComponentDecl>,
    pub structs: Vec<StructDecl>,
    pub app: Option<App>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub permissions: Vec<PermissionConfig>,
    pub plugins: Vec<PluginConfigDecl>,
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
    pub parameters: Vec<FunctionParameter>,
    pub return_type: TypeSyntax,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct FunctionParameter {
    pub name: String,
    pub ty: TypeSyntax,
    pub span: Span,
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
    pub body: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct StateDecl {
    pub name: String,
    pub ty: Option<TypeSyntax>,
    pub initial: Expr,
    pub mutable: bool,
    pub span: Span,
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
    Layout {
        kind: LayoutKind,
        spacing: Option<Expr>,
        style: LayoutStyle,
        children: Vec<Node>,
        span: Span,
    },
    Platform {
        target: PlatformTarget,
        children: Vec<Node>,
        span: Span,
    },
    StatusBar {
        style: Option<Expr>,
        hidden: Option<Expr>,
        span: Span,
    },
    Direction {
        value: Expr,
        span: Span,
    },
    OnAppear {
        actions: Vec<Stmt>,
        asynchronous: bool,
        span: Span,
    },
    OnDisappear {
        actions: Vec<Stmt>,
        span: Span,
    },
    Text {
        value: Expr,
        color: Option<Expr>,
        font_size: Option<Expr>,
        font_weight: Option<Expr>,
        line_limit: Option<Expr>,
        line_height: Option<Expr>,
        letter_spacing: Option<Expr>,
        selectable: Option<Expr>,
        span: Span,
    },
    Button {
        label: Expr,
        icon: Option<Expr>,
        loading: Option<Expr>,
        disabled: Option<Expr>,
        actions: Vec<Stmt>,
        span: Span,
    },
    TextInput {
        value: Expr,
        placeholder: Expr,
        keyboard: Option<Expr>,
        secure: Option<Expr>,
        multiline: Option<Expr>,
        autocorrect: Option<Expr>,
        capitalization: Option<Expr>,
        focused: Option<Expr>,
        max_length: Option<Expr>,
        actions: Vec<Stmt>,
        span: Span,
    },
    Switch {
        value: Expr,
        label: Expr,
        span: Span,
    },
    Image {
        source: ImageSource,
        description: Expr,
        scale: Option<Expr>,
        placeholder: Option<Expr>,
        span: Span,
    },
    Pressable {
        disabled: Option<Expr>,
        haptic: Option<Expr>,
        children: Vec<Node>,
        actions: Vec<Stmt>,
        long_press_actions: Vec<Stmt>,
        span: Span,
    },
    NavigationStack {
        root: Expr,
        span: Span,
    },
    NavigationLink {
        destination: Expr,
        children: Vec<Node>,
        span: Span,
    },
    Link {
        url: Expr,
        children: Vec<Node>,
        span: Span,
    },
    Accessibility {
        label: Expr,
        role: Option<Expr>,
        children: Vec<Node>,
        span: Span,
    },
    KeyboardAware {
        children: Vec<Node>,
        span: Span,
    },
    BottomSheet {
        is_presented: Expr,
        children: Vec<Node>,
        span: Span,
    },
    RefreshControl {
        is_refreshing: Expr,
        children: Vec<Node>,
        actions: Vec<Stmt>,
        span: Span,
    },
    AppBottomBar {
        selected: Expr,
        tabs: Vec<TabDecl>,
        span: Span,
    },
    FastList {
        source: ListSource,
        index: Option<Expr>,
        item: Option<Expr>,
        key: Option<Expr>,
        children: Vec<Node>,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Vec<Node>,
        else_body: Option<Vec<Node>>,
        span: Span,
    },
    When {
        value: Expr,
        cases: Vec<WhenCase>,
        else_body: Vec<Node>,
        span: Span,
    },
    Content {
        span: Span,
    },
    ComponentCall {
        name: String,
        arguments: BTreeMap<String, Expr>,
        children: Option<Vec<Node>>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct WhenCase {
    pub value: Expr,
    pub body: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ListSource {
    Count(Expr),
    Items(Expr),
}

#[derive(Clone, Debug)]
pub struct TabDecl {
    pub index: Expr,
    pub label: Expr,
    pub icon: Option<Expr>,
    pub badge: Option<Expr>,
    pub children: Vec<Node>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct LayoutStyle {
    pub alignment: Option<Expr>,
    pub padding: Option<Expr>,
    pub width: Option<Expr>,
    pub height: Option<Expr>,
    pub background: Option<Expr>,
    pub corner_radius: Option<Expr>,
    pub border_color: Option<Expr>,
    pub border_width: Option<Expr>,
    pub opacity: Option<Expr>,
    pub animation: Option<Expr>,
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutKind {
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformTarget {
    Ios,
    Android,
}

#[derive(Clone, Debug)]
pub enum ImageSource {
    Asset(Expr),
    Url(Expr),
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
    Add(Box<Expr>, Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Array(Vec<Expr>, Span),
    Map(Vec<(Expr, Expr)>, Span),
    Pair(Box<Expr>, Box<Expr>, Span),
    Triple(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    Call(String, Vec<Expr>, Span),
    MethodCall {
        base: Box<Expr>,
        name: String,
        arguments: Vec<Expr>,
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
        arguments: BTreeMap<String, Expr>,
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
    Null(Span),
    Coalesce(Box<Expr>, Box<Expr>, Span),
    Await(Box<Expr>, Span),
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
            | Self::Add(_, _, s)
            | Self::Not(_, s)
            | Self::Binary(_, _, _, s)
            | Self::Array(_, s)
            | Self::Map(_, s)
            | Self::Pair(_, _, s)
            | Self::Triple(_, _, _, s)
            | Self::Call(_, _, s)
            | Self::MethodCall { span: s, .. }
            | Self::Closure { span: s, .. }
            | Self::QualifiedCall { span: s, .. }
            | Self::Index { span: s, .. }
            | Self::Member { span: s, .. }
            | Self::Range { span: s, .. }
            | Self::Null(s)
            | Self::Coalesce(_, _, s)
            | Self::Await(_, s) => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
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
    CollectionMutation {
        name: String,
        method: String,
        arguments: Vec<Expr>,
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
