# Nexa language: first slice

The compiler currently supports stateful native screens, typed arrays and collection values, virtualized lists, conditional UI and event control flow, compile-time platform blocks, a parameterless native navigation stack, stateful user-defined components, native status bars, sheets, pull-to-refresh, labeled application bottom bars, static external links, accessibility semantics, static RTL/LTR direction, typed app-local async lifecycle work, and a feature-gated native network/file library. The entry `.nx` file declares one `app`; imported component files can declare reusable components without an app.

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

`state` is mutable and `let` is immutable; Nexa does not silently change mutability based on whether a binding is reassigned. Both declarations use the same compile-time type inference when their initializer is non-empty and unambiguous. For example, `let retries = 3` and `state retries = 3` both become `Int32`; neither is left for Swift or Kotlin to choose independently. Inference uses the same deterministic defaults on both platforms: integer literals become `Int32`, decimal literals become `Float64`, and the type of a non-empty value expression is propagated. The inferred type is written into the typed IR before native generation, so bindings remain predictable. Supported scalar types are `String`, `Bool`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Float32`, and `Float64`. Generic value types can be nested, including arrays, sets, maps, pairs, and triples.

The supported generic types are `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>`. Literals are checked against the declared or parameter type. In a declaration, non-empty `Array`, `Map`, `Pair`, and `Triple` values can propagate their element types; `Set` keeps an explicit annotation because its bracket spelling is shared with arrays, and empty collections have no element type to infer:

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

String literals can include a state or constant with `$name`, or any supported expression inside `\(...)`. Interpolation is parsed and resolved during semantic lowering and becomes native Swift or Kotlin string interpolation; it does not add a runtime template engine or an intermediate string. The embedded expression keeps its native type:

```nexa
state count = 3
let title = "Nexa"

