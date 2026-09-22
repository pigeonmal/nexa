Design and implement a high-performance cross-platform mobile framework that allows developers to build iOS and Android applications from a single codebase while preserving native performance, native UI components, and minimal runtime overhead.

The framework must prioritize near-native performance above all else.

The architecture must be modular, strongly typed, ahead-of-time compiled, and designed from the beginning so additional platform backends can be added in the future.

## Core goals

The framework must provide:

- One shared application codebase for iOS and Android.
- A small, statically typed programming language designed specifically for mobile application development.
- A compiler written in Rust.
- Ahead-of-time compilation.
- Native Swift/SwiftUI output for iOS.
- Native Kotlin/Jetpack Compose output for Android.
- No JavaScript runtime.
- No Dart VM.
- No interpreted UI runtime.
- No dynamic JavaScript-style bridge.
- No custom rendering engine.
- No unnecessary serialization layer.
- No JSON/RPC communication between application logic and native UI.
- Minimal shared runtime.
- Strong static typing.
- Predictable memory and CPU behavior.
- Extremely low abstraction overhead.
- Fast startup.
- Low memory usage.
- Smooth 60 Hz and 120 Hz rendering.
- Excellent performance for large and complex applications.

The framework should behave more like a native compiler toolchain than a traditional cross-platform runtime.

## User-facing language

Create a small statically typed programming language inspired by the best parts of:

- Swift
- Kotlin
- TypeScript

The language should feel modern, concise, readable, and familiar to mobile developers.

It must explicitly model important runtime characteristics instead of hiding them.

The type system must explicitly distinguish:

- value types,
- reference types,
- nullable types,
- non-nullable types,
- mutable values,
- immutable values,
- precise numeric types.

Numeric types should include explicit types such as:

- Int8
- Int16
- Int32
- Int64
- UInt8
- UInt16
- UInt32
- UInt64
- Float32
- Float64

The current compiler supports the scalar types above plus `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>` as contextually typed value declarations and literals. These map directly to Swift value types/tuples and Kotlin collection or tuple-value types. Set elements and map keys are currently limited to scalar `String`, `Bool`, and numeric types for native hashability. Collection lookup, mutation, iteration, and transformations are still roadmap work.

Generated modules that use remote images also receive a feature-gated native network/file library. iOS uses URLSession with a 16 MiB memory and 64 MiB disk URLCache; Android uses Play Services Cronet with a 64 MiB disk cache, HTTP/2, QUIC, and Brotli, and Coil 3 is wired to that same Cronet client. The library exposes asynchronous fetch/download options, optional certificate pinning, path directories, and asynchronous file reads/writes without a shared runtime bridge.

Avoid implicit numeric conversions that could create unpredictable behavior or performance costs.

The implemented expression/control-flow slice includes short-circuit boolean operations, scalar equality, numeric comparisons, and `if`/`else` branches in UI bodies and event handlers. These lower directly to the target language's native operators and branches. See [language design decisions](docs/language-design.md) for choices around the remaining Kotlin/Swift concepts.

String interpolation is implemented for `$name` and `\(name)` segments. Names are resolved and type-checked in the shared IR, then emitted as native Swift/Kotlin interpolation; arbitrary expressions inside a string remain future work.

For a fast authoring path, `let` and mutable `state` declarations may omit their type when the compiler can infer it from a non-empty initializer. Integer literals default to `Int32` and decimal literals to `Float64`; component parameters remain explicitly typed. Inference is compile-time only and does not add runtime metadata or alter native output for explicitly typed source.

## User-defined components and modules

App authors should be able to create reusable UI components in Nexa source files, pass typed inputs, compose built-in and custom components, and declare private per-instance state. Component files should be reusable through relative imports and resolve at compile time. Generated output should use native view/composable declarations without a dynamic registry or cross-platform component runtime.

The first implementation supports typed parameters, private state, nested custom components, relative `.nx` imports, import-cycle diagnostics, and reachability-based output pruning. Callback properties, content slots, navigation links from components, explicit visibility/module namespaces, and shared state bindings remain future work.

