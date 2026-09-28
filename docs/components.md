# Component Reference

Every native UI component in Nexa compiles Ahead-Of-Time into specialized SwiftUI views on iOS and Jetpack Compose composables on Android.

---

### `Column`
Arranges children vertically with optional spacing, alignment, padding, and borders.

```nexa
app ColumnExample {
    body {
        Column(spacing: 12, alignment: Start, padding: 16, background: "#FFFFFF", cornerRadius: 8) {
            Text("Title")
            Text("Subtitle")
        }
    }
}
```
- **Properties**: `spacing`, `alignment: Start | Center | End`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`.

---

### `Row`
Arranges children horizontally with optional spacing, alignment, and styling.

```nexa
app RowExample {
    body {
        Row(spacing: 8, alignment: Center) {
            Text("Status:")
            Text("Active")
        }
    }
}
```
- **Properties**: Accepts the same layout and border properties as `Column`.

---

### `Stack`
Layers children along the Z-axis (overlays).

```nexa
app StackExample {
    body {
        Stack(alignment: Center) {
            Image(asset: "hero_banner", description: "Banner", scale: Fill)
            Text("Overlay Title")
        }
    }
}
```
- **Properties**: `alignment: Start | Center | End`, along with the same sizing, visual effect, and background options as `Column`.

---

### `FastList`
High-performance virtualized list with zero `AnyView` type erasure and unboxed scroll state. Supports dynamic collections, count-based ranges, sectioned groups, sticky headers, and pagination.

```nexa
app FastListExample {
    state scrollPosition: Int32 = 0
    state pagesLoaded: Int32 = 0
    state scrollEvents: Int32 = 0

    body {
        FastList(count: 100000, rowHeight: 52, scrollPosition: scrollPosition) { index in
            Text(index)
        }.stickyHeader {
            Text("Category Header")
        }.onEndReached {
            pagesLoaded = pagesLoaded + 1
        }.onScroll {
            scrollEvents = scrollEvents + 1
        }
    }
}
```
- **Sources**: `FastList(collection, ...)`, `FastList(count: Int32, ...)`, or `FastList(sections: Array<Array<T>>, ...)`.
- **Options**: `axis: Vertical | Horizontal | Grid(columns)`, where `columns` is a positive integer literal; positive literal `rowHeight`, scalar `key`, and mutable `Int32` `scrollPosition`.
- **Modifiers**: `.stickyHeader { ... }`, `.sectionHeader { ... }`, `.onEndReached { ... }`, `.onScroll { ... }`.

---

### `Text`
Renders formatted text with typography, color, and wrapping controls.

```nexa
app TextExample {
    body {
        Text("Hello Nexa", color: "#333333", fontSize: 18, fontWeight: Bold, lineLimit: 2, selectable: true)
            .fontSize(18).bold().padding(12)
    }
}
```
- **Properties**: `value` (positional), `color`, `fontSize`, `fontWeight`, `padding`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `selectable`.
- **Chained styles**: `.fontSize(value)`, `.bold()`, `.padding(value)`, `.opacity(value)`, `.scale(value)`, `.rotation(degrees)`, `.shadow(radius:, x:, y:, color:)`, `.blur(radius)`, `.clip(shape: Rounded(radius))`, and `.zIndex(value)`. These apply to `Text`, `Column`, `Row`, and `Stack`.

Visual modifier values are compile-time numeric literals. Opacity must be between `0` and `1`; blur, shadow radius, and rounded clip radius must be non-negative. Rotation and shadow offsets may be negative. `zIndex` is a signed `Int32`.

```nexa
Column {
    Text("Featured")
}
.scale(1.02)
.shadow(radius: 8, x: 0, y: 4, color: "#00000040")
.clip(shape: Rounded(12))
```

---

### `Spacer`
Expands to consume remaining space along its parent `Row` or `Column` axis.

```nexa
Row {
    Text("Leading")
    Spacer()
    Text("Trailing")
}
```

---

### `Divider`
Draws a horizontal divider using the requested color and thickness.

```nexa
Column {
    Text("First")
    Divider(color: "#808080", thickness: 1)
    Text("Second")
}
```

- **Properties**: required `color` and `thickness`.

---

### `Button`
Native platform button triggering state mutations or actions.

```nexa
app ButtonExample {
    state saves: Int32 = 0

    body {
        Button("Save Record", icon: "checkmark", loading: false, disabled: false) {
            saves = saves + 1
        }
    }
}
```
- **Properties**: `label` (positional), `icon: String?`, `loading: Bool?`, `disabled: Bool?`, and a trailing action block.

---

### `TextInput`
Single or multi-line text field with software keyboard integration.

```nexa
app TextInputExample {
    state email: String = ""
    state isEmailFocused: Bool = false
    state submitted: Int32 = 0

    body {
        TextInput(
            value: email,
            placeholder: "name@domain.com",
            keyboard: Email,
            secure: false,
            multiline: false,
            autocorrect: false,
            capitalization: None,
            focused: isEmailFocused,
            maxLength: 100
        ) {
            submitted = submitted + 1
        }
    }
}
```
- **Properties**: `value`, `placeholder`, `keyboard` (`Text`, `Number`, `Email`, `Phone`, `Url`), `secure`, `multiline`, `autocorrect`, `capitalization` (`None`, `Characters`, `Words`, `Sentences`), `focused`, and positive literal `maxLength`. The optional trailing action block handles submit for single-line fields.

---

### `Switch`
Platform toggle switch for boolean state.

```nexa
app SwitchExample {
    state isEnabled: Bool = false

    body {
        Switch(value: isEnabled, label: "Enable Notifications")
    }
}
```
- **Properties**: `value: Bool` (two-way binding), `label: String`.

---

### `Slider`
Continuous value control backed by a mutable `Float64` state. Its range must contain a whole number of steps, and Android requires the bounds to fit a native `Float`.

```nexa
app SliderExample {
    state volume: Float64 = 0.5

    body {
        Slider(value: volume, min: 0.0, max: 1.0, step: 0.01)
    }
}
```
- **Properties**: `value: Float64` (two-way binding), `min`, `max`, and positive `step` as numeric literals.

---

### `ProgressBar` and `ProgressRing`
Native linear and circular progress indicators. Both accept a `Float64` expression and clamp it to the normalized `0...1` progress range.

```nexa
app DownloadProgress {
    state progress: Float64 = 0.35

    body {
        ProgressBar(progress: progress)
        ProgressRing(progress: progress)
    }
}
```
- **Properties**: `progress: Float64`.

### `SegmentedControl`
Compact single-choice control for a small set of distinct string options. `selected` is a mutable `String` state bound to the active option.

```nexa
app FilterTabs {
    state filters: Array<String> = ["All", "Open", "Closed"]
    state selectedFilter: String = "All"

    body {
        SegmentedControl(items: filters, selected: selectedFilter)
        Text(selectedFilter)
    }
}
```
- **Properties**: `items: Array<String>`, `selected: String` (mutable state binding).
- Option labels should be distinct; the selected value is the matching label.

### `Picker`
Native menu-style single-choice control for a set of string options. `selected` is a mutable `String` state bound to the active option.

```nexa
app SizePicker {
    state sizes: Array<String> = ["Small", "Medium", "Large"]
    state selectedSize: String = "Medium"

    body {
        Picker(items: sizes, selected: selectedSize)
        Text(selectedSize)
    }
}
```
- **Properties**: `items: Array<String>`, `selected: String` (mutable state binding).
- Option labels should be distinct; the selected value should match one of the labels.

---

### `Pressable`
Gesture wrapper detecting tap and long-press interactions with hardware haptics.

```nexa
app PressableExample {
    state taps: Int32 = 0

    body {
        Pressable(disabled: false, haptic: Medium) {
            Text("Tap me!")
        }.onPress {
            taps = taps + 1
        }.onLongPress {
            taps = 0
        }
    }
}
```
- **Properties**: `disabled: Bool`, optional `haptic: Light | Medium | Heavy`.
- **Modifiers**: `.onPress { ... }`, `.onLongPress { ... }`.

---

### `Image`
Renders local bundle assets or remote network images with scaling and placeholders.

```nexa
app ImageExample {
    body {
        Column {
            Image(asset: "logo", description: "App Logo", scale: Fit)
            Image(url: "https://example.com/pic.png", description: "Avatar", scale: Fill, placeholder: "avatar_ph")
        }
    }
}
```
- **Properties**: `asset: String` OR `url: String`, `description: String`, `scale: Fit | Fill`, `placeholder: String`.

Put app-owned image files directly in `assets/images/` and reference each file by its lowercase filename without an extension. For example, `assets/images/hero_banner.png` is `Image(asset: "hero_banner", ...)`. Nexa copies these files into the iOS asset catalog and Android `drawable-nodpi` resources when it generates the native projects. Supported formats are PNG, JPG/JPEG, and WebP. Asset names must start with a lowercase letter and contain only lowercase letters, digits, and underscores.

---

### `AppBottomBar`
Bottom tab navigation bar hosting multiple destinations with reactive tab switching.

```nexa
app TabExample {
    state currentTab: Int32 = 0

    body {
        AppBottomBar(selected: currentTab) {
            Tab(index: 0, label: "Home", icon: "house") {
                Column { Text("Home") }
            }
            Tab(index: 1, label: "Search", icon: "magnifyingglass") {
                Column { Text("Search") }
            }
            Tab(index: 2, label: "Profile", icon: "person", badge: "3") {
                Column { Text("Profile") }
            }
        }
    }
}
```
- **Properties**: `selected: Int32` (state binding).
- **Tab Declarations**: `Tab(index: Int32, label: String, icon: String?, badge: String?) { ... }`.
- Compiles to native `TabView` on iOS and `NavigationBar` on Android.

---

### `NavigationStack` & `NavigationLink`
Hierarchical screen stack navigation with push transitions and swipe-to-back gestures.

```nexa
app NavigationExample {
    screen Home {
        Column {
            NavigationLink(destination: Details) {
                Text("View Details")
            }
        }
    }

    screen Details {
        Column {
            Text("Details")
            NavigationBack(label: "Go Back")
        }
    }

    body {
        NavigationStack(root: Home)
    }
}
```
- **`NavigationStack(root: ScreenName)`**: Container hosting the stack.
- **`NavigationLink(destination: ScreenName, when: Bool?)`**: Push transition.
- **`NavigationBack(label: String?)`**: Back button.

---

### `Link`
Opens a URL in the system browser.

```nexa
app LinkExample {
    body {
        Column {
            Link(url: "https://example.com") {
                Text("Visit example.com")
            }
        }
    }
}
```
- **Properties**: `url: String` (must include a valid absolute scheme).

---

### `BottomSheet`
Modal presentation sheet appearing from the bottom of the screen.

```nexa
app BottomSheetExample {
    state showModal: Bool = false

    body {
        Column {
            Button("Open") { showModal = true }
            BottomSheet(isPresented: showModal, partial: true) {
                Column(padding: 20) {
                    Text("Modal Content")
                    Button("Close") { showModal = false }
                }
            }
        }
    }
}
```
- **Properties**: `isPresented: Bool` (state binding), `partial: Bool`.

### `Dialog`
Native modal alert controlled by mutable Boolean state. The title and message accept `String` expressions, and the child block contains the dialog buttons.

```nexa
app DialogExample {
    state showConfirmation: Bool = false
    state itemCount: Int32 = 1

    body {
        Button("Delete") { showConfirmation = true }
        Dialog(
            isPresented: showConfirmation,
            title: "Delete item?",
            message: "This action cannot be undone."
        ) {
            Button("Cancel") { showConfirmation = false }
            Button("Delete") {
                itemCount = 0
                showConfirmation = false
            }
        }
    }
}
```
- **Properties**: `isPresented: Bool` (state binding), `title: String`, `message: String`.
- **Children**: native button content. Buttons close the dialog by setting its binding to `false`.

---

### `RefreshControl`
Pull-to-refresh wrapper for scrollable containers.

```nexa
app RefreshExample {
    state isRefreshing: Bool = false
    state postCount: Int32 = 20

    body {
        RefreshControl(isRefreshing: isRefreshing) {
            FastList(count: postCount) { index in
                Text(index)
            }
        }.onRefresh {
            isRefreshing = false
        }
    }
}
```
- **Properties**: `isRefreshing: Bool`.
- **Modifiers**: `.onRefresh { ... }`.

---

### `KeyboardAware`
Automatically scrolls and insets content when the software keyboard appears.

```nexa
app KeyboardExample {
    state username: String = ""
    state password: String = ""

    body {
        KeyboardAware(dismiss: Interactive) {
            Column {
                TextInput(value: username, placeholder: "Username")
                TextInput(value: password, placeholder: "Password", secure: true)
            }
        }
    }
}
```
- **Properties**: `dismiss: Interactive | Never`.

---

### `StatusBar`
Configures system status bar styling per screen.

```nexa
app StatusBarExample {
    body {
        StatusBar(style: Light, hidden: false, background: "#000000")
        Text("Status bar example")
    }
}
```
- **Properties**: `style: Default | Light | Dark`, `hidden: Bool`, and an optional hexadecimal `background` color.

---

### `Direction`
Sets the app-wide text and layout direction for localization.

```nexa
app DirectionExample {
    body {
        Direction(value: RTL)
        Column { Text("Arabic or Hebrew localized UI") }
    }
}
```
- **Properties**: `value: LTR | RTL`.

---

### `Accessibility`
Screen reader annotations for VoiceOver (iOS) and TalkBack (Android).

```nexa
app AccessibilityExample {
    body {
        Accessibility(label: "Close dialog", hint: "Discards changes", role: Button) {
            Image(asset: "close_icon", description: "Close")
        }
    }
}
```
- **Properties**: `label: String`, optional `hint: String`, and `role: None | Button | Link | Header | Image`.

---

### Custom Components & `Content()`
Define reusable components with parameters and transclusion slots.

```nexa
component Card(title: String) {
    body {
        Column(spacing: 8, padding: 16, background: "#F5F5F7", cornerRadius: 12) {
            Text(title)
            Content() // Children passed to Card render here
        }
    }
}

app CardExample {
    body {
        Card(title: "Summary") {
            Text("Child text")
        }
    }
}
```
- `Content()` renders the children passed to the component call.
