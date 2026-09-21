use nexa_diagnostics::Span;

#[derive(Clone, Debug)]
pub struct App {
    pub name: String,
    pub states: Vec<StateDecl>,
    pub screens: Vec<ScreenDecl>,
    pub body: Vec<Node>,
    pub span: Span,
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
    pub ty: TypeSyntax,
    pub initial: Expr,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeSyntax {
    Named(String, Span),
    Generic(String, Vec<TypeSyntax>, Span),
}

impl TypeSyntax {
    pub fn span(&self) -> Span {
        match self {
            Self::Named(_, span) | Self::Generic(_, _, span) => *span,
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
    Text {
        value: Expr,
        span: Span,
    },
    Button {
        label: Expr,
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
        children: Vec<Node>,
        actions: Vec<Stmt>,
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
    KeyboardAware {
        children: Vec<Node>,
        span: Span,
    },
    FastList {
        source: ListSource,
        index: Option<Expr>,
        item: Option<Expr>,
        children: Vec<Node>,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum ListSource {
    Count(Expr),
    Items(Expr),
}

#[derive(Clone, Debug, Default)]
pub struct LayoutStyle {
    pub padding: Option<Expr>,
    pub width: Option<Expr>,
    pub height: Option<Expr>,
    pub background: Option<Expr>,
    pub corner_radius: Option<Expr>,
    pub opacity: Option<Expr>,
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutKind {
    View,
    Column,
    Row,
}

#[derive(Clone, Debug)]
pub enum ImageSource {
    Asset(Expr),
    Url(Expr),
}

#[derive(Clone, Debug)]
pub enum Expr {
    String(String, Span),
    Number(String, Span),
    Bool(bool, Span),
    Name(String, Span),
    Add(Box<Expr>, Box<Expr>, Span),
    Array(Vec<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Self::String(_, s)
            | Self::Number(_, s)
            | Self::Bool(_, s)
            | Self::Name(_, s)
            | Self::Add(_, _, s)
            | Self::Array(_, s) => *s,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
}
