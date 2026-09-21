# Nexa language: first slice

The compiler currently supports stateful native screens, typed arrays and collection values, virtualized lists, conditional UI and event control flow, compile-time platform blocks, a parameterless native navigation stack, stateful user-defined components, and a feature-gated native network/file library. The entry `.nx` file declares one `app`; imported component files can declare reusable components without an app.

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

`state` is mutable. Both `state` and `let` can use an explicit type or infer it at compile time from a non-empty initializer. Inference uses the same deterministic defaults on both platforms: integer literals become `Int32`, decimal literals become `Float64`, and the type of a non-empty value expression is propagated. The inferred type is written into the typed IR before native generation, so bindings remain predictable. Supported scalar types are `String`, `Bool`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Float32`, and `Float64`. Generic value types can be nested, including arrays, sets, maps, pairs, and triples.

The supported generic types are `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>`. Literals are checked against the declared or parameter type; Nexa does not infer generic element types from an untyped literal:

```nexa
let labels: Array<String> = ["Nexa", "SwiftUI"]
let tags: Set<String> = ["mobile", "compiler", "mobile"]
let versions: Map<String, Int32> = ["iOS": 16, "Android": 26]
let selected: Pair<String, Int32> = Pair("Nexa", 3)
let coordinates: Triple<Float32, Float32, Float32> = Triple(10.5, 20.0, 0.0)

// Mutable and immutable values can omit the type when the initializer is non-empty and unambiguous.
state retryCount = 3              // inferred as Int32
let title = "Nexa"
let retries = 3
let releaseMap = ["iOS": 16, "Android": 26]
let selectedPlatform = Pair("Nexa", 3)
```

String literals can include a state name with either `$name` or `\(name)`. Interpolation is resolved during semantic lowering and becomes native Swift or Kotlin string interpolation; it does not add a runtime template engine. The embedded value must be a declared state or constant name, and it keeps that value's native type:

```nexa
state count = 3
let title = "Nexa"

body {
    Text("$title: \(count)")
    Button("Increment $count") {
        count = count + 1
    }
}
```

The bracket literal is contextual: `[value, ...]` creates an `Array<T>` or `Set<T>` according to the expected type. A non-empty array literal can infer an `Array<T>` for `let`; a set still needs an explicit `Set<T>` annotation because the source spelling is shared with arrays. Empty arrays, sets, and maps also need explicit types. Map literals use `[key: value, ...]`; the empty map literal is `[:]`. If a map literal contains the same runtime key more than once, the last value wins on both platforms. Set elements are deduplicated. Neither set nor map iteration order is guaranteed across platforms.

`Pair(a, b)` and `Triple(a, b, c)` construct fixed-size ordered values. Their type arguments are checked position by position. Swift output uses native tuples for pairs and triples; Kotlin output uses the standard-library `Pair` and `Triple` classes, which are ordinary objects on Android/JVM. The compiler adds no Nexa-specific collection runtime or wrapper types. Arrays map to Swift `Array<T>` and Kotlin read-only `List<T>`, sets to Swift `Set<T>` and Kotlin read-only `Set<T>`, and maps to Swift `Dictionary<K, V>` and Kotlin read-only `Map<K, V>`.

For compatible hashing behavior on both platforms, `Set<T>` elements and `Map<K, V>` keys must be scalar `String`, `Bool`, or numeric values. Map values and pair/triple fields may use any supported type, including nested collection types. The language currently supports declaring, initializing, passing, assigning, and displaying these values. It does not yet support source-level indexing, key lookup, membership checks, insertion/removal, general iteration, transformation functions, or equality comparisons for collection and pair/triple values. `FastList(items: ...)` is the current way to render an `Array<T>` as repeated rows. See [collection-values.nx](../examples/collection-values.nx).

Numeric literal values are checked against the declared type, including signed negative literals. Numeric variables do not implicitly convert between types. `+` accepts operands of one numeric type; integer addition wraps on overflow and floating-point addition follows native IEEE behavior. A numeric literal takes its type from the other operand where possible, so `wideCount < 10` does not require a conversion.

Inferred `let` and `state` initializers can use the same expression forms as explicitly typed declarations. Mutable state initializers must still be independent of other state values in this version. Forward references, nullable types, user-defined types, functions, generics beyond the five built-in collection/value types, and async expressions are not implemented yet.

## Conditions and operators

`if` and `else` can choose UI content in a component body or control state assignments in a button or press handler:

```nexa
if enabled && count < 5 {
    Text("Keep going")
} else {
    Text("Paused")
}

