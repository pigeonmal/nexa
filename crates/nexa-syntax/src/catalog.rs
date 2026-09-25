//! Authoritative language catalog for the `.nx` surface vocabulary.
//!
//! This catalog is the single source of truth for built-in component *shape*:
//! argument names and requirements, child-block models, trailing dot-modifier
//! models, documentation keys, and feature tags (`COMPONENT_SCHEMAS`). The
//! parser validates invocations against these schemas and produces a generic
//! syntactic `ComponentInvocation`; semantic lowering converts each
//! invocation into the existing typed IR nodes, so generated code stays fully
//! specialized with no runtime maps or reflection.
//!
//! The IDE vocabulary tables (`COMPONENTS`, `KEYWORDS`, `TYPES`,
//! `DOT_MODIFIERS`) carry the human-facing summaries, snippets, and parse
//! probes. `component_schemas_cover_every_vocabulary_entry` keeps the two
//! tables in lockstep, and the probes guarantee every entry is accepted by
//! the parser — so completions and hover cannot drift from the grammar.
//!
//! Unsupported names must never be added here: `VStack`, `HStack`, `ZStack`
//! (SwiftUI aliases, not `.nx` components), `TextField` (the component is
//! `TextInput`), `FastSectionedList` (sectioned `FastList` uses the
//! `sections:` source), `Spacer`, and `Divider` have no parser production.

/// A UI component accepted by the parser's node dispatch.
pub struct ComponentEntry {
    /// Component name exactly as written in `.nx` source.
    pub name: &'static str,
    /// One-line summary shown in completion detail and hover.
    pub summary: &'static str,
    /// Snippet inserted on completion.
    pub snippet: &'static str,
    /// Full program that must parse; exercised by `catalog_tests`.
    pub probe: &'static str,
}

/// A keyword accepted by the grammar.
pub struct KeywordEntry {
    pub name: &'static str,
    pub kind: KeywordKind,
    pub summary: &'static str,
}

/// Editor-grammar bucket for a keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordKind {
    /// Control flow and statement keywords (`if`, `for`, `await`, ...).
    Control,
    /// Declaration keywords (`app`, `state`, `fn`, ...).
    Declaration,
}

/// A builtin type name resolved by semantic analysis.
pub struct TypeEntry {
    pub name: &'static str,
    pub summary: &'static str,
}

/// A trailing dot-modifier accepted after a component block.
pub struct DotModifierEntry {
    pub name: &'static str,
    pub summary: &'static str,
    pub snippet: &'static str,
}

