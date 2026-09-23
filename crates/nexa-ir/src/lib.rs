pub mod capabilities;
pub mod walk;

#[derive(Clone, Debug)]
pub struct Module {
    pub app_name: String,
    pub plugins: Vec<Plugin>,
    pub plugin_assets: Vec<PluginAsset>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub functions: Vec<Function>,
    pub states: Vec<State>,
    pub screens: Vec<Screen>,
    pub components: Vec<Component>,
    pub body: Vec<Node>,
    pub status_bar: Option<StatusBarConfig>,
    pub direction: Option<DirectionConfig>,
    pub on_appear: Option<Vec<Action>>,
    pub on_appear_async: bool,
    pub on_disappear: Option<Vec<Action>>,
    pub on_active: Option<Vec<Action>>,
    pub on_inactive: Option<Vec<Action>>,
    pub on_background: Option<Vec<Action>>,
}

#[derive(Clone, Debug)]
pub struct Plugin {
    pub namespace: String,
    pub idl_path: String,
    /// Absolute source roots/globs declared by `plugin.config.nx`.
    pub ios_sources: Vec<String>,
    pub android_sources: Vec<String>,
    /// Absolute optional C++ implementation source/header globs.
    pub cpp_sources: Vec<String>,
    pub cpp_headers: Vec<String>,
    /// Minimum C++ language standard required by this plugin's native sources.
    pub cpp_standard: Option<u8>,
    pub ios_min_version: Option<String>,
    pub android_min_sdk: Option<u32>,
    pub ios_frameworks: Vec<String>,
    /// Absolute paths to local XCFramework bundles declared by plugins.
    pub ios_xcframeworks: Vec<String>,
    /// Absolute paths to plugin-owned resources included in the iOS app bundle.
    pub ios_resources: Vec<String>,
    /// Absolute path to the plugin's `PrivacyInfo.xcprivacy` manifest.
    pub ios_privacy_manifest: Option<String>,
    pub swift_packages: Vec<SwiftPackage>,
    pub maven_dependencies: Vec<String>,
    /// Absolute paths to local Android AAR artifacts declared by plugins.
    pub android_aars: Vec<String>,
    /// Absolute paths to plugin-owned resources packaged as Android assets.
    pub android_resources: Vec<String>,
    /// Absolute paths to Android R8/ProGuard rules supplied by the plugin.
    pub android_proguard_rules: Vec<String>,
    pub android_maven_repositories: Vec<String>,
    pub ios_usage_descriptions: Vec<(String, String)>,
    pub ios_entitlements: Vec<(String, PluginEntitlementValue)>,
    pub ios_linker_flags: Vec<String>,
    pub android_permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginEntitlementValue {
    String(String),
    Bool(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwiftPackage {
    pub url: String,
    pub from: String,
    pub products: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PluginAsset {
    pub root: String,
}

#[derive(Clone, Debug)]
pub struct NativeComponentEventHandler {
    pub property: String,
    pub parameters: Vec<String>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    Camera,
    Microphone,
    Photos,
    Location,
    Notifications,
    Contacts,
    Calendar,
    Bluetooth,
}

#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub cases: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Clone, Debug)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<FunctionParameter>,
    pub locals: Vec<FunctionLocal>,
    pub return_type: Type,
    pub body: Expr,
}

#[derive(Clone, Debug)]
pub struct FunctionParameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct FunctionLocal {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub source_file: Option<String>,
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
    pub parameters: Vec<FunctionParameter>,
    pub states: Vec<State>,
    pub body: Vec<Node>,
    pub status_bar: Option<StatusBarConfig>,
    pub on_appear: Option<Vec<Action>>,
    pub on_appear_async: bool,
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

impl State {
    /// Whether this binding initializes a native class object.
    pub fn is_native_class_constructor_binding(&self) -> bool {
        matches!(&self.ty, Type::Plugin { .. })
            && matches!(
                &self.initial,
                Expr::Call {
                    is_constructor: true,
                    return_type,
                    ..
                } if return_type == &self.ty
            )
    }

    /// Whether this immutable app binding constructs one native class object.
    /// Such objects need view-identity storage so recomposition does not
    /// silently replace the native instance.
    pub fn is_native_class_instance_binding(&self) -> bool {
        !self.mutable && self.is_native_class_constructor_binding()
    }
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
    pub background: Option<ColorValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HapticStyle {
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardDismissMode {
    Interactive,
    Never,
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
    Void,
    String,
    Bool,
    Numeric(NumericType),
    Optional(Box<Type>),
    Array(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Pair(Box<Type>, Box<Type>),
    Triple(Box<Type>, Box<Type>, Box<Type>),
    Enum(String),
    Plugin {
        namespace: String,
        name: String,
    },
    NetworkResponse,
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
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
    EnumValue {
        enum_name: String,
        case_name: String,
    },
    Add(Box<Expr>, Box<Expr>, NumericType),
    Not(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Contains {
        value: Box<Expr>,
        collection: Box<Expr>,
        collection_type: Type,
    },
    Array(Vec<Expr>),
    Set(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Pair(Box<Expr>, Box<Expr>),
    Triple(Box<Expr>, Box<Expr>, Box<Expr>),
    Call {
        name: String,
        arguments: Vec<Expr>,
        return_type: Type,
        is_async: bool,
        is_constructor: bool,
    },
    CollectionTransform {
        operation: CollectionTransform,
        collection: Box<Expr>,
        initial: Option<Box<Expr>>,
        closure: Box<Expr>,
    },
    Closure {
        parameters: Vec<String>,
        body: Box<Expr>,
    },
    NativeCall {
        receiver: Option<Box<Expr>>,
        namespace: String,
        name: String,
        arguments: Vec<(String, Expr)>,
        return_type: Type,
        is_async: bool,
        is_throwing: bool,
    },
    Index {
        collection: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
        collection_type: Type,
        element_type: Type,
    },
    Member {
        base: Box<Expr>,
        name: String,
        optional: bool,
        base_type: Type,
        field_type: Type,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        step: Option<Box<Expr>>,
    },
    Null(Type),
    Coalesce(Box<Expr>, Box<Expr>),
    Await(Box<Expr>),
    /// An async throwing call inside an explicit Nexa try/catch action.
    TryAwait(Box<Expr>),
    IsRegularWidth,
    IsCompactWidth,
    IsRegularHeight,
    IsCompactHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionTransform {
    Map,
    Filter,
    Reduce,
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
    Contains,
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
        asynchronous: bool,
    },
    OnDisappear {
        actions: Vec<Action>,
    },
    OnActive {
        actions: Vec<Action>,
    },
    OnInactive {
        actions: Vec<Action>,
    },
    OnBackground {
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
        icon: Option<String>,
        loading: Option<Expr>,
        disabled: Option<Expr>,
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
        focused: Option<String>,
        max_length: Option<i32>,
        actions: Vec<Action>,
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
        disabled: Expr,
        haptic: Option<HapticStyle>,
        children: Vec<Node>,
        actions: Vec<Action>,
        long_press_actions: Vec<Action>,
    },
    NavigationStack {
        root: ScreenId,
        arguments: Vec<Expr>,
    },
    NavigationLink {
        destination: ScreenId,
        arguments: Vec<Expr>,
        guard: Option<Expr>,
        children: Vec<Node>,
    },
    NavigationBack {
        label: Expr,
    },
    Link {
        url: Expr,
        children: Vec<Node>,
    },
    Accessibility {
        label: Expr,
        hint: Option<Expr>,
        role: AccessibilityRole,
        children: Vec<Node>,
    },
    KeyboardAware {
        dismiss: KeyboardDismissMode,
        children: Vec<Node>,
    },
    BottomSheet {
        state: String,
        partial: bool,
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
        axis: ListAxis,
        item_extent: Option<f32>,
        index: String,
        item: Option<String>,
        key: Option<Expr>,
        scroll_position: Option<String>,
        section: Option<String>,
        children: Vec<Node>,
        on_end_reached: Option<Vec<Action>>,
        on_scroll: Option<Vec<Action>>,
        sticky_header: Option<Vec<Node>>,
        section_header: Option<Vec<Node>>,
        refresh: Option<FastListRefresh>,
    },
    If {
        condition: Expr,
        then_body: Vec<Node>,
        else_body: Option<Vec<Node>>,
    },
    When {
        value: Expr,
        cases: Vec<WhenCase>,
        else_body: Vec<Node>,
    },
    Content,
    ComponentCall {
        name: String,
        arguments: Vec<(String, Expr)>,
        children: Option<Vec<Node>>,
    },
    NativeComponentCall {
        namespace: String,
        name: String,
        arguments: Vec<(String, Expr)>,
        children: Option<Vec<Node>>,
        event_handlers: Vec<NativeComponentEventHandler>,
    },
}

#[derive(Clone, Debug)]
pub struct FastListRefresh {
    pub state: String,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug)]
pub struct BottomBarTab {
    pub index: i32,
    pub label: String,
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug)]
pub struct WhenCase {
    pub value: Expr,
    pub body: Vec<Node>,
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
    Sections {
        collection: Expr,
        element_type: Type,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListAxis {
    Vertical,
    Horizontal,
    Grid { columns: u32 },
}

#[derive(Clone, Copy, Debug)]
pub enum LayoutKind {
    Column,
    Row,
    Stack,
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
    RemoteUrl(Expr),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ViewStyle {
    pub alignment: Option<Alignment>,
    pub padding: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub background: Option<ColorValue>,
    pub corner_radius: Option<f32>,
    pub border_color: Option<ColorValue>,
    pub border_width: Option<f32>,
    pub opacity: Option<f32>,
    pub animation: Option<AnimationSpec>,
}

impl ViewStyle {
    /// Whether the style has visual properties emitted through a native modifier.
    pub fn has_modifiers(&self) -> bool {
        self.padding.is_some()
            || self.width.is_some()
            || self.height.is_some()
            || self.min_width.is_some()
            || self.max_width.is_some()
            || self.min_height.is_some()
            || self.max_height.is_some()
            || self.background.is_some()
            || self.corner_radius.is_some()
            || self.border_color.is_some()
            || self.border_width.is_some()
            || self.opacity.is_some()
            || self.animation.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationSpec {
    Spring,
    EaseIn,
    EaseOut,
    EaseInOut,
    Linear,
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
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub selectable: bool,
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
    Expression(Expr),
    Assign {
        name: String,
        value: Expr,
    },
    NativePropertyAssign {
        receiver: Expr,
        property: String,
        value: Expr,
    },
    NativeEventSubscribe {
        receiver: Expr,
        property: String,
        parameters: Vec<String>,
        actions: Vec<Action>,
    },
    CollectionMutation {
        name: String,
        operation: CollectionMutation,
        arguments: Vec<Expr>,
    },
    If {
        condition: Expr,
        then_branch: Vec<Action>,
        else_branch: Option<Vec<Action>>,
    },
    For {
        name: String,
        iterable: Expr,
        body: Vec<Action>,
    },
    ForMap {
        key_name: String,
        value_name: String,
        iterable: Expr,
        body: Vec<Action>,
    },
    While {
        condition: Expr,
        body: Vec<Action>,
    },
    TryCatch {
        body: Vec<Action>,
        error_catches: Vec<ErrorCatchArm>,
        catch_body: Option<Vec<Action>>,
    },
    Break,
    Continue,
}

#[derive(Clone, Debug)]
pub struct ErrorCatchArm {
    pub namespace: String,
    pub error_type: String,
    pub variant: String,
    /// Each payload binding stores its local name, IDL property name, and type.
    pub parameters: Vec<(String, String, Type)>,
    pub body: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionMutation {
    ArrayAppend,
    ArrayRemoveAt,
    SetInsert,
    SetRemove,
    MapSet,
    MapRemove,
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
            Self::Void => "Void".to_owned(),
            Self::String => "String".to_owned(),
            Self::Bool => "Bool".to_owned(),
            Self::Numeric(n) => n.swift().to_owned(),
            Self::Optional(inner) => format!("{}?", inner.swift()),
            Self::Array(element) => format!("[{}]", element.swift()),
            Self::Set(element) => format!("Set<{}>", element.swift()),
            Self::Map(key, value) => format!("[{}: {}]", key.swift(), value.swift()),
            Self::Pair(first, second) => format!("({}, {})", first.swift(), second.swift()),
            Self::Triple(first, second, third) => {
                format!("({}, {}, {})", first.swift(), second.swift(), third.swift())
            }
            Self::Enum(name) => native_enum_name(name),
            Self::Plugin { name, .. } => name.clone(),
            Self::NetworkResponse => "NexaNetworkResponse".to_owned(),
            Self::Struct { name, .. } => native_struct_name(name),
        }
    }
    pub fn kotlin(&self) -> String {
        match self {
            Self::Void => "Unit".to_owned(),
            Self::String => "String".to_owned(),
            Self::Bool => "Boolean".to_owned(),
            Self::Numeric(n) => n.kotlin().to_owned(),
            Self::Optional(inner) => format!("{}?", inner.kotlin()),
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
            Self::Enum(name) => native_enum_name(name),
            Self::Plugin { name, .. } => name.clone(),
            Self::NetworkResponse => "NexaNetworkResponse".to_owned(),
            Self::Struct { name, .. } => native_struct_name(name),
        }
    }
}

fn native_enum_name(name: &str) -> String {
    let mut result = String::from("Nexa");
    let mut uppercase = true;
    for character in name.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

fn native_struct_name(name: &str) -> String {
    let mut result = String::from("Nexa");
    let mut uppercase = true;
    for character in name.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    if result == "Nexa" {
        result.push_str("Struct");
    }
    result
}

#[cfg(test)]
mod state_tests {
    use super::{Expr, State, Type};

    #[test]
    fn recognizes_immutable_native_class_instances_for_view_storage() {
        let class = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let binding = State {
            name: "player".to_owned(),
            ty: class.clone(),
            initial: Expr::Call {
                name: "VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: class,
                is_async: false,
                is_constructor: true,
            },
            mutable: false,
        };

        assert!(binding.is_native_class_instance_binding());
    }

    #[test]
    fn recognizes_mutable_native_class_constructors_without_marking_them_immutable() {
        let class = Type::Plugin {
            namespace: "Video".to_owned(),
            name: "VideoPlayer".to_owned(),
        };
        let binding = State {
            name: "player".to_owned(),
            ty: class.clone(),
            initial: Expr::Call {
                name: "VideoPlayer".to_owned(),
                arguments: Vec::new(),
                return_type: class,
                is_async: false,
                is_constructor: true,
            },
            mutable: true,
        };

        assert!(binding.is_native_class_constructor_binding());
        assert!(!binding.is_native_class_instance_binding());
    }

    #[test]
    fn does_not_treat_enum_values_as_native_class_instances() {
        let binding = State {
            name: "state".to_owned(),
            ty: Type::Plugin {
                namespace: "Video".to_owned(),
                name: "PlayerState".to_owned(),
            },
            initial: Expr::EnumValue {
                enum_name: "PlayerState".to_owned(),
                case_name: "idle".to_owned(),
            },
            mutable: false,
        };

        assert!(!binding.is_native_class_instance_binding());
    }
}
