# Nexa

Nexa is an early ahead-of-time compiler prototype for a shared mobile language that emits native SwiftUI and Jetpack Compose source. Its frontend and typed intermediate representation are Rust; generated applications use the platform UI frameworks directly and do not include a JavaScript or Dart runtime.

The current implementation covers the compiler foundation and a growing native-control slice. It parses app state, checks primitive and collection value types and bindings, lowers to a platform-independent IR, and emits native SwiftUI or Compose controls, including virtualized range and collection lists. It is not yet a complete mobile framework or a project generator.

Nexa's product direction is to let people create complete mobile apps from `.nx` without writing native source. The current compiler supports only the documented language slice below; it does not yet generate complete iOS or Android projects. The core component set comes first, while integrations such as SQLite, MMKV, and maps are planned as optional plugins. Its first theme slice compiles typed color, spacing, radius, and font-size tokens directly into native code.

## Build and use

```sh
cargo build --workspace
cargo run -p nexa-cli -- check examples/counter.nx
cargo run -p nexa-cli -- build examples/counter.nx --target swift
cargo run -p nexa-cli -- build examples/counter.nx --target kotlin --out CounterScreen.kt
cargo run -p nexa-cli -- check examples/navigation.nx
cargo run -p nexa-cli -- check examples/virtualized-list.nx
cargo run -p nexa-cli -- check examples/collection-list.nx
cargo run -p nexa-cli -- check examples/collection-values.nx
cargo run -p nexa-cli -- check examples/themed-app.nx
cargo run -p nexa-cli -- check examples/custom-components.nx
cargo run -p nexa-cli -- check examples/function-locals.nx
cargo run -p nexa-cli -- check examples/conditional-logic.nx
cargo run -p nexa-cli -- check examples/responsive-layout.nx
cargo run -p nexa-cli -- check examples/constant-branches.nx
cargo run -p nexa-cli -- check examples/interpolation.nx
cargo run -p nexa-cli -- check examples/status-bar.nx
cargo run -p nexa-cli -- check examples/bottom-sheet.nx
cargo run -p nexa-cli -- check examples/refresh-control.nx
cargo run -p nexa-cli -- check examples/app-bottom-bar.nx
cargo run -p nexa-cli -- check examples/linking.nx
cargo run -p nexa-cli -- check examples/accessibility.nx
cargo run -p nexa-cli -- check examples/direction.nx
cargo run -p nexa-cli -- check examples/lifecycle.nx
cargo run -p nexa-cli -- check examples/text-style.nx
cargo run -p nexa-cli -- check examples/button-loading.nx
cargo run -p nexa-cli -- check examples/button-disabled.nx
cargo run -p nexa-cli -- check examples/pressable-long-press.nx
cargo run -p nexa-cli -- check examples/text-input-submit.nx
cargo run -p nexa-cli -- check examples/borders.nx
cargo run -p nexa-cli -- check examples/counter.nx --deny-warnings
cargo run -p nexa-cli -- check examples/platform-widgets.nx
cargo run -p nexa-cli -- check examples/network-image.nx
cargo run -p nexa-cli -- build examples/themed-app.nx --target swift --out /tmp/ThemedApp.swift
cargo run -p nexa-cli -- build examples/themed-app.nx --target kotlin --out /tmp/ThemedApp.kt
cargo run -p nexa-cli -- build examples/custom-components.nx --target swift --out /tmp/CustomComponents.swift
cargo run -p nexa-cli -- build examples/custom-components.nx --target kotlin --out /tmp/CustomComponents.kt
cargo run -p nexa-cli -- build examples/conditional-logic.nx --target swift --out /tmp/ConditionalLogic.swift
cargo run -p nexa-cli -- build examples/conditional-logic.nx --target kotlin --out /tmp/ConditionalLogic.kt
cargo run -p nexa-cli -- build examples/collection-values.nx --target swift --out /tmp/CollectionValues.swift
cargo run -p nexa-cli -- build examples/collection-values.nx --target kotlin --out /tmp/CollectionValues.kt
cargo run -p nexa-cli -- build examples/responsive-layout.nx --target swift --out /tmp/ResponsiveLayout.swift
cargo run -p nexa-cli -- build examples/responsive-layout.nx --target kotlin --out /tmp/ResponsiveLayout.kt
cargo run -p nexa-cli -- build examples/constant-branches.nx --target swift --out /tmp/ConstantBranches.swift
cargo run -p nexa-cli -- build examples/constant-branches.nx --target kotlin --out /tmp/ConstantBranches.kt
cargo run -p nexa-cli -- build examples/interpolation.nx --target swift --out /tmp/Interpolation.swift
cargo run -p nexa-cli -- build examples/interpolation.nx --target kotlin --out /tmp/Interpolation.kt
cargo run -p nexa-cli -- build examples/status-bar.nx --target swift --out /tmp/StatusBar.swift
cargo run -p nexa-cli -- build examples/status-bar.nx --target kotlin --out /tmp/StatusBar.kt
cargo run -p nexa-cli -- build examples/bottom-sheet.nx --target swift --out /tmp/BottomSheet.swift
cargo run -p nexa-cli -- build examples/bottom-sheet.nx --target kotlin --out /tmp/BottomSheet.kt
cargo run -p nexa-cli -- build examples/refresh-control.nx --target swift --out /tmp/RefreshControl.swift
cargo run -p nexa-cli -- build examples/refresh-control.nx --target kotlin --out /tmp/RefreshControl.kt
cargo run -p nexa-cli -- build examples/app-bottom-bar.nx --target swift --out /tmp/AppBottomBar.swift
cargo run -p nexa-cli -- build examples/app-bottom-bar.nx --target kotlin --out /tmp/AppBottomBar.kt
cargo run -p nexa-cli -- build examples/linking.nx --target swift --out /tmp/Linking.swift
cargo run -p nexa-cli -- build examples/linking.nx --target kotlin --out /tmp/Linking.kt
cargo run -p nexa-cli -- build examples/accessibility.nx --target swift --out /tmp/Accessibility.swift
cargo run -p nexa-cli -- build examples/accessibility.nx --target kotlin --out /tmp/Accessibility.kt
cargo run -p nexa-cli -- build examples/direction.nx --target swift --out /tmp/Direction.swift
cargo run -p nexa-cli -- build examples/direction.nx --target kotlin --out /tmp/Direction.kt
cargo run -p nexa-cli -- build examples/lifecycle.nx --target swift --out /tmp/Lifecycle.swift
cargo run -p nexa-cli -- build examples/lifecycle.nx --target kotlin --out /tmp/Lifecycle.kt
cargo run -p nexa-cli -- build examples/text-style.nx --target swift --out /tmp/TextStyle.swift
cargo run -p nexa-cli -- build examples/text-style.nx --target kotlin --out /tmp/TextStyle.kt
cargo run -p nexa-cli -- build examples/button-loading.nx --target swift --out /tmp/ButtonLoading.swift
cargo run -p nexa-cli -- build examples/button-loading.nx --target kotlin --out /tmp/ButtonLoading.kt
cargo run -p nexa-cli -- build examples/button-disabled.nx --target swift --out /tmp/ButtonDisabled.swift
cargo run -p nexa-cli -- build examples/button-disabled.nx --target kotlin --out /tmp/ButtonDisabled.kt
cargo run -p nexa-cli -- build examples/pressable-long-press.nx --target swift --out /tmp/PressableLongPress.swift
cargo run -p nexa-cli -- build examples/pressable-long-press.nx --target kotlin --out /tmp/PressableLongPress.kt
cargo run -p nexa-cli -- build examples/text-input-submit.nx --target swift --out /tmp/TextInputSubmit.swift
cargo run -p nexa-cli -- build examples/text-input-submit.nx --target kotlin --out /tmp/TextInputSubmit.kt
cargo run -p nexa-cli -- build examples/borders.nx --target swift --out /tmp/Borders.swift
cargo run -p nexa-cli -- build examples/borders.nx --target kotlin --out /tmp/Borders.kt
cargo run -p nexa-cli -- build examples/platform-widgets.nx --target swift --out /tmp/PlatformWidgets.swift
cargo run -p nexa-cli -- build examples/platform-widgets.nx --target kotlin --out /tmp/PlatformWidgets.kt
cargo run -p nexa-cli -- build examples/network-image.nx --target swift --out /tmp/NetworkImage.swift
cargo run -p nexa-cli -- build examples/network-image.nx --target kotlin --out /tmp/NetworkImage.kt
```