The first `StatusBar` slice is implemented as one top-level app declaration with static `style` (`Default`, `Light`, or `Dark`) and `hidden` options. It lowers to SwiftUI status-bar modifiers and direct Android system UI flags; background configuration and animated transitions remain future work.

Compile-time platform blocks are supported with `platform ios { ... }` and `platform android { ... }`. Target-specific lowering removes the inactive block before semantic analysis and backend generation, so platform selection adds no runtime branch or cross-platform UI wrapper.

The language should support at minimum:

- variables and constants,
- functions,
- structs,
- classes or reference types,
- enums,
- generics,
- protocols/interfaces,
- pattern matching,
- optional/nullability handling,
- async/await,
- closures,
- modules,
- imports,
- visibility modifiers,
- error handling,
- basic metaprogramming only when it can remain compile-time oriented.

Avoid runtime reflection unless absolutely necessary.

Prefer compile-time resolution wherever possible.

## Compiler

The compiler must be implemented in Rust.

Use the following main pipeline:

Source
→ Lexer
→ Parser
→ AST
→ Semantic Analysis
→ Type Checker
→ Common Intermediate Representation
→ Optimization
→ Backend Code Generation

The common IR must be platform-independent.

The IR should make it possible to add other backends later without redesigning the language frontend.

Potential future backends could include:

- macOS
- Windows
- Linux
- Web
- embedded platforms

The IR should represent:

- types,
- functions,
- ownership/lifetime-relevant information where useful,
- UI declarations,
- state relationships,
- async operations,
- platform calls,
- plugin calls,
- FFI calls,
- navigation,
- styling,
- animations.

Apply compile-time optimization aggressively.

The current compiler performs a conservative IR pass that folds pure literal boolean/scalar/numeric expressions, removes unreachable UI and event branches, and removes unused state declarations. It runs after semantic lowering and before either native backend, so it adds no runtime layer and keeps native component mappings unchanged. The compiler also reports unused declarations and constant conditions; `--deny-warnings` promotes those diagnostics to a failed check or build. See [compiler diagnostics and optimization](docs/compiler-diagnostics.md).

Potential optimizations should include:

- constant folding,
- dead code elimination,
- static dispatch,
- generic specialization,
- tree shaking,
- unused style removal,
- static property resolution,
- compile-time layout/style simplification,
- elimination of unnecessary wrappers,
- elimination of unnecessary allocations,
- inlining where appropriate.

## iOS backend

The iOS backend must generate native Swift.

UI code must primarily target SwiftUI.

Use UIKit only where necessary for functionality or performance that SwiftUI cannot provide efficiently.

Generated code should look and behave like normal native Swift code.

There must be no dynamic bridge between the shared application code and generated Swift UI.

Shared framework components should map as directly as possible to SwiftUI/UIKit primitives.

Examples:

Column → SwiftUI container primitives

Text → SwiftUI Text

Image → SwiftUI Image or the generated URLSession-backed image implementation

TextInput → TextField / SecureField / UIKit equivalent when needed

Button → SwiftUI Button

Switch → Toggle

Navigation → NavigationStack / UIKit navigation where necessary

Animations → SwiftUI/Core Animation

Lists → native SwiftUI/UIKit virtualization depending on performance requirements

## Android backend

The Android backend must generate native Kotlin.

UI code must primarily target Jetpack Compose.

Generated code should look and behave like normal native Kotlin/Compose code.

There must be no dynamic bridge between the shared application code and generated Kotlin UI.

Shared framework components should map as directly as possible to Jetpack Compose and Android primitives.

Examples:

Column → Compose layout primitives

Text → Compose Text

Image → Coil 3 with the generated Cronet-backed network client

TextInput → TextField / BasicTextField

Button → Compose Button

Switch → Compose Switch

Navigation → native Android / Compose navigation primitives

Animations → Compose animation APIs

Lists → LazyColumn / LazyRow or lower-level optimized implementations where needed

## Native rendering

Do not build a custom rendering engine.

Do not create a Skia-based cross-platform widget tree.