Button("Toggle") {
    if !enabled || count == 0 {
        enabled = true
    } else {
        enabled = false
    }
}
```

Logical operators are `&&`, `||`, and `!`; equality is `==` and `!=`; numeric comparisons are `<`, `<=`, `>`, and `>=`. `&&` and `||` short-circuit. Equality works for `Bool`, numeric scalars, and `String`; ordering currently works for numeric scalars. Compared values must have the same type. Nexa does not insert numeric conversions. Conditions lower directly to Swift/Kotlin operators and native `if` branches. Pure literal conditions are folded by the compiler, and unreachable UI/event branches are removed before code generation. See [conditional-logic.nx](../examples/conditional-logic.nx) and [constant-branches.nx](../examples/constant-branches.nx).

`Layout.isRegularWidth` is a built-in `Bool` for choosing a wider layout in a conditional. Swift reads the native horizontal size class and checks for `.regular`; Compose checks whether the current configuration width is at least `600dp`. The platform definitions differ, so use this as a layout hint rather than a guarantee that both platforms classify every device identically. It follows configuration changes and adds no wrapper view:

```nexa
if Layout.isRegularWidth {
    Row(spacing: 16) {
        Text("Details")
        Text("More information")
    }
} else {
    Column(spacing: 8) {
        Text("Details")
        Text("More information")
    }
}
```

See [responsive-layout.nx](../examples/responsive-layout.nx).

## Compile-time platform blocks

Use `platform ios { ... }` or `platform android { ... }` when a widget or component call belongs to one native target:

```nexa
body {
    Text("Shared content")

    platform ios {
        IOSOnlyBadge()
    }

    platform android {
        AndroidOnlyBadge()
    }
}
```

The selected build target keeps its block and removes the other block before semantic lowering. This is a compile-time selection: there is no runtime platform check, inactive custom-component calls do not reach the target IR, and no unused native declaration is emitted. The supported target names are `ios` and `android`; using another name is a compiler error. `nexa check` validates both targets, while `nexa build --target swift|kotlin` lowers only the selected target. See [platform-widgets.nx](../examples/platform-widgets.nx).

## Components

- `Column { ... }` is the vertical container. It maps to SwiftUI `VStack` and Compose `Column` without an extra runtime wrapper.
- `StatusBar(style: Default|Light|Dark, hidden: true|false)` configures the app status bar once at the top level. The compiler removes the declaration before rendering and emits SwiftUI status-bar modifiers or direct Android system UI flags. `Light` and `Dark` describe the status-bar content; background and animated transitions remain future options.
- `View` is not a Nexa component. Replace existing `View { ... }` blocks with `Column { ... }`; the native SwiftUI `View` protocol in generated Swift remains unchanged.
- `Row { ... }` maps to a native horizontal stack.
- `Column(spacing: 12) { ... }` and `Row(spacing: 12) { ... }` map spacing directly to platform layout parameters.
- Layouts accept a cross-axis `alignment` of `Start`, `Center`, or `End`; it maps to horizontal alignment for `Column` and vertical alignment for `Row`. For vertical layouts, `Start`/`End` follow the layout direction; for rows they mean top/bottom. Layouts also accept `padding`, `width`, `height`, `background`, `cornerRadius`, and `opacity`. Dimensions use points on iOS and density-independent pixels on Android. Backgrounds accept `#RGB`, `#RRGGBB`, `#RRGGBBAA`, or a color theme token; opacity must be from `0` to `1`.
- `Text(expression, color: ..., fontSize: ...)` accepts strings, booleans, and numeric values. Text color accepts a hexadecimal color or theme color token. Font size uses points on iOS and scale-independent pixels on Android.
- `Button("Label") { ... }` accepts a string or interpolated label and state assignments in its press handler.
- `TextInput(value: name, placeholder: "...", keyboard: Email)` binds a mutable `String` state to a native text field. Keyboard choices are `Text`, `Number`, `Email`, `Phone`, and `Url`; `secure: true` and `multiline: true` are optional. `autocorrect` optionally enables or disables keyboard suggestions, while `capitalization` accepts `None`, `Sentences`, `Words`, or `Characters`. Omitted keyboard options retain native defaults. Secure multiline input is rejected because the native APIs differ.
- `Switch(value: enabled, label: "...")` binds a mutable `Bool` state to the native switch control.
- `Image(asset: "nexa_mark", description: "...", scale: Fit)` loads a local platform drawable/asset. `Image(url: "https://...", description: "...", scale: Fill, placeholder: "avatar_loading")` loads a remote image; only absolute HTTPS URLs are accepted. Scale choices are `Fit` and `Fill`, `placeholder` is an optional local drawable/asset shown while loading or if the request fails, and an empty description marks a decorative image. iOS remote images use `URLSession`; Android remote images use Coil 3 with a Cronet-backed `NetworkClient`, so Coil never falls back to OkHttp for Nexa image requests.

