# Nexa language: first slice

The compiler currently supports stateful native screens, typed value structs, typed arrays and collection values, virtualized lists, conditional UI and event control flow, compile-time platform blocks, a typed native navigation stack, stateful user-defined components, native status bars, sheets, pull-to-refresh, labeled application bottom bars with static icons and badges, typed external links, accessibility semantics, static RTL/LTR direction, typed app-local async lifecycle work, local typed plugin calls, and a feature-gated native network/file library. The entry `.nx` file declares one `app`; imported files can declare reusable components, pure typed functions, and top-level structs without an app.

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

`state` is mutable and `let` is immutable; Nexa does not silently change mutability based on whether a binding is reassigned. Both declarations use the same compile-time type inference when their initializer is non-empty and unambiguous. For example, `let retries = 3` and `state retries = 3` both become `Int32`; neither is left for Swift or Kotlin to choose independently. Inference uses the same deterministic defaults on both platforms: integer literals become `Int32`, decimal literals become `Float64`, and the type of a non-empty value expression is propagated. The inferred type is written into the typed IR before native generation, so bindings remain predictable. Supported scalar types are `String`, `Bool`, `Int8`, `Int16`, `Int32`, `Int64`, `UInt8`, `UInt16`, `UInt32`, `Float32`, and `Float64`. Generic value types can be nested, including arrays, sets, maps, pairs, triples, and user-defined structs. See [inferred-bindings.nx](../examples/inferred-bindings.nx).

Top-level structs are compile-time value models. Fields are explicitly typed, constructors are positional, and member access is checked before backend generation:

```nexa
struct User {
    id: Int64,
    name: String
}

app Profile {
    state user: User = User(42, "Ada")

    body {
        Text(user.name)
    }
}
```

Swift receives a private value `struct` with a direct initializer; Kotlin receives a private `data class`. Struct calls lower directly to those native initializers, with no helper function, runtime model registry, or boxed field access. `user?.name` is supported for optional structs and returns an optional field. Recursive value layouts, named constructor arguments, methods, inheritance, and mutation are not supported.

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

For compatible hashing behavior on both platforms, `Set<T>` elements and `Map<K, V>` keys must be scalar `String`, `Bool`, or numeric values. Map values and pair/triple fields may use any supported type, including nested collection types. The language supports declaring, initializing, passing, assigning, displaying, and indexing arrays and maps. `array[index]` requires an `Array<T>` and an `Int32` index, and returns `T`; map lookup uses `map[key]` with the declared key type and returns `V?` because the key may be absent. Optional arrays/maps use `array?[index]` and return an optional element/value. Both forms use direct Swift and Kotlin native subscripting, so array bounds and map-missing behavior stay native. The `value in collection` operator performs direct native membership checks for scalar `Array<T>` and `Set<T>` elements and `Map<K, V>` keys. Equality now lowers directly for recursively equatable arrays, sets, maps, pairs, triples, optionals, enums, and non-empty value structs. Mutable collection states support direct `items.append(value)` and `items.remove(index)` for arrays, `tags.insert(value)` and `tags.remove(value)` for sets, and `prices.set(key, value)` and `prices.remove(key)` for maps. These operations are checked against the declared element, key, and index types, then lower directly to Swift collection methods or Compose snapshot collection methods without rebuilding the collection. Set iteration is available in action `for` loops and map iteration uses `for (key, value) in map` destructuring with native unspecified order.

Non-escaping collection transformations are available with inline closures: `values.map { value -> expression }`, `values.filter { value -> condition }`, and `values.reduce(initial) { accumulator, value -> expression }`. The compiler checks closure arity, parameter bindings, result types, and the `Bool` result required by `filter`; `Array<T>` is currently the supported source collection. Swift lowers directly to `map`, `filter`, and `reduce`, while Kotlin uses `map`, `filter`, and `fold` with no Nexa runtime callback or boxed collection wrapper. `await` is rejected inside these synchronous callbacks. See [collection-transform.nx](../examples/collection-transform.nx). `FastList(rows) { row, index in ... }` is the current way to render an `Array<T>` as repeated rows. See [collection-values.nx](../examples/collection-values.nx), [collection-indexing.nx](../examples/collection-indexing.nx), [collection-membership.nx](../examples/collection-membership.nx), [collection-mutation.nx](../examples/collection-mutation.nx), and [map-iteration.nx](../examples/map-iteration.nx).

Numeric literal values are checked against the declared type, including signed negative literals. Numeric variables do not implicitly convert between types. `+` accepts operands of one numeric type; integer addition wraps on overflow and floating-point addition follows native IEEE behavior. A numeric literal takes its type from the other operand where possible, so `wideCount < 10` does not require a conversion.

`when` provides exhaustive scalar multiple-choice UI branching with one required `else` case. Case values must be unique `String`, `Bool`, numeric literals, or cases of a declared enum and must match the scrutinee type. Swift receives a native `switch`; Kotlin receives a native `when`. No runtime case table or dynamic dispatch is generated:

```nexa
when selected {
    0: { Text("Home") }
    1: { Text("Settings") }
    else: { Text("Other") }
}
```

Optional values use `T?` and the `null` literal. `value ?? fallback` returns the wrapped value when present and otherwise evaluates the fallback; Swift receives `??`, while Kotlin receives its native `?:` spelling. Optional values lower directly to Swift optionals and Kotlin nullable types, with no wrapper object or runtime helper. A `null` initializer requires an explicit optional annotation, and the fallback must have the wrapped non-optional type. Pair, Triple, and struct values support compile-time checked safe member access (`pair?.first`, `triple?.third`, `user?.name`), and optional arrays/maps support safe indexing (`maybeValues?[0]`, `maybePrices?["pro"]`); all lower to native optional chaining and return an optional field/element value. Safe indexing requires an optional collection, while ordinary indexing requires a non-optional collection. Explicit unwrap operators remain future work. See [nullable-values.nx](../examples/nullable-values.nx) and [optional-members.nx](../examples/optional-members.nx).
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