pub const COMPONENTS: &[ComponentEntry] = &[
    ComponentEntry {
        name: "Column",
        summary: "Vertical layout container with optional spacing",
        snippet: "Column(spacing: ${1:8}) {\n    $0\n}",
        probe: "app P { body { Column(spacing: 8) { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Row",
        summary: "Horizontal layout container with optional spacing",
        snippet: "Row(spacing: ${1:8}) {\n    $0\n}",
        probe: "app P { body { Row(spacing: 8) { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Stack",
        summary: "Overlapping layout container",
        snippet: "Stack {\n    $0\n}",
        probe: "app P { body { Stack { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Text",
        summary: "Displays formatted text",
        snippet: "Text(\"${1:Label}\")",
        probe: "app P { body { Text(\"a\", fontSize: 18) } }",
    },
    ComponentEntry {
        name: "Button",
        summary: "Interactive button with click action handler",
        snippet: "Button(\"${1:Title}\") {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { Button(\"Go\") { n = 1 } } }",
    },
    ComponentEntry {
        name: "TextInput",
        summary: "Text input control bound to mutable state",
        snippet: "TextInput(value: ${1:binding}, placeholder: \"${2:Enter text}\")",
        probe: "app P { state name: String = \"\"\n body { TextInput(value: name, placeholder: \"Name\") } }",
    },
    ComponentEntry {
        name: "Switch",
        summary: "Boolean toggle with a label",
        snippet: "Switch(value: ${1:flag}, label: \"${2:Label}\")",
        probe: "app P { state flag: Bool = false\n body { Switch(value: flag, label: \"On\") } }",
    },
    ComponentEntry {
        name: "Image",
        summary: "Displays an asset or network image",
        snippet: "Image(asset: \"${1:icon}\", description: \"${2:}\")",
        probe: "app P { body { Image(asset: \"logo\", description: \"Logo\") } }",
    },
    ComponentEntry {
        name: "Pressable",
        summary: "Pressable region with onPress and onLongPress actions",
        snippet: "Pressable {\n    $0\n}.onPress {\n}",
        probe: "app P { state n: Int32 = 0\n body { Pressable() { Text(\"x\") }.onPress { n = 1 } } }",
    },
    ComponentEntry {
        name: "NavigationStack",
        summary: "Navigation host rooted at a declared screen",
        snippet: "NavigationStack(root: ${1:Home})",
        probe: "app P { screen Home { body { Text(\"x\") } }\n body { NavigationStack(root: Home) } }",
    },
    ComponentEntry {
        name: "NavigationLink",
        summary: "Navigates to a declared screen destination",
        snippet: "NavigationLink(destination: ${1:Details}) {\n    $0\n}",
        probe: "app P { screen Details { body { Text(\"x\") } }\n body { NavigationLink(destination: Details) { Text(\"go\") } } }",
    },
    ComponentEntry {
        name: "NavigationBack",
        summary: "Pops the navigation stack with an optional label",
        snippet: "NavigationBack(label: \"${1:Back}\")",
        probe: "app P { body { NavigationBack(label: \"Back\") } }",
    },
    ComponentEntry {
        name: "Link",
        summary: "Opens a URL in the system browser",
        snippet: "Link(url: \"${1:https://example.com}\") {\n    $0\n}",
        probe: "app P { body { Link(url: \"https://example.com\") { Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "Accessibility",
        summary: "Accessibility label, hint, and role wrapper",
        snippet: "Accessibility(label: \"${1:}\") {\n    $0\n}",
        probe: "app P { body { Accessibility(label: \"x\") { Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "KeyboardAware",
        summary: "Adjusts layout for the software keyboard",
        snippet: "KeyboardAware {\n    $0\n}",
        probe: "app P { body { KeyboardAware { Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "BottomSheet",
        summary: "Modal bottom sheet bound to a boolean binding",
        snippet: "BottomSheet(isPresented: ${1:show}) {\n    $0\n}",
        probe: "app P { state show: Bool = false\n body { BottomSheet(isPresented: show) { Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "RefreshControl",
        summary: "Pull-to-refresh wrapper with an onRefresh action",
        snippet: "RefreshControl(isRefreshing: ${1:busy}) {\n    $0\n}.onRefresh {\n}",
        probe: "app P { state busy: Bool = false\n body { RefreshControl(isRefreshing: busy) { Text(\"x\") }.onRefresh { busy = false } } }",
    },
    ComponentEntry {
        name: "AppBottomBar",
        summary: "Bottom tab bar bound to a selected-tab binding",
        snippet: "AppBottomBar(selected: ${1:tab}) {\n    Tab(index: 0, label: \"${2:Home}\") {\n        $0\n    }\n}",
        probe: "app P { state tab: Int32 = 0\n body { AppBottomBar(selected: tab) { Tab(index: 0, label: \"Home\") { Text(\"x\") } } } }",
    },
    ComponentEntry {
        name: "FastList",
        summary: "High-performance virtualized list view",
        snippet: "FastList(${1:items}) { ${2:item}, ${3:index} in\n    $0\n}",
        probe: "app P { body { FastList(count: 3) { index in Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "StatusBar",
        summary: "Status bar style, visibility, and background",
        snippet: "StatusBar(hidden: ${1:false})",
        probe: "app P { body { StatusBar(hidden: true) } }",
    },
    ComponentEntry {
        name: "Direction",
        summary: "Layout direction override (LTR or RTL)",
        snippet: "Direction(value: ${1:LTR})",
        probe: "app P { body { Direction(value: LTR) } }",
    },
    ComponentEntry {
        name: "OnAppear",
        summary: "Lifecycle trigger executed when the view appears",
        snippet: "OnAppear {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { OnAppear { n = 1 } } }",
    },
    ComponentEntry {
        name: "OnDisappear",
        summary: "Lifecycle trigger executed when the view disappears",
        snippet: "OnDisappear {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { OnDisappear { n = 1 } } }",
    },
    ComponentEntry {
        name: "OnActive",
        summary: "Lifecycle trigger executed when the app becomes active",
        snippet: "OnActive {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { OnActive { n = 1 } } }",
    },
    ComponentEntry {
        name: "OnInactive",
        summary: "Lifecycle trigger executed when the app becomes inactive",
        snippet: "OnInactive {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { OnInactive { n = 1 } } }",
    },
    ComponentEntry {
        name: "OnBackground",
        summary: "Lifecycle trigger executed when the app enters the background",
        snippet: "OnBackground {\n    $0\n}",
        probe: "app P { state n: Int32 = 0\n body { OnBackground { n = 1 } } }",
    },
    ComponentEntry {
        name: "Content",
        summary: "Renders a custom component's content slot",
        snippet: "Content()",
        probe: "component C() { body { Content() } }\napp P { body { Text(\"x\") } }",
    },
];

pub const KEYWORDS: &[KeywordEntry] = &[
    KeywordEntry {
        name: "app",
        kind: KeywordKind::Declaration,
        summary: "Defines the application root",
    },
    KeywordEntry {
        name: "screen",
        kind: KeywordKind::Declaration,
        summary: "Defines an application screen route",
    },
    KeywordEntry {
        name: "component",
        kind: KeywordKind::Declaration,
        summary: "Declares a reusable UI component",
    },
    KeywordEntry {
        name: "state",
        kind: KeywordKind::Declaration,
        summary: "Declares mutable reactive state",
    },
    KeywordEntry {
        name: "let",
        kind: KeywordKind::Declaration,
        summary: "Declares an immutable local binding",
    },
    KeywordEntry {
        name: "fn",
        kind: KeywordKind::Declaration,
        summary: "Declares a function",
    },
    KeywordEntry {
        name: "async",
        kind: KeywordKind::Declaration,
        summary: "Marks a function as asynchronous",
    },
    KeywordEntry {
        name: "await",
        kind: KeywordKind::Control,
        summary: "Awaits an asynchronous operation",
    },
    KeywordEntry {
        name: "enum",
        kind: KeywordKind::Declaration,
        summary: "Declares an enumeration",
    },
    KeywordEntry {
        name: "struct",
        kind: KeywordKind::Declaration,
        summary: "Declares a data structure",
    },
    KeywordEntry {
        name: "plugin",
        kind: KeywordKind::Declaration,
        summary: "Imports a native or pure Nexa plugin",
    },
    KeywordEntry {
        name: "try",
        kind: KeywordKind::Control,
        summary: "Initiates an error recovery block",
    },
    KeywordEntry {
        name: "catch",
        kind: KeywordKind::Control,
        summary: "Catches typed or general errors",
    },
    KeywordEntry {
        name: "case",
        kind: KeywordKind::Control,
        summary: "Pattern matches an error or enum variant",
    },
    KeywordEntry {
        name: "return",
        kind: KeywordKind::Control,
        summary: "Returns from a function",
    },
    KeywordEntry {
        name: "if",
        kind: KeywordKind::Control,
        summary: "Conditional branching",
    },
    KeywordEntry {
        name: "else",
        kind: KeywordKind::Control,
        summary: "Alternative conditional branch",
    },
    KeywordEntry {
        name: "while",
        kind: KeywordKind::Control,
        summary: "Repeats an action while condition is true",
    },
    KeywordEntry {
        name: "for",
        kind: KeywordKind::Control,
        summary: "Iterates over a collection or range",
    },
    KeywordEntry {
        name: "in",
        kind: KeywordKind::Control,
        summary: "Binds loop variables to an iterable",
    },
    KeywordEntry {
        name: "break",
        kind: KeywordKind::Control,
        summary: "Exits the enclosing loop",
    },
    KeywordEntry {
        name: "continue",
        kind: KeywordKind::Control,
        summary: "Skips to the next loop iteration",
    },
];

