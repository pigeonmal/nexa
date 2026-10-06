use serde::{Deserialize, Serialize};

pub mod capabilities;
pub mod facts;
pub mod localization;
pub mod system_icons;
pub mod walk;

pub use system_icons::SystemIcon;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub app_name: String,
    pub plugins: Vec<Plugin>,
    pub plugin_assets: Vec<PluginAsset>,
    pub enums: Vec<EnumDecl>,
    pub structs: Vec<StructDecl>,
    pub functions: Vec<Function>,
    #[serde(default)]
    pub background_tasks: Vec<BackgroundTask>,
    pub states: Vec<State>,
    /// Immutable process-lifetime module bindings, emitted outside UI trees.
    pub globals: Vec<State>,
    pub screens: Vec<Screen>,
    /// Cross-platform widget declarations compiled from `.nx` source.
    #[serde(default)]
    pub widgets: Vec<Widget>,
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
    Motion,
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
    /// Present for an instance user-defined class method. The receiver type
    /// carries the class's immutable constructor properties.
    pub receiver: Option<Type>,
    /// Immutable extra stored-property initializers for a class method's owner.
    /// Constructor parameters are stored in the receiver type itself.
    pub class_initializers: Vec<FunctionLocal>,
    pub is_async: bool,
    /// Whether this function propagates a native error (`throws` on Swift;
    /// Kotlin exceptions propagate without a declaration).
    #[serde(default)]
    pub is_throwing: bool,
    pub parameters: Vec<FunctionParameter>,
    pub locals: Vec<FunctionLocal>,
    pub return_type: Type,
    pub body: Expr,
    /// Statement body for functions that require control flow, expression
    /// statements, or multiple returns. Simple functions retain the compact
    /// locals + expression representation above.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_actions: Option<Vec<Action>>,
}

/// A periodic app task emitted as a native background worker.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub name: String,
    pub identifier: String,
    pub interval_minutes: u32,
    pub actions: Vec<Action>,
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
    #[serde(default)]
    pub on_appear: Option<Vec<Action>>,
    #[serde(default)]
    pub on_appear_async: bool,
    #[serde(default)]
    pub on_disappear: Option<Vec<Action>>,
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