body {
    Text("$title: \(count)")
    Text("Next: \(count + 1)")
    Button("Increment $count") {
        count = count + 1
    }
}
```

The bracket literal is contextual: `[value, ...]` creates an `Array<T>` or `Set<T>` according to the expected type. A non-empty array literal can infer an `Array<T>` for `let`; a set still needs an explicit `Set<T>` annotation because the source spelling is shared with arrays. Empty arrays, sets, and maps also need explicit types. Map literals use `[key: value, ...]`; the empty map literal is `[:]`. If a map literal contains the same runtime key more than once, the last value wins on both platforms. Set elements are deduplicated. Neither set nor map iteration order is guaranteed across platforms.

`Pair(a, b)` and `Triple(a, b, c)` construct fixed-size ordered values. Their type arguments are checked position by position, and `.first`, `.second`, and `.third` (for triples) are checked at compile time. Swift output uses native tuple positions for pairs and triples; Kotlin output uses the standard-library `Pair` and `Triple` fields, which are ordinary objects on Android/JVM. The compiler adds no Nexa-specific collection runtime or wrapper types. Arrays map to Swift `Array<T>` and Kotlin read-only `List<T>`, sets to Swift `Set<T>` and Kotlin read-only `Set<T>`, and maps to Swift `Dictionary<K, V>` and Kotlin read-only `Map<K, V>`. See [tuple-members.nx](../examples/tuple-members.nx).

For compatible hashing behavior on both platforms, `Set<T>` elements and `Map<K, V>` keys must be scalar `String`, `Bool`, or numeric values. Map values and pair/triple fields may use any supported type, including nested collection types. The language supports declaring, initializing, passing, assigning, displaying, and indexing arrays and maps. `array[index]` requires an `Array<T>` and an `Int32` index, and returns `T`; map lookup uses `map[key]` with the declared key type and returns `V?` because the key may be absent. Optional arrays/maps use `array?[index]` and return an optional element/value. Both forms use direct Swift and Kotlin native subscripting, so array bounds and map-missing behavior stay native. The `value in collection` operator performs direct native membership checks for scalar `Array<T>` and `Set<T>` elements and `Map<K, V>` keys. Insertion/removal, transformations, and equality comparisons for collection and pair/triple values remain future work; Set iteration is available in action `for` loops and map iteration uses `for (key, value) in map` destructuring with native unspecified order. `FastList(items: ...)` is the current way to render an `Array<T>` as repeated rows. See [collection-values.nx](../examples/collection-values.nx), [collection-indexing.nx](../examples/collection-indexing.nx), [collection-membership.nx](../examples/collection-membership.nx), and [map-iteration.nx](../examples/map-iteration.nx).

Numeric literal values are checked against the declared type, including signed negative literals. Numeric variables do not implicitly convert between types. `+` accepts operands of one numeric type; integer addition wraps on overflow and floating-point addition follows native IEEE behavior. A numeric literal takes its type from the other operand where possible, so `wideCount < 10` does not require a conversion.

`when` provides exhaustive scalar multiple-choice UI branching with one required `else` case. Case values must be unique `String`, `Bool`, numeric literals, or cases of a declared enum and must match the scrutinee type. Swift receives a native `switch`; Kotlin receives a native `when`. No runtime case table or dynamic dispatch is generated:

```nexa
when selected {
    0: { Text("Home") }
    1: { Text("Settings") }
    else: { Text("Other") }
}
```

Optional values use `T?` and the `null` literal. `value ?? fallback` returns the wrapped value when present and otherwise evaluates the fallback; Swift receives `??`, while Kotlin receives its native `?:` spelling. Optional values lower directly to Swift optionals and Kotlin nullable types, with no wrapper object or runtime helper. A `null` initializer requires an explicit optional annotation, and the fallback must have the wrapped non-optional type. Pair and Triple values support compile-time checked safe member access (`pair?.first`, `triple?.third`), and optional arrays/maps support safe indexing (`maybeValues?[0]`, `maybePrices?["pro"]`); all lower to native optional chaining and return an optional field/element value. Safe indexing requires an optional collection, while ordinary indexing requires a non-optional collection. General user-defined optional members and explicit unwrap operators remain future work. See [nullable-values.nx](../examples/nullable-values.nx) and [optional-members.nx](../examples/optional-members.nx).
Enums are closed value types declared inside an app. Cases have no associated payloads in this slice and are referenced with `EnumName.caseName`:

```nexa
app ThemeExample {
    enum ThemeMode { light, dark }
    state mode: ThemeMode = ThemeMode.light

    body {
        when mode {
            ThemeMode.light: { Text("Light") }
            ThemeMode.dark: { Text("Dark") }
            else: { Text("Unknown") }
        }
    }
}
```

The compiler validates enum names, case names, duplicate cases, assignment types, equality, and `when` case types before lowering. Swift emits a private `enum ...: String`; Kotlin emits a private `enum class`. Associated values, enum methods, and pattern payloads remain future work. See [enums.nx](../examples/enums.nx).

Inferred `let` and `state` initializers can use the same expression forms as explicitly typed declarations. Mutable state initializers must still be independent of other state values in this version. Forward references and generics beyond the five built-in collection/value types are not implemented yet.

## Functions

An app can declare a pure, statically typed function and call it from state initializers or UI expressions:

```nexa
app Functions {
    fn add(a: Int32, b: Int32) -> Int32 {
        return a + b
    }

    let total = add(1, 2) // inferred as Int32

    body {
        Text(total)
    }
}
```

Function parameters and the return type are explicit. A function body may declare ordered immutable local constants with `let` before exactly one `return` statement; local types use the same inference rules as app state, and later locals can reference earlier locals. An `async fn` uses the same typed signature and body rules, but may call another async function with `await`. Calls are checked for name, arity, argument types, async usage, and return type during semantic lowering, then become direct private top-level Swift or Kotlin functions (`async` on Swift and `suspend` on Kotlin). There is no runtime function registry or dynamic dispatch. Function parameters are ordinary typed bindings, so the same `Int32` and `Float64` literal defaults apply when a numeric literal has no expected narrower type. See [functions.nx](../examples/functions.nx), [function-locals.nx](../examples/function-locals.nx), and [async.nx](../examples/async.nx).

### Async functions and lifecycle work

The first async slice is deliberately small and maps directly to each platform's native lifecycle primitive:

```nexa
app AsyncDemo {
    async fn loadValue() -> Int32 {
        return 3
    }

    state value: Int32 = 0

    body {
        OnAppear async {
            value = await loadValue()
        }
        Text(value)
    }
}
```

`async fn` declarations are app-local and must keep explicit parameter and return types. `await` is accepted only for a direct async function call, and an async call must be awaited; synchronous calls cannot be awaited. Async work is allowed in `OnAppear async` action blocks and in async function local initializers/returns, and is rejected from state initializers, UI expressions, and synchronous action blocks. Swift emits `async` helpers and `.task {}`; Kotlin emits `suspend` helpers and the existing `LaunchedEffect(Unit)`. Cancellation and scheduling stay native, with no Nexa scheduler or callback registry. Network and file helper APIs remain native async libraries; exposing them as typed `.nx` calls is a later slice.

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

Logical operators are `&&`, `||`, and `!`; equality is `==` and `!=`; numeric comparisons are `<`, `<=`, `>`, and `>=`; membership is `in`. `&&` and `||` short-circuit. Equality works for `Bool`, numeric scalars, `String`, and closed enum values; ordering currently works for numeric scalars. `value in collection` works for scalar array/set elements and map keys. Compared values must have the same type. Nexa does not insert numeric conversions. Conditions lower directly to Swift/Kotlin operators and native `if` branches, while scalar and enum `when` lowers to native `switch`/`when`. Pure literal conditions are folded by the compiler, and unreachable UI/event branches are removed before code generation. See [conditional-logic.nx](../examples/conditional-logic.nx), [constant-branches.nx](../examples/constant-branches.nx), [enums.nx](../examples/enums.nx), and [when.nx](../examples/when.nx).

Event and lifecycle action blocks also support native loops:

```nexa
Button("Sum odd values") {
    for value in values {
        if value == 2 {
            continue
        }
        total = total + value
    }
    while total < 10 {
        total = total + 1
        if total == 8 {
            break
        }
    }
}
```

`for` iterates an `Array<T>` or `Set<T>` directly, an `Int32` range, or a map with explicit destructuring: `for (key, value) in prices`. Map keys and values are immutable bindings with their declared types. Range syntax uses `start..end` for an inclusive range, `start..<end` for an exclusive upper bound, and `start...end` as an inclusive alias for Swift familiarity. Add `step positiveInt32Literal` to use a native stepped progression, for example `for value in 0..10 step 2`; non-positive and dynamic steps are rejected so Swift and Kotlin keep the same deterministic behavior. The loop binding is an immutable value with the collection element type or `Int32` for a range. `while` requires a `Bool` condition. `break` and `continue` are valid only inside one of these loops. Loop actions are accepted in button, pressable, text-input submit, refresh, and lifecycle callbacks and lower directly to Swift `for`/`while` and Kotlin `for`/`while`; ranges and map entries stay native and do not materialize an array, iterator, callback registry, or shared loop runtime. Functions remain pure and continue to reject loops. Use `FastList` for repeated UI rows so virtualization is preserved. See [action-loops.nx](../examples/action-loops.nx), [ranges.nx](../examples/ranges.nx), [collection-membership.nx](../examples/collection-membership.nx), and [map-iteration.nx](../examples/map-iteration.nx).

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
- `Direction(value: LTR|RTL)` sets one static app-level layout direction. Swift emits the native `layoutDirection` environment value and Compose provides `LocalLayoutDirection`; omitted direction follows the platform/system setting. Directional alignment uses native leading/trailing or start/end behavior, while dynamic language changes and directional spacing APIs remain future work. See [direction.nx](../examples/direction.nx).
- `OnAppear { ... }` and `OnDisappear { ... }` run lifecycle callbacks directly on the app root or on a named screen. Each scope accepts at most one top-level block with the same direct state assignments, `if`, and loop actions as button handlers. Add `async` (`OnAppear async { ... }`) when the block awaits an `async fn`; Swift emits `.task` for the async form and `.onAppear` for the synchronous form, while Android uses `LaunchedEffect(Unit)` for both and `DisposableEffect(Unit)` for `OnDisappear`. Nested declarations are rejected, so callbacks attach without a lifecycle registry or runtime event bus. App active/background events and cancellation details remain native lifecycle behavior. See [lifecycle.nx](../examples/lifecycle.nx), [async.nx](../examples/async.nx), and [navigation.nx](../examples/navigation.nx).
- `BottomSheet(isPresented: mutableBool) { ... }` presents content with SwiftUI's native `.sheet` or Compose Material 3 `ModalBottomSheet`. Dismissal writes `false` to the bound mutable state; snap points, custom transitions, and drag callbacks remain future options.
- `RefreshControl(isRefreshing: mutableBool) { ... } { ... }` wraps content in SwiftUI `.refreshable` or Compose Material 3 `PullToRefreshBox`. The second block is the native refresh callback and can update the bound state directly. Content without a native list/scroll container receives one platform scroll wrapper; existing `FastList` and `KeyboardAware` content is not wrapped again. Custom indicators and refresh event streams remain future options.
- `AppBottomBar(selected: mutableInt32) { Tab(index: int32Literal, label: "...") { ... } ... }` provides a native application tab bar. Swift emits `TabView(selection:)` with native tab items; Android emits Material 3 `Scaffold` with a `NavigationBar` and keeps the selected tab content in a direct `when` branch. Tab indexes must be unique, non-negative `Int32` literals and labels are static strings. Icons, badges, custom transitions, and per-platform tab customization remain future options. See [app-bottom-bar.nx](../examples/app-bottom-bar.nx).
- `View` is not a Nexa component. Replace existing `View { ... }` blocks with `Column { ... }`; the native SwiftUI `View` protocol in generated Swift remains unchanged.
- `Row { ... }` maps to a native horizontal stack.
- `Column(spacing: 12) { ... }` and `Row(spacing: 12) { ... }` map spacing directly to platform layout parameters.
- Layouts accept a cross-axis `alignment` of `Start`, `Center`, or `End`; it maps to horizontal alignment for `Column` and vertical alignment for `Row`. For vertical layouts, `Start`/`End` follow the layout direction; for rows they mean top/bottom. Layouts also accept `padding`, `width`, `height`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, and `opacity`. `borderColor` and `borderWidth` must be provided together. Dimensions use points on iOS and density-independent pixels on Android. Backgrounds and borders accept `#RGB`, `#RRGGBB`, `#RRGGBBAA`, or a color theme token; opacity must be from `0` to `1`. See [borders.nx](../examples/borders.nx).
- Layouts optionally accept a static `animation` spec: `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`. Swift emits the matching SwiftUI animation modifier and Compose emits `Modifier.animateContentSize` with the corresponding native spring/tween spec. No cross-platform animation loop is generated. See [animations.nx](../examples/animations.nx).
- `Text(expression, color: ..., fontSize: ..., fontWeight: ..., lineLimit: ..., lineHeight: ..., letterSpacing: ..., selectable: ...)` accepts strings, booleans, and numeric values. Text color accepts a hexadecimal color or theme color token. Font size, line height, and letter spacing use points on iOS and scale-independent pixels on Android. `fontWeight` accepts `Normal`, `Medium`, `Semibold`, or `Bold`; `lineLimit` is a positive integer and maps to native line truncation; `selectable` enables native text selection. See [text-style.nx](../examples/text-style.nx).
- `Button("Label", loading: Bool, disabled: Bool) { ... }` accepts a string or interpolated label and state assignments in its press handler. When `loading` is true, SwiftUI shows `ProgressView` and Compose shows `CircularProgressIndicator`; `disabled` can independently disable the native button, and the compiler combines both conditions without a wrapper. Omit both options for the smallest direct button output. See [button-loading.nx](../examples/button-loading.nx) and [button-disabled.nx](../examples/button-disabled.nx).
- `TextInput(value: name, placeholder: "...", keyboard: Email, focused: isFocused) { ... }` binds a mutable `String` state to a native text field. Keyboard choices are `Text`, `Number`, `Email`, `Phone`, and `Url`; `secure: true` and `multiline: true` are optional. `focused` is an optional mutable `Bool` binding that maps to SwiftUI `@FocusState`/`.focused` and Compose `FocusRequester`/`onFocusChanged`. `autocorrect` optionally enables or disables keyboard suggestions, while `capitalization` accepts `None`, `Sentences`, `Words`, or `Characters`. The optional action block runs on the native submit/Done event (`.onSubmit` on SwiftUI and `KeyboardActions` on Compose); it is available for single-line fields, while omitting it keeps the existing minimal field output. Omitted keyboard options retain native defaults. Secure multiline input is rejected because the native APIs differ. See [text-input-submit.nx](../examples/text-input-submit.nx).
- `Switch(value: enabled, label: "...")` binds a mutable `Bool` state to the native switch control.
- `Image(asset: "nexa_mark", description: "...", scale: Fit)` loads a local platform drawable/asset. `Image(url: "https://...", description: "...", scale: Fill, placeholder: "avatar_loading")` loads a remote image; only absolute HTTPS URLs are accepted. Scale choices are `Fit` and `Fill`, `placeholder` is an optional local drawable/asset shown while loading or if the request fails, and an empty description marks a decorative image. iOS remote images use `URLSession`; Android remote images use Coil 3 with a Cronet-backed `NetworkClient`, so Coil never falls back to OkHttp for Nexa image requests.