/// Program exercising every keyword. Parsed by `catalog_tests`; keep in sync
/// with `KEYWORDS` above.
pub const KEYWORD_PROBE: &str = r#"
plugin "./plugins/video" as Video
component Row2(title: String) {
    body { Text(title) }
}
struct Point { x: Int32 }
app Kw {
    screen Home {
        body { Text("x") }
    }
    state n: Int32 = 0
    let limit: Int32 = 3
    enum Level { low, high }
    async fn load() -> Void {
        await Video.prepare()
    }
    fn compute(values: Array<Int32>) -> Int32 {
        let total: Int32 = 0
        for i in 0..<limit {
            if i == 0 {
                continue
            } else {
                total = total + i
            }
        }
        while total > 100 {
            break
        }
        try {
            total = compute(values)
        } catch {
            case Video.Playback.Failed { total = 0 }
            else { total = 1 }
        }
        return total
    }
    body {
        Text("x")
    }
}
"#;

pub const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "String",
        summary: "UTF-8 string",
    },
    TypeEntry {
        name: "Bool",
        summary: "Boolean value (true or false)",
    },
    TypeEntry {
        name: "Int8",
        summary: "8-bit signed integer",
    },
    TypeEntry {
        name: "Int16",
        summary: "16-bit signed integer",
    },
    TypeEntry {
        name: "Int32",
        summary: "32-bit signed integer",
    },
    TypeEntry {
        name: "Int64",
        summary: "64-bit signed integer",
    },
    TypeEntry {
        name: "UInt8",
        summary: "8-bit unsigned integer",
    },
    TypeEntry {
        name: "UInt16",
        summary: "16-bit unsigned integer",
    },
    TypeEntry {
        name: "UInt32",
        summary: "32-bit unsigned integer",
    },
    TypeEntry {
        name: "UInt64",
        summary: "64-bit unsigned integer",
    },
    TypeEntry {
        name: "Float32",
        summary: "32-bit floating point number",
    },
    TypeEntry {
        name: "Float64",
        summary: "64-bit floating point number",
    },
    TypeEntry {
        name: "Bytes",
        summary: "Binary byte buffer",
    },
    TypeEntry {
        name: "Result",
        summary: "Zero-overhead Result<T, E> error handling type",
    },
    TypeEntry {
        name: "Array",
        summary: "Ordered generic list Array<T>",
    },
    TypeEntry {
        name: "Map",
        summary: "Key-value dictionary Map<K, V>",
    },
    TypeEntry {
        name: "Set",
        summary: "Unique element set Set<T>",
    },
    TypeEntry {
        name: "Pair",
        summary: "Two-element tuple Pair<A, B>",
    },
    TypeEntry {
        name: "Triple",
        summary: "Three-element tuple Triple<A, B, C>",
    },
    TypeEntry {
        name: "Void",
        summary: "Empty return type for functions",
    },
    TypeEntry {
        name: "Ok",
        summary: "Constructs a successful Result value: Ok(value)",
    },
    TypeEntry {
        name: "Err",
        summary: "Constructs an error Result value: Err(error)",
    },
];

/// Program exercising every builtin type name in a type position plus the
/// `Ok`/`Err` constructors. Parsed by `catalog_tests`.
pub const TYPE_PROBE: &str = r#"
app T {
    state s: String = ""
    state b: Bool = false
    state i8: Int8 = 0
    state i16: Int16 = 0
    state i32: Int32 = 0
    state i64: Int64 = 0
    state u8: UInt8 = 0
    state u16: UInt16 = 0
    state u32: UInt32 = 0
    state u64: UInt64 = 0
    state f32: Float32 = 0
    state f64: Float64 = 0
    state bytes: Bytes = []
    state list: Array<Int32> = []
    state lookup: Map<String, Int32> = [:]
    state unique: Set<Int32> = []
    state pair: Pair<String, Int32> = Pair("", 0)
    state triple: Triple<String, Int32, Bool> = Triple("", 0, false)
    fn load() -> Result<Int32, String> {
        return Ok(1)
    }
    fn fail() -> Result<Int32, String> {
        return Err("nope")
    }
    fn noop() -> Void {
    }
    body {
        Text("x")
    }
}
"#;

