use serde::{Deserialize, Serialize};

pub mod capabilities;
pub mod facts;
pub mod walk;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// A native plugin referenced by the module: identity plus a pointer to its
/// IDL contract. Packaging and project facts (sources, frameworks, resources,
/// permissions, manifests) live in the CLI's `PluginPackage` model, not in
/// semantic IR: lowering only needs to know *which* plugins exist, while
/// scaffolding decides *what* to copy, link, and declare from their manifests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plugin {
    pub namespace: String,
    pub idl_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginAsset {
    pub root: String,
    pub package_root: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeComponentEventHandler {
    pub property: String,
    pub parameters: Vec<String>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub cases: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<FunctionParameter>,
    pub locals: Vec<FunctionLocal>,
    pub return_type: Type,
    pub body: Expr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionLocal {
    pub name: String,
    pub ty: Type,
    pub initial: Expr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub source_file: Option<String>,
    pub parameters: Vec<ComponentParameter>,
    pub states: Vec<State>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentParameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenId(pub usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusBarConfig {
    pub style: StatusBarStyle,
    pub hidden: bool,
    pub background: Option<ColorValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HapticStyle {
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyboardDismissMode {
    Interactive,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectionConfig {
    pub style: DirectionStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectionStyle {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusBarStyle {
    Default,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Void,
    String,
    Bool,
    Numeric(NumericType),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Validated `Network.fetch` call. Semantic lowering guarantees every
    /// field, so renderers never look arguments up by name.
    NetworkFetch(Box<NetworkRequest>),
    /// Validated `Network.download` call.
    NetworkDownload {
        destination: Box<Expr>,
        request: Box<NetworkRequest>,
    },
    /// Validated `Path.join` call.
    PathJoin {
        path: Box<Expr>,
        component: Box<Expr>,
    },
    /// Validated `File.exists` call.
    FileExists {
        path: Box<Expr>,
    },
    /// Validated `File.readText` call.
    FileReadText {
        path: Box<Expr>,
    },
    /// Validated `File.writeText` call.
    FileWriteText {
        path: Box<Expr>,
        contents: Box<Expr>,
    },
    /// Validated `File.delete` call.
    FileDelete {
        path: Box<Expr>,
    },
    /// Validated permission query: `Status` renders `status`, `Request`
    /// renders `request`.
    PermissionOp {
        op: PermissionOpKind,
        permission: Box<Expr>,
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
        kind: MemberKind,
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
    ResultOk {
        value: Box<Expr>,
        value_type: Type,
        error_type: Type,
    },
    ResultErr {
        error: Box<Expr>,
        value_type: Type,
        error_type: Type,
    },
    Try {
        expr: Box<Expr>,
        value_type: Type,
        error_type: Type,
    },
    IsRegularWidth,
    IsCompactWidth,
    IsRegularHeight,
    IsCompactHeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionTransform {
    Map,
    Filter,
    Reduce,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterpolatedPart {
    Literal(String),
    Value(Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        plan: ListPlan,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FastListRefresh {
    pub state: String,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BottomBarTab {
    pub index: i32,
    pub label: String,
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhenCase {
    pub value: Expr,
    pub body: Vec<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessibilityRole {
    None,
    Button,
    Link,
    Header,
    Image,
}

/// Options shared by the flat (count- and collection-backed) list plans.
/// Every field is independently optional; only the plan variant decides
/// which bindings exist.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListCommon {
    pub axis: ListAxis,
    pub item_extent: Option<f32>,
    /// Row index binding name (always `Int32`).
    pub index: String,
    pub key: Option<Expr>,
    pub scroll_position: Option<String>,
    pub children: Vec<Node>,
    pub on_end_reached: Option<Vec<Action>>,
    pub on_scroll: Option<Vec<Action>>,
    pub sticky_header: Option<Vec<Node>>,
    pub refresh: Option<FastListRefresh>,
}

/// Options for the sectioned list plan. Sectioned lists always render
/// vertically and use `section_header` instead of `sticky_header`; they
/// support neither scroll observers nor a scroll position, so those fields
/// cannot be represented here at all.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionedListCommon {
    pub item_extent: Option<f32>,
    /// Row index binding name (always `Int32`).
    pub index: String,
    pub key: Option<Expr>,
    pub children: Vec<Node>,
    pub section_header: Option<Vec<Node>>,
    pub refresh: Option<FastListRefresh>,
}

/// A validated virtualized-list plan. Semantic lowering is the only
/// constructor: each variant carries exactly the bindings its source
/// provides, so renderers never unwrap optional bindings or re-dispatch on
/// the source shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ListPlan {
    Count {
        count: Expr,
        common: ListCommon,
    },
    Items {
        collection: Expr,
        element_type: Type,
        /// Row item binding name.
        item: String,
        common: ListCommon,
    },
    Sections {
        collection: Expr,
        element_type: Type,
        /// Section index binding name (always `Int32`).
        section: String,
        /// Row item binding name.
        item: String,
        common: SectionedListCommon,
    },
}

impl ListPlan {
    /// Row children for analysis passes.
    pub fn children(&self) -> &[Node] {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => &common.children,
            ListPlan::Sections { common, .. } => &common.children,
        }
    }

    /// Row key expression, if any.
    pub fn key(&self) -> Option<&Expr> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.key.as_ref(),
            ListPlan::Sections { common, .. } => common.key.as_ref(),
        }
    }

    /// Scroll position binding, if any. Sectioned lists cannot have one.
    pub fn scroll_position(&self) -> Option<&str> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.scroll_position.as_deref()
            }
            ListPlan::Sections { .. } => None,
        }
    }

    /// End-reached actions, if any. Sectioned lists cannot have them.
    pub fn on_end_reached(&self) -> Option<&[Action]> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.on_end_reached.as_deref()
            }
            ListPlan::Sections { .. } => None,
        }
    }

    /// Scroll actions, if any. Sectioned lists cannot have them.
    pub fn on_scroll(&self) -> Option<&[Action]> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.on_scroll.as_deref()
            }
            ListPlan::Sections { .. } => None,
        }
    }

    /// Sticky header content, if any. Sectioned lists use `section_header`.
    pub fn sticky_header(&self) -> Option<&[Node]> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.sticky_header.as_deref()
            }
            ListPlan::Sections { .. } => None,
        }
    }

    /// Section header content, if any. Only sectioned lists can have one.
    pub fn section_header(&self) -> Option<&[Node]> {
        match self {
            ListPlan::Count { .. } | ListPlan::Items { .. } => None,
            ListPlan::Sections { common, .. } => common.section_header.as_deref(),
        }
    }

    /// Pull-to-refresh state, if any.
    pub fn refresh(&self) -> Option<&FastListRefresh> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.refresh.as_ref()
            }
            ListPlan::Sections { common, .. } => common.refresh.as_ref(),
        }
    }

    /// Mutable slot for the pull-to-refresh state attached by refresh
    /// lowering after the plan is built.
    pub fn refresh_slot(&mut self) -> &mut Option<FastListRefresh> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => &mut common.refresh,
            ListPlan::Sections { common, .. } => &mut common.refresh,
        }
    }

    /// Row height override, if any.
    pub fn item_extent(&self) -> Option<f32> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.item_extent,
            ListPlan::Sections { common, .. } => common.item_extent,
        }
    }

    /// Layout axis. Sectioned lists always lay out vertically.
    pub fn axis(&self) -> ListAxis {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.axis,
            ListPlan::Sections { .. } => ListAxis::Vertical,
        }
    }

    /// Row index binding name.
    pub fn index(&self) -> &str {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => &common.index,
            ListPlan::Sections { common, .. } => &common.index,
        }
    }
}

