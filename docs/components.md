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

`Row(alignment:)` positions children vertically (`Start`, `Center`, or `End`). Use `horizontalArrangement: Start|Center|End` to place the child group across the available width; it defaults to `Start`. Setting `horizontalArrangement` makes the Row use available width, constrained by any explicit width bounds.

## Input and interaction

Bind controls directly to declared `state`. A `TextInput` accepts `value` and `placeholder`; use `.onChange { ... }` only when extra action work is needed. `Button` action blocks contain app state updates.

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

## Repeated data

`FastList` is the virtualized list component. Pass exactly one data source (`collection`, `count:`, or `sections:`), a row key when rendering a collection, and an explicit binding block. The compiler checks the number and type of row bindings.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| Data source | `Array<T>`, `count: Int32`, or `sections: Array<Array<T>>` | Exactly one source | Positional arrays and named `count:` or `sections:` sources are supported. |
| `key` | Statically known scalar item property | Required for collection sources | Declared in the list row binding; provides stable identity for virtualized items. |
| `axis` | `Vertical`, `Horizontal`, `Grid(Int32)`, or `AutoGrid(Int32)` | `Vertical` | `Grid(columns)` fixes the column count. `AutoGrid(minCellWidth)` fits as many columns as available width allows while preserving the minimum cell width in logical units (dp on Android, points on iOS). Both numeric arguments must be positive integer literals. |
| `rowHeight` | `Float64` | Content-sized | Optional fixed row height; for grids it is the cell height. Must be greater than zero. |
| `spacing` | `Float64` | `0` | Gap between adjacent items. Applies along the list axis, or to both rows and columns for `Grid` and `AutoGrid`; `pageSnap` requires zero spacing. |
| `scrollPosition` | `Int32` state | Omitted | Binds the first visible item index for supported flat lists. |
| `scrollEnabled` | `Bool` expression | Platform default | Enables or disables user-driven scrolling for flat, sectioned, and adaptive-grid lists; supports reactive state. |
| `reverseLayout` | `Bool` | `false` | Reverses a flat vertical list; cannot be combined with sections, sticky headers, or page snapping. |
| `pageSnap` | `Bool` | `false` | Pages a flat vertical list by viewport height; omit `rowHeight`. |
| `native` | `Bool` | `false` | Uses a platform-native vertical list without custom scroll behavior. |
| `showDividers` | `Bool` | `true` | Shows automatically rendered separators between rows in vertical flat and sectioned lists. Set to `false` to hide them. Horizontal lists, grids, and page-snapping lists do not draw row separators. |
| `accessibilityLabel` | `String` | Omitted | Accessible label for the list. |

| Modifier | Arguments and body | Behavior |
|---|---|---|
| `.onEndReached { ... }` | Actions | Runs when a supported flat list nears its final item. |
| `.onScroll { ... }` | Actions | Runs as a supported flat list scrolls. |
| `.onMove(enabled:) { from, to -> ... }` | Optional `Bool` (`true`); event actions | Reorders a native array-backed vertical list. |
| `.stickyHeader { ... }` | Nodes | Adds a header to a flat vertical list. |
| `.sectionHeader { section -> ... }` | Nodes | Renders a header for each `sections:` group. |
| `.swipeActions { ... }` | Nodes | Adds one trailing action to a native list row. |

Use `AutoGrid` for responsive media galleries so the same source adapts to phones, tablets, and resizable windows:

```nx
FastList(videos, key: .id, axis: AutoGrid(168), rowHeight: 240) { video, index in
    Image(url: video.coverUrl, description: "Video thumbnail", scale: Fill, maxHeight: 240)
}
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
        FastList(orders, key: .id, native: true) { order, index in
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

## Navigation and modal content

Declare destinations as `screen` blocks and select the root with `NavigationStack(root: ...)`. A `NavigationLink` takes a declared destination. Incoming URL routing is not part of the current language surface; use `Link(url: ...)` to open an external URL.

`NavigationStack` uses the platform's native inline title by default: SwiftUI's centered inline title on iOS and Material 3's centered top app bar on Android. Set `titleAlignment: Leading` to place the title beside Back; set `Center` to keep the centered layout. Set `titleMode: Large` for SwiftUI's large title and Material 3's expanding large top app bar.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `root` | Declared screen | Required | Screen shown at the root of the stack. |
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
        }
    }

    screen ContactSupport {
        Column(spacing: 12, padding: 16) {
            Text("We're here to help")
            Link(url: "https://support.example.com") {
                Text("Open help center")
            }
            NavigationBack(label: "Back to support")
        }
    }

    body {
        NavigationStack(root: HelpHome)
    }
}
```