pub const DOT_MODIFIERS: &[DotModifierEntry] = &[
    DotModifierEntry {
        name: "onPress",
        summary: "Pressable tap action (required)",
        snippet: "onPress {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onLongPress",
        summary: "Pressable long-press action",
        snippet: "onLongPress {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onRefresh",
        summary: "RefreshControl pull-to-refresh action (required)",
        snippet: "onRefresh {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onEndReached",
        summary: "FastList pagination callback",
        snippet: "onEndReached {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onScroll",
        summary: "FastList scroll-position callback",
        snippet: "onScroll {\n    $0\n}",
    },
    DotModifierEntry {
        name: "stickyHeader",
        summary: "FastList sticky header content",
        snippet: "stickyHeader {\n    $0\n}",
    },
    DotModifierEntry {
        name: "sectionHeader",
        summary: "FastList section header content",
        snippet: "sectionHeader {\n    $0\n}",
    },
];

/// Look up a component entry by its source spelling.
pub fn component(name: &str) -> Option<&'static ComponentEntry> {
    COMPONENTS.iter().find(|entry| entry.name == name)
}

/// Look up a keyword entry by its source spelling.
pub fn keyword(name: &str) -> Option<&'static KeywordEntry> {
    KEYWORDS.iter().find(|entry| entry.name == name)
}

/// Look up a builtin type entry by its source spelling.
pub fn builtin_type(name: &str) -> Option<&'static TypeEntry> {
    TYPES.iter().find(|entry| entry.name == name)
}

/// Look up a trailing dot-modifier entry by its source spelling.
pub fn dot_modifier(name: &str) -> Option<&'static DotModifierEntry> {
    DOT_MODIFIERS.iter().find(|entry| entry.name == name)
}

// ---------------------------------------------------------------------------
// Declarative component schemas.
//
// The tables above describe IDE vocabulary (names, summaries, snippets).
// The schemas below are the single source of truth for component *shape*:
// argument names and requirements, child-block models, trailing dot-modifier
// models, documentation keys, and semantic feature tags. The parser validates
// invocations against these schemas and produces a generic syntactic
// `ComponentInvocation` (see `ast.rs`); semantic lowering converts each
// invocation into the existing typed IR nodes, so generated code stays fully
// specialized with no runtime maps or reflection.
// ---------------------------------------------------------------------------

/// How a component accepts its parenthesized head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParensModel {
    /// No parenthesis group at all; a `(` is a syntax error (`OnAppear`).
    None,
    /// `(...)` may be omitted (`Column`, `KeyboardAware`).
    Optional,
    /// `Name(...)` is required (`Text`, `TextInput`, ...).
    Required,
    /// `Name()` with exactly empty parens (`Content`).
    Empty,
}

/// A leading positional expression before any named options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionalModel {
    /// No positional value (`Column(value: ...)` is rejected).
    None,
    /// Exactly one required positional expression (`Text("label", ...)`).
    Single,
    /// FastList source grammar: a positional collection, `count:`, or
    /// `sections:`, parsed by the dedicated source reader.
    ListSource,
}

/// A named option accepted inside the parenthesis group.
pub struct ArgSchema {
    /// Option name exactly as written in `.nx` source.
    pub name: &'static str,
    /// Whether omitting the option is a compile error.
    pub required: bool,
    /// Exact diagnostics when a required option is missing. Defaults to
    /// `"{Component} requires `{name}`"` when `None`.
    pub required_message: Option<&'static str>,
}

/// A group of options of which exactly one must be present.
pub struct ExclusiveGroup {
    /// Competing option names.
    pub options: &'static [&'static str],
    /// Diagnostics when none is present.
    pub missing_message: &'static str,
    /// Diagnostics when more than one is present.
    pub both_message: &'static str,
}

/// The child block a component accepts after its head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildModel {
    /// No trailing block (`Text`, `Switch`, ...).
    None,
    /// A required `{ ... }` node block (`Column`, `Link`, ...).
    Nodes,
    /// An optional trailing action block (`Button`, `TextInput`).
    OptionalActions,
    /// A required trailing action block (`OnAppear` family).
    RequiredActions,
    /// `AppBottomBar` tab declarations.
    Tabs,
    /// FastList row bindings plus row body.
    ListRows,
}

/// The payload a trailing dot-modifier accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierBody {
    /// `.onPress { ... }` action statements.
    Actions,
    /// `.stickyHeader { ... }` child nodes.
    Nodes,
}

/// A trailing `.name { ... }` modifier accepted after a component block.
pub struct ModifierSchema {
    /// Modifier name exactly as written in `.nx` source.
    pub name: &'static str,
    /// Whether the modifier body holds actions or nodes.
    pub body: ModifierBody,
    /// Whether omitting the modifier is a compile error.
    pub required: bool,
}

/// Semantic feature family used to group components in the syntax audit and
/// to keep feature documentation aligned with the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureTag {
    Layout,
    Typography,
    Controls,
    Interactivity,
    Navigation,
    Lifecycle,
    Lists,
    Tabs,
    Refresh,
    Theming,
    Accessibility,
    Media,
    Overlays,
    Composition,
}