Function parameters and the return type are explicit. A function body may declare ordered immutable local constants with `let` before exactly one `return` statement; local types use the same inference rules as app state, and later locals can reference earlier locals. An `async fn` uses the same typed signature and body rules, but may call another async function with `await`. Calls are checked for name, arity, argument types, async usage, and return type during semantic lowering, then become direct private top-level Swift or Kotlin functions (`async` on Swift and `suspend` on Kotlin). There is no runtime function registry or dynamic dispatch. Function parameters are ordinary typed bindings, so the same `Int32` and `Float64` literal defaults apply when a numeric literal has no expected narrower type. Pure plugin packages may declare these functions at the package root; the compiler loads them into the same typed graph without a plugin runtime. See [functions.nx](../examples/functions.nx), [function-locals.nx](../examples/function-locals.nx), and [async.nx](../examples/async.nx).

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

`async fn` declarations are app-local and must keep explicit parameter and return types. `await` is accepted only for a direct async function call or a qualified native call, and an async call must be awaited; synchronous calls cannot be awaited. Async work is allowed in `OnAppear async` action blocks and in async function local initializers/returns, and is rejected from state initializers, UI expressions, and synchronous action blocks. Swift emits `async` helpers and `.task {}`; Kotlin emits `suspend` helpers and the existing `LaunchedEffect(Unit)`. Cancellation and scheduling stay native, with no Nexa scheduler or callback registry. Native network, file, and declared local-plugin calls are compile-time lowered to direct platform methods with no dynamic dispatch. Swift source calls use a deterministic empty/false response fallback when a native throwing operation fails; typed plugin error values remain future work.

App lifecycle callbacks use the same direct action syntax. `OnActive`, `OnInactive`, and `OnBackground` are app-body declarations only, each allowed once; they run when the native application moves to the corresponding foreground state. Swift observes `scenePhase` and Kotlin observes the host `Lifecycle` with one `LifecycleEventObserver`, so no polling, event bus, or shared lifecycle runtime is generated. Screen `OnAppear` and `OnDisappear` remain screen-local callbacks.

### Local plugin calls

Declare a local plugin before the `app` declaration:

```nexa
plugin "../CameraPlugin" as Camera

app CameraExample {
    state message: String = "idle"

    body {
        Text(message)
        OnAppear async {
            message = await Camera.ping(value: "hello")
        }
    }
}
```

The path is resolved from the entry file and may name a plugin directory (which
contains `interfaces.nxid`) or the IDL file itself. The namespace must match an
IDL interface name or the plugin's sole interface. Method names, named
arguments, parameter types, return types, and async usage are checked at compile
time. The `.nx` call surface uses the existing scalar, optional, collection,
pair, and triple values; named IDL models remain native binding types for the
plugin implementation. `Result<Success, Failure>` is lowered as the native
success value while the native implementation handles the failure. A plugin
call is emitted directly as `CameraPlugin.shared` on Swift or
`CameraPlugin.instance` on Kotlin. `nexa generate` includes the plugin's local
platform source tree in the generated project. Plugin-only modules do not emit
the core Network/Path/File helper library or Android Cronet dependencies.
Package installation, dynamic plugin lookup, and typed error recovery in `.nx`
are not part of this slice.

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

Logical operators are `&&`, `||`, and `!`; equality is `==` and `!=`; numeric comparisons are `<`, `<=`, `>`, and `>=`; membership is `in`. `&&` and `||` short-circuit. Equality works for `Bool`, numeric scalars, `String`, closed enum values, optionals, recursively equatable arrays, sets, maps, pairs, triples, and non-empty value structs; ordering currently works for numeric scalars. `value in collection` works for scalar array/set elements and map keys. Compared values must have the same type. Nexa does not insert numeric conversions. Conditions lower directly to Swift/Kotlin operators and native `if` branches, while scalar and enum `when` lowers to native `switch`/`when`. Pure literal conditions are folded by the compiler, and unreachable UI/event branches are removed before code generation. See [conditional-logic.nx](../examples/conditional-logic.nx), [collection-equality.nx](../examples/collection-equality.nx), [constant-branches.nx](../examples/constant-branches.nx), [enums.nx](../examples/enums.nx), and [when.nx](../examples/when.nx).

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

`for` iterates an `Array<T>` or `Set<T>` directly, an `Int32` range, or a map with explicit destructuring: `for (key, value) in prices`. Map keys and values are immutable bindings with their declared types. Range syntax uses `start..end` for an inclusive range and `start..<end` for an exclusive upper bound. Add `step positiveInt32Literal` to use a native stepped progression, for example `for value in 0..10 step 2`; non-positive and dynamic steps are rejected so Swift and Kotlin keep the same deterministic behavior. The loop binding is an immutable value with the collection element type or `Int32` for a range. `while` requires a `Bool` condition. `break` and `continue` are valid only inside one of these loops. Loop actions are accepted in button, pressable, text-input submit, refresh, and lifecycle callbacks and lower directly to Swift `for`/`while` and Kotlin `for`/`while`; ranges and map entries stay native and do not materialize an array, iterator, callback registry, or shared loop runtime. Functions remain pure and continue to reject loops. Use `FastList` for repeated UI rows so virtualization is preserved. See [action-loops.nx](../examples/action-loops.nx), [ranges.nx](../examples/ranges.nx), [collection-membership.nx](../examples/collection-membership.nx), and [map-iteration.nx](../examples/map-iteration.nx).

