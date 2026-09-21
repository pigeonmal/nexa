#[derive(Clone, Debug)]
pub struct Module {
    pub app_name: String,
    pub states: Vec<State>,
    pub screens: Vec<Screen>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct Screen {
    pub id: ScreenId,
    pub name: String,
    pub body: Vec<Node>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    String,
    Bool,
    Numeric(NumericType),
    Array(Box<Type>),
}

#[derive(Clone, Debug)]
pub enum Expr {
    String(String),
    Bool(bool),
    Number { raw: String, ty: NumericType },
    State(String, Type),
    Add(Box<Expr>, Box<Expr>, NumericType),
    Array(Vec<Expr>),
}

#[derive(Clone, Debug)]
pub enum Node {
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
    KeyboardAware {
        children: Vec<Node>,
    },
    FastList {
        source: ListSource,
        index: String,
        item: Option<String>,
        children: Vec<Node>,
    },
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
    View,
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
    pub padding: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub background: Option<ColorValue>,
    pub corner_radius: Option<f32>,
    pub opacity: Option<f32>,
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
}

#[derive(Clone, Debug)]
pub enum Action {
    Assign { name: String, value: Expr },
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
        }
    }
    pub fn kotlin(&self) -> String {
        match self {
            Self::String => "String".to_owned(),
            Self::Bool => "Boolean".to_owned(),
            Self::Numeric(n) => n.kotlin().to_owned(),
            Self::Array(element) => format!("List<{}>", element.kotlin()),
        }
    }
}
