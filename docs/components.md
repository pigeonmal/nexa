# Component Reference

Every native UI component in Nexa compiles Ahead-Of-Time into specialized SwiftUI views on iOS and Jetpack Compose composables on Android.

---

### `Column`
Arranges children vertically with optional spacing, alignment, padding, and borders.

```nexa
Column(spacing: 12, alignment: Start, padding: 16, background: "#FFFFFF", cornerRadius: 8) {
    Text("Title")
    Text("Subtitle")
}
```
- **Properties**: `spacing`, `alignment: Start | Center | End`, `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, `opacity`.

---

### `Row`
Arranges children horizontally with optional spacing, alignment, and styling.

```nexa
Row(spacing: 8, alignment: Center) {
    Text("Status:")
    Text("Active")
}
```
- **Properties**: Accepts the same layout and border properties as `Column`.

---

### `Stack`
Layers children along the Z-axis (overlays).

```nexa
Stack(alignment: Center) {
    Image(asset: "hero_banner", description: "Banner", scale: Fill)
    Text("Overlay Title")
}
```
- **Properties**: `alignment: Start | Center | End`, along with sizing and background modifiers.

---

### `FastList`
High-performance virtualized list with zero `AnyView` type erasure and unboxed scroll state. Supports dynamic collections, count-based ranges, sectioned groups, sticky headers, and pagination.

```nexa
// Dynamic collection with sticky header and infinite scroll pagination
FastList(items, axis: Vertical, key: .self, rowHeight: 52, scrollPosition: scrollPos) { item, index in
    Text("#${index}: ${item}")
}.stickyHeader {
    Text("Category Header")
}.onEndReached {
    loadNextPage()
}.onScroll {
    onScrolled()
}

// Or count-based: FastList(count: 100000, rowHeight: 48) { index in ... }
// Or sectioned:   FastList(sections: groups, key: .self, rowHeight: 48) { item, itemIndex, sectionIndex in ... }
```
- **Sources**: `FastList(collection, ...)` (collection), `count: Int32` (virtual sequence), or `sections: Array<Array<T>>` (sectioned).
- **Options**: `axis: Vertical | Horizontal`, `rowHeight: Float`, `key: .self | .id`, `scrollPosition: stateVar`.
- **Modifiers**: `.stickyHeader { ... }`, `.sectionHeader { ... }`, `.onEndReached { ... }`, `.onScroll { ... }`.

---

### `Text`
Renders formatted text with typography, color, and wrapping controls.

```nexa
Text("Hello Nexa", color: "#333333", fontSize: 18, fontWeight: Bold, lineLimit: 2, selectable: true)
```
- **Properties**: `value` (positional), `color`, `fontSize`, `fontWeight`, `lineLimit`, `lineHeight`, `letterSpacing`, `selectable`.

---

### `Button`
Native platform button triggering state mutations or actions.

```nexa
Button("Save Record", icon: "checkmark", loading: isSaving, disabled: false) {
    saveRecord()
}
```
- **Properties**: `label` (positional), `icon: String?`, `loading: Bool?`, `disabled: Bool?`, and a trailing action block.

---

### `TextInput`
Single or multi-line text field with software keyboard integration.

```nexa
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
    submitForm()
}
```
- **Properties**: `value`, `placeholder`, `keyboard` (`Default`, `Email`, `Numeric`, `Phone`, `Url`), `secure`, `multiline`, `autocorrect`, `capitalization` (`None`, `Characters`, `Words`, `Sentences`), `focused`, `maxLength`, and trailing onSubmit action.

---

### `Switch`
Platform toggle switch for boolean state.

```nexa
Switch(value: isEnabled, label: "Enable Notifications")
```
- **Properties**: `value: Bool` (two-way binding), `label: String`.

---

### `Pressable`
Gesture wrapper detecting tap and long-press interactions with hardware haptics.

```nexa
Pressable(disabled: false, haptic: Medium) {
    Text("Tap me!")
}.onPress {
    taps = taps + 1
}.onLongPress {
    taps = 0
}
```
- **Properties**: `disabled: Bool`, `haptic: Selection | Light | Medium | Heavy | Success | Warning | Error`.
- **Modifiers**: `.onPress { ... }`, `.onLongPress { ... }`.

---

### `Image`
Renders local bundle assets or remote network images with scaling and placeholders.

```nexa
// Local asset:
Image(asset: "logo", description: "App Logo", scale: Fit)