Do not render the entire application into Canvas, Metal, OpenGL, Vulkan, or custom graphics surfaces unless explicitly required for a specialized component.

The framework must directly use the native UI systems.

On iOS:

- SwiftUI
- UIKit
- Core Animation where appropriate

On Android:

- Jetpack Compose
- Android native UI APIs where appropriate

The native platforms must remain responsible for:

- accessibility,
- text rendering,
- native gestures,
- layout execution where possible,
- controls,
- platform behavior,
- input,
- focus,
- native animation execution.

## Minimal runtime

The shared runtime must remain extremely small.

It should only provide functionality that cannot reasonably be compiled away.

Core runtime responsibilities may include:

- state,
- lifecycle,
- async scheduling integration,
- observation,
- minimal event handling,
- a few essential primitives.

Avoid creating a large framework scheduler.

Avoid maintaining a duplicate cross-platform UI tree at runtime unless absolutely necessary.

Prefer platform-native state and rendering mechanisms where this can be done without sacrificing shared semantics.

## Core first-party UI components

The framework must include a production-ready set of first-party components.

At minimum implement:

- Column
- Pressable
- Text
- Image
- TextInput
- Button
- Switch
- FastList
- StatusBar
- AppBottomBar
- RefreshControl
- BottomSheet

These components must be part of the core framework.

They should not require third-party dependencies.

Each component must compile as directly as possible into native platform components.

## Column

Column provides the basic vertical layout and composition primitive.

It should support:

- children,
- layout,
- spacing,
- alignment,
- padding,
- margin where appropriate,
- background,
- borders,
- clipping,
- transforms,
- accessibility,
- gestures,
- responsive styling.

Avoid unnecessary native wrapper views.

The compiler should flatten or eliminate layout containers that are semantically unnecessary.

## Pressable

Pressable must provide high-performance gesture and interaction handling.

Support:

- press,
- long press,
- pressed state,
- disabled state,
- hover where supported,
- focus,
- accessibility actions,
- haptic integration where appropriate.

Map directly to native gesture/event systems.

## Text

Text must use native text rendering.

The first styling slice is implemented: `Text` accepts static `fontWeight` (`Normal`, `Medium`, `Semibold`, `Bold`), positive `lineLimit`, non-negative `lineHeight`/`letterSpacing`, and native `selectable` text, lowering directly to SwiftUI and Compose text APIs. See [text-style.nx](examples/text-style.nx).

Support:

- text styles,
- fonts,
- weights,
- line height,
- letter spacing,
- truncation,
- selectable text,
- accessibility,
- RTL/LTR,
- nested text spans where practical.

Do not create a custom text layout engine.

## Image

Image should support:

- local assets,
- remote images,
- resizing,
- aspect fit/fill,
- placeholders,
- caching,
- loading states,
- decoding optimization,
- memory-efficient image loading.

Use platform-native image pipelines where appropriate.

The abstraction must allow optimized third-party/native image loaders to be plugged in.

## TextInput

TextInput must be strongly integrated with native input systems.

Support:

- plain text,
- secure input,
- multiline input,
- keyboard type,
- autocorrect,
- capitalization,
- focus,
- selection,
- submit actions,
- validation,
- native autofill,
- password managers,
- accessibility.

Input latency should be equivalent or extremely close to native applications.

## Button

Button must map directly to native button/interaction primitives.

The first loading slice is implemented: `Button(..., loading: Bool)` emits a native `ProgressView` or `CircularProgressIndicator` and disables the button while loading. See [button-loading.nx](examples/button-loading.nx).

Support:

- disabled state,
- loading state,
- accessibility,
- native press semantics,
- styling,
- icons,
- text.

## Switch

Switch must use native switching behavior and accessibility.

Avoid recreating the control manually unless explicitly requested by the developer.

## FastList

Create a first-party extremely high-performance virtualized list.

This is a major priority.

It must support:

- very large datasets,
- smooth 60/120 Hz scrolling,
- vertical lists,
- horizontal lists,
- grids where possible,
- sections,
- sticky headers,
- item recycling where appropriate,
- native virtualization,
- predictable memory usage,
- item keys,
- pagination,
- infinite scrolling,
- pull-to-refresh integration,
- programmatic scrolling,
- scroll events,
- scroll position restoration,
- dynamic item sizes,
- fixed item size optimization,
- estimated sizes.

The implementation should minimize:

- allocations,
- temporary objects,
- layout work,
- component re-instantiation,
- memory churn,
- unnecessary state updates.

Use native list primitives or lower-level native APIs whenever they provide better performance.

The iOS `FastList` default is a native `UITableView` with reusable cells and SwiftUI row content hosted through `UIHostingConfiguration`. This keeps UIKit responsible for cell virtualization and reuse without creating an intermediate row array. Horizontal, grid, or sectioned list components may use a different native collection primitive when those capabilities are implemented and measured.

Android may use Compose lazy layouts or lower-level native mechanisms when necessary.

The goal is performance comparable to highly optimized native lists.

## StatusBar

The first cross-platform `StatusBar` API slice is implemented. It accepts one app-level static declaration with `style: Default|Light|Dark` and `hidden: true|false`, then lowers directly to SwiftUI status-bar modifiers or Android system UI flags.

Remaining work:

- background configuration where supported,
- animated transitions,
- screen-specific configuration.

The compiler rejects nested and repeated declarations and emits no shared status-bar runtime. The current style applies to the app root; screen-specific overrides remain future work.

## Application bottom bar

The first native `AppBottomBar` slice is implemented as:

```nexa
state selected: Int32 = 0

AppBottomBar(selected: selected) {
    Tab(index: 0, label: "Home") {
        Text("Home")
    }
    Tab(index: 1, label: "Settings") {
        Text("Settings")
    }
}
```

Swift lowers to `TabView(selection:)` with native tab items. Android lowers to Material 3 `Scaffold` and `NavigationBar`, with the active content selected by a direct `when` branch. This keeps tab content statically generated once per tab and leaves selection as ordinary typed mutable state.

The current syntax supports:

- tabs,
- selected state,
- labels,
- native safe area handling through the target tab/navigation primitive.

Remaining work:

- icons,
- badges,
- native transition customization,
- platform-specific customization.

Use native tab/navigation primitives wherever possible.

## RefreshControl

The first pull-to-refresh slice is implemented as `RefreshControl(isRefreshing: mutableBool) { ... } { ... }`. Swift lowers to `.refreshable`; Kotlin lowers to Material 3 `PullToRefreshBox` with the callback block as its native refresh handler.

Remaining work:

- customizable indicator where supported,
- refresh event streams and cancellation state.

The current implementation uses native gesture and scrolling behavior and does not add manual scrolling physics.

## BottomSheet

The first native bottom sheet slice is implemented with `BottomSheet(isPresented: mutableBool) { ... }`. Swift lowers to `.sheet(isPresented:)`; Kotlin lowers to Material 3 `ModalBottomSheet` with native dismissal updating the binding.

Remaining work:

- partial-height sheets,
- snap points,
- drag gestures,
- dismiss gestures,
- dynamic content,
- safe areas,
- keyboard integration.

Use native platform sheets when available. The current slice does not add a shared sheet runtime or duplicate the sheet content in the generated tree.

Provide optimized platform-specific implementations when required.

## Keyboard management

Keyboard behavior must be treated as a core framework concern.

Provide:

- keyboard-aware layout,
- automatic inset handling,
- focused input tracking,
- scroll-to-focused-input behavior,
- interactive keyboard dismissal,
- keyboard show/hide events,
- keyboard height,
- safe area integration,
- keyboard-driven animations.

Avoid JavaScript-style keyboard event bridges.

Integrate directly with native keyboard and window inset APIs.

Keyboard animation synchronization must be native and smooth.

## Layout

Layout must rely primarily on native platform layout systems.

The shared layout abstraction should compile into native layout primitives.

Do not introduce a full duplicated layout engine unless required.

Support:

- row,
- column,
- stack,
- overlay,
- absolute positioning where appropriate,
- flexible sizing,
- minimum/maximum sizes,
- percentage sizing where practical,
- safe areas,
- screen constraints,
- responsive layouts.

