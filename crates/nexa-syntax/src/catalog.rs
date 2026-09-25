//! Authoritative language catalog for the `.nx` surface vocabulary.
//!
//! The parser (`parser.rs`) is the source of truth for what Nexa accepts.
//! This catalog mirrors that vocabulary as data so IDE tooling (completions,
//! hover) cannot drift from the grammar — see the `catalog_tests` suite,
//! which parses every component probe, the keyword probe, and the type probe
//! and therefore fails if an entry stops being accepted by the parser.
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
    pub summary: &'static str,
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
    KeywordEntry { name: "app", summary: "Defines the application root" },
    KeywordEntry { name: "screen", summary: "Defines an application screen route" },
    KeywordEntry { name: "component", summary: "Declares a reusable UI component" },
    KeywordEntry { name: "state", summary: "Declares mutable reactive state" },
    KeywordEntry { name: "let", summary: "Declares an immutable local binding" },
    KeywordEntry { name: "fn", summary: "Declares a function" },
    KeywordEntry { name: "async", summary: "Marks a function as asynchronous" },
    KeywordEntry { name: "await", summary: "Awaits an asynchronous operation" },
    KeywordEntry { name: "enum", summary: "Declares an enumeration" },
    KeywordEntry { name: "struct", summary: "Declares a data structure" },
    KeywordEntry { name: "plugin", summary: "Imports a native or pure Nexa plugin" },
    KeywordEntry { name: "try", summary: "Initiates an error recovery block" },
    KeywordEntry { name: "catch", summary: "Catches typed or general errors" },
    KeywordEntry { name: "case", summary: "Pattern matches an error or enum variant" },
    KeywordEntry { name: "return", summary: "Returns from a function" },
    KeywordEntry { name: "if", summary: "Conditional branching" },
    KeywordEntry { name: "else", summary: "Alternative conditional branch" },
    KeywordEntry { name: "while", summary: "Repeats an action while condition is true" },
    KeywordEntry { name: "for", summary: "Iterates over a collection or range" },
    KeywordEntry { name: "in", summary: "Binds loop variables to an iterable" },
    KeywordEntry { name: "break", summary: "Exits the enclosing loop" },
    KeywordEntry { name: "continue", summary: "Skips to the next loop iteration" },
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
    TypeEntry { name: "String", summary: "UTF-8 string" },
    TypeEntry { name: "Bool", summary: "Boolean value (true or false)" },
    TypeEntry { name: "Int8", summary: "8-bit signed integer" },
    TypeEntry { name: "Int16", summary: "16-bit signed integer" },
    TypeEntry { name: "Int32", summary: "32-bit signed integer" },
    TypeEntry { name: "Int64", summary: "64-bit signed integer" },
    TypeEntry { name: "UInt8", summary: "8-bit unsigned integer" },
    TypeEntry { name: "UInt16", summary: "16-bit unsigned integer" },
    TypeEntry { name: "UInt32", summary: "32-bit unsigned integer" },
    TypeEntry { name: "UInt64", summary: "64-bit unsigned integer" },
    TypeEntry { name: "Float32", summary: "32-bit floating point number" },
    TypeEntry { name: "Float64", summary: "64-bit floating point number" },
    TypeEntry { name: "Bytes", summary: "Binary byte buffer" },
    TypeEntry { name: "Result", summary: "Zero-overhead Result<T, E> error handling type" },
    TypeEntry { name: "Array", summary: "Ordered generic list Array<T>" },
    TypeEntry { name: "Map", summary: "Key-value dictionary Map<K, V>" },
    TypeEntry { name: "Set", summary: "Unique element set Set<T>" },
    TypeEntry { name: "Pair", summary: "Two-element tuple Pair<A, B>" },
    TypeEntry { name: "Triple", summary: "Three-element tuple Triple<A, B, C>" },
    TypeEntry { name: "Void", summary: "Empty return type for functions" },
    TypeEntry { name: "Ok", summary: "Constructs a successful Result value: Ok(value)" },
    TypeEntry { name: "Err", summary: "Constructs an error Result value: Err(error)" },
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
