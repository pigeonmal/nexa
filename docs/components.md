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

Event modifiers are reserved for interactions and system triggers. For example, `Pressable` uses `.onTap`, `.onLongPress`, `.onDoubleTap`, `.onDrag`, `.onPinch`, and `.contextMenu` as documented in its syntax-audit entry. Styling remains component arguments.

## Repeated data

`FastList` is the virtualized list component. Pass exactly one data source (`collection`, `count:`, or `sections:`), a row key when rendering a collection, and an explicit binding block. The compiler checks the number and type of row bindings.

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

### Text

`Text` accepts optional `fontStyle` (`TextFontStyle`) for scalable native typography; omit it to use the platform default. Supported values are `LargeTitle`, `Title`, `Title2`, `Title3`, `Headline`, `Subheadline`, `Body`, `Callout`, `Footnote`, `Caption`, and `Caption2`. Swift output uses the matching SwiftUI text style, so Dynamic Type scales it with the user's accessibility setting. Android app output selects the nearest Material 3 typography role, which follows the system font scale. Android widgets use matching scalable `sp` sizes. An explicit `fontSize` overrides the role's default size.

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