- `Pressable(disabled: Bool) { ... } { ... } { ... }` wraps composed content in a native accessible press target. Its second block contains tap actions and its optional third block contains long-press actions; `disabled` accepts a Boolean expression and defaults to `false`. Swift emits a direct `Button` with `.disabled(...)` and `.onLongPressGesture`, while Compose passes the expression to `combinedClickable` or `clickable`. Omitting the third block keeps the smaller single-gesture path, and a static `false` adds no disabled modifier. See [pressable-long-press.nx](../examples/pressable-long-press.nx).
- `KeyboardAware { ... }` places content in a native scroll container that adjusts around the software keyboard. SwiftUI uses interactive scroll-to-dismiss behavior; Compose applies animated IME inset padding and native vertical scrolling.
- `screen Home { ... }` declares a named destination. An app with screens uses `body { NavigationStack(root: Home) }`; `NavigationLink(destination: Profile) { ... }` pushes a statically resolved screen. All declared screens share the app's state. Destinations must be declared in the same app.
- `Link(url: "https://example.com") { ... }` opens an external web URL or application scheme with the native platform handler. URLs are static strings with a valid scheme, so generated Swift uses SwiftUI `Link` and generated Android code uses an `ACTION_VIEW` intent after checking that a handler exists. URL availability checks, incoming link events, universal-link routing, and dynamic URL expressions remain future work. See [linking.nx](../examples/linking.nx).
- `Accessibility(label: "...", role: Button|Link|Header|Image) { ... }` adds a static accessible label and optional native role to any child subtree. Swift emits accessibility modifiers and Android emits Compose semantics; `Header` uses heading semantics on Android. Hints, focus control, custom actions, and dynamic accessibility values remain future work. See [accessibility.nx](../examples/accessibility.nx).
- `FastList(count: rowCount, index: row) { ... }` creates a virtualized, vertical list of `Int32` row indices. `index` is optional and defaults to `index`; row content is produced for visible rows by iOS `UITableView` or Android Compose `LazyColumn`. The count can be an `Int32` state value; negative runtime counts are treated as zero.
- `FastList(items: labels, index: row, item: label) { ... }` creates a virtualized list over an `Array<T>` state. Both bindings are optional and default to `index` and `item`; the index is read-only `Int32`, and the item is a read-only value of the array's element type. Rows use their current position as identity, and the collection is indexed directly without building an intermediate row array or row tree. `Set<T>` and `Map<K, V>` are values but are not accepted as list sources. See [collection-list.nx](../examples/collection-list.nx).