Avoid layout passes that duplicate work already performed by the native platform.

## Styling system

Create a strongly typed styling system.

Styles should be statically analyzable.

The compiler should resolve as much styling information as possible at compile time.

Support:

- width,
- height,
- min/max dimensions,
- padding,
- spacing,
- alignment,
- colors,
- backgrounds,
- borders,
- radius,
- opacity,
- shadows,
- transforms,
- typography,
- visibility,
- platform-specific styles,
- responsive styles.

Styles should not be represented as dynamic dictionaries of arbitrary values at runtime.

Prefer strongly typed structs or compiler-known style objects.

Static styles should have effectively zero runtime parsing overhead.

## Themes

The initial implementation provides one app-level compile-time theme block with typed adaptive light/dark color tokens and static spacing, radius, and font-size tokens. Theme values lower into the platform-independent IR and emit native system-appearance checks only for adaptive colors referenced by the app. This covers the first theme milestone; the capabilities below remain part of the broader roadmap.

Provide first-class theme support.

Developers must be able to define application themes.

Support:

- colors,
- typography,
- spacing,
- radius,
- shadows,
- component tokens,
- custom design tokens.

Support dynamic theme switching.

Examples:

- light theme,
- dark theme,
- OLED theme,
- brand theme,
- user-selected theme.

Theme updates must be efficient.

Only affected components should update.

Avoid forcing complete application reconstruction when unnecessary.

Support system theme detection.

## Dynamic and responsive styling

The current compiler implements the first responsive predicate, `Layout.isRegularWidth`, as a typed `Bool`: Swift checks `horizontalSizeClass == .regular`, while Compose checks whether the current configuration width is at least `600dp`. It can drive ordinary `.nx` conditional branches without adding a wrapper layout. This is a platform-native width hint; the broader device, orientation, safe-area, and breakpoint API below remains roadmap work.

Support responsive styles based on:

- phone/tablet,
- screen dimensions,
- width classes,
- height classes,
- orientation,
- platform,
- display scale,
- safe area,
- device capabilities.

Developers should be able to express rules similar to:

phone → use compact layout

tablet → use two-column layout

portrait → stack content vertically

landscape → show side-by-side layout

The framework must avoid expensive runtime style recomputation.

Use compile-time generated decision branches where possible.

## Animations

Animations must use native animation systems.

On iOS use:

- SwiftUI animations,
- Core Animation,
- UIViewPropertyAnimator where appropriate.

On Android use:

- Compose animations,
- native Android animation APIs where appropriate.

Animations should not depend on a centralized framework animation loop when native animation systems can execute them directly.

Support:

- spring animations,
- timing animations,
- transitions,
- transforms,
- opacity,
- layout animations,
- gestures driving animations,
- shared element-style patterns where practical.

Animations must remain smooth at 60/120 Hz.

Avoid cross-language calls on every frame.

## State management

Provide a minimal but high-performance state model.

Support:

- local state,
- observable state,
- derived/computed state,
- immutable state patterns,
- shared application state.

Changes should trigger only necessary UI updates.

Avoid global tree diffing when it can be avoided.

Prefer fine-grained observation.

Compile dependency relationships statically when possible.

## Lifecycle

Expose a common lifecycle abstraction.

The first slice is implemented as one top-level `OnAppear { ... }` or `OnDisappear { ... }` callback on the app body or each named screen. Their direct state actions lower to SwiftUI lifecycle modifiers and Compose `LaunchedEffect(Unit)`/`DisposableEffect(Unit)` without a shared lifecycle runtime. See [lifecycle.nx](examples/lifecycle.nx) and [navigation.nx](examples/navigation.nx).

Support concepts such as:

- app active,
- app inactive,
- app backgrounded,
- screen appeared,
- screen disappeared.

Map lifecycle events directly to native lifecycle systems.

## Async

Provide first-class async/await support.

The implementation must integrate efficiently with:

- Swift concurrency on iOS,
- Kotlin coroutines on Android.