The default output replaces the input file extension, producing `counter.swift` or `counter.kt`. Generated files are intended to be added to an existing SwiftUI or Compose application with the corresponding platform dependencies configured.

## Workspace layout

| Crate | Responsibility |
| --- | --- |
| `nexa-diagnostics` | Source spans and compiler diagnostics |
| `nexa-syntax` | Lexer, parser, and source AST |
| `nexa-compiler` | Semantic checks and source-to-IR compilation |
| `nexa-ir` | Shared platform-independent typed IR |
| `nexa-codegen` | Backend contract and generated-name rules |
| `nexa-backend-swift` | SwiftUI source generation |
| `nexa-backend-kotlin` | Jetpack Compose source generation |
| `nexa-cli` | `check` and `build` commands |

Each target backend consumes the same typed IR. Adding another backend should require implementing the `nexa-codegen::Backend` contract without changing the lexer or parser.

Compiler orchestration and semantic analysis are separate modules inside `nexa-compiler`. Native generation lives in backend-local `src/generator/` modules, with component-specific files for controls, inputs, images, layout, navigation, lists, keyboard behavior, expressions, state, networking, and formatting. A shared IR walker keeps structural scans consistent across backends; each backend analyzes a module once and uses that result to emit only the imports and native helpers the generated app needs. This keeps platform concerns out of the shared IR and makes compiler changes easier to review and maintain.