- `Pressable(disabled: false) { ... } { ... }` wraps composed content in a native accessible press target. Its second block contains state assignments; `disabled` is optional.
- `KeyboardAware { ... }` places content in a native scroll container that adjusts around the software keyboard. SwiftUI uses interactive scroll-to-dismiss behavior; Compose applies animated IME inset padding and native vertical scrolling.
- `screen Home { ... }` declares a named destination. An app with screens uses `body { NavigationStack(root: Home) }`; `NavigationLink(destination: Profile) { ... }` pushes a statically resolved screen. All declared screens share the app's state. Destinations must be declared in the same app.
- `FastList(count: rowCount, index: row) { ... }` creates a virtualized, vertical list of `Int32` row indices. `index` is optional and defaults to `index`; row content is produced for visible rows by iOS `UITableView` or Android Compose `LazyColumn`. The count can be an `Int32` state value; negative runtime counts are treated as zero.
- `FastList(items: labels, index: row, item: label) { ... }` creates a virtualized list over an `Array<T>` state. Both bindings are optional and default to `index` and `item`; the index is read-only `Int32`, and the item is a read-only value of the array's element type. Rows use their current position as identity, and the collection is indexed directly without building an intermediate row array or row tree. `Set<T>` and `Map<K, V>` are values but are not accepted as list sources. See [collection-list.nx](../examples/collection-list.nx).

## Native network and file library

When an app uses a remote image, the backend emits the small native `NexaNetwork`, `NexaPath`, and `NexaFile` library next to the generated screen. The same APIs are available to generated plugin bindings and future async Nexa expressions without a cross-platform request registry.

`NexaNetwork.fetch` accepts `url`, `method`, `body`, `headers`, `timeout`, `useCache`, `followRedirects`, `maxResponseBytes`, and optional certificate pins. `NexaNetwork.download` accepts the same request options plus `destinationPath` and streams the response to that path. Both APIs are asynchronous and return native errors for invalid URLs, non-2xx responses, cancellation, and size limits. Certificate pins are optional: iOS accepts lowercase SHA-256 hashes of the DER leaf certificate, while Android accepts lowercase SHA-256 public-key pins in the format expected by Cronet.

The iOS implementation configures `URLCache` with 16 MiB memory and 64 MiB disk capacity and uses `URLRequest.CachePolicy.useProtocolCachePolicy` by default. The Android implementation configures the Play Services Cronet engine with a 64 MiB disk cache, HTTP/2, QUIC, and Brotli. File reads and writes use native asynchronous APIs (`Task.detached` on iOS and `Dispatchers.IO` on Android). `NexaPath` provides `documents`, `caches`, `temporary`, `appSupport`, and `join` helpers.

The generated helpers are emitted only when the lowered module contains a remote image. This keeps offline apps free of networking imports and helper code; future async Nexa calls will reuse the same feature gate.

## Custom components and source files

Define reusable components with typed parameters, private state, and a body. Call them by name and provide each required parameter:

```nexa
component CounterButton(label: String) {
    state count: Int32 = 0

    body {
        Column {
            Text(label)
            Text(count)
            Button("Increment") {
                count = count + 1
            }
        }
    }
}

app Demo {
    body {
        CounterButton(label: "First")
        CounterButton(label: "Second")
    }
}
```

Each component instance gets its own native state (`@State` on iOS and Compose `remember` on Android). Components can use core components and call other custom components. Parameters are immutable typed inputs; state declared inside a component stays private to it.

Keep a component in its own `.nx` file and import it relative to the importing file with `import "components/CounterButton.nx"`. Imported files can import other component files; only the entry file declares an app. Imported components share one project-wide name scope. The compiler emits only components reachable from the app, with no runtime component registry. Recursive component composition, callback parameters, content slots, and `NavigationLink` inside a custom component are not supported yet. See [custom-components.nx](../examples/custom-components.nx) and its [component files](../examples/components/CounterButton.nx).

## Themes

An app can declare one compile-time `theme` block. Color tokens require both light and dark values and follow the system appearance. Spacing, radius, and font-size tokens hold static numeric values. Use `Theme.tokenName` only in matching style options: spacing for layout spacing and padding, radius for corner radius, colors for backgrounds and text, and font size for text.