Avoid maintaining a separate heavyweight async runtime if native runtimes can handle execution.

The compiler may transform shared async semantics into native Swift/Kotlin async constructs.

## Linking

The first `Link` slice is implemented:

```nexa
Link(url: "https://example.com") {
    Text("Open website")
}
```

Static URLs with a valid scheme lower to SwiftUI `Link` on iOS. Android emits an `ACTION_VIEW` intent and checks `resolveActivity` before opening it. This keeps external navigation in the platform handler without a shared runtime or dynamic event registry.

Remaining work:

- dynamic URL expressions,
- universal links,
- Android app links,
- deep links,
- incoming link handling,
- source-level URL availability checks.

Use direct native APIs.

## Navigation

Navigation is a core framework feature.

Provide a statically typed navigation system.

Support:

- screens,
- stacks,
- tabs,
- nested navigation,
- modals,
- bottom sheets,
- parameters,
- deep linking,
- back navigation,
- navigation guards/hooks where appropriate.

Routes and route parameters should be compile-time checked.

Example:

Screen<ProfileParams>

should make invalid navigation parameters a compile-time error.

Avoid dynamic route dictionaries.

Use native navigation systems whenever possible.

On iOS use SwiftUI NavigationStack/UIKit navigation where appropriate.

On Android use Compose/native Android navigation concepts.

Navigation transitions should remain native.

## Permissions

Provide a typed cross-platform permissions API.

Support common permissions such as:

- camera,
- microphone,
- photos/media,
- location,
- notifications,
- contacts,
- calendar,
- Bluetooth.

Expose platform differences explicitly when needed.

Permissions should have strongly typed status values.

For example:

PermissionStatus.granted

PermissionStatus.denied

PermissionStatus.restricted

PermissionStatus.notDetermined

Platform-specific permissions must remain accessible.

## RTL and LTR

The first static direction slice is implemented:

```nexa
body {
    Direction(value: RTL)
    Column(alignment: Start) {
        Text("مرحبا")
    }
}
```

Swift applies the native `layoutDirection` environment value. Android provides `LocalLayoutDirection` with Compose's native `LayoutDirection`. Omitting the declaration preserves the platform/system direction.

Remaining work:

- dynamic language and locale changes,
- directional padding and margin syntax,
- explicit direction-aware spacing tokens.

Do not rely on manual component mirroring when native platform behavior can perform it automatically.

## Accessibility

The first accessibility slice is implemented:

```nexa
Accessibility(label: "Open settings", role: Button) {
    Button("Settings") {}
}
```

Swift lowers labels and roles to native accessibility modifiers. Android lowers them to Compose semantics, using `contentDescription`, `Role` where the platform exposes a matching role, and `heading()` for `Header`. The wrapper emits no shared accessibility runtime.

Remaining work:

- hints,
- programmatic focus,
- custom accessibility actions,
- dynamic accessibility values,
- reduced-motion and high-contrast environment bindings.

The framework must preserve the accessibility advantages of native controls.

## Plugin system

Create a strongly typed plugin system using a common IDL.

The plugin system should take inspiration from systems such as:

- Nitro
- UniFFI

Plugins must expose typed interfaces.

Example:

interface Camera {
async takePhoto(options: CameraOptions): Result<Photo, CameraError>
}

The framework compiler/code generator should automatically generate:

- shared language bindings,
- Swift interfaces and bindings,
- Kotlin interfaces and bindings,
- Rust/C bindings when needed.

Avoid:

- JSON serialization,
- generic RPC,
- dynamic dictionaries,
- runtime reflection,
- string-based method dispatch.

Prefer direct typed calls.

The plugin architecture must minimize boundary-crossing overhead.

## Platform-specific implementations

A shared API must be able to provide separate platform implementations.

For example:

Shared interface:

interface Haptics {
impact(style: ImpactStyle): Void
}

iOS implementation:

Swift

Android implementation:

Kotlin

The compiler/build system must automatically connect the appropriate native implementation.

Platform-specific implementation files should be first-class citizens of the framework.

Support concepts similar to:

Haptics.ios.swift

Haptics.android.kt

or an equivalent project structure.

## Rust and C / C++ FFI

Rust and C / C++ libraries must be directly usable from applications and plugins.

FFI overhead must be minimal.

Avoid unnecessary intermediate objects.

Avoid serialization whenever ABI-safe values can be transferred directly.

Support efficient transfer of:

- integers,
- floating-point values,
- booleans,
- enums,
- structs where ABI-safe,
- byte buffers,
- strings,
- handles/references.

Provide safe generated wrappers around unsafe FFI boundaries.

The framework should make Rust especially easy to integrate because the compiler itself is written in Rust.

Possible architecture:

Application language
→ generated native binding
→ direct C ABI / UniFFI-style boundary
→ Rust library

Optimize hot-path FFI calls.

## Memory management

The language and compiler must have explicit semantics for value and reference types.

Generated Swift should integrate naturally with ARC.

Generated Kotlin should integrate naturally with JVM/Android memory management.

Avoid creating unnecessary shared wrapper objects.

Value types should preferably compile into native value-like types.

Reference types should preserve identity.

The compiler should understand when copying occurs.

Avoid hidden expensive copies.

## Events

Events should use direct generated native callbacks.

Avoid a generic event bus for component events.

For example:

onPress

should ideally compile directly into a native closure/callback.

Avoid converting events into dynamically typed event dictionaries.

## Build system

Create a project/build system that handles:

- shared source compilation,
- Rust compiler execution,
- iOS project generation/integration,
- Android project generation/integration,
- native dependencies,
- plugins,
- incremental builds,
- development builds,
- release builds,
- code generation.

Incremental compilation should be a major priority.

Changing one screen should not require recompiling the entire application when avoidable.

Generated native code should be cacheable.

## Developer experience

Despite prioritizing performance, developer experience should remain excellent.

Provide:

- clear compiler errors,
- source locations,
- type errors,
- plugin errors,
- generated code diagnostics,
- IDE-friendly project structure,
- formatter,
- language server,
- syntax highlighting,
- autocomplete,
- go-to-definition,
- incremental compilation.

The framework should eventually provide an LSP implementation.

## Debugging

Development builds should provide useful debugging capabilities without imposing overhead on release builds.

Potential functionality:

- state inspection,
- navigation inspection,
- layout debugging,
- performance metrics,
- logging.

Debug-only instrumentation must be completely removable from release builds.

## Release builds

Release builds must aggressively optimize generated output.

Release mode should:

- remove debug tooling,
- remove unused components,
- remove unused plugins,
- eliminate unused styles,
- perform dead-code elimination,
- specialize generics,
- inline hot functions,
- minimize runtime metadata.

The goal is native-like binary behavior.

## Architecture rules

The architecture must follow these principles:

1. Compile instead of interpret.

2. Generate instead of dynamically dispatch.

3. Use native platform capabilities instead of recreating them.

4. Resolve information at compile time whenever possible.

5. Prefer static dispatch over dynamic dispatch.

6. Prefer typed bindings over generic bridges.

7. Prefer direct calls over message passing.

8. Prefer native animation execution over framework-controlled frame loops.

9. Prefer native UI layout over duplicated layout engines.

10. Prefer minimal runtime state.

11. Avoid unnecessary allocations.

12. Avoid unnecessary copies.

13. Avoid serialization across internal framework boundaries.

14. Avoid reflection-heavy architectures.

15. Avoid runtime component registries when compile-time resolution is possible.

16. Avoid string-based method lookup.

17. Avoid runtime parsing of styles.

18. Avoid unnecessary wrapper views.

19. Avoid unnecessary intermediate representations after code generation.

20. Make every abstraction pay for itself.

## Zero-overhead philosophy

Treat zero overhead as a core architectural constraint, not a marketing claim.

The framework should aim for zero abstraction overhead wherever technically possible.

A simple component such as:

Column {
Text("Hello")
}

should compile into approximately the same native UI operations a developer would write manually in SwiftUI or Jetpack Compose.

There should not be an additional cross-platform UI runtime interpreting this structure.

