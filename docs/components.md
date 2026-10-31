# Nexa Component Guide

Nexa UI components compile to SwiftUI on iOS and Jetpack Compose on Android. This guide shows common composition patterns; the [generated syntax audit](syntax-audit.md) is the complete reference for every built-in component, argument, child block, and supported event modifier.

| **Scope**: built-in UI composition | **Targets**: iOS and Android | **Reference**: [generated component syntax](syntax-audit.md) |

## Quick start

```nx
app ProfileCard {
    state notificationsEnabled: Bool = true

    body {
        Column(spacing: 12, padding: 16, background: "#F1F5F9", cornerRadius: 12) {
            Row(spacing: 10, alignment: Center) {
                Icon(system: "account_circle", description: "Profile", size: 40, tint: "#2563EB")
                Column(spacing: 3) {
                    Text("Alex Morgan", fontSize: 20, fontWeight: Semibold)
                    Text("Product designer", fontSize: 14, color: "#475569")
                }
                Spacer()
            }
            Switch(value: notificationsEnabled, label: "Project updates")
        }
    }
}
```

## Custom icon fonts

Register one TTF or OTF font in `nexa.config.nx` and name the glyph codepoints once. The aliases compile into native font glyphs and work with built-in `icon:` parameters on both platforms.

```nexa
config {
    assets {
        iconFonts { Mazicon: "assets/fonts/Mazicon.ttf" },
        icons {
            mazica_play: ["Mazicon", 61696],
            mazica_search: ["Mazicon", 61697],
        }
    }
}
```

Put the font directly in `assets/fonts/`; its filename stem must match the declared family so Android can resolve the resource statically. Use aliases in `Button(icon:)`, `ContentUnavailable(icon:)`, `Picker(icon:)`, `Picker(itemIcons:)`, `Tab(icon:)`, or `Icon(system:)`. The compiler checks the font path, family, alias, and Unicode codepoint and emits a typed glyph without runtime name lookup.

```nexa
app PlayerActions {
    state filter: String = "All"

    body {
        Button("Play", icon: "mazica_play") { }
        Picker(items: ["All"], selected: filter, itemIcons: ["mazica_search"])
        Icon(system: "mazica_search", description: "Search", size: 24, tint: "#FFFFFF")
    }
}
```

## Layout and styling

Use `Column` for vertical flow, `Row` for horizontal flow, and `Stack` for overlapping content. Layout and visual options belong in each component's argument list. Nexa does not support SwiftUI-style styling chains such as `.padding()` or `.background()`.

```nx
app ReleaseSummary {
    body {
        Stack(alignment: Center, width: 320, height: 180, background: "#0F172A", cornerRadius: 16) {
            Image(asset: "release_banner", description: "Abstract blue release banner", scale: Fill, maxHeight: 180)
            Column(spacing: 8, padding: 16, alignment: Center) {
                Text("Version 2.4 is ready", fontSize: 22, fontWeight: Bold, color: "#FFFFFF")
                Text("Includes offline search and faster sync.", fontSize: 14, color: "#E2E8F0")
            }
        }
    }
}
```

