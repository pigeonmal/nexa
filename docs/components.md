# Nexa Component Catalog 🧩

Every native UI component in Nexa compiles Ahead-Of-Time (AOT) into specialized, high-performance **SwiftUI views** on iOS and **Jetpack Compose composables** on Android. There is zero `AnyView` type erasure and zero reflection overhead.

---

## 1. Layout Containers

### `Column` & `Row`

Linear vertical (`Column`) and horizontal (`Row`) arrangement of child elements.

```nexa
Column(spacing: 12, padding: 16, background: "#1E293B", cornerRadius: 8) {
    Text("User Profile").fontSize(18).bold()
    Row(spacing: 8) {
        Icon(system: "person.circle", size: 24, tint: "#38BDF8")
        Text("Alex Developer").fontSize(14)
    }
}
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `spacing` | `Float64` | `0.0` | Gap in logical points between adjacent children. |
| `alignment` | `Alignment` | `Start` | Cross-axis alignment: `Start`, `Center`, or `End`. |
| `padding` | `Float64` | `0.0` | Uniform inset padding applied around the container. |
| `background` | `String` | `null` | Background color hex code (e.g. `"#FFFFFF"`). |
| `cornerRadius`| `Float64` | `0.0` | Corner radius for clipping and border backgrounds. |
| `width` / `height` | `Float64` | `auto` | Fixed dimensions in points. |
| `minWidth` / `maxWidth` | `Float64` | `auto` | Bounded dimensional constraints. |

---

### `Stack`

Layers child components on top of each other along the Z-axis (overlapping overlays).

```nexa
Stack(alignment: Center) {
    Image(asset: "hero_background", width: 300, height: 200, scale: Fill)
    Column(spacing: 8) {
        Text("Featured Project").fontSize(20).bold().foregroundColor("#FFFFFF")
        Button("Explore Now") { navigateToProject() }
    }
}
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `alignment` | `Alignment` | `Center` | Alignment of layered children: `Start`, `Center`, `End`. |
| `width` / `height` | `Float64` | `auto` | Explicit bounds for the stack frame. |

---

### `Form` & `Section`

Native grouped settings and preference panels. Compiles to grouped SwiftUI `Form` and Material 3 grouped cards on Android.

```nexa
Form {
    Section(title: "Account", footer: "Your email is verified.") {
        Text("Email: alex@example.com")
        Switch(value: isNotificationsEnabled, label: "Push Alerts")
    }
    Section(title: "Security") {
        Button("Change Password") { openPasswordReset() }
    }
}
```

| Component | Parameter | Type | Default | Description |
|---|---|---|---|---|
| `Form` | None | N/A | N/A | Root grouped scrollable container. |
| `Section` | `title` | `String?` | `null` | Optional localized header text. |
| `Section` | `footer` | `String?` | `null` | Optional localized descriptive footer note. |

---

### `Spacer` & `Divider`

- `Spacer(minLength: Float64?)`: Expands flexibly to fill available space along the parent axis.
- `Divider()`: Renders a 1-pixel native hairline separator.

---

## 2. Text & Presentation Primitives

### `Text`

Renders localized or dynamic text strings with native platform typography.

```nexa
Text("Nexa Ahead-Of-Time Engine")
    .fontSize(18)
    .bold()
    .foregroundColor("#0F172A")
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `value` (Positional)| `String \| Any` | Required | Content to display (accepts numbers, booleans, or strings). |
| `maxLines` | `Int32?` | `null` | Truncates text with an ellipsis when exceeded. |

---

### `Image`

Renders local assets or remote URLs with caching and aspect scaling.

```nexa
Image(asset: "avatar_placeholder", width: 48, height: 48, scale: Fit)
    .cornerRadius(24)
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `asset` | `String?` | `null` | Named asset bundled in the app's `assets/` folder. |
| `url` | `String?` | `null` | Remote HTTP/HTTPS image URL. |
| `width` / `height` | `Float64` | `auto` | Image dimensions in points. |
| `scale` | `ImageScale` | `Fit` | Scaling mode: `Fit`, `Fill`, or `None`. |

---

### `Icon`

Cross-platform unified system icon primitive. Automatically maps to **Apple SF Symbols** on iOS and **Google Material Symbols** on Android.

```nexa
Icon(system: "gearshape", size: 24, tint: "#38BDF8")
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `system` | `String` | Required | Canonical cross-platform icon name (e.g. `"gearshape"`, `"heart"`, `"star"`). |
| `size` | `Float64` | `24.0` | Square icon frame size in logical points. |
| `tint` | `String` | `"#000000"` | Hexadecimal color applied to the icon vector. |

---

### `ContentUnavailable`

Native empty state view. Compiles directly to iOS 17 `ContentUnavailableView` and Material 3 Empty State on Android.

```nexa
ContentUnavailable(
    title: "No Tasks Found",
    description: "You have completed all pending tasks for today.",
    icon: "checkmark.circle"
)
```

---

## 3. User Input & Controls

### `Button`

Interactive tap target with text, icon, and action block.

```nexa
Button("Save Changes", icon: "square.and.arrow.down") {
    saveProfile()
}
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `label` (Positional)| `String` | Required | Button text label. |
| `icon` | `String?` | `null` | Optional leading system icon name. |
| `disabled` | `Bool` | `false` | When true, disables interactions and dims opacity. |

---

