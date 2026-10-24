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
//! `TextInput`), and `FastSectionedList` (sectioned `FastList` uses the
//! `sections:` source) have no parser production.

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

/// A dot modifier accepted after a component use.
pub struct DotModifierEntry {
    pub name: &'static str,
    pub summary: &'static str,
    pub snippet: &'static str,
}

pub const COMPONENTS: &[ComponentEntry] = &[
    ComponentEntry {
        name: "Appearance",
        summary: "Selects system, light, or dark appearance for its native content",
        snippet: "Appearance(mode: \"${1:system}\") {\n    $0\n}",
        probe: "app P { state mode: String = \"system\"\n body { Appearance(mode: mode) { Text(\"Hello\") } } }",
    },
    ComponentEntry {
        name: "Column",
        summary: "Vertical layout container with optional spacing",
        snippet: "Column(spacing: ${1:8}) {\n    $0\n}",
        probe: "app P { body { Column(spacing: 8) { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Row",
        summary: "Horizontal layout container with spacing and main-axis arrangement",
        snippet: "Row(spacing: ${1:8}) {\n    $0\n}",
        probe: "app P { body { Row(spacing: 8, horizontalArrangement: Center) { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Stack",
        summary: "Overlapping layout container",
        snippet: "Stack {\n    $0\n}",
        probe: "app P { body { Stack { Text(\"a\") } } }",
    },
    ComponentEntry {
        name: "Form",
        summary: "Native settings form with optional background and configurable row dividers",
        snippet: "Form(showDividers: false) {\n    Section(title: \"${1:General}\") {\n        $0\n    }\n}",
        probe: "app P { body { Form(showDividers: false, background: \"#FFFFFF\") { Section(title: \"General\", background: \"#F5F5F5\") { Text(\"Setting\") } } } }",
    },
    ComponentEntry {
        name: "Section",
        summary: "A titled or footnoted group with optional background and divider override",
        snippet: "Section(title: \"${1:General}\", showDividers: false) {\n    $0\n}",
        probe: "app P { body { Form { Section(title: \"General\", footer: \"Details\", showDividers: false, background: \"#FFFFFF\") { Text(\"Setting\") } } } }",
    },
    ComponentEntry {
        name: "Spacer",
        summary: "Expands along the parent layout axis",
        snippet: "Spacer()",
        probe: "app P { body { Column { Text(\"a\") Spacer(accessibilityLabel: \"Spacer\") Text(\"b\") } } }",
    },
    ComponentEntry {
        name: "Divider",
        summary: "Separates content with a native divider",
        snippet: "Divider(color: \"#808080\", thickness: 1)",
        probe: "app P { body { Column { Text(\"a\") Divider(color: \"#808080\", thickness: 1) Text(\"b\") } } }",
    },
    ComponentEntry {
        name: "Text",
        summary: "Displays formatted text",
        snippet: "Text(\"${1:Label}\")",
        probe: "app P { body { Text(\"a\", fontStyle: Subheadline, fontDesign: Serif, accessibilityLabel: \"a label\", accessibilityHint: \"more detail\", accessibilityValue: \"Ready\", accessibilityRole: Image) } }",
    },
    ComponentEntry {
        name: "ContentUnavailable",
        summary: "Shows a native empty or unavailable content state",
        snippet: "ContentUnavailable(title: \"${1:Nothing here}\", icon: \"${2:inbox}\")",
        probe: "app P { body { ContentUnavailable(title: \"Nothing here\", icon: \"inbox\", titleColor: \"#222222\", descriptionColor: \"#666666\", iconTint: \"#0088FF\", background: \"#F4F4F8\") } }",
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
        probe: "app P { state name: String = \"\"\n body { TextInput(value: name, placeholder: \"Name\", searchable: true) } }",
    },
    ComponentEntry {
        name: "Switch",
        summary: "Boolean toggle with a label",
        snippet: "Switch(value: ${1:flag}, label: \"${2:Label}\")",
        probe: "app P { state flag: Bool = false\n body { Switch(value: flag, label: \"On\") } }",
    },
    ComponentEntry {
        name: "Slider",
        summary: "Adjusts a Float64 state value within a stepped range",
        snippet: "Slider(value: ${1:amount}, min: 0.0, max: 1.0, step: 0.1)",
        probe: "app P { state amount: Float64 = 0.5\n body { Slider(value: amount, min: 0.0, max: 1.0, step: 0.1) } }",
    },
    ComponentEntry {
        name: "ProgressBar",
        summary: "Shows customizable determinate or indeterminate linear progress",
        snippet: "ProgressBar()",
        probe: "app P { state amount: Float64 = 0.5\n body { ProgressBar()\n ProgressBar(progress: amount, tint: \"#2563EB\", trackColor: \"#DBEAFE\", thickness: 8, cornerRadius: 4, android_wave: true) } }",
    },
    ComponentEntry {
        name: "ProgressRing",
        summary: "Shows customizable determinate or indeterminate circular progress",
        snippet: "ProgressRing()",
        probe: "app P { state amount: Float64 = 0.5\n state tint: String = \"#2563EB\"\n body { ProgressRing()\n ProgressRing(progress: amount, tint: tint, trackColor: \"#DBEAFE\", size: 48, thickness: 5, android_wave: true) } }",
    },
    ComponentEntry {
        name: "SegmentedControl",
        summary: "Selects one string option with customizable color and shape",
        snippet: "SegmentedControl(items: ${1:filters}, selected: ${2:selectedFilter}, tint: \"${3:#FFFFFF}\", background: \"${4:#F2F2F7}\", selectedTextColor: \"${5:#000000}\", textColor: \"${6:#555555}\", height: ${7:36}, cornerRadius: ${8:18})",
        probe: "app P { state filters: Array<String> = [\"All\", \"Open\"]\n state selected: String = \"All\"\n body { SegmentedControl(items: filters, selected: selected, tint: \"#FFFFFF\", background: \"#F2F2F7\", selectedTextColor: \"#000000\", textColor: \"#555555\", fontSize: 14, fontWeight: Medium, height: 36, cornerRadius: 18, borderColor: \"#D1D1D6\", borderWidth: 1) } }",
    },
    ComponentEntry {
        name: "Picker",
        summary: "Selects one string option from a native menu",
        snippet: "Picker(items: ${1:options}, selected: ${2:selectedOption}, itemIcons: [\"calendar\", \"flag_filled\"], icon: \"${3:sort}\", tint: \"${4:#6750A4}\")",
        probe: "app P { state options: Array<String> = [\"Small\", \"Medium\"]\n state selected: String = \"Medium\"\n body { Picker(items: options, selected: selected, itemIcons: [\"calendar\", \"flag_filled\"]) } }",
    },
    ComponentEntry {
        name: "PickerDate",
        summary: "Selects a date, time, date-time, or date range",
        snippet: "PickerDate(type: ${1:Date}, style: ${2:Graphical}, selection: ${3:Single}, timestamp: ${4:selectedDate}, tint: \"${5:#6750A4}\")",
        probe: "app P { state selected: Int64 = 1792153800000\n state start: Int64 = 1791763200000\n state end: Int64 = 1792454400000\n body { PickerDate(type: Date, timestamp: selected, style: Graphical, selection: Single, minimum: start, maximum: end, tint: \"#6750A4\", textColor: \"#111827\", backgroundColor: \"#FFFFFF\", pickerBackgroundColor: \"#F8FAFC\", borderColor: \"#D1D5DB\", borderWidth: 1, cornerRadius: 16, locale: \"en-US\", timeZone: \"UTC\", is24Hour: true, enabled: true, accessibilityLabel: \"Due date\") PickerDate(type: Date, style: Input, timestamp: selected) } }",
    },
    ComponentEntry {
        name: "Image",
        summary: "Displays a native image from an asset, URL, or local file",
        snippet: "Image(url: \"${1:https://example.com/image.png}\", description: \"${2:Image description}\", scale: Fill, fillMaxSize: true)",
        probe: "app P { body { Image(url: \"https://example.test/logo.png\", description: \"Logo\", showLoadingIndicator: true, fillMaxSize: true) } }",
    },
    ComponentEntry {
        name: "Icon",
        summary: "Displays one portable or platform-specific native system icon",
        snippet: "Icon(system: \"${1:favorite}\", description: \"${2:Like}\", size: ${3:28}, tint: \"#FFFFFF\")",
        probe: "app P { body { Icon(system: \"favorite\", description: \"Like\", size: 28, tint: \"#FFFFFF\") } }",
    },
    ComponentEntry {
        name: "LinearGradient",
        summary: "Draws a native linear color gradient",
        snippet: "LinearGradient(startColor: \"#CC000000\", endColor: \"#00000000\", direction: TopToBottom, height: 180)",
        probe: "app P { body { Stack { LinearGradient(startColor: \"#CC000000\", endColor: \"#00000000\", direction: TopToBottom, height: 180) } } }",
    },
    ComponentEntry {
        name: "Pressable",
        summary: "Pressable region with tap, context-menu, drag, pinch, double-tap, and long-press actions",
        snippet: "Pressable {\n    $0\n}.onTap {\n}",
        probe: "app P { state n: Int32 = 0\n body { Pressable() { Text(\"x\") }.onTap { n = 1 }.onLongPress(durationMs: 650) { n = 0 }.contextMenu { Button(\"Edit\") { n = 2 } }.onDoubleTap { n = 2 }.onDrag { x, y, vx, vy -> n = n + 1 }.onPinch { scaleFactor -> n = n + 1 } } }",
    },
    ComponentEntry {
        name: "NavigationStack",
        summary: "Navigation host rooted at a declared screen",
        snippet: "NavigationStack(root: ${1:Home}, titleMode: ${2|Inline,Large|}, titleAlignment: ${3|Automatic,Leading,Center|})",
        probe: "app P { screen Home { body { Text(\"x\") } }\n body { NavigationStack(root: Home, titleMode: Inline, titleAlignment: Automatic) } }",
    },
    ComponentEntry {
        name: "NavigationSplitView",
        summary: "Adaptive sidebar and detail navigation",
        snippet: "NavigationSplitView(detailVisible: ${1:showDetail}) {\n    Sidebar {\n        $0\n    }\n    Detail {\n        Text(\"Select an item\")\n    }\n}",
        probe: "app P { state showDetail: Bool = false\n body { NavigationSplitView(detailVisible: showDetail) { Sidebar { Text(\"Items\") } Detail { Text(\"Detail\") } } } }",
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
        name: "KeyboardAware",
        summary: "Adjusts layout for the software keyboard",
        snippet: "KeyboardAware {\n    $0\n}",
        probe: "app P { body { KeyboardAware { Text(\"x\") } } }",
    },
    ComponentEntry {
        name: "BottomSheet",
        summary: "Native modal sheet with custom snap points and dismissal behavior",
        snippet: "BottomSheet(isPresented: ${1:show}, snapPoints: [0.5, 1.0]) {\n    $0\n}.onDismiss {\n}",
        probe: "app P { state show: Bool = false\n body { BottomSheet(isPresented: show, contentPadding: 20, snapPoints: [0.5, 1.0]) { Text(\"x\") }.onDismiss { show = false } } }",
    },
    ComponentEntry {
        name: "Dialog",
        summary: "Native alert with button actions and optional single-line text input",
        snippet: "Dialog(isPresented: ${1:show}, title: \"${2:Title}\", message: \"${3:Message}\") {\n    TextInput(value: ${4:text}, placeholder: \"${5:Enter text}\")\n    Button(\"${6:Save}\") { show = false }\n    Button(\"${7:Cancel}\") { show = false }\n}",
        probe: "app P { state show: Bool = false\n state value: String = \"\"\n body { Dialog(isPresented: show, title: \"Title\", message: \"Message\") { TextInput(value: value, placeholder: \"Value\") Button(\"OK\") { show = false } } } }",
    },
    ComponentEntry {
        name: "ConfirmationDialog",
        summary: "Native confirmation dialog bound to a boolean binding",
        snippet: "ConfirmationDialog(isPresented: ${1:show}, title: \"${2:Choose an option}\") {\n    Button(\"${3:Continue}\") { show = false }\n    Button(\"${4:Cancel}\") { show = false }\n}",
        probe: "app P { state show: Bool = false\n body { ConfirmationDialog(isPresented: show, title: \"Choose\") { Button(\"OK\") { show = false } } } }",
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
        snippet: "AppBottomBar(selected: ${1:selectedTab}) {\n    Tab(index: 0, label: \"${2:Home}\", icon: \"home\") {\n        $0\n    }\n    TabBar {\n        Row(alignment: Center) {\n            Pressable(accessibilityLabel: \"Home tab\", accessibilityRole: Button) {\n                Column(spacing: 4, alignment: Center) {\n                    Icon(system: \"home\", description: \"Home\", size: 20)\n                    Text(\"Home\")\n                }\n            }.onTap { selectedTab = 0 }\n        }\n    }\n}",
        probe: "app P { state tab: Int32 = 0\n body { AppBottomBar(selected: tab) { Tab(index: 0, label: \"Home\", icon: \"home\") { Text(\"x\") } TabBar { Text(\"custom\") } } } }",
    },
    ComponentEntry {
        name: "PagePager",
        summary: "Swipeable pages with a native page indicator, bound to a selected-page value",
        snippet: "PagePager(selected: ${1:page}) {\n    Tab(index: 0) {\n        $0\n    }\n}",
        probe: "app P { state page: Int32 = 0\n body { PagePager(selected: page) { Tab(index: 0) { Text(\"x\") } } } }",
    },
    ComponentEntry {
        name: "Toolbar",
        summary: "Places native actions in a navigation toolbar",
        snippet: "Toolbar(placement: Trailing) {\n    $0\n}",
        probe: "app P { body { Toolbar(placement: Trailing) { Button(\"Add\") { } } } }",
    },
    ComponentEntry {
        name: "FastList",
        summary: "High-performance virtualized list with configurable dividers and responsive grids",
        snippet: "FastList(${1:items}) { ${2:item}, ${3:index} in\n    $0\n}",
        probe: "app P { state enabled: Bool = true\n body { FastList(count: 3, axis: AutoGrid(168), scrollEnabled: enabled, showDividers: false) { index in Text(\"x\") } } }",
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
        name: "Regex",
        summary: "Reusable native regular expression",
    },
    TypeEntry {
        name: "RegexMatch",
        summary: "Typed regular-expression match, range, and capture groups",
    },
    TypeEntry {
        name: "Range",
        summary: "Half-open UTF-16 source range returned by RegexMatch",
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
    state expression: Regex = /order-[0-9]+/i
    state match: RegexMatch? = null
    state range: Range? = null
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
        name: "sharedElement",
        summary: "Match an image element across screen transitions",
        snippet: "sharedElement(id: \"${1:hero}\")",
    },
    DotModifierEntry {
        name: "onTap",
        summary: "Pressable tap action",
        snippet: "onTap {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onChange",
        summary: "Text input change action",
        snippet: "onChange { text ->\n    $0\n}",
    },
    DotModifierEntry {
        name: "onLongPress",
        summary: "Pressable long-press action with an optional duration",
        snippet: "onLongPress(durationMs: ${1:500}) {\n    $0\n}",
    },
    DotModifierEntry {
        name: "contextMenu",
        summary: "Native context menu actions for a pressable view",
        snippet: "contextMenu {\n    Button(\"Edit\") {\n        $0\n    }\n}",
    },
    DotModifierEntry {
        name: "onDoubleTap",
        summary: "Pressable double-tap action",
        snippet: "onDoubleTap {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onDrag",
        summary: "Pressable drag callback with translation and velocity values",
        snippet: "onDrag { translationX, translationY, velocityX, velocityY ->\n    $0\n}",
    },
    DotModifierEntry {
        name: "onPinch",
        summary: "Pressable pinch callback with a cumulative scale factor",
        snippet: "onPinch { scaleFactor ->\n    $0\n}",
    },
    DotModifierEntry {
        name: "onRefresh",
        summary: "RefreshControl pull-to-refresh action (required)",
        snippet: "onRefresh {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onDismiss",
        summary: "BottomSheet action after the sheet is dismissed",
        snippet: "onDismiss {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onEndReached",
        summary: "FastList pagination callback",
        snippet: "onEndReached {\n    $0\n}",
    },
    DotModifierEntry {
        name: "onMove",
        summary: "FastList array-item reorder callback with optional enabled state and source/destination indexes",
        snippet: "onMove(enabled: true) { from, to ->\n    $0\n}",
    },
    DotModifierEntry {
        name: "onScroll",
        summary: "FastList scroll-position callback",
        snippet: "onScroll {\n    $0\n}",
    },
    DotModifierEntry {
        name: "scrollToIndex",
        summary: "FastList mutable leading-index binding",
        snippet: "scrollToIndex(index: ${1:position})",
    },
    DotModifierEntry {
        name: "scrollToTop",
        summary: "FastList command triggered by a changed Int32 state",
        snippet: "scrollToTop(trigger: ${1:topCommand})",
    },
    DotModifierEntry {
        name: "scrollToEnd",
        summary: "FastList command triggered by a changed Int32 state",
        snippet: "scrollToEnd(trigger: ${1:endCommand})",
    },
    DotModifierEntry {
        name: "scrollToOffset",
        summary: "FastList command to scroll to a logical offset in points/dp",
        snippet: "scrollToOffset(offset: ${1:contentOffset})",
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
    DotModifierEntry {
        name: "swipeActions",
        summary: "Native trailing actions for list rows",
        snippet: "swipeActions {\n    Button(\"Delete\", icon: \"trash\") {\n        $0\n    }\n}",
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
    /// `NavigationSplitView` sidebar and detail blocks.
    SplitPanes,
    /// FastList row bindings plus row body.
    ListRows,
}

/// The payload a trailing dot-modifier accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierBody {
    /// A value-only modifier with no trailing block.
    None,
    /// `.onTap { ... }` action statements.
    Actions,
    /// Event parameter bindings followed by action statements.
    EventActions,
    /// `.stickyHeader { ... }` child nodes.
    Nodes,
}

/// A trailing `.name { ... }` modifier accepted after a component block.
pub struct ModifierSchema {
    /// Modifier name exactly as written in `.nx` source.
    pub name: &'static str,
    /// Named options accepted before the modifier's trailing block.
    pub arguments: &'static [ArgSchema],
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

/// Human-facing type/default/behavior documentation for a component argument.
/// Kept beside the parser catalog so the published parameter table can be
/// generated and checked against the accepted language.
pub struct ComponentParameterDoc {
    pub name: &'static str,
    pub type_name: &'static str,
    pub default: &'static str,
    pub behavior: &'static str,
}

pub const SEGMENTED_CONTROL_PARAMETER_DOCS: &[ComponentParameterDoc] = &[
    ComponentParameterDoc {
        name: "items",
        type_name: "`Array<String>`",
        default: "Required",
        behavior: "Labels and values for the equal-width choices",
    },
    ComponentParameterDoc {
        name: "selected",
        type_name: "mutable `String` state",
        default: "Required",
        behavior: "Current choice; updated when a segment is tapped",
    },
    ComponentParameterDoc {
        name: "animation",
        type_name: "`Spring`, `Tween`, or easing preset",
        default: "Platform default",
        behavior: "Selection movement timing",
    },
    ComponentParameterDoc {
        name: "accessibilityLabel",
        type_name: "`String`",
        default: "Automatic",
        behavior: "Optional VoiceOver/TalkBack control name",
    },
    ComponentParameterDoc {
        name: "comment",
        type_name: "`String`",
        default: "`null`",
        behavior: "Translator note for literal choice labels",
    },
    ComponentParameterDoc {
        name: "tint",
        type_name: "color string or `String` state",
        default: "Platform default",
        behavior: "Fill color of the selected segment",
    },
    ComponentParameterDoc {
        name: "background",
        type_name: "color string or `String` state",
        default: "Platform default",
        behavior: "Segment track fill color",
    },
    ComponentParameterDoc {
        name: "selectedTextColor",
        type_name: "color string or `String` state",
        default: "Platform default",
        behavior: "Label color for the selected segment",
    },
    ComponentParameterDoc {
        name: "textColor",
        type_name: "color string or `String` state",
        default: "Platform default",
        behavior: "Label color for unselected segments",
    },
    ComponentParameterDoc {
        name: "fontSize",
        type_name: "non-negative `Sp` number",
        default: "Platform typography",
        behavior: "Label text size; uses the platform unit (points on iOS, sp on Android)",
    },
    ComponentParameterDoc {
        name: "fontWeight",
        type_name: "`Normal`, `Medium`, `Semibold`, or `Bold`",
        default: "Platform typography",
        behavior: "Label text weight",
    },
    ComponentParameterDoc {
        name: "height",
        type_name: "non-negative `Dp` number",
        default: "32 dp when custom styling is used",
        behavior: "Track height; touch area remains at least 48 dp",
    },
    ComponentParameterDoc {
        name: "cornerRadius",
        type_name: "non-negative `Dp` number",
        default: "Half the track height (pill)",
        behavior: "Track and selected-segment corner radius",
    },
    ComponentParameterDoc {
        name: "borderColor",
        type_name: "color string or `String` state",
        default: "No border",
        behavior: "Track outline color",
    },
    ComponentParameterDoc {
        name: "borderWidth",
        type_name: "non-negative `Dp` number",
        default: "1 dp with `borderColor`; otherwise 0 dp",
        behavior: "Track outline width; uses the platform outline color when no color is supplied",
    },
];

pub fn render_segmented_control_parameter_table() -> String {
    use std::fmt::Write as _;

    let mut out = String::from("| Parameter | Type | Default | Behavior |\n|---|---|---|---|\n");
    for parameter in SEGMENTED_CONTROL_PARAMETER_DOCS {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            parameter.name, parameter.type_name, parameter.default, parameter.behavior
        );
    }
    out.trim_end().to_owned()
}

pub const BOTTOM_SHEET_PARAMETER_DOCS: &[ComponentParameterDoc] = &[
    ComponentParameterDoc {
        name: "isPresented",
        type_name: "mutable `Bool` state",
        default: "Required",
        behavior: "Controls presentation and is reset after native dismissal",
    },
    ComponentParameterDoc {
        name: "contentPadding",
        type_name: "non-negative `Dp` number",
        default: "8 dp",
        behavior: "Uniform inset around sheet content; explicit child padding adds to it",
    },
    ComponentParameterDoc {
        name: "detents",
        type_name: "`Array<SheetDetent>` (`Medium`, `Expanded`)",
        default: "`[Medium, Expanded]`",
        behavior: "Available native resting sizes; at least one unique size is required and cannot be combined with `snapPoints`",
    },
    ComponentParameterDoc {
        name: "initialDetent",
        type_name: "`SheetDetent`",
        default: "First entry in `detents`",
        behavior: "Must be one of the configured detents; cannot be combined with `snapPoints`",
    },
    ComponentParameterDoc {
        name: "snapPoints",
        type_name: "`Array<Number>` fractions in `(0, 1]`",
        default: "Platform detents",
        behavior: "Custom visible-height fractions, ordered from smallest to largest; cannot be combined with `detents`",
    },
    ComponentParameterDoc {
        name: "initialSnapPoint",
        type_name: "`Number` fraction in `(0, 1]`",
        default: "First entry in `snapPoints`",
        behavior: "Must match one configured snap point and requires `snapPoints`",
    },
    ComponentParameterDoc {
        name: "selectedSnapPoint",
        type_name: "mutable `Float64` state",
        default: "Unbound; uses `initialSnapPoint`",
        behavior: "Two-way current snap selection; requires `snapPoints` and cannot be combined with `initialSnapPoint`",
    },
    ComponentParameterDoc {
        name: "showDragIndicator",
        type_name: "`Bool`",
        default: "`true`",
        behavior: "Shows or hides the platform-native drag indicator",
    },
    ComponentParameterDoc {
        name: "allowsDismissal",
        type_name: "`Bool`",
        default: "`true`",
        behavior: "Allows drag dismissal and outside-tap dismissal for modal sheets; Android Back also obeys `shouldDismissOnBackPress`; state changes can always close the sheet",
    },
    ComponentParameterDoc {
        name: "allowsBackgroundInteraction",
        type_name: "`Bool`",
        default: "`false`",
        behavior: "Removes the modal barrier and lets taps reach the screen behind the sheet (iOS 16.4+)",
    },
    ComponentParameterDoc {
        name: "shouldDismissOnBackPress",
        type_name: "`Bool`",
        default: "`true`",
        behavior: "On Android, Back dismisses when `allowsDismissal` is also true; otherwise Back is consumed while the sheet stays open",
    },
    ComponentParameterDoc {
        name: "backgroundColor",
        type_name: "color string or `String` state",
        default: "Platform sheet surface",
        behavior: "Overrides the sheet surface color; use `#00000000` for a transparent surface",
    },
    ComponentParameterDoc {
        name: "scrimColor",
        type_name: "color string or `String` state",
        default: "Platform modal scrim",
        behavior: "Overrides the Android modal dim layer; a transparent color does not remove the modal barrier",
    },
];

pub fn render_bottom_sheet_parameter_table() -> String {
    use std::fmt::Write as _;

    let mut out = String::from("| Parameter | Type | Default | Behavior |\n|---|---|---|---|\n");
    for parameter in BOTTOM_SHEET_PARAMETER_DOCS {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            parameter.name, parameter.type_name, parameter.default, parameter.behavior
        );
    }
    out.trim_end().to_owned()
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
    opt("scale"),
    opt("rotation"),
    opt("shadow"),
    opt("blur"),
    opt("clip"),
    opt("zIndex"),
    opt("glass"),
    opt("animation"),
];