`Layout.isRegularWidth`, `Layout.isCompactWidth`, `Layout.isRegularHeight`, and `Layout.isCompactHeight` are built-in `Bool` predicates for responsive conditional layouts. Swift reads native horizontal or vertical size classes and checks for `.regular` or `.compact`; Compose checks whether the current configuration width or height is at least `600dp` or below `600dp`. The platform definitions differ, so use these as layout hints rather than a guarantee that both platforms classify every device identically. They follow configuration changes and add no wrapper view. See [responsive-layout.nx](../examples/responsive-layout.nx) and [responsive-size.nx](../examples/responsive-size.nx):

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

- `Column { ... }` is the vertical container. It maps to SwiftUI `VStack` and Compose `Column`; the IR optimizer removes a one-child Column/Row/Stack when it has no spacing, alignment, or style effect, so semantically redundant native stacks are not emitted.
- `Stack { ... }` is the overlay container. It maps to SwiftUI `ZStack` and Compose `Box`; children are emitted once in source order, and static `alignment`, size, background, border, and other layout styles use the same native modifier path. `spacing` is rejected because overlay containers do not have inter-child spacing. See [stack.nx](../examples/stack.nx).
- `StatusBar(style: Default|Light|Dark, hidden: true|false, background: "#RRGGBB")` configures the app status bar's content appearance, visibility, and static background at the app body's top level or a named screen's top level. The compiler removes the declaration before rendering and emits SwiftUI safe-area coloring or AndroidX window status-bar calls for that native destination. The hexadecimal background is validated at compile time; animated transitions remain future options.
- `Direction(value: LTR|RTL)` sets one static app-level layout direction. Swift emits the native `layoutDirection` environment value and Compose provides `LocalLayoutDirection`; omitted direction follows the platform/system setting. Directional alignment uses native leading/trailing or start/end behavior, while dynamic language changes and directional spacing APIs remain future work. See [direction.nx](../examples/direction.nx).
- `OnAppear { ... }` and `OnDisappear { ... }` run lifecycle callbacks directly on the app root or on a named screen. Each scope accepts at most one top-level block with the same direct state assignments, `if`, and loop actions as button handlers. Add `async` (`OnAppear async { ... }`) when the block awaits an `async fn`; Swift emits `.task` for the async form and `.onAppear` for the synchronous form, while Android uses `LaunchedEffect(Unit)` for both and `DisposableEffect(Unit)` for `OnDisappear`. App roots may also declare one `OnActive`, `OnInactive`, and `OnBackground` block. Swift maps these to `scenePhase`; Android maps them to `Lifecycle.Event.ON_RESUME`, `ON_PAUSE`, and `ON_STOP` through one observer. Nested declarations and screen-level app lifecycle callbacks are rejected, so callbacks attach without a lifecycle registry or runtime event bus. See [lifecycle.nx](../examples/lifecycle.nx), [async.nx](../examples/async.nx), and [navigation.nx](../examples/navigation.nx).
- `BottomSheet(isPresented: mutableBool, partial: true|false) { ... }` presents content with SwiftUI's native `.sheet` or Compose Material 3 `ModalBottomSheet`. `partial: true` enables SwiftUI `.presentationDetents([.medium, .large])` or Compose's partially expanded sheet state; omitted and `false` keep the existing full-height path. Kotlin output adds the Material 3 experimental opt-in only to generated functions that use a bottom sheet. Dismissal writes `false` to the bound mutable state; custom snap points, transitions, and drag callbacks remain future options.
- `RefreshControl(isRefreshing: mutableBool) { ... }.onRefresh { ... }` uses native pull-to-refresh. A direct `FastList` child attaches `UIRefreshControl` to the UIKit table/collection view and emits a Compose Material 3 `PullToRefreshBox` around the lazy list, preserving native virtualization; other content uses SwiftUI `.refreshable` or the existing Compose wrapper. The named `.onRefresh` modifier is the native refresh callback and can update the bound state directly. Content without a native list/scroll container receives one platform scroll wrapper; existing `KeyboardAware` content is not wrapped again. Custom indicators and refresh event streams remain future options. See [list-refresh.nx](../examples/list-refresh.nx).
- `AppBottomBar(selected: mutableInt32) { Tab(index: int32Literal, label: "...", icon: "...", badge: "...") { ... } ... }` provides a native application tab bar. Swift emits `TabView(selection:)` with native tab items, interprets `icon` as an SF Symbol name, and applies a static `badge` string with the native tab badge modifier; Android emits Material 3 `Scaffold`/`NavigationBar`, resolves `icon` as a drawable resource name with one remembered lookup, and uses `BadgedBox` for a static badge. Tab indexes must be unique, non-negative `Int32` literals, labels, icon names, and badges are static strings, and icon/badge values are optional. Android uses a transparent painter when the named drawable is not supplied, so generated projects remain buildable before app assets are added. Custom transitions and per-platform tab customization remain future options. See [app-bottom-bar.nx](../examples/app-bottom-bar.nx).
- `Row { ... }` maps to a native horizontal stack.
- `Column(spacing: 12) { ... }` and `Row(spacing: 12) { ... }` map spacing directly to platform layout parameters.
- Layouts accept a cross-axis `alignment` of `Start`, `Center`, or `End`; it maps to horizontal alignment for `Column` and vertical alignment for `Row`. For vertical layouts, `Start`/`End` follow the layout direction; for rows they mean top/bottom. Layouts also accept `padding`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `background`, `cornerRadius`, `borderColor`, `borderWidth`, and `opacity`. Dimensions must be non-negative; each minimum cannot exceed its matching maximum. They use points on iOS and density-independent pixels on Android, and lower directly to SwiftUI frame bounds or Compose `widthIn`/`heightIn`. `borderColor` and `borderWidth` must be provided together. Backgrounds and borders accept `#RGB`, `#RRGGBB`, `#RRGGBBAA`, or a color theme token; opacity must be from `0` to `1`. See [borders.nx](../examples/borders.nx) and [layout-bounds.nx](../examples/layout-bounds.nx).
- Layouts optionally accept a static `animation` spec: `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`. Swift emits the matching SwiftUI animation modifier and Compose emits `Modifier.animateContentSize` with the corresponding native spring/tween spec. No cross-platform animation loop is generated. See [animations.nx](../examples/animations.nx).
- `Text(expression, color: ..., fontSize: ..., fontWeight: ..., lineLimit: ..., lineHeight: ..., letterSpacing: ..., selectable: ...)` accepts strings, booleans, and numeric values. Text color accepts a hexadecimal color or theme color token. Font size, line height, and letter spacing use points on iOS and scale-independent pixels on Android. `fontWeight` accepts `Normal`, `Medium`, `Semibold`, or `Bold`; `lineLimit` is a positive integer and maps to native line truncation; `selectable` enables native text selection. See [text-style.nx](../examples/text-style.nx).
- `Button("Label", icon: "name", loading: Bool, disabled: Bool) { ... }` accepts a string or interpolated label and state assignments in its press handler. `icon` is an optional static native name: SwiftUI uses an SF Symbol and Compose resolves an Android drawable through the shared remembered asset lookup. When `loading` is true, SwiftUI shows `ProgressView` and Compose shows `CircularProgressIndicator`; `disabled` can independently disable the native button, and the compiler combines both conditions without a wrapper. Omit optional values for the smallest direct button output. See [button-loading.nx](../examples/button-loading.nx) and [button-disabled.nx](../examples/button-disabled.nx).
- `TextInput(value: name, placeholder: "...", keyboard: Email, focused: isFocused, maxLength: 64) { ... }` binds a mutable `String` state to a native text field. Keyboard choices are `Text`, `Number`, `Email`, `Phone`, and `Url`; `secure: true` and `multiline: true` are optional. `focused` is an optional mutable `Bool` binding that maps to SwiftUI `@FocusState`/`.focused` and Compose `FocusRequester`/`onFocusChanged`. `autocorrect` optionally enables or disables keyboard suggestions, while `capitalization` accepts `None`, `Sentences`, `Words`, or `Characters`. `maxLength` is an optional positive integer literal enforced in the native change callback (`String.prefix` on SwiftUI and `take` on Compose). The optional action block runs on the native submit/Done event (`.onSubmit` on SwiftUI and `KeyboardActions` on Compose); it is available for single-line fields, while omitting it keeps the existing minimal field output. Omitted keyboard options retain native defaults. Secure multiline input is rejected because the native APIs differ. Selection, autofill, password-manager integration, and richer validation remain future slices. See [text-input-submit.nx](../examples/text-input-submit.nx).
- `Switch(value: enabled, label: "...")` binds a mutable `Bool` state to the native switch control.
- `Image(asset: "nexa_mark", description: "...", scale: Fit)` loads a local platform drawable/asset. Android resolves the drawable name once per composition and uses a transparent painter when an optional asset has not been supplied yet, so generated projects remain buildable while assets are added. `Image(url: "https://...", description: "...", scale: Fill, placeholder: "avatar_loading")` loads a remote image; the URL accepts any `String` expression, while literal values must be absolute HTTPS URLs and dynamic values are guarded by the generated native loader. Scale choices are `Fit` and `Fill`, `placeholder` is an optional local drawable/asset shown while loading or if the request fails, and an empty description marks a decorative image. iOS remote images use `URLSession`; Android remote images use Coil 3 with a Cronet-backed `NetworkClient`, so Coil never falls back to OkHttp for Nexa image requests.

