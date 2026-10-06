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

### `Form` and `Section`
Use `Form` for a native, scrollable settings screen. Its `Section` children map to SwiftUI form sections on iOS and grouped Material settings content on Android. Sections can have an optional localized title and footer.

```nexa
app SettingsExample {
    state enabled: Bool = true

    body {
        Form {
            Section {
                Switch(value: enabled, label: "Notifications")
            }
            Section(title: "About", footer: "Version 1.0") {
                Text("Example app")
            }
        }
    }
}
```

Place `Section(title: String?, footer: String?)` inside a `Form` for native grouped behavior.

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
- **Options**: `axis: Vertical | Horizontal | Grid(columns)`, where `columns` is a positive integer literal; positive literal `rowHeight`, scalar `key`, mutable `Int32` `scrollPosition`, `reverseLayout: true`, `pageSnap: true`, and `native: true`. Native mode emits SwiftUI `List` for flat and sectioned iOS lists, and the standard Compose lazy list on Android; it excludes custom scroll behavior and reverse or paged layouts. Native lists accept `.swipeActions` for trailing row actions. Omit `native` when using FastList's custom virtualization options. `pageSnap` is limited to flat vertical lists: each virtualized row fills the list viewport and native scrolling settles on page boundaries, with a maximum of one page per fling. Its `scrollPosition`, `onScroll`, and `onEndReached` updates are reported after a page settles. It cannot be combined with `rowHeight`, `reverseLayout`, sectioned sources, or `stickyHeader`. A `RefreshControl` can wrap a paged list and keeps the platform-native pull-to-refresh interaction. Reverse layout keeps the last logical item at the bottom, maps row bindings to source indexes, and follows new items while the user remains at the latest item; it cannot be combined with sectioned sources, `stickyHeader`, or `RefreshControl`.
- **Modifiers**: `.stickyHeader { ... }`, `.sectionHeader { ... }`, `.onEndReached { ... }`, `.onScroll { ... }`, `.swipeActions { ... }` for native trailing row actions, and `.onMove(enabled: canMove) { from, to -> ... }` for flat array-backed native lists (`enabled` defaults to `true`). The indexes are zero-based `Int32` positions in the displayed rows. Use `backing.moveSubset(from, to, visibleRows)` when `visibleRows` is a stable-order filtered subset of the backing array; this reorders visible entries while leaving hidden entries in their current slots. Do not pass a separately sorted projection. SwiftUI uses `List.onMove` and `.moveDisabled`; Compose detects a long-press drag only while enabled. Generated code, headless actions, and DevRuntime support the same mutation.

---

### `Text`
Renders formatted text with typography, color, and wrapping controls.

```nexa
app TextExample {
    body {
        Text("Hello Nexa", color: "#333333", fontSize: 18, fontWeight: Bold, lineLimit: 2, strikethrough: true, selectable: true)
            .fontSize(18).bold().padding(12)
    }
}
```
- **Properties**: `value` (positional), `color`, `alignment: Leading | Center | Trailing` (multiline text alignment), `fontSize`, `fontWeight`, `padding`, `opacity`, `scale`, `rotation`, `shadow`, `blur`, `clip`, `zIndex`, `lineLimit`, `lineHeight`, `letterSpacing`, `strikethrough`, `selectable`.
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

### `ContentUnavailable`
Shows a native-style empty or unavailable state with a shared system icon. The `title`, `description`, and optional translator `comment` are localized through Nexa's built-in localization system.

```nexa
ContentUnavailable(
    title: "Inbox is empty",
    icon: "inbox",
    description: "Tasks you add will appear here."
)
```