/// Row shares the layout styling options but also exposes placement along its
/// horizontal main axis. `alignment` remains the vertical cross-axis option.
const ROW_LAYOUT_ARGUMENTS: &[ArgSchema] = &[
    opt("spacing"),
    opt("alignment"),
    opt("horizontalArrangement"),
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
    opt("scale"),
    opt("rotation"),
    opt("shadow"),
    opt("blur"),
    opt("clip"),
    opt("zIndex"),
    opt("glass"),
    opt("animation"),
];

/// Visual layout options accepted by overlapping `Stack` containers. Stack
/// has no spacing parameter because its children overlap instead of flowing.
const STACK_ARGUMENTS: &[ArgSchema] = &[
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
    opt("scale"),
    opt("rotation"),
    opt("shadow"),
    opt("blur"),
    opt("clip"),
    opt("zIndex"),
    opt("glass"),
    opt("animation"),
];

/// FastList source keys. The dedicated source reader (`list_source_and_args`
/// in `parser.rs`) consumes these words; they are catalogued here so the
/// argument audit and editor vocabulary cannot drift from the grammar.
pub const FASTLIST_SOURCE_KEYS: &[&str] = &["count", "sections"];
/// FastList row-key option, consumed by the same dedicated source reader.
pub const FASTLIST_KEY_OPTION: &str = "key";
/// Accessibility options shared by visual built-ins, custom component calls,
/// and qualified native plugin components.
pub const ACCESSIBILITY_ARGUMENTS: &[&str] = &[
    "accessibilityLabel",
    "accessibilityHint",
    "accessibilityValue",
    "accessibilityRole",
];

