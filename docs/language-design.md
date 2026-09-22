# Language design decisions

Nexa borrows useful ideas from Kotlin and Swift, but keeps one source language and one typed IR. Syntax is selected for cross-platform clarity; Swift and Kotlin spelling differences are handled by the backends. Features enter the language when their type and runtime behavior are defined well enough to generate direct native code.

| Concept | Decision | Nexa design and status |
| --- | --- | --- |
| Condition (`if / else`) | Integrate now | Lowers directly to native branches for UI and event actions. |
| Platform-specific UI | Integrate now | `platform ios { ... }` and `platform android { ... }` are selected during target lowering; inactive widgets never reach the target IR. |
| Status bar | Integrate first slice | One top-level `StatusBar` declaration lowers to native SwiftUI status-bar modifiers and Android system UI flags. Background colors and animated transitions remain deferred. |
| Bottom sheet | Integrate first slice | `BottomSheet(isPresented: ...)` lowers to SwiftUI `.sheet` and Compose Material 3 `ModalBottomSheet`; advanced snap points and custom transitions remain deferred. |
| Refresh control | Integrate first slice | `RefreshControl` lowers to SwiftUI `.refreshable` and Compose Material 3 `PullToRefreshBox`; custom indicators and event streams remain deferred. |
| Application bottom bar | Integrate first slice | `AppBottomBar(selected: ...)` lowers to SwiftUI `TabView(selection:)` and Compose Material 3 `Scaffold`/`NavigationBar`; static labels and unique `Int32` tab indexes are supported while icons, badges, and custom transitions remain deferred. |
| Linking | Integrate first slice | `Link(url: ...)` lowers to SwiftUI `Link` and an Android `ACTION_VIEW` intent with handler checking. Incoming links, URL availability expressions, and universal-link routing remain deferred. |
| Accessibility | Integrate first slice | `Accessibility(label: ..., role: ...)` lowers to Swift accessibility modifiers and Compose semantics, including native heading semantics. Hints, focus control, and custom actions remain deferred. |
| RTL / LTR direction | Integrate first slice | `Direction(value: LTR|RTL)` applies a static native layout direction at the app root; dynamic locale changes and directional spacing APIs remain deferred. |
| Lifecycle callbacks | Integrate first slice | One top-level `OnAppear { ... }` and `OnDisappear { ... }` block is supported on the app body and named screens, lowering directly to SwiftUI lifecycle modifiers or Compose `LaunchedEffect(Unit)`/`DisposableEffect(Unit)`; app background events remain deferred. |
| Text weight and metrics | Integrate first slice | `Text` maps `fontWeight`, `lineLimit`, `lineHeight`, and `letterSpacing` directly to SwiftUI modifiers and Compose text parameters; custom text layout and nested spans remain deferred. |
| Button loading state | Integrate first slice | `Button(..., loading: Bool)` emits a native progress indicator and disables the native control while loading; omitting it keeps the direct minimal button path. |
| Network, paths, and files | Integrate native library now | Generated modules use URLSession on iOS and Play Services Cronet on Android. Fetch/download options, cache policy, redirect handling, byte limits, optional pinning, path directories, and asynchronous file operations map directly to native APIs; first-class `.nx` async call syntax is still being designed. |
| Multiple choice (`when / switch`) | Adopt after enums | Use one exhaustive `match` construct and emit Kotlin `when` or Swift `switch`; avoid separate source syntaxes and non-exhaustive UI states. |
| AND / OR / NOT | Integrate now | `&&`, `||`, `!`; native short-circuit operators. |
| Equality and comparison | Integrate now | `==`, `!=` for scalar values; numeric `<`, `<=`, `>`, `>=`. Exact types are required; no runtime conversions. |
| Mutable variable / constant | Keep Nexa terms | `state` means observed, mutable UI state and `let` means immutable. Both can infer a type from a non-empty initializer using shared defaults (`Int32` for integer literals and `Float64` for decimal literals); explicit annotations remain available for narrower numeric bindings. `var`/`val` aliases would add duplicate syntax. |
| Function / return | Adopt | Typed, statically resolved functions; avoid dynamic function registries. Not implemented yet. |
| Nullable / optional | Adopt | Explicit `T?` with checked unwrap semantics, mapped to Swift `Optional` and Kotlin nullable types. Not implemented yet. |
| Default value if null | Adopt one spelling | Prefer `??` in Nexa and lower to native short-circuit fallback; do not make Kotlin's `?:` the shared spelling. Depends on optionals. |
| Safe access | Adopt | `?.` with compile-time member/type checking; depends on optionals and user-defined types. |
| Loops | Adopt | Direct native `for`/`while` control flow. For UI collections, use `FastList` so large lists stay virtualized. Not implemented yet. |
| `break` / `continue` | Adopt with loops | Direct loop control, validated by the parser/type checker. |
| Range | Adopt with loops | A compact start/end/step IR, without eagerly materializing an array. Not implemented yet. |
| Anonymous function / closure | Adopt selectively | Permit closures at callback boundaries after capture and escape rules are explicit; avoid boxing or heap allocation for non-escaping callbacks. |
| `map / filter / reduce` | Adopt with optimization rules | Fuse non-escaping transforms into a single pass where possible; do not blindly emit allocation-heavy chained collection calls. |
| Array / List | Integrated | `Array<T>` lowers to Swift `Array<T>` and Kotlin read-only `List<T>`; literals are context-typed and mutable collection semantics are not implied. |
| Dictionary / Map | Initial value support integrated | `Map<K, V>` lowers to Swift `Dictionary<K, V>` and Kotlin read-only `Map<K, V>`. Keys are scalar `String`, `Bool`, or numeric values; map literals use last-value-wins for duplicate keys. Lookup and mutation are not implemented. |
| Set | Initial value support integrated | `Set<T>` lowers to native Swift/Kotlin set storage. Elements are scalar `String`, `Bool`, or numeric values; literals deduplicate, and iteration order is unspecified. Membership and mutation are not implemented. |
| Pair / Triple | Initial value support integrated | `Pair<A, B>` and `Triple<A, B, C>` lower to Swift tuples and Kotlin `Pair`/`Triple` values. Fields are type-checked by position; field access and collection operations are not implemented. |
| Class | Limit to identity cases | Prefer value types for ordinary models. Add references only where stable identity or native handles require them. |
| Constructor | Adopt with value types | Generate direct Swift initializers and Kotlin constructors; define initialization and mutability before exposing user types. |
| Inheritance | Avoid in core | Use composition and statically dispatched interfaces; class inheritance brings dynamic dispatch and fragile shared behavior. |
| Interface / protocol | Adopt as static constraints | Resolve implementations statically where possible; avoid implicit existential boxes in hot paths. |
| Enum | Adopt before matching | Closed, typed value cases enable efficient native enums/sealed representations and exhaustive `match`. |
| Extension | Defer as syntax sugar | Can desugar at compile time, but requires clear member lookup and conflict rules. |
| Generics | Adopt with monomorphization | Specialize statically to avoid runtime generic dispatch; track binary-size growth. |
| Type check (`is`) | Avoid general runtime checks | Prefer exhaustive matching on closed enums; no dynamic object hierarchy is planned for ordinary app models. |
| Type cast (`as / as?`) | Avoid unchecked casts | Add only explicit, type-checked conversions or optional casts when a real interop case needs them. |
| Exceptions | Prefer typed errors | Model recoverable cross-platform failures with `Result<T, E>`/typed effects; catch platform exceptions at native/plugin boundaries. |
| Access control | Adopt with modules | Enforce visibility at compile time; imports and namespaces need explicit module semantics first. |
| String interpolation | Integrate now | `$name` and `\(name)` lower into typed native Swift/Kotlin string interpolation for state and constant names; arbitrary expressions remain deferred. |
| Getter / setter | Defer | Computed properties can hide work or state changes; begin with explicit functions and state bindings, then add visible compile-time accessors if needed. |

The current operator and conditional syntax is demonstrated in [conditional-logic.nx](../examples/conditional-logic.nx). Component-local state and multi-file imports are described in the [language guide](language.md). This decision table is a design direction, not a claim that deferred features already compile.
