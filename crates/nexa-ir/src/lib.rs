pub mod walk;

#[derive(Clone, Debug)]
pub struct Module {
    pub app_name: String,
    pub states: Vec<State>,
    pub screens: Vec<Screen>,
    pub components: Vec<Component>,
    pub body: Vec<Node>,
    pub status_bar: Option<StatusBarConfig>,
    pub direction: Option<DirectionConfig>,
    pub on_appear: Option<Vec<Action>>,
    pub on_disappear: Option<Vec<Action>>,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub parameters: Vec<ComponentParameter>,
    pub states: Vec<State>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct ComponentParameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Screen {
    pub id: ScreenId,
    pub name: String,
    pub body: Vec<Node>,
    pub on_appear: Option<Vec<Action>>,
    pub on_disappear: Option<Vec<Action>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScreenId(pub usize);

#[derive(Clone, Debug)]
pub struct State {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
    pub mutable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusBarConfig {
    pub style: StatusBarStyle,
    pub hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionConfig {
    pub style: DirectionStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionStyle {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusBarStyle {
    Default,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    String,
    Bool,
    Numeric(NumericType),
    Array(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Pair(Box<Type>, Box<Type>),
    Triple(Box<Type>, Box<Type>, Box<Type>),
}

#[derive(Clone, Debug)]
pub enum Expr {
    String(String),
    Interpolation(Vec<InterpolatedPart>),
    Bool(bool),
    Number {
        raw: String,
        ty: NumericType,
    },
    State(String, Type),
    Add(Box<Expr>, Box<Expr>, NumericType),
    Not(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Array(Vec<Expr>),
    Set(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Pair(Box<Expr>, Box<Expr>),
    Triple(Box<Expr>, Box<Expr>, Box<Expr>),
    IsRegularWidth,
}

#[derive(Clone, Debug)]
pub enum InterpolatedPart {
    Literal(String),
    Value(Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    And,
    Or,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug)]
pub enum Node {
    StatusBar {
        config: StatusBarConfig,
    },
    Direction {
        config: DirectionConfig,
    },
    OnAppear {
        actions: Vec<Action>,
    },
    OnDisappear {
        actions: Vec<Action>,
    },
    Layout {
        kind: LayoutKind,
        spacing: f32,
        style: ViewStyle,
        children: Vec<Node>,
    },
    Text {
        value: Expr,
        style: TextStyle,
    },
    Button {
        label: Expr,
        actions: Vec<Action>,
    },
    TextInput {
        state: String,
        placeholder: String,
        keyboard: KeyboardType,
        secure: bool,
        multiline: bool,
        autocorrect: Option<bool>,
        capitalization: Option<Capitalization>,
    },
    Switch {
        state: String,
        label: String,
    },
    Image {
        source: ImageSource,
        description: String,
        scale: ImageScale,
        placeholder: Option<String>,
    },
    Pressable {
        disabled: bool,
        children: Vec<Node>,
        actions: Vec<Action>,
    },
    NavigationStack {
        root: ScreenId,
    },
    NavigationLink {
        destination: ScreenId,
        children: Vec<Node>,
    },
    Link {
        url: String,
        children: Vec<Node>,
    },
    Accessibility {
        label: String,
        role: AccessibilityRole,
        children: Vec<Node>,
    },
    KeyboardAware {
        children: Vec<Node>,
    },
    BottomSheet {
        state: String,
        children: Vec<Node>,
    },
    RefreshControl {
        state: String,
        children: Vec<Node>,
        actions: Vec<Action>,
    },
    AppBottomBar {
        state: String,
        tabs: Vec<BottomBarTab>,
    },
    FastList {
        source: ListSource,
        index: String,
        item: Option<String>,
        children: Vec<Node>,
    },
    If {
        condition: Expr,
        then_body: Vec<Node>,
        else_body: Option<Vec<Node>>,
    },
    ComponentCall {
        name: String,
        arguments: Vec<(String, Expr)>,
    },
}

#[derive(Clone, Debug)]
pub struct BottomBarTab {
    pub index: i32,
    pub label: String,
    pub children: Vec<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    None,
    Button,
    Link,
    Header,
    Image,
}

#[derive(Clone, Debug)]
pub enum ListSource {
    Count(Expr),
    Items {
        collection: Expr,
        element_type: Type,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutKind {
    Column,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardType {
    Text,
    Number,
    Email,
    Phone,
    Url,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capitalization {
    None,
    Sentences,
    Words,
    Characters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageScale {
    Fit,
    Fill,
}

#[derive(Clone, Debug)]
pub enum ImageSource {
    Asset(String),
    RemoteUrl(String),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewStyle {
    pub alignment: Option<Alignment>,
    pub padding: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub background: Option<ColorValue>,
    pub corner_radius: Option<f32>,
    pub opacity: Option<f32>,
}

impl ViewStyle {
    /// Whether the style has visual properties emitted through a native modifier.
    pub fn has_modifiers(&self) -> bool {
        self.padding.is_some()
            || self.width.is_some()
            || self.height.is_some()
            || self.background.is_some()
            || self.corner_radius.is_some()
            || self.opacity.is_some()
    }
}

/// Cross-axis alignment for a native row or column layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorValue {
    Static(Color),
    Adaptive { light: Color, dark: Color },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    pub color: Option<ColorValue>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub line_limit: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Debug)]
pub enum Action {
    Assign {
        name: String,
        value: Expr,
    },
    If {
        condition: Expr,
        then_branch: Vec<Action>,
        else_branch: Option<Vec<Action>>,
    },
}

impl NumericType {
    pub fn swift(self) -> &'static str {
        match self {
            Self::Int8 => "Int8",
            Self::Int16 => "Int16",
            Self::Int32 => "Int32",
            Self::Int64 => "Int64",
            Self::UInt8 => "UInt8",
            Self::UInt16 => "UInt16",
            Self::UInt32 => "UInt32",
            Self::UInt64 => "UInt64",
            Self::Float32 => "Float",
            Self::Float64 => "Double",
        }
    }
    pub fn kotlin(self) -> &'static str {
        match self {
            Self::Int8 => "Byte",
            Self::Int16 => "Short",
            Self::Int32 => "Int",
            Self::Int64 => "Long",
            Self::UInt8 => "UByte",
            Self::UInt16 => "UShort",
            Self::UInt32 => "UInt",
            Self::UInt64 => "ULong",
            Self::Float32 => "Float",
            Self::Float64 => "Double",
        }
    }
}

impl Type {
    pub fn swift(&self) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Bool => "Bool".to_owned(),
            Self::Numeric(n) => n.swift().to_owned(),
            Self::Array(element) => format!("[{}]", element.swift()),
            Self::Set(element) => format!("Set<{}>", element.swift()),
            Self::Map(key, value) => format!("[{}: {}]", key.swift(), value.swift()),
            Self::Pair(first, second) => format!("({}, {})", first.swift(), second.swift()),
            Self::Triple(first, second, third) => {
                format!("({}, {}, {})", first.swift(), second.swift(), third.swift())
            }
        }
    }
    pub fn kotlin(&self) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Bool => "Boolean".to_owned(),
            Self::Numeric(n) => n.kotlin().to_owned(),
            Self::Array(element) => format!("List<{}>", element.kotlin()),
            Self::Set(element) => format!("Set<{}>", element.kotlin()),
            Self::Map(key, value) => format!("Map<{}, {}>", key.kotlin(), value.kotlin()),
            Self::Pair(first, second) => {
                format!("Pair<{}, {}>", first.kotlin(), second.kotlin())
            }
            Self::Triple(first, second, third) => format!(
                "Triple<{}, {}, {}>",
                first.kotlin(),
                second.kotlin(),
                third.kotlin()
            ),
        }
    }
}