- **Properties**: required `title: String`, `icon: String` from the shared system icon catalog, and `description: String`; optional `comment: String` for translators.
- **iOS**: Uses SwiftUI `ContentUnavailableView` on iOS 17 and later, with a native SwiftUI fallback on earlier supported versions.
- **Android**: Uses a centered Jetpack Compose Material 3 icon and text layout.
- Use this component in the content area when a list, search, or feature has no results. It fills the available space in its parent.

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
        Button("Save Record", icon: "check", loading: false, disabled: false) {
            saves = saves + 1
        }
    }
}
```
- **Properties**: `label` (positional), `icon: String?` from the shared system icon catalog, `loading: Bool?`, `disabled: Bool?`, `style`, `size`, `shape`, and `tint`. A tint can be a hexadecimal color or a `String` state holding a hexadecimal color. The trailing action block handles the press. Use `Icon(sfsymbol:)` or `Icon(materialsymbol:)` for platform-specific icons outside the button.

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
            keyboardType: Email,
            autofill: Username,
            returnKeyType: Done,
            isSecure: false,
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
- **Properties**: `value`, `placeholder`, `keyboardType` (`Text`, `Number`, `Email`, `Phone`, `Url`; `keyboard` remains an alias), `isSecure` (`secure` remains an alias), `autofill` (`Username`, `Password`, `OneTimeCode`), `returnKeyType` (`Done`, `Search`, `Send`, `Next`), `multiline`, `autocorrect`, `capitalization` (`None`, `Characters`, `Words`, `Sentences`), `focused`, positive literal `maxLength`, `font` (`Body` or `Title3`), and optional positive `minLines` / `maxLines` for multiline fields. The optional trailing action block handles the selected return key for single-line fields. `Keyboard.dismiss()` hides the software keyboard.
- `searchable: true` adds a search affordance and search keyboard action to a `TextInput`. For a navigation search field, bind the query on the `Tab` declaration instead; iOS uses SwiftUI `.searchable`, while Android uses a native Material text field.

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
Native menu-style single-choice control for a set of string options. `selected` is a mutable `String` state bound to the active option. Optional `label` and `icon` arguments provide its native row label; the icon name is shared between iOS and Android.

```nexa
app SizePicker {
    state sizes: Array<String> = ["Small", "Medium", "Large"]
    state selectedSize: String = "Medium"

    body {
        Picker(items: sizes, selected: selectedSize, label: "Size", icon: "star")
        Text(selectedSize)
    }
}
```
- **Properties**: `items: Array<String>`, `selected: String` (mutable state binding), optional `label: String`, and optional `icon: shared system icon name`.
- Option labels should be distinct; the selected value should match one of the labels.

### `DatePicker`
Native calendar date picker with an optional time picker. `timestamp` binds an
`Int64` containing milliseconds since the Unix epoch, and `hasTime` binds a
mutable `Bool`.

```nexa
state dueAt: Int64 = Time.now()
state includesTime: Bool = false

DatePicker(timestamp: dueAt, hasTime: includesTime)
```

---

### `Pressable`
Gesture wrapper detecting taps, double-taps, long presses, drag updates, and pinch scale changes.

```nexa
app PressableExample {
    state taps: Int32 = 0
    state dragOffset: Float64 = 0.0
    state zoom: Float64 = 1.0

    body {
        Pressable(disabled: false, haptic: Medium, fillMaxSize: true) {
            Column {
                Text("Tap, drag, or pinch me")
                Text("Horizontal offset: $dragOffset")
            }
        }.onTap {
            taps = taps + 1
        }.onDoubleTap {
            taps = taps + 2
        }.onLongPress(durationMs: 700) {
            taps = 0
        }.contextMenu {
            Button("Reset") {
                taps = 0
            }
        }.onDrag { translationX, translationY, velocityX, velocityY ->
            dragOffset = translationX
        }.onPinch { scaleFactor ->
            zoom = zoom * scaleFactor
        }
    }
}
```
- **Properties**: `disabled: Bool`, optional `haptic: Light | Medium | Heavy`, and `fillMaxSize: Bool` (defaults to `false`). Set `fillMaxSize: true` when the pressable's hit area should expand to its parent's available size.
- **Modifiers**: `.onTap { ... }`, `.onDoubleTap { ... }`, `.onLongPress(durationMs: 700) { ... }`, `.contextMenu { Button(...) { ... } }` for native long-press menu actions, `.onDrag { translationX, translationY, velocityX, velocityY -> ... }`, and `.onPinch { scaleFactor -> ... }`.
- Android pressables and Material controls do not draw a ripple by default; tap actions and haptics still work.
- Long-press duration defaults to 500 ms. Runtime values are clamped to at least 1 ms for consistent platform behavior.
- Context menus are native SwiftUI context menus on iOS and Material dropdown menus opened by long-press on Android. Their contents currently accept `Button` actions.
- Drag translations use points on iOS and dp on Android; velocities use those same units per second.
- `scaleFactor` is the multiplicative scale delta for the current gesture update. Multiply the current zoom by it; Android and iOS report the same incremental semantics.

---

### `Image`
Renders local bundle assets, HTTPS images, or app-local image files with scaling and placeholders.

```nexa
app ImageExample {
    body {
        Column {
            Image(asset: "logo", description: "App Logo", scale: Fit)
            Image(url: "https://example.com/pic.png", description: "Avatar", scale: Fill, placeholder: "avatar_ph")
            Image(file: photoUri, description: "Saved photo", maxHeight: 200)
        }
    }
}
```
- **Properties**: exactly one of `asset: String`, `url: String` (HTTPS only), or `file: String` (local file URI); `description: String`, `scale: Fit | Fill`, optional remote `placeholder: String`, and optional positive `maxHeight`.

Put app-owned image files directly in `assets/images/` and reference each file by its lowercase filename without an extension. For example, `assets/images/hero_banner.png` is `Image(asset: "hero_banner", ...)`. Nexa copies these files into the iOS asset catalog and Android `drawable-nodpi` resources when it generates the native projects. Supported formats are PNG, JPG/JPEG, and WebP. Asset names must start with a lowercase letter and contain only lowercase letters, digits, and underscores.

Use `.sharedElement(id: ...)` on matching images in two navigation screens to animate the image between destinations. Both images must evaluate to the same string identifier:

```nexa
Image(asset: "product", description: "Product")
    .sharedElement(id: "product-42")