/// Declarative shape of one built-in component.
pub struct ComponentSchema {
    /// Component name exactly as written in `.nx` source.
    pub name: &'static str,
    /// Accepted spelling aliases. Empty today: the grammar is closed and
    /// platform-familiar spellings are rejected, but the model reserves
    /// the slot so an alias never needs a second representation.
    pub aliases: &'static [&'static str],
    /// Manual documentation anchor, e.g. `"components.md#column"`.
    pub doc: &'static str,
    /// Semantic feature families this component belongs to.
    pub features: &'static [FeatureTag],
    /// Parenthesized-head model.
    pub parens: ParensModel,
    /// Leading positional value model.
    pub positional: PositionalModel,
    /// Named options accepted inside the parenthesis group.
    pub arguments: &'static [ArgSchema],
    /// Exactly-one-of option groups.
    pub exclusive: &'static [ExclusiveGroup],
    /// Trailing child-block model.
    pub children: ChildModel,
    /// Trailing dot-modifiers.
    pub modifiers: &'static [ModifierSchema],
    /// Leading keywords accepted before the head (`OnAppear async`).
    pub flags: &'static [&'static str],
    /// Diagnostics when a trailing `{ ... }` or bare modifier word follows
    /// the component instead of dot syntax. `None` disables the check.
    pub trailing_message: Option<&'static str>,
}

/// An optional named option: `name: ...`.
const fn opt(name: &'static str) -> ArgSchema {
    ArgSchema {
        name,
        required: false,
        required_message: None,
    }
}

/// A required named option with the default
/// `"{Component} requires `{name}`"` diagnostic.
const fn req(name: &'static str) -> ArgSchema {
    ArgSchema {
        name,
        required: true,
        required_message: None,
    }
}

/// A required named option with a custom missing-option diagnostic.
const fn req_msg(name: &'static str, message: &'static str) -> ArgSchema {
    ArgSchema {
        name,
        required: true,
        required_message: Some(message),
    }
}

/// Layout style options shared by `Column`, `Row`, and `Stack`.
const LAYOUT_ARGUMENTS: &[ArgSchema] = &[
    opt("spacing"),
    opt("alignment"),
    opt("padding"),
    opt("width"),
    opt("height"),
    opt("minWidth"),
    opt("maxWidth"),
    opt("minHeight"),
    opt("maxHeight"),
    opt("background"),
    opt("cornerRadius"),
    opt("borderColor"),
    opt("borderWidth"),
    opt("opacity"),
    opt("animation"),
];

/// FastList source keys. The dedicated source reader (`list_source_and_args`
/// in `parser.rs`) consumes these words; they are catalogued here so the
/// argument audit and editor vocabulary cannot drift from the grammar.
pub const FASTLIST_SOURCE_KEYS: &[&str] = &["count", "sections"];
/// FastList row-key option, consumed by the same dedicated source reader.
pub const FASTLIST_KEY_OPTION: &str = "key";