// Remote URL:
Image(url: "https://example.com/pic.png", description: "Avatar", scale: Fill, placeholder: "avatar_ph")
```
- **Properties**: `asset: String` OR `url: String`, `description: String`, `scale: Fit | Fill`, `placeholder: String`.

---

### `AppBottomBar`
Bottom tab navigation bar hosting multiple destinations with reactive tab switching.

```nexa
AppBottomBar(selected: currentTab) {
    Tab(index: 0, label: "Home", icon: "house") {
        HomeScreen()
    }
    Tab(index: 1, label: "Search", icon: "magnifyingglass") {
        SearchScreen()
    }
    Tab(index: 2, label: "Profile", icon: "person", badge: "3") {
        ProfileScreen()
    }
}
```
- **Properties**: `selected: Int32` (state binding).
- **Tab Declarations**: `Tab(index: Int, label: String, icon: String?, badge: String?) { ... }`.
- Compiles to native `TabView` on iOS and `NavigationBar` on Android.

---

### `NavigationStack` & `NavigationLink`
Hierarchical screen stack navigation with push transitions and swipe-to-back gestures.

```nexa
NavigationStack(root: Home)

// Inside a screen:
NavigationLink(destination: Details, when: isAllowed) {
    Text("View Details")
}

// Inside destination screen:
NavigationBack(label: "Go Back")
```
- **`NavigationStack(root: ScreenName)`**: Container hosting the stack.
- **`NavigationLink(destination: ScreenName, when: Bool?)`**: Push transition.
- **`NavigationBack(label: String?)`**: Back button.

---

### `BottomSheet`
Modal presentation sheet appearing from the bottom of the screen.

```nexa
BottomSheet(isPresented: showModal, partial: true) {
    Column(padding: 20) {
        Text("Modal Content")
        Button("Close") { showModal = false }
    }
}
```
- **Properties**: `isPresented: Bool` (state binding), `partial: Bool`.

---

### `RefreshControl`
Pull-to-refresh wrapper for scrollable containers.

```nexa
RefreshControl(isRefreshing: isRefreshing).onRefresh {
    reloadFeed()
} {
    FastList(posts) { post in ... }
}
```
- **Properties**: `isRefreshing: Bool`.
- **Modifiers**: `.onRefresh { ... }`.

---

### `KeyboardAware`
Automatically scrolls and insets content when the software keyboard appears.

```nexa
KeyboardAware(dismiss: Interactive) {
    Column {
        TextInput(value: username, placeholder: "Username")
        TextInput(value: password, placeholder: "Password", secure: true)
    }
}
```
- **Properties**: `dismiss: Interactive | OnDrag | Tap`.

---

### `StatusBar`
Configures system status bar styling per screen.

```nexa
StatusBar(style: Light, hidden: false, background: "#000000")
```
- **Properties**: `style: Light | Dark`, `hidden: Bool`, `background: Color`.

---

### `Direction`
Overrides text and layout directionality for localization.

```nexa
Direction(value: RTL) {
    Column { Text("Arabic or Hebrew localized UI") }
}
```
- **Properties**: `value: LTR | RTL`.

---

### `Accessibility`
Screen reader annotations for VoiceOver (iOS) and TalkBack (Android).

```nexa
Accessibility(label: "Close dialog", hint: "Discards changes", role: Button) {
    Image(asset: "close_icon", description: "Close")
}
```
- **Properties**: `label: String`, `hint: String?`, `role: Button | Header | Image | Link | Search | Summary | Tab`.

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
```
- Call with: `Card(title: "Summary") { Text("Child text") }`.