- `Pressable(disabled: Bool, haptic: Light|Medium|Heavy) { ... }.onPress { ... }.onLongPress { ... }` wraps composed content in a native accessible press target. `.onPress` is required and `.onLongPress` is optional; `disabled` accepts a Boolean expression and defaults to `false`, while `haptic` is an optional static native feedback style. Swift emits a direct `Button` with `.disabled(...)`, UIKit impact feedback, and `.onLongPressGesture`; Compose passes the expression to `combinedClickable` or `clickable` and calls `performHapticFeedback`. Omitting haptic and `.onLongPress` keeps the smaller path, and a static `false` adds no disabled modifier. See [pressable-long-press.nx](../examples/pressable-long-press.nx).
- `KeyboardAware(dismiss: Interactive|Never) { ... }` places content in a native scroll container that adjusts around the software keyboard. `dismiss` defaults to `Interactive`; SwiftUI uses `scrollDismissesKeyboard`, while Compose applies animated IME inset padding and `imeNestedScroll` for the interactive path. `Never` omits interactive dismissal. Focus tracking, keyboard height/events, and programmatic dismissal remain future slices.
- `screen Home { ... }` declares a named destination. A screen may declare `state` or `let` bindings before its UI nodes; those bindings are scoped to that screen and lower to direct native storage. An app with screens uses `body { NavigationStack(root: Home) }`; `NavigationLink(destination: Profile) { ... }` pushes a statically resolved screen. Add `when: isAuthenticated` to a link to keep its content visible while disabling navigation when the Boolean guard is false. Screen state names must be unique across the app. Destinations must be declared in the same app. Screens may declare scalar route parameters, for example `screen Profile(userId: Int32, displayName: String) { ... }`; links and roots pass positional values with `Profile(userIdValue, "Ada")`. The compiler checks the destination, arity, and exact parameter types, then Swift uses typed route enum payloads and Kotlin uses typed native route arguments. Collections and nullable values are rejected as route parameters until their native encoding is defined. See [navigation-params.nx](../examples/navigation-params.nx) and [navigation-guard.nx](../examples/navigation-guard.nx).
- `Link(url: "https://example.com") { ... }` opens an external web URL or application scheme with the native platform handler. The URL accepts any expression whose type is `String`, including state, interpolation, and function results. Literal URLs are validated at compile time; dynamic values are checked by the native URL parser at render or click time and invalid values produce no link action. Swift uses SwiftUI `Link` and Android uses an `ACTION_VIEW` intent after checking that a handler exists. Incoming link events, universal-link routing, and URL availability expressions remain future work. See [linking.nx](../examples/linking.nx).
- `Accessibility(label: String, hint?: String, role?: Button|Link|Header|Image) { ... }` adds an accessible label, optional hint, and optional native role to any child subtree. The label and hint accept typed `String` expressions, including state, interpolation, and function results; literal values must be non-empty. Swift emits native accessibility modifiers and Android emits Compose semantics, importing `hintText`, the native `role` property, and heading semantics only when those features are used. Focus control, custom actions, and richer dynamic values remain future work. See [accessibility.nx](../examples/accessibility.nx).
- `FastList(count: rowCount, axis: Vertical|Horizontal|Grid(columns), rowHeight: positiveNumber, scrollPosition: position) { index in ... }` creates a virtualized list of `Int32` row indices. `axis` defaults to `Vertical`; vertical rows use iOS `UITableView` or Android `LazyColumn`, horizontal rows use iOS `UICollectionView` or Android `LazyRow`, and `Grid(columns)` uses a native compositional grid or `LazyVerticalGrid`. `columns` and `rowHeight` must be positive literals. `rowHeight` enables fixed native sizing; omit it to retain dynamic native sizing. The row block must declare exactly one binding (`index`). `scrollPosition` is optional and must be a mutable `Int32` state binding; it tracks the first visible row and applies external changes through native scroll APIs. Add `.onEndReached { ... }` to load another page, `.onScroll { ... }` to react once per first-visible-row change, or `.stickyHeader { ... }` to render a native sticky section header. Sticky headers are valid only for vertical lists; pagination and scroll callbacks do not run once per pixel, and the end callback re-arms when the source count changes.
- `FastList(labels, axis: Vertical|Horizontal|Grid(columns), rowHeight: positiveNumber, key: .self|.member, scrollPosition: position) { label, index in ... }` creates a virtualized list over an `Array<T>` state. The row block must declare exactly two bindings in item/index order; both are read-only and the item has the array element type. `key` accepts `.self` or one member path such as `.id`, and must resolve to a scalar `String`, `Bool`, or numeric type. Swift applies the identity to hosted cell identity and Compose passes it to the native lazy-list key slot; without a key, rows use their current position. `scrollPosition` is optional and must be a mutable `Int32` state binding; native paths report the first visible row/item when it changes and apply external index changes without animation. The collection is indexed directly without building an intermediate row array or row tree. `Set<T>` and `Map<K, V>` are not list sources. Add dot modifiers `.onEndReached { ... }`, `.onScroll { ... }`, or `.stickyHeader { ... }` as needed. When a `FastList` is the direct child of `RefreshControl`, refresh actions attach to the native list path without an intermediate wrapper. Sectioned data uses `FastList(sections: groups, key: .self, rowHeight: 48) { item, index, section in ... }.sectionHeader { ... }` with an `Array<Array<T>>` binding. The section/index/item values are read-only and sectioned lists support scalar keys, fixed heights, headers, and direct refresh integration. Sections reject non-vertical axes, ordinary `stickyHeader`, `scrollPosition`, `onEndReached`, and `onScroll` so the compiler never creates a flattened temporary collection. See [syntax-audit.md](syntax-audit.md) and the list examples.

