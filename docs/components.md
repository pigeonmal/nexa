# Component & Layout Reference

Nexa provides a rich set of built-in native UI components that compile Ahead-Of-Time directly into specialized SwiftUI views for iOS and Jetpack Compose composables for Android.

---

## 1. Virtualized Lists & Collections: `FastList`

`FastList` is Nexa's flagship high-performance virtualized list container. It handles infinite lists of tens of thousands of items at continuous 60/120 FPS by reusing native views (`UICollectionView`/SwiftUI `LazyVStack` on iOS, `LazyColumn`/`LazyRow` on Android) with **zero `AnyView` type erasure** and **unboxed index states**.

### 1.1 Simple Dynamic List
Render a collection with explicit item and index bindings:

```nexa
app TodoListApp {
    state tasks: Array<String> = ["Buy groceries", "Write Nexa docs", "Ship release"]

    body {
        FastList(tasks, axis: Vertical, key: .self, rowHeight: 52) { task, index in
            Row(spacing: 12) {
                Text(index)
                Text(task)
            }
        }
    }
}
```

### 1.2 Count-Based Virtualized List
Efficiently render a large virtual sequence without allocating backing collection models:

```nexa
app MassiveListDemo {
    state totalRows: Int32 = 100000

    body {
        FastList(count: totalRows, rowHeight: 48) { index in
            Text("Row number: ${index}")
        }
    }
}
```

### 1.3 Sectioned List with Headers
Render grouped data with individual section headers:

```nexa
app SectionedDemo {
    state sections: Array<Array<String>> = [
        ["Apple", "Banana", "Cherry"],
        ["Carrot", "Daikon", "Eggplant"],
        ["Falcon", "Giraffe", "Hawk"]
    ]

    body {
        FastList(sections: sections, key: .self, rowHeight: 44) { item, itemIndex, sectionIndex in
            Text("Item: ${item}")
        }.sectionHeader {
            Text("Section ${sectionIndex}")
        }
    }
}
```

### 1.4 Sticky Headers, Pagination & Scroll Events
`FastList` supports declarative modifiers for sticky headers, infinite scroll pagination, and scroll tracking:

```nexa
app InfiniteScrollDemo {
    state items: Array<String> = ["Item 1", "Item 2", "Item 3"]
    state scrollPos: Int32 = 0
    state scrollCount: Int32 = 0

    body {
        FastList(items, key: .self, scrollPosition: scrollPos, rowHeight: 56) { item, index in
            Text(item)
        }.stickyHeader {
            // Pinned to the top while scrolling
            Text("Pinned Category Header")
        }.onEndReached {
            // Automatically triggered when user scrolls near the end
            items.append("Loaded Item")
        }.onScroll {
            // Emitted on scroll change with distinct flow emissions
            scrollCount = scrollCount + 1
        }
    }
}
```

### `FastList` Options & Modifiers Table

| Parameter / Modifier | Type | Description |
|---|---|---|
| *(positional)* | `Array<T>` | The primary collection of items to render. |
| `count:` | `Int32` | Render a count-based virtual list without allocating a collection. |
| `sections:` | `Array<Array<T>>` | Render a two-dimensional sectioned list. |
| `axis:` | `Vertical` \| `Horizontal` | Scrolling orientation (defaults to `Vertical`). |
| `rowHeight:` | `Float` | Fixed row extent for layout measurement optimizations. |
| `key:` | `.self` \| `.id` | Uniquely identify rows across updates for diffing. |
| `scrollPosition:` | `Int32` (state) | Two-way reactive binding for current first visible row index. |
| `.stickyHeader { ... }` | View Block | Renders a header pinned to the top of the viewport. |
| `.sectionHeader { ... }` | View Block | Renders a header preceding each section. |
| `.onEndReached { ... }` | Action Block | Dispatched when scrolled near bottom for pagination. |
| `.onScroll { ... }` | Action Block | Dispatched when scrolling occurs. |