### `TextInput`

Native single-line or multi-line text input field.

```nexa
TextInput(
    value: emailText,
    placeholder: "user@example.com",
    keyboard: Email,
    onChange: text => { emailText = text }
)
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `value` | `String` | Required | Bound string state. |
| `placeholder` | `String` | `""` | Placeholder hint when field is empty. |
| `keyboard` | `KeyboardType` | `Default` | `Default`, `Email`, `Numeric`, `Phone`, `URL`. |
| `isSecure` | `Bool` | `false` | Hides input text for passwords. |
| `onChange` | `(String) -> Void` | Required | Callback triggered on every keystroke. |

---

### `Switch`, `Slider`, and `SegmentedControl`

```nexa
// Toggle Switch
Switch(value: isDarkMode, label: "Dark Theme")

// Continuous Numeric Slider
Slider(value: volumeLevel, range: 0.0..100.0, step: 1.0)

// Tabbed Segmented Switcher
SegmentedControl(options: ["Day", "Week", "Month"], selected: selectedInterval)
```

---

### `DatePicker`

Native calendar and time selection component.

```nexa
DatePicker(
    timestamp: selectedDate,
    hasTime: true,
    onChange: ts => { selectedDate = ts }
)
```

---

## 4. Virtualized Lists: `FastList`

`FastList` is Nexa's high-performance virtualized list primitive. It generates pure concrete native views (zero `AnyView`) on SwiftUI and non-boxing lazy layouts on Compose.

```nexa
FastList(tasks, key: "id", rowHeight: 64) { task in
    Row(spacing: 12, padding: 12) {
        Button(icon: task.completed ? "checkmark.circle.fill" : "circle") {
            task.completed = !task.completed
        }
        Text(task.title).fontSize(16)
        Spacer()
    }
}.stickyHeader {
    Text("Today's Priority Tasks").bold()
}.onEndReached {
    loadMoreTasks()
}.onRefresh {
    await refreshFeed()
}
```

### `FastList` Configuration Matrix

| Parameter | Type | Default | Description |
|---|---|---|---|
| `collection` (Positional)| `Array<T>` | Required | Data source collection. |
| `key` | `String` | Required | Struct property name providing stable unique row identity. |
| `rowHeight` | `Float64?` | `null` | Optional fixed row height enabling fast offset indexing. |
| `axis` | `Axis` | `Vertical` | Scroll direction: `Vertical` or `Horizontal`. |

### `FastList` Trailing Modifiers

| Modifier | Argument | Description |
|---|---|---|
| `.stickyHeader { ... }` | Node block | Pins a persistent header to the top of the viewport during scrolling. |
| `.onEndReached { ... }` | Action block | Triggers pagination callback when approaching list bottom. |
| `.onRefresh { ... }` | Async action | Enables native Pull-To-Refresh control. |
| `.onMove { from, to => }`| Move closure | Enables native interactive row drag-and-drop reordering. |

---

## 5. Navigation & Sheets

### `AppBottomBar`

Top-level persistent native tab navigation bar.

```nexa
AppBottomBar(selected: currentTab) {
    Tab("Feed", icon: "house.fill") { FeedScreen() }
    Tab("Search", icon: "magnifyingglass") { SearchScreen() }
    Tab("Settings", icon: "gearshape") { SettingsScreen() }
}
```

---

### `NavigationSplitView`

Adaptive split-view navigation for tablets, foldables, and desktop layouts.

```nexa
NavigationSplitView(detailVisible: isDetailOpen) {
    Sidebar {
        SidebarContent()
    }
    Detail {
        DetailContent()
    }
}
```

---

### `BottomSheet` & `Dialog`

Modal overlays with native animations and gestures.

```nexa
BottomSheet(isPresented: showFilters) {
    FilterSheetContent()
}

Dialog(isPresented: showAlert, title: "Delete Item?", message: "This action cannot be undone.") {
    Button("Cancel") { showAlert = false }
    Button("Delete", role: Destructive) { performDelete() }
}
```

---

## 6. Universal Dot-Modifiers

Modifiers can be chained on any UI component in `.nx`:

```nexa
Text("Submit")
    .fontSize(16)
    .bold()
    .foregroundColor("#FFFFFF")
    .padding(horizontal: 24, vertical: 12)
    .background("#3B82F6")
    .cornerRadius(8)
    .shadow(color: "#000000", radius: 4, y: 2)
    .opacity(0.95)
```

| Category | Modifier | Description |
|---|---|---|
| **Typography** | `.fontSize(n)`, `.bold()`, `.foregroundColor(hex)` | Font styling and foreground color. |
| **Geometry** | `.padding(all \| h, v)`, `.width(n)`, `.height(n)` | Box model dimensions and insets. |
| **Visuals** | `.background(hex)`, `.cornerRadius(n)`, `.border(color, width)` | Borders, backgrounds, and clipping. |
| **Effects** | `.opacity(0.0..1.0)`, `.blur(n)`, `.shadow(color, radius, x, y)` | Native GPU graphics shaders and shadows. |
| **Gestures** | `.onTap { ... }`, `.onLongPress { ... }`, `.onDrag { dx, dy => ... }` | Touch gesture callbacks. |
| **Accessibility**| `accessibilityLabel`, `accessibilityHint`, `accessibilityRole` | Screen-reader metadata for VoiceOver and TalkBack. |
