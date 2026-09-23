# Component & Layout Reference

Nexa compiles declarative layout trees directly into native SwiftUI views and Jetpack Compose composables.

---

## 1. Layout Containers

### `Column`
Arranges child elements vertically.

```nexa
Column(spacing: 16, alignment: Start, padding: 12) {
    Text("Header")
    Text("Subheader")
}
```
- **Swift**: Emitted as `VStack(alignment: .leading, spacing: 16) { ... }`.
- **Kotlin**: Emitted as `Column(verticalArrangement = Arrangement.spacedBy(16.dp)) { ... }`.

### `Row`
Arranges child elements horizontally.

```nexa
Row(spacing: 8, alignment: Center) {
    Text("Label:")
    Text("Value")
}
```
- **Swift**: Emitted as `HStack(spacing: 8) { ... }`.
- **Kotlin**: Emitted as `Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) { ... }`.

### `Stack`
Layers elements on top of each other along the Z-axis.

```nexa
Stack(alignment: Center) {
    Image(asset: "background")
    Text("Overlaid Title")
}
```
- **Swift**: Emitted as `ZStack { ... }`.
- **Kotlin**: Emitted as `Box { ... }`.

---

## 2. Interactive Native Controls

### `Button`
Standard platform button triggering state mutations.

```nexa
Button("Submit") {
    count = count + 1
}
```

### `TextInput`
Single or multi-line text input field.

```nexa
TextInput(
    value: email,
    placeholder: "user@example.com",
    keyboard: Email,
    autocorrect: false,
    capitalization: None
)
```

### `Switch` / `Toggle`
Boolean toggle switch.

```nexa
Switch(value: isEnabled, label: "Enable Notifications")
```

### `Pressable`
Interactive wrapper detecting tap and press interactions with gesture feedback.

```nexa
Pressable(disabled: false) {
    Card {
        Text("Tap me!")
    }
}.onPress {
    taps = taps + 1
}
```

---

## 3. Custom Reusable Components

Define custom components outside `app`:

```nexa
component UserBadge(name: String, role: String) {
    body {
        Row(spacing: 8) {
            Text(name)
            Text("(${role})")
        }
    }
}

app MainApp {
    body {
        Column {
            UserBadge(name: "Alice", role: "Admin")
            UserBadge(name: "Bob", role: "Member")
        }
    }
}
```