/// Declarative shape of every built-in component, in `COMPONENTS` order.
/// The parser validates invocations against these schemas; semantic lowering
/// converts each validated invocation into the existing typed IR nodes.
pub const COMPONENT_SCHEMAS: &[ComponentSchema] = &[
    ComponentSchema {
        name: "Column",
        aliases: &[],
        doc: "components.md#column",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: LAYOUT_ARGUMENTS,
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Row",
        aliases: &[],
        doc: "components.md#row",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: LAYOUT_ARGUMENTS,
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Stack",
        aliases: &[],
        doc: "components.md#stack",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: LAYOUT_ARGUMENTS,
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Text",
        aliases: &[],
        doc: "components.md#text",
        features: &[FeatureTag::Typography],
        parens: ParensModel::Required,
        positional: PositionalModel::Single,
        arguments: &[
            opt("color"),
            opt("fontSize"),
            opt("fontWeight"),
            opt("lineLimit"),
            opt("lineHeight"),
            opt("letterSpacing"),
            opt("selectable"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Button",
        aliases: &[],
        doc: "components.md#button",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::Single,
        arguments: &[opt("icon"), opt("loading"), opt("disabled")],
        exclusive: &[],
        children: ChildModel::OptionalActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "TextInput",
        aliases: &[],
        doc: "components.md#textinput",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("value"),
            req("placeholder"),
            opt("keyboard"),
            opt("secure"),
            opt("multiline"),
            opt("autocorrect"),
            opt("capitalization"),
            opt("focused"),
            opt("maxLength"),
        ],
        exclusive: &[],
        children: ChildModel::OptionalActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Switch",
        aliases: &[],
        doc: "components.md#switch",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("value"), req("label")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Image",
        aliases: &[],
        doc: "components.md#image",
        features: &[FeatureTag::Media],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            opt("asset"),
            opt("url"),
            req("description"),
            opt("scale"),
            opt("placeholder"),
        ],
        exclusive: &[ExclusiveGroup {
            options: &["asset", "url"],
            missing_message: "Image requires exactly one of `asset` or `url`",
            both_message: "Image accepts either `asset` or `url`, not both",
        }],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Pressable",
        aliases: &[],
        doc: "components.md#pressable",
        features: &[FeatureTag::Interactivity],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("disabled"), opt("haptic")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[
            ModifierSchema {
                name: "onPress",
                body: ModifierBody::Actions,
                required: true,
            },
            ModifierSchema {
                name: "onLongPress",
                body: ModifierBody::Actions,
                required: false,
            },
        ],
        flags: &[],
        trailing_message: Some(
            "Pressable actions must use `.onPress { ... }` or `.onLongPress { ... }`",
        ),
    },
    ComponentSchema {
        name: "NavigationStack",
        aliases: &[],
        doc: "components.md#navigationstack--navigationlink",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req_msg(
            "root",
            "NavigationStack requires a declared screen as `root`",
        )],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "NavigationLink",
        aliases: &[],
        doc: "components.md#navigationstack--navigationlink",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req_msg(
                "destination",
                "NavigationLink requires a declared screen as `destination`",
            ),
            opt("when"),
        ],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "NavigationBack",
        aliases: &[],
        doc: "components.md#navigationstack--navigationlink",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("label")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Link",
        aliases: &[],
        doc: "components.md#link",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("url")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Accessibility",
        aliases: &[],
        doc: "components.md#accessibility",
        features: &[FeatureTag::Accessibility],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("label"), opt("hint"), opt("role")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "KeyboardAware",
        aliases: &[],
        doc: "components.md#keyboardaware",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: &[opt("dismiss")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "BottomSheet",
        aliases: &[],
        doc: "components.md#bottomsheet",
        features: &[FeatureTag::Overlays],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("isPresented"), opt("partial")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "RefreshControl",
        aliases: &[],
        doc: "components.md#refreshcontrol",
        features: &[FeatureTag::Refresh],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("isRefreshing")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[ModifierSchema {
            name: "onRefresh",
            body: ModifierBody::Actions,
            required: true,
        }],
        flags: &[],
        trailing_message: Some("RefreshControl actions must use `.onRefresh { ... }`"),
    },
    ComponentSchema {
        name: "AppBottomBar",
        aliases: &[],
        doc: "components.md#appbottombar",
        features: &[FeatureTag::Tabs],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("selected")],
        exclusive: &[],
        children: ChildModel::Tabs,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "FastList",
        aliases: &[],
        doc: "components.md#fastlist",
        features: &[FeatureTag::Lists],
        parens: ParensModel::Required,
        positional: PositionalModel::ListSource,
        arguments: &[opt("axis"), opt("rowHeight"), opt("scrollPosition")],
        exclusive: &[],
        children: ChildModel::ListRows,
        modifiers: &[
            ModifierSchema {
                name: "onEndReached",
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "onScroll",
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "stickyHeader",
                body: ModifierBody::Nodes,
                required: false,
            },
            ModifierSchema {
                name: "sectionHeader",
                body: ModifierBody::Nodes,
                required: false,
            },
        ],
        flags: &[],
        trailing_message: Some(
            "FastList modifiers must use dot syntax, for example `.onEndReached { ... }`",
        ),
    },
    ComponentSchema {
        name: "StatusBar",
        aliases: &[],
        doc: "components.md#statusbar",
        features: &[FeatureTag::Theming],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("style"), opt("hidden"), opt("background")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Direction",
        aliases: &[],
        doc: "components.md#direction",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req_msg(
            "value",
            "Direction requires `value: LTR` or `value: RTL`",
        )],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "OnAppear",
        aliases: &[],
        doc: "state-and-navigation.md#4-lifecycle-hooks",
        features: &[FeatureTag::Lifecycle],
        parens: ParensModel::None,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::RequiredActions,
        modifiers: &[],
        flags: &["async"],
        trailing_message: None,
    },
    ComponentSchema {
        name: "OnDisappear",
        aliases: &[],
        doc: "state-and-navigation.md#4-lifecycle-hooks",
        features: &[FeatureTag::Lifecycle],
        parens: ParensModel::None,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::RequiredActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "OnActive",
        aliases: &[],
        doc: "state-and-navigation.md#4-lifecycle-hooks",
        features: &[FeatureTag::Lifecycle],
        parens: ParensModel::None,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::RequiredActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "OnInactive",
        aliases: &[],
        doc: "state-and-navigation.md#4-lifecycle-hooks",
        features: &[FeatureTag::Lifecycle],
        parens: ParensModel::None,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::RequiredActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "OnBackground",
        aliases: &[],
        doc: "state-and-navigation.md#4-lifecycle-hooks",
        features: &[FeatureTag::Lifecycle],
        parens: ParensModel::None,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::RequiredActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Content",
        aliases: &[],
        doc: "components.md#custom-components--content",
        features: &[FeatureTag::Composition],
        parens: ParensModel::Empty,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
];

/// Look up a component schema by its source spelling (primary name or alias).
pub fn component_schema(name: &str) -> Option<&'static ComponentSchema> {
    COMPONENT_SCHEMAS
        .iter()
        .find(|schema| schema.name == name || schema.aliases.contains(&name))
}

/// Whether `name` is a built-in component spelling (primary name or alias).
pub fn is_component(name: &str) -> bool {
    component_schema(name).is_some()
}

/// Names a schema's parenthesis group accepts, for `named_args` validation.
pub fn schema_option_names(schema: &ComponentSchema) -> Vec<&'static str> {
    schema.arguments.iter().map(|arg| arg.name).collect()
}

/// Missing-option diagnostic for a required argument.
pub fn required_message(schema: &ComponentSchema, arg: &ArgSchema) -> String {
    match arg.required_message {
        Some(message) => message.to_owned(),
        None => format!("{} requires `{}`", schema.name, arg.name),
    }
}

/// Format a backticked modifier list: one, two (`` `a` or `b` ``), or Oxford
/// (`` `a`, `b`, or `c` ``). Reproduces the historical per-component wording.
fn modifier_alternation(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`.{name}`")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => {
            let (last, rest) = quoted.split_last().expect("non-empty modifier list");
            format!("{}, or {last}", rest.join(", "))
        }
    }
}

/// `unknown {Component} modifier `.x`; expected ...` diagnostic.
pub fn unknown_modifier_message(schema: &ComponentSchema, modifier: &str) -> String {
    let expected = modifier_alternation(
        &schema
            .modifiers
            .iter()
            .map(|modifier| modifier.name)
            .collect::<Vec<_>>(),
    );
    format!(
        "unknown {} modifier `.{modifier}`; expected {expected}",
        schema.name
    )
}