/// Declarative shape of every built-in component, in `COMPONENTS` order.
/// The parser validates invocations against these schemas; semantic lowering
/// converts each validated invocation into the existing typed IR nodes.
pub const COMPONENT_SCHEMAS: &[ComponentSchema] = &[
    ComponentSchema {
        name: "Column",
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
        doc: "components.md#row",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: ROW_LAYOUT_ARGUMENTS,
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Stack",
        doc: "components.md#stack",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: STACK_ARGUMENTS,
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Form",
        doc: "components.md#form",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: &[opt("showDividers"), opt("background")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Section",
        doc: "components.md#section",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Optional,
        positional: PositionalModel::None,
        arguments: &[
            opt("title"),
            opt("footer"),
            opt("comment"),
            opt("showDividers"),
            opt("background"),
        ],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Spacer",
        doc: "components.md#spacer",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Empty,
        positional: PositionalModel::None,
        arguments: &[],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Divider",
        doc: "components.md#divider",
        features: &[FeatureTag::Layout],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("color"), req("thickness")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Text",
        doc: "components.md#text",
        features: &[FeatureTag::Typography],
        parens: ParensModel::Required,
        positional: PositionalModel::Single,
        arguments: &[
            opt("comment"),
            opt("color"),
            opt("alignment"),
            opt("fontSize"),
            opt("fontStyle"),
            opt("fontDesign"),
            opt("fontWeight"),
            opt("padding"),
            opt("opacity"),
            opt("scale"),
            opt("rotation"),
            opt("shadow"),
            opt("blur"),
            opt("clip"),
            opt("zIndex"),
            opt("lineLimit"),
            opt("lineHeight"),
            opt("letterSpacing"),
            opt("strikethrough"),
            opt("italic"),
            opt("underline"),
            opt("selectable"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "ContentUnavailable",
        doc: "components.md#content-unavailable",
        features: &[
            FeatureTag::Layout,
            FeatureTag::Typography,
            FeatureTag::Media,
        ],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("title"),
            req("icon"),
            opt("description"),
            opt("titleColor"),
            opt("descriptionColor"),
            opt("iconTint"),
            opt("background"),
            opt("comment"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Button",
        doc: "components.md#button",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::Single,
        arguments: &[
            opt("comment"),
            opt("icon"),
            opt("loading"),
            opt("disabled"),
            opt("style"),
            opt("size"),
            opt("shape"),
            opt("tint"),
            opt("glass"),
        ],
        exclusive: &[],
        children: ChildModel::OptionalActions,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "TextInput",
        doc: "components.md#textinput",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("value"),
            req("placeholder"),
            opt("comment"),
            opt("keyboardType"),
            opt("isSecure"),
            opt("autofill"),
            opt("returnKeyType"),
            opt("multiline"),
            opt("autocorrect"),
            opt("capitalization"),
            opt("focused"),
            opt("maxLength"),
            opt("font"),
            opt("minLines"),
            opt("maxLines"),
            opt("searchable"),
        ],
        exclusive: &[],
        children: ChildModel::OptionalActions,
        modifiers: &[ModifierSchema {
            name: "onChange",
            arguments: &[],
            body: ModifierBody::EventActions,
            required: false,
        }],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Switch",
        doc: "components.md#switch",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("value"), req("label"), opt("comment")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Slider",
        doc: "components.md#slider",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("value"), req("min"), req("max"), req("step")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "ProgressBar",
        doc: "components.md#progressbar",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            opt("progress"),
            opt("tint"),
            opt("trackColor"),
            opt("thickness"),
            opt("cornerRadius"),
            opt("android_wave"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "ProgressRing",
        doc: "components.md#progressring",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            opt("progress"),
            opt("tint"),
            opt("trackColor"),
            opt("size"),
            opt("thickness"),
            opt("android_wave"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "SegmentedControl",
        doc: "components.md#segmentedcontrol",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("items"),
            req("selected"),
            opt("animation"),
            opt("comment"),
            opt("tint"),
            opt("background"),
            opt("selectedTextColor"),
            opt("textColor"),
            opt("fontSize"),
            opt("fontWeight"),
            opt("height"),
            opt("cornerRadius"),
            opt("borderColor"),
            opt("borderWidth"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Picker",
        doc: "components.md#picker",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("items"),
            req("selected"),
            opt("itemIcons"),
            opt("icon"),
            opt("label"),
            opt("tint"),
            opt("comment"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "PickerDate",
        doc: "components.md#pickerdate",
        features: &[FeatureTag::Controls],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("type"),
            opt("style"),
            opt("selection"),
            opt("timestamp"),
            opt("startTimestamp"),
            opt("endTimestamp"),
            opt("minimum"),
            opt("maximum"),
            opt("tint"),
            opt("textColor"),
            opt("backgroundColor"),
            opt("pickerBackgroundColor"),
            opt("borderColor"),
            opt("borderWidth"),
            opt("cornerRadius"),
            opt("locale"),
            opt("timeZone"),
            opt("is24Hour"),
            opt("enabled"),
            opt("accessibilityLabel"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Image",
        doc: "components.md#image",
        features: &[FeatureTag::Media],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            opt("asset"),
            opt("url"),
            opt("file"),
            req("description"),
            opt("scale"),
            opt("placeholder"),
            opt("showLoadingIndicator"),
            opt("maxHeight"),
            opt("fillMaxSize"),
        ],
        exclusive: &[ExclusiveGroup {
            options: &["asset", "url", "file"],
            missing_message: "Image requires exactly one of `asset`, `url`, or `file`",
            both_message: "Image accepts exactly one of `asset`, `url`, or `file`",
        }],
        children: ChildModel::None,
        modifiers: &[ModifierSchema {
            name: "sharedElement",
            arguments: &[req("id")],
            body: ModifierBody::None,
            required: false,
        }],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Icon",
        doc: "components.md#icon",
        features: &[FeatureTag::Media],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            opt("system"),
            opt("sfsymbol"),
            opt("materialsymbol"),
            req("description"),
            req("size"),
            req("tint"),
        ],
        exclusive: &[ExclusiveGroup {
            options: &["system", "sfsymbol", "materialsymbol"],
            missing_message: "Icon requires exactly one of `system`, `sfsymbol`, or `materialsymbol`",
            both_message: "Icon accepts exactly one of `system`, `sfsymbol`, or `materialsymbol`",
        }],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "LinearGradient",
        doc: "components.md#lineargradient",
        features: &[FeatureTag::Media],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("startColor"),
            req("endColor"),
            opt("direction"),
            opt("height"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Pressable",
        doc: "components.md#pressable",
        features: &[FeatureTag::Interactivity],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("disabled"), opt("haptic"), opt("fillMaxSize")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[
            ModifierSchema {
                name: "onTap",
                arguments: &[],
                body: ModifierBody::Actions,
                required: true,
            },
            ModifierSchema {
                name: "onLongPress",
                arguments: &[opt("durationMs")],
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "contextMenu",
                arguments: &[],
                body: ModifierBody::Nodes,
                required: false,
            },
            ModifierSchema {
                name: "onDoubleTap",
                arguments: &[],
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "onDrag",
                arguments: &[],
                body: ModifierBody::EventActions,
                required: false,
            },
            ModifierSchema {
                name: "onPinch",
                arguments: &[],
                body: ModifierBody::EventActions,
                required: false,
            },
        ],
        flags: &[],
        trailing_message: Some(
            "Pressable actions must use `.onTap { ... }`, with optional `.onLongPress(durationMs: 500) { ... }`, `.contextMenu { ... }`, `.onDoubleTap { ... }`, `.onDrag { x, y, vx, vy -> ... }`, or `.onPinch { scaleFactor -> ... }`",
        ),
    },
    ComponentSchema {
        name: "NavigationStack",
        doc: "components.md#navigationstack--navigationlink",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req_msg(
                "root",
                "NavigationStack requires a declared screen as `root`",
            ),
            opt("titleMode"),
            opt("titleAlignment"),
        ],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "NavigationSplitView",
        doc: "components.md#navigationsplitview",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("detailVisible")],
        exclusive: &[],
        children: ChildModel::SplitPanes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "NavigationLink",
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
        doc: "components.md#navigationstack--navigationlink",
        features: &[FeatureTag::Navigation],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("label"), opt("comment")],
        exclusive: &[],
        children: ChildModel::None,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Link",
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
        name: "KeyboardAware",
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
        doc: "components.md#bottomsheet",
        features: &[FeatureTag::Overlays],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("isPresented"),
            opt("contentPadding"),
            opt("detents"),
            opt("initialDetent"),
            opt("snapPoints"),
            opt("initialSnapPoint"),
            opt("selectedSnapPoint"),
            opt("showDragIndicator"),
            opt("allowsDismissal"),
            opt("allowsBackgroundInteraction"),
            opt("shouldDismissOnBackPress"),
            opt("backgroundColor"),
            opt("scrimColor"),
        ],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[ModifierSchema {
            name: "onDismiss",
            arguments: &[],
            body: ModifierBody::Actions,
            required: false,
        }],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "Dialog",
        doc: "components.md#dialog",
        features: &[FeatureTag::Overlays],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[
            req("isPresented"),
            req("title"),
            req("message"),
            opt("comment"),
        ],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "ConfirmationDialog",
        doc: "components.md#confirmationdialog",
        features: &[FeatureTag::Overlays],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("isPresented"), req("title"), opt("comment")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "RefreshControl",
        doc: "components.md#refreshcontrol",
        features: &[FeatureTag::Refresh],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("isRefreshing")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[ModifierSchema {
            name: "onRefresh",
            arguments: &[],
            body: ModifierBody::Actions,
            required: true,
        }],
        flags: &[],
        trailing_message: Some("RefreshControl actions must use `.onRefresh { ... }`"),
    },
    ComponentSchema {
        name: "AppBottomBar",
        doc: "components.md#appbottombar",
        features: &[FeatureTag::Tabs],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("selected"), opt("tint")],
        exclusive: &[],
        children: ChildModel::Tabs,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "PagePager",
        doc: "components.md#pagepager",
        features: &[FeatureTag::Navigation],
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
        name: "Toolbar",
        doc: "components.md#toolbar",
        features: &[FeatureTag::Tabs],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[opt("placement")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "FastList",
        doc: "components.md#fastlist",
        features: &[FeatureTag::Lists],
        parens: ParensModel::Required,
        positional: PositionalModel::ListSource,
        arguments: &[
            opt("axis"),
            opt("rowHeight"),
            opt("spacing"),
            opt("scrollPosition"),
            opt("scrollEnabled"),
            opt("reverseLayout"),
            opt("pageSnap"),
            opt("showDividers"),
        ],
        exclusive: &[],
        children: ChildModel::ListRows,
        modifiers: &[
            ModifierSchema {
                name: "onEndReached",
                arguments: &[],
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "onScroll",
                arguments: &[],
                body: ModifierBody::Actions,
                required: false,
            },
            ModifierSchema {
                name: "scrollToIndex",
                arguments: &[req("index")],
                body: ModifierBody::None,
                required: false,
            },
            ModifierSchema {
                name: "scrollToTop",
                arguments: &[req("trigger")],
                body: ModifierBody::None,
                required: false,
            },
            ModifierSchema {
                name: "scrollToEnd",
                arguments: &[req("trigger")],
                body: ModifierBody::None,
                required: false,
            },
            ModifierSchema {
                name: "scrollToOffset",
                arguments: &[req("offset")],
                body: ModifierBody::None,
                required: false,
            },
            ModifierSchema {
                name: "onMove",
                arguments: &[opt("enabled")],
                body: ModifierBody::EventActions,
                required: false,
            },
            ModifierSchema {
                name: "stickyHeader",
                arguments: &[],
                body: ModifierBody::Nodes,
                required: false,
            },
            ModifierSchema {
                name: "sectionHeader",
                arguments: &[],
                body: ModifierBody::Nodes,
                required: false,
            },
            ModifierSchema {
                name: "swipeActions",
                arguments: &[],
                body: ModifierBody::Nodes,
                required: false,
            },
        ],
        flags: &[],
        trailing_message: Some(
            "FastList modifiers must use dot syntax, for example `.onEndReached { ... }` or `.onMove { from, to -> ... }`",
        ),
    },
    ComponentSchema {
        name: "Appearance",
        doc: "components.md#appearance",
        features: &[FeatureTag::Theming],
        parens: ParensModel::Required,
        positional: PositionalModel::None,
        arguments: &[req("mode")],
        exclusive: &[],
        children: ChildModel::Nodes,
        modifiers: &[],
        flags: &[],
        trailing_message: None,
    },
    ComponentSchema {
        name: "StatusBar",
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

/// Look up a component schema by its source spelling.
pub fn component_schema(name: &str) -> Option<&'static ComponentSchema> {
    COMPONENT_SCHEMAS.iter().find(|schema| schema.name == name)
}

/// Whether `name` is a built-in component spelling.
pub fn is_component(name: &str) -> bool {
    component_schema(name).is_some()
}

/// Whether a built-in has a user-visible rendered node that can receive
/// accessibility semantics.
pub fn supports_accessibility_options(schema: &ComponentSchema) -> bool {
    !matches!(
        schema.name,
        "Appearance"
            | "Content"
            | "Direction"
            | "OnAppear"
            | "OnDisappear"
            | "OnActive"
            | "OnInactive"
            | "OnBackground"
            | "StatusBar"
    ) && !matches!(schema.parens, ParensModel::None)
}

/// Names a schema's parenthesis group accepts, for `named_args` validation.
pub fn schema_option_names(schema: &ComponentSchema) -> Vec<&'static str> {
    let mut names: Vec<_> = schema.arguments.iter().map(|arg| arg.name).collect();
    if supports_accessibility_options(schema) {
        names.extend(ACCESSIBILITY_ARGUMENTS);
    }
    names
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
pub const GRAMMAR_EXTRA_COMPONENTS: &[&str] = &["Tab", "TabBar"];

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
    let options: Vec<String> = schema_option_names(schema)
        .iter()
        .map(|name| {
            if schema
                .arguments
                .iter()
                .any(|arg| arg.required && arg.name == *name)
            {
                (*name).to_owned()
            } else {
                format!("{name}:")
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
        ChildModel::Tabs if schema.name == "AppBottomBar" => format!(
            "`{}(selected:) {{ Tab(..) ... TabBar {{ ... }}? }}`",
            schema.name
        ),
        ChildModel::Tabs => format!("`{}(selected:) {{ Tab(..) ... }}`", schema.name),
        ChildModel::SplitPanes => format!(
            "`{}(detailVisible:) {{ Sidebar {{ ... }} Detail {{ ... }} }}`",
            schema.name
        ),
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
        | **Scope**: accepted `.nx` component grammar | **Source**: compiler catalog | **Status**: generated and parse-checked |\n\n\
        This reference lists every built-in component's accepted arguments, child blocks, and event, interaction, or composition modifiers. Visual styling is expressed through component arguments. The entries are generated from the parser catalog and checked by parser probes.\n\n\
        > Generated from `crates/nexa-syntax/src/catalog.rs` — do not edit by hand.\n\
        > Run `cargo test -p nexa-syntax` with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate.\n\
        > Every entry mirrors a parser production: the catalog test suite parses\n\
        > each component probe, so an audit entry without a working probe fails.\n\n\
        Each component lists its canonical signature, argument requirements,\n\
        child-block model, trailing modifiers, and manual documentation anchor.\n\
        Required options are bare names; optional options carry a trailing colon.\n\n\
        ## Quick start\n\n\
        This valid `.nx` app shows the basic component and state syntax covered by this catalog.\n\n\
        ```nx\n\
        app SyntaxAuditQuickStart {\n\
            state savedItems: Int32 = 0\n\n\
            body {\n\
                Column(spacing: 8, padding: 16) {\n\
                    Text(\"Saved items: \\(savedItems)\")\n\
                    Button(\"Save an item\") {\n\
                        savedItems = savedItems + 1\n\
                    }\n\
                }\n\
            }\n\
        }\n\
        ```\n\n",
    );
    out.push_str("Sections:\n\n");
    for tag in FEATURE_ORDER {
        if *tag == FeatureTag::Accessibility
            || COMPONENT_SCHEMAS
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
        if *tag == FeatureTag::Accessibility {
            out.push_str(
                "## Accessibility\n\n\
                ### Component options\n\n\
                Pass accessibility options directly to a visual built-in, custom component, or qualified native plugin component.\n\n\
                Signature: `Component(..., accessibilityLabel: String, accessibilityHint: String, accessibilityValue: String, accessibilityRole: None|Button|Link|Header|Image)`\n\n\
                `accessibilityLabel` is required whenever any accessibility option is present. Labels, hints, and values accept typed `String` expressions; literal labels and hints must be non-empty. The options are optional and may be used with the component's normal children and modifiers.\n\n\
                Swift emits native accessibility modifiers. Android emits Compose semantics, including hint text and heading semantics for `Header`.\n\n\
                Reference: components.md#accessibility-options\n\n",
            );
            continue;
        }
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
            let (required, _optional): (Vec<_>, Vec<_>) =
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
            let required_names: std::collections::HashSet<_> =
                required.iter().map(|arg| arg.name).collect();
            let optional_names: Vec<_> = schema_option_names(schema)
                .into_iter()
                .filter(|name| !required_names.contains(name))
                .collect();
            if !optional_names.is_empty() {
                out.push_str(&format!(
                    "Optional options: {}\n\n",
                    optional_names
                        .iter()
                        .map(|name| format!("`{name}`"))
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
                    ChildModel::Tabs if schema.name == "AppBottomBar" => {
                        "`Tab` destinations and an optional custom `TabBar` block"
                    }
                    ChildModel::Tabs => "`Tab` declarations",
                    ChildModel::SplitPanes => "sidebar and detail blocks",
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
                            ModifierBody::None => "no block",
                            ModifierBody::Actions => "actions",
                            ModifierBody::EventActions => "event actions",
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