```

This maps to SwiftUI matched geometry on iOS and Compose shared transitions on Android. It also works in `nexa dev` hot reload; the development host already includes the native transition support.

### `Icon`
Renders a native system icon: an SF Symbol on iOS and a Compose Material icon on Android. `system` uses a shared semantic name; `sfsymbol` and `materialsymbol` are escape hatches for platform-specific names.

```nexa
Icon(system: "favorite_filled", description: "Like", size: 30, tint: "#FFFFFF")
Icon(sfsymbol: "person.crop.circle.fill", description: "Profile", size: 30, tint: "#FFFFFF")
Icon(materialsymbol: "outlined:account_circle", description: "Profile", size: 30, tint: "#FFFFFF")
```

`system` accepts only the portable semantic names listed in the [shared system icon catalog](system-icons.md). Use `sfsymbol` or `materialsymbol` for platform-specific names. Unknown shared names are compile errors.

Use `sfsymbol` for an iOS-only SF Symbol and `materialsymbol` for an Android-only Compose icon. Material names default to the `Filled` family; prefix with `outlined:`, `rounded:`, `sharp:`, `twoTone:`, `autoMirroredFilled:`, or `autoMirroredOutlined:` to select a family. `description` is exposed to accessibility, `size` is a positive point/dp value up to 512, and `tint` accepts a hexadecimal color or a `String` state holding a hexadecimal color.

### `LinearGradient`
Draws a native linear gradient with two static colors. Use it as a child of a `Stack` to shade content without intercepting taps.

```nexa
Stack {
    LinearGradient(startColor: "#CC000000", endColor: "#00000000", direction: TopToBottom, height: 180)
}
```

- **Properties**: `startColor`, `endColor`, optional `direction` (`TopToBottom`, `BottomToTop`, `LeadingToTrailing`, or `TrailingToLeading`), and optional `height` (defaults to `180`).
- Native rendering uses SwiftUI `LinearGradient` and Compose gradient brushes.

---

### `AppBottomBar`
Bottom tab navigation bar hosting multiple destinations with reactive tab switching.

```nexa
app TabExample {
    state currentTab: Int32 = 0

    body {
        AppBottomBar(selected: currentTab) {
            Tab(index: 0, label: "Home", icon: "home", comment: "Primary navigation destination") {
                Column { Text("Home") }
            }
            Tab(index: 1, label: "Search", icon: "search") {
                Column { Text("Search") }
            }
            Tab(index: 2, label: "Profile", icon: "person", badge: "3") {
                Column { Text("Profile") }
            }
        }
    }
}
```
- **Properties**: `selected: Int32` (state binding) and optional `tint`, which accepts a hexadecimal color or a `String` state holding one.
- **Tab Declarations**: `Tab(index: Int32, label: String, icon: String?, badge: String?, role: "search"?, comment: String?, title: String?, largeTitle: Bool?, searchable: String?, searchPrompt: String?) { ... }`. The label, optional navigation title, and search prompt are localized source strings; `comment` provides translator context. `largeTitle: true` opts into iOS's native large title mode, including `.inlineLarge` on iOS 26+ and the large navigation bar title on earlier supported releases. `searchable` accepts a mutable `String` state and requires `role: "search"`; iOS attaches a native SwiftUI search field to the destination, and Android renders a Material text field above the destination content. On iOS 18 and later, the native sidebar-adaptable style presents the tab bar on iPhone and adapts it to iPad; earlier iOS versions use the native `TabView` tab bar.
- Compiles to native SwiftUI `TabView` on iOS and Material 3 `NavigationSuiteScaffold` on Android, which adapts between bottom navigation and wider-window rail/drawer layouts. Per-tab Android content uses saveable state so switching destinations preserves supported screen and navigation state. The tab label is used only in the navigation component; each destination owns its screen content and title.
- `role: "search"` opts an iOS tab into SwiftUI's native search-tab presentation and activation. Android renders it as a regular adaptive navigation destination.

### `ConfirmationDialog`
Native confirmation choices bound to mutable Boolean state. SwiftUI uses `.confirmationDialog`; Android uses a Material alert dialog.

```nexa
ConfirmationDialog(isPresented: showingPriority, title: "Select Priority") {
    Button("High") { priority = "High" }
    Button("Normal") { priority = "Normal" }
    Button("Cancel") { showingPriority = false }
}
```

- **Properties**: `isPresented: Bool` (state binding), `title: String`, and optional translator `comment`.
- **Children**: one or more native `Button` actions.

### `PagePager`
Swipeable, app-authored pages with a page indicator. Use this for onboarding and other linear page flows; use `AppBottomBar` for destination navigation.

```nexa
app OnboardingPages {
    state currentPage: Int32 = 0

    body {
        PagePager(selected: currentPage) {
            Tab(index: 0) {
                Column { Text("Welcome") }
            }
            Tab(index: 1) {
                Column { Text("You're all set") }
            }
        }
    }
}
```

- **Properties**: `selected: Int32` (mutable state binding).
- **Pages**: `Tab(index: Int32) { ... }`; indexes must be unique, non-negative, and contiguous from zero. The tab body owns its page content. Swipe gestures update `selected`, and changing `selected` animates to that page.
- iOS uses SwiftUI `TabView` with the native page indicator. Android uses Compose `HorizontalPager` with page dots. `PagePager` does not display a bottom navigation bar or tab labels.
- See [`examples/onboarding.nx`](../examples/onboarding.nx) for a complete welcome, notification-permission, and completion flow. Its visible copy is written directly as text literals.

### `Toolbar`
Places actions in the native navigation toolbar on iOS and in a top action row on Android.

```nexa
Toolbar(placement: Trailing) {
    Button("", icon: "add", accessibilityLabel: "Add") { }
}
```
- **Properties**: `placement` is `Leading` or `Trailing` and defaults to `Trailing`.
- The toolbar is hosted by the nearest navigation screen. iOS uses SwiftUI `ToolbarItemGroup`.

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

### `NavigationSplitView`
Adaptive master-detail navigation for iPhone, iPad, Android tablets, and foldables. iOS uses native SwiftUI `NavigationSplitView`, which adapts its columns to the current window. Android shows both columns when the current Compose window is at least 840 dp wide and switches between sidebar and detail on compact windows; unfolding a foldable updates the layout from the available width. The system Back action returns to the sidebar.

```nexa
app MailExample {
    state showDetail: Bool = false

    body {
        NavigationSplitView(detailVisible: showDetail) {
            Sidebar {
                Text("Messages")
                Button(label: "Open message") {
                    showDetail = true
                }
            }
            Detail {
                Text("Message detail")
            }
        }
    }
}
```
- **`detailVisible: Bool`**: Mutable state that identifies the compact-width column; selection controls remain app-authored.

Use this layout when the same content should become a two-pane workspace on a tablet or an unfolded foldable, and a single-column navigation flow on a phone. The layout responds to window width, so it also adapts when the device rotates or a foldable changes posture. See [`adaptive_workspace.nx`](../examples/adaptive_workspace.nx) for a complete example.
- **`Sidebar { ... }`** and **`Detail { ... }`**: Exactly one node block for each pane.

### Responsive layouts

Use `Layout.isRegularWidth`, `Layout.isCompactWidth`, `Layout.isRegularHeight`, and `Layout.isCompactHeight` when the composition itself should change with the available window size. These values update as the window resizes or rotates, so they work for phones, tablets, desktop-sized windows, and folded or unfolded foldables. Prefer adapting to available width over checking a physical device type: a phone in landscape may have less room than a tablet in split screen.

```nexa
app ResponsiveContent {
    body {
        if Layout.isRegularWidth {
            Row(spacing: 24, padding: 24) {
                Column(width: 280) {
                    Text("Navigation")
                }
                Column {
                    Text("Main content")
                }
            }
        } else {
            Column(spacing: 16, padding: 16) {
                Text("Navigation")
                Text("Main content")
            }
        }
    }
}
```

On iOS these predicates follow SwiftUI horizontal and vertical size classes. On Android, Nexa evaluates the current Compose window width and height in dp against the 600 dp compact/regular boundary. Use `NavigationSplitView` when you need standard master-detail navigation; use the predicates for app-specific responsive composition.

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
- **Properties**: `isPresented: Bool` (state binding), `partial: Bool`, and optional `title: String`.
- A titled partial sheet uses an inline SwiftUI navigation title, native sheet detents and drag indicator on iOS; Android shows a Material title row. `Toolbar` children in the sheet become iOS toolbar actions and Android action rows.

### `Dialog`
Native modal alert controlled by mutable Boolean state. The title and message accept `String` expressions. Children may contain button actions and, optionally, one single-line `TextInput` for native text-entry alerts. Nexa emits an iOS alert text field and an Android Material alert field; multiline, secure, searchable, or submit-action text fields are rejected in this context.

```nexa
app DialogExample {
    state showConfirmation: Bool = false
    state itemCount: Int32 = 1
    state comment: String = ""

    body {
        Button("Delete") { showConfirmation = true }
        Dialog(
            isPresented: showConfirmation,
            title: "Delete item?",
            message: ""
        ) {
            TextInput(value: comment, placeholder: "Comment")
            Button("Save") { showConfirmation = false }
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
- **Children**: native button actions and at most one plain, single-line `TextInput`. Buttons close the dialog by setting its binding to `false`.

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
                TextInput(value: password, placeholder: "Password", isSecure: true)
            }
        }
    }
}
```
- **Properties**: `dismiss: Interactive | Never`.

---

### `Appearance`

Wraps content in a native appearance override. `mode` accepts `"system"`, `"light"`, or `"dark"`; a `String` state can change it at runtime. `system` follows the device setting. On iOS this uses SwiftUI's preferred color scheme; Android supplies the matching Material 3 color scheme.

```nexa
state appearance: String = "system"

Appearance(mode: appearance) {
    Text("The app follows the selected appearance")
}
```

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

### Accessibility options
Add screen reader semantics directly to a visual component, custom component, or qualified native plugin component.

```nexa
app AccessibilityExample {
    body {
        Image(asset: "close_icon", description: "Close", accessibilityLabel: "Close dialog", accessibilityHint: "Closes this dialog", accessibilityRole: Button)
        Text("Profile", accessibilityLabel: "Profile heading", accessibilityRole: Header)
    }
}
```
- **Properties**: optional `accessibilityLabel: String`, `accessibilityHint: String`, `accessibilityValue: String`, and `accessibilityRole: None | Button | Link | Header | Image`.
- `accessibilityLabel` is required whenever any accessibility option is provided. Labels, hints, and values accept typed `String` expressions; literal labels and hints must be non-empty. Nexa emits accessibility values separately where the native API supports them.
- Swift emits native accessibility modifiers. Android includes hints in the Compose content description because the framework's pinned stable Compose version has no general hint semantic; hints are not exposed as the control's state. Both platforms emit heading semantics for `Header`.

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
