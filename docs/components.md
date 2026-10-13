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

`Text` accepts optional `fontStyle` (`TextFontStyle`) for scalable native typography; omit it to use the platform default. Supported values are `LargeTitle`, `Title`, `Title2`, `Title3`, `Headline`, `Subheadline`, `Body`, `Callout`, `Footnote`, `Caption`, and `Caption2`. Swift output uses the matching SwiftUI text style, so Dynamic Type scales it with the user's accessibility setting. Android app output selects the nearest Material 3 typography role, which follows the system font scale. Android widgets use matching scalable `sp` sizes. An explicit `fontSize` overrides the role's default size.

Use `fontDesign` (`Default`, `Serif`, `Rounded`, or `Monospaced`) to select a system font family while keeping the same design choice on both platforms. The default family is used when the parameter is omitted.

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

### Selection and date controls

`Picker` binds a `String` state to native selection UI on both platforms. Use
`icon` for an icon-only picker and pass `tint` when its selected icon should
reflect the current value, such as a task priority. Android presents its
choices in a centered Material dialog; selecting an option leaves the parent
bottom sheet open. iOS uses a native menu.

`DatePicker(timestamp:, hasTime:)` stores a local date and time as epoch
milliseconds. It shows compact date and optional time controls: Android opens
centered Material date/time dialogs, while iOS uses compact native pickers.
Both stay within the app's date editor flow so the user can still cancel or
confirm the enclosing sheet.

```nx
app PriorityControl {
    state priorities: Array<String> = ["None", "High", "Normal", "Low"]
    state priority: String = "Normal"
    state priorityTint: String = "#FFCC00"
    state dueAt: Int64 = 1791446400000
    state includeTime: Bool = false

    body {
        Column(spacing: 12, padding: 16) {
            Picker(items: priorities, selected: priority, icon: "flag_filled", tint: priorityTint)
            DatePicker(timestamp: dueAt, hasTime: includeTime)
        }
    }
}
```