Avoid architectures resembling:

Application code
→ framework runtime
→ bridge
→ serialized message
→ native runtime
→ native UI

Prefer:

Application source
→ compiler
→ generated Swift/Kotlin
→ native UI

For native libraries prefer:

Application source
→ generated typed binding
→ native ABI call
→ Rust/C

The generated output should contain as little framework machinery as possible.

## Performance requirements

Performance is the absolute priority.

Optimize for:

- startup time,
- CPU usage,
- memory usage,
- binary size,
- UI latency,
- input latency,
- list performance,
- animation performance,
- navigation speed,
- FFI latency,
- build performance.

Target native-quality interaction latency.

Ensure no framework boundary is crossed on every animation frame unless absolutely unavoidable.

Avoid allocations on hot paths.

Avoid creating temporary maps, dictionaries, JSON values, boxed primitives, or reflection metadata during normal UI rendering.

## Benchmarking

Create benchmarks comparing the framework against equivalent native implementations.

Benchmark:

- cold startup,
- warm startup,
- memory consumption,
- simple screen rendering,
- navigation,
- list scrolling,
- 1,000-item list,
- 10,000-item list,
- 100,000-item virtualized list,
- TextInput latency,
- animation frame stability,
- state updates,
- FFI calls,
- plugin calls.

Compare generated iOS applications against handwritten Swift/SwiftUI.

Compare generated Android applications against handwritten Kotlin/Jetpack Compose.

Benchmark framework overhead separately from platform overhead.

Performance regressions should be detectable automatically in CI.

## Future backend support

Do not hard-code the language frontend around Swift or Kotlin.

The compiler architecture should make additional backends possible.

Use a stable common IR and backend interface.

Conceptually:

Frontend
→ Common IR
→ Swift backend
→ Kotlin backend
→ Future backend X
→ Future backend Y

Platform APIs should expose capability abstractions instead of assuming only iOS and Android will ever exist.

## Initial implementation order

Build the project incrementally.

Start with:

1. Language grammar.
2. Lexer.
3. Parser.
4. AST.
5. Type system.
6. Type checker.
7. Common IR.
8. Simple Swift backend.
9. Simple Kotlin backend.
10. Column.
11. Text.
12. Button.
13. State.
14. Styling.
15. Events.
16. TextInput.
17. Image.
18. Pressable.
19. Switch.
20. Navigation.
21. Lists.
22. Keyboard handling.
23. Themes.
24. Responsive styles.
25. Animations.
26. Bottom sheets.
27. RefreshControl.
28. StatusBar.
29. AppBottomBar.
30. Linking.
31. Permissions.
32. RTL/LTR.
33. Plugin IDL.
34. Swift plugin bindings.
35. Kotlin plugin bindings.
36. Rust/C FFI.
37. Optimization passes.
38. Incremental compilation.
39. Tooling/LSP.
40. Performance benchmark suite.

At each step, keep the architecture usable and compilable.

Do not build unnecessary abstractions in advance.

## Final objective

The final result should feel like a modern cross-platform framework to application developers while behaving internally much closer to a native compiler.

Developer experience:

one language,
one application codebase,
strong typing,
modern declarative UI,
shared application logic,
shared component APIs.

Runtime behavior:

generated native Swift,
generated native Kotlin,
native SwiftUI,
native Jetpack Compose,
native navigation,
native text,
native accessibility,
native animations,
native controls,
native FFI,
minimal shared runtime,
no JavaScript/Dart VM,
no dynamic bridge,
no custom rendering engine,
no unnecessary serialization.

The primary design objective is:

Maximum performance.
Minimum overhead.
Maximum native integration.
Strong compile-time guarantees.
Modular architecture.
Future backend extensibility.

Whenever there is a trade-off between framework convenience and runtime efficiency, choose the solution that minimizes runtime overhead while maintaining a clean API, strong type safety, maintainability, and predictable behavior.

Do not imitate React Native or Flutter internally.

The framework should instead be designed as an AOT source-to-native compiler that turns a shared statically typed mobile language into optimized native applications.