/// Typed, platform-neutral widget UI and immutable timeline entry shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Widget {
    pub name: String,
    /// Optional source text for native widget gallery metadata.
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Optional source text for the configuration UI.
    #[serde(default)]
    pub configuration_title: Option<String>,
    #[serde(default)]
    pub configuration_description: Option<String>,
    /// Optional user-configurable value and its statically resolved struct type.
    /// The default expression remains typed IR so each platform can generate
    /// its native widget-configuration mechanism without runtime reflection.
    #[serde(default)]
    pub configuration: Option<WidgetConfiguration>,
    /// Statically resolved provider used to produce timeline entries.
    pub entry_provider: Expr,
    /// Synchronous provider used for native placeholders and provider errors.
    #[serde(default)]
    pub placeholder_provider: Option<Expr>,
    /// Whether the entry provider contains an awaited operation.
    #[serde(default)]
    pub entry_provider_async: bool,
    /// Whether the entry provider contains a throwing awaited operation.
    #[serde(default)]
    pub entry_provider_throws: bool,
    pub entry_type: Type,
    pub families: Vec<WidgetFamily>,
    /// Minimum requested refresh delay. Native schedulers may defer the update.
    pub refresh_seconds: u32,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetConfiguration {
    pub ty: Type,
    pub default: Expr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetFamily {
    Small,
    Medium,
    Large,
    ExtraLarge,
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
pub enum ArithmeticOp {
    Subtract,
    Multiply,
    Divide,
    Remainder,
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
    /// A raw byte buffer, spelled `Data` on Swift and `ByteArray` on Kotlin.
    ///
    /// `Bytes` is deliberately distinct from `Array<UInt8>`: the platform
    /// byte containers are the only zero-conversion spellings of binary data
    /// across a plugin boundary, and `Array<UInt8>` lowers to a boxed
    /// element list on Kotlin. Keeping them separate means a contract, the
    /// generated native code, and `.nx` code all agree on one spelling.
    Bytes,
    Bool,
    Numeric(NumericType),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Array(Box<Type>),
    Set(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Pair(Box<Type>, Box<Type>),
    Triple(Box<Type>, Box<Type>, Box<Type>),
    Signal(Box<Type>),
    Enum(String),
    /// A plugin method type parameter, present only in signatures. Every call
    /// site resolves it to a concrete value type before the IR is built, so
    /// code generation never sees this variant.
    TypeParam(String),
    Plugin {
        namespace: String,
        name: String,
    },
    /// A native cancellation handle: Swift `Task<Void, Never>` / Kotlin `Job`.
    /// It is intentionally not a plugin codec or serializable app value.
    TaskHandle,
    NetworkResponse,
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    /// User-defined reference identity. Unlike `Struct`, class equality is
    /// identity-based and instances may retain native plugin objects.
    Class {
        name: String,
        fields: Vec<(String, Type)>,
        constructor_parameter_count: usize,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expr {
    /// The implicit instance receiver inside a user class method.
    This(Type),
    String(String),
    /// A static UI text value that uses its source template as its localization
    /// key. Dynamic interpolation arguments remain typed child expressions.
    LocalizedText {
        key: String,
        value: Box<Expr>,
        comment: Option<String>,
    },
    Interpolation(Vec<InterpolatedPart>),
    Bool(bool),
    Number {
        raw: String,
        ty: NumericType,
    },
    State(String, Type),
    /// A floating-point state read used for presentation in Compose. The
    /// logical state remains the source of truth and action expressions keep
    /// using `State` directly.
    AnimatedState(String, Type),
    EnumValue {
        enum_name: String,
        case_name: String,
    },
    /// A case declared by a native plugin enum contract.
    PluginEnumValue {
        namespace: String,
        enum_name: String,
        case_name: String,
    },
    /// A typed payload-bearing enum case declared by a native plugin.
    PluginEnumConstructor {
        namespace: String,
        enum_name: String,
        case_name: String,
        payload_names: Vec<String>,
        arguments: Vec<Expr>,
        return_type: Type,
    },
    /// Maps a nullable scalar to a payload-bearing plugin enum case without
    /// evaluating the nullable expression more than once.
    PluginEnumOptionalConstructor {
        namespace: String,
        enum_name: String,
        case_name: String,
        null_case_name: String,
        payload_name: String,
        value: Box<Expr>,
        return_type: Type,
    },
    Add(Box<Expr>, Box<Expr>, NumericType),
    Concat(Box<Expr>, Box<Expr>),
    Arithmetic {
        op: ArithmeticOp,
        left: Box<Expr>,
        right: Box<Expr>,
        ty: NumericType,
    },
    Negate {
        value: Box<Expr>,
        ty: NumericType,
    },
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
        #[serde(default)]
        is_throwing: bool,
        is_constructor: bool,
    },
    CollectionTransform {
        operation: CollectionTransform,
        collection: Box<Expr>,
        initial: Option<Box<Expr>>,
        closure: Box<Expr>,
    },
    CollectionUtility {
        operation: CollectionUtilityKind,
        collection: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
        element_type: Type,
    },
    Conditional {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
        value_type: Type,
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
        /// Value codecs for a generic plugin call, in the order the generated
        /// contract declares its codec parameters. Empty for every call whose
        /// signature has no type parameters.
        codecs: Vec<PluginCodec>,
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
    /// Validated `Bytes.fromText` call: a `String` rendered directly as the
    /// platform byte container, so no intermediate array is allocated.
    BytesFromText {
        text: Box<Expr>,
    },
    /// Validated `Bytes.fromArray` call.
    BytesFromArray {
        values: Box<Expr>,
    },
    /// Validated `Bytes.count` call.
    BytesCount {
        bytes: Box<Expr>,
    },
    /// Validated `Time.*` call.
    ///
    /// The clock is a core API, not a plugin, so it has its own variant rather
    /// than borrowing the plugin call shape: both backends render a direct
    /// platform call, and the hot-reload interpreter can evaluate it.
    TimeCall {
        method: TimeMethod,
        arguments: Vec<Expr>,
        return_type: Type,
        is_async: bool,
    },
    /// A typed write to the platform log.
    LogCall {
        method: LogMethod,
        message: Box<Expr>,
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
    FlatMap,
    Filter,
    Reduce,
    SortedBy,
    GroupedBy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionUtilityKind {
    Random,
    First,
    Last,
    Shuffled,
    Reverse,
    Slice,
    Take,
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
    Appearance {
        mode: Expr,
        children: Vec<Node>,
    },
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
    /// Native settings-style container. Child sections map to SwiftUI Form
    /// sections and to Material settings groups on Android.
    Form {
        children: Vec<Node>,
    },
    FormSection {
        title: Option<Expr>,
        footer: Option<Expr>,
        children: Vec<Node>,
    },
    Text {
        value: Expr,
        style: TextStyle,
    },
    /// Empty-state content rendered with native platform presentation.
    ContentUnavailable {
        title: Expr,
        icon: SystemIcon,
        description: Expr,
    },
    Spacer,
    Divider {
        color: ColorValue,
        thickness: f32,
    },
    Button {
        label: Expr,
        icon: Option<SystemIcon>,
        loading: Option<Expr>,
        disabled: Option<Expr>,
        style: Option<ButtonStyle>,
        size: Option<ButtonSize>,
        shape: Option<ButtonShape>,
        tint: Option<ColorExpression>,
        glass: bool,
        actions: Vec<Action>,
    },
    TextInput {
        state: String,
        placeholder: String,
        /// Optional translator context for the placeholder.
        #[serde(default)]
        comment: Option<String>,
        keyboard: KeyboardType,
        secure: bool,
        multiline: bool,
        autofill: Option<AutofillType>,
        return_key: Option<ReturnKeyType>,
        autocorrect: Option<bool>,
        capitalization: Option<Capitalization>,
        focused: Option<String>,
        max_length: Option<i32>,
        font: Option<TextInputFont>,
        min_lines: Option<i32>,
        max_lines: Option<i32>,
        #[serde(default)]
        searchable: bool,
        actions: Vec<Action>,
        #[serde(default)]
        on_change: Option<TextInputChange>,
    },
    Switch {
        state: String,
        label: Expr,
    },
    Slider {
        state: String,
        /// Read the presentation value through Compose's native animator.
        animated: bool,
        min: f64,
        max: f64,
        step: f64,
    },
    ProgressBar {
        progress: Expr,
    },
    ProgressRing {
        progress: Expr,
    },
    SegmentedControl {
        items: Expr,
        state: String,
    },
    Picker {
        items: Expr,
        state: String,
        icon: Option<SystemIcon>,
        label: Option<Expr>,
    },
    DatePicker {
        timestamp_state: String,
        has_time_state: String,
    },
    Image {
        source: ImageSource,
        description: String,
        scale: ImageScale,
        placeholder: Option<String>,
        max_height: Option<f32>,
        shared_element: Option<Expr>,
    },
    SystemIcon {
        icon: SystemIcon,
        description: String,
        size: f32,
        tint: ColorExpression,
    },
    LinearGradient {
        start_color: ColorValue,
        end_color: ColorValue,
        direction: GradientDirection,
        height: f32,
    },
    Pressable {
        disabled: Expr,
        haptic: Option<HapticStyle>,
        fill_max_size: bool,
        children: Vec<Node>,
        actions: Vec<Action>,
        double_tap_actions: Vec<Action>,
        long_press_duration_ms: Expr,
        long_press_actions: Vec<Action>,
        #[serde(default)]
        context_menu: Vec<Node>,
        drag_parameters: Vec<String>,
        drag_actions: Vec<Action>,
        pinch_parameter: Option<String>,
        pinch_actions: Vec<Action>,
    },
    NavigationStack {
        root: ScreenId,
        arguments: Vec<Expr>,
    },
    NavigationSplitView {
        detail_visible: String,
        sidebar: Vec<Node>,
        detail: Vec<Node>,
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
        /// Optional current value announced separately from the label where
        /// the native accessibility system supports that distinction.
        #[serde(default)]
        value: Option<Expr>,
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
        #[serde(default)]
        large_only: bool,
        #[serde(default)]
        title: Option<Expr>,
        children: Vec<Node>,
    },
    Dialog {
        state: String,
        title: Expr,
        message: Expr,
        children: Vec<Node>,
    },
    ConfirmationDialog {
        state: String,
        title: Expr,
        children: Vec<Node>,
    },
    RefreshControl {
        state: String,
        children: Vec<Node>,
        actions: Vec<Action>,
    },
    AppBottomBar {
        state: String,
        tint: Option<ColorExpression>,
        tabs: Vec<BottomBarTab>,
    },
    PagePager {
        state: String,
        pages: Vec<Vec<Node>>,
    },
    Toolbar {
        placement: ToolbarPlacement,
        children: Vec<Node>,
    },
    FastList {
        plan: ListPlan,
    },
    If {
        condition: Expr,
        then_body: Vec<Node>,
        else_body: Option<Vec<Node>>,
        transition: Option<ViewTransition>,
    },
    When {
        value: Expr,
        cases: Vec<WhenCase>,
        else_body: Vec<Node>,
        transition: Option<ViewTransition>,
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
pub struct TextInputChange {
    pub parameter: String,
    pub actions: Vec<Action>,
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
    #[serde(default)]
    pub comment: Option<String>,
    pub icon: Option<SystemIcon>,
    pub badge: Option<String>,
    pub role: Option<String>,
    /// Optional native navigation title for this destination.
    #[serde(default)]
    pub navigation_title: Option<String>,
    /// Use the native large title presentation when a navigation title exists.
    #[serde(default)]
    pub large_title: bool,
    /// Mutable string state bound to the native navigation search field.
    #[serde(default)]
    pub search_state: Option<String>,
    /// Native navigation search prompt.
    #[serde(default)]
    pub search_prompt: Option<String>,
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
    /// Render a vertical collection with the platform's native list control.
    /// On iOS this emits SwiftUI `List`; Android keeps the Compose lazy list.
    #[serde(default)]
    pub native: bool,
    /// Invert a flat vertical list so logical rows appear bottom-to-top.
    #[serde(default)]
    pub reverse_layout: bool,
    /// Snap flat vertical rows to exactly one viewport per page.
    #[serde(default)]
    pub page_snap: bool,
    pub item_extent: Option<f32>,
    /// Row index binding name (always `Int32`).
    pub index: String,
    pub key: Option<Expr>,
    pub scroll_position: Option<String>,
    pub children: Vec<Node>,
    pub on_end_reached: Option<Vec<Action>>,
    pub on_scroll: Option<Vec<Action>>,
    /// Drag-reorder callback for native flat collection lists.
    #[serde(default)]
    pub on_move: Option<FastListMove>,
    #[serde(default)]
    pub swipe_actions: Option<Vec<Node>>,
    pub sticky_header: Option<Vec<Node>>,
    pub refresh: Option<FastListRefresh>,
}

/// Callback payload for a flat-list row move. Indexes use zero-based logical
/// positions in the displayed rows, independent of reverse visual layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FastListMove {
    pub from: String,
    pub to: String,
    /// Whether native move affordances are enabled for the current list state.
    pub enabled: Expr,
    pub actions: Vec<Action>,
}

/// Options for the sectioned list plan. Sectioned lists always render
/// vertically and use `section_header` instead of `sticky_header`; they
/// support neither scroll observers nor a scroll position, so those fields
/// cannot be represented here at all.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionedListCommon {
    /// Use the platform's standard list container where the backend supports it.
    #[serde(default)]
    pub native: bool,
    pub item_extent: Option<f32>,
    /// Row index binding name (always `Int32`).
    pub index: String,
    pub key: Option<Expr>,
    pub children: Vec<Node>,
    pub section_header: Option<Vec<Node>>,
    #[serde(default)]
    pub swipe_actions: Option<Vec<Node>>,
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
    /// Whether a vertical list opts into the platform's standard list control
    /// instead of Nexa's custom virtualized surface.
    pub fn native(&self) -> bool {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.native,
            ListPlan::Sections { common, .. } => common.native,
        }
    }

    /// Whether a flat list is laid out from its logical end toward its start.
    pub fn reverse_layout(&self) -> bool {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.reverse_layout
            }
            ListPlan::Sections { .. } => false,
        }
    }

    /// Whether flat vertical rows occupy and snap to the full viewport.
    pub fn page_snap(&self) -> bool {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => common.page_snap,
            ListPlan::Sections { .. } => false,
        }
    }

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

    /// Native reorder callback for flat lists.
    pub fn on_move(&self) -> Option<&FastListMove> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.on_move.as_ref()
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

    /// Native trailing swipe actions, if any.
    pub fn swipe_actions(&self) -> Option<&[Node]> {
        match self {
            ListPlan::Count { common, .. } | ListPlan::Items { common, .. } => {
                common.swipe_actions.as_deref()
            }
            ListPlan::Sections { common, .. } => common.swipe_actions.as_deref(),
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
    /// User-defined class property access; carries the source property name.
    ClassField(String),
    /// User-defined class-level immutable property access; carries the source property name.
    ClassStaticField(String),
    /// Native class property access; carries the source property name.
    PluginField(String),
    /// `NetworkResponse.statusCode`.
    NetworkStatusCode,
    /// `NetworkResponse.headers`.
    NetworkHeaders,
    /// `NetworkResponse.body`.
    NetworkBody,
    /// `Array<T>`, `Set<T>`, or `Map<K, V>.count`.
    CollectionCount,
    /// `Array<T>`, `Set<T>`, or `Map<K, V>.isEmpty`.
    CollectionIsEmpty,
    /// `String.trimmed` whitespace trimming property.
    StringTrimmed,
    /// `Signal<T>.value` reactive value property.
    SignalValue,
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

/// One value codec a generic plugin call must supply.
///
/// `ty` is the concrete type the call site bound; the backends name its
/// generated writer or reader from it. `decodes` distinguishes a read codec
/// from a write codec, matching the order of the codec parameters the
/// generated contract declares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCodec {
    pub ty: Type,
    pub decodes: bool,
    /// This call binds named row metadata to one generated struct constructor.
    /// It uses a row-mapper factory, not the binary value codec.
    #[serde(default)]
    pub row_mapper: bool,
}

/// A core clock operation.
///
/// The two clocks are deliberately separate: `Now` is a wall clock that can
/// jump, so it is for *when* something happened, and `Monotonic` never moves
/// backwards, so it is for *how long* something took.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeMethod {
    /// `Time.now()`: milliseconds since the Unix epoch, UTC.
    Now,
    /// `Time.monotonic()`: nanoseconds from an arbitrary fixed origin. Only
    /// differences between two readings mean anything.
    Monotonic,
    /// `Time.elapsed(since:)`: nanoseconds elapsed since a monotonic reading.
    Elapsed,
    /// `Time.sleep(milliseconds:)`: suspends the caller.
    Sleep,
    /// `Time.iso8601(timestamp:)`: the UTC instant as an ISO 8601 string.
    Iso8601,
    /// `Time.iso8601ToMillis(text:)`: the inverse, or null when malformed.
    Iso8601ToMillis,
    /// `Time.startOfDay(timestamp:)`: local calendar start of the timestamp's day.
    StartOfDay,
    /// `Time.addCalendarDays(timestamp:days:)`: advances by local calendar days,
    /// preserving the wall-clock time across daylight-saving transitions.
    AddCalendarDays,
    /// `Time.localizedDate(timestamp:)`: local date formatted using native locale conventions.
    LocalizedDate,
    /// `Time.localizedTime(timestamp:)`: local time formatted using native locale conventions.
    LocalizedTime,
    /// `Time.localizedDateTime(timestamp:)`: local date and time formatted using native locale conventions.
    LocalizedDateTime,
    /// `Time.format(timestamp, pattern)`: formats a local timestamp with a shared Foundation/Java date pattern.
    Format,
}

/// A core logging operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogMethod {
    Info,
    Warning,
    Error,
}

impl TimeMethod {
    /// The IDL-level name of the method, as `.nx` spells it.
    pub fn idl_name(self) -> &'static str {
        match self {
            TimeMethod::Now => "now",
            TimeMethod::Monotonic => "monotonic",
            TimeMethod::Elapsed => "elapsed",
            TimeMethod::Sleep => "sleep",
            TimeMethod::Iso8601 => "iso8601",
            TimeMethod::Iso8601ToMillis => "iso8601ToMillis",
            TimeMethod::StartOfDay => "startOfDay",
            TimeMethod::AddCalendarDays => "addCalendarDays",
            TimeMethod::LocalizedDate => "localizedDate",
            TimeMethod::LocalizedTime => "localizedTime",
            TimeMethod::LocalizedDateTime => "localizedDateTime",
            TimeMethod::Format => "format",
        }
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AutofillType {
    Username,
    Password,
    OneTimeCode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReturnKeyType {
    Done,
    Search,
    Send,
    Next,
}

/// Native, dynamic-type typography presets available to editable text fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextInputFont {
    Body,
    Title3,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradientDirection {
    TopToBottom,
    BottomToTop,
    LeadingToTrailing,
    TrailingToLeading,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ImageSource {
    Asset(String),
    RemoteUrl(Expr),
    LocalFile(Expr),
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
    pub effects: ViewEffects,
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
            || self.effects.has_modifiers()
            || self.animation.is_some()
    }
}

/// Static visual effects applied to one native view.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ViewEffects {
    pub scale: Option<f32>,
    pub rotation: Option<f32>,
    pub shadow: Option<ViewShadow>,
    pub blur: Option<f32>,
    pub clip_rounded: Option<f32>,
    pub z_index: Option<i32>,
    pub glass: Option<ViewGlass>,
}

impl ViewEffects {
    pub fn has_modifiers(&self) -> bool {
        self.scale.is_some()
            || self.rotation.is_some()
            || self.shadow.is_some()
            || self.blur.is_some()
            || self.clip_rounded.is_some()
            || self.z_index.is_some()
            || self.glass.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewGlass {
    pub tint: Option<ColorValue>,
    pub shape: GlassShape,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum GlassShape {
    Circle,
    Capsule,
    Rounded(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonStyle {
    BorderedProminent,
    Bordered,
    Borderless,
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonSize {
    Small,
    Regular,
    Large,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ButtonShape {
    Capsule,
    Circle,
    Rounded(f32),
}

/// Static rounded-rectangle shadow parameters shared by SwiftUI and Compose.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ViewShadow {
    pub radius: f32,
    pub x: f32,
    pub y: f32,
    pub color: ColorValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum AnimationSpec {
    Spring { response: f32, damping: f32 },
    EaseIn,
    EaseOut,
    EaseInOut,
    Linear,
}

/// Native appearance/removal effect for a conditional view block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewTransition {
    Fade,
    SlideFromBottom,
    SlideFromLeft,
    SlideFromRight,
    Scale,
}

/// Cross-axis alignment for a native row or column layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolbarPlacement {
    Leading,
    Trailing,
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

/// A native color supplied either as a compile-time color value or as a
/// runtime hexadecimal string expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ColorExpression {
    Static(ColorValue),
    Dynamic(Expr),
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct TextStyle {
    pub alignment: Option<TextAlignment>,
    pub color: Option<ColorValue>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub padding: Option<f32>,
    pub opacity: Option<f32>,
    pub effects: ViewEffects,
    pub line_limit: Option<i32>,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    #[serde(default)]
    pub strikethrough: bool,
    pub selectable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlignment {
    Leading,
    Center,
    Trailing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Expression(Expr),
    /// Immutable binding scoped to the current action block.
    Let {
        name: String,
        ty: Type,
        value: Expr,
    },
    /// Function return, including returns nested in control-flow actions.
    Return {
        value: Expr,
    },
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
    NetworkStatusSubscribe {
        parameter: String,
        actions: Vec<Action>,
    },
    CollectionMutation {
        name: String,
        operation: CollectionMutation,
        arguments: Vec<Expr>,
    },
    TaskLaunch {
        handle: Option<String>,
        executor: TaskExecutor,
        actions: Vec<Action>,
    },
    TaskCancel {
        handle: String,
    },
    /// Runs state updates in one native animation transaction.
    WithAnimation {
        animation: AnimationSpec,
        animated_states: Vec<String>,
        actions: Vec<Action>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskExecutor {
    Main,
    Background,
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
    ArrayMove,
    /// Moves an item in an order-preserving visible subset while keeping
    /// items outside that subset in their existing backing-array slots.
    ArrayMoveSubset,
    SetInsert,
    SetRemove,
    MapSet,
    MapRemove,
    /// Replaces a whole collection state with a new value.
    ///
    /// A collection state is a snapshot-state collection on Kotlin, so the
    /// value cannot simply be reassigned: the contents are replaced instead.
    /// The distinction lives in the IR so each backend spells it the way its
    /// platform requires, and so the hot-reload interpreter applies the same
    /// semantics as generated code.
    Replace,
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
            } | Expr::Call {
                is_throwing: true,
                ..
            } | Expr::NetworkFetch(_)
                | Expr::NetworkDownload { .. }
                | Expr::FileReadText { .. }
                | Expr::FileWriteText { .. }
                | Expr::FileDelete { .. }
                | Expr::TimeCall {
                    method: TimeMethod::Sleep,
                    ..
                }
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
                is_throwing: false,
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
                is_throwing: false,
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