/// `{Component} accepts only one `.x` modifier` diagnostic.
pub fn duplicate_modifier_message(schema: &ComponentSchema, modifier: &str) -> String {
    format!("{} accepts only one `.{modifier}` modifier", schema.name)
}

/// `{Component} requires an `.x { ... }` action block` diagnostic.
pub fn required_modifier_message(schema: &ComponentSchema, modifier: &str) -> String {
    format!(
        "{} requires an `.{modifier} {{ ... }}` action block",
        schema.name
    )
}

// ---------------------------------------------------------------------------
// Generated editor vocabulary and documentation audit.
//
// The TextMate grammar (`editors/vscode/syntaxes/nexa.tmLanguage.json`) and
// the syntax audit (`docs/syntax-audit.md`) are snapshots of this catalog:
// `catalog_tests` asserts the checked-in files match these generators byte
// for byte (run with `NEXA_UPDATE_SNAPSHOTS=1` to rewrite them), so grammar
// highlighting and the audit cannot drift from the accepted language.
// ---------------------------------------------------------------------------

/// Child-syntax names that are not component invocations but still deserve
/// component highlighting (`AppBottomBar` tab entries).
pub const GRAMMAR_EXTRA_COMPONENTS: &[&str] = &["Tab"];

/// Sorted control-keyword alternation for the grammar `keyword.control` rule.
pub fn grammar_control_keywords() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = KEYWORDS
        .iter()
        .filter(|entry| entry.kind == KeywordKind::Control)
        .map(|entry| entry.name)
        .collect();
    names.sort_unstable();
    names
}

/// Sorted declaration-keyword alternation for the grammar rule.
pub fn grammar_declaration_keywords() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = KEYWORDS
        .iter()
        .filter(|entry| entry.kind == KeywordKind::Declaration)
        .map(|entry| entry.name)
        .collect();
    names.sort_unstable();
    names
}

/// Sorted builtin-type alternation for the grammar `builtin-types` rule.
pub fn grammar_type_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = TYPES.iter().map(|entry| entry.name).collect();
    names.sort_unstable();
    names
}

/// Sorted component alternation for the grammar `ui-components` rule,
/// including child-syntax extras like `Tab`.
pub fn grammar_component_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = COMPONENTS.iter().map(|entry| entry.name).collect();
    names.extend_from_slice(GRAMMAR_EXTRA_COMPONENTS);
    names.sort_unstable();
    names
}

/// Render one grammar `match` alternation: `\b(a|b|c)\b`.
pub fn grammar_alternation(names: &[&str]) -> String {
    format!("\\b({})\\b", names.join("|"))
}

/// Feature tag display name used as an audit section heading.
fn feature_heading(tag: FeatureTag) -> &'static str {
    match tag {
        FeatureTag::Layout => "Layout",
        FeatureTag::Typography => "Typography",
        FeatureTag::Controls => "Controls",
        FeatureTag::Interactivity => "Interactivity",
        FeatureTag::Navigation => "Navigation",
        FeatureTag::Lifecycle => "Lifecycle",
        FeatureTag::Lists => "Lists",
        FeatureTag::Tabs => "Tabs",
        FeatureTag::Refresh => "Refresh",
        FeatureTag::Theming => "Theming",
        FeatureTag::Accessibility => "Accessibility",
        FeatureTag::Media => "Media",
        FeatureTag::Overlays => "Overlays",
        FeatureTag::Composition => "Composition",
    }
}

/// Feature tag anchor slug used for audit section links.
fn feature_anchor(tag: FeatureTag) -> &'static str {
    match tag {
        FeatureTag::Layout => "layout",
        FeatureTag::Typography => "typography",
        FeatureTag::Controls => "controls",
        FeatureTag::Interactivity => "interactivity",
        FeatureTag::Navigation => "navigation",
        FeatureTag::Lifecycle => "lifecycle",
        FeatureTag::Lists => "lists",
        FeatureTag::Tabs => "tabs",
        FeatureTag::Refresh => "refresh",
        FeatureTag::Theming => "theming",
        FeatureTag::Accessibility => "accessibility",
        FeatureTag::Media => "media",
        FeatureTag::Overlays => "overlays",
        FeatureTag::Composition => "composition",
    }
}

/// All feature tags in audit order.
const FEATURE_ORDER: &[FeatureTag] = &[
    FeatureTag::Layout,
    FeatureTag::Typography,
    FeatureTag::Controls,
    FeatureTag::Interactivity,
    FeatureTag::Media,
    FeatureTag::Navigation,
    FeatureTag::Lifecycle,
    FeatureTag::Lists,
    FeatureTag::Tabs,
    FeatureTag::Refresh,
    FeatureTag::Theming,
    FeatureTag::Accessibility,
    FeatureTag::Overlays,
    FeatureTag::Composition,
];