### BottomSheet

`BottomSheet` presents modal content using the native sheet presentation on each
platform. Partial sheets use SwiftUI presentation detents on iOS and Material 3
Expressive `ModalBottomSheet` on Android. The drag handle is visible by default;
set `showDragHandle: false` to hide it consistently on both platforms.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `isPresented` | `Bool` binding | Required | Controls whether the sheet is shown. |
| `partial` | `Bool` | `false` | Uses a resizable partial sheet when true; false presents full-screen content. |
| `largeOnly` | `Bool` | `false` | For partial sheets, disables the medium/partially expanded detent. |
| `showDragHandle` | `Bool` | `true` | Shows or hides the native sheet drag indicator. It has no visible effect for full-screen presentation. |
| `title` | `String` | `nil` | Optional title for the partial sheet's native navigation bar. |
| child block | Nexa nodes | Required | Content displayed inside the sheet. `Toolbar` leading/trailing items are supported when a partial sheet has a title. |

```nx
screen TaskEditor {
    state editorPresented: Bool = false

    body {
        Column(spacing: 12, padding: 16) {
            Button("Edit task") { editorPresented = true }
            BottomSheet(
                isPresented: editorPresented,
                partial: true,
                title: "Edit task",
                showDragHandle: false
            ) {
                Column(spacing: 12, padding: 24) {
                    Text("Update the task details")
                    Button("Done") { editorPresented = false }
                }
            }
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
| `fontDesign` | `Default`, `Serif`, `Rounded`, `Monospaced` | Platform default design | Selects the system font design. |
| `fontWeight` | `Normal`, `Medium`, `Semibold`, `Bold` | Semantic style weight, otherwise `Normal` | Explicit weight overrides the weight associated with `fontStyle`. |
| `padding` | Dimension | None | Adds equal padding around the text. |
| `opacity` | `Float` from `0` to `1` | `1` | Applies alpha to the rendered text. |
| `scale` | `Float` | `1` | Scales the text view uniformly. |
| `rotation` | `Float` degrees | `0` | Rotates the text view. |
| `shadow` | `Shadow(radius, x, y, color)` | None | Draws a shadow behind glyphs using the blur radius and offsets. |
| `blur` | Dimension | None | Blurs the rendered text. |
| `clip` | `Rounded(radius)` | None | Clips the text view to a rounded rectangle. |
| `zIndex` | `Int32` | `0` | Controls drawing order relative to overlapping siblings. |
| `lineLimit` | Positive integer | Unlimited | Limits visible lines and truncates overflow with an ellipsis. |
| `lineHeight` | Dimension | Native line height | Sets the total line box height. |
| `letterSpacing` | Dimension | `0` | Adds tracking between characters. |
| `strikethrough` | `Bool` | `false` | Draws a line through the text. |
| `italic` | `Bool` | `false` | Uses the italic font face. |
| `underline` | `Bool` | `false` | Underlines the text. |
| `selectable` | `Bool` | `false` | Enables native text selection and copy actions. |
| `accessibilityLabel` | `String` | Text value | Overrides the accessibility label. |
| `accessibilityHint` | `String` | None | Supplies an accessibility hint. |
| `accessibilityValue` | `String` | None | Supplies an accessibility value. |
| `accessibilityRole` | Accessibility role | Text role | Overrides the accessibility role. |

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

### ContentUnavailable

`ContentUnavailable` presents an empty or unavailable state using the platform's native visual treatment: SwiftUI `ContentUnavailableView` on iOS and a centered Material 3 empty state on Android. The supporting description is optional; omitting it removes the detail text and its spacing. Optional color parameters customize each visual part independently; omitted colors keep the native platform defaults.

| Parameter | Type | Required/default | Behavior |
|---|---|---|---|
| `title` | `String` expression | Required | Main empty-state heading. |
| `icon` | Shared system icon name (`String`) | Required | Icon name from Nexa's shared icon catalog; unknown names are compile errors. |
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

`itemIcons` optionally supplies shared system icons for menu rows. Its string
literal entries map by index to `items`; an option without a matching entry has
no leading icon. The compiler resolves names to native SF Symbols and Material
icons, so an unknown icon name is a compile error.

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