---

## 2. Layout Containers

All layout containers accept common styling properties and arrange child nodes along designated axes.

### 2.1 `Column`
Arranges children vertically.

```nexa
Column(spacing: 16, alignment: Start, padding: 12) {
    Text("Title")
    Text("Subtitle")
}
```

### 2.2 `Row`
Arranges children horizontally.

```nexa
Row(spacing: 8, alignment: Center) {
    Text("Profile Status:")
    Text("Online")
}
```

### 2.3 `Stack`
Layers children along the Z-axis (overlays).

```nexa
Stack(alignment: Center) {
    Image(asset: "hero_background", description: "Background image", scale: Fill)
    Text("Hero Title")
}
```

### Container Styling Properties

| Property | Type | Description |
|---|---|---|
| `spacing:` | `Float` | Inter-item gap spacing. |
| `alignment:` | `Start` \| `Center` \| `End` | Alignment along cross axis. |
| `padding:` | `Float` | Content inset padding on all edges. |
| `width:` / `height:` | `Float` | Exact dimensions. |
| `minWidth:` / `maxWidth:` | `Float` | Horizontal dimension bounds. |
| `minHeight:` / `maxHeight:` | `Float` | Vertical dimension bounds. |
| `background:` | `String` / Color | Background color fill (e.g. `"#FFFFFF"`). |
| `cornerRadius:` | `Float` | Corner rounding radius. |
| `borderColor:` | `String` / Color | Border stroke color. |
| `borderWidth:` | `Float` | Border stroke width. |
| `opacity:` | `Float` | Alpha opacity (0.0 to 1.0). |

---

## 3. Interactive Controls

### 3.1 `Button`
Platform button triggering state mutations or actions.

```nexa
Button("Save Changes", icon: "checkmark", loading: isLoading, disabled: false) {
    saveRecord()
}
```
- Parameters: `label: String` (positional), `icon: String?`, `loading: Bool?`, `disabled: Bool?`.

### 3.2 `TextInput`
Single or multi-line text input field with software keyboard control.

```nexa
TextInput(
    value: email,
    placeholder: "user@domain.com",
    keyboard: Email,
    secure: false,
    multiline: false,
    autocorrect: false,
    capitalization: None,
    focused: isEmailFocused,
    maxLength: 100
) {
    // Optional onSubmit action block
    submitForm()
}
```

- **`keyboard:`**: `Default`, `Email`, `Numeric`, `Phone`, `Url`.
- **`capitalization:`**: `None`, `Characters`, `Words`, `Sentences`.

### 3.3 `Switch`
Platform toggle switch for boolean preferences.

```nexa
Switch(value: notificationsEnabled, label: "Allow push notifications")
```

### 3.4 `Pressable`
Wrap any container or view to add touch interactions with haptic feedback.

```nexa
Pressable(disabled: false, haptic: Medium) {
    Row(spacing: 8) {
        Text("Press Counter:")
        Text(taps)
    }
}.onPress {
    taps = taps + 1
}.onLongPress {
    taps = 0
}
```
- **`haptic:`**: `Selection`, `Light`, `Medium`, `Heavy`, `Success`, `Warning`, `Error`.
- Modifiers: `.onPress { ... }`, `.onLongPress { ... }`.

### 3.5 `Image`
Renders local asset images or remote network images with placeholders and scaling modes.

```nexa
// Local bundle asset:
Image(asset: "app_logo", description: "Application Logo", scale: Fit)

// Remote network URL:
Image(
    url: "https://example.com/photo.jpg",
    description: "User avatar",
    scale: Fill,
    placeholder: "avatar_placeholder"
)
```
- **`scale:`**: `Fit` (aspect fit), `Fill` (aspect fill).

---

## 4. Navigation & Modals

### 4.1 `NavigationStack` & `NavigationLink`
Establishes a stack-based navigation flow across declared `screen` targets:

```nexa
app AppNavigator {
    screen Home {
        Column {
            Text("Home Screen")
            NavigationLink(destination: Profile) {
                Text("Open Profile")
            }
        }
    }

    screen Profile {
        Column {
            Text("Profile Screen")
            NavigationBack(label: "Return")
        }
    }

    body {
        NavigationStack(root: Home)
    }
}
```

### 4.2 `BottomSheet`
Presents a modal sheet over the current view hierarchy:

```nexa
BottomSheet(isPresented: showFilterSheet, partial: true) {
    Column(padding: 20) {
        Text("Filter Options")
        Button("Apply") {
            showFilterSheet = false
        }
    }
}
```

### 4.3 `AppBottomBar`
Bottom navigation tab bar hosting multiple destinations:

```nexa
AppBottomBar(selected: currentTab) {
    Tab(index: 0, label: "Feed", icon: "newspaper")
    Tab(index: 1, label: "Search", icon: "magnifyingglass")
    Tab(index: 2, label: "Notifications", icon: "bell", badge: 4)
    Tab(index: 3, label: "Settings", icon: "gear")
}
```

---

## 5. System & Interaction Modifiers

### 5.1 `RefreshControl`
Adds pull-to-refresh behavior to scrollable content:

```nexa
RefreshControl(isRefreshing: isRefreshing).onRefresh {
    loadLatestData()
} {
    Column {
        Text("Pull down to refresh")
    }
}
```

### 5.2 `KeyboardAware`
Automatically offsets and scrolls content above the software keyboard:

```nexa
KeyboardAware(dismiss: Interactive) {
    Column {
        TextInput(value: username, placeholder: "Username")
        TextInput(value: password, placeholder: "Password", secure: true)
    }
}
```
- **`dismiss:`**: `Interactive`, `OnDrag`, `Tap`.

### 5.3 `StatusBar`
Controls platform status bar appearance per screen:

```nexa
StatusBar(style: Light, hidden: false, background: "#000000")
```
- **`style:`**: `Light`, `Dark`.

### 5.4 `Direction`
Overrides layout and text directionality:

```nexa
Direction(value: RTL) {
    Column {
        Text("Right-to-left localized content")
    }
}
```

### 5.5 `Accessibility`
Screen reader annotations for VoiceOver and TalkBack:

```nexa
Accessibility(label: "Close settings", hint: "Discards changes and exits", role: Button) {
    Image(asset: "close_icon", description: "Close")
}
```
- **`role:`**: `Button`, `Header`, `Image`, `Link`, `Search`, `Summary`, `Tab`.

---

## 6. Lifecycle Hooks

Respond to appearance and application state events:

```nexa
Column {
    OnAppear {
        // Synchronous setup
        totalViews = totalViews + 1
    }

    OnAppear async {
        // Asynchronous data fetching
        await syncDataFromServer()
    }

    OnDisappear {
        // Cleanup resources
        stopTimer()
    }

    OnActive {
        // App returned to foreground
    }

    OnBackground {
        // App sent to background
    }
}
```

---

## 7. Custom Reusable Components & Content Slots

Create reusable UI components with props and transclusion slots using `Content()`:

```nexa
// Custom reusable Card with a content slot
component TitledCard(title: String) {
    body {
        Column(spacing: 8, padding: 16, background: "#F5F5F7", cornerRadius: 12) {
            Text(title)
            Content() // Children passed to TitledCard will render here
        }
    }
}

app MainApp {
    body {
        Column {
            TitledCard(title: "Account Overview") {
                Text("Balance: $4,500")
                Button("Transfer") { ... }
            }
        }
    }
}
```

---

## 8. Platform-Specific Blocks

Conditionally specialize view trees for iOS or Android at compile-time:

```nexa
Column {
    Platform(ios) {
        Text("Styled with iOS Human Interface Guidelines")
    }

    Platform(android) {
        Text("Styled with Android Material You Design")
    }
}
```