/// Reconstruct the canonical invocation signature from a schema.
fn audit_signature(schema: &ComponentSchema) -> String {
    let mut head = String::new();
    match schema.positional {
        PositionalModel::None => {}
        PositionalModel::Single => head.push_str("value, "),
        PositionalModel::ListSource => head.push_str("collection | count: | sections:, "),
    }
    let options: Vec<String> = schema
        .arguments
        .iter()
        .map(|arg| {
            if arg.required {
                arg.name.to_owned()
            } else {
                format!("{}:", arg.name)
            }
        })
        .collect();
    head.push_str(&options.join(", "));
    let parens_note = match schema.parens {
        ParensModel::None => String::new(),
        ParensModel::Optional => " (parentheses optional)".to_owned(),
        ParensModel::Required => String::new(),
        ParensModel::Empty => String::new(),
    };
    match schema.children {
        ChildModel::None => format!("`{}({head})`{parens_note}", schema.name),
        ChildModel::Nodes => format!("`{}({head}) {{ ... }}`{parens_note}", schema.name),
        ChildModel::OptionalActions => {
            format!(
                "`{}({head})`{parens_note} with optional trailing `{{ ... }}` actions",
                schema.name
            )
        }
        ChildModel::RequiredActions => {
            let flags = if schema.flags.is_empty() {
                String::new()
            } else {
                format!("{} ", schema.flags.join(" "))
            };
            format!("`{} {flags}{{ ... }}` actions{parens_note}", schema.name)
        }
        ChildModel::Tabs => format!("`{}(selected:) {{ Tab(..) ... }}`", schema.name),
        ChildModel::ListRows => format!(
            "`{}(collection | count: | sections:, axis:, ...)` with `{{ bindings in ... }}` rows",
            schema.name
        ),
    }
}

/// Render the complete syntax audit from the catalog. The output is the
/// checked-in `docs/syntax-audit.md`; `catalog_tests` enforces equality.
pub fn render_syntax_audit() -> String {
    let mut out = String::from(
        "# Syntax Audit\n\n\
        > Generated from `crates/nexa-syntax/src/catalog.rs` — do not edit by hand.\n\
        > Run `cargo test -p nexa-syntax` with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate.\n\
        > Every entry mirrors a parser production: the catalog test suite parses\n\
        > each component probe, so an audit entry without a working probe fails.\n\n\
        Each component lists its canonical signature, argument requirements,\n\
        child-block model, trailing modifiers, and manual documentation anchor.\n\
        Required options are bare names; optional options carry a trailing colon.\n\n",
    );
    out.push_str("Sections:\n\n");
    for tag in FEATURE_ORDER {
        if COMPONENT_SCHEMAS
            .iter()
            .any(|schema| schema.features.contains(tag))
        {
            out.push_str(&format!(
                "- [{}](#{})\n",
                feature_heading(*tag),
                feature_anchor(*tag)
            ));
        }
    }
    out.push('\n');
    for tag in FEATURE_ORDER {
        let mut entries: Vec<&ComponentSchema> = COMPONENT_SCHEMAS
            .iter()
            .filter(|schema| schema.features.contains(tag))
            .collect();
        if entries.is_empty() {
            continue;
        }
        entries.sort_by_key(|schema| schema.name);
        out.push_str(&format!("## {}\n\n", feature_heading(*tag)));
        for schema in entries {
            let entry = COMPONENTS
                .iter()
                .find(|entry| entry.name == schema.name)
                .expect("schema without a vocabulary entry");
            out.push_str(&format!("### `{}`\n\n", schema.name));
            out.push_str(&format!("{}\n\n", entry.summary));
            out.push_str(&format!("Signature: {}\n\n", audit_signature(schema)));
            if !schema.aliases.is_empty() {
                out.push_str(&format!("Aliases: {}\n\n", schema.aliases.join(", ")));
            }
            let (required, optional): (Vec<_>, Vec<_>) =
                schema.arguments.iter().partition(|arg| arg.required);
            if !required.is_empty() {
                out.push_str(&format!(
                    "Required options: {}\n\n",
                    required
                        .iter()
                        .map(|arg| format!("`{}`", arg.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !optional.is_empty() {
                out.push_str(&format!(
                    "Optional options: {}\n\n",
                    optional
                        .iter()
                        .map(|arg| format!("`{}`", arg.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            for group in schema.exclusive {
                out.push_str(&format!(
                    "Exactly one of: {}\n\n",
                    group
                        .options
                        .iter()
                        .map(|option| format!("`{option}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if schema.name == "FastList" {
                out.push_str(&format!(
                    "Source forms: positional collection, `{}`:, or `sections:` \
                    (exactly one); row-key option `{}`.\n\n",
                    FASTLIST_SOURCE_KEYS[0], FASTLIST_KEY_OPTION
                ));
            }
            out.push_str(&format!(
                "Children: {}\n\n",
                match schema.children {
                    ChildModel::None => "none",
                    ChildModel::Nodes => "node block",
                    ChildModel::OptionalActions => "optional action block",
                    ChildModel::RequiredActions => "action block",
                    ChildModel::Tabs => "`Tab` declarations",
                    ChildModel::ListRows => "row bindings plus row body",
                }
            ));
            if !schema.flags.is_empty() {
                out.push_str(&format!(
                    "Leading flags: {}\n\n",
                    schema
                        .flags
                        .iter()
                        .map(|flag| format!("`{flag}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if schema.modifiers.is_empty() {
                out.push_str("Modifiers: none\n\n");
            } else {
                out.push_str("Modifiers:\n\n");
                for modifier in schema.modifiers {
                    out.push_str(&format!(
                        "- `.{}` ({}, {})\n",
                        modifier.name,
                        match modifier.body {
                            ModifierBody::Actions => "actions",
                            ModifierBody::Nodes => "nodes",
                        },
                        if modifier.required {
                            "required"
                        } else {
                            "optional"
                        },
                    ));
                }
                out.push('\n');
            }
            out.push_str(&format!("Reference: {}\n\n", schema.doc));
        }
    }
    out.push_str(&format!(
        "## Vocabulary\n\n\
        Keywords: {}\n\n\
        Types: {}\n\n",
        KEYWORDS
            .iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>()
            .join(", "),
        TYPES
            .iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>()
            .join(", "),
    ));
    out
}