```nexa
app ThemeExample {
    theme {
        color surface(light: "#FFFFFF", dark: "#101216")
        spacing page: 20
        radius card: 16
        fontSize body: 16
    }

    body {
        Column(padding: Theme.page, background: Theme.surface, cornerRadius: Theme.card) {
            Text("Welcome", fontSize: Theme.body)
        }
    }
}
```

These values resolve into typed IR during compilation. Swift output reads the native color-scheme environment only when an adaptive color is used. Compose output reads `isSystemInDarkTheme()` only when needed. User-selected palettes, theme switching controls, typography families and weights, shadows, and component-specific tokens are not supported yet. See [themed-app.nx](../examples/themed-app.nx).

An app or custom component with several top-level body nodes is placed in a vertical native stack. Built-in controls and custom-component calls are leaves.

## Generated code

The Swift backend emits a SwiftUI `View`, stores mutable values in `@State`, and maps layouts and controls to `VStack`, `HStack`, `Text`, `TextField`/`SecureField`, `Toggle`, `Image`, the generated URLSession-backed remote image view, `NavigationStack`/`NavigationLink`, and `Button`. `FastList` is backed by `UITableView` with reusable cells and SwiftUI row content hosted by [`UIHostingConfiguration`](https://developer.apple.com/documentation/swiftui/uihostingconfiguration). The Kotlin backend emits a `@Composable` function and maps controls to Compose `Column`, `Row`, `LazyColumn`, `Text`, `TextField`, `Switch`, Coil 3 `AsyncImage`, AndroidX Navigation Compose `NavHost`, `Button`, and a `clickable` layout for `Pressable`. Remote `AsyncImage` nodes receive a remembered Coil `ImageLoader` backed by the generated Cronet client. Text input and switch values bind directly to native state callbacks. The compiler generates source code rather than a runtime component tree; generated files join an existing native app target.

Android apps that use `Image` need [Coil 3 Compose](https://coil-kt.github.io/coil/compose/), Coil's `coil-network-core`, [Cronet from Google Play services](https://developer.android.com/develop/connectivity/cronet/start), and the Internet permission. Add `implementation("io.coil-kt.coil3:coil-compose:3.6.3")`, `implementation("io.coil-kt.coil3:coil-network-core:3.6.3")`, and `implementation("com.google.android.gms:play-services-cronet:18.0.1")`, plus `<uses-permission android:name="android.permission.INTERNET" />`. Generated code uses the Play Services provider through `CronetEngine.Builder`; applications that initialize the provider explicitly should call `CronetProviderInstaller.installProvider(context)` before the first generated image or network call. Do not add Coil's OkHttp network module to a Nexa app. Apps with navigation need `implementation("androidx.navigation:navigation-compose:<compatible-version>")`. When using `TextInput(autocorrect: ...)`, use a Compose UI version that supports `KeyboardOptions.autoCorrectEnabled` (1.7.0 or newer). `KeyboardAware` requires the Activity to use `android:windowSoftInputMode="adjustResize"` so Compose receives IME insets. Swift remote images use the generated URLSession-backed loader and require an iOS 15 or newer deployment target; SwiftUI navigation and `KeyboardAware` use iOS 16 or newer.

## Current boundaries

This prototype does not yet generate full Xcode or Gradle projects. An iOS app using `FastList` must target iOS 16 or newer for `UIHostingConfiguration`; table rows use self-sizing with estimated heights, update visible cells in place, and reload table data only when the row count changes. Navigation currently supports parameterless screens, a single stack, and compile-time checked links; typed route parameters, tabs, deep links, modals, and navigation guards remain future work. `FastList` supports integer ranges and primitive `Array<T>` collections. Custom stable row keys, mutable element-level bindings, paging, grids, horizontal lists, scroll controls, and refresh integration remain future work. `KeyboardAware` handles basic scrolling and inset adjustment, but keyboard height/events, explicit focus management, and programmatic dismissal remain future work. `StatusBar` currently supports one app-level static style/visibility declaration; background colors and animated transitions remain future work. Network and file APIs are emitted as native helpers; first-class `.nx` async declarations and typed error handling remain future language work. `Pressable` currently supports press and disabled state; long press, hover, pressed-state styling, focus, and haptics remain future work. User-defined functions, enums and exhaustive matching, loops, nullable values, closures, collection transformations, and class/value-type declarations remain future work. Text input selection/autofill, custom component callbacks and content slots, advanced theme features, plugins, FFI bindings, incremental compilation, formatting, and language-server support remain on the roadmap in `plan.md`. Compiler warnings and the current IR optimization pass are documented in [compiler-diagnostics.md](compiler-diagnostics.md).
