# Nexa language: first slice

The compiler currently supports stateful native screens, typed arrays, virtualized lists, and a parameterless native navigation stack. A source file contains one `app`, optional state and screen declarations, and a `body` containing native-mappable components.

```nexa
app Counter {
    state count: Int32 = 0
    state email: String = ""
    state enabled: Bool = false

    body {
        Column(spacing: 12, padding: 20, background: "#F4F5F7", cornerRadius: 16) {
            Text("Tap count")
            Text(count)
            TextInput(value: email, placeholder: "Email", keyboard: Email)
            Switch(value: enabled, label: "Notifications")
            Image(asset: "nexa_mark", description: "Nexa", scale: Fit)
            Image(url: "https://example.com/avatar.jpg", description: "Profile photo", scale: Fill)
            Button("Increment") {
                count = count + 1
            }
        }
    }
}
```

Run `nexa check file.nx` to parse and type-check a source file. Run `nexa build file.nx --target swift|kotlin` to generate native source.

## Types and state

State declarations use an explicit type. `state` is mutable and can be assigned in a button handler or bound to a control; `let` declares an immutable value. Supported scalar types are `String`, `Bool`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Float32`, and `Float64`; arrays can hold these values or other arrays.

`Array<T>` is also supported for those primitive types. Array literals use `[value, ...]`, including `[]` when the state declaration supplies the element type, for example `let labels: Array<String> = ["Nexa", "SwiftUI"]`.

Numeric literal values are checked against the declared type, including signed negative literals. Numeric variables do not implicitly convert between types. In this slice, `+` accepts operands of one identical numeric type. Integer addition wraps on overflow; floating-point addition follows the native IEEE arithmetic behavior.

Immutable `let` initializers can refer to earlier declarations. Mutable state initializers must be literal expressions in this version. Forward references, nullable types, user-defined types, functions, generics beyond the built-in `Array<T>`, and async expressions are not implemented yet.

## Components

- `View { ... }` and `Column { ... }` map to a native vertical stack.
- `Row { ... }` maps to a native horizontal stack.
- `Column(spacing: 12) { ... }` and `Row(spacing: 12) { ... }` map spacing directly to platform layout parameters.
- Layouts accept static `padding`, `width`, `height`, `background`, `cornerRadius`, and `opacity` options. Dimensions use points on iOS and density-independent pixels on Android. Backgrounds accept `#RGB`, `#RRGGBB`, or `#RRGGBBAA`; opacity must be from `0` to `1`.
- `Text(expression)` accepts strings, booleans, and numeric values.
- `Button("Label") { ... }` accepts a string label and state assignments in its press handler.
- `TextInput(value: name, placeholder: "...", keyboard: Email)` binds a mutable `String` state to a native text field. Keyboard choices are `Text`, `Number`, `Email`, `Phone`, and `Url`; `secure: true` and `multiline: true` are optional. `autocorrect` optionally enables or disables keyboard suggestions, while `capitalization` accepts `None`, `Sentences`, `Words`, or `Characters`. Omitted keyboard options retain native defaults. Secure multiline input is rejected because the native APIs differ.
- `Switch(value: enabled, label: "...")` binds a mutable `Bool` state to the native switch control.
- `Image(asset: "nexa_mark", description: "...", scale: Fit)` loads a local platform drawable/asset. `Image(url: "https://...", description: "...", scale: Fill, placeholder: "avatar_loading")` loads a remote image; only absolute HTTPS URLs are accepted. Scale choices are `Fit` and `Fill`, `placeholder` is an optional local drawable/asset shown while loading or if the request fails, and an empty description marks a decorative image.
- `Pressable(disabled: false) { ... } { ... }` wraps composed content in a native accessible press target. Its second block contains state assignments; `disabled` is optional.
- `KeyboardAware { ... }` places content in a native scroll container that adjusts around the software keyboard. SwiftUI uses interactive scroll-to-dismiss behavior; Compose applies animated IME inset padding and native vertical scrolling.
- `screen Home { ... }` declares a named destination. An app with screens uses `body { NavigationStack(root: Home) }`; `NavigationLink(destination: Profile) { ... }` pushes a statically resolved screen. All declared screens share the app's state. Destinations must be declared in the same app.
- `FastList(count: rowCount, index: row) { ... }` creates a virtualized, vertical list of `Int32` row indices. `index` is optional and defaults to `index`; row content is emitted only for visible items by SwiftUI `List` or Compose `LazyColumn`. The count can be an `Int32` state value; negative runtime counts are treated as zero.
- `FastList(items: labels, index: row, item: label) { ... }` creates a virtualized list over an `Array<T>` state. Both bindings are optional and default to `index` and `item`; the index is read-only `Int32`, and the item is a read-only value of the array's element type. Rows use their current position as identity, and the collection is read directly without building an intermediate row tree. See [collection-list.nx](../examples/collection-list.nx).

An app with several top-level body components is placed in a vertical native stack. `Text` and `Button` are leaves in this version.

## Generated code

The Swift backend emits a SwiftUI `View`, stores mutable values in `@State`, and maps controls to `VStack`, `HStack`, `List`, `Text`, `TextField`/`SecureField`, `Toggle`, `Image`/`AsyncImage`, `NavigationStack`/`NavigationLink`, and `Button`. The Kotlin backend emits a `@Composable` function and maps controls to Compose `Column`, `Row`, `LazyColumn`, `Text`, `TextField`, `Switch`, Coil 3 `AsyncImage`, AndroidX Navigation Compose `NavHost`, `Button`, and a `clickable` layout for `Pressable`. Text input and switch values bind directly to native state callbacks. The compiler generates source code rather than a runtime component tree; generated files join an existing native app target.

Android apps that use `Image` need [Coil 3 Compose](https://coil-kt.github.io/coil/compose/). Remote URLs also need Coil's [OkHttp network module](https://coil-kt.github.io/coil/network/) and Android's Internet permission. For example, add `implementation("io.coil-kt.coil3:coil-compose:3.6.3")`, and for remote images add `implementation("io.coil-kt.coil3:coil-network-okhttp:3.6.3")` plus `<uses-permission android:name="android.permission.INTERNET" />` to the manifest. Apps with navigation need `implementation("androidx.navigation:navigation-compose:<compatible-version>")`. When using `TextInput(autocorrect: ...)`, use a Compose UI version that supports `KeyboardOptions.autoCorrectEnabled` (1.7.0 or newer). `KeyboardAware` requires the Activity to use `android:windowSoftInputMode="adjustResize"` so Compose receives IME insets. Swift remote images use SwiftUI `AsyncImage` and require an iOS 15 or newer deployment target; SwiftUI navigation and `KeyboardAware` use iOS 16 or newer.

## Current boundaries

This prototype does not yet generate full Xcode or Gradle projects. Navigation currently supports parameterless screens, a single stack, and compile-time checked links; typed route parameters, tabs, deep links, modals, and navigation guards remain future work. `FastList` supports integer ranges and primitive `Array<T>` collections. Custom stable row keys, mutable element-level bindings, paging, grids, horizontal lists, scroll controls, and refresh integration remain future work. `KeyboardAware` handles basic scrolling and inset adjustment, but keyboard height/events, explicit focus management, and programmatic dismissal remain future work. Remote image loading uses the platform loaders' default request, cache, and decoding behavior; custom image-loader configuration and observable loading/error state are not exposed yet. `Pressable` currently supports press and disabled state; long press, hover, pressed-state styling, focus, and haptics remain future work. Text input selection/autofill, themes, plugins, FFI bindings, incremental compilation, formatting, language-server support, and optimization passes also remain on the roadmap in `plan.md`.