/// Validated member-access kind, computed once by semantic lowering from the
/// base type and member name. Renderers match on this instead of
/// re-deriving legality from `(base_type, name)` pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TuplePosition {
    First,
    Second,
    Third,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberKind {
    /// `Pair.first`/`second`, `Triple.first`/`second`/`third`.
    TupleIndex(TuplePosition),
    /// Struct field access; carries the source field name.
    StructField(String),
    /// Native class property access; carries the source property name.
    PluginField(String),
    /// `NetworkResponse.statusCode`.
    NetworkStatusCode,
    /// `NetworkResponse.headers`.
    NetworkHeaders,
    /// `NetworkResponse.body`.
    NetworkBody,
}

/// Validated `Network.fetch` / `Network.download` arguments. Every field is
/// required (defaults are applied by semantic lowering); `body` is `None`
/// when the argument was absent or the `Null` literal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: Box<Expr>,
    pub method: Box<Expr>,
    pub headers: Box<Expr>,
    pub timeout: Box<Expr>,
    pub use_cache: Box<Expr>,
    pub follow_redirects: Box<Expr>,
    pub max_response_bytes: Box<Expr>,
    pub certificate_pins: Box<Expr>,
    pub body: Option<Box<Expr>>,
}

/// Validated permission query kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionOpKind {
    Status,
    Request,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListAxis {
    Vertical,
    Horizontal,
    Grid { columns: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutKind {
    Column,
    Row,
    Stack,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyboardType {
    Text,
    Number,
    Email,
    Phone,
    Url,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capitalization {
    None,
    Sentences,
    Words,
    Characters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageScale {
    Fit,
    Fill,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ImageSource {
    Asset(String),
    RemoteUrl(Expr),
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationSpec {
    Spring,
    EaseIn,
    EaseOut,
    EaseInOut,
    Linear,
}

/// Cross-axis alignment for a native row or column layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorValue {
    Static(Color),
    Adaptive { light: Color, dark: Color },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct TextStyle {
    pub color: Option<ColorValue>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub line_limit: Option<i32>,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub selectable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorCatchArm {
    pub namespace: String,
    pub error_type: String,
    pub variant: String,
    /// Each payload binding stores its local name, IDL property name, and type.
    pub parameters: Vec<(String, String, Type)>,
    pub body: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionMutation {
    ArrayAppend,
    ArrayRemoveAt,
    SetInsert,
    SetRemove,
    MapSet,
    MapRemove,
}

impl Expr {
    /// Whether this expression is a call that can throw at runtime and must
    /// be awaited with error propagation (`try await` in Swift). This is the
    /// single source of truth shared by `await` rendering and throwing-call
    /// analysis; the call shapes themselves carry no flags to misread.
    pub fn is_throwing_call(&self) -> bool {
        matches!(
            self,
            Expr::NativeCall {
                is_throwing: true,
                ..
            } | Expr::NetworkFetch(_)
                | Expr::NetworkDownload { .. }
                | Expr::FileReadText { .. }
                | Expr::FileWriteText { .. }
                | Expr::FileDelete { .. }
        )
    }
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