The compiler also runs a conservative IR optimization pass before backend generation. It folds pure literal conditions, removes statically unreachable UI and event branches, and prunes pure functions that are unreachable from the app. These changes add no runtime machinery and do not alter native component mappings. See [constant-branches.nx](examples/constant-branches.nx).

Compiler warnings cover unused declarations, unused pure functions, unused component parameters, and constant conditions. Read [compiler diagnostics and optimization](docs/compiler-diagnostics.md) for the warning policy, `--deny-warnings`, and the native-code optimization boundaries.

Compile-time platform widgets use `platform ios { ... }` and `platform android { ... }`. The inactive block is removed before semantic lowering and native generation; see [platform-widgets.nx](examples/platform-widgets.nx).

User-defined `.nx` components can live in imported files, declare typed inputs and private state, compose other components, and compile directly into native SwiftUI or Compose declarations. Each reachable component has one generated native declaration, and every use calls that declaration directly; imports resolve relative to the source file and are statically compiled, while unreachable component declarations are omitted from generated output.

Remote images use the generated network stack: URLSession with a 16 MiB memory/64 MiB disk URLCache on iOS, and Coil 3 backed by a Cronet client with a 64 MiB disk cache, HTTP/2, QUIC, and Brotli on Android. The generated `NexaNetwork`, `NexaPath`, and `NexaFile` helpers are emitted only for modules that use networking. See the [network and file API guide](docs/language.md#native-network-and-file-library) for request options, downloads, cache behavior, and certificate pinning.

## Project skills

The repository includes three project-scoped Codex skills under `.agents/skills/`:

- `nexa-framework` for the compiler, IR, core components, native backends, and performance architecture.
- `nexa-plugin` for designing optional, typed platform integrations.
- `nexa-app` for helping app authors build with the supported `.nx` language without writing Swift or Kotlin.

These skills distinguish the current prototype from the longer-term goals in `plan.md` and guide changes toward modular, native code generation.

## Language slice

See the [language guide](docs/language.md) for syntax, supported types, themes, current limits, and native mappings, and the [language design decisions](docs/language-design.md) for how Nexa adopts or defers Kotlin/Swift concepts. The full roadmap remains in [plan.md](plan.md).

The authoring surface keeps mutability explicit: `state` stays mutable and `let` stays immutable. Both declarations infer their type from non-empty initializers using the same defaults: integer literals become `Int32` and decimal literals become `Float64`; the resolved type is fixed in the typed IR before native bindings are generated. `Column` is the single vertical container; it emits direct native stack code on both platforms. `View` is no longer a Nexa component; replace it with `Column` in existing `.nx` files. SwiftUI's native `View` protocol remains part of generated Swift output.

String interpolation supports `$name` and `\(name)` for declared state and constant names. The compiler lowers each segment into direct Swift or Kotlin interpolation without a template runtime; see [interpolation.nx](examples/interpolation.nx).

`StatusBar(style: Default|Light|Dark, hidden: true|false)` is a single top-level app configuration that lowers to native status-bar APIs; see [status-bar.nx](examples/status-bar.nx).

`Pressable` supports a tap action block, an optional third long-press action block, and a Boolean `disabled` expression. Long press and disabled state map to native SwiftUI and Compose gestures without a shared runtime; see [pressable-long-press.nx](examples/pressable-long-press.nx).

`Button` supports optional `loading` and `disabled` Boolean expressions. The compiler emits native loading indicators and combines both conditions into one platform-native enabled/disabled expression; see [button-loading.nx](examples/button-loading.nx) and [button-disabled.nx](examples/button-disabled.nx).

`TextInput` supports an optional mutable `focused` Boolean binding. It maps to SwiftUI focus state and Compose `FocusRequester`/focus callbacks; submit actions remain native `.onSubmit`/`KeyboardActions`. See [text-input-submit.nx](examples/text-input-submit.nx).

`Column` and `Row` support paired static `borderColor` and `borderWidth` values, compiled to native SwiftUI and Compose border primitives; see [borders.nx](examples/borders.nx).

`Column` and `Row` also support a static native `animation` spec (`Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`) for content-size changes; see [animations.nx](examples/animations.nx).

`AppBottomBar(selected: ...)` provides static labeled tabs with native `TabView` and Material 3 `NavigationBar` output; see [app-bottom-bar.nx](examples/app-bottom-bar.nx).

`Direction(value: LTR|RTL)` applies a static native layout direction at the app root; see [direction.nx](examples/direction.nx).

`OnAppear { ... }` and `OnDisappear { ... }` are top-level app or named-screen lifecycle callbacks. Use `OnAppear async { ... }` to await an app-local `async fn`; Swift lowers it to `.task`, while Android uses `LaunchedEffect(Unit)` for the same native coroutine lifecycle. See [lifecycle.nx](examples/lifecycle.nx), [async.nx](examples/async.nx), and [navigation.nx](examples/navigation.nx). App background events remain future lifecycle slices.

Typed app functions use `fn name(parameters) -> ReturnType { let local = expression; return expression }`; local constants are resolved and emitted as direct Swift `let` or Kotlin `val` statements. `async fn` adds a native async boundary and `await` calls it from `OnAppear async`. Calls are resolved during compilation with no runtime registry. See [functions.nx](examples/functions.nx), [function-locals.nx](examples/function-locals.nx), and [async.nx](examples/async.nx).

## License

Copyright © 2026 Nexa contributors. Nexa is licensed under the GNU General Public License v3.0 only; see [LICENSE](LICENSE) for the full terms.