## Compile-time permissions

Declare the host permissions an app needs in one compile-time block:

```nexa
app CameraExample {
    permissions {
        camera,
        microphone,
        photos,
        location,
        notifications,
        contacts,
        calendar,
        bluetooth
    }

    body {
        Text("Permission declarations are part of the generated host project")
    }
}
```

Supported names are `camera`, `microphone`, `photos`, `location`, `notifications`, `contacts`, `calendar`, and `bluetooth`. The compiler validates names and duplicates, then stores the result as project metadata. `nexa generate` writes the matching iOS `Info.plist` usage descriptions and Android manifest permissions, so supported apps do not need native edits for declarations. This slice does not request access or expose runtime status yet; typed status/request APIs remain future work. Notification access has no iOS usage-description key, while Android receives `POST_NOTIFICATIONS`. See [permissions.nx](../examples/permissions.nx).

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

The Swift backend emits a SwiftUI `View`, stores mutable values in `@State`, and maps layouts and controls to `VStack`, `HStack`, `Text`, `TextField`/`SecureField`, `Toggle`, `Image`, the generated URLSession-backed remote image view, `NavigationStack`/`NavigationLink`, `Link`, `Button`, and `TabView` for `AppBottomBar`. `FastList` is backed by `UITableView` with reusable cells and SwiftUI row content hosted by [`UIHostingConfiguration`](https://developer.apple.com/documentation/swiftui/uihostingconfiguration). The Kotlin backend emits a `@Composable` function and maps controls to Compose `Column`, `Row`, `LazyColumn`, `Text`, `TextField`, `Switch`, Coil 3 `AsyncImage`, AndroidX Navigation Compose `NavHost`, `Button`, a `clickable`/`combinedClickable` layout for `Pressable`, and Material 3 `Scaffold`/`NavigationBar` for `AppBottomBar`. Remote `AsyncImage` nodes receive a remembered Coil `ImageLoader` backed by the generated Cronet client. Text input values bind directly to native state callbacks, and submit blocks map to SwiftUI `.onSubmit` or Compose `KeyboardActions`. The compiler generates source code rather than a runtime component tree; generated files join an existing native app target.

Android apps that use `Image` need [Coil 3 Compose](https://coil-kt.github.io/coil/compose/), Coil's `coil-network-core`, [Cronet from Google Play services](https://developer.android.com/develop/connectivity/cronet/start), and the Internet permission. Add `implementation("io.coil-kt.coil3:coil-compose:3.6.3")`, `implementation("io.coil-kt.coil3:coil-network-core:3.6.3")`, and `implementation("com.google.android.gms:play-services-cronet:18.0.1")`, plus `<uses-permission android:name="android.permission.INTERNET" />`. Generated code uses the Play Services provider through `CronetEngine.Builder`; applications that initialize the provider explicitly should call `CronetProviderInstaller.installProvider(context)` before the first generated image or network call. Do not add Coil's OkHttp network module to a Nexa app. Apps with navigation need `implementation("androidx.navigation:navigation-compose:<compatible-version>")`. When using `TextInput(autocorrect: ...)`, use a Compose UI version that supports `KeyboardOptions.autoCorrectEnabled` (1.7.0 or newer). `KeyboardAware` requires the Activity to use `android:windowSoftInputMode="adjustResize"` so Compose receives IME insets. Swift remote images use the generated URLSession-backed loader and require an iOS 15 or newer deployment target; SwiftUI navigation and `KeyboardAware` use iOS 16 or newer.

## Current boundaries

`nexa generate entry.nx --target ios|android|all --out AppProject --name AppName` creates a native project bundle with generated Swift/Kotlin sources, an iOS Xcode project, an Android Compose/Gradle project, platform dependency declarations, and a `nexa.project.json` manifest. Edit the `.nx` entry source and regenerate; generated native files are outputs. An iOS app using `FastList` must target iOS 16 or newer for `UIHostingConfiguration`; table rows use self-sizing with estimated heights, update visible cells in place, and reload table data only when the row count changes. Navigation currently supports parameterless screens, a single stack, and compile-time checked links; typed route parameters, tabs, deep links, modals, and navigation guards remain future work. `Link` currently opens static URLs and application schemes through native handlers; incoming links, URL availability expressions, and universal-link routing remain future work. `Accessibility` currently supports static labels and the `Button`, `Link`, `Header`, and `Image` roles; hints, focus control, custom accessibility actions, and dynamic values remain future work. `Direction` currently supports one app-level static `LTR` or `RTL` override; dynamic language changes and direction-specific spacing/margins remain future work. `OnAppear` and `OnDisappear` support one top-level callback on the app body or each named screen; `OnAppear async` and typed `async fn` are implemented for direct lifecycle work, while app active/background events and richer cancellation controls remain future work. `FastList` supports integer ranges and primitive `Array<T>` collections. Custom stable row keys, mutable element-level bindings, paging, grids, horizontal lists, scroll controls, and FastList-specific refresh integration remain future work. `KeyboardAware` handles basic scrolling and inset adjustment, but keyboard height/events, explicit focus management, and programmatic dismissal remain future work. `StatusBar` currently supports one app-level static style/visibility declaration; background colors and animated transitions remain future work. `BottomSheet` currently supports a native modal with a mutable Boolean binding; snap points, custom transitions, and drag callbacks remain future work. `RefreshControl` currently supports a native pull-to-refresh wrapper and callback block; custom indicators and refresh event streams remain future work. Network and file APIs are emitted as native helpers; typed `.nx` calls to those async libraries and typed error handling remain future language work. `Pressable` currently supports press, long press, and static or dynamic disabled state; hover, pressed-state styling, focus, and haptics remain future work. `TextInput` supports native submit/Done callbacks and mutable focus bindings; selection, autofill, password-manager integration, validation, and richer submit actions remain future work. Pure app functions support typed parameters, ordered immutable local constants, one return expression, and native async boundaries; closures, generic functions, enum associated-value patterns, loop expressions, general optional member access, collection transformations, and class/value-type declarations remain future work. Pair/Triple safe optional member access is implemented with native chaining. Callback and lifecycle action blocks support direct native `for`/`while` loops with arrays, sets, Int32 ranges, positive literal steps, `break`, and `continue`; use `FastList` for UI repetition. Custom component callbacks and content slots, advanced theme features, plugins, FFI bindings, incremental compilation, formatting, and language-server support remain on the roadmap in `plan.md`. Compiler warnings and the current IR optimization pass are documented in [compiler-diagnostics.md](compiler-diagnostics.md).