The exact layout, typography, accessibility, and visual options are listed for each component in the [syntax audit](syntax-audit.md#layout) and [`crates/nexa-syntax/src/catalog.rs`](../crates/nexa-syntax/src/catalog.rs).

For a branded bottom toolbar, use Nexa's recommended [`AppBottomBar` custom `TabBar` pattern](state-and-navigation.md#recommended-custom-bottom-toolbar). It keeps tab routing in `AppBottomBar` and gives the app full control of the visible toolbar layout. Use `Toolbar` for native actions attached to a screen's top navigation bar; it is a different surface.

Layout defaults are shared by iOS and Android: `Column` aligns its children and text to `Center`, `Row` aligns children vertically to `Center` and starts its horizontal arrangement at `Start`, and `Stack` centers its content. Omitted `padding` adds no inset. Explicit layout parameters replace these defaults.

`width` and `height` set a fixed content size. `minWidth`, `maxWidth`, `minHeight`, and `maxHeight` constrain that size or the content's natural size. A fixed size must fit within any supplied bounds; contradictory values are compile errors. Padding is added outside the content size, so it increases the component's total size.

### Shape

Use `Shape` for filled geometry such as a page indicator, badge dot, or divider pill. It emits a concrete native path instead of relying on an empty layout container's background.

```nx
app WelcomeIndicator {
    body {
        Row(spacing: 4) {
            Shape(shape: Capsule, width: 28, height: 4, fill: "#FFFFFF")
            Shape(shape: Circle, width: 4, height: 4, fill: "#FFFFFF")
            Shape(shape: Rounded(2), width: 16, height: 4, fill: "#FFFFFF")
        }
    }
}
```

| Option | Type | Required | Default | Behavior |
|---|---|---:|---|---|
| `shape` | `Capsule \| Circle \| Rounded(radius: Double)` | Yes | — | Selects the native path. `Rounded` uses a logical-point radius. |
| `width` | `Double` | Yes | — | Shape width in logical points. |
| `height` | `Double` | Yes | — | Shape height in logical points. |
| `fill` | `Color` | Yes | — | Static or adaptive fill color. |

`Shape` has no child block or component-specific dot modifiers. SwiftUI receives a concrete `Capsule`, `Circle`, or `RoundedRectangle`; Compose paints a sized `Spacer` with the matching shape.

### Spacer

`Spacer()` expands along its parent layout axis. Give it a fixed `width` or `height` to express a gap without an empty layout container; dimensions use points on iOS and dp on Android.

```nx
app OnboardingLayout {
    body {
        Column {
            Spacer(height: 234)
            Text("MAZICA", fontSize: 86, fontWeight: Semibold)
            Spacer()
            Text("Discover music picked for you.")
            Spacer(height: 29)
            Button("Continue") { }
        }
    }
}
```

`Column`, `Row`, and `Stack` also accept `screenWidthFraction` and `screenHeightFraction` from `0` to `1` for proportional dimensions. They use native screen metrics on iOS and Android. `screenWidthFraction` cannot be combined with `width` or `fillMaxWidth`; `screenHeightFraction` cannot be combined with `height` or `fillMaxHeight`.

Set `cornerRadiusBottom` to round both bottom corners while keeping the top corners square. It can be combined with `cornerRadius`; when both are present, `cornerRadiusBottom` overrides the bottom pair. SwiftUI uses an uneven rounded rectangle and Compose uses a four-corner `RoundedCornerShape`.

Use `fillMaxWidth: true` or `fillMaxHeight: true` when a layout should consume the corresponding space its parent offers. They compose with `minWidth`, `maxWidth`, `minHeight`, and `maxHeight`, but cannot be combined with a fixed size on the same axis. Both platforms use their native flexible frame and layout modifiers; omitted flags preserve intrinsic sizing.

`Row(alignment:)` positions children vertically (`Start`, `Center`, or `End`). Use `horizontalArrangement: Start|Center|End` to place the child group across the available width; it defaults to `Start`. Setting `horizontalArrangement` makes the Row use available width, constrained by any explicit width bounds.

Use `scale` for uniform visual scaling and `scaleY` for an independent vertical transform on `Column`, `Row`, `Stack`, and `Text`. Both accept a `Float64` expression and animate when the state is changed inside `withAnimation`; when both are provided, `scaleY` overrides the vertical part while `scale` controls the horizontal part.

### System bars

Set native system-bar appearance once at the app or screen root. Nexa draws requested backgrounds into the system insets while retaining native status icons.

| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `style` | `Default \| Light \| Dark` | `Default` | Selects status-bar content appearance. `Light` requests light icons and `Dark` requests dark icons. |
| `hidden` | `Bool` | `false` | Shows or hides the status bar. |
| `background` | Hex color | None | Fills the status-bar inset on Android and iOS. |
| `navigationBarBackground` | Hex color | None | Fills the Android navigation-bar inset. iOS ignores this Android-only surface color. |

```nx
app MazicaShell {
    screen Home { Text("Home") }

    body {
        StatusBar(style: Light, hidden: false, navigationBarBackground: "#000000")
        NavigationStack(root: Home)
    }
}
```

## Safe area and edge-attached content

Nexa roots draw edge-to-edge by default. A background image or gradient can extend beneath system bars while a foreground subtree opts into visual-safe placement. Use ordinary `Stack`, `Column`, and `Row` composition; no screen-region container is required.

```nx
app StoryViewer {
    body {
        Stack {
            Image(asset: "story_cover", description: "Mountain landscape", scale: Fill, maxHeight: 640)
            Column(spacing: 12, padding: 20, alignment: Center) {
                Text("A weekend above the clouds", fontSize: 26, fontWeight: Bold, color: "#FFFFFF")
                Text("Read the field notes", fontSize: 16, color: "#F1F5F9")
            }
            .safeAreaPadding(edges: [Top, Leading, Trailing], spacing: 16)
        }
        .safeAreaInset(edge: Bottom, spacing: 12) {
            Button("Save story") { }
        }
    }
}
```

The generated [syntax audit](syntax-audit.md#shared-view-modifiers) is the parameter reference for both modifiers, including accepted edge names, defaults, and required arguments.

| Concern | Contract |
|---|---|
| Visual safe insets | SwiftUI reads `GeometryProxy.safeAreaInsets` before expanding only the app child to full bleed; it adds those insets once to the safe-region size to recover the window viewport. Toolbar-free navigation roots and destinations use that viewport edge-to-edge, and Nexa safe-area modifiers apply the selected insets. Android uses the per-edge union of system bars, display cutouts, and waterfall insets. Overlapping sources are not added together. Values follow system visibility, orientation, window resizing, and layout direction without polling or inset-reader state. |
| Gesture regions | System gesture, mandatory gesture, and tappable-element insets are not part of visual safe-area padding and are not consumed by these modifiers. |
| IME | Keyboard/IME height is separate from visual safe insets and fixed spacing. `KeyboardAware` defaults to `Stay`, keeping its scope anchored to the window while the IME can cover content; `behavior: Avoid` opts into resizing above the IME. |
| Native scrolling content | `FastList`, `Form`, and `KeyboardAware` keep scroll content within remaining visual safe insets. When a unique owner is accommodated, its scroll viewport reserves the attachment's measured extent, fixed gap, and selected dynamic inset; surrounding backgrounds and other composed layers can still fill the screen. SwiftUI uses native `safeAreaInset`; Compose applies padding to the scroll container. An ancestor `safeAreaPadding` consumes its selected edges first. Reverse-layout and page-snapping lists preserve their specialized native inset behavior. |
| Fixed spacing | `safeAreaPadding` adds its fixed `spacing` after the remaining selected visual inset. `safeAreaInset` offsets its attached content by the fixed gap; for a unique scroll owner, the gap is included once in that owner's viewport reservation. Neither value is consumed as a system inset. |
| Attached content size | `safeAreaInset` measures its node block during the platform layout pass. Android uses subcomposition, so the measured extent is available to the body and scroll viewport in that same pass without a size-state write and follow-up composition. The attachment extent stays distinct from the dynamic safe inset and fixed gap. |
| Cutouts and orientation | Edge attachments also inset their cross-axis content around cutouts and curved display edges. The platform reports logical leading/trailing values after layout direction is applied; rotating or resizing updates the native inset modifiers. |
| Nested scopes | A selected dynamic edge is consumed once for descendants. Nested safe-area padding receives only the remaining inset, so selecting the same edge again adds no second system inset. |
| Scroll owner | Placement works without scrolling. When exactly one compatible scroll owner is found, its native viewport reserves the attachment's measured extent, fixed gap, and remaining visual inset so rows do not render behind the attachment. No artificial row or per-item inset is added. If ownership is ambiguous, placement remains and the compiler does not choose an arbitrary owner. |

For a feed, the same edge attachment is automatically coordinated with the unique `FastList` scroll owner:

```nx
app ReadingFeed {
    body {
        FastList(count: 32) { index in
            Column(spacing: 4, padding: 12) {
                Text("Field note \(index + 1)", fontSize: 18, fontWeight: Semibold)
                Text("A short update from the expedition.", fontSize: 14, color: "#64748B")
            }
        }
        .safeAreaInset(edge: Bottom, spacing: 8) {
            Button("Write a note") { }
        }
    }
}
```

SwiftUI keeps native scroll behavior through `safeAreaInset` when a compatible owner is unique. Compose keeps `FastList`/`Form` virtualization and applies the measured reservation to the scroll container, outside list items. AOT and DevRuntime use the same typed edge, spacing, and owner resolution. `safeAreaPadding` and `safeAreaInset` do not alter gesture policy or replace `KeyboardAware`.

### Adaptive scroll headers

Attach one custom header to a subtree containing exactly one vertical native scroll owner. The modifier can wrap `FastList`, `Form`, or a future compatible owner; it does not replace scrolling or introduce an extra scroll container. If there are zero or multiple compatible owners, the compiler reports the relationship and asks you to place the modifier on the intended owner.

| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `behavior` | `Pinned \| Collapse \| HideOnScroll` | `Pinned` | `Pinned` keeps the measured header height. `Collapse` shrinks it linearly as the owner scrolls through the first 120 logical points/dp. `HideOnScroll` hides after 8 logical points/dp of downward content movement and reveals on upward movement. |
| `collapsedHeight` | `Float64` (points on iOS, dp on Android) | `56` for `Collapse` | The minimum height used by `Collapse`. It must be positive and is valid only with `behavior: Collapse`. |
| Header block | Node block | Required | Ordinary Nexa composition for the header. Use `safeAreaPadding` in this block when header controls need protection from a system edge; header behavior does not consume visual safe insets. |

```nx
app Preferences {
    state notificationsEnabled: Bool = true

    body {
        Form {
            Section(title: "Notifications") {
                Switch(value: notificationsEnabled, label: "Daily updates")
            }
        }
        .scrollHeader(behavior: Collapse, collapsedHeight: 56) {
            Column(padding: 16, background: "#F8FAFC") {
                Text("Preferences", fontSize: 30, fontWeight: Bold)
                    .safeAreaPadding(edges: [Top])
            }
        }
    }
}
```

The system `NavigationStack(titleMode: Large)` remains the native choice for standard large titles. Custom header tracking is local to the header: Swift observes the native scroll view offset, Compose observes nested vertical scroll deltas, and neither backend measures list rows. Hide/reveal honors reduced-motion settings on iOS and the system animation scale on Android. Scroll state and list identity remain owned by `FastList` or `Form`.

### KeyboardAware

`KeyboardAware` preserves its existing scroll dismissal behavior and can either stay fixed to the window or avoid the software keyboard. It reuses a nested `FastList` or `Form` as the native scroll owner; when there is no scroll owner, it supplies a vertical scrolling container.

| Argument | Type | Default | Behavior |
|---|---|---|---|
| `dismiss` | `Interactive \| Never` | `Interactive` | Controls whether a drag can dismiss the software keyboard. |
| `behavior` | `Stay \| Avoid` | `Stay` | `Stay` leaves the scope at the window position and allows the IME to cover content. `Avoid` resizes or moves the scope above the IME. |

```nx
app MessageComposer {
    state message: String = ""

    body {
        KeyboardAware(behavior: Avoid, dismiss: Interactive) {
            Column(spacing: 12, padding: 16) {
                Text("Message the expedition team", fontWeight: Semibold)
                TextInput(value: message, placeholder: "Write a message")
            }
        }
    }
}
```

The Android host uses a non-resizing IME policy by default so `Stay` remains anchored to the physical window. `Avoid` applies native IME insets to the `KeyboardAware` scope. Text input focus and keyboard dismissal continue to use native platform behavior.

## Input and interaction

Bind controls directly to declared `state`. `TextInput` and `Switch` accept typed `.onChange { ... }` actions when a control change also needs side effects. `Button` action blocks contain app state updates.

### Button

`Button` renders a native action on both platforms. Its optional `accessibilityLabel` is useful for icon-only actions; without it, assistive technology uses the visible label or the platform's native icon description.

| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `label` | `String` expression | Required positional value | Visible button text. |
| `comment` | `String` | None | Translator context for the visible label. |
| `icon` | Nexa shared icon or `assets.icons` alias (`String`) | None | Adds a shared or registered custom glyph; unknown names are compile errors. |
| `loading` | `Bool` expression | `false` | Replaces the label with a native progress indicator and disables activation. |
| `disabled` | `Bool` expression | `false` | Disables activation while retaining the label. |
| `style` | `BorderedProminent`, `Bordered`, `Borderless`, or `Plain` | `BorderedProminent` | Selects the shared button treatment. |
| `size` | `Small`, `Regular`, or `Large` | Native default | Selects the native control size. |
| `shape` | `Capsule`, `Circle`, or `Rounded(radius)` | Shared capsule | Selects the outline shape. |
| `tint` | Color expression (`String`) | Platform accent | Overrides the button tint. |
| `glass` | `Bool` | `false` | Enables the native glass treatment where available. |
| `accessibilityLabel` | `String` expression | Native label | Overrides the accessible name, including while the button is loading. |
| action block | Actions | Required | Runs when the enabled button is activated. |

```nx
Button("", icon: "add", shape: Circle, accessibilityLabel: "Add task") {
    taskCount = taskCount + 1
}
```

### TextInput

`TextInput` binds directly to mutable `String` state and uses the native text control on each
platform. Static font and color options are typed at compile time. Placeholder strings are
localized automatically. `clearable` adds the iOS inline clear control; Android retains its
native field behavior, matching React Native's iOS-only clear-button mode. Use `.onFocus` with
`focused` when an interaction must run after the user focuses the field.

<!-- nexadoc:begin TEXT_INPUT_PARAMETERS -->
| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `value` | mutable `String` state | Required | Current text; native edits update the bound state before `.onChange` runs |
| `placeholder` | `String` literal | Required | Shown while the bound value is empty and automatically localized |
| `comment` | `String` literal | Omitted | Translator context for the placeholder |
| `keyboardType` | `Text`, `Number`, `Email`, `Phone`, or `Url` | `Text` | Selects the native keyboard layout |
| `isSecure` | `Bool` | `false` | Masks the entered text and selects secure-entry behavior |
| `autofill` | `Username`, `Password`, or `OneTimeCode` | Omitted | Hints the native autofill service about the expected value |
| `returnKeyType` | `Done`, `Search`, `Send`, or `Next` | Native default; `Done` for an action block and `Search` for `searchable` | Sets the keyboard action; submit actions run before native dismissal |
| `multiline` | `Bool` | `false` | Uses a vertically expanding native text control; submit action blocks are not supported |
| `autocorrect` | `Bool` | Platform default | Enables or disables native spelling correction |
| `capitalization` | `None`, `Sentences`, `Words`, or `Characters` | `Sentences` | Selects automatic capitalization behavior |
| `focused` | mutable `Bool` state | Omitted | Requests focus when true and reflects native focus changes |
| `maxLength` | positive integer literal | Unlimited | Bounds text accepted from native editing |
| `font` | `Body` or `Title3` | Platform default | Selects a shared native text style |
| `fontSize` | non-negative number or `FontSize` theme token | Platform default | Sets entered-text and placeholder size in points on iOS and `sp` on Android |
| `fontFamily` | registered font-face `String` or font-family theme token | `Theme.Default` when declared, otherwise platform default | Uses the registered native font face from `assets/fonts/` |
| `textColor` | hex color `String` or color theme token | Platform default | Sets the entered-text color |
| `placeholderColor` | hex color `String` or color theme token | Platform default | Sets the placeholder color independently of entered text |
| `clearable` | `Bool` | `false` | Shows an inline clear control on iOS while text is present; Android keeps its native field behavior |
| `minLines` | positive integer literal | 1 | Minimum visible line count; requires `multiline: true` |
| `maxLines` | positive integer literal | Unlimited | Maximum visible line count; requires `multiline: true` |
| `searchable` | `Bool` | `false` | Adds a native search icon and uses the Search return-key action; cannot be combined with multiline or submit actions |
<!-- nexadoc:end TEXT_INPUT_PARAMETERS -->

<!-- nexadoc:begin TEXT_INPUT_EVENTS -->
| Action or modifier | Type | Default | Behavior |
|---|---|---|---|
| `action block` | actions | Omitted | Runs on submit for a single-line field; defaults the return key to `Done` |
| `.onChange` | one `String` binding and actions | Omitted | Runs after user-driven edits with the new text bound to its parameter; programmatic state changes do not trigger it |
| `.onFocus` | actions | Omitted | Runs when the field gains native focus; requires `focused` so both native targets can track the focus binding |
<!-- nexadoc:end TEXT_INPUT_EVENTS -->

Common accessibility options also apply: `accessibilityLabel` is required when any of
`accessibilityHint`, `accessibilityValue`, or `accessibilityRole` is supplied. Accessibility
strings are localized automatically.

```nx
app SongSearchField {
    state query: String = ""
    state changedText: String = ""
    state focused: Bool = false
    state focusEntries: Int32 = 0

    body {
        Column(spacing: 8, padding: 16) {
            TextInput(
                value: query,
                placeholder: "Search songs and artists",
                keyboardType: Text,
                returnKeyType: Search,
                autocorrect: false,
                capitalization: None,
                fontSize: 16,
                textColor: "#FFFFFF",
                placeholderColor: "#9F9F9F",
                clearable: true,
                focused: focused,
                accessibilityLabel: "Search songs and artists",
            ).onFocus {
                if query != "" {
                    query = ""
                    changedText = ""
                }
                focusEntries = focusEntries + 1
            }.onChange { typed ->
                changedText = typed
            }
            Text("Last edit: \(changedText) · focused \(focusEntries) times")
        }
    }
}
```

### Switch

`Switch` binds a Boolean state value to the native toggle on each platform. Its optional
`.onChange` action receives the new value when the user changes the control: SwiftUI uses a
native `Toggle`, and Android uses Material 3 `Switch`. Keep persistence and immediate behavior
changes in this event action; the bound state is updated before the action runs.

| Argument or modifier | Type | Required/default | Behavior |
|---|---|---|---|
| `value` | Mutable `Bool` state | Required | The current toggle value. |
| `label` | `String` expression | Required | Visible and accessible control label. |
| `comment` | `String` | Omitted | Translator context for the label. |
| `.onChange { value -> ... }` | One `Bool` binding and actions | Omitted | Runs after a user changes the toggle, with the new value bound to `value`. |

```nx
app AudioPreferences {
    state skipSilence: Bool = false
    state savedPreference: Bool = false

    body {
        Switch(value: skipSilence, label: "Skip silent passages")
            .onChange { enabled ->
                savedPreference = enabled
            }
        Text("Persisted: \(savedPreference)")
    }
}
```

### Slider

`Slider` binds to mutable `Float64` state and accepts a dynamic range, which lets a
media player bind its position directly to the current track duration. Use
`step: 0.0` for a continuous seek bar. The shared renderer draws the same rounded
track and circular thumb on iOS and Android; track, thumb, and hit-area sizes use
points on iOS and dp on Android. Horizontal drag pauses the player through
`.onEditingChanged`; the callback ends before the final position is sought.

<!-- nexadoc:begin SLIDER_PARAMETERS -->
| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `value` | mutable `Float64` state | Required | Current slider value; updated before `.onChange` actions run |
| `min` | `Float64` expression | Required | Lower end of the value range |
| `max` | `Float64` expression | Required | Upper end of the range; must be greater than `min` when rendered |
| `step` | non-negative `Float64` literal | Required; use `0.0` for continuous values | Quantization interval; a zero step keeps native pointer precision |
| `tint` | color expression (`String`) or `String` state | Platform accent | Filled portion of the track |
| `trackColor` | color expression (`String`) or `String` state | Platform inactive track color | Unfilled portion of the track |
| `thumbColor` | color expression (`String`) or `String` state | White | Circular thumb fill |
| `trackHeight` | positive `Dp` number | 4 dp | Track thickness |
| `thumbSize` | positive `Dp` number | 14 dp | Thumb diameter |
| `hitSlop` | non-negative `Dp` number | 0 dp | Extra vertical pointer hit area above and below the visual control |
<!-- nexadoc:end SLIDER_PARAMETERS -->

<!-- nexadoc:begin SLIDER_EVENTS -->
| Modifier | Type | Default | Behavior |
|---|---|---|---|
| `.onChange` | one `Float64` binding and actions | Omitted | Runs as the bound value changes; its `Float64` parameter is commonly named `value` and tap and drag input are supported |
| `.onEditingChanged` | one `Bool` binding and actions | Omitted | Runs with `true` when horizontal dragging begins and `false` when it ends; its `Bool` parameter is commonly named `editing`; taps do not enter editing state |
<!-- nexadoc:end SLIDER_EVENTS -->

```nx
app EpisodePlayer {
    state position: Float64 = 45.0
    state duration: Float64 = 202.0
    state isSeeking: Bool = false
    state lastSeekRequest: Float64 = 0.0

    body {
        Column(spacing: 10, padding: 16, background: "#0F0F0F") {
            Slider(
                value: position,
                min: 0.0,
                max: duration,
                step: 0.0,
                tint: "#CA2828",
                trackColor: "#424242",
                thumbColor: "#FFFFFF",
                trackHeight: 3.51,
                thumbSize: 14.04,
                hitSlop: 18.72
            )
                .onChange { seconds ->
                    if !isSeeking {
                        lastSeekRequest = seconds
                    }
                }
                .onEditingChanged { editing ->
                    isSeeking = editing
                    if !editing {
                        lastSeekRequest = position
                    }
                }
            Row(fillMaxWidth: true) {
                Text(Time.formatDuration(seconds: position), color: "#9F9F9F")
                Spacer()
                Text(Time.formatDuration(seconds: duration), color: "#9F9F9F")
            }
        }
    }
}
```

```nx
app AccountPreferences {
    state email: String = "alex@example.com"
    state marketingEnabled: Bool = false
    state saved: Bool = false

    body {
        Form {
            Section(title: "Contact") {
                TextInput(value: email, placeholder: "Email address", keyboardType: Email)
                Switch(value: marketingEnabled, label: "Product announcements")
                Button("Save preferences", icon: "save") {
                    saved = true
                }
                if saved {
                    Text("Preferences saved", color: "#15803D")
                }
            }
        }
    }
}
```

Rows rendered by `Form` use a shared 56-point/dp minimum height on iOS and Android. Row content or explicit padding can make a row taller.

### Form and Section dividers

`Form` and `Section` control automatically rendered row separators with `showDividers`. The option defaults to `true` for existing layouts. A Section value overrides its enclosing Form; an omitted Section value inherits it. These options do not hide an explicit `Divider` child. Form and section backgrounds are transparent by default; set `background` when a filled surface is wanted.

| Component | Parameter | Type | Default | Behavior |
|---|---|---|---|---|
| `Form` | `showDividers` | `Bool` | `true` | Shows separators between direct form rows and rows inside sections. `false` hides those separators for the whole form unless a Section opts back in. |
| `Form` | `background` | `Color?` | `nil` (transparent) | Optional form canvas color, accepted as a hex color string or theme color token. |
| `Section` | `showDividers` | `Bool` | Inherits from `Form` (`true` when omitted outside an explicit Form value) | Overrides the enclosing Form setting for rows in this section. |
| `Section` | `background` | `Color?` | `nil` (transparent) | Optional rounded section surface color, accepted as a hex color string or theme color token. |

```nx
app Preferences {
    state remindersEnabled: Bool = true
    state summariesEnabled: Bool = false

    body {
        Form(showDividers: false) {
            Section(title: "Notifications", showDividers: true) {
                Switch(value: remindersEnabled, label: "Reminders")
                Switch(value: summariesEnabled, label: "Daily summary")
            }
        }
    }
}
```

The section above opts back into dividers while other sections remain divider-free.

Event modifiers are reserved for interactions and system triggers. For example, `Pressable` uses `.onTap`, `.onLongPress`, `.onDoubleTap`, `.onDrag`, `.onPinch`, and `.contextMenu` as documented in its syntax-audit entry. Styling remains component arguments.

### ProgressBar

`ProgressBar` renders a linear indicator. Omit `progress` to show the
indeterminate animation; provide a `Float64` value for determinate progress.
Determinate mode uses SwiftUI `ProgressView` on iOS and Material 3
`LinearProgressIndicator` on Android. A custom determinate bar with
`cornerRadius: 0` uses a square track and fill on both platforms and animates
progress changes with a 250 ms linear tween; the animation respects each
platform's reduced-motion setting. Indeterminate mode uses an animated SwiftUI
bar on iOS and the native Material 3 linear indicator on Android. Set
`androidWave` to opt into Android Material 3's native wavy indicator in either
mode; iOS accepts and ignores that Android-specific argument. Determinate values
are clamped to `0...1`; NaN and infinity render as zero.

If an app used the former `android_wave` spelling, rename it to `androidWave`;
the compiler accepts only the camelCase name.

| Argument | Type | Default | Behavior |
|---|---|---|---|
| `progress` | `Float64` | Omitted | Omitted selects the native indeterminate animation. When supplied, values below zero render as zero, values above one render as complete, and non-finite values render as zero. |
| `tint` | `Color` (`String` expression) | Platform native accent | Colors the determinate fill and indeterminate segment. A state string may change the color while the app is running. |
| `trackColor` | `Color` (`String` expression) | Platform native track | Colors the unfilled track in both modes. |
| `thickness` | `Float` (dp/pt) | Platform native thickness | Sets the bar height. Must be finite and non-negative. |
| `cornerRadius` | `Float` (dp/pt) | Platform native shape; half the custom thickness when omitted | Sets the bar and track corner radius in both modes. Must be finite and non-negative. |
| `androidWave` | `Bool` | `false` | Uses Android Material 3's native wavy indicator in determinate or indeterminate mode. iOS accepts the shared argument and ignores it. Android projects using this argument select Material 3 1.5.0-beta01 or later. |
| `accessibilityLabel` | `String` | Omitted | Names the progress indicator for assistive technology. |
| `accessibilityHint` | `String` | Omitted | Adds usage guidance for assistive technology. |
| `accessibilityValue` | `String` | Native progress value | Overrides the announced progress value. |
| `accessibilityRole` | `AccessibilityRole` | Native progress role | Overrides the announced role where supported. |

```nx
app BackupStatus {
    state backupProgress: Float64 = 0.45
    state backupTint: String = "#2563EB"
    state backupTrack: String = "#DBEAFE"

    body {
        Column(spacing: 8, padding: 16) {
            Text("Uploading project archive", fontWeight: Semibold)
            ProgressBar()
            ProgressBar(
                progress: backupProgress,
                tint: backupTint,
                trackColor: backupTrack,
                thickness: 8,
                cornerRadius: 4,
                androidWave: true,
                accessibilityLabel: "Backup upload",
                accessibilityValue: "45 percent"
            )
            Text("45% complete", fontSize: 13, color: "#475569")
        }
    }
}
```

### ProgressRing

`ProgressRing` displays determinate or indeterminate progress with a native SwiftUI or Material 3
indicator. Omitting `progress` starts the native indeterminate animation, matching `ProgressBar()`.
The shared `tint`, `trackColor`, `size`, and `thickness` parameters customize its appearance;
omitted dimensions keep the 40-point/dp diameter and 4-point/dp stroke defaults.

| Argument | Type | Default | Behavior |
|---|---|---|---|
| `progress` | `Float64` | Omitted | Omission selects an indeterminate animation. Supplied progress is clamped to `0...1`; non-finite values render as zero. |
| `tint` | `Color` (`String` expression) | Platform native accent | Colors the active arc. A string state can update the tint at runtime. |
| `trackColor` | `Color` (`String` expression) | Tint at 16% opacity | Colors the unfilled track. |
| `size` | `Float` (dp/pt) | `40` | Sets the indicator diameter. Must be finite and non-negative. |
| `thickness` | `Float` (dp/pt) | `4` | Sets the arc and track stroke width. Must be finite and non-negative. |
| `androidWave` | `Bool` | `false` | Uses Android Material 3's native circular wavy indicator. iOS accepts and ignores it. Android projects using this argument select Material 3 1.5.0-beta01 or later. |
| `accessibilityLabel` | `String` | Omitted | Names the progress indicator for assistive technology. |
| `accessibilityHint` | `String` | Omitted | Adds usage guidance for assistive technology. |
| `accessibilityValue` | `String` | Native progress value | Overrides the announced progress value. |
| `accessibilityRole` | `AccessibilityRole` | Native progress role | Overrides the announced role where supported. |

```nx
app UploadStatus {
    state uploadProgress: Float64 = 0.45
    state uploadTint: String = "#2563EB"

    body {
        Column(spacing: 12, padding: 16) {
            Text("Uploading project archive", fontWeight: Semibold)
            ProgressRing(
                progress: uploadProgress,
                tint: uploadTint,
                trackColor: "#DBEAFE",
                size: 48,
                thickness: 5,
                accessibilityLabel: "Project archive upload",
                accessibilityValue: "45 percent"
            )
            ProgressRing(androidWave: true)
        }
    }
}
```

## Repeated data

`FastList` is the virtualized list component. Pass exactly one data source
(`collection`, `count:`, or `sections:`) and an explicit binding block.

Add a row key when collection rows can move, be inserted, or be removed.
Without `key`, row identity follows position and the compiler warns. Use
`key: .id` for records and `key: .self` for values with unique scalar identity.

The compiler checks the number and type of row bindings. When `rowHeight` is
set, row content is vertically centered within that height on both platforms.
When omitted, rows grow to fit their content and have a shared minimum height
of 52 pt/dp.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| Data source | `Array<T>`, `count: Int32`, or `sections: Array<Array<T>>` | Exactly one source | Positional arrays and named `count:` or `sections:` sources are supported. |
| `key` | Statically known scalar item property | Omitted | Recommended for collection rows whose order or membership can change; without it, identity follows the row position. Use `.self` for unique scalar rows or a stable item property such as `.id`. |
| `axis` | `Vertical`, `Horizontal`, `Grid(Int32)`, `AutoGrid(Int32)`, or `Masonry(Int32)` | `Vertical` | `Grid(columns)` fixes the column count. `AutoGrid(minCellWidth)` fits columns to available width. `Masonry(columns)` places variable-height items into the currently shortest column. Both numeric arguments must be positive integer literals. Masonry is a flat vertical list and cannot use `rowHeight`, `reverseLayout`, `pageSnap`, sticky headers, reordering, or swipe actions. |
| `rowHeight` | `Float64` | Content-sized, with a 52 pt/dp minimum | Optional fixed row height; for horizontal lists it sets item height while item width comes from row content, and for grids it is the cell height. Row content is vertically centered within the fixed height. Without it, content can grow rows beyond the shared minimum. Must be greater than zero. |
| `spacing` | `Float64` | `0` | Gap between adjacent items. Applies along the list axis, or to both rows and columns for `Grid`, `AutoGrid`, and `Masonry`; `pageSnap` requires zero spacing. |
| `scrollPosition` | Mutable `Int32` state | Omitted | Binds and reports the leading unobscured data-row index for flat lists; rows behind a sticky header do not count as visible. |
| `scrollEnabled` | `Bool` expression | Platform default | Enables or disables user-driven scrolling for flat, sectioned, and adaptive-grid lists; supports reactive state. |
| `reverseLayout` | `Bool` | `false` | Reverses a flat vertical list; cannot be combined with sections, sticky headers, or page snapping. |
| `pageSnap` | `Bool` | `false` | Pages a flat vertical list by viewport height; omit `rowHeight`. |
| `showDividers` | `Bool` | `true` | Shows automatically rendered separators between rows in vertical flat and sectioned lists. Set to `false` to hide them. Horizontal lists, grids, and page-snapping lists do not draw row separators. |
| `ref` | `ComponentRef<FastList>` | Omitted | Attaches a typed imperative handle for the current list instance. Calls made while it is detached return the typed not-attached failure. |
| `accessibilityLabel` | `String` | Omitted | Accessible label for the list. |

| Modifier | Arguments and body | Behavior |
|---|---|---|
| `.onEndReached { ... }` | Actions | Runs once when the final data row becomes visible; it can run again after the item count changes. |
| `.onScroll { ... }` | Actions | Runs when the leading unobscured data row changes, not for every pixel; the initial viewport does not emit it. |
| `.onScrollBeginDrag { ... }` | Actions | Runs once when the user begins dragging a forward vertical flat list. Programmatic scrolling does not emit it; sectioned, reversed, horizontal, and page-snapping lists are not supported. |
| `.onMove(enabled:) { from, to -> ... }` | Optional `Bool` (`true`); event actions | Reorders an array-backed vertical list; the platform list surface is selected automatically. Android starts reordering with a long-press and drag. Reordering and swipe actions are separate interaction modes on both platforms; turn reordering off to reveal swipe actions. |
| `.stickyHeader { ... }` | Nodes | Adds a header to a flat vertical list. |
| `.sectionHeader { section -> ... }` | Nodes | Renders a header for each `sections:` group. |
| `.swipeActions { ... }` | Nodes | Adds one trailing swipe action; the platform list surface is selected automatically. |

Use `AutoGrid` for responsive media galleries so the same source adapts to phones, tablets, and resizable windows:

```nx
FastList(videos, key: .id, axis: AutoGrid(168), rowHeight: 240) { video, index in
    Image(url: video.coverUrl, description: "Video thumbnail", scale: Fill, maxHeight: 240)
}
```

Use `Masonry` when cards have different heights:

```nx
FastList(count: 40, axis: Masonry(2), spacing: 12) { index in
    Column(padding: 12, background: "#E8F0FF", cornerRadius: 12) {
        Text("Card \(index)", fontWeight: Semibold)
        if index % 3 == 0 {
            Text("Additional details make this card taller than its neighbors.")
        }
    }
}
```

Use `ComponentRef<FastList>()` when an action needs to scroll a list. The handle
stays the same across updates and is attached only while that list instance is
mounted:

```nx
struct OrderSummary {
    id: Int32,
    itemName: String,
    status: String,
}

app OrderQueue {
    let queueRef = ComponentRef<FastList>()
    state orders: Array<OrderSummary> = [
        OrderSummary(701, "Travel mug", "Packing"),
        OrderSummary(702, "Notebook set", "Ready")
    ]

    body {
        Column(spacing: 8) {
            Button("Show the last order") {
                queueRef.scrollToIndex(
                    index: orders.count - 1,
                    animated: true,
                    viewPosition: 0.5
                )
            }
            Button("Return to the first order") {
                queueRef.scrollToTop()
            }
            FastList(orders, key: .id, ref: queueRef) { order, index in
                Row(spacing: 8) {
                    Text("\(index + 1). \(order.itemName)")
                    Spacer()
                    Text(order.status)
                }
            }
        }
    }
}
```

`scrollToIndex` takes an optional `animated: Bool = true` and
`viewPosition: Float64 = 0.0`. Position `0.0` places the row at the viewport
start, `0.5` centers it, and `1.0` places it at the end. Values are clamped to
that range. `scrollToTop`, `scrollToEnd`, and `scrollToOffset(offset:)` also
take `animated: Bool = true`.

Each method returns a `Result`. It succeeds while the list is mounted; if the
list is not attached, it returns `ComponentRefError.notAttached`.

```nx
queueRef.scrollToIndex(index: 20) // animated, at the viewport start
queueRef.scrollToIndex(index: 20, viewPosition: 0.5) // center the row
queueRef.scrollToIndex(index: 20, animated: false, viewPosition: 1.0) // jump to the end
queueRef.scrollToEnd(animated: false)
queueRef.scrollToOffset(offset: 240.0, animated: true)
```

```nx
struct OrderLine {
    id: Int32,
    itemName: String,
    quantity: Int32,
    status: String,
}

app OrderHistory {
    state orders: Array<OrderLine> = [
        OrderLine(101, "Travel mug", 2, "Shipped"),
        OrderLine(102, "Notebook set", 1, "Processing")
    ]

    body {
        FastList(orders, key: .id) { order, index in
            Row(spacing: 12, padding: 12, background: "#F8FAFC", cornerRadius: 8) {
                Text("\(index + 1). \(order.itemName)", fontWeight: Semibold)
                Text("Qty \(order.quantity)", color: "#475569")
                Spacer()
                Text(order.status, fontSize: 12, color: "#1D4ED8")
            }
        }
    }
}
```

## Reusable components and typed refs

A reusable `.nx` component can expose a typed API by declaring methods inside
its component body. Create a matching `ComponentRef` and pass it with `ref:`.
Methods can be synchronous or `async`. A method that propagates a plugin error
declares that error type after `throws`; its ref call returns a typed `Result`.
Calls made while the component is unmounted report `notAttached`.

Async and failure support follows the component's declared API:

| Ref target | Async methods | Typed method failures | Notes |
|---|---|---|---|
| Built-in `FastList` | Its current scroll commands are synchronous | Only `notAttached` | Scrolling is an immediate command; the API has no other failure case. |
| Reusable `.nx` component | Yes | Yes, with `Result<T, AppError>` using a local enum, or `throws Namespace.ErrorType` for a plugin error | The method body can await async work. A returned `Result` failure becomes the ref call's typed failure. |
| Native plugin component | Yes | Yes, from the component's `.nxid` method signature | The Swift and Kotlin implementations attach the method to the component's ref. |

So the sync-only behavior applies to the built-in `FastList` scroll commands;
reusable and native plugin component methods support async calls and typed
failures.

Use an app enum when the reusable component owns its failure cases. The ref
call keeps those cases in its `Result` and adds `notAttached` to the same typed
failure channel:

```nx
enum ProfileLoadError { offline, notCached }

component ProfileStatus() {
    async fn loadName() -> Result<String, ProfileLoadError> {
        return Err(ProfileLoadError.offline)
    }

    body { Text("Profile") }
}

app ProfileScreen {
    let profileRef = ComponentRef<ProfileStatus>()

    body {
        ProfileStatus(ref: profileRef)
        OnAppear async {
            let result: Result<String, ComponentRefError<ProfileLoadError>> =
                await profileRef.loadName()
        }
    }
}
```

```nx
component QuantityStepper() {
    state quantity: Int32 = 1

    fn add(amount: Int32) -> Void {
        quantity = quantity + amount
    }

    body {
        Text("Quantity: \(quantity)")
    }
}

app Cart {
    let stepperRef = ComponentRef<QuantityStepper>()

    body {
        Column {
            QuantityStepper(ref: stepperRef)
            Button("Add two") {
                stepperRef.add(amount: 2)
            }
        }
    }
}
```

The handle keeps the same identity across updates. A component without declared
ref methods cannot be used as `ComponentRef<T>` or receive `ref:`. Each method
uses the types in its declaration and returns
`Result<Return, ComponentRefError<Failure>>`. A normal non-throwing method uses
`Void` for `Failure`. A method returning `Result<Value, AppError>` exposes
`Result<Value, ComponentRefError<AppError>>`; its success or failure is passed
through directly. A `throws Namespace.ErrorType` method uses that plugin error
as `Failure`. Choose one failure style for each method instead of combining
`throws` and `Result`.

```nx
plugin "plugins/video" as Video

component PlayerControls(player: Video.Player) {
    async fn currentSource() -> String throws Video.PlayerFailure {
        return await player.currentSource()
    }

    fn seek(position: Float64) -> Void throws Video.PlayerFailure {
        player.seek(position: position)
    }

    body { Text("Playback controls") }
}

app LessonPlayback {
    let player = Video.Player("lesson-player")
    let controlsRef = ComponentRef<PlayerControls>()

    body {
        PlayerControls(player: player, ref: controlsRef)
        OnAppear async {
            let source: Result<String, ComponentRefError<Video.PlayerFailure>> =
                await controlsRef.currentSource()
        }
        Button("Seek to the preview") {
            let result: Result<Void, ComponentRefError<Video.PlayerFailure>> =
                controlsRef.seek(position: 30.0)
        }
    }
}
```

`await` is required when calling an async ref method. The result keeps the
plugin's declared error type, so callers can handle not-attached and plugin
failures without a platform-specific exception API.

## Navigation and modal content

Declare destinations as `screen` blocks and select the root with `NavigationStack(root: ...)`. A `NavigationLink` is a declarative link in the view tree; `Navigation.push(...)` and `Navigation.back()` are imperative actions that work in foreground action blocks, including button, lifecycle, list, and plugin event actions. `NavigationBack` draws a back control; `Navigation.back()` only performs the pop. App links configured with `deepLinks` route incoming URLs by screen slug and scalar arguments in the path or named query parameters; use `Link(url: ...)` to open an external URL.

Pass route arguments positionally in the destination's declaration order. The
compiler checks the destination and each argument type. A screen declared in
the same `.nx` file is available directly. A screen declared in another source
file is available after importing that file relative to the importing source.

`NavigationStack` uses the platform's native inline title by default: SwiftUI's centered inline title on iOS and Material 3's centered top app bar on Android. Set `titleAlignment: Leading` to place the title beside Back; set `Center` to keep the centered layout. Set `titleMode: Large` for SwiftUI's large title and Material 3's expanding large top app bar. Set `showNavigationBar: false` when screens draw a custom header and back control; pushed screens then use the full content area without an extra native bar.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `root` | Declared screen | Required | Screen shown at the root of the stack. |
| `showNavigationBar` | `Bool` | `true` | Shows native navigation chrome on pushed screens. Set `false` when each screen supplies its own header and back control. |
| `titleMode` | `Inline` or `Large` | `Inline` | Inline keeps the title in the back-control row. Large expands and collapses with scroll on both platforms. |
| `titleAlignment` | `Automatic`, `Leading`, or `Center` | `Automatic` | Inline title placement. Automatic uses each platform's native centered title. Leading is an explicit layout beside the back button. Large titles keep their native leading alignment. |

```nx
app SupportCenter {
    screen HelpHome {
        Column(spacing: 12, padding: 16) {
            Text("Support", fontSize: 24, fontWeight: Bold)
            NavigationLink(destination: ContactSupport) {
                Text("Contact support")
            }
            Button("Open order details") {
                Navigation.push(OrderDetail(101))
            }
        }
    }

    screen ContactSupport {
        Column(spacing: 12, padding: 16) {
            Text("We're here to help")
            Link(url: "https://support.example.com") {
                Text("Open help center")
            }
            NavigationBack(label: "Back to support")
            Button("Close this screen") {
                Navigation.back()
            }
        }
    }

    screen OrderDetail(orderID: Int32) {
        Column(spacing: 8, padding: 16) {
            Text("Order \(orderID)", fontSize: 22, fontWeight: Bold)
            Button("Back to support") { Navigation.back() }
        }
    }

    body {
        NavigationStack(root: HelpHome)
    }
}
```

For a destination stored in `screens/OrderDetail.nx`, add this at the top of
`App.nx`: `import "screens/OrderDetail.nx"`. Then push it with
`Navigation.push(OrderDetail(orderID))`, using a value whose type matches the
screen parameter. No import is needed for a screen in the current file.

### BottomSheet

`BottomSheet` uses SwiftUI `.sheet` on iOS and a Compose sheet on Android. Use
fractional `detents` for fixed resting heights, or set `contentSized: true` to
fit the sheet to its measured child content. These modes are exclusive. Drag a
fractional sheet to change its height; a content-sized sheet follows its content
height and can still be dismissed by dragging. The child block uses Nexa
`Column` defaults on both platforms: children are centered horizontally and
start at the top of the available content area immediately after the native drag
indicator. The sheet adds no content inset by default. Set `contentPadding` to
add a uniform inset; padding on child layouts adds to it.
Set `cornerRadius` to override the top corner radius on both platforms. iOS
uses SwiftUI's presentation radius on iOS 16.4+ and the UIKit sheet controller
on earlier supported iOS versions. Omit it to preserve each platform's native
sheet shape.
A `NavigationStack` may be the sheet's direct child; its pushed destinations stay
inside the sheet. Back returns to the sheet's previous screen and dismisses the
sheet only when its root is active.

<!-- nexadoc:begin BOTTOM_SHEET_PARAMETERS -->
| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `ref` | `ComponentRef<BottomSheet>` | Required | Controls presentation through the typed `present()` and `dismiss()` methods |
| `contentSized` | `Bool` | `false` | Fits the sheet to its laid-out content; cannot be combined with `detents` or `initialDetent` |
| `contentPadding` | non-negative `Dp` number | 0 dp | Uniform inset around sheet content; explicit child padding adds to it |
| `cornerRadius` | non-negative `Dp` number | Platform sheet shape | Sets the top radius on Android and on all supported iOS versions through SwiftUI or UIKit |
| `detents` | `Array<Float64>` fractions in `(0, 1]` | `[0.5, 1.0]` | Available resting heights as fractions of screen height; at least one unique ascending value is required |
| `initialDetent` | `Float64` fraction in `(0, 1]` | First entry in `detents` | Initial resting height; must match one configured detent |
| `showDragIndicator` | `Bool` | `true` | Shows or hides the platform-native drag indicator |
| `allowsDismissal` | `Bool` | `true` | Allows drag dismissal and outside-tap dismissal for modal sheets; Android Back also obeys `shouldDismissOnBackPress`; `ref.dismiss()` can always close the sheet |
| `allowsBackgroundInteraction` | `Bool` | `false` | Removes the modal barrier and lets taps reach the screen behind the sheet (iOS 16.4+) |
| `shouldDismissOnBackPress` | `Bool` | `true` | On Android, Back dismisses when `allowsDismissal` is also true; otherwise Back is consumed while the sheet stays open |
| `backgroundColor` | color string or `String` state | Platform sheet surface | Overrides the sheet surface color; use `#00000000` for a transparent surface |
| `scrimColor` | color string or `String` state | Platform modal scrim | Overrides the Android modal dim layer; a transparent color does not remove the modal barrier |
<!-- nexadoc:end BOTTOM_SHEET_PARAMETERS -->

A single ascending fractional list defines every resting height. For example,
`detents: [0.25, 0.5, 0.85]` allows 25%, 50%, and 85% heights. `initialDetent`
must match one entry in that list. `allowsDismissal: false` disables drag-to-dismiss;
`ref.dismiss()` can still close the sheet. `shouldDismissOnBackPress:
false` consumes Android Back while keeping the sheet open. `allowsBackgroundInteraction:
true` makes the sheet non-modal: taps outside its surface reach the screen behind
it and do not close the sheet. On iOS 16.4 or later this uses SwiftUI background
interaction; on Android the sheet is presented as a native Compose overlay rather
than a modal dialog.

Create one `ComponentRef<BottomSheet>` for each sheet. Call `present()` and
`dismiss()` from any action context, including buttons, lifecycle actions, and
custom component methods. These calls return
`Result<Void, ComponentRefError<Void>>`; a call made while the sheet is not
attached returns `notAttached`. A native drag or Back dismissal does not require
an extra state update.

To stack a sub-sheet above an open sheet, declare the second `BottomSheet`
inside the first sheet's child content and give it its own reference. Calling
`present()` on the child keeps the parent open underneath. Calling `dismiss()`
on the child reveals the same parent sheet and preserves its state. This nested declaration uses SwiftUI's
native sheet presenter on iOS and a second Material 3 modal sheet on Android.

```nx
let taskSheetRef = ComponentRef<BottomSheet>()
let taskActionsSheetRef = ComponentRef<BottomSheet>()

BottomSheet(ref: taskSheetRef, detents: [0.5, 1.0]) {
    Column(spacing: 12, padding: 16) {
        Text("Task details")
        Button("More actions") { taskActionsSheetRef.present() }
        BottomSheet(ref: taskActionsSheetRef, detents: [0.5]) {
            Column(spacing: 12, padding: 16) {
                Text("Task actions")
                Button("Return to task") { taskActionsSheetRef.dismiss() }
            }
        }
    }
}
```

For an edit or details sheet, compose a header from ordinary Nexa layout and
buttons. A Save action can update state while leaving the sheet open; a close
button calls `dismiss()` so the platform dismissal animation runs.

```nx
screen DeliveryAddressEditor {
    let editorSheetRef = ComponentRef<BottomSheet>()
    state addressSaved: Bool = false

    body {
        Column(spacing: 12, padding: 16) {
            Button("Edit delivery address") {
                editorSheetRef.present()
            }
            BottomSheet(ref: editorSheetRef, detents: [0.5, 1.0]) {
                Column(spacing: 12, padding: 20, alignment: Start) {
                    Row(spacing: 12) {
                        Text("Edit delivery address", fontSize: 20, fontWeight: Semibold)
                        Spacer()
                        Button("Save", icon: "save", style: Borderless) {
                            addressSaved = true
                        }
                        Button("Close", icon: "close", style: Plain) {
                            editorSheetRef.dismiss()
                        }
                    }
                    Text(addressSaved ? "Address saved" : "Unsaved changes")
                }
            }
        }
    }
}
```

Detents are fractions of the available height on both platforms. Custom values
use the same sheet surface, drag indicator, dismissal, and keyboard handling as
the default half and full heights.

`backgroundColor` colors the sheet surface; `scrimColor` colors the modal layer
behind it. Leave `backgroundColor` omitted to keep the native sheet surface.
`allowsBackgroundInteraction: true` removes the modal barrier completely so
underlying controls remain tappable without dismissing the sheet. Set
`shouldDismissOnBackPress: false` to keep Android Back from dismissing it. Android
applies custom scrim colors in modal mode. SwiftUI owns its native modal scrim
and has no public custom tint; use `allowsBackgroundInteraction` to remove its
dimming and allow taps through.

```nx
screen TaskEditor {
    let editorSheetRef = ComponentRef<BottomSheet>()
    state editorSurface: String = "#F1F5F9"
    state note: String = ""
    state didDismissEditor: Bool = false

    body {
        Column(spacing: 12, padding: 16) {
            Button("Edit task") {
                editorSheetRef.present()
            }
            BottomSheet(
                ref: editorSheetRef,
                detents: [0.3, 0.55, 0.9],
                initialDetent: 0.55,
                showDragIndicator: true,
                allowsDismissal: true,
                backgroundColor: editorSurface
            ) {
                Column(spacing: 12, padding: 20) {
                    Text("Edit task", fontSize: 20, fontWeight: Semibold)
                    Text("Update the task details and add an optional note.")
                    TextInput(value: note, placeholder: "Note", accessibilityLabel: "Task note")
                    Button("Done") {
                        editorSheetRef.dismiss()
                    }
                }
            }.onDismiss {
                didDismissEditor = true
            }
            Text(didDismissEditor ? "Editor dismissed" : "Editor is ready")
        }
    }
}
```

## Complete component reference

The compiler schema is the source of truth. The [syntax audit](syntax-audit.md) enumerates every current component and its accepted argument names, required values, child blocks, and event modifiers. Read it alongside the [language guide](language-guide.md); do not infer `.nx` syntax from SwiftUI or Compose APIs.

### Image

`Image` displays one asset, HTTPS URL, or local file. URL and file images render
blank while loading and after a failed request by default on both platforms.
Pass `showLoadingIndicator: true` to opt into a small native loading indicator.
An explicit URL `placeholder` is shown while loading and after failure; it takes
precedence over the loading indicator.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `asset` | `String` | Exactly one of `asset`, `url`, or `file` | Android drawable and iOS asset catalog name. |
| `url` | `String` or `Signal<String>` | Exactly one of `asset`, `url`, or `file` | Absolute HTTPS image URL. |
| `file` | `String` or `Signal<String>` | Exactly one of `asset`, `url`, or `file` | Local `file://` URL. |
| `description` | `String` | Required | Accessible image description. |
| `scale` | `Fit` or `Fill` | `Fit` | Preserve the full image or crop it to fill. |
| `placeholder` | `String` | Omitted | URL-only drawable/asset name shown during loading and on failure. |
| `showLoadingIndicator` | `Bool` | `false` | URL/file-only; shows a small native indicator while loading when no placeholder is supplied. |
| `maxHeight` | `Float64` | Omitted | Optional maximum rendered height; must be greater than 0 and at most 4096. |
| `fillMaxSize` | `Bool` | `false` | Expands the image to the available width and height of its parent, cropping when `scale: Fill`; useful for media backgrounds and full-page posters. |
| `.sharedElement(id:)` | `String` | Omitted | Joins the image to a matching shared-element transition. |

```nx
Image(
    url: "https://cdn.example.com/products/summer-jacket.png",
    description: "Navy summer jacket",
    placeholder: "product_image",
    maxHeight: 260
)
```

### Text

`Text` renders with SwiftUI on iOS and Compose Material 3 `Text` on Android. Semantic font styles use shared point/`sp` sizes and weights; SwiftUI applies the matching text style for Dynamic Type, and Android follows the system font scale. If both `fontStyle` and `fontSize` are supplied, the explicit size replaces the style's size while keeping its default weight. An explicit `fontWeight` takes precedence over that semantic weight.

| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `value` | `String` expression | Required | Text content. Newlines are preserved on both targets. |
| `comment` | `String` | None | Translator context for localized text. |
| `color` | Color expression | Platform text color | Sets the text foreground color. |
| `alignment` | `Leading`, `Center`, `Trailing` | Inherited column text alignment | Aligns lines within the text's measured width. |
| `fontSize` | Dimension | Platform body size | Uses points on iOS and `sp` on Android. Overrides the semantic style's size. |
| `fontStyle` | `LargeTitle`, `Title`, `Title2`, `Title3`, `Headline`, `Subheadline`, `Body`, `Callout`, `Footnote`, `Caption`, `Caption2` | None | Applies the matching semantic size and default weight; the style scales with accessibility text settings. |
| `fontDesign` | `Default`, `Serif`, `Rounded`, `Monospaced` | Platform default design | Selects the system font design. Ignored when `fontFamily` is supplied. |
| `fontFamily` | Static registered font face `String` or `Theme` font-family token | `Theme.Default` when declared, otherwise platform default | Selects a registered PostScript font face from `assets/fonts/`. The name must contain letters, numbers, spaces, `_`, or `-`, and begin with a letter. |
| `fontWeight` | `Normal`, `Medium`, `Semibold`, `Bold` | Semantic style weight, otherwise `Normal` | Explicit weight overrides the weight associated with `fontStyle`. |
| `padding` | Dimension | None | Adds equal padding around the text. |
| `opacity` | `Float` from `0` to `1` | `1` | Applies alpha to the rendered text. |
| `scale` | `Float64` expression | `1` | Scales the text view uniformly; a state expression can be changed inside `withAnimation` for native transform animation. |
| `rotation` | `Float` degrees | `0` | Rotates the text view. |
| `shadow` | `Shadow(radius, x, y, color)` | None | Draws a shadow behind glyphs using the blur radius and offsets. |
| `blur` | Dimension | None | Blurs the rendered text. |
| `clip` | `Rounded(radius)` | None | Clips the text view to a rounded rectangle. |
| `zIndex` | `Int32` | `0` | Controls drawing order relative to overlapping siblings. |
| `lineLimit` | Positive integer | Unlimited | Limits visible lines and truncates overflow with an ellipsis. |
| `lineHeight` | Dimension | Native line height | Sets the total line box height. |
| `letterSpacing` | Signed dimension | `0` | Adds or removes tracking between characters; negative values bring glyphs closer together. |
| `strikethrough` | `Bool` | `false` | Draws a line through the text. |
| `italic` | `Bool` | `false` | Uses the italic font face. |
| `underline` | `Bool` | `false` | Underlines the text. |
| `selectable` | `Bool` | `false` | Enables native text selection and copy actions. |
| `accessibilityLabel` | `String` | Text value | Overrides the accessibility label. |
| `accessibilityHint` | `String` | None | Supplies an accessibility hint. |
| `accessibilityValue` | `String` | None | Supplies an accessibility value. |
| `accessibilityRole` | Accessibility role | Text role | Overrides the accessibility role. |

Declare an app-wide default face once with `theme { fontFamily Default: "Gilroy-Regular" }`; an explicit `Text(fontFamily:)` overrides it. Place `.ttf` or `.otf` files directly in `assets/fonts/`. Name weighted faces `<family>_regular`, `<family>_medium`, or `<family>_semibold`; Nexa registers their PostScript names (for example `Gilroy-Regular`) on iOS and the matching static resource IDs on Android. A single-face font uses its PostScript name, such as `Mazicon`. For a face ending in `-Regular` or `_regular`, Nexa selects the matching Medium or Semibold face from `fontWeight`; Bold uses Semibold when the family has no separate bold file.

All numeric dimensions use the app's shared logical units: points on iOS and `dp`/`sp` on Android as appropriate. `fontDesign` (`Default`, `Serif`, `Rounded`, or `Monospaced`) selects the system font family while keeping the design choice aligned across targets.

Use `italic`, `underline`, and `strikethrough` for inline emphasis. They are independent Boolean parameters and can be combined; the same combinations work in app text and widgets on iOS and Android.

```nx
app TaskTypographyPreview {
    body {
        Column(spacing: 3) {
            Text("Renew team insurance", fontStyle: Subheadline)
            Text("Tomorrow · 09:00", fontStyle: Caption, color: "#8E8E93")
        }
    }
}
```

### TextReveal

`TextReveal` is the reusable timed-text counterpart to `Text`. It measures the same wrapped text used for drawing, divides timed segments across visual lines, and reveals the active segment with native line clipping. AOT and DevRuntime call the same Compose and TextKit primitives. `segments` is a statically typed array of any app-declared struct that contains the timing fields below; no plugin or Nexa-specific lyric model is required.

```nx
struct TranscriptCue {
    text: String,
    startTimeSeconds: Float64,
    endTimeSeconds: Float64,
    hasTrailingSpace: Bool?,
}

app AudioTranscript {
    state cues: Array<TranscriptCue> = [
        TranscriptCue(text: "Turn", startTimeSeconds: 4.2, endTimeSeconds: 4.5, hasTrailingSpace: true),
        TranscriptCue(text: "left", startTimeSeconds: 4.6, endTimeSeconds: 5.0, hasTrailingSpace: false),
    ]
    state position: Float64 = 4.2
    state active: Bool = true
    state playing: Bool = true
    state revealEnabled: Bool = true
    state seekRevision: Int32 = 0

    body {
        TextReveal(
            "Turn left after the bridge",
            segments: cues,
            position: position,
            active: active,
            playing: playing,
            enabled: revealEnabled,
            seekRevision: seekRevision,
            fontSize: 20,
            fontWeight: Semibold,
            color: "#FFFFFF",
            lineHeight: 27,
        )
    }
}
```

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `value` | `String` expression | Required | Complete text shown by the component. It should contain the same words and spaces as `segments` in the same order. |
| `segments` | `Array<Struct>` | Required | Each element must provide `text: String`, `startTimeSeconds: Float64`, and `endTimeSeconds: Float64`. Optional `hasTrailingSpace: Bool?` defaults to `true`; it contributes one text unit when Nexa assigns segments to wrapped lines. |
| `position` | `Float64` | Required | Current playback or timeline position in seconds. |
| `active` | `Bool` | Required | Enables the highlight overlay for this text row. Inactive rows keep only their dimmed base text. |
| `playing` | `Bool` | Required | Controls timeline smoothing. When false, the current position is applied immediately and no animation clock runs. |
| `enabled` | `Bool` | Required | Enables timed word reveal. When false, an active row is fully highlighted. |
| `seekRevision` | Integer | Required | Increment after an explicit seek. Nexa snaps to the earlier of `position` and the first segment start time. |
| `revealOpacity` | `Float64` literal from `0` to `1` | `0.38` | Opacity of the dim base text. The reveal overlay uses `color` at full opacity. |
| `leadTime` | Non-negative numeric literal in seconds | `0.3` | Initial lead after a row becomes active during playback. |
| `targetOffset` | Non-negative numeric literal in seconds | `0.56` | Position offset targeted when smoothing each playback sample. |
| `smoothingDuration` | Non-negative numeric literal in seconds | `0.25` | Linear native animation duration used to settle to a playback sample. |
| `comment` | `String` | None | Translator context for static text, following `Text` localization rules. |
| `color` | Color expression | Platform text color | Foreground color for the revealed text. |
| `alignment` | `Leading`, `Center`, `Trailing` | Inherited column text alignment | Aligns each wrapped line. Leading and trailing follow layout direction. |
| `fontSize` | Dimension | Platform body size | Uses points on iOS and `sp` on Android. |
| `fontStyle` | Semantic text style | None | Uses the same semantic style names and default weights as `Text`. |
| `fontDesign` | `Default`, `Serif`, `Rounded`, `Monospaced` | Platform default | Selects the system font design when no registered `fontFamily` is supplied. |
| `fontFamily` | Registered font face or theme font-family token | Theme default or platform default | Selects a font registered from `assets/fonts/`. |
| `fontWeight` | `Normal`, `Medium`, `Semibold`, `Bold` | Semantic style weight, otherwise `Normal` | Selects the requested native font weight. |
| `padding` | Dimension | None | Adds equal padding around the measured text. |
| `opacity` | `Float` from `0` to `1` | `1` | Applies alpha to both base text and reveal overlay. |
| `scale` | `Float64` expression | `1` | Scales the complete text component. |
| `scaleY` | `Float64` expression | `scale` | Overrides vertical scale independently. |
| `rotation` | `Float` degrees | `0` | Rotates the complete text component. |
| `shadow` | `Shadow(radius, x, y, color)` | None | Draws the same glyph shadow in both text layers. |
| `blur` | Dimension | None | Blurs the complete text component. |
| `clip` | `Rounded(radius)` | None | Clips the component to a rounded rectangle. |
| `zIndex` | `Int32` | `0` | Controls drawing order among overlapping siblings. |
| `lineLimit` | Positive integer | Unlimited | Limits visible lines and truncates overflow with an ellipsis. |
| `lineHeight` | Dimension | Native line height | Sets the total line box height. |
| `letterSpacing` | Signed dimension | `0` | Adds or removes tracking between glyphs. |
| `strikethrough` | `Bool` | `false` | Draws a line through both text layers. |
| `italic` | `Bool` | `false` | Uses the italic face where available. |
| `underline` | `Bool` | `false` | Underlines both text layers. |

The component measures line wrapping with the selected font and width, then caches line and segment timing ranges. Android reads the `TextLayoutResult`; iOS lays out with TextKit. A native frame clock runs only for an active, enabled row while playing and settling. The clock stops after smoothing completes, on pause, when the row leaves the screen, or when timed reveal is disabled. RTL rows reveal from the logical leading edge.

### ContentUnavailable

`ContentUnavailable` presents an empty or unavailable state using the platform's native visual treatment: SwiftUI `ContentUnavailableView` on iOS and a centered Material 3 empty state on Android. The supporting description is optional; omitting it removes the detail text and its spacing. Optional color parameters customize each visual part independently; omitted colors keep the native platform defaults.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `title` | `String` expression | Required | Main empty-state heading. |
| `icon` | Nexa shared icon or `assets.icons` alias (`String`) | Required | Icon name from Nexa's shared catalog or a registered custom glyph alias; unknown names are compile errors. |
| `description` | `String` expression | Omitted | Optional supporting copy below the title. |
| `titleColor` | Color expression (`String`) | Native default | Optional title color. Accepts a hex color, theme color, or dynamic string expression. |
| `descriptionColor` | Color expression (`String`) | Native default | Optional supporting description color. |
| `iconTint` | Color expression (`String`) | Native default | Optional icon tint. |
| `background` | Color expression (`String`) | Transparent/native default | Optional color behind the whole empty state. |
| `comment` | `String` | None | Translator context applied to the title and description. |
| `accessibilityLabel` | `String` | Native default | Overrides the component's accessibility label. |
| `accessibilityHint` | `String` | None | Adds an accessibility hint. |
| `accessibilityValue` | `String` | None | Adds an accessibility value. |
| `accessibilityRole` | Accessibility role | Native default | Overrides the accessibility role. |

```nx
ContentUnavailable(title: "No reminders", icon: "calendar")
ContentUnavailable(
    title: "No saved tasks",
    icon: "inbox",
    description: "Saved tasks will appear here.",
    titleColor: "#111111",
    descriptionColor: "#666666",
    iconTint: "#007AFF",
    background: "#F2F2F7"
)
```

### Selection and date controls

For shared springs, timed easing, and animated conditional content, see the
[animation guide](animations.md). iOS uses one native `UISegmentedControl` for
both default and customized cases, so selection, accessibility, and native
interaction share one implementation. Omitted style parameters retain the
system appearance, including iOS 26 glass. Explicit `tint`, `background`, and
text-color parameters map to UIKit's native selected-segment, control-background, and
title-attribute properties. UIKit keeps the native selected capsule and applies its
iOS 26 track material when drawing a custom background, so the displayed track may
blend with the system material. Selection, touch input, and accessibility remain in
the same native control.
Android uses Nexa's existing custom Compose renderer with Material 3 colors and
motion for both default and styled controls. A selected-pill `tint`, track
background, label colors or typography, size, radius, and borders use Nexa's
native control styling APIs on iOS. Android keeps its existing Material 3
implementation.

`SegmentedControl(items:, selected:, animation:)` binds a string selection to
equal-width choices. `animation` is optional: when omitted, iOS keeps its native
segmented-control motion and Android uses the Material 3 segmented button's native
selection feedback. Pass a shared `Spring`, `Tween`, or easing preset to customize
the animated indicator on both platforms. Android uses Compose Animation Core with
Material 3 Expressive spring tokens (`dampingRatio = 0.8`, `stiffness = 380`) so the
custom animated renderer remains available to API 23 apps. Tweens accept a start
delay and custom cubic Bézier curve; the indicator uses Compose's native spring or
finite animation spec, without a per-frame interpreter.

<!-- nexadoc:begin SEGMENTED_CONTROL_PARAMETERS -->
| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `items` | `Array<String>` | Required | Labels and values for the equal-width choices |
| `selected` | mutable `String` state | Required | Current choice; updated when a segment is tapped |
| `animation` | `Spring`, `Tween`, or easing preset | Platform default | Selection movement timing |
| `accessibilityLabel` | `String` | Automatic | Optional VoiceOver/TalkBack control name |
| `comment` | `String` | `null` | Translator note for literal choice labels |
| `tint` | color string or `String` state | Platform default | Fill color of the selected segment |
| `background` | color string or `String` state | Platform default | Segment track fill color |
| `selectedTextColor` | color string or `String` state | Platform default | Label color for the selected segment |
| `textColor` | color string or `String` state | Platform default | Label color for unselected segments |
| `fontSize` | non-negative `Sp` number | Platform typography | Label text size; uses the platform unit (points on iOS, sp on Android) |
| `fontWeight` | `Normal`, `Medium`, `Semibold`, or `Bold` | Platform typography | Label text weight |
| `height` | non-negative `Dp` number | 32 dp when custom styling is used | Track height; touch area remains at least 48 dp |
| `cornerRadius` | non-negative `Dp` number | Half the track height (pill) | Track and selected-segment corner radius |
| `borderColor` | color string or `String` state | No border | Track outline color |
| `borderWidth` | non-negative `Dp` number | 1 dp with `borderColor`; otherwise 0 dp | Track outline width; uses the platform outline color when no color is supplied |
<!-- nexadoc:end SEGMENTED_CONTROL_PARAMETERS -->

`comment` annotates string literals supplied directly in `items`. When choices
come from a state or another dynamic array, localize each source string where
it is declared with `Locale.localized(...)`; a component-level note cannot
infer the contents of a dynamic array.

```nx
SegmentedControl(
    items: ranges,
    selected: selectedRange,
    animation: Spring(response: 0.35, damping: 0.82),
    tint: "#FFFFFF",
    background: "#E9EAF0",
    selectedTextColor: "#17171A",
    textColor: "#68686F",
    height: 36,
    cornerRadius: 18,
    borderColor: "#D2D3DA",
    borderWidth: 1
)
```

Use a custom timed curve when a design calls for specific motion:

```nx
SegmentedControl(
    items: ranges,
    selected: selectedRange,
    animation: Tween(
        duration: 0.32,
        delay: 0.04,
        easing: CubicBezier(x1: 0.2, y1: 0.0, x2: 0.2, y2: 1.0)
    )
)
```

An empty `items` array renders no control and reserves no layout height. A single
item renders as one selectable segment. If `selected` is not present in
`items`, no segment is highlighted; the control keeps the supplied value until
the user chooses an item.

`Picker` binds a `String` state to native selection UI on both platforms. Use
`icon` for an icon-only picker and pass `tint` when its selected icon should
reflect the current value, such as a task priority. Android presents an
anchored Material 3 Expressive menu; iOS uses a native menu.

`itemIcons` optionally supplies Nexa shared icons or registered `assets.icons`
aliases for menu rows. Its string
literal entries map by index to `items`; an option without a matching entry has
no leading icon. The compiler resolves built-in icons to native SF Symbols and
Material icons and custom aliases to registered font glyphs, so an unknown
icon name is a compile error.

Picker labels and choices, along with SegmentedControl choices, use Nexa's
generated localization catalogs. Literal strings are extracted automatically.
When choices are stored in a state array, localize each choice where it is
declared with `Locale.localized(...)`; initialize the selected state with the
same localized value because the choice strings are also the bound values.
The `comment` parameter supplies translator context when choices are literal
strings in the component call.

`PickerDate(type: Date)`, `PickerDate(type: Time)`, and
`PickerDate(type: DateTime)` each render only the picker. Add `Text(...)`
separately when a label is needed. The shared API covers single values and date
ranges, five presentation styles, optional bounds, locale, time zone, and native
picker colors. A picker manages its own value unless a mutable `Int64` timestamp
state is bound. Timestamps and bounds use milliseconds since the Unix epoch. For
`Date` and `Range`, `minimum` and `maximum` constrain whole calendar days in the
selected `timeZone`, or the device's time zone when omitted. For `Time`, exact
bounds are intersected with the currently selected local day. For `DateTime`,
exact bounds are rounded inward to the nearest selectable minute on both
platforms.

| Argument | Type | Default | Nullable | Behavior |
|---|---|---:|:---:|---|
| `type` | `Date`, `Time`, or `DateTime` | Required | No | Chooses calendar date, time, or date plus time. |
| `timestamp` | Mutable `Int64` state, epoch milliseconds | Current date and time | Yes | Binds one selected date/time and writes changes back. Keep it unset for a component-owned value. It cannot be combined with `selection: Range`. |
| `style` | `Automatic`, `Graphical`, `Compact`, `Input`, or `Wheel` | `Graphical` | No | Chooses the picker presentation. For single values, `Automatic` follows SwiftUI's native automatic choice (compact in the standard iOS view); Android mirrors its soft filled capsule trigger and opens a Material 3 dialog. `Graphical` keeps the calendar inline. `Compact` explicitly requests SwiftUI's native compact control on iOS and a Material 3 dialog on Android, using the same filled single-value trigger. A compact range uses an outlined capsule button that opens one native calendar in an iOS sheet or an Android Material 3 dialog; both require explicit confirmation, and dismissal cancels edits. `Input` maps to SwiftUI's native compact picker on iOS because SwiftUI has no field date-picker style there; Android uses editable Material 3 date/time fields inline. For ranges, `Automatic` and `Graphical` keep one calendar inline; `Compact` and `Input` open it in a sheet/dialog. `Wheel` uses the native wheel on iOS and Material 3 input controls on Android, which has no wheel picker. |
| `selection` | `Single` or `Range` | `Single` | No | `Range` selects a date interval and requires `type: Date`; bind both endpoints with `startTimestamp` and `endTimestamp`, or leave both unbound for local selection. |
| `startTimestamp` | Mutable `Int64` state, epoch milliseconds | No bound start | Yes | Start of a selected date range. Requires `selection: Range` and `endTimestamp`. |
| `endTimestamp` | Mutable `Int64` state, epoch milliseconds | No bound end | Yes | End of a selected date range. Requires `selection: Range` and `startTimestamp`. |
| `minimum` | `Int64` expression, epoch milliseconds | No lower bound | Yes | Earliest selectable calendar day for `Date` and `Range`; for `Time` it is intersected with the selected local day; for `DateTime` it is rounded up to a selectable minute. |
| `maximum` | `Int64` expression, epoch milliseconds | No upper bound | Yes | Latest selectable calendar day for `Date` and `Range`; for `Time` it is intersected with the selected local day; for `DateTime` it is rounded down to a selectable minute. |
| `tint` | `Color` | Platform native accent | Yes | Colors the selected date/time and picker accent. |
| `textColor` | `Color` | Platform native text color | Yes | Colors calendar, date-entry, and time-entry text. The native iOS compact control keeps its system text color. |
| `backgroundColor` | `Color` | Transparent | Yes | Colors the picker container behind the native control. |
| `pickerBackgroundColor` | `Color` | Platform native picker surface | Yes | Colors the native calendar or time-control surface, including the Android compact dialog. |
| `borderColor` | `Color` | No border | Yes | Draws a border around the picker container when `borderWidth` is greater than zero. |
| `borderWidth` | `Float` (dp/pt) | `0` | Yes | Border thickness. A positive value is required for the border to render. |
| `cornerRadius` | `Float` (dp/pt) | `0` | Yes | Rounds the picker container and its custom surfaces. |
| `locale` | `String` (BCP 47) | Device locale | Yes | Sets date order, date/time formatting, localized picker text, calendar conventions, and first weekday where supported. Examples: `"en-GB"`, `"fr-FR"`. |
| `timeZone` | `String` (IANA identifier) | Device time zone | Yes | Sets the civil time zone used to display timestamps, edit date/time fields, and interpret calendar-day bounds. Examples: `"Europe/Paris"`, `"America/Los_Angeles"`. |
| `is24Hour` | `Bool` | Locale/device preference | Yes | Overrides the hour cycle for `Time` and `DateTime`. If omitted, an explicit `locale` supplies its 12/24-hour cycle; without an explicit locale, the device's clock preference is used. |
| `enabled` | `Bool` | `true` | Yes | Disables interaction while keeping the picker visible. |
| `accessibilityLabel` | `String` | Type-based native label | Yes | Names the picker for assistive technology. Range pickers expose start and end controls using this label; compact range buttons also announce the formatted interval as their value. |

PickerDate does not add a separate label. Single-date and time selection use
SwiftUI `DatePicker` on iOS and Material 3 `DatePicker`,
`TimePicker`, or editable time fields on Android. The default presentation is
inline. For a single value, SwiftUI's native `Automatic` style chooses its
compact control in the standard app layout, and Android mirrors its soft filled
capsule trigger and opens a Material 3 dialog. `Compact` explicitly requests
the same presentation. For ranges, SwiftUI's native one-month `MultiDatePicker`
appears inline for `Automatic` and `Graphical`; `Compact` and `Input` open it in
a native sheet. Android uses a Material 3-styled, single-month range calendar
inline or in a compact dialog, matching iOS's one-month view while keeping
Android's native typography, colors, and touch behavior. Both range dialogs
keep selections as drafts until the user confirms; Cancel, Back, and outside
dismissal discard them. The Android dialog sizes its calendar to the available
screen width and reserves room for its actions, keeping all seven columns and
the confirmation controls reachable. `Input` maps to the closest native input
presentation, and Android maps `Wheel` to Material 3 input because Compose
Material 3 has no wheel picker.

A range without bound endpoint states keeps its selection locally. Date ranges
represent calendar days. `locale` controls date order and localized text while
`timeZone` controls how epoch timestamps map to displayed dates and times. See
the [SwiftUI DatePicker reference](https://developer.apple.com/documentation/swiftui/datepicker)
and the [Compose Material 3 date picker guide](https://developer.android.com/develop/ui/compose/components/datepickers)
for native control behavior. AndroidX's
[Material 3 DatePicker source](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/DatePicker.kt)
defines the dialog and calendar's native sizing and selection layout.

All color parameters accept ordinary `String` expressions, including state, so
an app can change the picker appearance while it is on screen. Both locale and
time-zone values accept ordinary `String` expressions as well, so an app can
switch regional settings while the picker is visible. Invalid locale or
time-zone identifiers fall back to the device setting. Compact iOS presentation
is owned by SwiftUI; Android's compact dialogs provide explicit actions and
discard unconfirmed edits when dismissed.
`is24Hour` accepts a Boolean expression, including state, so a time picker can
switch between 12-hour and 24-hour display without changing the locale.

```nx
app PriorityControl {
    state priorities: Array<String> = [
        Locale.localized("None"),
        Locale.localized("High"),
        Locale.localized("Normal"),
        Locale.localized("Low")
    ]
    state priority: String = Locale.localized("Normal")
    state priorityTint: String = "#FFCC00"
    state dueAt: Int64 = 1791446400000
    state rangeStart: Int64 = 1791244800000
    state rangeEnd: Int64 = 1791590400000
    state pickerLocale: String = "fr-FR"
    state pickerTimeZone: String = "America/Los_Angeles"

    body {
        Column(spacing: 12, padding: 16) {
            Picker(items: priorities, selected: priority, icon: "flag_filled", tint: priorityTint)
            Picker(items: priorities, selected: priority, itemIcons: ["calendar", "flag_filled", "edit", "flag"])
            PickerDate(type: Date)
            PickerDate(type: Time)
            PickerDate(timestamp: dueAt, type: DateTime, style: Compact)
            PickerDate(timestamp: dueAt, type: DateTime, locale: pickerLocale, timeZone: pickerTimeZone)
            PickerDate(timestamp: dueAt, type: Date, style: Graphical, minimum: 1788134400000, maximum: 1796083200000, pickerBackgroundColor: "#EFF6FF")
            PickerDate(timestamp: dueAt, type: Date, style: Input, locale: "fr-FR")
            PickerDate(type: Date, selection: Range, startTimestamp: rangeStart, endTimestamp: rangeEnd, tint: "#6750A4", pickerBackgroundColor: "#F5F3FF")
            PickerDate(timestamp: dueAt, type: Time, tint: "#6750A4", textColor: "#1D4ED8", backgroundColor: "#E8EDF5")
        }
    }
}
```