## Compile-time permissions

For `FastList`, the generated iOS runtime uses self-sizing estimated rows and cells by default. A positive literal `rowHeight` switches vertical rows to a fixed `UITableView` height, horizontal cells to a fixed `UICollectionView` cell size, and grid cells to a fixed compositional height; Compose receives matching native `Modifier` constraints. A vertical `stickyHeader` uses the table's native section-header pinning and Compose's lazy-list sticky-header API. Sectioned lists use native table sections on iOS and direct nested `LazyColumn` iteration on Android; their optional `sectionHeader` block is the per-section header API. Omit `rowHeight` when rows need dynamic sizing.

Permissions and their iOS purpose messages live in the generated project
configuration, separate from the app UI source. `nexa generate` creates
`nexa.config.nx` automatically at the project root:

```nexa
config {
    permissions {
        camera: "This app uses the camera to capture photos.",
        photos: "This app uses your photos so you can choose images."
    }
}
```

The config parser validates permission names, duplicate entries, and non-empty
messages. The CLI uses each message for the corresponding iOS `Info.plist`
purpose string and uses the same permission set for Android manifest entries.
Supported names are `camera`, `microphone`, `photos`, `location`,
`notifications`, `contacts`, `calendar`, and `bluetooth`.
Notifications do not create an iOS usage-description key because Apple does not
require one; Android receives `POST_NOTIFICATIONS`. Editing the config and
regenerating updates the native host metadata without changing `.nx` UI code.
Permissions are configured only in this project file. The iOS mapping follows Apple's
[protected resources documentation](https://developer.apple.com/documentation/bundleresources/protected-resources),
including the required camera and photo-library usage-description keys. Calendar
projects receive the current full-access usage-description key.

Runtime status is available as `await Permissions.status(permission: Camera)`
and returns the typed `PermissionStatus.granted`, `.denied`, `.restricted`, or
`.notDetermined` value. iOS queries the native authorization frameworks and
Android checks the declared runtime permission with `AppOpsManager` using the
application context. Android exposes `granted`, `denied`, and
`notDetermined`; `restricted` is reserved for platforms that expose that state.
Typed `await Permissions.request(permission: Camera)` flows use the native authorization APIs and return the resulting status; only request calls emit the Android callback bridge and request helper. Status-only calls omit that machinery. Denial recovery and app-specific rationale UI remain future work. See
[permissions-status.nx](../examples/permissions-status.nx).
Both native backends tree-shake permission code. An app that uses only `Camera`
emits only the Camera enum case and status/request branches; Swift also omits
unneeded Apple framework imports, while Kotlin omits the location `combine`
helper unless Location is used. Dynamic permission values retain the complete
platform set conservatively.

### Plugin options in the project config

Plugin authors declare their supported compile-time options in
`interfaces.nxid`:

```text
config {
    compiledOptionCreateByThePluginAuthor: String
    optionalOption: Bool? = false
}
```

The generated project config sets values using the app's plugin namespace:

```nexa
config {
    permissions {
    }
    plugins {
        CustomPlugin {
            compiledOptionCreateByThePluginAuthor: "fast",
            optionalOption: true
        }
    }
}
```

Options are validated against the plugin schema at generation time. Required
options must be supplied, optional options may use `null`, and schema defaults
are applied when a value is omitted. Supported option values are scalar strings,
Booleans, integers, and floating-point numbers. Reachable plugins receive
native `NexaPluginConfig.CustomPlugin` constants; no runtime configuration
registry or serialization layer is added.

## Native network and file library

When an app uses a remote image or a typed native API call, the backend emits only the corresponding native helper fragments next to the generated screen: `NexaNetwork` for typed network calls, a private URLSession/Cronet transport for remote images, `NexaPath` for path calls, and `NexaFile` for file calls. Calls are resolved in the compiler and become direct platform calls without a cross-platform request registry.

Use the typed source forms `await Network.fetch(url: ...)` and `await Network.download(url: ..., destinationPath: ...)`. Both accept `method`, optional `body`, `headers`, `timeout` in seconds, `useCache`, `followRedirects`, `maxResponseBytes`, and optional `certificatePins`; omitted options use the documented defaults. `Network.fetch` returns a `NetworkResponse` with `statusCode`, `headers`, and decoded `body` text. `Path.documents()`, `Path.caches()`, `Path.temporary()`, `Path.appSupport()`, and `Path.join(path: ..., component: ...)` return native paths. `File.exists`, `await File.readText`, `await File.writeText`, and `await File.delete` use native file operations. Certificate pins are optional: iOS accepts lowercase SHA-256 hashes of the DER leaf certificate, while Android accepts lowercase SHA-256 public-key pins in the format expected by Cronet.

The iOS implementation configures `URLCache` with 16 MiB memory and 64 MiB disk capacity and uses `URLRequest.CachePolicy.useProtocolCachePolicy` by default. Optional certificate pins use the current Security trust-chain API. The Android implementation configures the Play Services Cronet engine with a 64 MiB disk cache, HTTP/2, QUIC, and Brotli; its private callback client is wrapped directly in suspend functions without a request registry. Image-only Android modules use a smaller GET-only callback client, so upload providers, file sinks, and the full request-option client are not emitted without a typed `Network` call. File reads and writes use native asynchronous APIs (`Task.detached` on iOS and `Dispatchers.IO` on Android). `NexaPath` provides `documents`, `caches`, `temporary`, `appSupport`, and `join` helpers.

The generated helpers are feature-gated to modules that use a remote image or one of these typed native calls. Path/file-only apps do not receive the Cronet or URLSession network helper, and offline apps remain free of networking imports, dependencies, and helper code. Image-only apps receive only the private URLSession or Cronet transport needed by the image loader; the public `Network.fetch`/`download` API is omitted. Android coroutines are added only when generated helper code for asynchronous network/file operations, remote images, or permission requests references them. The Kotlin backend publishes these requirements as a typed project-feature manifest, so CLI host scaffolding does not infer dependencies by scanning generated source text. See [native-apis.nx](../examples/native-apis.nx) for a complete source example.

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

Each component instance gets its own native state (`@State` on iOS and Compose `remember` on Android). Components can use core components and call other custom components. Parameters are immutable typed inputs; state declared inside a component stays private to it. A component can declare one reusable content slot with `Content()` and receive its content with a trailing block:

```nexa
component Card(title: String) {
    body {
        Column {
            Text(title)
            Content()
        }
    }
}

app ContentSlots {
    body {
        Card(title: "Account") {
            Text("Signed in")
        }
    }
}
```

The content block is lowered directly to a SwiftUI `@ViewBuilder` closure or a Compose `@Composable () -> Unit` parameter. A component that declares `Content()` requires a block at every call site; passing a block to a component without `Content()` is a compile-time error. `Content()` is valid only inside a custom component declaration.

Keep a component in its own `.nx` file and import it relative to the importing file with `import "components/CounterButton.nx"`. Imported files can import other component files; only the entry file declares an app. Imported components share one project-wide name scope. The compiler emits only components reachable from the app, with no runtime component registry. Recursive component composition and callback parameters are not supported yet. `NavigationLink` is also valid inside a custom component; Kotlin passes the native `NavHostController` only to components that need it. See [custom-components.nx](../examples/custom-components.nx), [custom-component-slot.nx](../examples/custom-component-slot.nx), [custom-component-navigation.nx](../examples/custom-component-navigation.nx), and its [component files](../examples/components/CounterButton.nx).

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

The Swift backend emits a SwiftUI `View`, stores mutable values in `@State`, and maps layouts and controls to `VStack`, `HStack`, `ZStack`, `Text`, `TextField`/`SecureField`, `Toggle`, `Image`, the generated URLSession-backed remote image view, `NavigationStack`/`NavigationLink`, `NavigationBack`, `Link`, `Button`, native Pressable haptics, and `TabView` for `AppBottomBar`. Focus bindings use native `@FocusState` storage without a duplicate initializer, and static `maxLength` limits use a direct `onChange` callback. Stepped `Int32` ranges convert only the stride argument to Swift's required `Int` type. Each retained Nexa struct is emitted once as a private Swift value struct, and constructor calls use its native initializer directly. Vertical `FastList` is backed by `UITableView`, horizontal lists use `UICollectionView` flow layout, and grids use compositional `UICollectionView` layouts with reusable cells and SwiftUI row content hosted by [`UIHostingConfiguration`](https://developer.apple.com/documentation/swiftui/uihostingconfiguration). A mutable `scrollPosition` binding reports the first visible native row/item only when its index changes and applies external positions through non-animated native scrolling. `onScroll` uses the same native delegate callback and dispatches its action only when that first-visible index changes; its Swift closure fields and forwarding code are emitted only for modules that use the event. The Kotlin backend emits a `@Composable` function and maps controls to Compose `Column`, `Row`, `Box`, `LazyColumn`, `LazyRow`, `LazyVerticalGrid`, `Text`, `TextField`, `Switch`, Coil 3 `AsyncImage`, AndroidX Navigation Compose `NavHost` with `NavigationBack` mapped to `popBackStack`, `Button`, a `clickable`/`combinedClickable` layout for `Pressable` with optional native haptic feedback, and Material 3 `Scaffold`/`NavigationBar` for `AppBottomBar`. Retained Nexa structs become private Kotlin data classes with direct constructors; unused struct declarations are removed during IR optimization. Remote `AsyncImage` nodes receive a remembered Coil `ImageLoader` backed by the generated Cronet client. `scrollPosition` and `onScroll` share one `snapshotFlow` over the lazy state's first visible index, so scroll actions are not dispatched for every pixel. Text input values bind directly to native state callbacks, submit blocks map to SwiftUI `.onSubmit` or Compose `KeyboardActions`, and static `maxLength` limits use Compose's direct `take` callback path. Optional availability markers and experimental opt-ins are emitted only on generated declarations that use the corresponding native feature. The compiler generates source code rather than a runtime component tree; generated files join an existing native app target.

Android apps that use a remote `Image` need [Coil 3 Compose](https://coil-kt.github.io/coil/compose/), Coil's `coil-network-core`, [Cronet from Google Play services](https://developer.android.com/develop/connectivity/cronet/start), and the Internet permission. Android apps that use only typed `Network` calls need Cronet and coroutines, but do not receive the Coil image adapter or its dependencies. `File.readText`, `File.writeText`, and `File.delete` add coroutines without adding Cronet; synchronous `File.exists` stays free of that dependency. `nexa generate` emits only the dependencies selected by the lowered features: remote images add `implementation("io.coil-kt.coil3:coil-compose:3.6.3")` and `implementation("io.coil-kt.coil3:coil-network-core:3.6.3")`; any Cronet request or remote image adds `implementation("com.google.android.gms:play-services-cronet:18.0.1")` and `implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")`; app `OnActive`, `OnInactive`, or `OnBackground` callbacks add `implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")`; generated code that imports `androidx.compose.ui.graphics` adds `implementation("androidx.compose.ui:ui-graphics")`. Compose tooling and preview dependencies are omitted because Nexa does not emit preview code. Generated Android projects use AGP 9.2.1 with built-in Kotlin 2.4.20, the Compose compiler plugin 2.4.20, compile/target SDK 37, Compose BOM 2026.09.00, and Navigation Compose 2.10.1 so predictive-back navigation stays on the current stable AndroidX APIs. The generated manifest opts into `android:enableOnBackInvokedCallback="true"`, and Navigation Compose provides the native predictive-pop behavior without a Nexa back-event runtime. The generated Coil adapter opts into the narrow `NetworkFetcher.Factory` API locally and does not expose that experimental dependency to app code. It adds `<uses-permission android:name="android.permission.INTERNET" />` and initializes `CronetProviderInstaller` in the generated `MainActivity` before Compose content starts. Generated code uses the Play Services provider through `CronetEngine.Builder`; do not add Coil's OkHttp network module to a Nexa app. Apps with navigation use `implementation("androidx.navigation:navigation-compose:2.10.1")`. When using `TextInput(autocorrect: ...)`, use a Compose UI version that supports `KeyboardOptions.autoCorrectEnabled` (1.7.0 or newer). `KeyboardAware` requires the Activity to use `android:windowSoftInputMode="adjustResize"` so Compose receives IME insets. Swift remote images use the generated URLSession-backed loader and require an iOS 15 or newer deployment target; Swift typed network-only apps omit the image view helper. SwiftUI navigation and `KeyboardAware` use iOS 16 or newer.

When `nexa generate` targets both platforms, the compiler loads and parses the entry/import graph once, then performs independent Swift and Kotlin semantic lowering. This removes duplicate frontend I/O without sharing target-specific IR or changing compile-time platform selection. `nexa build`, `nexa check`, and `nexa generate` fingerprint their relevant source graph and restore unchanged native output, diagnostics, or complete host projects from `.nexa/cache`.

## Current boundaries

Generated iOS projects use Xcode 27 project metadata, Swift 6 language settings, an iOS 17 deployment baseline, and require Xcode 27 or newer. The generated host keeps native SwiftUI output separate from Nexa compiler sources.

`nexa generate entry.nx --target ios|android|all --out AppProject --name AppName` creates a native project bundle with generated Swift/Kotlin sources, an iOS Xcode project and shared scheme, an Android Compose/Gradle project, platform dependency declarations, a `nexa.project.json` manifest, and a generated `nexa.config.nx` project configuration. Edit that config to change compile-time permissions and iOS purpose messages, or edit the `.nx` entry source, then regenerate; generated native files are outputs. Android release builds are configured with R8 code shrinking, resource shrinking, the optimized default Android rules, and an intentionally empty project-specific keep file because Nexa bindings use direct calls instead of reflection. iOS release builds use whole-module Swift `-O`, dead-code stripping, and size-oriented Clang optimization so generated hosts keep native release behavior without debug instrumentation. Generated iOS hosts target iOS 17 or newer; `UIHostingConfiguration` itself is available from iOS 16; vertical table rows use self-sizing estimated heights, vertical sticky headers use native pinned section headers, sectioned lists use native `UITableView` sections on iOS and direct nested `LazyColumn` iteration on Android, horizontal rows use a native `UICollectionView` flow layout, grids use a compositional `UICollectionView` layout, visible cells update in place, and data reloads only when the count changes. The generated Swift host includes only the FastList axis implementations used by the module. `onScroll` is coalesced by the first visible row/item index on both platforms, so each native scroll callback performs only one comparison before an action is emitted. Navigation supports named screens, one native stack, compile-time checked links, and screen-local `state`/`let` declarations. `NavigationBack(label: ...)` is available inside declared screens and lowers to SwiftUI dismiss or Compose `popBackStack`. Screen state declarations must appear before that screen's UI nodes and are lowered to direct native state storage; screen state names are unique across the app so generated Swift and Kotlin bindings remain unambiguous. Scalar typed route parameters and `NavigationLink(..., when: Bool)` guards are implemented; tabs, deep links, modals, and asynchronous authorization hooks remain future work. `Link` accepts static or typed dynamic `String` URLs; literal schemes are checked at compile time and dynamic values are guarded by each platform's native URL parser. Incoming links, URL availability expressions, and universal-link routing remain future work. `Accessibility` supports static or typed dynamic `String` labels and the `Button`, `Link`, `Header`, and `Image` roles; literal labels are validated while state and function expressions lower directly to native accessibility values. Hints, focus control, custom accessibility actions, and richer dynamic values remain future work. `Direction` currently supports one app-level static `LTR` or `RTL` override; dynamic language changes and direction-specific spacing/margins remain future work. `OnAppear` and `OnDisappear` support one top-level callback on the app body or each named screen; `OnAppear async` and typed `async fn` are implemented for direct lifecycle work. App roots additionally support one `OnActive`, `OnInactive`, and `OnBackground` callback lowered to Swift `scenePhase` or Android `LifecycleEventObserver`; richer cancellation controls remain future work. `FastList` supports integer ranges and primitive `Array<T>` collections on vertical, horizontal, and grid native list paths, with scalar keys, fixed `rowHeight` sizing, `scrollPosition` restoration/programmatic scrolling, `onEndReached` pagination callbacks, `onScroll` actions coalesced by first-visible index, direct native refresh integration through `RefreshControl`, and vertical `stickyHeader` blocks. Mutable element-level bindings remain future work; sectioned `FastList` sources are implemented with `Array<Array<T>>` and explicit section/index/item bindings. `KeyboardAware(dismiss: Interactive|Never)` handles native scrolling, IME inset adjustment, and static interactive or never-dismiss keyboard behavior; keyboard height/events, explicit focus scrolling, and programmatic dismissal remain future work. `StatusBar` supports static style, visibility, and hexadecimal background declarations at the app root or named screen level; animated transitions remain future work. `BottomSheet` currently supports a native modal with a mutable Boolean binding; snap points, custom transitions, and drag callbacks remain future work. `RefreshControl` supports native pull-to-refresh for generic content and direct `FastList` children; custom indicators and refresh event streams remain future work. Network and file APIs are emitted as native helpers and can be called from `.nx` with qualified `Network`, `Path`, and `File` expressions; typed error values remain future language work. Local plugins are declared as `plugin "path" as Namespace`, checked against `interfaces.nxid`, and lowered to direct Swift/Kotlin implementation calls; generated projects include their conventional platform source trees, while package installation and typed error recovery remain future work. `Pressable` currently supports press, long press, static or dynamic disabled state, and static native haptic feedback styles; hover, pressed-state styling, focus, and custom accessibility actions remain future work. `TextInput` supports native submit/Done callbacks, mutable focus bindings, and static positive `maxLength` limits enforced by the generated SwiftUI and Compose callbacks; selection, autofill, password-manager integration, richer validation, and richer submit actions remain future work. Pure app functions support typed parameters, ordered immutable local constants, one return expression, and native async boundaries; closures outside collection transformation callbacks, generic functions, enum associated-value patterns, loop expressions, and class/reference types remain future work. Struct construction and member access, including optional chaining, are implemented with native value types. Callback and lifecycle action blocks support direct native `for`/`while` loops with arrays, sets, Int32 ranges, positive literal steps, `break`, and `continue`; use `FastList` for UI repetition. Custom component callbacks, advanced theme features, FFI bindings, fine-grained incremental semantic compilation, formatting, and language-server support remain on the roadmap in `plan.md`. Compiler warnings and the current IR optimization pass are documented in [compiler-diagnostics.md](compiler-diagnostics.md).
